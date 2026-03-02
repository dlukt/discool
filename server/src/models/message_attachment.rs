use std::collections::HashMap;

use sqlx::QueryBuilder;

use crate::{AppError, db::DbPool};

#[derive(Debug, Clone, sqlx::FromRow, PartialEq, Eq)]
pub struct MessageAttachment {
    pub id: String,
    pub message_id: String,
    pub storage_key: String,
    pub original_filename: String,
    pub mime_type: String,
    pub size_bytes: i64,
    pub created_at: String,
}

#[allow(clippy::too_many_arguments)]
pub async fn insert_message_attachment(
    pool: &DbPool,
    id: &str,
    message_id: &str,
    storage_key: &str,
    original_filename: &str,
    mime_type: &str,
    size_bytes: i64,
    created_at: &str,
) -> Result<bool, AppError> {
    let rows = match pool {
        DbPool::Postgres(pool) => sqlx::query(
            "INSERT INTO message_attachments (
                    id,
                    message_id,
                    storage_key,
                    original_filename,
                    mime_type,
                    size_bytes,
                    created_at
                ) VALUES ($1, $2, $3, $4, $5, $6, $7)
                ON CONFLICT DO NOTHING",
        )
        .bind(id)
        .bind(message_id)
        .bind(storage_key)
        .bind(original_filename)
        .bind(mime_type)
        .bind(size_bytes)
        .bind(created_at)
        .execute(pool)
        .await
        .map(|result| result.rows_affected()),
        DbPool::Sqlite(pool) => sqlx::query(
            "INSERT INTO message_attachments (
                    id,
                    message_id,
                    storage_key,
                    original_filename,
                    mime_type,
                    size_bytes,
                    created_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                ON CONFLICT DO NOTHING",
        )
        .bind(id)
        .bind(message_id)
        .bind(storage_key)
        .bind(original_filename)
        .bind(mime_type)
        .bind(size_bytes)
        .bind(created_at)
        .execute(pool)
        .await
        .map(|result| result.rows_affected()),
    }
    .map_err(|err| AppError::Internal(err.to_string()))?;

    Ok(rows == 1)
}

pub async fn find_message_attachment_by_id(
    pool: &DbPool,
    attachment_id: &str,
) -> Result<Option<MessageAttachment>, AppError> {
    let attachment = match pool {
        DbPool::Postgres(pool) => {
            sqlx::query_as(
                "SELECT id, message_id, storage_key, original_filename, mime_type, size_bytes, created_at
                 FROM message_attachments
                 WHERE id = $1
                 LIMIT 1",
            )
            .bind(attachment_id)
            .fetch_optional(pool)
            .await
        }
        DbPool::Sqlite(pool) => {
            sqlx::query_as(
                "SELECT id, message_id, storage_key, original_filename, mime_type, size_bytes, created_at
                 FROM message_attachments
                 WHERE id = ?1
                 LIMIT 1",
            )
            .bind(attachment_id)
            .fetch_optional(pool)
            .await
        }
    }
    .map_err(|err| AppError::Internal(err.to_string()))?;

    Ok(attachment)
}

pub async fn list_message_attachments_by_message_ids(
    pool: &DbPool,
    message_ids: &[String],
) -> Result<HashMap<String, Vec<MessageAttachment>>, AppError> {
    if message_ids.is_empty() {
        return Ok(HashMap::new());
    }

    let rows: Vec<MessageAttachment> = match pool {
        DbPool::Postgres(pool) => {
            let mut query_builder = QueryBuilder::<sqlx::Postgres>::new(
                "SELECT id, message_id, storage_key, original_filename, mime_type, size_bytes, created_at
                 FROM message_attachments
                 WHERE message_id IN (",
            );
            let mut separated = query_builder.separated(", ");
            for message_id in message_ids {
                separated.push_bind(message_id);
            }
            query_builder.push(") ORDER BY message_id ASC, created_at ASC, id ASC");

            query_builder
                .build_query_as::<MessageAttachment>()
                .fetch_all(pool)
                .await
                .map_err(|err| AppError::Internal(err.to_string()))?
        }
        DbPool::Sqlite(pool) => {
            let mut query_builder = QueryBuilder::<sqlx::Sqlite>::new(
                "SELECT id, message_id, storage_key, original_filename, mime_type, size_bytes, created_at
                 FROM message_attachments
                 WHERE message_id IN (",
            );
            let mut separated = query_builder.separated(", ");
            for message_id in message_ids {
                separated.push_bind(message_id);
            }
            query_builder.push(") ORDER BY message_id ASC, created_at ASC, id ASC");

            query_builder
                .build_query_as::<MessageAttachment>()
                .fetch_all(pool)
                .await
                .map_err(|err| AppError::Internal(err.to_string()))?
        }
    };

    let mut grouped = HashMap::<String, Vec<MessageAttachment>>::new();
    for row in rows {
        grouped.entry(row.message_id.clone()).or_default().push(row);
    }
    Ok(grouped)
}

pub async fn list_message_attachments_by_author_user_id(
    pool: &DbPool,
    author_user_id: &str,
) -> Result<Vec<MessageAttachment>, AppError> {
    let attachments = match pool {
        DbPool::Postgres(pool) => {
            sqlx::query_as(
                "SELECT ma.id, ma.message_id, ma.storage_key, ma.original_filename, ma.mime_type, ma.size_bytes, ma.created_at
                 FROM message_attachments ma
                 JOIN messages m ON m.id = ma.message_id
                 WHERE m.author_user_id = $1
                   AND m.deleted_at IS NULL
                 ORDER BY ma.created_at ASC, ma.id ASC",
            )
            .bind(author_user_id)
            .fetch_all(pool)
            .await
        }
        DbPool::Sqlite(pool) => {
            sqlx::query_as(
                "SELECT ma.id, ma.message_id, ma.storage_key, ma.original_filename, ma.mime_type, ma.size_bytes, ma.created_at
                 FROM message_attachments ma
                 JOIN messages m ON m.id = ma.message_id
                 WHERE m.author_user_id = ?1
                   AND m.deleted_at IS NULL
                 ORDER BY ma.created_at ASC, ma.id ASC",
            )
            .bind(author_user_id)
            .fetch_all(pool)
            .await
        }
    }
    .map_err(|err| AppError::Internal(err.to_string()))?;

    Ok(attachments)
}
