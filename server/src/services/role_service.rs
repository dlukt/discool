use std::collections::HashSet;

use chrono::Utc;
use serde::Serialize;
use uuid::Uuid;

use crate::{
    AppError,
    db::DbPool,
    models::{
        guild::{self, Guild},
        role::{self, Role},
    },
    permissions,
};

const MAX_ROLE_NAME_CHARS: usize = 64;
const EVERYONE_ROLE_NAME: &str = "@everyone";
const DEFAULT_EVERYONE_COLOR: &str = "#99aab5";
const DEFAULT_EVERYONE_POSITION: i64 = 2_147_483_647;
const OWNER_ROLE_NAME: &str = "Owner";
const OWNER_ROLE_COLOR: &str = "#f59e0b";

#[derive(Debug, Clone)]
pub struct CreateRoleInput {
    pub name: String,
    pub color: String,
}

#[derive(Debug, Clone)]
pub struct UpdateRoleInput {
    pub name: Option<String>,
    pub color: Option<String>,
    pub permissions_bitflag: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct ReorderRolesInput {
    pub role_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RoleResponse {
    pub id: String,
    pub name: String,
    pub color: String,
    pub position: i64,
    pub permissions_bitflag: i64,
    pub is_default: bool,
    pub is_system: bool,
    pub can_edit: bool,
    pub can_delete: bool,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct DeleteRoleResponse {
    pub deleted_id: String,
    pub removed_assignment_count: i64,
}

pub async fn list_roles(
    pool: &DbPool,
    user_id: &str,
    guild_slug: &str,
) -> Result<Vec<RoleResponse>, AppError> {
    let guild = load_guild_with_role_management_access(pool, user_id, guild_slug).await?;
    ensure_default_role(pool, &guild).await?;
    let roles = role::list_roles_by_guild_id(pool, &guild.id).await?;

    let mut response = Vec::with_capacity(roles.len() + 1);
    response.push(owner_role_response(&guild));
    response.extend(roles.into_iter().map(to_role_response));
    Ok(response)
}

pub async fn create_role(
    pool: &DbPool,
    user_id: &str,
    guild_slug: &str,
    input: CreateRoleInput,
) -> Result<RoleResponse, AppError> {
    let guild = load_guild_with_role_management_access(pool, user_id, guild_slug).await?;
    ensure_default_role(pool, &guild).await?;

    let name = normalize_custom_role_name(&input.name)?;
    let color = normalize_role_color(&input.color)?;
    if role_name_exists(pool, &guild.id, &name, None).await? {
        return Err(AppError::Conflict(
            "Role name is already in use".to_string(),
        ));
    }

    let id = Uuid::new_v4().to_string();
    let created_at = Utc::now().to_rfc3339();
    let position = role::next_custom_role_position(pool, &guild.id).await?;
    let inserted = role::insert_role(
        pool,
        &id,
        &guild.id,
        &name,
        &color,
        position,
        0,
        false,
        &created_at,
        &created_at,
    )
    .await?;
    if !inserted {
        return Err(AppError::Conflict(
            "Role name is already in use".to_string(),
        ));
    }

    let created = role::find_role_by_id(pool, &guild.id, &id)
        .await?
        .ok_or_else(|| AppError::Internal("Created role not found".to_string()))?;
    Ok(to_role_response(created))
}

pub async fn update_role(
    pool: &DbPool,
    user_id: &str,
    guild_slug: &str,
    role_id: &str,
    input: UpdateRoleInput,
) -> Result<RoleResponse, AppError> {
    let guild = load_guild_with_role_management_access(pool, user_id, guild_slug).await?;
    if role_id.starts_with("owner:") {
        return Err(AppError::ValidationError(
            "The Owner role cannot be edited".to_string(),
        ));
    }
    let existing = role::find_role_by_id(pool, &guild.id, role_id)
        .await?
        .ok_or(AppError::NotFound)?;

    if existing.is_default != 0 {
        return Err(AppError::ValidationError(
            "The @everyone role cannot be edited".to_string(),
        ));
    }

    if input.name.is_none() && input.color.is_none() && input.permissions_bitflag.is_none() {
        return Err(AppError::ValidationError(
            "At least one role field is required".to_string(),
        ));
    }

    let name = match input.name {
        Some(value) => normalize_custom_role_name(&value)?,
        None => existing.name.clone(),
    };
    if role_name_exists(pool, &guild.id, &name, Some(&existing.id)).await? {
        return Err(AppError::Conflict(
            "Role name is already in use".to_string(),
        ));
    }

    let color = match input.color {
        Some(value) => normalize_role_color(&value)?,
        None => existing.color.clone(),
    };
    let permissions_mask = match input.permissions_bitflag {
        Some(value) => permissions::parse_permissions_bitflag(value)?,
        None => permissions::stored_permissions_to_mask(existing.permissions_bitflag)?,
    };
    let permissions_bitflag = permissions::mask_to_stored_permissions(permissions_mask)?;
    let updated_at = Utc::now().to_rfc3339();
    let rows = role::update_custom_role(
        pool,
        &existing.id,
        &name,
        &color,
        permissions_bitflag,
        &updated_at,
    )
    .await?;
    if rows == 0 {
        return Err(AppError::NotFound);
    }
    permissions::invalidate_guild_permission_cache(&guild.id);

    let updated = role::find_role_by_id(pool, &guild.id, &existing.id)
        .await?
        .ok_or(AppError::NotFound)?;
    Ok(to_role_response(updated))
}

pub async fn delete_role(
    pool: &DbPool,
    user_id: &str,
    guild_slug: &str,
    role_id: &str,
) -> Result<DeleteRoleResponse, AppError> {
    let guild = load_guild_with_role_management_access(pool, user_id, guild_slug).await?;
    if role_id.starts_with("owner:") {
        return Err(AppError::ValidationError(
            "The Owner role cannot be deleted".to_string(),
        ));
    }
    let existing = role::find_role_by_id(pool, &guild.id, role_id)
        .await?
        .ok_or(AppError::NotFound)?;
    if existing.is_default != 0 {
        return Err(AppError::ValidationError(
            "The @everyone role cannot be deleted".to_string(),
        ));
    }

    let removed_assignment_count = role::delete_role_assignments_by_role_id(pool, &existing.id)
        .await?
        .try_into()
        .map_err(|_| AppError::Internal("Role assignment count overflow".to_string()))?;
    let rows = role::delete_custom_role(pool, &existing.id).await?;
    if rows == 0 {
        return Err(AppError::NotFound);
    }
    permissions::invalidate_guild_permission_cache(&guild.id);

    Ok(DeleteRoleResponse {
        deleted_id: existing.id,
        removed_assignment_count,
    })
}

pub async fn reorder_roles(
    pool: &DbPool,
    user_id: &str,
    guild_slug: &str,
    input: ReorderRolesInput,
) -> Result<Vec<RoleResponse>, AppError> {
    let guild = load_guild_with_role_management_access(pool, user_id, guild_slug).await?;
    ensure_default_role(pool, &guild).await?;

    let existing = role::list_roles_by_guild_id(pool, &guild.id).await?;
    let custom_role_ids: HashSet<&str> = existing
        .iter()
        .filter(|role| role.is_default == 0)
        .map(|role| role.id.as_str())
        .collect();
    if input.role_ids.len() != custom_role_ids.len() {
        return Err(AppError::ValidationError(
            "role_ids must include every custom role exactly once".to_string(),
        ));
    }

    let mut incoming = HashSet::new();
    for role_id in &input.role_ids {
        if !custom_role_ids.contains(role_id.as_str()) {
            return Err(AppError::ValidationError(
                "role_ids contains unknown custom role".to_string(),
            ));
        }
        if !incoming.insert(role_id.clone()) {
            return Err(AppError::ValidationError(
                "role_ids contains duplicate roles".to_string(),
            ));
        }
    }

    let updated_at = Utc::now().to_rfc3339();
    role::reorder_custom_roles(pool, &guild.id, &input.role_ids, &updated_at).await?;
    permissions::invalidate_guild_permission_cache(&guild.id);

    let reordered = role::list_roles_by_guild_id(pool, &guild.id).await?;
    let mut response = Vec::with_capacity(reordered.len() + 1);
    response.push(owner_role_response(&guild));
    response.extend(reordered.into_iter().map(to_role_response));
    Ok(response)
}

async fn ensure_default_role(pool: &DbPool, guild: &Guild) -> Result<(), AppError> {
    let existing = role::find_default_role_by_guild_id(pool, &guild.id).await?;
    if existing.is_some() {
        return Ok(());
    }

    let now = Utc::now().to_rfc3339();
    let inserted = role::insert_role(
        pool,
        &format!("role-everyone-{}", guild.id),
        &guild.id,
        EVERYONE_ROLE_NAME,
        DEFAULT_EVERYONE_COLOR,
        DEFAULT_EVERYONE_POSITION,
        permissions::default_everyone_permissions_i64(),
        true,
        &now,
        &now,
    )
    .await?;
    if inserted {
        Ok(())
    } else {
        Err(AppError::Internal(
            "Failed to ensure default @everyone role".to_string(),
        ))
    }
}

async fn load_guild_with_role_management_access(
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
        permissions::MANAGE_ROLES,
        "MANAGE_ROLES",
    )
    .await?;
    Ok(guild)
}

async fn role_name_exists(
    pool: &DbPool,
    guild_id: &str,
    candidate_name: &str,
    exclude_role_id: Option<&str>,
) -> Result<bool, AppError> {
    let candidate_name = candidate_name.to_ascii_lowercase();
    let roles = role::list_roles_by_guild_id(pool, guild_id).await?;
    Ok(roles.into_iter().any(|role| {
        if let Some(exclude_role_id) = exclude_role_id
            && role.id == exclude_role_id
        {
            return false;
        }
        role.name.to_ascii_lowercase() == candidate_name
    }))
}

fn normalize_custom_role_name(value: &str) -> Result<String, AppError> {
    let name = normalize_role_name(value)?;
    if name.eq_ignore_ascii_case(EVERYONE_ROLE_NAME) {
        return Err(AppError::ValidationError(
            "The @everyone role name is reserved".to_string(),
        ));
    }
    Ok(name)
}

fn normalize_role_name(value: &str) -> Result<String, AppError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(AppError::ValidationError("name is required".to_string()));
    }
    if trimmed.chars().count() > MAX_ROLE_NAME_CHARS {
        return Err(AppError::ValidationError(format!(
            "name must be {MAX_ROLE_NAME_CHARS} characters or less"
        )));
    }
    if trimmed.chars().any(|ch| ch.is_control()) {
        return Err(AppError::ValidationError(
            "name contains invalid characters".to_string(),
        ));
    }
    Ok(trimmed.to_string())
}

