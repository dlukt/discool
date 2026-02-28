use crate::{AppError, db::DbPool};

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ChannelCategory {
    pub id: String,
    pub guild_id: String,
    pub slug: String,
    pub name: String,
    pub position: i64,
    pub created_at: String,
    pub updated_at: String,
}

pub async fn list_categories_by_guild_id(
    pool: &DbPool,
    guild_id: &str,
) -> Result<Vec<ChannelCategory>, AppError> {
    let categories = match pool {
        DbPool::Postgres(pool) => {
            sqlx::query_as(
                "SELECT id, guild_id, slug, name, position, created_at, updated_at
                 FROM channel_categories
                 WHERE guild_id = $1
                 ORDER BY position ASC, created_at ASC",
            )
            .bind(guild_id)
            .fetch_all(pool)
            .await
        }
        DbPool::Sqlite(pool) => {
            sqlx::query_as(
                "SELECT id, guild_id, slug, name, position, created_at, updated_at
                 FROM channel_categories
                 WHERE guild_id = ?1
                 ORDER BY position ASC, created_at ASC",
            )
            .bind(guild_id)
            .fetch_all(pool)
            .await
        }
    }
    .map_err(|err| AppError::Internal(err.to_string()))?;
    Ok(categories)
}

pub async fn find_category_by_slug(
    pool: &DbPool,
    guild_id: &str,
    slug: &str,
) -> Result<Option<ChannelCategory>, AppError> {
    let category = match pool {
        DbPool::Postgres(pool) => {
            sqlx::query_as(
                "SELECT id, guild_id, slug, name, position, created_at, updated_at
                 FROM channel_categories
                 WHERE guild_id = $1 AND slug = $2
                 LIMIT 1",
            )
            .bind(guild_id)
            .bind(slug)
            .fetch_optional(pool)
            .await
        }
        DbPool::Sqlite(pool) => {
            sqlx::query_as(
                "SELECT id, guild_id, slug, name, position, created_at, updated_at
                 FROM channel_categories
                 WHERE guild_id = ?1 AND slug = ?2
                 LIMIT 1",
            )
            .bind(guild_id)
            .bind(slug)
            .fetch_optional(pool)
            .await
        }
    }
    .map_err(|err| AppError::Internal(err.to_string()))?;
    Ok(category)
}

