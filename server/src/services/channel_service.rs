use std::collections::{HashMap, HashSet};

use chrono::Utc;
use serde::Serialize;
use uuid::Uuid;

use crate::{
    AppError,
    db::DbPool,
    models::{
        category,
        channel::{self, Channel, ChannelPositionUpdate},
        guild::{self, Guild},
    },
    permissions,
};

const MAX_CHANNEL_NAME_CHARS: usize = 64;
const MAX_CHANNEL_SLUG_CHARS: usize = 48;
const MAX_CHANNEL_SLUG_ATTEMPTS: usize = 100;

#[derive(Debug, Clone)]
pub struct CreateChannelInput {
    pub name: String,
    pub channel_type: String,
    pub category_slug: Option<String>,
}

#[derive(Debug, Clone)]
pub struct UpdateChannelInput {
    pub name: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ReorderChannelsInput {
    pub channel_slugs: Vec<String>,
    pub channel_positions: Vec<ReorderChannelPositionInput>,
}

#[derive(Debug, Clone)]
pub struct ReorderChannelPositionInput {
    pub channel_slug: String,
    pub category_slug: Option<String>,
    pub position: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ChannelResponse {
    pub id: String,
    pub slug: String,
    pub name: String,
    pub channel_type: String,
    pub position: i64,
    pub is_default: bool,
    pub category_slug: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct DeleteChannelResponse {
    pub deleted_slug: String,
    pub fallback_channel_slug: String,
}

pub async fn list_channels(
    pool: &DbPool,
    user_id: &str,
    guild_slug: &str,
) -> Result<Vec<ChannelResponse>, AppError> {
    let guild = load_viewable_guild(pool, user_id, guild_slug).await?;
    let channels = channel::list_channels_by_guild_id(pool, &guild.id).await?;
    Ok(channels
        .into_iter()
        .map(|item| to_channel_response(item, &guild.default_channel_slug))
        .collect())
}

pub async fn create_channel(
    pool: &DbPool,
    user_id: &str,
    guild_slug: &str,
    input: CreateChannelInput,
) -> Result<ChannelResponse, AppError> {
    let guild = load_guild_with_channel_manage_access(pool, user_id, guild_slug).await?;
    let name = normalize_channel_name(&input.name)?;
    let channel_type = normalize_channel_type(&input.channel_type)?;
    let category_id = resolve_category_id(pool, &guild.id, input.category_slug.as_deref()).await?;
    let base_slug = slugify(&name);
    let position =
        channel::next_channel_position_for_category(pool, &guild.id, category_id.as_deref())
            .await?;
    let created_at = Utc::now().to_rfc3339();

    for attempt in 0..MAX_CHANNEL_SLUG_ATTEMPTS {
        let slug = slug_for_attempt(&base_slug, attempt);
        let id = Uuid::new_v4().to_string();
        let inserted = channel::insert_channel(
            pool,
            &id,
            &guild.id,
            &slug,
            &name,
            channel_type,
            position,
            category_id.as_deref(),
            &created_at,
            &created_at,
        )
        .await?;
        if inserted {
            let created = channel::find_channel_by_slug(pool, &guild.id, &slug)
                .await?
                .ok_or_else(|| AppError::Internal("Created channel not found".to_string()))?;
            return Ok(to_channel_response(created, &guild.default_channel_slug));
        }
    }

    Err(AppError::Conflict(
        "Channel name is already in use".to_string(),
    ))
}

pub async fn update_channel(
    pool: &DbPool,
    user_id: &str,
    guild_slug: &str,
    channel_slug: &str,
    input: UpdateChannelInput,
) -> Result<ChannelResponse, AppError> {
    let guild = load_guild_with_channel_manage_access(pool, user_id, guild_slug).await?;
    let existing = channel::find_channel_by_slug(pool, &guild.id, channel_slug)
        .await?
        .ok_or(AppError::NotFound)?;

    let Some(name_input) = input.name else {
        return Err(AppError::ValidationError(
            "At least one channel field is required".to_string(),
        ));
    };
    let name = normalize_channel_name(&name_input)?;
    let base_slug = slugify(&name);
    let slug = choose_available_slug_for_update(pool, &guild.id, &existing.id, &base_slug).await?;
    let updated_at = Utc::now().to_rfc3339();

    let rows = channel::update_channel(pool, &existing.id, &name, &slug, &updated_at).await?;
    if rows == 0 {
        return Err(AppError::NotFound);
    }

    let mut default_channel_slug = guild.default_channel_slug.clone();
    if guild.default_channel_slug == existing.slug && slug != existing.slug {
        let default_rows =
            guild::update_default_channel_slug(pool, &guild.id, &slug, &updated_at).await?;
        if default_rows == 0 {
            return Err(AppError::NotFound);
        }
        default_channel_slug = slug.clone();
    }

    let updated = channel::find_channel_by_slug(pool, &guild.id, &slug)
        .await?
        .ok_or(AppError::NotFound)?;
    Ok(to_channel_response(updated, &default_channel_slug))
}

pub async fn delete_channel(
    pool: &DbPool,
    user_id: &str,
    guild_slug: &str,
    channel_slug: &str,
) -> Result<DeleteChannelResponse, AppError> {
    let guild = load_guild_with_channel_manage_access(pool, user_id, guild_slug).await?;
    let target = channel::find_channel_by_slug(pool, &guild.id, channel_slug)
        .await?
        .ok_or(AppError::NotFound)?;

    let count = channel::count_channels_by_guild_id(pool, &guild.id).await?;
    if count <= 1 {
        return Err(AppError::ValidationError(
            "At least one channel must remain in the guild".to_string(),
        ));
    }

    let rows = channel::delete_channel(pool, &target.id).await?;
    if rows == 0 {
        return Err(AppError::NotFound);
    }

    let updated_at = Utc::now().to_rfc3339();
    channel::compact_channel_positions(pool, &guild.id, &updated_at).await?;
    let remaining = channel::list_channels_by_guild_id(pool, &guild.id).await?;
    let fallback_channel_slug = remaining
        .first()
        .map(|item| item.slug.clone())
        .ok_or_else(|| AppError::Internal("Expected at least one remaining channel".to_string()))?;

    let existing_default_exists = remaining
        .iter()
        .any(|item| item.slug == guild.default_channel_slug);
    let next_default = if existing_default_exists {
        guild.default_channel_slug.clone()
    } else {
        fallback_channel_slug.clone()
    };

    if next_default != guild.default_channel_slug {
        let default_rows =
            guild::update_default_channel_slug(pool, &guild.id, &next_default, &updated_at).await?;
        if default_rows == 0 {
            return Err(AppError::NotFound);
        }
    }

    Ok(DeleteChannelResponse {
        deleted_slug: target.slug,
        fallback_channel_slug: next_default,
    })
}

pub async fn reorder_channels(
    pool: &DbPool,
    user_id: &str,
    guild_slug: &str,
    input: ReorderChannelsInput,
) -> Result<Vec<ChannelResponse>, AppError> {
    let guild = load_guild_with_channel_manage_access(pool, user_id, guild_slug).await?;
    let existing = channel::list_channels_by_guild_id(pool, &guild.id).await?;

    if !input.channel_positions.is_empty() {
        if input.channel_positions.len() != existing.len() {
            return Err(AppError::ValidationError(
                "channel_positions must include every channel exactly once".to_string(),
            ));
        }

        let categories = category::list_categories_by_guild_id(pool, &guild.id).await?;
        let category_by_slug: HashMap<String, String> = categories
            .into_iter()
            .map(|item| (item.slug, item.id))
            .collect();

        let existing_set: HashSet<String> = existing.iter().map(|item| item.slug.clone()).collect();
        let mut seen_channels = HashSet::new();
        let mut seen_positions = HashMap::<Option<String>, HashSet<i64>>::new();
        let mut updates = Vec::with_capacity(input.channel_positions.len());

        for item in input.channel_positions {
            if item.position < 0 {
                return Err(AppError::ValidationError(
                    "position must be non-negative".to_string(),
                ));
            }
            if !existing_set.contains(&item.channel_slug) {
                return Err(AppError::ValidationError(
                    "channel_positions contains unknown channel".to_string(),
                ));
            }
            if !seen_channels.insert(item.channel_slug.clone()) {
                return Err(AppError::ValidationError(
                    "channel_positions contains duplicate channels".to_string(),
                ));
            }

            let normalized_category_slug = item.category_slug.and_then(|slug| {
                let trimmed = slug.trim();
                if trimmed.is_empty() {
                    None
                } else {
                    Some(trimmed.to_string())
                }
            });

            let category_id = match normalized_category_slug.as_deref() {
                Some(slug) => Some(
                    category_by_slug
                        .get(slug)
                        .ok_or_else(|| {
                            AppError::ValidationError(
                                "channel_positions contains unknown category".to_string(),
                            )
                        })?
                        .clone(),
                ),
                None => None,
            };

            let positions = seen_positions.entry(category_id.clone()).or_default();
            if !positions.insert(item.position) {
                return Err(AppError::ValidationError(
                    "channel_positions contains duplicate positions within a category".to_string(),
                ));
            }

            updates.push(ChannelPositionUpdate {
                slug: item.channel_slug,
                category_id,
                position: item.position,
            });
        }

        let updated_at = Utc::now().to_rfc3339();
        channel::reorder_channel_positions(pool, &guild.id, &updates, &updated_at).await?;
    } else {
        if input.channel_slugs.is_empty() {
            return Err(AppError::ValidationError(
                "channel_slugs or channel_positions is required".to_string(),
            ));
        }
        if input.channel_slugs.len() != existing.len() {
            return Err(AppError::ValidationError(
                "channel_slugs must include every channel exactly once".to_string(),
            ));
        }

        let existing_set: HashSet<String> = existing.iter().map(|item| item.slug.clone()).collect();
        let existing_category_by_slug: HashMap<String, Option<String>> = existing
            .iter()
            .map(|item| (item.slug.clone(), item.category_id.clone()))
            .collect();
        let mut incoming_set = HashSet::new();
        for slug in &input.channel_slugs {
            if !existing_set.contains(slug) {
                return Err(AppError::ValidationError(
                    "channel_slugs contains unknown channel".to_string(),
                ));
            }
            if !incoming_set.insert(slug.clone()) {
                return Err(AppError::ValidationError(
                    "channel_slugs contains duplicate channels".to_string(),
                ));
            }
        }

        let mut next_positions = HashMap::<Option<String>, i64>::new();
        let mut updates = Vec::with_capacity(input.channel_slugs.len());
        for slug in input.channel_slugs {
            let category_id = existing_category_by_slug
                .get(&slug)
                .cloned()
                .ok_or_else(|| {
                    AppError::ValidationError("channel_slugs contains unknown channel".to_string())
                })?;
            let position = *next_positions.get(&category_id).unwrap_or(&0);
            next_positions.insert(category_id.clone(), position + 1);
            updates.push(ChannelPositionUpdate {
                slug,
                category_id,
                position,
            });
        }

        let updated_at = Utc::now().to_rfc3339();
        channel::reorder_channel_positions(pool, &guild.id, &updates, &updated_at).await?;
    }

    let reordered = channel::list_channels_by_guild_id(pool, &guild.id).await?;
    Ok(reordered
        .into_iter()
        .map(|item| to_channel_response(item, &guild.default_channel_slug))
        .collect())
}

fn to_channel_response(channel: Channel, default_channel_slug: &str) -> ChannelResponse {
    ChannelResponse {
        id: channel.id,
        slug: channel.slug.clone(),
        name: channel.name,
        channel_type: channel.channel_type,
        position: channel.position,
        is_default: channel.slug == default_channel_slug,
        category_slug: channel.category_slug,
        created_at: channel.created_at,
    }
}

async fn load_guild_with_channel_manage_access(
    pool: &DbPool,
    user_id: &str,
    guild_slug: &str,
) -> Result<Guild, AppError> {
    let guild = guild::find_guild_by_slug(pool, guild_slug)
        .await?
        .ok_or(AppError::NotFound)?;
    permissions::require_guild_permission(
        pool,
        &guild,
        user_id,
        permissions::MANAGE_CHANNELS,
        "MANAGE_CHANNELS",
    )
    .await?;
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
    if permissions::can_view_guild(pool, &guild, user_id).await? {
        return Ok(guild);
    }
    Err(AppError::Forbidden(
        "Only guild members can view channels".to_string(),
    ))
}

async fn resolve_category_id(
    pool: &DbPool,
    guild_id: &str,
    category_slug: Option<&str>,
) -> Result<Option<String>, AppError> {
    let Some(category_slug) = category_slug else {
        return Ok(None);
    };
    let trimmed = category_slug.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }

    let category = category::find_category_by_slug(pool, guild_id, trimmed)
        .await?
        .ok_or_else(|| AppError::ValidationError("category_slug does not exist".to_string()))?;
    Ok(Some(category.id))
}

fn normalize_channel_name(value: &str) -> Result<String, AppError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(AppError::ValidationError("name is required".to_string()));
    }
    if trimmed.chars().count() > MAX_CHANNEL_NAME_CHARS {
        return Err(AppError::ValidationError(format!(
            "name must be {MAX_CHANNEL_NAME_CHARS} characters or less"
        )));
    }
    if trimmed.chars().any(|ch| ch.is_control()) {
        return Err(AppError::ValidationError(
            "name contains invalid characters".to_string(),
        ));
    }
    Ok(trimmed.to_string())
}

