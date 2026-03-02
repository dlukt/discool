use crate::{AppError, db::DbPool};

pub const MODERATION_ACTION_TYPE_MUTE: &str = "mute";
pub const MODERATION_ACTION_TYPE_KICK: &str = "kick";
pub const MODERATION_ACTION_TYPE_BAN: &str = "ban";

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

async fn postgres_actor_outranks_target_member_in_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    guild_id: &str,
    owner_id: &str,
    actor_user_id: &str,
    target_user_id: &str,
) -> Result<bool, AppError> {
    if actor_user_id == target_user_id {
        return Ok(false);
    }

    let target_is_member = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(
             SELECT 1
             FROM guild_members
             WHERE guild_id = $1 AND user_id = $2
         )",
    )
    .bind(guild_id)
    .bind(target_user_id)
    .fetch_one(&mut **tx)
    .await
    .map_err(|err| AppError::Internal(err.to_string()))?;

    if actor_user_id == owner_id {
        return Ok(true);
    }
    if target_user_id == owner_id {
        return Ok(false);
    }

    let actor_is_member = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(
             SELECT 1
             FROM guild_members
             WHERE guild_id = $1 AND user_id = $2
         )",
    )
    .bind(guild_id)
    .bind(actor_user_id)
    .fetch_one(&mut **tx)
    .await
    .map_err(|err| AppError::Internal(err.to_string()))?;
    if !actor_is_member {
        return Ok(false);
    }
    if !target_is_member {
        return Ok(true);
    }

    let default_position = sqlx::query_scalar::<_, i64>(
        "SELECT COALESCE(
             (SELECT position
              FROM roles
              WHERE guild_id = $1 AND is_default = 1
              LIMIT 1),
             $2
         )",
    )
    .bind(guild_id)
    .bind(i64::MAX)
    .fetch_one(&mut **tx)
    .await
    .map_err(|err| AppError::Internal(err.to_string()))?;

    let actor_min_position = sqlx::query_scalar::<_, Option<i64>>(
        "SELECT MIN(r.position)
         FROM role_assignments ra
         JOIN roles r ON r.id = ra.role_id
         WHERE ra.guild_id = $1
           AND ra.user_id = $2
           AND r.guild_id = $1",
    )
    .bind(guild_id)
    .bind(actor_user_id)
    .fetch_one(&mut **tx)
    .await
    .map_err(|err| AppError::Internal(err.to_string()))?;
    let target_min_position = sqlx::query_scalar::<_, Option<i64>>(
        "SELECT MIN(r.position)
         FROM role_assignments ra
         JOIN roles r ON r.id = ra.role_id
         WHERE ra.guild_id = $1
           AND ra.user_id = $2
           AND r.guild_id = $1",
    )
    .bind(guild_id)
    .bind(target_user_id)
    .fetch_one(&mut **tx)
    .await
    .map_err(|err| AppError::Internal(err.to_string()))?;

    let actor_position =
        actor_min_position.map_or(default_position, |value| value.min(default_position));
    let target_position =
        target_min_position.map_or(default_position, |value| value.min(default_position));
    Ok(actor_position < target_position)
}

async fn sqlite_actor_outranks_target_member_in_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    guild_id: &str,
    owner_id: &str,
    actor_user_id: &str,
    target_user_id: &str,
) -> Result<bool, AppError> {
    if actor_user_id == target_user_id {
        return Ok(false);
    }

    let target_is_member = sqlx::query_scalar::<_, i64>(
        "SELECT EXISTS(
             SELECT 1
             FROM guild_members
             WHERE guild_id = ?1 AND user_id = ?2
         )",
    )
    .bind(guild_id)
    .bind(target_user_id)
    .fetch_one(&mut **tx)
    .await
    .map_err(|err| AppError::Internal(err.to_string()))?
        != 0;

    if actor_user_id == owner_id {
        return Ok(true);
    }
    if target_user_id == owner_id {
        return Ok(false);
    }

    let actor_is_member = sqlx::query_scalar::<_, i64>(
        "SELECT EXISTS(
             SELECT 1
             FROM guild_members
             WHERE guild_id = ?1 AND user_id = ?2
         )",
    )
    .bind(guild_id)
    .bind(actor_user_id)
    .fetch_one(&mut **tx)
    .await
    .map_err(|err| AppError::Internal(err.to_string()))?
        != 0;
    if !actor_is_member {
        return Ok(false);
    }
    if !target_is_member {
        return Ok(true);
    }

    let default_position = sqlx::query_scalar::<_, i64>(
        "SELECT COALESCE(
             (SELECT position
              FROM roles
              WHERE guild_id = ?1 AND is_default = 1
              LIMIT 1),
             ?2
         )",
    )
    .bind(guild_id)
    .bind(i64::MAX)
    .fetch_one(&mut **tx)
    .await
    .map_err(|err| AppError::Internal(err.to_string()))?;

    let actor_min_position = sqlx::query_scalar::<_, Option<i64>>(
        "SELECT MIN(r.position)
         FROM role_assignments ra
         JOIN roles r ON r.id = ra.role_id
         WHERE ra.guild_id = ?1
           AND ra.user_id = ?2
           AND r.guild_id = ?1",
    )
    .bind(guild_id)
    .bind(actor_user_id)
    .fetch_one(&mut **tx)
    .await
    .map_err(|err| AppError::Internal(err.to_string()))?;
    let target_min_position = sqlx::query_scalar::<_, Option<i64>>(
        "SELECT MIN(r.position)
         FROM role_assignments ra
         JOIN roles r ON r.id = ra.role_id
         WHERE ra.guild_id = ?1
           AND ra.user_id = ?2
           AND r.guild_id = ?1",
    )
    .bind(guild_id)
    .bind(target_user_id)
    .fetch_one(&mut **tx)
    .await
    .map_err(|err| AppError::Internal(err.to_string()))?;

    let actor_position =
        actor_min_position.map_or(default_position, |value| value.min(default_position));
    let target_position =
        target_min_position.map_or(default_position, |value| value.min(default_position));
    Ok(actor_position < target_position)
}

