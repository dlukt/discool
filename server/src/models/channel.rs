use std::collections::HashMap;

use crate::{AppError, db::DbPool};

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Channel {
    pub id: String,
    pub guild_id: String,
    pub slug: String,
    pub name: String,
    pub channel_type: String,
    pub position: i64,
    pub category_id: Option<String>,
    pub category_slug: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone)]
pub struct ChannelPositionUpdate {
    pub slug: String,
    pub category_id: Option<String>,
    pub position: i64,
}

pub async fn list_channels_by_guild_id(
    pool: &DbPool,
    guild_id: &str,
) -> Result<Vec<Channel>, AppError> {
    let channels = match pool {
        DbPool::Postgres(pool) => {
            sqlx::query_as(
                "SELECT c.id,
                    c.guild_id,
                    c.slug,
                    c.name,
                    c.channel_type,
                    c.position,
                    c.category_id,
                    cc.slug AS category_slug,
                    c.created_at,
                    c.updated_at
             FROM channels c
             LEFT JOIN channel_categories cc ON cc.id = c.category_id
             WHERE c.guild_id = $1
             ORDER BY CASE WHEN c.category_id IS NULL THEN 1 ELSE 0 END,
                      COALESCE(cc.position, 9223372036854775807),
                      c.position ASC,
                      c.created_at ASC",
            )
            .bind(guild_id)
            .fetch_all(pool)
            .await
        }
        DbPool::Sqlite(pool) => {
            sqlx::query_as(
                "SELECT c.id,
                    c.guild_id,
                    c.slug,
                    c.name,
                    c.channel_type,
                    c.position,
                    c.category_id,
                    cc.slug AS category_slug,
                    c.created_at,
                    c.updated_at
             FROM channels c
             LEFT JOIN channel_categories cc ON cc.id = c.category_id
             WHERE c.guild_id = ?1
             ORDER BY CASE WHEN c.category_id IS NULL THEN 1 ELSE 0 END,
                      COALESCE(cc.position, 9223372036854775807),
                      c.position ASC,
                      c.created_at ASC",
            )
            .bind(guild_id)
            .fetch_all(pool)
            .await
        }
    }
    .map_err(|err| AppError::Internal(err.to_string()))?;

    Ok(channels)
}

pub async fn count_channels_by_guild_id(pool: &DbPool, guild_id: &str) -> Result<i64, AppError> {
    let count = match pool {
        DbPool::Postgres(pool) => {
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM channels WHERE guild_id = $1")
                .bind(guild_id)
                .fetch_one(pool)
                .await
        }
        DbPool::Sqlite(pool) => {
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM channels WHERE guild_id = ?1")
                .bind(guild_id)
                .fetch_one(pool)
                .await
        }
    }
    .map_err(|err| AppError::Internal(err.to_string()))?;

    Ok(count)
}

pub async fn find_channel_by_slug(
    pool: &DbPool,
    guild_id: &str,
    slug: &str,
) -> Result<Option<Channel>, AppError> {
    let channel = match pool {
        DbPool::Postgres(pool) => {
            sqlx::query_as(
                "SELECT c.id,
                    c.guild_id,
                    c.slug,
                    c.name,
                    c.channel_type,
                    c.position,
                    c.category_id,
                    cc.slug AS category_slug,
                    c.created_at,
                    c.updated_at
             FROM channels c
             LEFT JOIN channel_categories cc ON cc.id = c.category_id
             WHERE c.guild_id = $1 AND c.slug = $2
             LIMIT 1",
            )
            .bind(guild_id)
            .bind(slug)
            .fetch_optional(pool)
            .await
        }
        DbPool::Sqlite(pool) => {
            sqlx::query_as(
                "SELECT c.id,
                    c.guild_id,
                    c.slug,
                    c.name,
                    c.channel_type,
                    c.position,
                    c.category_id,
                    cc.slug AS category_slug,
                    c.created_at,
                    c.updated_at
             FROM channels c
             LEFT JOIN channel_categories cc ON cc.id = c.category_id
             WHERE c.guild_id = ?1 AND c.slug = ?2
             LIMIT 1",
            )
            .bind(guild_id)
            .bind(slug)
            .fetch_optional(pool)
            .await
        }
    }
    .map_err(|err| AppError::Internal(err.to_string()))?;

    Ok(channel)
}