fn normalize_role_color(value: &str) -> Result<String, AppError> {
    let trimmed = value.trim();
    let bytes = trimmed.as_bytes();
    if bytes.len() != 7 || bytes[0] != b'#' || !bytes[1..].iter().all(|b| b.is_ascii_hexdigit()) {
        return Err(AppError::ValidationError(
            "color must be a hex color like #3399ff".to_string(),
        ));
    }
    Ok(trimmed.to_ascii_lowercase())
}

fn owner_role_response(guild: &Guild) -> RoleResponse {
    RoleResponse {
        id: format!("owner:{}", guild.owner_id),
        name: OWNER_ROLE_NAME.to_string(),
        color: OWNER_ROLE_COLOR.to_string(),
        position: -1,
        permissions_bitflag: permissions::all_permissions_i64(),
        is_default: false,
        is_system: true,
        can_edit: false,
        can_delete: false,
        created_at: guild.created_at.clone(),
    }
}

fn to_role_response(role: Role) -> RoleResponse {
    let is_default = role.is_default != 0;
    RoleResponse {
        id: role.id,
        name: role.name,
        color: role.color,
        position: role.position,
        permissions_bitflag: role.permissions_bitflag,
        is_default,
        is_system: is_default,
        can_edit: !is_default,
        can_delete: !is_default,
        created_at: role.created_at,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_role_color_rejects_invalid_values() {
        assert_eq!(normalize_role_color("#3366FF").unwrap(), "#3366ff");
        assert!(normalize_role_color("3366ff").is_err());
        assert!(normalize_role_color("#zzz999").is_err());
    }
}
