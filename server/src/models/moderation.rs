use crate::{AppError, db::DbPool};

pub const MODERATION_ACTION_TYPE_MUTE: &str = "mute";

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ModerationActionRecord {
    pub id: String,
    pub action_type: String,
    pub guild_id: String,
    pub actor_user_id: String,
    pub target_user_id: String,
    pub reason: String,
    pub duration_seconds: Option<i64>,
    pub expires_at: Option<String>,
    pub is_active: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[allow(clippy::too_many_arguments)]
pub async fn insert_moderation_action(
    pool: &DbPool,
    id: &str,
    action_type: &str,
    guild_id: &str,
    actor_user_id: &str,
    target_user_id: &str,
    reason: &str,
    duration_seconds: Option<i64>,
    expires_at: Option<&str>,
    is_active: bool,
    created_at: &str,
    updated_at: &str,
) -> Result<(), AppError> {
    let is_active_value = if is_active { 1_i64 } else { 0_i64 };
    match pool {
        DbPool::Postgres(pool) => sqlx::query(
            "INSERT INTO moderation_actions (
                    id,
                    action_type,
                    guild_id,
                    actor_user_id,
                    target_user_id,
                    reason,
                    duration_seconds,
                    expires_at,
                    is_active,
                    created_at,
                    updated_at
                ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)",
        )
        .bind(id)
        .bind(action_type)
        .bind(guild_id)
        .bind(actor_user_id)
        .bind(target_user_id)
        .bind(reason)
        .bind(duration_seconds)
        .bind(expires_at)
        .bind(is_active_value)
        .bind(created_at)
        .bind(updated_at)
        .execute(pool)
        .await
        .map(|_| ()),
        DbPool::Sqlite(pool) => sqlx::query(
            "INSERT INTO moderation_actions (
                    id,
                    action_type,
                    guild_id,
                    actor_user_id,
                    target_user_id,
                    reason,
                    duration_seconds,
                    expires_at,
                    is_active,
                    created_at,
                    updated_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        )
        .bind(id)
        .bind(action_type)
        .bind(guild_id)
        .bind(actor_user_id)
        .bind(target_user_id)
        .bind(reason)
        .bind(duration_seconds)
        .bind(expires_at)
        .bind(is_active_value)
        .bind(created_at)
        .bind(updated_at)
        .execute(pool)
        .await
        .map(|_| ()),
    }
    .map_err(|err| AppError::Internal(err.to_string()))?;

    Ok(())
}

pub async fn find_latest_active_mute_for_target(
    pool: &DbPool,
    guild_id: &str,
    target_user_id: &str,
) -> Result<Option<ModerationActionRecord>, AppError> {
    let record = match pool {
        DbPool::Postgres(pool) => {
            sqlx::query_as(
                "SELECT id,
                    action_type,
                    guild_id,
                    actor_user_id,
                    target_user_id,
                    reason,
                    duration_seconds,
                    expires_at,
                    is_active,
                    created_at,
                    updated_at
             FROM moderation_actions
             WHERE guild_id = $1
               AND target_user_id = $2
               AND action_type = $3
               AND is_active = 1
             ORDER BY created_at DESC, id DESC
             LIMIT 1",
            )
            .bind(guild_id)
            .bind(target_user_id)
            .bind(MODERATION_ACTION_TYPE_MUTE)
            .fetch_optional(pool)
            .await
        }
        DbPool::Sqlite(pool) => {
            sqlx::query_as(
                "SELECT id,
                    action_type,
                    guild_id,
                    actor_user_id,
                    target_user_id,
                    reason,
                    duration_seconds,
                    expires_at,
                    is_active,
                    created_at,
                    updated_at
             FROM moderation_actions
             WHERE guild_id = ?1
               AND target_user_id = ?2
               AND action_type = ?3
               AND is_active = 1
             ORDER BY created_at DESC, id DESC
             LIMIT 1",
            )
            .bind(guild_id)
            .bind(target_user_id)
            .bind(MODERATION_ACTION_TYPE_MUTE)
            .fetch_optional(pool)
            .await
        }
    }
    .map_err(|err| AppError::Internal(err.to_string()))?;

    Ok(record)
}

pub async fn deactivate_active_mutes_for_target(
    pool: &DbPool,
    guild_id: &str,
    target_user_id: &str,
    updated_at: &str,
) -> Result<u64, AppError> {
    let rows_affected = match pool {
        DbPool::Postgres(pool) => sqlx::query(
            "UPDATE moderation_actions
                 SET is_active = 0,
                     updated_at = $1
                 WHERE guild_id = $2
                   AND target_user_id = $3
                   AND action_type = $4
                   AND is_active = 1",
        )
        .bind(updated_at)
        .bind(guild_id)
        .bind(target_user_id)
        .bind(MODERATION_ACTION_TYPE_MUTE)
        .execute(pool)
        .await
        .map(|result| result.rows_affected()),
        DbPool::Sqlite(pool) => sqlx::query(
            "UPDATE moderation_actions
                 SET is_active = 0,
                     updated_at = ?1
                 WHERE guild_id = ?2
                   AND target_user_id = ?3
                   AND action_type = ?4
                   AND is_active = 1",
        )
        .bind(updated_at)
        .bind(guild_id)
        .bind(target_user_id)
        .bind(MODERATION_ACTION_TYPE_MUTE)
        .execute(pool)
        .await
        .map(|result| result.rows_affected()),
    }
    .map_err(|err| AppError::Internal(err.to_string()))?;

    Ok(rows_affected)
}

pub async fn deactivate_moderation_action_by_id(
    pool: &DbPool,
    id: &str,
    updated_at: &str,
) -> Result<u64, AppError> {
    let rows_affected = match pool {
        DbPool::Postgres(pool) => sqlx::query(
            "UPDATE moderation_actions
                 SET is_active = 0,
                     updated_at = $1
                 WHERE id = $2
                   AND is_active = 1",
        )
        .bind(updated_at)
        .bind(id)
        .execute(pool)
        .await
        .map(|result| result.rows_affected()),
        DbPool::Sqlite(pool) => sqlx::query(
            "UPDATE moderation_actions
                 SET is_active = 0,
                     updated_at = ?1
                 WHERE id = ?2
                   AND is_active = 1",
        )
        .bind(updated_at)
        .bind(id)
        .execute(pool)
        .await
        .map(|result| result.rows_affected()),
    }
    .map_err(|err| AppError::Internal(err.to_string()))?;

    Ok(rows_affected)
}