#[allow(clippy::too_many_arguments)]
pub async fn insert_category(
    pool: &DbPool,
    id: &str,
    guild_id: &str,
    slug: &str,
    name: &str,
    position: i64,
    created_at: &str,
    updated_at: &str,
) -> Result<bool, AppError> {
    let rows = match pool {
        DbPool::Postgres(pool) => {
            sqlx::query(
                "INSERT INTO channel_categories (id, guild_id, slug, name, position, created_at, updated_at)
                 VALUES ($1, $2, $3, $4, $5, $6, $7)
                 ON CONFLICT DO NOTHING",
            )
            .bind(id)
            .bind(guild_id)
            .bind(slug)
            .bind(name)
            .bind(position)
            .bind(created_at)
            .bind(updated_at)
            .execute(pool)
            .await
            .map(|r| r.rows_affected())
        }
        DbPool::Sqlite(pool) => {
            sqlx::query(
                "INSERT INTO channel_categories (id, guild_id, slug, name, position, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                 ON CONFLICT DO NOTHING",
            )
            .bind(id)
            .bind(guild_id)
            .bind(slug)
            .bind(name)
            .bind(position)
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

pub async fn next_category_position(pool: &DbPool, guild_id: &str) -> Result<i64, AppError> {
    let next_position = match pool {
        DbPool::Postgres(pool) => sqlx::query_scalar::<_, Option<i64>>(
            "SELECT COALESCE(MAX(position), -1) + 1 FROM channel_categories WHERE guild_id = $1",
        )
        .bind(guild_id)
        .fetch_one(pool)
        .await,
        DbPool::Sqlite(pool) => sqlx::query_scalar::<_, Option<i64>>(
            "SELECT COALESCE(MAX(position), -1) + 1 FROM channel_categories WHERE guild_id = ?1",
        )
        .bind(guild_id)
        .fetch_one(pool)
        .await,
    }
    .map_err(|err| AppError::Internal(err.to_string()))?
    .unwrap_or(0);
    Ok(next_position.max(0))
}

pub async fn update_category(
    pool: &DbPool,
    id: &str,
    name: &str,
    slug: &str,
    updated_at: &str,
) -> Result<u64, AppError> {
    let rows = match pool {
        DbPool::Postgres(pool) => sqlx::query(
            "UPDATE channel_categories
                 SET name = $1, slug = $2, updated_at = $3
                 WHERE id = $4",
        )
        .bind(name)
        .bind(slug)
        .bind(updated_at)
        .bind(id)
        .execute(pool)
        .await
        .map(|r| r.rows_affected()),
        DbPool::Sqlite(pool) => sqlx::query(
            "UPDATE channel_categories
                 SET name = ?1, slug = ?2, updated_at = ?3
                 WHERE id = ?4",
        )
        .bind(name)
        .bind(slug)
        .bind(updated_at)
        .bind(id)
        .execute(pool)
        .await
        .map(|r| r.rows_affected()),
    }
    .map_err(|err| AppError::Internal(err.to_string()))?;
    Ok(rows)
}

pub async fn delete_category(pool: &DbPool, id: &str) -> Result<u64, AppError> {
    let rows = match pool {
        DbPool::Postgres(pool) => sqlx::query("DELETE FROM channel_categories WHERE id = $1")
            .bind(id)
            .execute(pool)
            .await
            .map(|r| r.rows_affected()),
        DbPool::Sqlite(pool) => sqlx::query("DELETE FROM channel_categories WHERE id = ?1")
            .bind(id)
            .execute(pool)
            .await
            .map(|r| r.rows_affected()),
    }
    .map_err(|err| AppError::Internal(err.to_string()))?;
    Ok(rows)
}

pub async fn reorder_categories(
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
                    "UPDATE channel_categories
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
                        "reorder payload contains unknown category slug".to_string(),
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
                    "UPDATE channel_categories
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
                        "reorder payload contains unknown category slug".to_string(),
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

pub async fn compact_category_positions(
    pool: &DbPool,
    guild_id: &str,
    updated_at: &str,
) -> Result<(), AppError> {
    let categories = list_categories_by_guild_id(pool, guild_id).await?;
    let ordered_slugs: Vec<String> = categories
        .into_iter()
        .map(|category| category.slug)
        .collect();
    reorder_categories(pool, guild_id, &ordered_slugs, updated_at).await
}

pub async fn list_collapsed_category_ids(
    pool: &DbPool,
    user_id: &str,
    guild_id: &str,
) -> Result<Vec<String>, AppError> {
    let category_ids = match pool {
        DbPool::Postgres(pool) => {
            sqlx::query_scalar::<_, String>(
                "SELECT category_id
                 FROM channel_category_collapses
                 WHERE user_id = $1 AND guild_id = $2 AND collapsed = 1",
            )
            .bind(user_id)
            .bind(guild_id)
            .fetch_all(pool)
            .await
        }
        DbPool::Sqlite(pool) => {
            sqlx::query_scalar::<_, String>(
                "SELECT category_id
                 FROM channel_category_collapses
                 WHERE user_id = ?1 AND guild_id = ?2 AND collapsed = 1",
            )
            .bind(user_id)
            .bind(guild_id)
            .fetch_all(pool)
            .await
        }
    }
    .map_err(|err| AppError::Internal(err.to_string()))?;
    Ok(category_ids)
}

pub async fn upsert_category_collapse(
    pool: &DbPool,
    user_id: &str,
    guild_id: &str,
    category_id: &str,
    collapsed: bool,
    now: &str,
) -> Result<(), AppError> {
    let collapsed_value = if collapsed { 1_i64 } else { 0_i64 };
    match pool {
        DbPool::Postgres(pool) => {
            sqlx::query(
                "INSERT INTO channel_category_collapses (user_id, guild_id, category_id, collapsed, created_at, updated_at)
                 VALUES ($1, $2, $3, $4, $5, $6)
                 ON CONFLICT (user_id, guild_id, category_id)
                 DO UPDATE SET collapsed = EXCLUDED.collapsed, updated_at = EXCLUDED.updated_at",
            )
            .bind(user_id)
            .bind(guild_id)
            .bind(category_id)
            .bind(collapsed_value)
            .bind(now)
            .bind(now)
            .execute(pool)
            .await
            .map_err(|err| AppError::Internal(err.to_string()))?;
        }
        DbPool::Sqlite(pool) => {
            sqlx::query(
                "INSERT INTO channel_category_collapses (user_id, guild_id, category_id, collapsed, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                 ON CONFLICT (user_id, guild_id, category_id)
                 DO UPDATE SET collapsed = excluded.collapsed, updated_at = excluded.updated_at",
            )
            .bind(user_id)
            .bind(guild_id)
            .bind(category_id)
            .bind(collapsed_value)
            .bind(now)
            .bind(now)
            .execute(pool)
            .await
            .map_err(|err| AppError::Internal(err.to_string()))?;
        }
    }
    Ok(())
}
