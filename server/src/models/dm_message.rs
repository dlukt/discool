use crate::{AppError, db::DbPool};

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct DmMessage {
    pub id: String,
    pub dm_channel_id: String,
    pub author_user_id: String,
    pub content: String,
    pub is_system: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DmMessageCursor {
    pub created_at: String,
    pub id: String,
}

#[derive(Debug, Clone)]
pub struct DmMessagePage {
    pub messages: Vec<DmMessage>,
    pub has_more: bool,
}

#[allow(clippy::too_many_arguments)]
pub async fn insert_dm_message(
    pool: &DbPool,
    id: &str,
    dm_channel_id: &str,
    author_user_id: &str,
    content: &str,
    is_system: bool,
    created_at: &str,
    updated_at: &str,
) -> Result<bool, AppError> {
    let is_system_value = if is_system { 1_i64 } else { 0_i64 };
    let rows = match pool {
        DbPool::Postgres(pool) => {
            sqlx::query(
                "INSERT INTO dm_messages (id, dm_channel_id, author_user_id, content, is_system, created_at, updated_at)
                 VALUES ($1, $2, $3, $4, $5, $6, $7)
                 ON CONFLICT DO NOTHING",
            )
            .bind(id)
            .bind(dm_channel_id)
            .bind(author_user_id)
            .bind(content)
            .bind(is_system_value)
            .bind(created_at)
            .bind(updated_at)
            .execute(pool)
            .await
            .map(|result| result.rows_affected())
        }
        DbPool::Sqlite(pool) => {
            sqlx::query(
                "INSERT INTO dm_messages (id, dm_channel_id, author_user_id, content, is_system, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                 ON CONFLICT DO NOTHING",
            )
            .bind(id)
            .bind(dm_channel_id)
            .bind(author_user_id)
            .bind(content)
            .bind(is_system_value)
            .bind(created_at)
            .bind(updated_at)
            .execute(pool)
            .await
            .map(|result| result.rows_affected())
        }
    }
    .map_err(|err| AppError::Internal(err.to_string()))?;

    Ok(rows == 1)
}

pub async fn list_messages_page_by_dm_channel_id(
    pool: &DbPool,
    dm_channel_id: &str,
    before: Option<&DmMessageCursor>,
    limit: i64,
) -> Result<DmMessagePage, AppError> {
    let normalized_limit = limit.clamp(1, 200);
    let fetch_limit = normalized_limit + 1;
    let mut messages = match pool {
        DbPool::Postgres(pool) => match before {
            Some(cursor) => {
                sqlx::query_as(
                    "SELECT id, dm_channel_id, author_user_id, content, is_system, created_at, updated_at
                     FROM dm_messages
                     WHERE dm_channel_id = $1
                       AND (created_at < $2 OR (created_at = $2 AND id < $3))
                     ORDER BY created_at DESC, id DESC
                     LIMIT $4",
                )
                .bind(dm_channel_id)
                .bind(&cursor.created_at)
                .bind(&cursor.id)
                .bind(fetch_limit)
                .fetch_all(pool)
                .await
            }
            None => {
                sqlx::query_as(
                    "SELECT id, dm_channel_id, author_user_id, content, is_system, created_at, updated_at
                     FROM dm_messages
                     WHERE dm_channel_id = $1
                     ORDER BY created_at DESC, id DESC
                     LIMIT $2",
                )
                .bind(dm_channel_id)
                .bind(fetch_limit)
                .fetch_all(pool)
                .await
            }
        },
        DbPool::Sqlite(pool) => match before {
            Some(cursor) => {
                sqlx::query_as(
                    "SELECT id, dm_channel_id, author_user_id, content, is_system, created_at, updated_at
                     FROM dm_messages
                     WHERE dm_channel_id = ?1
                       AND (created_at < ?2 OR (created_at = ?2 AND id < ?3))
                     ORDER BY created_at DESC, id DESC
                     LIMIT ?4",
                )
                .bind(dm_channel_id)
                .bind(&cursor.created_at)
                .bind(&cursor.id)
                .bind(fetch_limit)
                .fetch_all(pool)
                .await
            }
            None => {
                sqlx::query_as(
                    "SELECT id, dm_channel_id, author_user_id, content, is_system, created_at, updated_at
                     FROM dm_messages
                     WHERE dm_channel_id = ?1
                     ORDER BY created_at DESC, id DESC
                     LIMIT ?2",
                )
                .bind(dm_channel_id)
                .bind(fetch_limit)
                .fetch_all(pool)
                .await
            }
        },
    }
    .map_err(|err| AppError::Internal(err.to_string()))?;

    let page_limit = usize::try_from(normalized_limit).unwrap_or(200);
    let has_more = messages.len() > page_limit;
    if has_more {
        messages.truncate(page_limit);
    }
    messages.reverse();

    Ok(DmMessagePage { messages, has_more })
}
