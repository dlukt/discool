use crate::{AppError, db::DbPool};

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct UserBlockRecord {
    pub id: String,
    pub owner_user_id: String,
    pub blocked_user_id: String,
    pub blocked_at: String,
    pub unblocked_at: Option<String>,
    pub blocked_user_display_name: Option<String>,
    pub blocked_user_username: Option<String>,
    pub blocked_user_avatar_color: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

pub async fn list_user_blocks_for_owner(
    pool: &DbPool,
    owner_user_id: &str,
) -> Result<Vec<UserBlockRecord>, AppError> {
    let records = match pool {
        DbPool::Postgres(pool) => {
            sqlx::query_as(
                "SELECT id,
                        owner_user_id,
                        blocked_user_id,
                        blocked_at,
                        unblocked_at,
                        blocked_user_display_name,
                        blocked_user_username,
                        blocked_user_avatar_color,
                        created_at,
                        updated_at
                 FROM user_blocks
                 WHERE owner_user_id = $1
                 ORDER BY blocked_at DESC, id DESC",
            )
            .bind(owner_user_id)
            .fetch_all(pool)
            .await
        }
        DbPool::Sqlite(pool) => {
            sqlx::query_as(
                "SELECT id,
                        owner_user_id,
                        blocked_user_id,
                        blocked_at,
                        unblocked_at,
                        blocked_user_display_name,
                        blocked_user_username,
                        blocked_user_avatar_color,
                        created_at,
                        updated_at
                 FROM user_blocks
                 WHERE owner_user_id = ?1
                 ORDER BY blocked_at DESC, id DESC",
            )
            .bind(owner_user_id)
            .fetch_all(pool)
            .await
        }
    }
    .map_err(|err| AppError::Internal(err.to_string()))?;

    Ok(records)
}

pub async fn find_user_block_by_id(
    pool: &DbPool,
    id: &str,
) -> Result<Option<UserBlockRecord>, AppError> {
    let record = match pool {
        DbPool::Postgres(pool) => {
            sqlx::query_as(
                "SELECT id,
                        owner_user_id,
                        blocked_user_id,
                        blocked_at,
                        unblocked_at,
                        blocked_user_display_name,
                        blocked_user_username,
                        blocked_user_avatar_color,
                        created_at,
                        updated_at
                 FROM user_blocks
                 WHERE id = $1
                 LIMIT 1",
            )
            .bind(id)
            .fetch_optional(pool)
            .await
        }
        DbPool::Sqlite(pool) => {
            sqlx::query_as(
                "SELECT id,
                        owner_user_id,
                        blocked_user_id,
                        blocked_at,
                        unblocked_at,
                        blocked_user_display_name,
                        blocked_user_username,
                        blocked_user_avatar_color,
                        created_at,
                        updated_at
                 FROM user_blocks
                 WHERE id = ?1
                 LIMIT 1",
            )
            .bind(id)
            .fetch_optional(pool)
            .await
        }
    }
    .map_err(|err| AppError::Internal(err.to_string()))?;

    Ok(record)
}

pub async fn find_active_user_block(
    pool: &DbPool,
    owner_user_id: &str,
    blocked_user_id: &str,
) -> Result<Option<UserBlockRecord>, AppError> {
    let record = match pool {
        DbPool::Postgres(pool) => {
            sqlx::query_as(
                "SELECT id,
                        owner_user_id,
                        blocked_user_id,
                        blocked_at,
                        unblocked_at,
                        blocked_user_display_name,
                        blocked_user_username,
                        blocked_user_avatar_color,
                        created_at,
                        updated_at
                 FROM user_blocks
                 WHERE owner_user_id = $1
                   AND blocked_user_id = $2
                   AND unblocked_at IS NULL
                 ORDER BY blocked_at DESC, id DESC
                 LIMIT 1",
            )
            .bind(owner_user_id)
            .bind(blocked_user_id)
            .fetch_optional(pool)
            .await
        }
        DbPool::Sqlite(pool) => {
            sqlx::query_as(
                "SELECT id,
                        owner_user_id,
                        blocked_user_id,
                        blocked_at,
                        unblocked_at,
                        blocked_user_display_name,
                        blocked_user_username,
                        blocked_user_avatar_color,
                        created_at,
                        updated_at
                 FROM user_blocks
                 WHERE owner_user_id = ?1
                   AND blocked_user_id = ?2
                   AND unblocked_at IS NULL
                 ORDER BY blocked_at DESC, id DESC
                 LIMIT 1",
            )
            .bind(owner_user_id)
            .bind(blocked_user_id)
            .fetch_optional(pool)
            .await
        }
    }
    .map_err(|err| AppError::Internal(err.to_string()))?;

    Ok(record)
}

#[allow(clippy::too_many_arguments)]
pub async fn insert_user_block(
    pool: &DbPool,
    id: &str,
    owner_user_id: &str,
    blocked_user_id: &str,
    blocked_at: &str,
    blocked_user_display_name: Option<&str>,
    blocked_user_username: Option<&str>,
    blocked_user_avatar_color: Option<&str>,
    created_at: &str,
    updated_at: &str,
) -> Result<(), AppError> {
    match pool {
        DbPool::Postgres(pool) => sqlx::query(
            "INSERT INTO user_blocks (
                    id,
                    owner_user_id,
                    blocked_user_id,
                    blocked_at,
                    unblocked_at,
                    blocked_user_display_name,
                    blocked_user_username,
                    blocked_user_avatar_color,
                    created_at,
                    updated_at
                ) VALUES ($1, $2, $3, $4, NULL, $5, $6, $7, $8, $9)",
        )
        .bind(id)
        .bind(owner_user_id)
        .bind(blocked_user_id)
        .bind(blocked_at)
        .bind(blocked_user_display_name)
        .bind(blocked_user_username)
        .bind(blocked_user_avatar_color)
        .bind(created_at)
        .bind(updated_at)
        .execute(pool)
        .await
        .map(|_| ()),
        DbPool::Sqlite(pool) => sqlx::query(
            "INSERT INTO user_blocks (
                    id,
                    owner_user_id,
                    blocked_user_id,
                    blocked_at,
                    unblocked_at,
                    blocked_user_display_name,
                    blocked_user_username,
                    blocked_user_avatar_color,
                    created_at,
                    updated_at
                ) VALUES (?1, ?2, ?3, ?4, NULL, ?5, ?6, ?7, ?8, ?9)",
        )
        .bind(id)
        .bind(owner_user_id)
        .bind(blocked_user_id)
        .bind(blocked_at)
        .bind(blocked_user_display_name)
        .bind(blocked_user_username)
        .bind(blocked_user_avatar_color)
        .bind(created_at)
        .bind(updated_at)
        .execute(pool)
        .await
        .map(|_| ()),
    }
    .map_err(|err| AppError::Internal(err.to_string()))?;

    Ok(())
}

pub async fn unblock_active_user_blocks(
    pool: &DbPool,
    owner_user_id: &str,
    blocked_user_id: &str,
    unblocked_at: &str,
) -> Result<u64, AppError> {
    let rows_affected = match pool {
        DbPool::Postgres(pool) => sqlx::query(
            "UPDATE user_blocks
                 SET unblocked_at = $1,
                     updated_at = $1
                 WHERE owner_user_id = $2
                   AND blocked_user_id = $3
                   AND unblocked_at IS NULL",
        )
        .bind(unblocked_at)
        .bind(owner_user_id)
        .bind(blocked_user_id)
        .execute(pool)
        .await
        .map(|result| result.rows_affected()),
        DbPool::Sqlite(pool) => sqlx::query(
            "UPDATE user_blocks
                 SET unblocked_at = ?1,
                     updated_at = ?1
                 WHERE owner_user_id = ?2
                   AND blocked_user_id = ?3
                   AND unblocked_at IS NULL",
        )
        .bind(unblocked_at)
        .bind(owner_user_id)
        .bind(blocked_user_id)
        .execute(pool)
        .await
        .map(|result| result.rows_affected()),
    }
    .map_err(|err| AppError::Internal(err.to_string()))?;

    Ok(rows_affected)
}
