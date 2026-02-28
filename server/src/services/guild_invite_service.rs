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
    permissions,
};

const MAX_INVITE_CODE_ATTEMPTS: usize = 50;
pub const INVALID_INVITE_MESSAGE: &str = "This invite link is invalid or has expired";

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

#[derive(Debug, Clone, Serialize)]
pub struct InviteWelcomeScreenResponse {
    pub enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rules: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accept_label: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct InviteMetadataResponse {
    pub code: String,
    pub guild_slug: String,
    pub guild_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub guild_icon_url: Option<String>,
    pub default_channel_slug: String,
    pub welcome_screen: InviteWelcomeScreenResponse,
}

#[derive(Debug, Clone, Serialize)]
pub struct JoinGuildByInviteResponse {
    pub guild_slug: String,
    pub guild_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub guild_icon_url: Option<String>,
    pub default_channel_slug: String,
    pub already_member: bool,
    pub welcome_screen: InviteWelcomeScreenResponse,
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

pub async fn resolve_invite_metadata(
    pool: &DbPool,
    code: &str,
) -> Result<InviteMetadataResponse, AppError> {
    let invite = guild_invite::find_active_invite_with_guild_by_code(pool, code)
        .await?
        .ok_or_else(|| AppError::ValidationError(INVALID_INVITE_MESSAGE.to_string()))?;

    Ok(InviteMetadataResponse {
        code: invite.code,
        guild_slug: invite.guild_slug.clone(),
        guild_name: invite.guild_name,
        guild_icon_url: icon_url_for_guild(
            &invite.guild_slug,
            invite.guild_icon_storage_key.as_deref(),
        ),
        default_channel_slug: invite.guild_default_channel_slug,
        welcome_screen: default_welcome_screen(),
    })
}

pub async fn join_guild_by_invite(
    pool: &DbPool,
    user_id: &str,
    code: &str,
) -> Result<JoinGuildByInviteResponse, AppError> {
    let joined = guild_invite::join_guild_via_invite(pool, code, user_id, &Utc::now().to_rfc3339())
        .await?
        .ok_or_else(|| AppError::ValidationError(INVALID_INVITE_MESSAGE.to_string()))?;

    Ok(JoinGuildByInviteResponse {
        guild_slug: joined.guild_slug.clone(),
        guild_name: joined.guild_name,
        guild_icon_url: icon_url_for_guild(&joined.guild_slug, joined.icon_storage_key.as_deref()),
        default_channel_slug: joined.default_channel_slug,
        already_member: joined.already_member,
        welcome_screen: default_welcome_screen(),
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

fn icon_url_for_guild(guild_slug: &str, icon_storage_key: Option<&str>) -> Option<String> {
    icon_storage_key.map(|_| format!("/api/v1/guilds/{guild_slug}/icon"))
}

fn default_welcome_screen() -> InviteWelcomeScreenResponse {
    InviteWelcomeScreenResponse {
        enabled: false,
        title: None,
        rules: None,
        accept_label: None,
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
    permissions::require_guild_permission(
        pool,
        &guild,
        user_id,
        permissions::MANAGE_INVITES,
        "MANAGE_INVITES",
    )
    .await?;
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
