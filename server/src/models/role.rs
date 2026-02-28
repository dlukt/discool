use crate::{AppError, db::DbPool};

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Role {
    pub id: String,
    pub guild_id: String,
    pub name: String,
    pub color: String,
    pub position: i64,
    pub permissions_bitflag: i64,
    pub is_default: i64,
    pub created_at: String,
    pub updated_at: String,
}

pub async fn list_roles_by_guild_id(pool: &DbPool, guild_id: &str) -> Result<Vec<Role>, AppError> {
    let roles = match pool {
        DbPool::Postgres(pool) => {
            sqlx::query_as(
                "SELECT id, guild_id, name, color, position, permissions_bitflag, is_default, created_at, updated_at
                 FROM roles
                 WHERE guild_id = $1
                 ORDER BY CASE WHEN is_default = 1 THEN 1 ELSE 0 END,
                          position ASC,
                          created_at ASC",
            )
            .bind(guild_id)
            .fetch_all(pool)
            .await
        }
        DbPool::Sqlite(pool) => {
            sqlx::query_as(
                "SELECT id, guild_id, name, color, position, permissions_bitflag, is_default, created_at, updated_at
                 FROM roles
                 WHERE guild_id = ?1
                 ORDER BY CASE WHEN is_default = 1 THEN 1 ELSE 0 END,
                          position ASC,
                          created_at ASC",
            )
            .bind(guild_id)
            .fetch_all(pool)
            .await
        }
    }
    .map_err(|err| AppError::Internal(err.to_string()))?;

    Ok(roles)
}

pub async fn find_role_by_id(
    pool: &DbPool,
    guild_id: &str,
    role_id: &str,
) -> Result<Option<Role>, AppError> {
    let role = match pool {
        DbPool::Postgres(pool) => {
            sqlx::query_as(
                "SELECT id, guild_id, name, color, position, permissions_bitflag, is_default, created_at, updated_at
                 FROM roles
                 WHERE guild_id = $1 AND id = $2
                 LIMIT 1",
            )
            .bind(guild_id)
            .bind(role_id)
            .fetch_optional(pool)
            .await
        }
        DbPool::Sqlite(pool) => {
            sqlx::query_as(
                "SELECT id, guild_id, name, color, position, permissions_bitflag, is_default, created_at, updated_at
                 FROM roles
                 WHERE guild_id = ?1 AND id = ?2
                 LIMIT 1",
            )
            .bind(guild_id)
            .bind(role_id)
            .fetch_optional(pool)
            .await
        }
    }
    .map_err(|err| AppError::Internal(err.to_string()))?;

    Ok(role)
}

pub async fn find_default_role_by_guild_id(
    pool: &DbPool,
    guild_id: &str,
) -> Result<Option<Role>, AppError> {
    let role = match pool {
        DbPool::Postgres(pool) => {
            sqlx::query_as(
                "SELECT id, guild_id, name, color, position, permissions_bitflag, is_default, created_at, updated_at
                 FROM roles
                 WHERE guild_id = $1 AND is_default = 1
                 LIMIT 1",
            )
            .bind(guild_id)
            .fetch_optional(pool)
            .await
        }
        DbPool::Sqlite(pool) => {
            sqlx::query_as(
                "SELECT id, guild_id, name, color, position, permissions_bitflag, is_default, created_at, updated_at
                 FROM roles
                 WHERE guild_id = ?1 AND is_default = 1
                 LIMIT 1",
            )
            .bind(guild_id)
            .fetch_optional(pool)
            .await
        }
    }
    .map_err(|err| AppError::Internal(err.to_string()))?;

    Ok(role)
}

