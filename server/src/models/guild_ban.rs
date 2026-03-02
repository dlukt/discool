use crate::{AppError, db::DbPool};

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct GuildBanRecord {
    pub id: String,
    pub guild_id: String,
    pub target_user_id: String,
    pub actor_user_id: String,
    pub reason: String,
    pub delete_messages_window_seconds: Option<i64>,
    pub is_active: i64,
    pub created_at: String,
    pub updated_at: String,
    pub unbanned_by_user_id: Option<String>,
    pub unbanned_at: Option<String>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct GuildBanListItem {
    pub id: String,
    pub target_user_id: String,
    pub target_username: String,
    pub target_display_name: Option<String>,
    pub actor_user_id: String,
    pub actor_username: String,
    pub actor_display_name: Option<String>,
    pub reason: String,
    pub delete_messages_window_seconds: Option<i64>,
    pub created_at: String,
}

pub async fn find_active_guild_ban_for_target(
    pool: &DbPool,
    guild_id: &str,
    target_user_id: &str,
) -> Result<Option<GuildBanRecord>, AppError> {
    let ban = match pool {
        DbPool::Postgres(pool) => {
            sqlx::query_as(
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
                 WHERE guild_id = $1
                   AND target_user_id = $2
                   AND is_active = 1
                 ORDER BY created_at DESC, id DESC
                 LIMIT 1",
            )
            .bind(guild_id)
            .bind(target_user_id)
            .fetch_optional(pool)
            .await
        }
        DbPool::Sqlite(pool) => {
            sqlx::query_as(
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
                 WHERE guild_id = ?1
                   AND target_user_id = ?2
                   AND is_active = 1
                 ORDER BY created_at DESC, id DESC
                 LIMIT 1",
            )
            .bind(guild_id)
            .bind(target_user_id)
            .fetch_optional(pool)
            .await
        }
    }
    .map_err(|err| AppError::Internal(err.to_string()))?;

    Ok(ban)
}

pub async fn list_active_guild_bans_for_guild(
    pool: &DbPool,
    guild_id: &str,
) -> Result<Vec<GuildBanListItem>, AppError> {
    let bans = match pool {
        DbPool::Postgres(pool) => {
            sqlx::query_as(
                "SELECT gb.id,
                        gb.target_user_id,
                        target_user.username AS target_username,
                        target_user.display_name AS target_display_name,
                        gb.actor_user_id,
                        actor_user.username AS actor_username,
                        actor_user.display_name AS actor_display_name,
                        gb.reason,
                        gb.delete_messages_window_seconds,
                        gb.created_at
                 FROM guild_bans gb
                 JOIN users target_user
                   ON target_user.id = gb.target_user_id
                 JOIN users actor_user
                   ON actor_user.id = gb.actor_user_id
                 WHERE gb.guild_id = $1
                   AND gb.is_active = 1
                 ORDER BY gb.created_at DESC, gb.id DESC",
            )
            .bind(guild_id)
            .fetch_all(pool)
            .await
        }
        DbPool::Sqlite(pool) => {
            sqlx::query_as(
                "SELECT gb.id,
                        gb.target_user_id,
                        target_user.username AS target_username,
                        target_user.display_name AS target_display_name,
                        gb.actor_user_id,
                        actor_user.username AS actor_username,
                        actor_user.display_name AS actor_display_name,
                        gb.reason,
                        gb.delete_messages_window_seconds,
                        gb.created_at
                 FROM guild_bans gb
                 JOIN users target_user
                   ON target_user.id = gb.target_user_id
                 JOIN users actor_user
                   ON actor_user.id = gb.actor_user_id
                 WHERE gb.guild_id = ?1
                   AND gb.is_active = 1
                 ORDER BY gb.created_at DESC, gb.id DESC",
            )
            .bind(guild_id)
            .fetch_all(pool)
            .await
        }
    }
    .map_err(|err| AppError::Internal(err.to_string()))?;

    Ok(bans)
}

pub async fn deactivate_guild_ban_by_id(
    pool: &DbPool,
    guild_id: &str,
    ban_id: &str,
    unbanned_by_user_id: &str,
    now: &str,
) -> Result<u64, AppError> {
    let rows = match pool {
        DbPool::Postgres(pool) => sqlx::query(
            "UPDATE guild_bans
                 SET is_active = 0,
                     updated_at = $1,
                     unbanned_by_user_id = $2,
                     unbanned_at = $3
                 WHERE id = $4
                   AND guild_id = $5
                   AND is_active = 1",
        )
        .bind(now)
        .bind(unbanned_by_user_id)
        .bind(now)
        .bind(ban_id)
        .bind(guild_id)
        .execute(pool)
        .await
        .map(|result| result.rows_affected()),
        DbPool::Sqlite(pool) => sqlx::query(
            "UPDATE guild_bans
                 SET is_active = 0,
                     updated_at = ?1,
                     unbanned_by_user_id = ?2,
                     unbanned_at = ?3
                 WHERE id = ?4
                   AND guild_id = ?5
                   AND is_active = 1",
        )
        .bind(now)
        .bind(unbanned_by_user_id)
        .bind(now)
        .bind(ban_id)
        .bind(guild_id)
        .execute(pool)
        .await
        .map(|result| result.rows_affected()),
    }
    .map_err(|err| AppError::Internal(err.to_string()))?;

    Ok(rows)
}

pub async fn find_guild_ban_by_id(
    pool: &DbPool,
    guild_id: &str,
    ban_id: &str,
) -> Result<Option<GuildBanRecord>, AppError> {
    let ban = match pool {
        DbPool::Postgres(pool) => {
            sqlx::query_as(
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
                 WHERE guild_id = $1
                   AND id = $2
                 LIMIT 1",
            )
            .bind(guild_id)
            .bind(ban_id)
            .fetch_optional(pool)
            .await
        }
        DbPool::Sqlite(pool) => {
            sqlx::query_as(
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
                 WHERE guild_id = ?1
                   AND id = ?2
                 LIMIT 1",
            )
            .bind(guild_id)
            .bind(ban_id)
            .fetch_optional(pool)
            .await
        }
    }
    .map_err(|err| AppError::Internal(err.to_string()))?;

    Ok(ban)
}