fn normalize_channel_type(value: &str) -> Result<&'static str, AppError> {
    match value.trim().to_ascii_lowercase().as_str() {
        "text" => Ok("text"),
        "voice" => Ok("voice"),
        _ => Err(AppError::ValidationError(
            "channel_type must be one of: text, voice".to_string(),
        )),
    }
}

async fn choose_available_slug_for_update(
    pool: &DbPool,
    guild_id: &str,
    current_channel_id: &str,
    base_slug: &str,
) -> Result<String, AppError> {
    for attempt in 0..MAX_CHANNEL_SLUG_ATTEMPTS {
        let candidate = slug_for_attempt(base_slug, attempt);
        let existing = channel::find_channel_by_slug(pool, guild_id, &candidate).await?;
        if let Some(existing) = existing {
            if existing.id == current_channel_id {
                return Ok(candidate);
            }
            continue;
        }
        return Ok(candidate);
    }

    Err(AppError::Conflict(
        "Channel name is already in use".to_string(),
    ))
}

fn slugify(value: &str) -> String {
    let mut slug = String::new();
    let mut dash_pending = false;
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() {
            if slug.len() >= MAX_CHANNEL_SLUG_CHARS {
                break;
            }
            slug.push(ch.to_ascii_lowercase());
            dash_pending = false;
        } else if !slug.is_empty() {
            dash_pending = true;
        }

        if dash_pending && !slug.ends_with('-') && slug.len() < MAX_CHANNEL_SLUG_CHARS {
            slug.push('-');
            dash_pending = false;
        }
    }

    while slug.ends_with('-') {
        slug.pop();
    }

    if slug.is_empty() {
        "channel".to_string()
    } else {
        slug
    }
}

fn slug_for_attempt(base_slug: &str, attempt: usize) -> String {
    if attempt == 0 {
        return base_slug.to_string();
    }

    let suffix = format!("-{}", attempt + 1);
    let max_base_chars = MAX_CHANNEL_SLUG_CHARS.saturating_sub(suffix.len());
    let mut truncated: String = base_slug.chars().take(max_base_chars).collect();
    while truncated.ends_with('-') {
        truncated.pop();
    }
    if truncated.is_empty() {
        truncated = "channel".to_string();
    }
    format!("{truncated}{suffix}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugify_produces_route_safe_slug() {
        assert_eq!(slugify("General Chat"), "general-chat");
        assert_eq!(slugify("   "), "channel");
        assert_eq!(slugify("Voice___Room"), "voice-room");
    }

    #[test]
    fn normalize_channel_type_rejects_invalid_values() {
        assert_eq!(normalize_channel_type("text").unwrap(), "text");
        assert_eq!(normalize_channel_type("VOICE").unwrap(), "voice");
        assert!(normalize_channel_type("video").is_err());
    }
}
