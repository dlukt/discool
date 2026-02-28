use crate::{AppError, db::DbPool};

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct GuildInviteWithCreator {
    pub id: String,
    pub guild_id: String,
    pub code: String,
    pub invite_type: String,
    pub uses_remaining: i64,
    pub created_by: String,
    pub creator_username: String,
    pub created_at: String,
    pub revoked: i64,
}

#[allow(clippy::too_many_arguments)]
pub async fn insert_guild_invite(
    pool: &DbPool,
    id: &str,
    guild_id: &str,
    code: &str,
    invite_type: &str,
    uses_remaining: i64,
    created_by: &str,
    created_at: &str,
) -> Result<bool, AppError> {
    let rows = match pool {
        DbPool::Postgres(pool) => {
            sqlx::query(
                "INSERT INTO guild_invites (id, guild_id, code, type, uses_remaining, created_by, created_at, revoked)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, 0)
                 ON CONFLICT DO NOTHING",
            )
            .bind(id)
            .bind(guild_id)
            .bind(code)
            .bind(invite_type)
            .bind(uses_remaining)
            .bind(created_by)
            .bind(created_at)
            .execute(pool)
            .await
            .map(|result| result.rows_affected())
        }
        DbPool::Sqlite(pool) => {
            sqlx::query(
                "INSERT INTO guild_invites (id, guild_id, code, type, uses_remaining, created_by, created_at, revoked)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 0)
                 ON CONFLICT DO NOTHING",
            )
            .bind(id)
            .bind(guild_id)
            .bind(code)
            .bind(invite_type)
            .bind(uses_remaining)
            .bind(created_by)
            .bind(created_at)
            .execute(pool)
            .await
            .map(|result| result.rows_affected())
        }
    }
    .map_err(|err| AppError::Internal(err.to_string()))?;

    Ok(rows == 1)
}

pub async fn list_active_guild_invites(
    pool: &DbPool,
    guild_id: &str,
) -> Result<Vec<GuildInviteWithCreator>, AppError> {
    let invites = match pool {
        DbPool::Postgres(pool) => {
            sqlx::query_as(
                "SELECT gi.id,
                        gi.guild_id,
                        gi.code,
                        gi.type AS invite_type,
                        gi.uses_remaining,
                        gi.created_by,
                        u.username AS creator_username,
                        gi.created_at,
                        gi.revoked
                 FROM guild_invites gi
                 JOIN users u ON u.id = gi.created_by
                 WHERE gi.guild_id = $1
                   AND gi.revoked = 0
                   AND (gi.type = 'reusable' OR gi.uses_remaining > 0)
                 ORDER BY gi.created_at DESC",
            )
            .bind(guild_id)
            .fetch_all(pool)
            .await
        }
        DbPool::Sqlite(pool) => {
            sqlx::query_as(
                "SELECT gi.id,
                        gi.guild_id,
                        gi.code,
                        gi.type AS invite_type,
                        gi.uses_remaining,
                        gi.created_by,
                        u.username AS creator_username,
                        gi.created_at,
                        gi.revoked
                 FROM guild_invites gi
                 JOIN users u ON u.id = gi.created_by
                 WHERE gi.guild_id = ?1
                   AND gi.revoked = 0
                   AND (gi.type = 'reusable' OR gi.uses_remaining > 0)
                 ORDER BY gi.created_at DESC",
            )
            .bind(guild_id)
            .fetch_all(pool)
            .await
        }
    }
    .map_err(|err| AppError::Internal(err.to_string()))?;

    Ok(invites)
}

pub async fn find_invite_with_creator_by_code(
    pool: &DbPool,
    guild_id: &str,
    code: &str,
) -> Result<Option<GuildInviteWithCreator>, AppError> {
    let invite = match pool {
        DbPool::Postgres(pool) => {
            sqlx::query_as(
                "SELECT gi.id,
                        gi.guild_id,
                        gi.code,
                        gi.type AS invite_type,
                        gi.uses_remaining,
                        gi.created_by,
                        u.username AS creator_username,
                        gi.created_at,
                        gi.revoked
                 FROM guild_invites gi
                 JOIN users u ON u.id = gi.created_by
                 WHERE gi.guild_id = $1
                   AND gi.code = $2
                 LIMIT 1",
            )
            .bind(guild_id)
            .bind(code)
            .fetch_optional(pool)
            .await
        }
        DbPool::Sqlite(pool) => {
            sqlx::query_as(
                "SELECT gi.id,
                        gi.guild_id,
                        gi.code,
                        gi.type AS invite_type,
                        gi.uses_remaining,
                        gi.created_by,
                        u.username AS creator_username,
                        gi.created_at,
                        gi.revoked
                 FROM guild_invites gi
                 JOIN users u ON u.id = gi.created_by
                 WHERE gi.guild_id = ?1
                   AND gi.code = ?2
                 LIMIT 1",
            )
            .bind(guild_id)
            .bind(code)
            .fetch_optional(pool)
            .await
        }
    }
    .map_err(|err| AppError::Internal(err.to_string()))?;

    Ok(invite)
}

pub async fn revoke_invite_by_code(
    pool: &DbPool,
    guild_id: &str,
    code: &str,
) -> Result<u64, AppError> {
    let rows = match pool {
        DbPool::Postgres(pool) => sqlx::query(
            "UPDATE guild_invites
                 SET revoked = 1
                 WHERE guild_id = $1
                   AND code = $2
                   AND revoked = 0",
        )
        .bind(guild_id)
        .bind(code)
        .execute(pool)
        .await
        .map(|result| result.rows_affected()),
        DbPool::Sqlite(pool) => sqlx::query(
            "UPDATE guild_invites
                 SET revoked = 1
                 WHERE guild_id = ?1
                   AND code = ?2
                   AND revoked = 0",
        )
        .bind(guild_id)
        .bind(code)
        .execute(pool)
        .await
        .map(|result| result.rows_affected()),
    }
    .map_err(|err| AppError::Internal(err.to_string()))?;

    Ok(rows)
}
