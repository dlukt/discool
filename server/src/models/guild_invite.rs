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

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct InviteWithGuild {
    pub id: String,
    pub guild_id: String,
    pub guild_slug: String,
    pub guild_name: String,
    pub guild_default_channel_slug: String,
    pub guild_icon_storage_key: Option<String>,
    pub code: String,
    pub invite_type: String,
    pub uses_remaining: i64,
    pub revoked: i64,
}

#[derive(Debug, Clone)]
pub struct InviteJoinResult {
    pub guild_slug: String,
    pub guild_name: String,
    pub default_channel_slug: String,
    pub icon_storage_key: Option<String>,
    pub already_member: bool,
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

pub async fn find_active_invite_with_guild_by_code(
    pool: &DbPool,
    code: &str,
) -> Result<Option<InviteWithGuild>, AppError> {
    let invite = match pool {
        DbPool::Postgres(pool) => {
            sqlx::query_as(
                "SELECT gi.id,
                        gi.guild_id,
                        g.slug AS guild_slug,
                        g.name AS guild_name,
                        g.default_channel_slug AS guild_default_channel_slug,
                        g.icon_storage_key AS guild_icon_storage_key,
                        gi.code,
                        gi.type AS invite_type,
                        gi.uses_remaining,
                        gi.revoked
                 FROM guild_invites gi
                 JOIN guilds g ON g.id = gi.guild_id
                 WHERE gi.code = $1
                   AND gi.revoked = 0
                   AND (gi.type = 'reusable' OR gi.uses_remaining > 0)
                 LIMIT 1",
            )
            .bind(code)
            .fetch_optional(pool)
            .await
        }
        DbPool::Sqlite(pool) => {
            sqlx::query_as(
                "SELECT gi.id,
                        gi.guild_id,
                        g.slug AS guild_slug,
                        g.name AS guild_name,
                        g.default_channel_slug AS guild_default_channel_slug,
                        g.icon_storage_key AS guild_icon_storage_key,
                        gi.code,
                        gi.type AS invite_type,
                        gi.uses_remaining,
                        gi.revoked
                 FROM guild_invites gi
                 JOIN guilds g ON g.id = gi.guild_id
                 WHERE gi.code = ?1
                   AND gi.revoked = 0
                   AND (gi.type = 'reusable' OR gi.uses_remaining > 0)
                 LIMIT 1",
            )
            .bind(code)
            .fetch_optional(pool)
            .await
        }
    }
    .map_err(|err| AppError::Internal(err.to_string()))?;

    Ok(invite)
}

pub async fn join_guild_via_invite(
    pool: &DbPool,
    code: &str,
    user_id: &str,
    joined_at: &str,
) -> Result<Option<InviteJoinResult>, AppError> {
    match pool {
        DbPool::Postgres(pool) => {
            let mut tx = pool
                .begin()
                .await
                .map_err(|err| AppError::Internal(err.to_string()))?;

            let invite: Option<InviteWithGuild> = sqlx::query_as(
                "SELECT gi.id,
                        gi.guild_id,
                        g.slug AS guild_slug,
                        g.name AS guild_name,
                        g.default_channel_slug AS guild_default_channel_slug,
                        g.icon_storage_key AS guild_icon_storage_key,
                        gi.code,
                        gi.type AS invite_type,
                        gi.uses_remaining,
                        gi.revoked
                 FROM guild_invites gi
                 JOIN guilds g ON g.id = gi.guild_id
                 WHERE gi.code = $1
                   AND gi.revoked = 0
                 LIMIT 1",
            )
            .bind(code)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|err| AppError::Internal(err.to_string()))?;

            let Some(invite) = invite else {
                return Ok(None);
            };

            let already_member = sqlx::query_scalar::<_, bool>(
                "SELECT EXISTS(
                    SELECT 1
                    FROM guild_members
                    WHERE guild_id = $1 AND user_id = $2
                )",
            )
            .bind(&invite.guild_id)
            .bind(user_id)
            .fetch_one(&mut *tx)
            .await
            .map_err(|err| AppError::Internal(err.to_string()))?;

            if already_member {
                tx.commit()
                    .await
                    .map_err(|err| AppError::Internal(err.to_string()))?;
                return Ok(Some(InviteJoinResult {
                    guild_slug: invite.guild_slug,
                    guild_name: invite.guild_name,
                    default_channel_slug: invite.guild_default_channel_slug,
                    icon_storage_key: invite.guild_icon_storage_key,
                    already_member: true,
                }));
            }

            if invite.invite_type == "single_use" && invite.uses_remaining <= 0 {
                return Ok(None);
            }

            let inserted = sqlx::query(
                "INSERT INTO guild_members (guild_id, user_id, joined_at, joined_via_invite_code)
                 VALUES ($1, $2, $3, $4)
                 ON CONFLICT DO NOTHING",
            )
            .bind(&invite.guild_id)
            .bind(user_id)
            .bind(joined_at)
            .bind(&invite.code)
            .execute(&mut *tx)
            .await
            .map_err(|err| AppError::Internal(err.to_string()))?
            .rows_affected()
                == 1;

            if !inserted {
                tx.commit()
                    .await
                    .map_err(|err| AppError::Internal(err.to_string()))?;
                return Ok(Some(InviteJoinResult {
                    guild_slug: invite.guild_slug,
                    guild_name: invite.guild_name,
                    default_channel_slug: invite.guild_default_channel_slug,
                    icon_storage_key: invite.guild_icon_storage_key,
                    already_member: true,
                }));
            }

            if invite.invite_type == "single_use" {
                let consumed = sqlx::query(
                    "UPDATE guild_invites
                     SET uses_remaining = uses_remaining - 1
                     WHERE id = $1
                       AND revoked = 0
                       AND uses_remaining > 0",
                )
                .bind(&invite.id)
                .execute(&mut *tx)
                .await
                .map_err(|err| AppError::Internal(err.to_string()))?
                .rows_affected()
                    == 1;
                if !consumed {
                    return Ok(None);
                }
            }

            tx.commit()
                .await
                .map_err(|err| AppError::Internal(err.to_string()))?;
            Ok(Some(InviteJoinResult {
                guild_slug: invite.guild_slug,
                guild_name: invite.guild_name,
                default_channel_slug: invite.guild_default_channel_slug,
                icon_storage_key: invite.guild_icon_storage_key,
                already_member: false,
            }))
        }
        DbPool::Sqlite(pool) => {
            let mut tx = pool
                .begin()
                .await
                .map_err(|err| AppError::Internal(err.to_string()))?;

            let invite: Option<InviteWithGuild> = sqlx::query_as(
                "SELECT gi.id,
                        gi.guild_id,
                        g.slug AS guild_slug,
                        g.name AS guild_name,
                        g.default_channel_slug AS guild_default_channel_slug,
                        g.icon_storage_key AS guild_icon_storage_key,
                        gi.code,
                        gi.type AS invite_type,
                        gi.uses_remaining,
                        gi.revoked
                 FROM guild_invites gi
                 JOIN guilds g ON g.id = gi.guild_id
                 WHERE gi.code = ?1
                   AND gi.revoked = 0
                 LIMIT 1",
            )
            .bind(code)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|err| AppError::Internal(err.to_string()))?;

