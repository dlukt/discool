use std::collections::HashMap;

use sqlx::QueryBuilder;

use crate::{AppError, db::DbPool};

#[derive(Debug, Clone, sqlx::FromRow, PartialEq, Eq)]
pub struct MessageEmbed {
    pub id: String,
    pub message_id: String,
    pub url: String,
    pub normalized_url: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub thumbnail_url: Option<String>,
    pub domain: String,
    pub created_at: String,
}

#[derive(Debug, Clone, sqlx::FromRow, PartialEq, Eq)]
pub struct EmbedUrlCache {
    pub normalized_url: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub thumbnail_url: Option<String>,
    pub domain: String,
    pub fetched_at: String,
    pub updated_at: String,
}

#[allow(clippy::too_many_arguments)]
pub async fn insert_message_embed(
    pool: &DbPool,
    id: &str,
    message_id: &str,
    url: &str,
    normalized_url: &str,
    title: Option<&str>,
    description: Option<&str>,
    thumbnail_url: Option<&str>,
    domain: &str,
    created_at: &str,
) -> Result<bool, AppError> {
    let rows = match pool {
        DbPool::Postgres(pool) => sqlx::query(
            "INSERT INTO message_embeds (
                    id,
                    message_id,
                    url,
                    normalized_url,
                    title,
                    description,
                    thumbnail_url,
                    domain,
                    created_at
                ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
                ON CONFLICT DO NOTHING",
        )
        .bind(id)
        .bind(message_id)
        .bind(url)
        .bind(normalized_url)
        .bind(title)
        .bind(description)
        .bind(thumbnail_url)
        .bind(domain)
        .bind(created_at)
        .execute(pool)
        .await
        .map(|result| result.rows_affected()),
        DbPool::Sqlite(pool) => sqlx::query(
            "INSERT INTO message_embeds (
                    id,
                    message_id,
                    url,
                    normalized_url,
                    title,
                    description,
                    thumbnail_url,
                    domain,
                    created_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
                ON CONFLICT DO NOTHING",
        )
        .bind(id)
        .bind(message_id)
        .bind(url)
        .bind(normalized_url)
        .bind(title)
        .bind(description)
        .bind(thumbnail_url)
        .bind(domain)
        .bind(created_at)
        .execute(pool)
        .await
        .map(|result| result.rows_affected()),
    }
    .map_err(|err| AppError::Internal(err.to_string()))?;

    Ok(rows == 1)
}

pub async fn delete_message_embeds_by_message_id(
    pool: &DbPool,
    message_id: &str,
) -> Result<u64, AppError> {
    let rows = match pool {
        DbPool::Postgres(pool) => sqlx::query("DELETE FROM message_embeds WHERE message_id = $1")
            .bind(message_id)
            .execute(pool)
            .await
            .map(|result| result.rows_affected()),
        DbPool::Sqlite(pool) => sqlx::query("DELETE FROM message_embeds WHERE message_id = ?1")
            .bind(message_id)
            .execute(pool)
            .await
            .map(|result| result.rows_affected()),
    }
    .map_err(|err| AppError::Internal(err.to_string()))?;
    Ok(rows)
}

