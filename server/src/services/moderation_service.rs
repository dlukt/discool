use chrono::{DateTime, Duration, Utc};
use serde::Serialize;
use uuid::Uuid;

use crate::{
    AppError,
    db::DbPool,
    models::{
        guild::{self, Guild},
        guild_member, moderation,
    },
    permissions,
};

const MAX_MUTE_REASON_CHARS: usize = 500;
const MAX_MUTE_DURATION_SECONDS: i64 = 315_360_000; // 10 years

#[derive(Debug, Clone)]
pub struct CreateMuteInput {
    pub target_user_id: String,
    pub reason: String,
    pub duration_seconds: Option<i64>,
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

    if !guild_member::is_guild_member(pool, &guild.id, &target_user_id).await? {
        return Err(AppError::ValidationError(
            "target_user_id must belong to a guild member".to_string(),
        ));
    }
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
        .bind(permissions::MUTE_MEMBERS as i64)
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