            let Some(invite) = invite else {
                return Ok(None);
            };

            let already_member = sqlx::query_scalar::<_, i64>(
                "SELECT EXISTS(
                    SELECT 1
                    FROM guild_members
                    WHERE guild_id = ?1 AND user_id = ?2
                )",
            )
            .bind(&invite.guild_id)
            .bind(user_id)
            .fetch_one(&mut *tx)
            .await
            .map_err(|err| AppError::Internal(err.to_string()))?
                != 0;

            if already_member {
                tx.commit()
                    .await
                    .map_err(|err| AppError::Internal(err.to_string()))?;
                return Ok(Some(InviteJoinResult {
                    guild_slug: invite.guild_slug,
                    guild_name: invite.guild_name,
                    default_channel_slug: invite.guild_default_channel_slug,
                    icon_storage_key: invite.guild_icon_storage_key,
                    already_member: true,
                }));
            }

            if invite.invite_type == "single_use" && invite.uses_remaining <= 0 {
                return Ok(None);
            }

            let inserted = sqlx::query(
                "INSERT INTO guild_members (guild_id, user_id, joined_at, joined_via_invite_code)
                 VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT DO NOTHING",
            )
            .bind(&invite.guild_id)
            .bind(user_id)
            .bind(joined_at)
            .bind(&invite.code)
            .execute(&mut *tx)
            .await
            .map_err(|err| AppError::Internal(err.to_string()))?
            .rows_affected()
                == 1;

            if !inserted {
                tx.commit()
                    .await
                    .map_err(|err| AppError::Internal(err.to_string()))?;
                return Ok(Some(InviteJoinResult {
                    guild_slug: invite.guild_slug,
                    guild_name: invite.guild_name,
                    default_channel_slug: invite.guild_default_channel_slug,
                    icon_storage_key: invite.guild_icon_storage_key,
                    already_member: true,
                }));
            }

            if invite.invite_type == "single_use" {
                let consumed = sqlx::query(
                    "UPDATE guild_invites
                     SET uses_remaining = uses_remaining - 1
                     WHERE id = ?1
                       AND revoked = 0
                       AND uses_remaining > 0",
                )
                .bind(&invite.id)
                .execute(&mut *tx)
                .await
                .map_err(|err| AppError::Internal(err.to_string()))?
                .rows_affected()
                    == 1;
                if !consumed {
                    return Ok(None);
                }
            }

            tx.commit()
                .await
                .map_err(|err| AppError::Internal(err.to_string()))?;
            Ok(Some(InviteJoinResult {
                guild_slug: invite.guild_slug,
                guild_name: invite.guild_name,
                default_channel_slug: invite.guild_default_channel_slug,
                icon_storage_key: invite.guild_icon_storage_key,
                already_member: false,
            }))
        }
    }
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
