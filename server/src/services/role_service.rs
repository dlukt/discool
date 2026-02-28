use std::collections::{HashMap, HashSet};

use chrono::Utc;
use serde::Serialize;
use uuid::Uuid;

use crate::{
    AppError,
    db::DbPool,
    models::{
        guild::{self, Guild},
        guild_member::{self, GuildMemberProfile},
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

#[derive(Debug, Clone)]
pub struct UpdateMemberRolesInput {
    pub role_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct GuildMemberResponse {
    pub user_id: String,
    pub username: String,
    pub display_name: String,
    pub avatar_color: Option<String>,
    pub highest_role_color: String,
    pub role_ids: Vec<String>,
    pub is_owner: bool,
    pub can_assign_roles: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct GuildMemberRoleDataResponse {
    pub members: Vec<GuildMemberResponse>,
    pub roles: Vec<RoleResponse>,
    pub assignable_role_ids: Vec<String>,
    pub can_manage_roles: bool,
}

pub async fn list_roles(
    pool: &DbPool,
    user_id: &str,
    guild_slug: &str,
) -> Result<Vec<RoleResponse>, AppError> {
    let guild = load_guild_with_role_assignment_access(pool, user_id, guild_slug).await?;
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
    let guild = load_owned_guild_for_role_definitions(pool, user_id, guild_slug).await?;
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
    let guild = load_owned_guild_for_role_definitions(pool, user_id, guild_slug).await?;
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
    let guild = load_owned_guild_for_role_definitions(pool, user_id, guild_slug).await?;
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
    let guild = load_owned_guild_for_role_definitions(pool, user_id, guild_slug).await?;
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

pub async fn list_guild_members(
    pool: &DbPool,
    user_id: &str,
    guild_slug: &str,
) -> Result<GuildMemberRoleDataResponse, AppError> {
    let guild = load_viewable_guild(pool, user_id, guild_slug).await?;
    ensure_default_role(pool, &guild).await?;
    let roles = role::list_roles_by_guild_id(pool, &guild.id).await?;
    let role_by_id: HashMap<String, Role> = roles
        .iter()
        .cloned()
        .map(|record| (record.id.clone(), record))
        .collect();
    let default_role_color = roles
        .iter()
        .find(|record| record.is_default != 0)
        .map(|record| record.color.clone())
        .ok_or_else(|| AppError::Internal("Default role not found".to_string()))?;

    let mut members = guild_member::list_guild_member_profiles(pool, &guild.id).await?;
    if !members
        .iter()
        .any(|member| member.user_id == guild.owner_id)
        && let Some(owner_profile) =
            guild_member::find_user_profile_by_id(pool, &guild.owner_id).await?
    {
        members.insert(
            0,
            GuildMemberProfile {
                user_id: owner_profile.user_id,
                username: owner_profile.username,
                display_name: owner_profile.display_name,
                avatar_color: owner_profile.avatar_color,
                joined_at: guild.created_at.clone(),
            },
        );
    }

    let assignments = role::list_role_assignments_by_guild_id(pool, &guild.id).await?;
    let mut role_ids_by_user: HashMap<String, Vec<String>> = HashMap::new();
    for assignment in assignments {
        role_ids_by_user
            .entry(assignment.user_id)
            .or_default()
            .push(assignment.role_id);
    }
    for role_ids in role_ids_by_user.values_mut() {
        sort_role_ids_by_authority(role_ids, &role_by_id);
    }

    let can_manage_roles = actor_can_manage_roles(pool, &guild, user_id).await?;
    let actor_authority = if can_manage_roles {
        permissions::highest_guild_role_authority(pool, &guild, user_id).await?
    } else {
        None
    };
    let assignable_role_ids = collect_assignable_custom_role_ids(&roles, actor_authority);

    let mut member_responses = Vec::with_capacity(members.len());
    for member in members {
        let role_ids = role_ids_by_user.remove(&member.user_id).unwrap_or_default();
        let can_assign_roles = if can_manage_roles {
            can_actor_assign_member(pool, &guild, user_id, &member.user_id, actor_authority).await?
        } else {
            false
        };
        member_responses.push(build_guild_member_response(
            member,
            &guild,
            &default_role_color,
            &role_by_id,
            role_ids,
            can_assign_roles,
        ));
    }

    let mut role_responses = Vec::with_capacity(roles.len() + 1);
    role_responses.push(owner_role_response(&guild));
    role_responses.extend(roles.into_iter().map(to_role_response));

    Ok(GuildMemberRoleDataResponse {
        members: member_responses,
        roles: role_responses,
        assignable_role_ids,
        can_manage_roles,
    })
}

pub async fn update_member_roles(
    pool: &DbPool,
    user_id: &str,
    guild_slug: &str,
    target_user_id: &str,
    input: UpdateMemberRolesInput,
) -> Result<GuildMemberResponse, AppError> {
    let guild = load_guild_with_role_assignment_access(pool, user_id, guild_slug).await?;
    ensure_default_role(pool, &guild).await?;

    if !guild_member::is_guild_member(pool, &guild.id, target_user_id).await? {
        return Err(AppError::ValidationError(
            "target user is not a guild member".to_string(),
        ));
    }
    if target_user_id == guild.owner_id {
        return Err(AppError::ValidationError(
            "Guild owner role assignments cannot be modified".to_string(),
        ));
    }

    let actor_authority = permissions::highest_guild_role_authority(pool, &guild, user_id)
        .await?
        .ok_or_else(|| {
            AppError::Forbidden("Missing MANAGE_ROLES permission in this guild".to_string())
        })?;
    if user_id != guild.owner_id
        && !permissions::actor_outranks_target_member(pool, &guild, user_id, target_user_id).await?
    {
        return Err(AppError::Forbidden(
            "You can only assign roles to members below your highest role".to_string(),
        ));
    }

    let normalized_role_ids = normalize_role_ids(&input.role_ids)?;
    let role_ids_before_assignment =
        role::list_assigned_role_ids(pool, &guild.id, target_user_id).await?;
    let roles = role::list_roles_by_guild_id(pool, &guild.id).await?;
    let role_by_id: HashMap<String, Role> = roles
        .iter()
        .cloned()
        .map(|record| (record.id.clone(), record))
        .collect();
    let default_role_color = roles
        .iter()
        .find(|record| record.is_default != 0)
        .map(|record| record.color.clone())
        .ok_or_else(|| AppError::Internal("Default role not found".to_string()))?;

    for role_id in &normalized_role_ids {
        if role_id.starts_with("owner:") {
            return Err(AppError::ValidationError(
                "owner role cannot be assigned".to_string(),
            ));
        }
        let Some(candidate_role) = role_by_id.get(role_id) else {
            return Err(AppError::ValidationError(
                "role_ids contains unknown role".to_string(),
            ));
        };
        if candidate_role.is_default != 0 {
            return Err(AppError::ValidationError(
                "@everyone cannot be assigned manually".to_string(),
            ));
        }
        if !role_is_assignable_to_actor(actor_authority, candidate_role) {
            return Err(AppError::Forbidden(
                "Cannot assign roles equal to or above your highest role".to_string(),
            ));
        }
    }

    let assigned_at = Utc::now().to_rfc3339();
    role::set_role_assignments_for_user(
        pool,
        &guild.id,
        target_user_id,
        &normalized_role_ids,
        &assigned_at,
    )
    .await?;
    permissions::invalidate_guild_permission_cache(&guild.id);
    if user_id != guild.owner_id {
        let current_role_ids: HashSet<String> =
            role_ids_before_assignment.iter().cloned().collect();
        let next_role_ids: HashSet<String> = normalized_role_ids.iter().cloned().collect();
        let added_role_ids: Vec<String> = normalized_role_ids
            .iter()
            .filter(|role_id| !current_role_ids.contains(*role_id))
            .cloned()
            .collect();
        let removed_role_ids: Vec<String> = role_ids_before_assignment
            .iter()
            .filter(|role_id| !next_role_ids.contains(*role_id))
            .cloned()
            .collect();
        tracing::info!(
            guild_id = %guild.id,
            guild_slug = %guild.slug,
            actor_user_id = %user_id,
            target_user_id = %target_user_id,
            added_role_ids = ?added_role_ids,
            removed_role_ids = ?removed_role_ids,
            "Delegated role assignment updated"
        );
    }

    let target_member = guild_member::find_guild_member_profile(pool, &guild.id, target_user_id)
        .await?
        .ok_or_else(|| {
            AppError::ValidationError("target user is not a guild member".to_string())
        })?;
    let assigned_role_ids = role::list_assigned_role_ids(pool, &guild.id, target_user_id).await?;
    Ok(build_guild_member_response(
        target_member,
        &guild,
        &default_role_color,
        &role_by_id,
        assigned_role_ids,
        can_actor_assign_member(pool, &guild, user_id, target_user_id, Some(actor_authority))
            .await?,
    ))
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

async fn load_owned_guild_for_role_definitions(
    pool: &DbPool,
    user_id: &str,
    guild_slug: &str,
) -> Result<Guild, AppError> {
    let guild = guild::find_guild_by_slug(pool, guild_slug)
        .await?
        .ok_or(AppError::NotFound)?;
    if guild.owner_id != user_id {
        return Err(AppError::Forbidden(
            "Only guild owners can manage role definitions".to_string(),
        ));
    }
    Ok(guild)
}

async fn load_guild_with_role_assignment_access(
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
        "Only guild members can view member list".to_string(),
    ))
}

async fn actor_can_manage_roles(
    pool: &DbPool,
    guild: &Guild,
    user_id: &str,
) -> Result<bool, AppError> {
    if user_id == guild.owner_id {
        return Ok(true);
    }
    let effective = permissions::effective_guild_permissions(pool, guild, user_id).await?;
    Ok(permissions::has_permission(
        effective,
        permissions::MANAGE_ROLES,
    ))
}

async fn can_actor_assign_member(
    pool: &DbPool,
    guild: &Guild,
    actor_user_id: &str,
    target_user_id: &str,
    actor_authority: Option<permissions::RoleAuthority>,
) -> Result<bool, AppError> {
    if actor_user_id == target_user_id {
        return Ok(false);
    }
    if target_user_id == guild.owner_id {
        return Ok(false);
    }
    if actor_user_id == guild.owner_id {
        return guild_member::is_guild_member(pool, &guild.id, target_user_id).await;
    }
    let Some(_) = actor_authority else {
        return Ok(false);
    };
    permissions::actor_outranks_target_member(pool, guild, actor_user_id, target_user_id).await
}

fn role_is_assignable_to_actor(
    authority: permissions::RoleAuthority,
    candidate_role: &Role,
) -> bool {
    if candidate_role.is_default != 0 {
        return false;
    }
    match authority {
        permissions::RoleAuthority::Owner => true,
        permissions::RoleAuthority::Position(position) => candidate_role.position > position,
    }
}

fn collect_assignable_custom_role_ids(
    roles: &[Role],
    actor_authority: Option<permissions::RoleAuthority>,
) -> Vec<String> {
    let Some(actor_authority) = actor_authority else {
        return Vec::new();
    };
    roles
        .iter()
        .filter(|record| role_is_assignable_to_actor(actor_authority, record))
        .map(|record| record.id.clone())
        .collect()
}

fn normalize_role_ids(role_ids: &[String]) -> Result<Vec<String>, AppError> {
    let mut seen = HashSet::new();
    let mut normalized = Vec::with_capacity(role_ids.len());
    for role_id in role_ids {
        let trimmed = role_id.trim();
        if trimmed.is_empty() {
            return Err(AppError::ValidationError(
                "role_ids contains invalid role id".to_string(),
            ));
        }
        let candidate = trimmed.to_string();
        if !seen.insert(candidate.clone()) {
            return Err(AppError::ValidationError(
                "role_ids contains duplicate roles".to_string(),
            ));
        }
        normalized.push(candidate);
    }
    Ok(normalized)
}

fn sort_role_ids_by_authority(role_ids: &mut [String], role_by_id: &HashMap<String, Role>) {
    role_ids.sort_by(|left, right| {
        let left_position = role_by_id
            .get(left)
            .map(|role| role.position)
            .unwrap_or(i64::MAX);
        let right_position = role_by_id
            .get(right)
            .map(|role| role.position)
            .unwrap_or(i64::MAX);
        left_position
            .cmp(&right_position)
            .then_with(|| left.cmp(right))
    });
}

fn normalize_member_display_name(display_name: Option<&str>, username: &str) -> String {
    display_name
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(username)
        .to_string()
}

fn resolve_highest_role_color(
    default_role_color: &str,
    role_by_id: &HashMap<String, Role>,
    assigned_role_ids: &[String],
    is_owner: bool,
) -> String {
    if is_owner {
        return OWNER_ROLE_COLOR.to_string();
    }
    for role_id in assigned_role_ids {
        if let Some(role) = role_by_id.get(role_id) {
            return role.color.clone();
        }
    }
    if let Some(default_role) = role_by_id.values().find(|role| role.is_default != 0) {
        return default_role.color.clone();
    }
    default_role_color.to_string()
}

fn build_guild_member_response(
    member: GuildMemberProfile,
    guild: &Guild,
    default_role_color: &str,
    role_by_id: &HashMap<String, Role>,
    mut role_ids: Vec<String>,
    can_assign_roles: bool,
) -> GuildMemberResponse {
    sort_role_ids_by_authority(&mut role_ids, role_by_id);
    let is_owner = member.user_id == guild.owner_id;
    GuildMemberResponse {
        user_id: member.user_id,
        username: member.username.clone(),
        display_name: normalize_member_display_name(
            member.display_name.as_deref(),
            &member.username,
        ),
        avatar_color: member.avatar_color,
        highest_role_color: resolve_highest_role_color(
            default_role_color,
            role_by_id,
            &role_ids,
            is_owner,
        ),
        role_ids,
        is_owner,
        can_assign_roles,
    }
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

    #[test]
    fn collect_assignable_custom_roles_excludes_everyone_and_higher_roles() {
        let roles = vec![
            Role {
                id: "role-a".to_string(),
                guild_id: "guild-1".to_string(),
                name: "A".to_string(),
                color: "#111111".to_string(),
                position: 0,
                permissions_bitflag: 0,
                is_default: 0,
                created_at: "2026-02-28T00:00:00.000Z".to_string(),
                updated_at: "2026-02-28T00:00:00.000Z".to_string(),
            },
            Role {
                id: "role-b".to_string(),
                guild_id: "guild-1".to_string(),
                name: "B".to_string(),
                color: "#222222".to_string(),
                position: 1,
                permissions_bitflag: 0,
                is_default: 0,
                created_at: "2026-02-28T00:00:00.000Z".to_string(),
                updated_at: "2026-02-28T00:00:00.000Z".to_string(),
            },
            Role {
                id: "role-everyone".to_string(),
                guild_id: "guild-1".to_string(),
                name: "@everyone".to_string(),
                color: "#99aab5".to_string(),
                position: 2_147_483_647,
                permissions_bitflag: 5633,
                is_default: 1,
                created_at: "2026-02-28T00:00:00.000Z".to_string(),
                updated_at: "2026-02-28T00:00:00.000Z".to_string(),
            },
        ];

        let owner_assignable =
            collect_assignable_custom_role_ids(&roles, Some(permissions::RoleAuthority::Owner));
        assert_eq!(owner_assignable, vec!["role-a", "role-b"]);

        let manager_assignable = collect_assignable_custom_role_ids(
            &roles,
            Some(permissions::RoleAuthority::Position(1)),
        );
        assert!(manager_assignable.is_empty());

        let helper_assignable = collect_assignable_custom_role_ids(
            &roles,
            Some(permissions::RoleAuthority::Position(0)),
        );
        assert_eq!(helper_assignable, vec!["role-b"]);
    }

    #[test]
    fn normalize_role_ids_rejects_duplicates_and_blanks() {
        assert!(normalize_role_ids(&["".to_string()]).is_err());
        assert!(normalize_role_ids(&["role-1".to_string(), "role-1".to_string()]).is_err());
        assert_eq!(
            normalize_role_ids(&[" role-1 ".to_string(), "role-2".to_string()]).unwrap(),
            vec!["role-1".to_string(), "role-2".to_string()]
        );
    }
}
