use chrono::Utc;
use serde::Serialize;
use uuid::Uuid;

use crate::{
    AppError,
    db::DbPool,
    models::{
        guild::{self, Guild},
        guild_invite::{self, GuildInviteWithCreator},
    },
};

const MAX_INVITE_CODE_ATTEMPTS: usize = 50;

#[derive(Debug, Clone)]
pub struct CreateGuildInviteInput {
    pub invite_type: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct GuildInviteResponse {
    pub code: String,
    pub r#type: String,
    pub uses_remaining: i64,
    pub created_by: String,
    pub creator_username: String,
    pub created_at: String,
    pub revoked: bool,
    pub invite_url: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct RevokeGuildInviteResponse {
    pub code: String,
    pub revoked: bool,
}

pub async fn list_invites(
    pool: &DbPool,
    user_id: &str,
    guild_slug: &str,
) -> Result<Vec<GuildInviteResponse>, AppError> {
    let guild = load_managed_guild(pool, user_id, guild_slug).await?;
    let invites = guild_invite::list_active_guild_invites(pool, &guild.id).await?;
    Ok(invites.into_iter().map(to_invite_response).collect())
}

pub async fn create_invite(
    pool: &DbPool,
    user_id: &str,
    guild_slug: &str,
    input: CreateGuildInviteInput,
) -> Result<GuildInviteResponse, AppError> {
    let guild = load_managed_guild(pool, user_id, guild_slug).await?;
    let invite_type = normalize_invite_type(&input.invite_type)?;
    let uses_remaining = uses_remaining_for_type(invite_type);

    for _ in 0..MAX_INVITE_CODE_ATTEMPTS {
        let now = Utc::now().to_rfc3339();
        let code = generate_invite_code();
        let inserted = guild_invite::insert_guild_invite(
            pool,
            &Uuid::new_v4().to_string(),
            &guild.id,
            &code,
            invite_type,
            uses_remaining,
            user_id,
            &now,
        )
        .await?;

        if !inserted {
            continue;
        }

        let created = guild_invite::find_invite_with_creator_by_code(pool, &guild.id, &code)
            .await?
            .ok_or_else(|| AppError::Internal("Created invite not found".to_string()))?;
        return Ok(to_invite_response(created));
    }

    Err(AppError::Conflict(
        "Could not generate a unique invite code".to_string(),
    ))
}

pub async fn revoke_invite(
    pool: &DbPool,
    user_id: &str,
    guild_slug: &str,
    code: &str,
) -> Result<RevokeGuildInviteResponse, AppError> {
    let guild = load_managed_guild(pool, user_id, guild_slug).await?;
    let rows = guild_invite::revoke_invite_by_code(pool, &guild.id, code).await?;
    if rows == 0 {
        return Err(AppError::NotFound);
    }
    Ok(RevokeGuildInviteResponse {
        code: code.to_string(),
        revoked: true,
    })
}

fn to_invite_response(record: GuildInviteWithCreator) -> GuildInviteResponse {
    GuildInviteResponse {
        code: record.code.clone(),
        r#type: record.invite_type,
        uses_remaining: record.uses_remaining,
        created_by: record.created_by,
        creator_username: record.creator_username,
        created_at: record.created_at,
        revoked: record.revoked != 0,
        invite_url: format!("/invite/{}", record.code),
    }
}

async fn load_managed_guild(
    pool: &DbPool,
    user_id: &str,
    guild_slug: &str,
) -> Result<Guild, AppError> {
    let guild = guild::find_guild_by_slug(pool, guild_slug)
        .await?
        .ok_or(AppError::NotFound)?;
    // Owner-only for now; Epic 5 permissions can plug in here.
    if guild.owner_id != user_id {
        return Err(AppError::Forbidden(
            "Only guild owners can manage invites".to_string(),
        ));
    }
    Ok(guild)
}

fn normalize_invite_type(value: &str) -> Result<&'static str, AppError> {
    match value.trim().to_ascii_lowercase().as_str() {
        "reusable" => Ok("reusable"),
        "single_use" => Ok("single_use"),
        _ => Err(AppError::ValidationError(
            "type must be one of: reusable, single_use".to_string(),
        )),
    }
}

fn uses_remaining_for_type(invite_type: &str) -> i64 {
    if invite_type == "single_use" { 1 } else { 0 }
}

fn generate_invite_code() -> String {
    Uuid::new_v4().simple().to_string()
}
