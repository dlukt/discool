use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::{DateTime, Duration, Utc};
use serde::Serialize;
use uuid::Uuid;

#[cfg(test)]
use crate::models::role;
use crate::{
    AppError,
    config::AttachmentConfig,
    db::DbPool,
    models::{
        channel,
        guild::{self, Guild},
        guild_ban, guild_member, message, message_attachment, moderation,
    },
    permissions,
    services::file_storage_service::FileStorageProvider,
};

const MAX_MUTE_REASON_CHARS: usize = 500;
const MAX_MUTE_DURATION_SECONDS: i64 = 315_360_000; // 10 years
const DEFAULT_MODERATION_LOG_LIMIT: i64 = 50;
const MAX_MODERATION_LOG_LIMIT: i64 = 200;

#[derive(Debug, Clone)]
pub struct CreateMuteInput {
    pub target_user_id: String,
    pub reason: String,
    pub duration_seconds: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct CreateKickInput {
    pub target_user_id: String,
    pub reason: String,
}

#[derive(Debug, Clone)]
pub struct CreateVoiceKickInput {
    pub target_user_id: String,
    pub reason: String,
    pub channel_slug: String,
}

#[derive(Debug, Clone)]
pub struct CreateBanInput {
    pub target_user_id: String,
    pub reason: String,
    pub delete_message_window: String,
}

#[derive(Debug, Clone)]
pub struct CreateMessageDeleteInput {
    pub message_id: String,
    pub channel_slug: String,
    pub reason: String,
}

#[derive(Debug, Clone)]
pub struct CreateMessageReportInput {
    pub message_id: String,
    pub reason: String,
    pub category: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CreateUserReportInput {
    pub target_user_id: String,
    pub reason: String,
    pub category: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ListModerationLogInput {
    pub limit: Option<String>,
    pub cursor: Option<String>,
    pub order: Option<String>,
    pub action_type: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ListUserMessageHistoryInput {
    pub target_user_id: String,
    pub limit: Option<String>,
    pub cursor: Option<String>,
    pub channel_slug: Option<String>,
    pub from: Option<String>,
    pub to: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MuteActionResponse {
    pub id: String,
    pub guild_slug: String,
    pub actor_user_id: String,
    pub target_user_id: String,
    pub reason: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_seconds: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<String>,
    pub is_permanent: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct MuteStatusResponse {
    pub active: bool,
    pub is_permanent: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct KickActionResponse {
    pub id: String,
    pub guild_slug: String,
    pub actor_user_id: String,
    pub target_user_id: String,
    pub reason: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct VoiceKickActionResponse {
    pub id: String,
    pub guild_slug: String,
    pub channel_slug: String,
    pub actor_user_id: String,
    pub target_user_id: String,
    pub reason: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct MessageDeleteActionResponse {
    pub id: String,
    pub message_id: String,
    pub guild_slug: String,
    pub channel_slug: String,
    pub actor_user_id: String,
    pub target_user_id: String,
    pub reason: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct UserContentReportResponse {
    pub id: String,
    pub guild_slug: String,
    pub reporter_user_id: String,
    pub target_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_message_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_user_id: Option<String>,
    pub reason: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct BanActionResponse {
    pub id: String,
    pub ban_id: String,
    pub guild_slug: String,
    pub actor_user_id: String,
    pub target_user_id: String,
    pub reason: String,
    pub delete_message_window: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delete_messages_window_seconds: Option<i64>,
    pub deleted_messages_count: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct GuildBanResponse {
    pub id: String,
    pub target_user_id: String,
    pub target_username: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_display_name: Option<String>,
    pub actor_user_id: String,
    pub actor_username: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actor_display_name: Option<String>,
    pub reason: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delete_messages_window_seconds: Option<i64>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct UnbanActionResponse {
    pub id: String,
    pub guild_slug: String,
    pub target_user_id: String,
    pub unbanned_by_user_id: String,
    pub unbanned_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ModerationLogEntryResponse {
    pub id: String,
    pub action_type: String,
    pub reason: String,
    pub created_at: String,
    pub actor_user_id: String,
    pub actor_username: String,
    pub actor_display_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actor_avatar_color: Option<String>,
    pub target_user_id: String,
    pub target_username: String,
    pub target_display_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_avatar_color: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ListModerationLogResult {
    pub entries: Vec<ModerationLogEntryResponse>,
    pub cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct UserMessageHistoryEntryResponse {
    pub id: String,
    pub channel_slug: String,
    pub channel_name: String,
    pub content: String,
    pub created_at: String,
}

#[derive(Debug, Clone)]
pub struct ListUserMessageHistoryResult {
    pub entries: Vec<UserMessageHistoryEntryResponse>,
    pub cursor: Option<String>,
}

pub async fn create_mute(
    pool: &DbPool,
    actor_user_id: &str,
    guild_slug: &str,
    input: CreateMuteInput,
) -> Result<MuteActionResponse, AppError> {
    let actor_user_id = normalize_id(actor_user_id, "actor_user_id")?;
    let guild_slug = normalize_slug(guild_slug, "guild_slug")?;
    let target_user_id = normalize_id(&input.target_user_id, "target_user_id")?;
    if actor_user_id == target_user_id {
        return Err(AppError::ValidationError(
            "Cannot mute yourself".to_string(),
        ));
    }
    let reason = normalize_reason(&input.reason)?;
    let duration_seconds = normalize_duration_seconds(input.duration_seconds)?;

    let guild = guild::find_guild_by_slug(pool, &guild_slug)
        .await?
        .ok_or(AppError::NotFound)?;
    permissions::require_guild_permission(
        pool,
        &guild,
        &actor_user_id,
        permissions::MUTE_MEMBERS,
        "MUTE_MEMBERS",
    )
    .await?;

    if !permissions::actor_outranks_target_member(pool, &guild, &actor_user_id, &target_user_id)
        .await?
    {
        return Err(AppError::Forbidden(
            "You can only mute members below your highest role".to_string(),
        ));
    }

    let now = Utc::now();
    let now_str = now.to_rfc3339();
    let expires_at = duration_seconds
        .map(|seconds| now + Duration::seconds(seconds))
        .map(|dt| dt.to_rfc3339());
    moderation::deactivate_active_mutes_for_target(pool, &guild.id, &target_user_id, &now_str)
        .await?;

    let id = Uuid::new_v4().to_string();
    moderation::insert_moderation_action(
        pool,
        &id,
        moderation::MODERATION_ACTION_TYPE_MUTE,
        &guild.id,
        &actor_user_id,
        &target_user_id,
        &reason,
        duration_seconds,
        expires_at.as_deref(),
        true,
        &now_str,
        &now_str,
    )
    .await?;

    Ok(MuteActionResponse {
        id,
        guild_slug: guild.slug,
        actor_user_id,
        target_user_id,
        reason,
        duration_seconds,
        expires_at,
        is_permanent: duration_seconds.is_none(),
        created_at: now_str.clone(),
        updated_at: now_str,
    })
}

pub async fn create_kick(
    pool: &DbPool,
    actor_user_id: &str,
    guild_slug: &str,
    input: CreateKickInput,
) -> Result<KickActionResponse, AppError> {
    let actor_user_id = normalize_id(actor_user_id, "actor_user_id")?;
    let guild_slug = normalize_slug(guild_slug, "guild_slug")?;
    let target_user_id = normalize_id(&input.target_user_id, "target_user_id")?;
    if actor_user_id == target_user_id {
        return Err(AppError::ValidationError(
            "Cannot kick yourself".to_string(),
        ));
    }
    let reason = normalize_reason(&input.reason)?;

    let guild = guild::find_guild_by_slug(pool, &guild_slug)
        .await?
        .ok_or(AppError::NotFound)?;
    permissions::require_guild_permission(
        pool,
        &guild,
        &actor_user_id,
        permissions::KICK_MEMBERS,
        "KICK_MEMBERS",
    )
    .await?;

    if target_user_id == guild.owner_id {
        return Err(AppError::Forbidden(
            "Cannot kick the guild owner".to_string(),
        ));
    }

    let now = Utc::now();
    let now_str = now.to_rfc3339();
    let id = Uuid::new_v4().to_string();
    moderation::apply_kick_action(
        pool,
        &id,
        &guild.id,
        &actor_user_id,
        &target_user_id,
        &reason,
        &now_str,
    )
    .await?;

    Ok(KickActionResponse {
        id,
        guild_slug: guild.slug,
        actor_user_id,
        target_user_id,
        reason,
        created_at: now_str.clone(),
        updated_at: now_str,
    })
}

pub async fn create_voice_kick(
    pool: &DbPool,
    actor_user_id: &str,
    guild_slug: &str,
    input: CreateVoiceKickInput,
) -> Result<VoiceKickActionResponse, AppError> {
    let actor_user_id = normalize_id(actor_user_id, "actor_user_id")?;
    let guild_slug = normalize_slug(guild_slug, "guild_slug")?;
    let target_user_id = normalize_id(&input.target_user_id, "target_user_id")?;
    if actor_user_id == target_user_id {
        return Err(AppError::ValidationError(
            "Cannot kick yourself from voice".to_string(),
        ));
    }
    let reason = normalize_reason(&input.reason)?;
    let channel_slug = normalize_slug(&input.channel_slug, "channel_slug")?;

    let guild = guild::find_guild_by_slug(pool, &guild_slug)
        .await?
        .ok_or(AppError::NotFound)?;
    permissions::require_guild_permission(
        pool,
        &guild,
        &actor_user_id,
        permissions::MUTE_MEMBERS,
        "MUTE_MEMBERS",
    )
    .await?;

    if target_user_id == guild.owner_id {
        return Err(AppError::Forbidden(
            "Cannot kick the guild owner from voice".to_string(),
        ));
    }
    if !permissions::actor_outranks_target_member(pool, &guild, &actor_user_id, &target_user_id)
        .await?
    {
        return Err(AppError::Forbidden(
            "You can only kick members below your highest role".to_string(),
        ));
    }

    let channel = channel::find_channel_by_slug(pool, &guild.id, &channel_slug)
        .await?
        .ok_or(AppError::NotFound)?;
    if channel.channel_type != "voice" {
        return Err(AppError::ValidationError(
            "channel_slug must reference a voice channel".to_string(),
        ));
    }
    if !guild_member::is_guild_member(pool, &guild.id, &target_user_id).await? {
        return Err(AppError::ValidationError(
            "target_user_id must belong to a guild member".to_string(),
        ));
    }

    let now = Utc::now();
    let now_str = now.to_rfc3339();
    let id = Uuid::new_v4().to_string();
    moderation::insert_moderation_action(
        pool,
        &id,
        moderation::MODERATION_ACTION_TYPE_VOICE_KICK,
        &guild.id,
        &actor_user_id,
        &target_user_id,
        &reason,
        None,
        None,
        false,
        &now_str,
        &now_str,
    )
    .await?;

    Ok(VoiceKickActionResponse {
        id,
        guild_slug: guild.slug,
        channel_slug,
        actor_user_id,
        target_user_id,
        reason,
        created_at: now_str.clone(),
        updated_at: now_str,
    })
}

pub async fn create_message_delete(
    pool: &DbPool,
    actor_user_id: &str,
    guild_slug: &str,
    input: CreateMessageDeleteInput,
) -> Result<MessageDeleteActionResponse, AppError> {
    let actor_user_id = normalize_id(actor_user_id, "actor_user_id")?;
    let guild_slug = normalize_slug(guild_slug, "guild_slug")?;
    let message_id = normalize_id(&input.message_id, "message_id")?;
    let channel_slug = normalize_slug(&input.channel_slug, "channel_slug")?;
    let reason = normalize_reason(&input.reason)?;

    let guild = guild::find_guild_by_slug(pool, &guild_slug)
        .await?
        .ok_or(AppError::NotFound)?;
    permissions::require_guild_permission(
        pool,
        &guild,
        &actor_user_id,
        permissions::MANAGE_MESSAGES,
        "MANAGE_MESSAGES",
    )
    .await?;

    let channel = channel::find_channel_by_slug(pool, &guild.id, &channel_slug)
        .await?
        .ok_or(AppError::NotFound)?;
    let existing = message::find_message_by_id(pool, &message_id)
        .await?
        .ok_or(AppError::NotFound)?;
    if existing.guild_id != guild.id || existing.channel_id != channel.id {
        return Err(AppError::NotFound);
    }
    let target_user_id = existing.author_user_id;

    if target_user_id == guild.owner_id && actor_user_id != guild.owner_id {
        return Err(AppError::Forbidden(
            "Cannot delete messages from the guild owner".to_string(),
        ));
    }
    if actor_user_id != guild.owner_id && actor_user_id != target_user_id {
        let target_is_member =
            guild_member::is_guild_member(pool, &guild.id, &target_user_id).await?;
        if target_is_member
            && !permissions::actor_outranks_target_member(
                pool,
                &guild,
                &actor_user_id,
                &target_user_id,
            )
            .await?
        {
            return Err(AppError::Forbidden(
                "You can only delete messages from members below your highest role".to_string(),
            ));
        }
    }

    let now_str = Utc::now().to_rfc3339();
    let id = Uuid::new_v4().to_string();
    let applied = moderation::apply_message_delete_action(
        pool,
        &id,
        &guild.id,
        &channel.id,
        &message_id,
        &actor_user_id,
        &target_user_id,
        &reason,
        &now_str,
    )
    .await?;
    if !applied {
        return Err(AppError::NotFound);
    }

    Ok(MessageDeleteActionResponse {
        id,
        message_id,
        guild_slug: guild.slug,
        channel_slug: channel.slug,
        actor_user_id,
        target_user_id,
        reason,
        created_at: now_str.clone(),
        updated_at: now_str,
    })
}

pub async fn create_message_report(
    pool: &DbPool,
    reporter_user_id: &str,
    guild_slug: &str,
    input: CreateMessageReportInput,
) -> Result<UserContentReportResponse, AppError> {
    let reporter_user_id = normalize_id(reporter_user_id, "reporter_user_id")?;
    let guild_slug = normalize_slug(guild_slug, "guild_slug")?;
    let message_id = normalize_id(&input.message_id, "message_id")?;
    let reason = normalize_reason(&input.reason)?;
    let category = normalize_report_category(input.category.as_deref())?;

    let guild = guild::find_guild_by_slug(pool, &guild_slug)
        .await?
        .ok_or(AppError::NotFound)?;
    if !guild_member::is_guild_member(pool, &guild.id, &reporter_user_id).await? {
        return Err(AppError::Forbidden(
            "Only guild members can submit reports in this guild".to_string(),
        ));
    }

    let message = message::find_message_by_id(pool, &message_id)
        .await?
        .ok_or(AppError::NotFound)?;
    if message.guild_id != guild.id {
        return Err(AppError::NotFound);
    }
    if message.author_user_id == reporter_user_id {
        return Err(AppError::ValidationError(
            "Cannot report your own message".to_string(),
        ));
    }

    let now_str = Utc::now().to_rfc3339();
    let id = Uuid::new_v4().to_string();
    moderation::insert_report(
        pool,
        &id,
        &guild.id,
        &reporter_user_id,
        moderation::REPORT_TARGET_TYPE_MESSAGE,
        Some(&message_id),
        None,
        &reason,
        category.as_deref(),
        moderation::REPORT_STATUS_PENDING,
        &now_str,
        &now_str,
    )
    .await?;

    Ok(UserContentReportResponse {
        id,
        guild_slug: guild.slug,
        reporter_user_id,
        target_type: moderation::REPORT_TARGET_TYPE_MESSAGE.to_string(),
        target_message_id: Some(message_id),
        target_user_id: None,
        reason,
        category,
        status: moderation::REPORT_STATUS_PENDING.to_string(),
        created_at: now_str.clone(),
        updated_at: now_str,
    })
}

pub async fn create_user_report(
    pool: &DbPool,
    reporter_user_id: &str,
    guild_slug: &str,
    input: CreateUserReportInput,
) -> Result<UserContentReportResponse, AppError> {
    let reporter_user_id = normalize_id(reporter_user_id, "reporter_user_id")?;
    let guild_slug = normalize_slug(guild_slug, "guild_slug")?;
    let target_user_id = normalize_id(&input.target_user_id, "target_user_id")?;
    if reporter_user_id == target_user_id {
        return Err(AppError::ValidationError(
            "Cannot report your own user account".to_string(),
        ));
    }
    let reason = normalize_reason(&input.reason)?;
    let category = normalize_report_category(input.category.as_deref())?;

    let guild = guild::find_guild_by_slug(pool, &guild_slug)
        .await?
        .ok_or(AppError::NotFound)?;
    if !guild_member::is_guild_member(pool, &guild.id, &reporter_user_id).await? {
        return Err(AppError::Forbidden(
            "Only guild members can submit reports in this guild".to_string(),
        ));
    }
    if !guild_member::is_guild_member(pool, &guild.id, &target_user_id).await? {
        return Err(AppError::ValidationError(
            "target_user_id must belong to a guild member".to_string(),
        ));
    }

    let now_str = Utc::now().to_rfc3339();
    let id = Uuid::new_v4().to_string();
    moderation::insert_report(
        pool,
        &id,
        &guild.id,
        &reporter_user_id,
        moderation::REPORT_TARGET_TYPE_USER,
        None,
        Some(&target_user_id),
        &reason,
        category.as_deref(),
        moderation::REPORT_STATUS_PENDING,
        &now_str,
        &now_str,
    )
    .await?;

    Ok(UserContentReportResponse {
        id,
        guild_slug: guild.slug,
        reporter_user_id,
        target_type: moderation::REPORT_TARGET_TYPE_USER.to_string(),
        target_message_id: None,
        target_user_id: Some(target_user_id),
        reason,
        category,
        status: moderation::REPORT_STATUS_PENDING.to_string(),
        created_at: now_str.clone(),
        updated_at: now_str,
    })
}

pub async fn create_ban(
    pool: &DbPool,
    attachment_config: &AttachmentConfig,
    actor_user_id: &str,
    guild_slug: &str,
    input: CreateBanInput,
) -> Result<BanActionResponse, AppError> {
    let actor_user_id = normalize_id(actor_user_id, "actor_user_id")?;
    let guild_slug = normalize_slug(guild_slug, "guild_slug")?;
    let target_user_id = normalize_id(&input.target_user_id, "target_user_id")?;
    if actor_user_id == target_user_id {
        return Err(AppError::ValidationError("Cannot ban yourself".to_string()));
    }
    let reason = normalize_reason(&input.reason)?;
    let (delete_message_window, delete_messages_window_seconds) =
        normalize_delete_message_window(&input.delete_message_window)?;

    let guild = guild::find_guild_by_slug(pool, &guild_slug)
        .await?
        .ok_or(AppError::NotFound)?;
    permissions::require_guild_permission(
        pool,
        &guild,
        &actor_user_id,
        permissions::BAN_MEMBERS,
        "BAN_MEMBERS",
    )
    .await?;

    if target_user_id == guild.owner_id {
        return Err(AppError::Forbidden(
            "Cannot ban the guild owner".to_string(),
        ));
    }

    let now = Utc::now();
    let now_str = now.to_rfc3339();
    let id = Uuid::new_v4().to_string();
    let ban_id = Uuid::new_v4().to_string();
    moderation::apply_ban_action(
        pool,
        &id,
        &ban_id,
        &guild.id,
        &actor_user_id,
        &target_user_id,
        &reason,
        delete_messages_window_seconds,
        &now_str,
    )
    .await?;

    let deleted_messages_count = match delete_messages_window_seconds {
        Some(seconds) => {
            let delete_since = (now - Duration::seconds(seconds)).to_rfc3339();
            match delete_recent_messages_for_banned_user(
                pool,
                attachment_config,
                &guild.id,
                &target_user_id,
                &delete_since,
            )
            .await
            {
                Ok(count) => count,
                Err(err) => {
                    tracing::warn!(
                        error = ?err,
                        guild_id = %guild.id,
                        target_user_id = %target_user_id,
                        "Ban committed but recent-message cleanup failed"
                    );
                    0
                }
            }
        }
        None => 0,
    };

    Ok(BanActionResponse {
        id,
        ban_id,
        guild_slug: guild.slug,
        actor_user_id,
        target_user_id,
        reason,
        delete_message_window: delete_message_window.to_string(),
        delete_messages_window_seconds,
        deleted_messages_count,
        created_at: now_str.clone(),
        updated_at: now_str,
    })
}

pub async fn list_bans(
    pool: &DbPool,
    actor_user_id: &str,
    guild_slug: &str,
) -> Result<Vec<GuildBanResponse>, AppError> {
    let actor_user_id = normalize_id(actor_user_id, "actor_user_id")?;
    let guild_slug = normalize_slug(guild_slug, "guild_slug")?;

    let guild = guild::find_guild_by_slug(pool, &guild_slug)
        .await?
        .ok_or(AppError::NotFound)?;
    permissions::require_guild_permission(
        pool,
        &guild,
        &actor_user_id,
        permissions::BAN_MEMBERS,
        "BAN_MEMBERS",
    )
    .await?;

    let bans = guild_ban::list_active_guild_bans_for_guild(pool, &guild.id).await?;
    Ok(bans
        .into_iter()
        .map(|ban| GuildBanResponse {
            id: ban.id,
            target_user_id: ban.target_user_id,
            target_username: ban.target_username,
            target_display_name: ban.target_display_name,
            actor_user_id: ban.actor_user_id,
            actor_username: ban.actor_username,
            actor_display_name: ban.actor_display_name,
            reason: ban.reason,
            delete_messages_window_seconds: ban.delete_messages_window_seconds,
            created_at: ban.created_at,
        })
        .collect())
}

pub async fn list_moderation_log(
    pool: &DbPool,
    actor_user_id: &str,
    guild_slug: &str,
    input: ListModerationLogInput,
) -> Result<ListModerationLogResult, AppError> {
    let actor_user_id = normalize_id(actor_user_id, "actor_user_id")?;
    let guild_slug = normalize_slug(guild_slug, "guild_slug")?;
    let limit = normalize_moderation_log_limit(input.limit.as_deref())?;
    let order = normalize_moderation_log_order(input.order.as_deref())?;
    let action_type = normalize_moderation_log_action_type(input.action_type.as_deref())?;
    let cursor = decode_moderation_log_cursor(input.cursor.as_deref())?;

    let guild = guild::find_guild_by_slug(pool, &guild_slug)
        .await?
        .ok_or(AppError::NotFound)?;
    permissions::require_guild_permission(
        pool,
        &guild,
        &actor_user_id,
        permissions::VIEW_MOD_LOG,
        "VIEW_MOD_LOG",
    )
    .await?;

    let page = moderation::list_moderation_log_page_by_guild_id(
        pool,
        &guild.id,
        action_type.as_deref(),
        cursor.as_ref(),
        limit,
        order,
    )
    .await?;
    let next_cursor = if page.has_more {
        page.entries.last().map(|entry| {
            encode_moderation_log_cursor(&moderation::ModerationLogCursor {
                created_at: entry.created_at.clone(),
                id: entry.id.clone(),
            })
        })
    } else {
        None
    };

    let entries = page
        .entries
        .into_iter()
        .map(|entry| ModerationLogEntryResponse {
            id: entry.id,
            action_type: entry.action_type,
            reason: entry.reason,
            created_at: entry.created_at,
            actor_user_id: entry.actor_user_id,
            actor_username: entry.actor_username.clone(),
            actor_display_name: profile_display_name(
                entry.actor_display_name.as_deref(),
                &entry.actor_username,
            ),
            actor_avatar_color: entry.actor_avatar_color,
            target_user_id: entry.target_user_id,
            target_username: entry.target_username.clone(),
            target_display_name: profile_display_name(
                entry.target_display_name.as_deref(),
                &entry.target_username,
            ),
            target_avatar_color: entry.target_avatar_color,
        })
        .collect();

    Ok(ListModerationLogResult {
        entries,
        cursor: next_cursor,
    })
}

pub async fn list_user_message_history(
    pool: &DbPool,
    actor_user_id: &str,
    guild_slug: &str,
    input: ListUserMessageHistoryInput,
) -> Result<ListUserMessageHistoryResult, AppError> {
    let actor_user_id = normalize_id(actor_user_id, "actor_user_id")?;
    let guild_slug = normalize_slug(guild_slug, "guild_slug")?;
    let target_user_id = normalize_id(&input.target_user_id, "target_user_id")?;
    let limit = normalize_moderation_log_limit(input.limit.as_deref())?;
    let cursor = decode_user_message_history_cursor(input.cursor.as_deref())?;
    let channel_slug = normalize_optional_slug(input.channel_slug.as_deref(), "channel_slug")?;
    let from = normalize_optional_rfc3339(input.from.as_deref(), "from")?;
    let to = normalize_optional_rfc3339(input.to.as_deref(), "to")?;
    if let (Some(from), Some(to)) = (&from, &to)
        && from > to
    {
        return Err(AppError::ValidationError(
            "from must be less than or equal to to".to_string(),
        ));
    }

    let guild = guild::find_guild_by_slug(pool, &guild_slug)
        .await?
        .ok_or(AppError::NotFound)?;
    let effective_permissions =
        permissions::effective_guild_permissions(pool, &guild, &actor_user_id).await?;
    let can_manage_messages =
        permissions::has_permission(effective_permissions, permissions::MANAGE_MESSAGES);
    let can_kick_members =
        permissions::has_permission(effective_permissions, permissions::KICK_MEMBERS);
    if !can_manage_messages && !can_kick_members {
        return Err(AppError::Forbidden(
            "Missing MANAGE_MESSAGES or KICK_MEMBERS permission in this guild".to_string(),
        ));
    }

    let page = message::list_message_history_page_by_guild_and_author(
        pool,
        &guild.id,
        &target_user_id,
        channel_slug.as_deref(),
        from.as_deref(),
        to.as_deref(),
        cursor.as_ref(),
        limit,
    )
    .await?;
    let next_cursor = if page.has_more {
        page.entries.last().map(|entry| {
            encode_user_message_history_cursor(&message::GuildAuthorMessageHistoryCursor {
                created_at: entry.created_at.clone(),
                id: entry.id.clone(),
            })
        })
    } else {
        None
    };

    let entries = page
        .entries
        .into_iter()
        .map(|entry| UserMessageHistoryEntryResponse {
            id: entry.id,
            channel_slug: entry.channel_slug,
            channel_name: entry.channel_name,
            content: entry.content,
            created_at: entry.created_at,
        })
        .collect();

    Ok(ListUserMessageHistoryResult {
        entries,
        cursor: next_cursor,
    })
}

pub async fn unban(
    pool: &DbPool,
    actor_user_id: &str,
    guild_slug: &str,
    ban_id: &str,
) -> Result<UnbanActionResponse, AppError> {
    let actor_user_id = normalize_id(actor_user_id, "actor_user_id")?;
    let guild_slug = normalize_slug(guild_slug, "guild_slug")?;
    let ban_id = normalize_id(ban_id, "ban_id")?;

    let guild = guild::find_guild_by_slug(pool, &guild_slug)
        .await?
        .ok_or(AppError::NotFound)?;
    permissions::require_guild_permission(
        pool,
        &guild,
        &actor_user_id,
        permissions::BAN_MEMBERS,
        "BAN_MEMBERS",
    )
    .await?;

    let now_str = Utc::now().to_rfc3339();
    let rows =
        guild_ban::deactivate_guild_ban_by_id(pool, &guild.id, &ban_id, &actor_user_id, &now_str)
            .await?;
    if rows == 0 {
        return Err(AppError::NotFound);
    }

    let updated = guild_ban::find_guild_ban_by_id(pool, &guild.id, &ban_id)
        .await?
        .ok_or(AppError::NotFound)?;

    Ok(UnbanActionResponse {
        id: updated.id,
        guild_slug: guild.slug,
        target_user_id: updated.target_user_id,
        unbanned_by_user_id: updated
            .unbanned_by_user_id
            .unwrap_or_else(|| actor_user_id.clone()),
        unbanned_at: updated.unbanned_at.unwrap_or(now_str),
        updated_at: updated.updated_at,
    })
}

pub async fn get_my_mute_status(
    pool: &DbPool,
    user_id: &str,
    guild_slug: &str,
) -> Result<MuteStatusResponse, AppError> {
    let user_id = normalize_id(user_id, "user_id")?;
    let guild_slug = normalize_slug(guild_slug, "guild_slug")?;
    let guild = guild::find_guild_by_slug(pool, &guild_slug)
        .await?
        .ok_or(AppError::NotFound)?;
    if !permissions::can_view_guild(pool, &guild, &user_id).await? {
        return Err(AppError::Forbidden(
            "Only guild members can view mute status".to_string(),
        ));
    }
    let active = find_active_mute(pool, &guild, &user_id).await?;
    Ok(to_mute_status_response(active))
}

pub async fn assert_member_can_send_messages(
    pool: &DbPool,
    guild_slug: &str,
    user_id: &str,
) -> Result<(), AppError> {
    let user_id = normalize_id(user_id, "user_id")?;
    let guild_slug = normalize_slug(guild_slug, "guild_slug")?;
    let guild = guild::find_guild_by_slug(pool, &guild_slug)
        .await?
        .ok_or(AppError::NotFound)?;
    let active = find_active_mute(pool, &guild, &user_id).await?;
    if let Some(record) = active {
        return Err(AppError::Forbidden(mute_send_error_message(&record)));
    }
    Ok(())
}

pub async fn assert_member_can_start_typing(
    pool: &DbPool,
    guild_slug: &str,
    user_id: &str,
) -> Result<(), AppError> {
    let user_id = normalize_id(user_id, "user_id")?;
    let guild_slug = normalize_slug(guild_slug, "guild_slug")?;
    let Some(guild) = guild::find_guild_by_slug(pool, &guild_slug).await? else {
        return Ok(());
    };
    let active = find_active_mute(pool, &guild, &user_id).await?;
    if let Some(record) = active {
        return Err(AppError::Forbidden(mute_send_error_message(&record)));
    }
    Ok(())
}

async fn find_active_mute(
    pool: &DbPool,
    guild: &Guild,
    user_id: &str,
) -> Result<Option<moderation::ModerationActionRecord>, AppError> {
    let Some(record) =
        moderation::find_latest_active_mute_for_target(pool, &guild.id, user_id).await?
    else {
        return Ok(None);
    };

    let Some(expires_at) = record.expires_at.as_deref() else {
        return Ok(Some(record));
    };

    let parsed_expires_at = DateTime::parse_from_rfc3339(expires_at)
        .map_err(|_| AppError::Internal("Stored mute expiration has invalid format".to_string()))?
        .with_timezone(&Utc);
    if parsed_expires_at > Utc::now() {
        return Ok(Some(record));
    }

    moderation::deactivate_moderation_action_by_id(pool, &record.id, &Utc::now().to_rfc3339())
        .await?;
    Ok(None)
}

fn to_mute_status_response(
    active: Option<moderation::ModerationActionRecord>,
) -> MuteStatusResponse {
    let Some(record) = active else {
        return MuteStatusResponse {
            active: false,
            is_permanent: false,
            expires_at: None,
            reason: None,
        };
    };
    MuteStatusResponse {
        active: true,
        is_permanent: record.duration_seconds.is_none(),
        expires_at: record.expires_at,
        reason: Some(record.reason),
    }
}

fn normalize_id(value: &str, field: &str) -> Result<String, AppError> {
    let normalized = value.trim();
    if normalized.is_empty() {
        return Err(AppError::ValidationError(format!("{field} is required")));
    }
    if normalized.chars().any(|ch| ch.is_control()) {
        return Err(AppError::ValidationError(format!(
            "{field} contains invalid characters"
        )));
    }
    Ok(normalized.to_string())
}

fn normalize_slug(value: &str, field: &str) -> Result<String, AppError> {
    let slug = value.trim();
    if slug.is_empty() {
        return Err(AppError::ValidationError(format!("{field} is required")));
    }
    Ok(slug.to_string())
}

fn normalize_reason(value: &str) -> Result<String, AppError> {
    let reason = value.trim();
    if reason.is_empty() {
        return Err(AppError::ValidationError("reason is required".to_string()));
    }
    if reason.chars().count() > MAX_MUTE_REASON_CHARS {
        return Err(AppError::ValidationError(format!(
            "reason must be {MAX_MUTE_REASON_CHARS} characters or less"
        )));
    }
    if reason
        .chars()
        .any(|ch| ch.is_control() && ch != '\n' && ch != '\t')
    {
        return Err(AppError::ValidationError(
            "reason contains invalid characters".to_string(),
        ));
    }
    Ok(reason.to_string())
}

fn normalize_report_category(value: Option<&str>) -> Result<Option<String>, AppError> {
    let Some(value) = value else {
        return Ok(None);
    };
    let normalized = value.trim().to_ascii_lowercase();
    if normalized.is_empty() {
        return Ok(None);
    }
    match normalized.as_str() {
        moderation::REPORT_CATEGORY_SPAM
        | moderation::REPORT_CATEGORY_HARASSMENT
        | moderation::REPORT_CATEGORY_RULE_VIOLATION
        | moderation::REPORT_CATEGORY_OTHER => Ok(Some(normalized)),
        _ => Err(AppError::ValidationError(
            "category must be one of: spam, harassment, rule_violation, other".to_string(),
        )),
    }
}

fn normalize_duration_seconds(value: Option<i64>) -> Result<Option<i64>, AppError> {
    let Some(duration_seconds) = value else {
        return Ok(None);
    };
    if duration_seconds <= 0 {
        return Err(AppError::ValidationError(
            "duration_seconds must be greater than zero".to_string(),
        ));
    }
    if duration_seconds > MAX_MUTE_DURATION_SECONDS {
        return Err(AppError::ValidationError(format!(
            "duration_seconds must be {MAX_MUTE_DURATION_SECONDS} seconds or less"
        )));
    }
    Ok(Some(duration_seconds))
}

fn normalize_delete_message_window(value: &str) -> Result<(&'static str, Option<i64>), AppError> {
    match value.trim().to_ascii_lowercase().as_str() {
        "none" => Ok(("none", None)),
        "1h" => Ok(("1h", Some(60 * 60))),
        "24h" => Ok(("24h", Some(24 * 60 * 60))),
        "7d" => Ok(("7d", Some(7 * 24 * 60 * 60))),
        _ => Err(AppError::ValidationError(
            "delete_message_window must be one of: none, 1h, 24h, 7d".to_string(),
        )),
    }
}

fn normalize_moderation_log_limit(raw: Option<&str>) -> Result<i64, AppError> {
    let Some(raw) = raw else {
        return Ok(DEFAULT_MODERATION_LOG_LIMIT);
    };
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(DEFAULT_MODERATION_LOG_LIMIT);
    }
    let parsed = trimmed
        .parse::<i64>()
        .map_err(|_| AppError::ValidationError("limit must be a valid integer".to_string()))?;
    Ok(parsed.clamp(1, MAX_MODERATION_LOG_LIMIT))
}

fn normalize_moderation_log_order(
    raw: Option<&str>,
) -> Result<moderation::ModerationLogSortOrder, AppError> {
    let Some(raw) = raw else {
        return Ok(moderation::ModerationLogSortOrder::Desc);
    };
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(moderation::ModerationLogSortOrder::Desc);
    }
    match trimmed.to_ascii_lowercase().as_str() {
        "asc" => Ok(moderation::ModerationLogSortOrder::Asc),
        "desc" => Ok(moderation::ModerationLogSortOrder::Desc),
        _ => Err(AppError::ValidationError(
            "order must be one of: asc, desc".to_string(),
        )),
    }
}

fn normalize_moderation_log_action_type(raw: Option<&str>) -> Result<Option<String>, AppError> {
    let Some(raw) = raw else {
        return Ok(None);
    };
    let normalized = raw.trim().to_ascii_lowercase();
    if normalized.is_empty() {
        return Ok(None);
    }
    match normalized.as_str() {
        moderation::MODERATION_ACTION_TYPE_MUTE
        | moderation::MODERATION_ACTION_TYPE_KICK
        | moderation::MODERATION_ACTION_TYPE_BAN
        | moderation::MODERATION_ACTION_TYPE_VOICE_KICK
        | moderation::MODERATION_ACTION_TYPE_MESSAGE_DELETE
        | moderation::MODERATION_ACTION_TYPE_WARN => Ok(Some(normalized)),
        _ => Err(AppError::ValidationError(
            "action_type must be one of: mute, kick, ban, voice_kick, message_delete, warn"
                .to_string(),
        )),
    }
}

fn normalize_optional_slug(raw: Option<&str>, field: &str) -> Result<Option<String>, AppError> {
    let Some(raw) = raw else {
        return Ok(None);
    };
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    Ok(Some(normalize_slug(trimmed, field)?))
}

fn normalize_optional_rfc3339(raw: Option<&str>, field: &str) -> Result<Option<String>, AppError> {
    let Some(raw) = raw else {
        return Ok(None);
    };
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    chrono::DateTime::parse_from_rfc3339(trimmed).map_err(|_| {
        AppError::ValidationError(format!("{field} must be a valid RFC3339 timestamp"))
    })?;
    Ok(Some(trimmed.to_string()))
}

fn decode_moderation_log_cursor(
    value: Option<&str>,
) -> Result<Option<moderation::ModerationLogCursor>, AppError> {
    let Some(raw) = value else {
        return Ok(None);
    };
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    decode_moderation_log_cursor_value(trimmed).map(Some)
}

fn decode_user_message_history_cursor(
    value: Option<&str>,
) -> Result<Option<message::GuildAuthorMessageHistoryCursor>, AppError> {
    let Some(raw) = value else {
        return Ok(None);
    };
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    decode_user_message_history_cursor_value(trimmed).map(Some)
}

fn decode_moderation_log_cursor_value(
    encoded: &str,
) -> Result<moderation::ModerationLogCursor, AppError> {
    let decoded = URL_SAFE_NO_PAD
        .decode(encoded)
        .map_err(|_| AppError::ValidationError("cursor is invalid".to_string()))?;
    let decoded_str = std::str::from_utf8(&decoded)
        .map_err(|_| AppError::ValidationError("cursor is invalid".to_string()))?;
    let (created_at, id) = decoded_str
        .split_once('|')
        .ok_or_else(|| AppError::ValidationError("cursor is invalid".to_string()))?;
    if id.trim().is_empty() {
        return Err(AppError::ValidationError("cursor is invalid".to_string()));
    }
    chrono::DateTime::parse_from_rfc3339(created_at)
        .map_err(|_| AppError::ValidationError("cursor is invalid".to_string()))?;
    Ok(moderation::ModerationLogCursor {
        created_at: created_at.to_string(),
        id: id.to_string(),
    })
}

fn decode_user_message_history_cursor_value(
    encoded: &str,
) -> Result<message::GuildAuthorMessageHistoryCursor, AppError> {
    let decoded = URL_SAFE_NO_PAD
        .decode(encoded)
        .map_err(|_| AppError::ValidationError("cursor is invalid".to_string()))?;
    let decoded_str = std::str::from_utf8(&decoded)
        .map_err(|_| AppError::ValidationError("cursor is invalid".to_string()))?;
    let (created_at, id) = decoded_str
        .split_once('|')
        .ok_or_else(|| AppError::ValidationError("cursor is invalid".to_string()))?;
    if id.trim().is_empty() {
        return Err(AppError::ValidationError("cursor is invalid".to_string()));
    }
    chrono::DateTime::parse_from_rfc3339(created_at)
        .map_err(|_| AppError::ValidationError("cursor is invalid".to_string()))?;
    Ok(message::GuildAuthorMessageHistoryCursor {
        created_at: created_at.to_string(),
        id: id.to_string(),
    })
}

fn encode_moderation_log_cursor(cursor: &moderation::ModerationLogCursor) -> String {
    URL_SAFE_NO_PAD.encode(format!("{}|{}", cursor.created_at, cursor.id))
}

fn encode_user_message_history_cursor(cursor: &message::GuildAuthorMessageHistoryCursor) -> String {
    URL_SAFE_NO_PAD.encode(format!("{}|{}", cursor.created_at, cursor.id))
}

fn profile_display_name(display_name: Option<&str>, username: &str) -> String {
    display_name
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(username)
        .to_string()
}

async fn delete_recent_messages_for_banned_user(
    pool: &DbPool,
    attachment_config: &AttachmentConfig,
    guild_id: &str,
    target_user_id: &str,
    created_at_since: &str,
) -> Result<i64, AppError> {
    let message_ids = message::list_message_ids_by_guild_and_author_since(
        pool,
        guild_id,
        target_user_id,
        Some(created_at_since),
    )
    .await?;
    if message_ids.is_empty() {
        return Ok(0);
    }

    let attachments =
        message_attachment::list_message_attachments_by_message_ids(pool, &message_ids).await?;
    let deleted_rows = message::delete_messages_by_ids(pool, &message_ids).await?;
    if deleted_rows == 0 {
        return Ok(0);
    }

    let storage = FileStorageProvider::local(attachment_config.upload_dir.clone());
    for attachment_group in attachments.values() {
        for attachment in attachment_group {
            if let Err(err) = storage.delete(&attachment.storage_key).await {
                tracing::warn!(
                    error = ?err,
                    target_user_id = %target_user_id,
                    message_id = %attachment.message_id,
                    attachment_id = %attachment.id,
                    storage_key = %attachment.storage_key,
                    "Failed to delete attachment file after ban message cleanup"
                );
            }
        }
    }

    i64::try_from(deleted_rows)
        .map_err(|_| AppError::Internal("Deleted message count overflow".to_string()))
}

fn mute_send_error_message(record: &moderation::ModerationActionRecord) -> String {
    match record.expires_at.as_deref() {
        Some(expires_at) => format!("You are muted in this guild until {expires_at}"),
        None => "You are muted in this guild permanently".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        config::DatabaseConfig,
        db::{DbPool, init_pool, run_migrations},
    };

    async fn setup_service_pool() -> DbPool {
        let pool = init_pool(&DatabaseConfig {
            url: "sqlite::memory:".to_string(),
            max_connections: 1,
        })
        .await
        .unwrap();
        run_migrations(&pool).await.unwrap();
        permissions::invalidate_guild_permission_cache("guild-id");
        seed_fixture(&pool).await;
        pool
    }

    async fn seed_fixture(pool: &DbPool) {
        let DbPool::Sqlite(pool) = pool else {
            panic!("service test fixture expects sqlite pool");
        };

        let now = "2026-03-01T00:00:00Z";
        for (id, username) in [
            ("owner-user-id", "owner"),
            ("mod-user-id", "mod"),
            ("target-user-id", "target"),
            ("peer-user-id", "peer"),
            ("high-target-user-id", "high-target"),
        ] {
            sqlx::query(
                "INSERT INTO users (id, did_key, public_key_multibase, username, avatar_color, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            )
            .bind(id)
            .bind(format!("did:key:{id}"))
            .bind(format!("pk:{id}"))
            .bind(username)
            .bind("#99aab5")
            .bind(now)
            .bind(now)
            .execute(pool)
            .await
            .unwrap();
        }

        sqlx::query(
            "INSERT INTO guilds (id, slug, name, description, owner_id, default_channel_slug, created_at, updated_at)
             VALUES (?1, ?2, ?3, NULL, ?4, ?5, ?6, ?7)",
        )
        .bind("guild-id")
        .bind("test-guild")
        .bind("Test Guild")
        .bind("owner-user-id")
        .bind("general")
        .bind(now)
        .bind(now)
        .execute(pool)
        .await
        .unwrap();

        for user_id in [
            "owner-user-id",
            "mod-user-id",
            "target-user-id",
            "peer-user-id",
            "high-target-user-id",
        ] {
            sqlx::query(
                "INSERT INTO guild_members (guild_id, user_id, joined_at, joined_via_invite_code)
                 VALUES (?1, ?2, ?3, NULL)",
            )
            .bind("guild-id")
            .bind(user_id)
            .bind(now)
            .execute(pool)
            .await
            .unwrap();
        }

        sqlx::query(
            "INSERT INTO roles (
                id,
                guild_id,
                name,
                color,
                position,
                permissions_bitflag,
                is_default,
                created_at,
                updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        )
        .bind("role-everyone")
        .bind("guild-id")
        .bind("@everyone")
        .bind("#99aab5")
        .bind(2_147_483_647_i64)
        .bind(permissions::default_everyone_permissions_i64())
        .bind(1_i64)
        .bind(now)
        .bind(now)
        .execute(pool)
        .await
        .unwrap();

        sqlx::query(
            "INSERT INTO roles (
                id,
                guild_id,
                name,
                color,
                position,
                permissions_bitflag,
                is_default,
                created_at,
                updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        )
        .bind("role-moderator")
        .bind("guild-id")
        .bind("Moderator")
        .bind("#3366ff")
        .bind(10_i64)
        .bind(
            (permissions::MUTE_MEMBERS
                | permissions::KICK_MEMBERS
                | permissions::BAN_MEMBERS
                | permissions::VIEW_MOD_LOG
                | permissions::MANAGE_MESSAGES) as i64,
        )
        .bind(0_i64)
        .bind(now)
        .bind(now)
        .execute(pool)
        .await
        .unwrap();

        sqlx::query(
            "INSERT INTO roles (
                id,
                guild_id,
                name,
                color,
                position,
                permissions_bitflag,
                is_default,
                created_at,
                updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        )
        .bind("role-target-low")
        .bind("guild-id")
        .bind("Target Low")
        .bind("#22aa88")
        .bind(20_i64)
        .bind(0_i64)
        .bind(0_i64)
        .bind(now)
        .bind(now)
        .execute(pool)
        .await
        .unwrap();

        sqlx::query(
            "INSERT INTO roles (
                id,
                guild_id,
                name,
                color,
                position,
                permissions_bitflag,
                is_default,
                created_at,
                updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        )
        .bind("role-target-high")
        .bind("guild-id")
        .bind("Target High")
        .bind("#ff5555")
        .bind(5_i64)
        .bind(0_i64)
        .bind(0_i64)
        .bind(now)
        .bind(now)
        .execute(pool)
        .await
        .unwrap();

        sqlx::query(
            "INSERT INTO role_assignments (guild_id, user_id, role_id, assigned_at)
             VALUES (?1, ?2, ?3, ?4)",
        )
        .bind("guild-id")
        .bind("mod-user-id")
        .bind("role-moderator")
        .bind(now)
        .execute(pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO role_assignments (guild_id, user_id, role_id, assigned_at)
             VALUES (?1, ?2, ?3, ?4)",
        )
        .bind("guild-id")
        .bind("target-user-id")
        .bind("role-target-low")
        .bind(now)
        .execute(pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO role_assignments (guild_id, user_id, role_id, assigned_at)
             VALUES (?1, ?2, ?3, ?4)",
        )
        .bind("guild-id")
        .bind("high-target-user-id")
        .bind("role-target-high")
        .bind(now)
        .execute(pool)
        .await
        .unwrap();
    }

    async fn seed_voice_channels(pool: &DbPool) {
        let DbPool::Sqlite(pool) = pool else {
            panic!("service test fixture expects sqlite pool");
        };
        let now = "2026-03-01T00:00:00Z";

        sqlx::query(
            "INSERT INTO channels (id, guild_id, slug, name, channel_type, position, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        )
        .bind("channel-general")
        .bind("guild-id")
        .bind("general")
        .bind("general")
        .bind("text")
        .bind(0_i64)
        .bind(now)
        .bind(now)
        .execute(pool)
        .await
        .unwrap();

        sqlx::query(
            "INSERT INTO channels (id, guild_id, slug, name, channel_type, position, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        )
        .bind("channel-voice-room")
        .bind("guild-id")
        .bind("voice-room")
        .bind("Voice Room")
        .bind("voice")
        .bind(1_i64)
        .bind(now)
        .bind(now)
        .execute(pool)
        .await
        .unwrap();
    }

    async fn grant_view_mod_log_permission(pool: &DbPool) {
        let DbPool::Sqlite(sqlite_pool) = pool else {
            panic!("service test fixture expects sqlite pool");
        };
        sqlx::query(
            "UPDATE roles
             SET permissions_bitflag = ?1
             WHERE id = ?2",
        )
        .bind(
            (permissions::MUTE_MEMBERS
                | permissions::KICK_MEMBERS
                | permissions::BAN_MEMBERS
                | permissions::VIEW_MOD_LOG
                | permissions::MANAGE_MESSAGES) as i64,
        )
        .bind("role-moderator")
        .execute(sqlite_pool)
        .await
        .unwrap();
        permissions::invalidate_guild_permission_cache("guild-id");
    }

    #[test]
    fn normalize_reason_requires_non_empty_reason() {
        assert!(normalize_reason("   ").is_err());
        assert!(normalize_reason(&"x".repeat(MAX_MUTE_REASON_CHARS + 1)).is_err());
        assert_eq!(
            normalize_reason("  valid reason  ").unwrap(),
            "valid reason"
        );
    }

    #[test]
    fn normalize_report_category_accepts_known_values_and_rejects_unknown_values() {
        assert_eq!(normalize_report_category(None).unwrap(), None);
        assert_eq!(normalize_report_category(Some("   ")).unwrap(), None);
        assert_eq!(
            normalize_report_category(Some(" SPAM ")).unwrap(),
            Some("spam".to_string())
        );
        assert_eq!(
            normalize_report_category(Some("rule_violation")).unwrap(),
            Some("rule_violation".to_string())
        );
        assert!(normalize_report_category(Some("invalid")).is_err());
    }

    #[test]
    fn moderation_log_input_normalizers_validate_supported_values() {
        assert_eq!(normalize_moderation_log_limit(None).unwrap(), 50);
        assert_eq!(normalize_moderation_log_limit(Some("")).unwrap(), 50);
        assert_eq!(normalize_moderation_log_limit(Some("0")).unwrap(), 1);
        assert_eq!(normalize_moderation_log_limit(Some("500")).unwrap(), 200);
        assert!(normalize_moderation_log_limit(Some("abc")).is_err());

        assert_eq!(
            normalize_moderation_log_order(None).unwrap(),
            moderation::ModerationLogSortOrder::Desc
        );
        assert_eq!(
            normalize_moderation_log_order(Some("ASC")).unwrap(),
            moderation::ModerationLogSortOrder::Asc
        );
        assert!(normalize_moderation_log_order(Some("sideways")).is_err());

        assert_eq!(
            normalize_moderation_log_action_type(Some("voice_kick")).unwrap(),
            Some("voice_kick".to_string())
        );
        assert!(normalize_moderation_log_action_type(Some("unknown")).is_err());
    }

    #[test]
    fn moderation_log_cursor_round_trip_and_validation() {
        let cursor = moderation::ModerationLogCursor {
            created_at: "2026-03-01T00:00:00Z".to_string(),
            id: "moderation-action-id".to_string(),
        };
        let encoded = encode_moderation_log_cursor(&cursor);
        let decoded = decode_moderation_log_cursor_value(&encoded).unwrap();
        assert_eq!(decoded, cursor);
        assert!(decode_moderation_log_cursor_value("bad").is_err());
    }

    #[tokio::test]
    async fn create_mute_enforces_permission_and_hierarchy() {
        let pool = setup_service_pool().await;
        let missing_permission = create_mute(
            &pool,
            "peer-user-id",
            "test-guild",
            CreateMuteInput {
                target_user_id: "target-user-id".to_string(),
                reason: "reason".to_string(),
                duration_seconds: Some(3600),
            },
        )
        .await
        .unwrap_err();
        assert!(matches!(missing_permission, AppError::Forbidden(_)));

        let hierarchy_error = create_mute(
            &pool,
            "mod-user-id",
            "test-guild",
            CreateMuteInput {
                target_user_id: "high-target-user-id".to_string(),
                reason: "reason".to_string(),
                duration_seconds: Some(3600),
            },
        )
        .await
        .unwrap_err();
        assert!(matches!(hierarchy_error, AppError::Forbidden(_)));
    }

    #[tokio::test]
    async fn create_mute_supports_timed_and_permanent() {
        let pool = setup_service_pool().await;
        let timed = create_mute(
            &pool,
            "mod-user-id",
            "test-guild",
            CreateMuteInput {
                target_user_id: "target-user-id".to_string(),
                reason: "24h cooldown".to_string(),
                duration_seconds: Some(24 * 60 * 60),
            },
        )
        .await
        .unwrap();
        assert_eq!(timed.target_user_id, "target-user-id");
        assert_eq!(timed.duration_seconds, Some(24 * 60 * 60));
        assert!(timed.expires_at.is_some());
        assert!(!timed.is_permanent);

        let permanent = create_mute(
            &pool,
            "mod-user-id",
            "test-guild",
            CreateMuteInput {
                target_user_id: "target-user-id".to_string(),
                reason: "permanent mute".to_string(),
                duration_seconds: None,
            },
        )
        .await
        .unwrap();
        assert_eq!(permanent.duration_seconds, None);
        assert_eq!(permanent.expires_at, None);
        assert!(permanent.is_permanent);
    }

    #[tokio::test]
    async fn create_kick_enforces_permission_hierarchy_and_target_guards() {
        let pool = setup_service_pool().await;
        let missing_permission = create_kick(
            &pool,
            "peer-user-id",
            "test-guild",
            CreateKickInput {
                target_user_id: "target-user-id".to_string(),
                reason: "reason".to_string(),
            },
        )
        .await
        .unwrap_err();
        assert!(matches!(missing_permission, AppError::Forbidden(_)));

        let hierarchy_error = create_kick(
            &pool,
            "mod-user-id",
            "test-guild",
            CreateKickInput {
                target_user_id: "high-target-user-id".to_string(),
                reason: "reason".to_string(),
            },
        )
        .await
        .unwrap_err();
        assert!(matches!(hierarchy_error, AppError::Forbidden(_)));

        let self_error = create_kick(
            &pool,
            "mod-user-id",
            "test-guild",
            CreateKickInput {
                target_user_id: "mod-user-id".to_string(),
                reason: "reason".to_string(),
            },
        )
        .await
        .unwrap_err();
        assert!(matches!(self_error, AppError::ValidationError(_)));

        let owner_error = create_kick(
            &pool,
            "mod-user-id",
            "test-guild",
            CreateKickInput {
                target_user_id: "owner-user-id".to_string(),
                reason: "reason".to_string(),
            },
        )
        .await
        .unwrap_err();
        assert!(matches!(owner_error, AppError::Forbidden(_)));
    }

    #[tokio::test]
    async fn create_kick_removes_member_and_records_audit_action() {
        let pool = setup_service_pool().await;
        moderation::insert_moderation_action(
            &pool,
            "mute-before-kick",
            moderation::MODERATION_ACTION_TYPE_MUTE,
            "guild-id",
            "mod-user-id",
            "target-user-id",
            "active mute",
            Some(3600),
            Some("2030-01-01T00:00:00Z"),
            true,
            "2026-03-01T00:00:00Z",
            "2026-03-01T00:00:00Z",
        )
        .await
        .unwrap();

        let created = create_kick(
            &pool,
            "mod-user-id",
            "test-guild",
            CreateKickInput {
                target_user_id: "target-user-id".to_string(),
                reason: "serious breach".to_string(),
            },
        )
        .await
        .unwrap();

        assert_eq!(created.target_user_id, "target-user-id");
        assert!(
            !guild_member::is_guild_member(&pool, "guild-id", "target-user-id")
                .await
                .unwrap()
        );
        assert!(
            role::list_assigned_role_ids(&pool, "guild-id", "target-user-id")
                .await
                .unwrap()
                .is_empty()
        );

        let DbPool::Sqlite(sqlite_pool) = &pool else {
            panic!("service test fixture expects sqlite pool");
        };
        let action_type = sqlx::query_scalar::<_, String>(
            "SELECT action_type FROM moderation_actions WHERE id = ?1",
        )
        .bind(&created.id)
        .fetch_one(sqlite_pool)
        .await
        .unwrap();
        let duration_seconds = sqlx::query_scalar::<_, Option<i64>>(
            "SELECT duration_seconds FROM moderation_actions WHERE id = ?1",
        )
        .bind(&created.id)
        .fetch_one(sqlite_pool)
        .await
        .unwrap();
        let expires_at = sqlx::query_scalar::<_, Option<String>>(
            "SELECT expires_at FROM moderation_actions WHERE id = ?1",
        )
        .bind(&created.id)
        .fetch_one(sqlite_pool)
        .await
        .unwrap();
        let is_active =
            sqlx::query_scalar::<_, i64>("SELECT is_active FROM moderation_actions WHERE id = ?1")
                .bind(&created.id)
                .fetch_one(sqlite_pool)
                .await
                .unwrap();
        assert_eq!(action_type, moderation::MODERATION_ACTION_TYPE_KICK);
        assert_eq!(duration_seconds, None);
        assert_eq!(expires_at, None);
        assert_eq!(is_active, 0);

        assert!(
            moderation::find_latest_active_mute_for_target(&pool, "guild-id", "target-user-id")
                .await
                .unwrap()
                .is_none()
        );
    }

    #[tokio::test]
    async fn create_voice_kick_enforces_permission_hierarchy_target_guards_and_channel_type() {
        let pool = setup_service_pool().await;
        seed_voice_channels(&pool).await;

        let missing_permission = create_voice_kick(
            &pool,
            "peer-user-id",
            "test-guild",
            CreateVoiceKickInput {
                target_user_id: "target-user-id".to_string(),
                reason: "reason".to_string(),
                channel_slug: "voice-room".to_string(),
            },
        )
        .await
        .unwrap_err();
        assert!(matches!(missing_permission, AppError::Forbidden(_)));

        let hierarchy_error = create_voice_kick(
            &pool,
            "mod-user-id",
            "test-guild",
            CreateVoiceKickInput {
                target_user_id: "high-target-user-id".to_string(),
                reason: "reason".to_string(),
                channel_slug: "voice-room".to_string(),
            },
        )
        .await
        .unwrap_err();
        assert!(matches!(hierarchy_error, AppError::Forbidden(_)));

        let self_error = create_voice_kick(
            &pool,
            "mod-user-id",
            "test-guild",
            CreateVoiceKickInput {
                target_user_id: "mod-user-id".to_string(),
                reason: "reason".to_string(),
                channel_slug: "voice-room".to_string(),
            },
        )
        .await
        .unwrap_err();
        assert!(matches!(self_error, AppError::ValidationError(_)));

        let owner_error = create_voice_kick(
            &pool,
            "mod-user-id",
            "test-guild",
            CreateVoiceKickInput {
                target_user_id: "owner-user-id".to_string(),
                reason: "reason".to_string(),
                channel_slug: "voice-room".to_string(),
            },
        )
        .await
        .unwrap_err();
        assert!(matches!(owner_error, AppError::Forbidden(_)));

        let text_channel_error = create_voice_kick(
            &pool,
            "mod-user-id",
            "test-guild",
            CreateVoiceKickInput {
                target_user_id: "target-user-id".to_string(),
                reason: "reason".to_string(),
                channel_slug: "general".to_string(),
            },
        )
        .await
        .unwrap_err();
        assert!(matches!(text_channel_error, AppError::ValidationError(_)));
    }

    #[tokio::test]
    async fn create_voice_kick_records_audit_action_without_removing_membership() {
        let pool = setup_service_pool().await;
        seed_voice_channels(&pool).await;

        let created = create_voice_kick(
            &pool,
            "mod-user-id",
            "test-guild",
            CreateVoiceKickInput {
                target_user_id: "target-user-id".to_string(),
                reason: "voice disruption".to_string(),
                channel_slug: "voice-room".to_string(),
            },
        )
        .await
        .unwrap();

        assert_eq!(created.target_user_id, "target-user-id");
        assert_eq!(created.channel_slug, "voice-room");
        assert!(
            guild_member::is_guild_member(&pool, "guild-id", "target-user-id")
                .await
                .unwrap()
        );
        assert!(
            role::list_assigned_role_ids(&pool, "guild-id", "target-user-id")
                .await
                .unwrap()
                .iter()
                .any(|role_id| role_id == "role-target-low")
        );

        let DbPool::Sqlite(sqlite_pool) = &pool else {
            panic!("service test fixture expects sqlite pool");
        };
        let action_type = sqlx::query_scalar::<_, String>(
            "SELECT action_type FROM moderation_actions WHERE id = ?1",
        )
        .bind(&created.id)
        .fetch_one(sqlite_pool)
        .await
        .unwrap();
        let duration_seconds = sqlx::query_scalar::<_, Option<i64>>(
            "SELECT duration_seconds FROM moderation_actions WHERE id = ?1",
        )
        .bind(&created.id)
        .fetch_one(sqlite_pool)
        .await
        .unwrap();
        let expires_at = sqlx::query_scalar::<_, Option<String>>(
            "SELECT expires_at FROM moderation_actions WHERE id = ?1",
        )
        .bind(&created.id)
        .fetch_one(sqlite_pool)
        .await
        .unwrap();
        let is_active =
            sqlx::query_scalar::<_, i64>("SELECT is_active FROM moderation_actions WHERE id = ?1")
                .bind(&created.id)
                .fetch_one(sqlite_pool)
                .await
                .unwrap();
        assert_eq!(action_type, moderation::MODERATION_ACTION_TYPE_VOICE_KICK);
        assert_eq!(duration_seconds, None);
        assert_eq!(expires_at, None);
        assert_eq!(is_active, 0);
    }

    #[tokio::test]
    async fn create_message_delete_enforces_permission_hierarchy_and_reason_requirements() {
        let pool = setup_service_pool().await;
        seed_voice_channels(&pool).await;

        message::insert_message(
            &pool,
            "message-high-target",
            "guild-id",
            "channel-general",
            "high-target-user-id",
            "high role message",
            false,
            "2026-03-01T00:01:00Z",
            "2026-03-01T00:01:00Z",
        )
        .await
        .unwrap();

        let missing_permission = create_message_delete(
            &pool,
            "peer-user-id",
            "test-guild",
            CreateMessageDeleteInput {
                message_id: "message-high-target".to_string(),
                channel_slug: "general".to_string(),
                reason: "reason".to_string(),
            },
        )
        .await
        .unwrap_err();
        assert!(matches!(missing_permission, AppError::Forbidden(_)));

        let hierarchy_error = create_message_delete(
            &pool,
            "mod-user-id",
            "test-guild",
            CreateMessageDeleteInput {
                message_id: "message-high-target".to_string(),
                channel_slug: "general".to_string(),
                reason: "reason".to_string(),
            },
        )
        .await
        .unwrap_err();
        assert!(matches!(hierarchy_error, AppError::Forbidden(_)));

        let reason_error = create_message_delete(
            &pool,
            "mod-user-id",
            "test-guild",
            CreateMessageDeleteInput {
                message_id: "message-high-target".to_string(),
                channel_slug: "general".to_string(),
                reason: "   ".to_string(),
            },
        )
        .await
        .unwrap_err();
        assert!(matches!(reason_error, AppError::ValidationError(_)));
    }

    #[tokio::test]
    async fn create_message_delete_soft_deletes_message_and_records_audit_action() {
        let pool = setup_service_pool().await;
        seed_voice_channels(&pool).await;

        message::insert_message(
            &pool,
            "message-target",
            "guild-id",
            "channel-general",
            "target-user-id",
            "remove this",
            false,
            "2026-03-01T00:01:00Z",
            "2026-03-01T00:01:00Z",
        )
        .await
        .unwrap();

        let created = create_message_delete(
            &pool,
            "owner-user-id",
            "test-guild",
            CreateMessageDeleteInput {
                message_id: "message-target".to_string(),
                channel_slug: "general".to_string(),
                reason: "policy violation".to_string(),
            },
        )
        .await
        .unwrap();

        assert_eq!(created.message_id, "message-target");
        assert_eq!(created.guild_slug, "test-guild");
        assert_eq!(created.channel_slug, "general");
        assert_eq!(created.target_user_id, "target-user-id");
        assert_eq!(created.reason, "policy violation");
        assert!(
            message::find_message_by_id(&pool, "message-target")
                .await
                .unwrap()
                .is_none()
        );

        let DbPool::Sqlite(sqlite_pool) = &pool else {
            panic!("service test fixture expects sqlite pool");
        };
        let deleted_metadata = sqlx::query_as::<
            _,
            (
                Option<String>,
                Option<String>,
                Option<String>,
                Option<String>,
            ),
        >(
            "SELECT deleted_at, deleted_by_user_id, deleted_reason, deleted_moderation_action_id
             FROM messages
             WHERE id = ?1",
        )
        .bind("message-target")
        .fetch_one(sqlite_pool)
        .await
        .unwrap();
        assert!(deleted_metadata.0.is_some());
        assert_eq!(deleted_metadata.1, Some("owner-user-id".to_string()));
        assert_eq!(deleted_metadata.2, Some("policy violation".to_string()));
        assert_eq!(deleted_metadata.3, Some(created.id.clone()));

        let action_row = sqlx::query_as::<_, (String, String, String, i64)>(
            "SELECT action_type, actor_user_id, target_user_id, is_active
             FROM moderation_actions
             WHERE id = ?1",
        )
        .bind(&created.id)
        .fetch_one(sqlite_pool)
        .await
        .unwrap();
        assert_eq!(
            action_row.0,
            moderation::MODERATION_ACTION_TYPE_MESSAGE_DELETE
        );
        assert_eq!(action_row.1, "owner-user-id");
        assert_eq!(action_row.2, "target-user-id");
        assert_eq!(action_row.3, 0);
    }

    #[tokio::test]
    async fn create_message_report_validates_membership_self_report_and_duplicates() {
        let pool = setup_service_pool().await;
        seed_voice_channels(&pool).await;
        message::insert_message(
            &pool,
            "report-message-1",
            "guild-id",
            "channel-general",
            "target-user-id",
            "flag this",
            false,
            "2026-03-01T00:01:00Z",
            "2026-03-01T00:01:00Z",
        )
        .await
        .unwrap();

        let missing_membership = create_message_report(
            &pool,
            "outsider-user-id",
            "test-guild",
            CreateMessageReportInput {
                message_id: "report-message-1".to_string(),
                reason: "spam".to_string(),
                category: Some("spam".to_string()),
            },
        )
        .await
        .unwrap_err();
        assert!(matches!(missing_membership, AppError::Forbidden(_)));

        let self_report = create_message_report(
            &pool,
            "target-user-id",
            "test-guild",
            CreateMessageReportInput {
                message_id: "report-message-1".to_string(),
                reason: "cannot self report".to_string(),
                category: None,
            },
        )
        .await
        .unwrap_err();
        assert!(matches!(self_report, AppError::ValidationError(_)));

        let created = create_message_report(
            &pool,
            "mod-user-id",
            "test-guild",
            CreateMessageReportInput {
                message_id: "report-message-1".to_string(),
                reason: "harmful content".to_string(),
                category: Some("harassment".to_string()),
            },
        )
        .await
        .unwrap();
        assert_eq!(created.target_type, moderation::REPORT_TARGET_TYPE_MESSAGE);
        assert_eq!(
            created.target_message_id,
            Some("report-message-1".to_string())
        );
        assert_eq!(created.category, Some("harassment".to_string()));
        assert_eq!(created.status, moderation::REPORT_STATUS_PENDING);

        let duplicate = create_message_report(
            &pool,
            "mod-user-id",
            "test-guild",
            CreateMessageReportInput {
                message_id: "report-message-1".to_string(),
                reason: "duplicate".to_string(),
                category: None,
            },
        )
        .await
        .unwrap_err();
        assert!(matches!(duplicate, AppError::Conflict(_)));
    }

    #[tokio::test]
    async fn create_user_report_validates_target_self_report_and_duplicates() {
        let pool = setup_service_pool().await;

        let missing_membership = create_user_report(
            &pool,
            "outsider-user-id",
            "test-guild",
            CreateUserReportInput {
                target_user_id: "target-user-id".to_string(),
                reason: "spam".to_string(),
                category: Some("spam".to_string()),
            },
        )
        .await
        .unwrap_err();
        assert!(matches!(missing_membership, AppError::Forbidden(_)));

        let missing_target = create_user_report(
            &pool,
            "mod-user-id",
            "test-guild",
            CreateUserReportInput {
                target_user_id: "not-a-member".to_string(),
                reason: "spam".to_string(),
                category: Some("spam".to_string()),
            },
        )
        .await
        .unwrap_err();
        assert!(matches!(missing_target, AppError::ValidationError(_)));

        let self_report = create_user_report(
            &pool,
            "mod-user-id",
            "test-guild",
            CreateUserReportInput {
                target_user_id: "mod-user-id".to_string(),
                reason: "self".to_string(),
                category: None,
            },
        )
        .await
        .unwrap_err();
        assert!(matches!(self_report, AppError::ValidationError(_)));

        let created = create_user_report(
            &pool,
            "mod-user-id",
            "test-guild",
            CreateUserReportInput {
                target_user_id: "target-user-id".to_string(),
                reason: "impersonation".to_string(),
                category: Some("other".to_string()),
            },
        )
        .await
        .unwrap();
        assert_eq!(created.target_type, moderation::REPORT_TARGET_TYPE_USER);
        assert_eq!(created.target_user_id, Some("target-user-id".to_string()));
        assert_eq!(created.category, Some("other".to_string()));
        assert_eq!(created.status, moderation::REPORT_STATUS_PENDING);

        let duplicate = create_user_report(
            &pool,
            "mod-user-id",
            "test-guild",
            CreateUserReportInput {
                target_user_id: "target-user-id".to_string(),
                reason: "duplicate".to_string(),
                category: None,
            },
        )
        .await
        .unwrap_err();
        assert!(matches!(duplicate, AppError::Conflict(_)));
    }

    #[tokio::test]
    async fn apply_kick_action_rolls_back_when_membership_delete_fails() {
        let pool = setup_service_pool().await;
        moderation::insert_moderation_action(
            &pool,
            "mute-before-failed-kick",
            moderation::MODERATION_ACTION_TYPE_MUTE,
            "guild-id",
            "mod-user-id",
            "target-user-id",
            "active mute",
            Some(3600),
            Some("2030-01-01T00:00:00Z"),
            true,
            "2026-03-01T00:00:00Z",
            "2026-03-01T00:00:00Z",
        )
        .await
        .unwrap();

        guild_member::remove_guild_member(&pool, "guild-id", "target-user-id")
            .await
            .unwrap();

        let err = moderation::apply_kick_action(
            &pool,
            "failed-kick",
            "guild-id",
            "mod-user-id",
            "target-user-id",
            "reason",
            "2026-03-01T01:00:00Z",
        )
        .await
        .unwrap_err();
        assert!(matches!(err, AppError::ValidationError(_)));

        let DbPool::Sqlite(sqlite_pool) = &pool else {
            panic!("service test fixture expects sqlite pool");
        };
        let maybe_kick_id =
            sqlx::query_scalar::<_, String>("SELECT id FROM moderation_actions WHERE id = ?1")
                .bind("failed-kick")
                .fetch_optional(sqlite_pool)
                .await
                .unwrap();
        assert!(maybe_kick_id.is_none());

        let role_ids = role::list_assigned_role_ids(&pool, "guild-id", "target-user-id")
            .await
            .unwrap();
        assert!(role_ids.iter().any(|role_id| role_id == "role-target-low"));

        let active_mute =
            moderation::find_latest_active_mute_for_target(&pool, "guild-id", "target-user-id")
                .await
                .unwrap();
        assert!(active_mute.is_some());
    }

    #[tokio::test]
    async fn apply_kick_action_revalidates_hierarchy_within_transaction() {
        let pool = setup_service_pool().await;
        role::set_role_assignments_for_user(
            &pool,
            "guild-id",
            "target-user-id",
            &[String::from("role-target-high")],
            "2026-03-01T00:00:00Z",
        )
        .await
        .unwrap();

        let err = moderation::apply_kick_action(
            &pool,
            "failed-kick-hierarchy",
            "guild-id",
            "mod-user-id",
            "target-user-id",
            "reason",
            "2026-03-01T01:00:00Z",
        )
        .await
        .unwrap_err();
        assert!(matches!(err, AppError::Forbidden(_)));

        assert!(
            guild_member::is_guild_member(&pool, "guild-id", "target-user-id")
                .await
                .unwrap()
        );
        let DbPool::Sqlite(sqlite_pool) = &pool else {
            panic!("service test fixture expects sqlite pool");
        };
        let maybe_kick_id =
            sqlx::query_scalar::<_, String>("SELECT id FROM moderation_actions WHERE id = ?1")
                .bind("failed-kick-hierarchy")
                .fetch_optional(sqlite_pool)
                .await
                .unwrap();
        assert!(maybe_kick_id.is_none());
    }

    #[tokio::test]
    async fn create_ban_enforces_permission_hierarchy_and_target_guards() {
        let pool = setup_service_pool().await;
        let attachment_config = crate::config::AttachmentConfig::default();

        let missing_permission = create_ban(
            &pool,
            &attachment_config,
            "peer-user-id",
            "test-guild",
            CreateBanInput {
                target_user_id: "target-user-id".to_string(),
                reason: "reason".to_string(),
                delete_message_window: "none".to_string(),
            },
        )
        .await
        .unwrap_err();
        assert!(matches!(missing_permission, AppError::Forbidden(_)));

        let hierarchy_error = create_ban(
            &pool,
            &attachment_config,
            "mod-user-id",
            "test-guild",
            CreateBanInput {
                target_user_id: "high-target-user-id".to_string(),
                reason: "reason".to_string(),
                delete_message_window: "none".to_string(),
            },
        )
        .await
        .unwrap_err();
        assert!(matches!(hierarchy_error, AppError::Forbidden(_)));

        let self_error = create_ban(
            &pool,
            &attachment_config,
            "mod-user-id",
            "test-guild",
            CreateBanInput {
                target_user_id: "mod-user-id".to_string(),
                reason: "reason".to_string(),
                delete_message_window: "none".to_string(),
            },
        )
        .await
        .unwrap_err();
        assert!(matches!(self_error, AppError::ValidationError(_)));

        let owner_error = create_ban(
            &pool,
            &attachment_config,
            "mod-user-id",
            "test-guild",
            CreateBanInput {
                target_user_id: "owner-user-id".to_string(),
                reason: "reason".to_string(),
                delete_message_window: "none".to_string(),
            },
        )
        .await
        .unwrap_err();
        assert!(matches!(owner_error, AppError::Forbidden(_)));
    }

    #[tokio::test]
    async fn create_ban_removes_member_and_records_ban_and_audit_action() {
        let pool = setup_service_pool().await;
        let attachment_config = crate::config::AttachmentConfig::default();
        moderation::insert_moderation_action(
            &pool,
            "mute-before-ban",
            moderation::MODERATION_ACTION_TYPE_MUTE,
            "guild-id",
            "mod-user-id",
            "target-user-id",
            "active mute",
            Some(3600),
            Some("2030-01-01T00:00:00Z"),
            true,
            "2026-03-01T00:00:00Z",
            "2026-03-01T00:00:00Z",
        )
        .await
        .unwrap();

        let created = create_ban(
            &pool,
            &attachment_config,
            "mod-user-id",
            "test-guild",
            CreateBanInput {
                target_user_id: "target-user-id".to_string(),
                reason: "serious breach".to_string(),
                delete_message_window: "none".to_string(),
            },
        )
        .await
        .unwrap();

        assert_eq!(created.target_user_id, "target-user-id");
        assert_eq!(created.delete_message_window, "none");
        assert_eq!(created.deleted_messages_count, 0);
        assert!(
            !guild_member::is_guild_member(&pool, "guild-id", "target-user-id")
                .await
                .unwrap()
        );
        assert!(
            role::list_assigned_role_ids(&pool, "guild-id", "target-user-id")
                .await
                .unwrap()
                .is_empty()
        );

        let DbPool::Sqlite(sqlite_pool) = &pool else {
            panic!("service test fixture expects sqlite pool");
        };
        let stored_ban = sqlx::query_as::<_, guild_ban::GuildBanRecord>(
            "SELECT id,
                    guild_id,
                    target_user_id,
                    actor_user_id,
                    reason,
                    delete_messages_window_seconds,
                    is_active,
                    created_at,
                    updated_at,
                    unbanned_by_user_id,
                    unbanned_at
             FROM guild_bans
             WHERE id = ?1",
        )
        .bind(&created.ban_id)
        .fetch_one(sqlite_pool)
        .await
        .unwrap();
        assert_eq!(stored_ban.target_user_id, "target-user-id");
        assert_eq!(stored_ban.reason, "serious breach");
        assert_eq!(stored_ban.delete_messages_window_seconds, None);
        assert_eq!(stored_ban.is_active, 1);

        let action_type = sqlx::query_scalar::<_, String>(
            "SELECT action_type FROM moderation_actions WHERE id = ?1",
        )
        .bind(&created.id)
        .fetch_one(sqlite_pool)
        .await
        .unwrap();
        let is_active =
            sqlx::query_scalar::<_, i64>("SELECT is_active FROM moderation_actions WHERE id = ?1")
                .bind(&created.id)
                .fetch_one(sqlite_pool)
                .await
                .unwrap();
        assert_eq!(action_type, moderation::MODERATION_ACTION_TYPE_BAN);
        assert_eq!(is_active, 0);

        assert!(
            moderation::find_latest_active_mute_for_target(&pool, "guild-id", "target-user-id")
                .await
                .unwrap()
                .is_none()
        );
    }

    #[tokio::test]
    async fn create_ban_delete_window_removes_recent_messages_and_attachments() {
        let pool = setup_service_pool().await;
        let upload_dir =
            std::env::temp_dir().join(format!("discool-ban-cleanup-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&upload_dir).unwrap();
        let attachment_config = crate::config::AttachmentConfig {
            upload_dir: upload_dir.to_string_lossy().to_string(),
            max_size_bytes: 10 * 1024 * 1024,
        };

        let DbPool::Sqlite(sqlite_pool) = &pool else {
            panic!("service test fixture expects sqlite pool");
        };
        sqlx::query(
            "INSERT INTO channels (id, guild_id, slug, name, channel_type, position, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        )
        .bind("channel-general")
        .bind("guild-id")
        .bind("general")
        .bind("general")
        .bind("text")
        .bind(0_i64)
        .bind("2026-03-01T00:00:00Z")
        .bind("2026-03-01T00:00:00Z")
        .execute(sqlite_pool)
        .await
        .unwrap();

        let recent_timestamp = Utc::now().to_rfc3339();
        sqlx::query(
            "INSERT INTO messages (id, guild_id, channel_id, author_user_id, content, is_system, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        )
        .bind("message-recent")
        .bind("guild-id")
        .bind("channel-general")
        .bind("target-user-id")
        .bind("recent")
        .bind(0_i64)
        .bind(&recent_timestamp)
        .bind(&recent_timestamp)
        .execute(sqlite_pool)
        .await
        .unwrap();

        sqlx::query(
            "INSERT INTO messages (id, guild_id, channel_id, author_user_id, content, is_system, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        )
        .bind("message-old")
        .bind("guild-id")
        .bind("channel-general")
        .bind("target-user-id")
        .bind("old")
        .bind(0_i64)
        .bind("2020-01-01T00:00:00Z")
        .bind("2020-01-01T00:00:00Z")
        .execute(sqlite_pool)
        .await
        .unwrap();

        sqlx::query(
            "INSERT INTO message_attachments (id, message_id, storage_key, original_filename, mime_type, size_bytes, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        )
        .bind("attachment-recent")
        .bind("message-recent")
        .bind("ban-cleanup.png")
        .bind("ban-cleanup.png")
        .bind("image/png")
        .bind(3_i64)
        .bind(&recent_timestamp)
        .execute(sqlite_pool)
        .await
        .unwrap();

        std::fs::write(upload_dir.join("ban-cleanup.png"), b"png").unwrap();

        let created = create_ban(
            &pool,
            &attachment_config,
            "mod-user-id",
            "test-guild",
            CreateBanInput {
                target_user_id: "target-user-id".to_string(),
                reason: "clean recent".to_string(),
                delete_message_window: "24h".to_string(),
            },
        )
        .await
        .unwrap();

        assert_eq!(created.delete_message_window, "24h");
        assert_eq!(created.delete_messages_window_seconds, Some(24 * 60 * 60));
        assert_eq!(created.deleted_messages_count, 1);

        let remaining_messages =
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM messages WHERE author_user_id = ?1")
                .bind("target-user-id")
                .fetch_one(sqlite_pool)
                .await
                .unwrap();
        assert_eq!(remaining_messages, 1);

        let remaining_attachment =
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM message_attachments WHERE id = ?1")
                .bind("attachment-recent")
                .fetch_one(sqlite_pool)
                .await
                .unwrap();
        assert_eq!(remaining_attachment, 0);
        assert!(!upload_dir.join("ban-cleanup.png").exists());

        let _ = std::fs::remove_dir_all(&upload_dir);
    }

    #[tokio::test]
    async fn create_ban_succeeds_when_message_cleanup_fails_after_commit() {
        let pool = setup_service_pool().await;
        let attachment_config = crate::config::AttachmentConfig::default();

        let DbPool::Sqlite(sqlite_pool) = &pool else {
            panic!("service test fixture expects sqlite pool");
        };
        sqlx::query("DROP TABLE messages")
            .execute(sqlite_pool)
            .await
            .unwrap();

        let created = create_ban(
            &pool,
            &attachment_config,
            "mod-user-id",
            "test-guild",
            CreateBanInput {
                target_user_id: "target-user-id".to_string(),
                reason: "ban despite cleanup failure".to_string(),
                delete_message_window: "24h".to_string(),
            },
        )
        .await
        .unwrap();

        assert_eq!(created.deleted_messages_count, 0);
        let stored_ban =
            guild_ban::find_active_guild_ban_for_target(&pool, "guild-id", "target-user-id")
                .await
                .unwrap();
        assert!(stored_ban.is_some());
    }

    #[tokio::test]
    async fn apply_ban_action_rolls_back_when_membership_delete_fails() {
        let pool = setup_service_pool().await;
        guild_member::remove_guild_member(&pool, "guild-id", "target-user-id")
            .await
            .unwrap();

        let err = moderation::apply_ban_action(
            &pool,
            "failed-ban-action",
            "failed-ban-record",
            "guild-id",
            "mod-user-id",
            "target-user-id",
            "reason",
            None,
            "2026-03-01T01:00:00Z",
        )
        .await
        .unwrap_err();
        assert!(matches!(err, AppError::ValidationError(_)));

        let DbPool::Sqlite(sqlite_pool) = &pool else {
            panic!("service test fixture expects sqlite pool");
        };
        let moderation_action_exists =
            sqlx::query_scalar::<_, String>("SELECT id FROM moderation_actions WHERE id = ?1")
                .bind("failed-ban-action")
                .fetch_optional(sqlite_pool)
                .await
                .unwrap();
        assert!(moderation_action_exists.is_none());

        let ban_exists = sqlx::query_scalar::<_, String>("SELECT id FROM guild_bans WHERE id = ?1")
            .bind("failed-ban-record")
            .fetch_optional(sqlite_pool)
            .await
            .unwrap();
        assert!(ban_exists.is_none());
    }

    #[tokio::test]
    async fn list_bans_and_unban_round_trip() {
        let pool = setup_service_pool().await;
        let attachment_config = crate::config::AttachmentConfig::default();
        let created = create_ban(
            &pool,
            &attachment_config,
            "mod-user-id",
            "test-guild",
            CreateBanInput {
                target_user_id: "target-user-id".to_string(),
                reason: "cleanup".to_string(),
                delete_message_window: "none".to_string(),
            },
        )
        .await
        .unwrap();

        let bans = list_bans(&pool, "mod-user-id", "test-guild").await.unwrap();
        assert_eq!(bans.len(), 1);
        assert_eq!(bans[0].id, created.ban_id);
        assert_eq!(bans[0].target_user_id, "target-user-id");

        let unbanned = unban(&pool, "mod-user-id", "test-guild", &created.ban_id)
            .await
            .unwrap();
        assert_eq!(unbanned.id, created.ban_id);
        assert_eq!(unbanned.target_user_id, "target-user-id");

        let bans_after = list_bans(&pool, "mod-user-id", "test-guild").await.unwrap();
        assert!(bans_after.is_empty());
    }

    #[tokio::test]
    async fn list_moderation_log_requires_permission_and_supports_query_controls() {
        let pool = setup_service_pool().await;
        for (id, action_type, created_at) in [
            (
                "log-001",
                moderation::MODERATION_ACTION_TYPE_MUTE,
                "2026-03-01T00:00:01Z",
            ),
            (
                "log-002",
                moderation::MODERATION_ACTION_TYPE_KICK,
                "2026-03-01T00:00:02Z",
            ),
            (
                "log-003",
                moderation::MODERATION_ACTION_TYPE_WARN,
                "2026-03-01T00:00:03Z",
            ),
        ] {
            moderation::insert_moderation_action(
                &pool,
                id,
                action_type,
                "guild-id",
                "mod-user-id",
                "target-user-id",
                "reason",
                None,
                None,
                false,
                created_at,
                created_at,
            )
            .await
            .unwrap();
        }

        let forbidden = list_moderation_log(
            &pool,
            "peer-user-id",
            "test-guild",
            ListModerationLogInput {
                limit: Some("2".to_string()),
                cursor: None,
                order: Some("desc".to_string()),
                action_type: None,
            },
        )
        .await
        .unwrap_err();
        assert!(matches!(forbidden, AppError::Forbidden(_)));

        grant_view_mod_log_permission(&pool).await;

        let first_page = list_moderation_log(
            &pool,
            "mod-user-id",
            "test-guild",
            ListModerationLogInput {
                limit: Some("2".to_string()),
                cursor: None,
                order: Some("desc".to_string()),
                action_type: None,
            },
        )
        .await
        .unwrap();
        assert_eq!(first_page.entries.len(), 2);
        assert_eq!(first_page.entries[0].id, "log-003");
        assert_eq!(first_page.entries[1].id, "log-002");
        assert!(first_page.cursor.is_some());

        let second_page = list_moderation_log(
            &pool,
            "mod-user-id",
            "test-guild",
            ListModerationLogInput {
                limit: Some("2".to_string()),
                cursor: first_page.cursor.clone(),
                order: Some("desc".to_string()),
                action_type: None,
            },
        )
        .await
        .unwrap();
        assert_eq!(second_page.entries.len(), 1);
        assert_eq!(second_page.entries[0].id, "log-001");

        let filtered = list_moderation_log(
            &pool,
            "mod-user-id",
            "test-guild",
            ListModerationLogInput {
                limit: Some("5".to_string()),
                cursor: None,
                order: Some("desc".to_string()),
                action_type: Some("kick".to_string()),
            },
        )
        .await
        .unwrap();
        assert_eq!(filtered.entries.len(), 1);
        assert_eq!(filtered.entries[0].id, "log-002");

        let ascending = list_moderation_log(
            &pool,
            "mod-user-id",
            "test-guild",
            ListModerationLogInput {
                limit: Some("5".to_string()),
                cursor: None,
                order: Some("asc".to_string()),
                action_type: None,
            },
        )
        .await
        .unwrap();
        assert_eq!(
            ascending
                .entries
                .iter()
                .map(|entry| entry.id.as_str())
                .collect::<Vec<_>>(),
            vec!["log-001", "log-002", "log-003"]
        );
    }

    #[tokio::test]
    async fn list_user_message_history_supports_permissions_and_filters() {
        let pool = setup_service_pool().await;
        seed_voice_channels(&pool).await;

        for (id, channel_id, author_user_id, content, created_at) in [
            (
                "history-001",
                "channel-general",
                "target-user-id",
                "first",
                "2026-03-01T00:00:01Z",
            ),
            (
                "history-002",
                "channel-voice-room",
                "target-user-id",
                "second",
                "2026-03-01T00:00:02Z",
            ),
            (
                "history-003",
                "channel-general",
                "target-user-id",
                "third",
                "2026-03-01T00:00:03Z",
            ),
            (
                "history-soft-deleted",
                "channel-general",
                "target-user-id",
                "soft-deleted",
                "2026-03-01T00:00:04Z",
            ),
            (
                "history-peer",
                "channel-general",
                "peer-user-id",
                "peer",
                "2026-03-01T00:00:05Z",
            ),
        ] {
            message::insert_message(
                &pool,
                id,
                "guild-id",
                channel_id,
                author_user_id,
                content,
                false,
                created_at,
                created_at,
            )
            .await
            .unwrap();
        }

        let _ = create_message_delete(
            &pool,
            "mod-user-id",
            "test-guild",
            CreateMessageDeleteInput {
                message_id: "history-soft-deleted".to_string(),
                channel_slug: "general".to_string(),
                reason: "cleanup".to_string(),
            },
        )
        .await
        .unwrap();

        let forbidden = list_user_message_history(
            &pool,
            "peer-user-id",
            "test-guild",
            ListUserMessageHistoryInput {
                target_user_id: "target-user-id".to_string(),
                limit: Some("2".to_string()),
                cursor: None,
                channel_slug: None,
                from: None,
                to: None,
            },
        )
        .await
        .unwrap_err();
        assert!(matches!(forbidden, AppError::Forbidden(_)));

        let DbPool::Sqlite(sqlite_pool) = &pool else {
            panic!("service test fixture expects sqlite pool");
        };
        sqlx::query(
            "UPDATE roles
             SET permissions_bitflag = ?1
             WHERE id = ?2",
        )
        .bind(permissions::KICK_MEMBERS as i64)
        .bind("role-moderator")
        .execute(sqlite_pool)
        .await
        .unwrap();
        permissions::invalidate_guild_permission_cache("guild-id");

        let first_page = list_user_message_history(
            &pool,
            "mod-user-id",
            "test-guild",
            ListUserMessageHistoryInput {
                target_user_id: "target-user-id".to_string(),
                limit: Some("2".to_string()),
                cursor: None,
                channel_slug: None,
                from: None,
                to: None,
            },
        )
        .await
        .unwrap();
        assert_eq!(
            first_page
                .entries
                .iter()
                .map(|entry| entry.id.as_str())
                .collect::<Vec<_>>(),
            vec!["history-003", "history-002"]
        );
        assert!(first_page.cursor.is_some());

        let second_page = list_user_message_history(
            &pool,
            "mod-user-id",
            "test-guild",
            ListUserMessageHistoryInput {
                target_user_id: "target-user-id".to_string(),
                limit: Some("2".to_string()),
                cursor: first_page.cursor.clone(),
                channel_slug: None,
                from: None,
                to: None,
            },
        )
        .await
        .unwrap();
        assert_eq!(
            second_page
                .entries
                .iter()
                .map(|entry| entry.id.as_str())
                .collect::<Vec<_>>(),
            vec!["history-001"]
        );
        assert!(second_page.cursor.is_none());

        let filtered_by_channel = list_user_message_history(
            &pool,
            "mod-user-id",
            "test-guild",
            ListUserMessageHistoryInput {
                target_user_id: "target-user-id".to_string(),
                limit: Some("10".to_string()),
                cursor: None,
                channel_slug: Some("general".to_string()),
                from: None,
                to: None,
            },
        )
        .await
        .unwrap();
        assert_eq!(
            filtered_by_channel
                .entries
                .iter()
                .map(|entry| entry.id.as_str())
                .collect::<Vec<_>>(),
            vec!["history-003", "history-001"]
        );

        let filtered_by_date = list_user_message_history(
            &pool,
            "mod-user-id",
            "test-guild",
            ListUserMessageHistoryInput {
                target_user_id: "target-user-id".to_string(),
                limit: Some("10".to_string()),
                cursor: None,
                channel_slug: None,
                from: Some("2026-03-01T00:00:02Z".to_string()),
                to: Some("2026-03-01T00:00:03Z".to_string()),
            },
        )
        .await
        .unwrap();
        assert_eq!(
            filtered_by_date
                .entries
                .iter()
                .map(|entry| entry.id.as_str())
                .collect::<Vec<_>>(),
            vec!["history-003", "history-002"]
        );
    }

    #[tokio::test]
    async fn expired_mute_is_marked_inactive_during_status_reads() {
        let pool = setup_service_pool().await;
        moderation::insert_moderation_action(
            &pool,
            "mute-expired",
            moderation::MODERATION_ACTION_TYPE_MUTE,
            "guild-id",
            "mod-user-id",
            "target-user-id",
            "expired mute",
            Some(60),
            Some("2020-01-01T00:00:00Z"),
            true,
            "2020-01-01T00:00:00Z",
            "2020-01-01T00:00:00Z",
        )
        .await
        .unwrap();

        let status = get_my_mute_status(&pool, "target-user-id", "test-guild")
            .await
            .unwrap();
        assert!(!status.active);

        let stored =
            moderation::find_latest_active_mute_for_target(&pool, "guild-id", "target-user-id")
                .await
                .unwrap();
        assert!(stored.is_none());
    }
}
