use crate::{AppError, db::DbPool};

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ChannelPermissionOverride {
    pub channel_id: String,
    pub role_id: String,
    pub allow_bitflag: i64,
    pub deny_bitflag: i64,
}

pub async fn list_overrides_by_channel_id(
    pool: &DbPool,
    channel_id: &str,
) -> Result<Vec<ChannelPermissionOverride>, AppError> {
    let overrides = match pool {
        DbPool::Postgres(pool) => {
            sqlx::query_as(
                "SELECT channel_id, role_id, allow_bitflag, deny_bitflag
                 FROM channel_permission_overrides
                 WHERE channel_id = $1
                 ORDER BY role_id ASC",
            )
            .bind(channel_id)
            .fetch_all(pool)
            .await
        }
        DbPool::Sqlite(pool) => {
            sqlx::query_as(
                "SELECT channel_id, role_id, allow_bitflag, deny_bitflag
                 FROM channel_permission_overrides
                 WHERE channel_id = ?1
                 ORDER BY role_id ASC",
            )
            .bind(channel_id)
            .fetch_all(pool)
            .await
        }
    }
    .map_err(|err| AppError::Internal(err.to_string()))?;

    Ok(overrides)
}

pub async fn list_overrides_by_guild_id(
    pool: &DbPool,
    guild_id: &str,
) -> Result<Vec<ChannelPermissionOverride>, AppError> {
    let overrides = match pool {
        DbPool::Postgres(pool) => {
            sqlx::query_as(
                "SELECT cpo.channel_id, cpo.role_id, cpo.allow_bitflag, cpo.deny_bitflag
                 FROM channel_permission_overrides cpo
                 JOIN channels c ON c.id = cpo.channel_id
                 WHERE c.guild_id = $1
                 ORDER BY cpo.channel_id ASC, cpo.role_id ASC",
            )
            .bind(guild_id)
            .fetch_all(pool)
            .await
        }
        DbPool::Sqlite(pool) => {
            sqlx::query_as(
                "SELECT cpo.channel_id, cpo.role_id, cpo.allow_bitflag, cpo.deny_bitflag
                 FROM channel_permission_overrides cpo
                 JOIN channels c ON c.id = cpo.channel_id
                 WHERE c.guild_id = ?1
                 ORDER BY cpo.channel_id ASC, cpo.role_id ASC",
            )
            .bind(guild_id)
            .fetch_all(pool)
            .await
        }
    }
    .map_err(|err| AppError::Internal(err.to_string()))?;

    Ok(overrides)
}

pub async fn upsert_override(
    pool: &DbPool,
    channel_id: &str,
    role_id: &str,
    allow_bitflag: i64,
    deny_bitflag: i64,
) -> Result<(), AppError> {
    match pool {
        DbPool::Postgres(pool) => {
            sqlx::query(
                "INSERT INTO channel_permission_overrides (channel_id, role_id, allow_bitflag, deny_bitflag)
                 VALUES ($1, $2, $3, $4)
                 ON CONFLICT (channel_id, role_id)
                 DO UPDATE SET allow_bitflag = EXCLUDED.allow_bitflag, deny_bitflag = EXCLUDED.deny_bitflag",
            )
            .bind(channel_id)
            .bind(role_id)
            .bind(allow_bitflag)
            .bind(deny_bitflag)
            .execute(pool)
            .await
            .map_err(|err| AppError::Internal(err.to_string()))?;
        }
        DbPool::Sqlite(pool) => {
            sqlx::query(
                "INSERT INTO channel_permission_overrides (channel_id, role_id, allow_bitflag, deny_bitflag)
                 VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT (channel_id, role_id)
                 DO UPDATE SET allow_bitflag = excluded.allow_bitflag, deny_bitflag = excluded.deny_bitflag",
            )
            .bind(channel_id)
            .bind(role_id)
            .bind(allow_bitflag)
            .bind(deny_bitflag)
            .execute(pool)
            .await
            .map_err(|err| AppError::Internal(err.to_string()))?;
        }
    }

    Ok(())
}

pub async fn delete_override(
    pool: &DbPool,
    channel_id: &str,
    role_id: &str,
) -> Result<u64, AppError> {
    let rows = match pool {
        DbPool::Postgres(pool) => sqlx::query(
            "DELETE FROM channel_permission_overrides
                 WHERE channel_id = $1 AND role_id = $2",
        )
        .bind(channel_id)
        .bind(role_id)
        .execute(pool)
        .await
        .map(|result| result.rows_affected()),
        DbPool::Sqlite(pool) => sqlx::query(
            "DELETE FROM channel_permission_overrides
                 WHERE channel_id = ?1 AND role_id = ?2",
        )
        .bind(channel_id)
        .bind(role_id)
        .execute(pool)
        .await
        .map(|result| result.rows_affected()),
    }
    .map_err(|err| AppError::Internal(err.to_string()))?;

    Ok(rows)
}
