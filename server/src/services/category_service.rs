use std::collections::HashSet;

use chrono::Utc;
use serde::Serialize;
use uuid::Uuid;

use crate::{
    AppError,
    db::DbPool,
    models::{
        category::{self, ChannelCategory},
        channel::{self, ChannelPositionUpdate},
        guild::{self, Guild},
        guild_member,
    },
};

const MAX_CATEGORY_NAME_CHARS: usize = 64;
const MAX_CATEGORY_SLUG_CHARS: usize = 48;
const MAX_CATEGORY_SLUG_ATTEMPTS: usize = 100;

#[derive(Debug, Clone)]
pub struct CreateCategoryInput {
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct UpdateCategoryInput {
    pub name: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ReorderCategoriesInput {
    pub category_slugs: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct UpdateCategoryCollapseInput {
    pub collapsed: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct CategoryResponse {
    pub id: String,
    pub slug: String,
    pub name: String,
    pub position: i64,
    pub collapsed: bool,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct DeleteCategoryResponse {
    pub deleted_slug: String,
    pub reassigned_channel_count: i64,
}

pub async fn list_categories(
    pool: &DbPool,
    user_id: &str,
    guild_slug: &str,
) -> Result<Vec<CategoryResponse>, AppError> {
    let guild = load_viewable_guild(pool, user_id, guild_slug).await?;
    let categories = category::list_categories_by_guild_id(pool, &guild.id).await?;
    let collapsed_ids = category::list_collapsed_category_ids(pool, user_id, &guild.id).await?;
    let collapsed_set: HashSet<String> = collapsed_ids.into_iter().collect();
    Ok(categories
        .into_iter()
        .map(|item| {
            let collapsed = collapsed_set.contains(&item.id);
            to_category_response(item, collapsed)
        })
        .collect())
}

pub async fn create_category(
    pool: &DbPool,
    user_id: &str,
    guild_slug: &str,
    input: CreateCategoryInput,
) -> Result<CategoryResponse, AppError> {
    let guild = load_owned_guild(pool, user_id, guild_slug).await?;
    let name = normalize_category_name(&input.name)?;
    let base_slug = slugify(&name);
    let position = category::next_category_position(pool, &guild.id).await?;
    let created_at = Utc::now().to_rfc3339();

    for attempt in 0..MAX_CATEGORY_SLUG_ATTEMPTS {
        let slug = slug_for_attempt(&base_slug, attempt);
        let id = Uuid::new_v4().to_string();
        let inserted = category::insert_category(
            pool,
            &id,
            &guild.id,
            &slug,
            &name,
            position,
            &created_at,
            &created_at,
        )
        .await?;
        if inserted {
            let created = category::find_category_by_slug(pool, &guild.id, &slug)
                .await?
                .ok_or_else(|| AppError::Internal("Created category not found".to_string()))?;
            return Ok(to_category_response(created, false));
        }
    }

    Err(AppError::Conflict(
        "Category name is already in use".to_string(),
    ))
}

pub async fn update_category(
    pool: &DbPool,
    user_id: &str,
    guild_slug: &str,
    category_slug: &str,
    input: UpdateCategoryInput,
) -> Result<CategoryResponse, AppError> {
    let guild = load_owned_guild(pool, user_id, guild_slug).await?;
    let existing = category::find_category_by_slug(pool, &guild.id, category_slug)
        .await?
        .ok_or(AppError::NotFound)?;

    let Some(name_input) = input.name else {
        return Err(AppError::ValidationError(
            "At least one category field is required".to_string(),
        ));
    };
    let name = normalize_category_name(&name_input)?;
    let base_slug = slugify(&name);
    let slug = choose_available_slug_for_update(pool, &guild.id, &existing.id, &base_slug).await?;
    let updated_at = Utc::now().to_rfc3339();

    let rows = category::update_category(pool, &existing.id, &name, &slug, &updated_at).await?;
    if rows == 0 {
        return Err(AppError::NotFound);
    }

    let updated = category::find_category_by_slug(pool, &guild.id, &slug)
        .await?
        .ok_or(AppError::NotFound)?;
    let collapsed_ids = category::list_collapsed_category_ids(pool, user_id, &guild.id).await?;
    let collapsed_set: HashSet<String> = collapsed_ids.into_iter().collect();
    Ok(to_category_response(
        updated.clone(),
        collapsed_set.contains(&updated.id),
    ))
}

pub async fn delete_category(
    pool: &DbPool,
    user_id: &str,
    guild_slug: &str,
    category_slug: &str,
) -> Result<DeleteCategoryResponse, AppError> {
    let guild = load_owned_guild(pool, user_id, guild_slug).await?;
    let target = category::find_category_by_slug(pool, &guild.id, category_slug)
        .await?
        .ok_or(AppError::NotFound)?;
    let channels = channel::list_channels_by_category_id(pool, &guild.id, &target.id).await?;
    let reassigned_channel_count = channels.len() as i64;

    if !channels.is_empty() {
        let next_uncategorized =
            channel::next_channel_position_for_category(pool, &guild.id, None).await?;
        let updated_at = Utc::now().to_rfc3339();
        let updates: Vec<ChannelPositionUpdate> = channels
            .into_iter()
            .enumerate()
            .map(|(index, item)| ChannelPositionUpdate {
                slug: item.slug,
                category_id: None,
                position: next_uncategorized + index as i64,
            })
            .collect();
        channel::reorder_channel_positions(pool, &guild.id, &updates, &updated_at).await?;
    }

    let rows = category::delete_category(pool, &target.id).await?;
    if rows == 0 {
        return Err(AppError::NotFound);
    }

    let updated_at = Utc::now().to_rfc3339();
    category::compact_category_positions(pool, &guild.id, &updated_at).await?;
    channel::compact_channel_positions(pool, &guild.id, &updated_at).await?;

    Ok(DeleteCategoryResponse {
        deleted_slug: target.slug,
        reassigned_channel_count,
    })
}

pub async fn reorder_categories(
    pool: &DbPool,
    user_id: &str,
    guild_slug: &str,
    input: ReorderCategoriesInput,
) -> Result<Vec<CategoryResponse>, AppError> {
    let guild = load_owned_guild(pool, user_id, guild_slug).await?;
    if input.category_slugs.is_empty() {
        return Err(AppError::ValidationError(
            "category_slugs is required".to_string(),
        ));
    }

    let existing = category::list_categories_by_guild_id(pool, &guild.id).await?;
    if input.category_slugs.len() != existing.len() {
        return Err(AppError::ValidationError(
            "category_slugs must include every category exactly once".to_string(),
        ));
    }

    let existing_set: HashSet<String> = existing.iter().map(|item| item.slug.clone()).collect();
    let mut incoming_set = HashSet::new();
    for slug in &input.category_slugs {
        if !existing_set.contains(slug) {
            return Err(AppError::ValidationError(
                "category_slugs contains unknown category".to_string(),
            ));
        }
        if !incoming_set.insert(slug.clone()) {
            return Err(AppError::ValidationError(
                "category_slugs contains duplicate categories".to_string(),
            ));
        }
    }

    let updated_at = Utc::now().to_rfc3339();
    category::reorder_categories(pool, &guild.id, &input.category_slugs, &updated_at).await?;

    let collapsed_ids = category::list_collapsed_category_ids(pool, user_id, &guild.id).await?;
    let collapsed_set: HashSet<String> = collapsed_ids.into_iter().collect();
    let categories = category::list_categories_by_guild_id(pool, &guild.id).await?;
    Ok(categories
        .into_iter()
        .map(|item| {
            let collapsed = collapsed_set.contains(&item.id);
            to_category_response(item, collapsed)
        })
        .collect())
}

pub async fn update_category_collapse(
    pool: &DbPool,
    user_id: &str,
    guild_slug: &str,
    category_slug: &str,
    input: UpdateCategoryCollapseInput,
) -> Result<CategoryResponse, AppError> {
    let guild = load_owned_guild(pool, user_id, guild_slug).await?;
    let category_record = category::find_category_by_slug(pool, &guild.id, category_slug)
        .await?
        .ok_or(AppError::NotFound)?;
    let now = Utc::now().to_rfc3339();
    category::upsert_category_collapse(
        pool,
        user_id,
        &guild.id,
        &category_record.id,
        input.collapsed,
        &now,
    )
    .await?;

    Ok(to_category_response(category_record, input.collapsed))
}

fn to_category_response(category: ChannelCategory, collapsed: bool) -> CategoryResponse {
    CategoryResponse {
        id: category.id,
        slug: category.slug,
        name: category.name,
        position: category.position,
        collapsed,
        created_at: category.created_at,
    }
}

async fn load_owned_guild(
    pool: &DbPool,
    user_id: &str,
    guild_slug: &str,
) -> Result<Guild, AppError> {
    let guild = guild::find_guild_by_slug(pool, guild_slug)
        .await?
        .ok_or(AppError::NotFound)?;
    if guild.owner_id != user_id {
        return Err(AppError::Forbidden(
            "Only guild owners can manage categories".to_string(),
        ));
    }
    Ok(guild)
}

async fn load_viewable_guild(
    pool: &DbPool,
    user_id: &str,
    guild_slug: &str,
) -> Result<Guild, AppError> {
    let guild = guild::find_guild_by_slug(pool, guild_slug)
        .await?
        .ok_or(AppError::NotFound)?;
    if guild.owner_id == user_id || guild_member::is_guild_member(pool, &guild.id, user_id).await? {
        return Ok(guild);
    }
    Err(AppError::Forbidden(
        "Only guild members can view categories".to_string(),
    ))
}

async fn choose_available_slug_for_update(
    pool: &DbPool,
    guild_id: &str,
    current_category_id: &str,
    base_slug: &str,
) -> Result<String, AppError> {
    for attempt in 0..MAX_CATEGORY_SLUG_ATTEMPTS {
        let candidate = slug_for_attempt(base_slug, attempt);
        let existing = category::find_category_by_slug(pool, guild_id, &candidate).await?;
        if let Some(existing) = existing {
            if existing.id == current_category_id {
                return Ok(candidate);
            }
            continue;
        }
        return Ok(candidate);
    }

    Err(AppError::Conflict(
        "Category name is already in use".to_string(),
    ))
}

fn normalize_category_name(value: &str) -> Result<String, AppError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(AppError::ValidationError("name is required".to_string()));
    }
    if trimmed.chars().count() > MAX_CATEGORY_NAME_CHARS {
        return Err(AppError::ValidationError(format!(
            "name must be {MAX_CATEGORY_NAME_CHARS} characters or less"
        )));
    }
    if trimmed.chars().any(|ch| ch.is_control()) {
        return Err(AppError::ValidationError(
            "name contains invalid characters".to_string(),
        ));
    }
    Ok(trimmed.to_string())
}

fn slugify(value: &str) -> String {
    let mut slug = String::new();
    let mut dash_pending = false;
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() {
            if slug.len() >= MAX_CATEGORY_SLUG_CHARS {
                break;
            }
            slug.push(ch.to_ascii_lowercase());
            dash_pending = false;
        } else if !slug.is_empty() {
            dash_pending = true;
        }

        if dash_pending && !slug.ends_with('-') && slug.len() < MAX_CATEGORY_SLUG_CHARS {
            slug.push('-');
            dash_pending = false;
        }
    }

    while slug.ends_with('-') {
        slug.pop();
    }

    if slug.is_empty() {
        "category".to_string()
    } else {
        slug
    }
}

fn slug_for_attempt(base_slug: &str, attempt: usize) -> String {
    if attempt == 0 {
        return base_slug.to_string();
    }

    let suffix = format!("-{}", attempt + 1);
    let max_base_chars = MAX_CATEGORY_SLUG_CHARS.saturating_sub(suffix.len());
    let mut truncated: String = base_slug.chars().take(max_base_chars).collect();
    while truncated.ends_with('-') {
        truncated.pop();
    }
    if truncated.is_empty() {
        truncated = "category".to_string();
    }
    format!("{truncated}{suffix}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugify_generates_ascii_slug() {
        assert_eq!(slugify("Ops Team"), "ops-team");
        assert_eq!(slugify("   "), "category");
    }
}