#[allow(clippy::too_many_arguments)]
pub async fn insert_role(
    pool: &DbPool,
    id: &str,
    guild_id: &str,
    name: &str,
    color: &str,
    position: i64,
    permissions_bitflag: i64,
    is_default: bool,
    created_at: &str,
    updated_at: &str,
) -> Result<bool, AppError> {
    let is_default_value = if is_default { 1_i64 } else { 0_i64 };
    let rows = match pool {
        DbPool::Postgres(pool) => {
            sqlx::query(
                "INSERT INTO roles (id, guild_id, name, color, position, permissions_bitflag, is_default, created_at, updated_at)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
                 ON CONFLICT DO NOTHING",
            )
            .bind(id)
            .bind(guild_id)
            .bind(name)
            .bind(color)
            .bind(position)
            .bind(permissions_bitflag)
            .bind(is_default_value)
            .bind(created_at)
            .bind(updated_at)
            .execute(pool)
            .await
            .map(|result| result.rows_affected())
        }
        DbPool::Sqlite(pool) => {
            sqlx::query(
                "INSERT INTO roles (id, guild_id, name, color, position, permissions_bitflag, is_default, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
                 ON CONFLICT DO NOTHING",
            )
            .bind(id)
            .bind(guild_id)
            .bind(name)
            .bind(color)
            .bind(position)
            .bind(permissions_bitflag)
            .bind(is_default_value)
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

pub async fn next_custom_role_position(pool: &DbPool, guild_id: &str) -> Result<i64, AppError> {
    let next_position = match pool {
        DbPool::Postgres(pool) => {
            sqlx::query_scalar::<_, Option<i64>>(
                "SELECT COALESCE(MAX(position), -1) + 1
                 FROM roles
                 WHERE guild_id = $1 AND is_default = 0",
            )
            .bind(guild_id)
            .fetch_one(pool)
            .await
        }
        DbPool::Sqlite(pool) => {
            sqlx::query_scalar::<_, Option<i64>>(
                "SELECT COALESCE(MAX(position), -1) + 1
                 FROM roles
                 WHERE guild_id = ?1 AND is_default = 0",
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

pub async fn update_custom_role(
    pool: &DbPool,
    role_id: &str,
    name: &str,
    color: &str,
    updated_at: &str,
) -> Result<u64, AppError> {
    let rows = match pool {
        DbPool::Postgres(pool) => sqlx::query(
            "UPDATE roles
                 SET name = $1, color = $2, updated_at = $3
                 WHERE id = $4 AND is_default = 0",
        )
        .bind(name)
        .bind(color)
        .bind(updated_at)
        .bind(role_id)
        .execute(pool)
        .await
        .map(|result| result.rows_affected()),
        DbPool::Sqlite(pool) => sqlx::query(
            "UPDATE roles
                 SET name = ?1, color = ?2, updated_at = ?3
                 WHERE id = ?4 AND is_default = 0",
        )
        .bind(name)
        .bind(color)
        .bind(updated_at)
        .bind(role_id)
        .execute(pool)
        .await
        .map(|result| result.rows_affected()),
    }
    .map_err(|err| AppError::Internal(err.to_string()))?;

    Ok(rows)
}

pub async fn delete_role_assignments_by_role_id(
    pool: &DbPool,
    role_id: &str,
) -> Result<u64, AppError> {
    let rows = match pool {
        DbPool::Postgres(pool) => sqlx::query("DELETE FROM role_assignments WHERE role_id = $1")
            .bind(role_id)
            .execute(pool)
            .await
            .map(|result| result.rows_affected()),
        DbPool::Sqlite(pool) => sqlx::query("DELETE FROM role_assignments WHERE role_id = ?1")
            .bind(role_id)
            .execute(pool)
            .await
            .map(|result| result.rows_affected()),
    }
    .map_err(|err| AppError::Internal(err.to_string()))?;

    Ok(rows)
}

pub async fn delete_custom_role(pool: &DbPool, role_id: &str) -> Result<u64, AppError> {
    let rows = match pool {
        DbPool::Postgres(pool) => sqlx::query("DELETE FROM roles WHERE id = $1 AND is_default = 0")
            .bind(role_id)
            .execute(pool)
            .await
            .map(|result| result.rows_affected()),
        DbPool::Sqlite(pool) => sqlx::query("DELETE FROM roles WHERE id = ?1 AND is_default = 0")
            .bind(role_id)
            .execute(pool)
            .await
            .map(|result| result.rows_affected()),
    }
    .map_err(|err| AppError::Internal(err.to_string()))?;

    Ok(rows)
}
