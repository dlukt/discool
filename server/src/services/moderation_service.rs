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
            (permissions::MUTE_MEMBERS | permissions::KICK_MEMBERS | permissions::BAN_MEMBERS)
                as i64,
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

    #[test]
    fn normalize_reason_requires_non_empty_reason() {
        assert!(normalize_reason("   ").is_err());
        assert!(normalize_reason(&"x".repeat(MAX_MUTE_REASON_CHARS + 1)).is_err());
        assert_eq!(
            normalize_reason("  valid reason  ").unwrap(),
            "valid reason"
        );
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
