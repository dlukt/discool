use crate::{AppError, db::DbPool};

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct DmChannel {
    pub id: String,
    pub slug: String,
    pub user_low_id: String,
    pub user_high_id: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct DmChannelListEntry {
    pub id: String,
    pub slug: String,
    pub user_low_id: String,
    pub user_high_id: String,
    pub created_at: String,
    pub updated_at: String,
    pub last_message_content: Option<String>,
    pub last_message_created_at: Option<String>,
}

pub async fn find_dm_channel_by_slug(
    pool: &DbPool,
    slug: &str,
) -> Result<Option<DmChannel>, AppError> {
    let channel = match pool {
        DbPool::Postgres(pool) => {
            sqlx::query_as(
                "SELECT id, slug, user_low_id, user_high_id, created_at, updated_at
                 FROM dm_channels
                 WHERE slug = $1
                 LIMIT 1",
            )
            .bind(slug)
            .fetch_optional(pool)
            .await
        }
        DbPool::Sqlite(pool) => {
            sqlx::query_as(
                "SELECT id, slug, user_low_id, user_high_id, created_at, updated_at
                 FROM dm_channels
                 WHERE slug = ?1
                 LIMIT 1",
            )
            .bind(slug)
            .fetch_optional(pool)
            .await
        }
    }
    .map_err(|err| AppError::Internal(err.to_string()))?;

    Ok(channel)
}

pub async fn find_dm_channel_by_participant_pair(
    pool: &DbPool,
    user_low_id: &str,
    user_high_id: &str,
) -> Result<Option<DmChannel>, AppError> {
    let channel = match pool {
        DbPool::Postgres(pool) => {
            sqlx::query_as(
                "SELECT id, slug, user_low_id, user_high_id, created_at, updated_at
                 FROM dm_channels
                 WHERE user_low_id = $1
                   AND user_high_id = $2
                 LIMIT 1",
            )
            .bind(user_low_id)
            .bind(user_high_id)
            .fetch_optional(pool)
            .await
        }
        DbPool::Sqlite(pool) => {
            sqlx::query_as(
                "SELECT id, slug, user_low_id, user_high_id, created_at, updated_at
                 FROM dm_channels
                 WHERE user_low_id = ?1
                   AND user_high_id = ?2
                 LIMIT 1",
            )
            .bind(user_low_id)
            .bind(user_high_id)
            .fetch_optional(pool)
            .await
        }
    }
    .map_err(|err| AppError::Internal(err.to_string()))?;

    Ok(channel)
}

#[allow(clippy::too_many_arguments)]
pub async fn insert_dm_channel(
    pool: &DbPool,
    id: &str,
    slug: &str,
    user_low_id: &str,
    user_high_id: &str,
    created_at: &str,
    updated_at: &str,
) -> Result<bool, AppError> {
    let rows = match pool {
        DbPool::Postgres(pool) => sqlx::query(
            "INSERT INTO dm_channels (id, slug, user_low_id, user_high_id, created_at, updated_at)
                 VALUES ($1, $2, $3, $4, $5, $6)
                 ON CONFLICT DO NOTHING",
        )
        .bind(id)
        .bind(slug)
        .bind(user_low_id)
        .bind(user_high_id)
        .bind(created_at)
        .bind(updated_at)
        .execute(pool)
        .await
        .map(|result| result.rows_affected()),
        DbPool::Sqlite(pool) => sqlx::query(
            "INSERT INTO dm_channels (id, slug, user_low_id, user_high_id, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                 ON CONFLICT DO NOTHING",
        )
        .bind(id)
        .bind(slug)
        .bind(user_low_id)
        .bind(user_high_id)
        .bind(created_at)
        .bind(updated_at)
        .execute(pool)
        .await
        .map(|result| result.rows_affected()),
    }
    .map_err(|err| AppError::Internal(err.to_string()))?;

    Ok(rows == 1)
}

pub async fn touch_dm_channel(
    pool: &DbPool,
    channel_id: &str,
    updated_at: &str,
) -> Result<bool, AppError> {
    let rows = match pool {
        DbPool::Postgres(pool) => sqlx::query(
            "UPDATE dm_channels
                 SET updated_at = $1
                 WHERE id = $2",
        )
        .bind(updated_at)
        .bind(channel_id)
        .execute(pool)
        .await
        .map(|result| result.rows_affected()),
        DbPool::Sqlite(pool) => sqlx::query(
            "UPDATE dm_channels
                 SET updated_at = ?1
                 WHERE id = ?2",
        )
        .bind(updated_at)
        .bind(channel_id)
        .execute(pool)
        .await
        .map(|result| result.rows_affected()),
    }
    .map_err(|err| AppError::Internal(err.to_string()))?;

    Ok(rows == 1)
}

pub async fn list_dm_channels_for_user(
    pool: &DbPool,
    user_id: &str,
) -> Result<Vec<DmChannelListEntry>, AppError> {
    let channels = match pool {
        DbPool::Postgres(pool) => {
            sqlx::query_as(
                "SELECT dc.id,
                        dc.slug,
                        dc.user_low_id,
                        dc.user_high_id,
                        dc.created_at,
                        dc.updated_at,
                        (
                            SELECT dm.content
                            FROM dm_messages dm
                            WHERE dm.dm_channel_id = dc.id
                            ORDER BY dm.created_at DESC, dm.id DESC
                            LIMIT 1
                        ) AS last_message_content,
                        (
                            SELECT dm.created_at
                            FROM dm_messages dm
                            WHERE dm.dm_channel_id = dc.id
                            ORDER BY dm.created_at DESC, dm.id DESC
                            LIMIT 1
                        ) AS last_message_created_at
                 FROM dm_channels dc
                 WHERE dc.user_low_id = $1
                    OR dc.user_high_id = $1
                 ORDER BY COALESCE(
                            (
                                SELECT dm.created_at
                                FROM dm_messages dm
                                WHERE dm.dm_channel_id = dc.id
                                ORDER BY dm.created_at DESC, dm.id DESC
                                LIMIT 1
                            ),
                            dc.updated_at
                        ) DESC,
                        dc.created_at DESC",
            )
            .bind(user_id)
            .fetch_all(pool)
            .await
        }
        DbPool::Sqlite(pool) => {
            sqlx::query_as(
                "SELECT dc.id,
                        dc.slug,
                        dc.user_low_id,
                        dc.user_high_id,
                        dc.created_at,
                        dc.updated_at,
                        (
                            SELECT dm.content
                            FROM dm_messages dm
                            WHERE dm.dm_channel_id = dc.id
                            ORDER BY dm.created_at DESC, dm.id DESC
                            LIMIT 1
                        ) AS last_message_content,
                        (
                            SELECT dm.created_at
                            FROM dm_messages dm
                            WHERE dm.dm_channel_id = dc.id
                            ORDER BY dm.created_at DESC, dm.id DESC
                            LIMIT 1
                        ) AS last_message_created_at
                 FROM dm_channels dc
                 WHERE dc.user_low_id = ?1
                    OR dc.user_high_id = ?1
                 ORDER BY COALESCE(
                            (
                                SELECT dm.created_at
                                FROM dm_messages dm
                                WHERE dm.dm_channel_id = dc.id
                                ORDER BY dm.created_at DESC, dm.id DESC
                                LIMIT 1
                            ),
                            dc.updated_at
                        ) DESC,
                        dc.created_at DESC",
            )
            .bind(user_id)
            .fetch_all(pool)
            .await
        }
    }
    .map_err(|err| AppError::Internal(err.to_string()))?;

    Ok(channels)
}
