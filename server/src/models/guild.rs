use serde::Serialize;

use crate::{AppError, db::DbPool};

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct Guild {
    pub id: String,
    pub slug: String,
    pub name: String,
    pub description: Option<String>,
    pub owner_id: String,
    pub default_channel_slug: String,
    pub icon_storage_key: Option<String>,
    pub icon_mime_type: Option<String>,
    pub icon_size_bytes: Option<i64>,
    pub icon_updated_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct GuildResponse {
    pub id: String,
    pub slug: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub default_channel_slug: String,
    pub is_owner: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon_url: Option<String>,
    pub created_at: String,
}

impl GuildResponse {
    pub fn from_guild(guild: Guild, viewer_user_id: &str) -> Self {
        let icon_url = guild
            .icon_storage_key
            .as_ref()
            .map(|_| format!("/api/v1/guilds/{}/icon", guild.slug));
        Self {
            id: guild.id,
            slug: guild.slug,
            name: guild.name,
            description: guild.description,
            default_channel_slug: guild.default_channel_slug,
            is_owner: guild.owner_id == viewer_user_id,
            icon_url,
            created_at: guild.created_at,
        }
    }
}

pub async fn list_guilds_by_owner(pool: &DbPool, owner_id: &str) -> Result<Vec<Guild>, AppError> {
    let guilds: Vec<Guild> = match pool {
        DbPool::Postgres(pool) => {
            sqlx::query_as(
                "SELECT id, slug, name, description, owner_id, default_channel_slug, icon_storage_key, icon_mime_type, icon_size_bytes, icon_updated_at, created_at, updated_at FROM guilds WHERE owner_id = $1 ORDER BY created_at ASC",
            )
            .bind(owner_id)
            .fetch_all(pool)
            .await
        }
        DbPool::Sqlite(pool) => {
            sqlx::query_as(
                "SELECT id, slug, name, description, owner_id, default_channel_slug, icon_storage_key, icon_mime_type, icon_size_bytes, icon_updated_at, created_at, updated_at FROM guilds WHERE owner_id = ?1 ORDER BY created_at ASC",
            )
            .bind(owner_id)
            .fetch_all(pool)
            .await
        }
    }
    .map_err(|err| AppError::Internal(err.to_string()))?;

    Ok(guilds)
}

pub async fn find_guild_by_slug(pool: &DbPool, slug: &str) -> Result<Option<Guild>, AppError> {
    let guild: Option<Guild> = match pool {
        DbPool::Postgres(pool) => {
            sqlx::query_as(
                "SELECT id, slug, name, description, owner_id, default_channel_slug, icon_storage_key, icon_mime_type, icon_size_bytes, icon_updated_at, created_at, updated_at FROM guilds WHERE slug = $1 LIMIT 1",
            )
            .bind(slug)
            .fetch_optional(pool)
            .await
        }
        DbPool::Sqlite(pool) => {
            sqlx::query_as(
                "SELECT id, slug, name, description, owner_id, default_channel_slug, icon_storage_key, icon_mime_type, icon_size_bytes, icon_updated_at, created_at, updated_at FROM guilds WHERE slug = ?1 LIMIT 1",
            )
            .bind(slug)
            .fetch_optional(pool)
            .await
        }
    }
    .map_err(|err| AppError::Internal(err.to_string()))?;

    Ok(guild)
}

#[allow(clippy::too_many_arguments)]
pub async fn insert_guild(
    pool: &DbPool,
    id: &str,
    slug: &str,
    name: &str,
    description: Option<&str>,
    owner_id: &str,
    default_channel_slug: &str,
    created_at: &str,
    updated_at: &str,
) -> Result<bool, AppError> {
    let rows = match pool {
        DbPool::Postgres(pool) => {
            sqlx::query(
                "INSERT INTO guilds (id, slug, name, description, owner_id, default_channel_slug, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8) ON CONFLICT DO NOTHING",
            )
            .bind(id)
            .bind(slug)
            .bind(name)
            .bind(description)
            .bind(owner_id)
            .bind(default_channel_slug)
            .bind(created_at)
            .bind(updated_at)
            .execute(pool)
            .await
            .map(|r| r.rows_affected())
        }
        DbPool::Sqlite(pool) => {
            sqlx::query(
                "INSERT INTO guilds (id, slug, name, description, owner_id, default_channel_slug, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8) ON CONFLICT DO NOTHING",
            )
            .bind(id)
            .bind(slug)
            .bind(name)
            .bind(description)
            .bind(owner_id)
            .bind(default_channel_slug)
            .bind(created_at)
            .bind(updated_at)
            .execute(pool)
            .await
            .map(|r| r.rows_affected())
        }
    }
    .map_err(|err| AppError::Internal(err.to_string()))?;

    Ok(rows == 1)
}

pub async fn update_guild_profile(
    pool: &DbPool,
    id: &str,
    name: &str,
    description: Option<&str>,
    updated_at: &str,
) -> Result<u64, AppError> {
    let rows = match pool {
        DbPool::Postgres(pool) => sqlx::query(
            "UPDATE guilds SET name = $1, description = $2, updated_at = $3 WHERE id = $4",
        )
        .bind(name)
        .bind(description)
        .bind(updated_at)
        .bind(id)
        .execute(pool)
        .await
        .map(|r| r.rows_affected()),
        DbPool::Sqlite(pool) => sqlx::query(
            "UPDATE guilds SET name = ?1, description = ?2, updated_at = ?3 WHERE id = ?4",
        )
        .bind(name)
        .bind(description)
        .bind(updated_at)
        .bind(id)
        .execute(pool)
        .await
        .map(|r| r.rows_affected()),
    }
    .map_err(|err| AppError::Internal(err.to_string()))?;

    Ok(rows)
}

pub async fn update_guild_icon(
    pool: &DbPool,
    id: &str,
    icon_storage_key: &str,
    icon_mime_type: &str,
    icon_size_bytes: i64,
    icon_updated_at: &str,
    updated_at: &str,
) -> Result<u64, AppError> {
    let rows = match pool {
        DbPool::Postgres(pool) => {
            sqlx::query(
                "UPDATE guilds SET icon_storage_key = $1, icon_mime_type = $2, icon_size_bytes = $3, icon_updated_at = $4, updated_at = $5 WHERE id = $6",
            )
            .bind(icon_storage_key)
            .bind(icon_mime_type)
            .bind(icon_size_bytes)
            .bind(icon_updated_at)
            .bind(updated_at)
            .bind(id)
            .execute(pool)
            .await
            .map(|r| r.rows_affected())
        }
        DbPool::Sqlite(pool) => {
            sqlx::query(
                "UPDATE guilds SET icon_storage_key = ?1, icon_mime_type = ?2, icon_size_bytes = ?3, icon_updated_at = ?4, updated_at = ?5 WHERE id = ?6",
            )
            .bind(icon_storage_key)
            .bind(icon_mime_type)
            .bind(icon_size_bytes)
            .bind(icon_updated_at)
            .bind(updated_at)
            .bind(id)
            .execute(pool)
            .await
            .map(|r| r.rows_affected())
        }
    }
    .map_err(|err| AppError::Internal(err.to_string()))?;

    Ok(rows)
}