#[allow(clippy::too_many_arguments)]
pub async fn insert_channel(
    pool: &DbPool,
    id: &str,
    guild_id: &str,
    slug: &str,
    name: &str,
    channel_type: &str,
    position: i64,
    category_id: Option<&str>,
    created_at: &str,
    updated_at: &str,
) -> Result<bool, AppError> {
    let rows = match pool {
        DbPool::Postgres(pool) => {
            sqlx::query(
                "INSERT INTO channels (id, guild_id, slug, name, channel_type, position, category_id, created_at, updated_at)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
                 ON CONFLICT DO NOTHING",
            )
            .bind(id)
            .bind(guild_id)
            .bind(slug)
            .bind(name)
            .bind(channel_type)
            .bind(position)
            .bind(category_id)
            .bind(created_at)
            .bind(updated_at)
            .execute(pool)
            .await
            .map(|r| r.rows_affected())
        }
        DbPool::Sqlite(pool) => {
            sqlx::query(
                "INSERT INTO channels (id, guild_id, slug, name, channel_type, position, category_id, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
                 ON CONFLICT DO NOTHING",
            )
            .bind(id)
            .bind(guild_id)
            .bind(slug)
            .bind(name)
            .bind(channel_type)
            .bind(position)
            .bind(category_id)
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

pub async fn next_channel_position(pool: &DbPool, guild_id: &str) -> Result<i64, AppError> {
    next_channel_position_for_category(pool, guild_id, None).await
}

pub async fn next_channel_position_for_category(
    pool: &DbPool,
    guild_id: &str,
    category_id: Option<&str>,
) -> Result<i64, AppError> {
    let next_position = match (pool, category_id) {
        (DbPool::Postgres(pool), Some(category_id)) => {
            sqlx::query_scalar::<_, Option<i64>>(
                "SELECT COALESCE(MAX(position), -1) + 1
                 FROM channels
                 WHERE guild_id = $1 AND category_id = $2",
            )
            .bind(guild_id)
            .bind(category_id)
            .fetch_one(pool)
            .await
        }
        (DbPool::Postgres(pool), None) => {
            sqlx::query_scalar::<_, Option<i64>>(
                "SELECT COALESCE(MAX(position), -1) + 1
                 FROM channels
                 WHERE guild_id = $1 AND category_id IS NULL",
            )
            .bind(guild_id)
            .fetch_one(pool)
            .await
        }
        (DbPool::Sqlite(pool), Some(category_id)) => {
            sqlx::query_scalar::<_, Option<i64>>(
                "SELECT COALESCE(MAX(position), -1) + 1
                 FROM channels
                 WHERE guild_id = ?1 AND category_id = ?2",
            )
            .bind(guild_id)
            .bind(category_id)
            .fetch_one(pool)
            .await
        }
        (DbPool::Sqlite(pool), None) => {
            sqlx::query_scalar::<_, Option<i64>>(
                "SELECT COALESCE(MAX(position), -1) + 1
                 FROM channels
                 WHERE guild_id = ?1 AND category_id IS NULL",
            )
            .bind(guild_id)
            .fetch_one(pool)
            .await
        }
    }
    .map_err(|err| AppError::Internal(err.to_string()))?
    .unwrap_or(0);

    Ok(next_position.max(0))
}

pub async fn update_channel(
    pool: &DbPool,
    id: &str,
    name: &str,
    slug: &str,
    updated_at: &str,
) -> Result<u64, AppError> {
    let rows = match pool {
        DbPool::Postgres(pool) => {
            sqlx::query("UPDATE channels SET name = $1, slug = $2, updated_at = $3 WHERE id = $4")
                .bind(name)
                .bind(slug)
                .bind(updated_at)
                .bind(id)
                .execute(pool)
                .await
                .map(|r| r.rows_affected())
        }
        DbPool::Sqlite(pool) => {
            sqlx::query("UPDATE channels SET name = ?1, slug = ?2, updated_at = ?3 WHERE id = ?4")
                .bind(name)
                .bind(slug)
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

pub async fn delete_channel(pool: &DbPool, id: &str) -> Result<u64, AppError> {
    let rows = match pool {
        DbPool::Postgres(pool) => sqlx::query("DELETE FROM channels WHERE id = $1")
            .bind(id)
            .execute(pool)
            .await
            .map(|r| r.rows_affected()),
        DbPool::Sqlite(pool) => sqlx::query("DELETE FROM channels WHERE id = ?1")
            .bind(id)
            .execute(pool)
            .await
            .map(|r| r.rows_affected()),
    }
    .map_err(|err| AppError::Internal(err.to_string()))?;

    Ok(rows)
}

pub async fn reorder_channels(
    pool: &DbPool,
    guild_id: &str,
    ordered_slugs: &[String],
    updated_at: &str,
) -> Result<(), AppError> {
    match pool {
        DbPool::Postgres(pool) => {
            let mut tx = pool
                .begin()
                .await
                .map_err(|err| AppError::Internal(err.to_string()))?;
            for (position, slug) in ordered_slugs.iter().enumerate() {
                let rows = sqlx::query(
                    "UPDATE channels
                     SET position = $1, updated_at = $2
                     WHERE guild_id = $3 AND slug = $4",
                )
                .bind(position as i64)
                .bind(updated_at)
                .bind(guild_id)
                .bind(slug)
                .execute(&mut *tx)
                .await
                .map_err(|err| AppError::Internal(err.to_string()))?
                .rows_affected();
                if rows != 1 {
                    return Err(AppError::ValidationError(
                        "reorder payload contains unknown channel slug".to_string(),
                    ));
                }
            }
            tx.commit()
                .await
                .map_err(|err| AppError::Internal(err.to_string()))?;
        }
        DbPool::Sqlite(pool) => {
            let mut tx = pool
                .begin()
                .await
                .map_err(|err| AppError::Internal(err.to_string()))?;
            for (position, slug) in ordered_slugs.iter().enumerate() {
                let rows = sqlx::query(
                    "UPDATE channels
                     SET position = ?1, updated_at = ?2
                     WHERE guild_id = ?3 AND slug = ?4",
                )
                .bind(position as i64)
                .bind(updated_at)
                .bind(guild_id)
                .bind(slug)
                .execute(&mut *tx)
                .await
                .map_err(|err| AppError::Internal(err.to_string()))?
                .rows_affected();
                if rows != 1 {
                    return Err(AppError::ValidationError(
                        "reorder payload contains unknown channel slug".to_string(),
                    ));
                }
            }
            tx.commit()
                .await
                .map_err(|err| AppError::Internal(err.to_string()))?;
        }
    }
    Ok(())
}

pub async fn reorder_channel_positions(
    pool: &DbPool,
    guild_id: &str,
    updates: &[ChannelPositionUpdate],
    updated_at: &str,
) -> Result<(), AppError> {
    match pool {
        DbPool::Postgres(pool) => {
            let mut tx = pool
                .begin()
                .await
                .map_err(|err| AppError::Internal(err.to_string()))?;
            for update in updates {
                let rows = sqlx::query(
                    "UPDATE channels
                     SET category_id = $1, position = $2, updated_at = $3
                     WHERE guild_id = $4 AND slug = $5",
                )
                .bind(update.category_id.as_deref())
                .bind(update.position)
                .bind(updated_at)
                .bind(guild_id)
                .bind(&update.slug)
                .execute(&mut *tx)
                .await
                .map_err(|err| AppError::Internal(err.to_string()))?
                .rows_affected();
                if rows != 1 {
                    return Err(AppError::ValidationError(
                        "reorder payload contains unknown channel slug".to_string(),
                    ));
                }
            }
            tx.commit()
                .await
                .map_err(|err| AppError::Internal(err.to_string()))?;
        }
        DbPool::Sqlite(pool) => {
            let mut tx = pool
                .begin()
                .await
                .map_err(|err| AppError::Internal(err.to_string()))?;
            for update in updates {
                let rows = sqlx::query(
                    "UPDATE channels
                     SET category_id = ?1, position = ?2, updated_at = ?3
                     WHERE guild_id = ?4 AND slug = ?5",
                )
                .bind(update.category_id.as_deref())
                .bind(update.position)
                .bind(updated_at)
                .bind(guild_id)
                .bind(&update.slug)
                .execute(&mut *tx)
                .await
                .map_err(|err| AppError::Internal(err.to_string()))?
                .rows_affected();
                if rows != 1 {
                    return Err(AppError::ValidationError(
                        "reorder payload contains unknown channel slug".to_string(),
                    ));
                }
            }
            tx.commit()
                .await
                .map_err(|err| AppError::Internal(err.to_string()))?;
        }
    }
    Ok(())
}

pub async fn compact_channel_positions(
    pool: &DbPool,
    guild_id: &str,
    updated_at: &str,
) -> Result<(), AppError> {
    let channels = list_channels_by_guild_id(pool, guild_id).await?;
    let mut category_counters: HashMap<Option<String>, i64> = HashMap::new();
    let mut updates = Vec::with_capacity(channels.len());

    for channel in channels {
        let key = channel.category_id.clone();
        let next_position = *category_counters.get(&key).unwrap_or(&0);
        category_counters.insert(key.clone(), next_position + 1);
        updates.push(ChannelPositionUpdate {
            slug: channel.slug,
            category_id: key,
            position: next_position,
        });
    }

    reorder_channel_positions(pool, guild_id, &updates, updated_at).await
}

pub async fn list_channels_by_category_id(
    pool: &DbPool,
    guild_id: &str,
    category_id: &str,
) -> Result<Vec<Channel>, AppError> {
    let channels = match pool {
        DbPool::Postgres(pool) => {
            sqlx::query_as(
                "SELECT c.id,
                    c.guild_id,
                    c.slug,
                    c.name,
                    c.channel_type,
                    c.position,
                    c.category_id,
                    cc.slug AS category_slug,
                    c.created_at,
                    c.updated_at
             FROM channels c
             LEFT JOIN channel_categories cc ON cc.id = c.category_id
             WHERE c.guild_id = $1 AND c.category_id = $2
             ORDER BY c.position ASC, c.created_at ASC",
            )
            .bind(guild_id)
            .bind(category_id)
            .fetch_all(pool)
            .await
        }
        DbPool::Sqlite(pool) => {
            sqlx::query_as(
                "SELECT c.id,
                    c.guild_id,
                    c.slug,
                    c.name,
                    c.channel_type,
                    c.position,
                    c.category_id,
                    cc.slug AS category_slug,
                    c.created_at,
                    c.updated_at
             FROM channels c
             LEFT JOIN channel_categories cc ON cc.id = c.category_id
             WHERE c.guild_id = ?1 AND c.category_id = ?2
             ORDER BY c.position ASC, c.created_at ASC",
            )
            .bind(guild_id)
            .bind(category_id)
            .fetch_all(pool)
            .await
        }
    }
    .map_err(|err| AppError::Internal(err.to_string()))?;

    Ok(channels)
}