#[allow(clippy::too_many_arguments)]
pub async fn apply_kick_action(
    pool: &DbPool,
    id: &str,
    guild_id: &str,
    actor_user_id: &str,
    target_user_id: &str,
    reason: &str,
    now: &str,
) -> Result<(), AppError> {
    match pool {
        DbPool::Postgres(pool) => {
            let mut tx = pool
                .begin()
                .await
                .map_err(|err| AppError::Internal(err.to_string()))?;
            let owner_id = sqlx::query_scalar::<_, String>(
                "SELECT owner_id
                 FROM guilds
                 WHERE id = $1
                 LIMIT 1",
            )
            .bind(guild_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|err| AppError::Internal(err.to_string()))?
            .ok_or(AppError::NotFound)?;
            if target_user_id == owner_id {
                return Err(AppError::Forbidden(
                    "Cannot kick the guild owner".to_string(),
                ));
            }
            if !postgres_actor_outranks_target_member_in_tx(
                &mut tx,
                guild_id,
                &owner_id,
                actor_user_id,
                target_user_id,
            )
            .await?
            {
                return Err(AppError::Forbidden(
                    "You can only kick members below your highest role".to_string(),
                ));
            }
            sqlx::query(
                "UPDATE moderation_actions
                     SET is_active = 0,
                         updated_at = $1
                     WHERE guild_id = $2
                       AND target_user_id = $3
                       AND action_type = $4
                       AND is_active = 1",
            )
            .bind(now)
            .bind(guild_id)
            .bind(target_user_id)
            .bind(MODERATION_ACTION_TYPE_MUTE)
            .execute(&mut *tx)
            .await
            .map_err(|err| AppError::Internal(err.to_string()))?;

            sqlx::query(
                "DELETE FROM role_assignments
                     WHERE guild_id = $1 AND user_id = $2",
            )
            .bind(guild_id)
            .bind(target_user_id)
            .execute(&mut *tx)
            .await
            .map_err(|err| AppError::Internal(err.to_string()))?;

            let removed_membership = sqlx::query(
                "DELETE FROM guild_members
                     WHERE guild_id = $1 AND user_id = $2",
            )
            .bind(guild_id)
            .bind(target_user_id)
            .execute(&mut *tx)
            .await
            .map_err(|err| AppError::Internal(err.to_string()))?
            .rows_affected();
            if removed_membership != 1 {
                return Err(AppError::ValidationError(
                    "target_user_id must belong to a guild member".to_string(),
                ));
            }

            sqlx::query(
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
            .bind(MODERATION_ACTION_TYPE_KICK)
            .bind(guild_id)
            .bind(actor_user_id)
            .bind(target_user_id)
            .bind(reason)
            .bind(None::<i64>)
            .bind(None::<&str>)
            .bind(0_i64)
            .bind(now)
            .bind(now)
            .execute(&mut *tx)
            .await
            .map_err(|err| AppError::Internal(err.to_string()))?;

            tx.commit()
                .await
                .map_err(|err| AppError::Internal(err.to_string()))?;
        }
        DbPool::Sqlite(pool) => {
            let mut tx = pool
                .begin()
                .await
                .map_err(|err| AppError::Internal(err.to_string()))?;
            let owner_id = sqlx::query_scalar::<_, String>(
                "SELECT owner_id
                 FROM guilds
                 WHERE id = ?1
                 LIMIT 1",
            )
            .bind(guild_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|err| AppError::Internal(err.to_string()))?
            .ok_or(AppError::NotFound)?;
            if target_user_id == owner_id {
                return Err(AppError::Forbidden(
                    "Cannot kick the guild owner".to_string(),
                ));
            }
            if !sqlite_actor_outranks_target_member_in_tx(
                &mut tx,
                guild_id,
                &owner_id,
                actor_user_id,
                target_user_id,
            )
            .await?
            {
                return Err(AppError::Forbidden(
                    "You can only kick members below your highest role".to_string(),
                ));
            }
            sqlx::query(
                "UPDATE moderation_actions
                     SET is_active = 0,
                         updated_at = ?1
                     WHERE guild_id = ?2
                       AND target_user_id = ?3
                       AND action_type = ?4
                       AND is_active = 1",
            )
            .bind(now)
            .bind(guild_id)
            .bind(target_user_id)
            .bind(MODERATION_ACTION_TYPE_MUTE)
            .execute(&mut *tx)
            .await
            .map_err(|err| AppError::Internal(err.to_string()))?;

            sqlx::query(
                "DELETE FROM role_assignments
                     WHERE guild_id = ?1 AND user_id = ?2",
            )
            .bind(guild_id)
            .bind(target_user_id)
            .execute(&mut *tx)
            .await
            .map_err(|err| AppError::Internal(err.to_string()))?;

            let removed_membership = sqlx::query(
                "DELETE FROM guild_members
                     WHERE guild_id = ?1 AND user_id = ?2",
            )
            .bind(guild_id)
            .bind(target_user_id)
            .execute(&mut *tx)
            .await
            .map_err(|err| AppError::Internal(err.to_string()))?
            .rows_affected();
            if removed_membership != 1 {
                return Err(AppError::ValidationError(
                    "target_user_id must belong to a guild member".to_string(),
                ));
            }

            sqlx::query(
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
            .bind(MODERATION_ACTION_TYPE_KICK)
            .bind(guild_id)
            .bind(actor_user_id)
            .bind(target_user_id)
            .bind(reason)
            .bind(None::<i64>)
            .bind(None::<&str>)
            .bind(0_i64)
            .bind(now)
            .bind(now)
            .execute(&mut *tx)
            .await
            .map_err(|err| AppError::Internal(err.to_string()))?;

            tx.commit()
                .await
                .map_err(|err| AppError::Internal(err.to_string()))?;
        }
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub async fn apply_ban_action(
    pool: &DbPool,
    id: &str,
    ban_id: &str,
    guild_id: &str,
    actor_user_id: &str,
    target_user_id: &str,
    reason: &str,
    delete_messages_window_seconds: Option<i64>,
    now: &str,
) -> Result<(), AppError> {
    match pool {
        DbPool::Postgres(pool) => {
            let mut tx = pool
                .begin()
                .await
                .map_err(|err| AppError::Internal(err.to_string()))?;
            let owner_id = sqlx::query_scalar::<_, String>(
                "SELECT owner_id
                 FROM guilds
                 WHERE id = $1
                 LIMIT 1",
            )
            .bind(guild_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|err| AppError::Internal(err.to_string()))?
            .ok_or(AppError::NotFound)?;
            if target_user_id == owner_id {
                return Err(AppError::Forbidden(
                    "Cannot ban the guild owner".to_string(),
                ));
            }
            if !postgres_actor_outranks_target_member_in_tx(
                &mut tx,
                guild_id,
                &owner_id,
                actor_user_id,
                target_user_id,
            )
            .await?
            {
                return Err(AppError::Forbidden(
                    "You can only ban members below your highest role".to_string(),
                ));
            }
            sqlx::query(
                "UPDATE moderation_actions
                     SET is_active = 0,
                         updated_at = $1
                     WHERE guild_id = $2
                       AND target_user_id = $3
                       AND action_type = $4
                       AND is_active = 1",
            )
            .bind(now)
            .bind(guild_id)
            .bind(target_user_id)
            .bind(MODERATION_ACTION_TYPE_MUTE)
            .execute(&mut *tx)
            .await
            .map_err(|err| AppError::Internal(err.to_string()))?;

            sqlx::query(
                "DELETE FROM role_assignments
                     WHERE guild_id = $1 AND user_id = $2",
            )
            .bind(guild_id)
            .bind(target_user_id)
            .execute(&mut *tx)
            .await
            .map_err(|err| AppError::Internal(err.to_string()))?;

            let removed_membership = sqlx::query(
                "DELETE FROM guild_members
                     WHERE guild_id = $1 AND user_id = $2",
            )
            .bind(guild_id)
            .bind(target_user_id)
            .execute(&mut *tx)
            .await
            .map_err(|err| AppError::Internal(err.to_string()))?
            .rows_affected();
            if removed_membership != 1 {
                return Err(AppError::ValidationError(
                    "target_user_id must belong to a guild member".to_string(),
                ));
            }

            sqlx::query(
                "INSERT INTO guild_bans (
                        id,
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
                    ) VALUES ($1, $2, $3, $4, $5, $6, 1, $7, $8, NULL, NULL)",
            )
            .bind(ban_id)
            .bind(guild_id)
            .bind(target_user_id)
            .bind(actor_user_id)
            .bind(reason)
            .bind(delete_messages_window_seconds)
            .bind(now)
            .bind(now)
            .execute(&mut *tx)
            .await
            .map_err(|err| AppError::Internal(err.to_string()))?;

            sqlx::query(
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
            .bind(MODERATION_ACTION_TYPE_BAN)
            .bind(guild_id)
            .bind(actor_user_id)
            .bind(target_user_id)
            .bind(reason)
            .bind(None::<i64>)
            .bind(None::<&str>)
            .bind(0_i64)
            .bind(now)
            .bind(now)
            .execute(&mut *tx)
            .await
            .map_err(|err| AppError::Internal(err.to_string()))?;

            tx.commit()
                .await
                .map_err(|err| AppError::Internal(err.to_string()))?;
        }
        DbPool::Sqlite(pool) => {
            let mut tx = pool
                .begin()
                .await
                .map_err(|err| AppError::Internal(err.to_string()))?;
            let owner_id = sqlx::query_scalar::<_, String>(
                "SELECT owner_id
                 FROM guilds
                 WHERE id = ?1
                 LIMIT 1",
            )
            .bind(guild_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|err| AppError::Internal(err.to_string()))?
            .ok_or(AppError::NotFound)?;
            if target_user_id == owner_id {
                return Err(AppError::Forbidden(
                    "Cannot ban the guild owner".to_string(),
                ));
            }
            if !sqlite_actor_outranks_target_member_in_tx(
                &mut tx,
                guild_id,
                &owner_id,
                actor_user_id,
                target_user_id,
            )
            .await?
            {
                return Err(AppError::Forbidden(
                    "You can only ban members below your highest role".to_string(),
                ));
            }
            sqlx::query(
                "UPDATE moderation_actions
                     SET is_active = 0,
                         updated_at = ?1
                     WHERE guild_id = ?2
                       AND target_user_id = ?3
                       AND action_type = ?4
                       AND is_active = 1",
            )
            .bind(now)
            .bind(guild_id)
            .bind(target_user_id)
            .bind(MODERATION_ACTION_TYPE_MUTE)
            .execute(&mut *tx)
            .await
            .map_err(|err| AppError::Internal(err.to_string()))?;

            sqlx::query(
                "DELETE FROM role_assignments
                     WHERE guild_id = ?1 AND user_id = ?2",
            )
            .bind(guild_id)
            .bind(target_user_id)
            .execute(&mut *tx)
            .await
            .map_err(|err| AppError::Internal(err.to_string()))?;

            let removed_membership = sqlx::query(
                "DELETE FROM guild_members
                     WHERE guild_id = ?1 AND user_id = ?2",
            )
            .bind(guild_id)
            .bind(target_user_id)
            .execute(&mut *tx)
            .await
            .map_err(|err| AppError::Internal(err.to_string()))?
            .rows_affected();
            if removed_membership != 1 {
                return Err(AppError::ValidationError(
                    "target_user_id must belong to a guild member".to_string(),
                ));
            }

            sqlx::query(
                "INSERT INTO guild_bans (
                        id,
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
                    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 1, ?7, ?8, NULL, NULL)",
            )
            .bind(ban_id)
            .bind(guild_id)
            .bind(target_user_id)
            .bind(actor_user_id)
            .bind(reason)
            .bind(delete_messages_window_seconds)
            .bind(now)
            .bind(now)
            .execute(&mut *tx)
            .await
            .map_err(|err| AppError::Internal(err.to_string()))?;

            sqlx::query(
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
            .bind(MODERATION_ACTION_TYPE_BAN)
            .bind(guild_id)
            .bind(actor_user_id)
            .bind(target_user_id)
            .bind(reason)
            .bind(None::<i64>)
            .bind(None::<&str>)
            .bind(0_i64)
            .bind(now)
            .bind(now)
            .execute(&mut *tx)
            .await
            .map_err(|err| AppError::Internal(err.to_string()))?;

            tx.commit()
                .await
                .map_err(|err| AppError::Internal(err.to_string()))?;
        }
    }

    Ok(())
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