pub async fn list_message_embeds_by_message_ids(
    pool: &DbPool,
    message_ids: &[String],
) -> Result<HashMap<String, Vec<MessageEmbed>>, AppError> {
    if message_ids.is_empty() {
        return Ok(HashMap::new());
    }

    let rows: Vec<MessageEmbed> = match pool {
        DbPool::Postgres(pool) => {
            let mut query_builder = QueryBuilder::<sqlx::Postgres>::new(
                "SELECT id, message_id, url, normalized_url, title, description, thumbnail_url, domain, created_at
                 FROM message_embeds
                 WHERE message_id IN (",
            );
            let mut separated = query_builder.separated(", ");
            for message_id in message_ids {
                separated.push_bind(message_id);
            }
            query_builder.push(") ORDER BY message_id ASC, created_at ASC, id ASC");

            query_builder
                .build_query_as::<MessageEmbed>()
                .fetch_all(pool)
                .await
                .map_err(|err| AppError::Internal(err.to_string()))?
        }
        DbPool::Sqlite(pool) => {
            let mut query_builder = QueryBuilder::<sqlx::Sqlite>::new(
                "SELECT id, message_id, url, normalized_url, title, description, thumbnail_url, domain, created_at
                 FROM message_embeds
                 WHERE message_id IN (",
            );
            let mut separated = query_builder.separated(", ");
            for message_id in message_ids {
                separated.push_bind(message_id);
            }
            query_builder.push(") ORDER BY message_id ASC, created_at ASC, id ASC");

            query_builder
                .build_query_as::<MessageEmbed>()
                .fetch_all(pool)
                .await
                .map_err(|err| AppError::Internal(err.to_string()))?
        }
    };

    let mut grouped = HashMap::<String, Vec<MessageEmbed>>::new();
    for row in rows {
        grouped.entry(row.message_id.clone()).or_default().push(row);
    }
    Ok(grouped)
}

pub async fn find_embed_url_cache_by_normalized_url(
    pool: &DbPool,
    normalized_url: &str,
) -> Result<Option<EmbedUrlCache>, AppError> {
    let cache = match pool {
        DbPool::Postgres(pool) => {
            sqlx::query_as(
                "SELECT normalized_url, title, description, thumbnail_url, domain, fetched_at, updated_at
                 FROM embed_url_cache
                 WHERE normalized_url = $1
                 LIMIT 1",
            )
            .bind(normalized_url)
            .fetch_optional(pool)
            .await
        }
        DbPool::Sqlite(pool) => {
            sqlx::query_as(
                "SELECT normalized_url, title, description, thumbnail_url, domain, fetched_at, updated_at
                 FROM embed_url_cache
                 WHERE normalized_url = ?1
                 LIMIT 1",
            )
            .bind(normalized_url)
            .fetch_optional(pool)
            .await
        }
    }
    .map_err(|err| AppError::Internal(err.to_string()))?;

    Ok(cache)
}

#[allow(clippy::too_many_arguments)]
pub async fn upsert_embed_url_cache(
    pool: &DbPool,
    normalized_url: &str,
    title: Option<&str>,
    description: Option<&str>,
    thumbnail_url: Option<&str>,
    domain: &str,
    fetched_at: &str,
    updated_at: &str,
) -> Result<(), AppError> {
    match pool {
        DbPool::Postgres(pool) => {
            sqlx::query(
                "INSERT INTO embed_url_cache (
                    normalized_url,
                    title,
                    description,
                    thumbnail_url,
                    domain,
                    fetched_at,
                    updated_at
                ) VALUES ($1, $2, $3, $4, $5, $6, $7)
                ON CONFLICT (normalized_url) DO UPDATE
                SET title = EXCLUDED.title,
                    description = EXCLUDED.description,
                    thumbnail_url = EXCLUDED.thumbnail_url,
                    domain = EXCLUDED.domain,
                    fetched_at = EXCLUDED.fetched_at,
                    updated_at = EXCLUDED.updated_at",
            )
            .bind(normalized_url)
            .bind(title)
            .bind(description)
            .bind(thumbnail_url)
            .bind(domain)
            .bind(fetched_at)
            .bind(updated_at)
            .execute(pool)
            .await
            .map_err(|err| AppError::Internal(err.to_string()))?;
        }
        DbPool::Sqlite(pool) => {
            sqlx::query(
                "INSERT INTO embed_url_cache (
                    normalized_url,
                    title,
                    description,
                    thumbnail_url,
                    domain,
                    fetched_at,
                    updated_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                ON CONFLICT(normalized_url) DO UPDATE
                SET title = excluded.title,
                    description = excluded.description,
                    thumbnail_url = excluded.thumbnail_url,
                    domain = excluded.domain,
                    fetched_at = excluded.fetched_at,
                    updated_at = excluded.updated_at",
            )
            .bind(normalized_url)
            .bind(title)
            .bind(description)
            .bind(thumbnail_url)
            .bind(domain)
            .bind(fetched_at)
            .bind(updated_at)
            .execute(pool)
            .await
            .map_err(|err| AppError::Internal(err.to_string()))?;
        }
    }
    Ok(())
}
