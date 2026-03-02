use std::collections::HashSet;

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

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct RoleAssignment {
    pub guild_id: String,
    pub user_id: String,
    pub role_id: String,
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
    permissions_bitflag: i64,
    updated_at: &str,
) -> Result<u64, AppError> {
    let rows = match pool {
        DbPool::Postgres(pool) => sqlx::query(
            "UPDATE roles
                 SET name = $1, color = $2, permissions_bitflag = $3, updated_at = $4
                 WHERE id = $5 AND is_default = 0",
        )
        .bind(name)
        .bind(color)
        .bind(permissions_bitflag)
        .bind(updated_at)
        .bind(role_id)
        .execute(pool)
        .await
        .map(|result| result.rows_affected()),
        DbPool::Sqlite(pool) => sqlx::query(
            "UPDATE roles
                 SET name = ?1, color = ?2, permissions_bitflag = ?3, updated_at = ?4
                 WHERE id = ?5 AND is_default = 0",
        )
        .bind(name)
        .bind(color)
        .bind(permissions_bitflag)
        .bind(updated_at)
        .bind(role_id)
        .execute(pool)
        .await
        .map(|result| result.rows_affected()),
    }
    .map_err(|err| AppError::Internal(err.to_string()))?;

    Ok(rows)
}

pub async fn reorder_custom_roles(
    pool: &DbPool,
    guild_id: &str,
    ordered_role_ids: &[String],
    updated_at: &str,
) -> Result<(), AppError> {
    match pool {
        DbPool::Postgres(pool) => {
            let mut tx = pool
                .begin()
                .await
                .map_err(|err| AppError::Internal(err.to_string()))?;
            for (position, role_id) in ordered_role_ids.iter().enumerate() {
                let rows = sqlx::query(
                    "UPDATE roles
                     SET position = $1, updated_at = $2
                     WHERE guild_id = $3 AND id = $4 AND is_default = 0",
                )
                .bind(position as i64)
                .bind(updated_at)
                .bind(guild_id)
                .bind(role_id)
                .execute(&mut *tx)
                .await
                .map_err(|err| AppError::Internal(err.to_string()))?
                .rows_affected();
                if rows != 1 {
                    return Err(AppError::ValidationError(
                        "reorder payload contains unknown custom role id".to_string(),
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
            for (position, role_id) in ordered_role_ids.iter().enumerate() {
                let rows = sqlx::query(
                    "UPDATE roles
                     SET position = ?1, updated_at = ?2
                     WHERE guild_id = ?3 AND id = ?4 AND is_default = 0",
                )
                .bind(position as i64)
                .bind(updated_at)
                .bind(guild_id)
                .bind(role_id)
                .execute(&mut *tx)
                .await
                .map_err(|err| AppError::Internal(err.to_string()))?
                .rows_affected();
                if rows != 1 {
                    return Err(AppError::ValidationError(
                        "reorder payload contains unknown custom role id".to_string(),
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

pub async fn list_assigned_role_permission_bitflags(
    pool: &DbPool,
    guild_id: &str,
    user_id: &str,
) -> Result<Vec<i64>, AppError> {
    let bitflags = match pool {
        DbPool::Postgres(pool) => {
            sqlx::query_scalar::<_, i64>(
                "SELECT r.permissions_bitflag
                 FROM role_assignments ra
                 JOIN roles r ON r.id = ra.role_id
                 WHERE ra.guild_id = $1
                   AND ra.user_id = $2
                   AND r.guild_id = $1",
            )
            .bind(guild_id)
            .bind(user_id)
            .fetch_all(pool)
            .await
        }
        DbPool::Sqlite(pool) => {
            sqlx::query_scalar::<_, i64>(
                "SELECT r.permissions_bitflag
                 FROM role_assignments ra
                 JOIN roles r ON r.id = ra.role_id
                 WHERE ra.guild_id = ?1
                   AND ra.user_id = ?2
                   AND r.guild_id = ?1",
            )
            .bind(guild_id)
            .bind(user_id)
            .fetch_all(pool)
            .await
        }
    }
    .map_err(|err| AppError::Internal(err.to_string()))?;

    Ok(bitflags)
}

pub async fn list_assigned_role_positions(
    pool: &DbPool,
    guild_id: &str,
    user_id: &str,
) -> Result<Vec<i64>, AppError> {
    let positions = match pool {
        DbPool::Postgres(pool) => {
            sqlx::query_scalar::<_, i64>(
                "SELECT r.position
                 FROM role_assignments ra
                 JOIN roles r ON r.id = ra.role_id
                 WHERE ra.guild_id = $1
                   AND ra.user_id = $2
                   AND r.guild_id = $1
                 ORDER BY r.position ASC",
            )
            .bind(guild_id)
            .bind(user_id)
            .fetch_all(pool)
            .await
        }
        DbPool::Sqlite(pool) => {
            sqlx::query_scalar::<_, i64>(
                "SELECT r.position
                 FROM role_assignments ra
                 JOIN roles r ON r.id = ra.role_id
                 WHERE ra.guild_id = ?1
                   AND ra.user_id = ?2
                   AND r.guild_id = ?1
                 ORDER BY r.position ASC",
            )
            .bind(guild_id)
            .bind(user_id)
            .fetch_all(pool)
            .await
        }
    }
    .map_err(|err| AppError::Internal(err.to_string()))?;

    Ok(positions)
}

pub async fn list_assigned_role_ids(
    pool: &DbPool,
    guild_id: &str,
    user_id: &str,
) -> Result<Vec<String>, AppError> {
    let role_ids = match pool {
        DbPool::Postgres(pool) => {
            sqlx::query_scalar::<_, String>(
                "SELECT ra.role_id
                 FROM role_assignments ra
                 JOIN roles r ON r.id = ra.role_id
                 WHERE ra.guild_id = $1
                   AND ra.user_id = $2
                   AND r.guild_id = $1
                 ORDER BY r.position ASC, r.created_at ASC, r.id ASC",
            )
            .bind(guild_id)
            .bind(user_id)
            .fetch_all(pool)
            .await
        }
        DbPool::Sqlite(pool) => {
            sqlx::query_scalar::<_, String>(
                "SELECT ra.role_id
                 FROM role_assignments ra
                 JOIN roles r ON r.id = ra.role_id
                 WHERE ra.guild_id = ?1
                   AND ra.user_id = ?2
                   AND r.guild_id = ?1
                 ORDER BY r.position ASC, r.created_at ASC, r.id ASC",
            )
            .bind(guild_id)
            .bind(user_id)
            .fetch_all(pool)
            .await
        }
    }
    .map_err(|err| AppError::Internal(err.to_string()))?;

    Ok(role_ids)
}

pub async fn list_role_assignments_by_guild_id(
    pool: &DbPool,
    guild_id: &str,
) -> Result<Vec<RoleAssignment>, AppError> {
    let assignments = match pool {
        DbPool::Postgres(pool) => {
            sqlx::query_as(
                "SELECT guild_id, user_id, role_id
                 FROM role_assignments
                 WHERE guild_id = $1
                 ORDER BY user_id ASC, role_id ASC",
            )
            .bind(guild_id)
            .fetch_all(pool)
            .await
        }
        DbPool::Sqlite(pool) => {
            sqlx::query_as(
                "SELECT guild_id, user_id, role_id
                 FROM role_assignments
                 WHERE guild_id = ?1
                 ORDER BY user_id ASC, role_id ASC",
            )
            .bind(guild_id)
            .fetch_all(pool)
            .await
        }
    }
    .map_err(|err| AppError::Internal(err.to_string()))?;

    Ok(assignments)
}

pub async fn set_role_assignments_for_user(
    pool: &DbPool,
    guild_id: &str,
    user_id: &str,
    role_ids: &[String],
    assigned_at: &str,
) -> Result<(), AppError> {
    let desired_role_ids: HashSet<String> = role_ids.iter().cloned().collect();
    match pool {
        DbPool::Postgres(pool) => {
            let mut tx = pool
                .begin()
                .await
                .map_err(|err| AppError::Internal(err.to_string()))?;

            let existing_role_ids: HashSet<String> = sqlx::query_scalar::<_, String>(
                "SELECT role_id
                 FROM role_assignments
                 WHERE guild_id = $1 AND user_id = $2",
            )
            .bind(guild_id)
            .bind(user_id)
            .fetch_all(&mut *tx)
            .await
            .map_err(|err| AppError::Internal(err.to_string()))?
            .into_iter()
            .collect();

            for role_id in existing_role_ids.difference(&desired_role_ids) {
                sqlx::query(
                    "DELETE FROM role_assignments
                     WHERE guild_id = $1 AND user_id = $2 AND role_id = $3",
                )
                .bind(guild_id)
                .bind(user_id)
                .bind(role_id)
                .execute(&mut *tx)
                .await
                .map_err(|err| AppError::Internal(err.to_string()))?;
            }

            for role_id in desired_role_ids.difference(&existing_role_ids) {
                sqlx::query(
                    "INSERT INTO role_assignments (guild_id, user_id, role_id, assigned_at)
                     VALUES ($1, $2, $3, $4)
                     ON CONFLICT DO NOTHING",
                )
                .bind(guild_id)
                .bind(user_id)
                .bind(role_id)
                .bind(assigned_at)
                .execute(&mut *tx)
                .await
                .map_err(|err| AppError::Internal(err.to_string()))?;
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

            let existing_role_ids: HashSet<String> = sqlx::query_scalar::<_, String>(
                "SELECT role_id
                 FROM role_assignments
                 WHERE guild_id = ?1 AND user_id = ?2",
            )
            .bind(guild_id)
            .bind(user_id)
            .fetch_all(&mut *tx)
            .await
            .map_err(|err| AppError::Internal(err.to_string()))?
            .into_iter()
            .collect();

            for role_id in existing_role_ids.difference(&desired_role_ids) {
                sqlx::query(
                    "DELETE FROM role_assignments
                     WHERE guild_id = ?1 AND user_id = ?2 AND role_id = ?3",
                )
                .bind(guild_id)
                .bind(user_id)
                .bind(role_id)
                .execute(&mut *tx)
                .await
                .map_err(|err| AppError::Internal(err.to_string()))?;
            }

            for role_id in desired_role_ids.difference(&existing_role_ids) {
                sqlx::query(
                    "INSERT INTO role_assignments (guild_id, user_id, role_id, assigned_at)
                     VALUES (?1, ?2, ?3, ?4)
                     ON CONFLICT DO NOTHING",
                )
                .bind(guild_id)
                .bind(user_id)
                .bind(role_id)
                .bind(assigned_at)
                .execute(&mut *tx)
                .await
                .map_err(|err| AppError::Internal(err.to_string()))?;
            }

            tx.commit()
                .await
                .map_err(|err| AppError::Internal(err.to_string()))?;
        }
    }

    Ok(())
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

pub async fn remove_role_assignments_for_member(
    pool: &DbPool,
    guild_id: &str,
    user_id: &str,
) -> Result<u64, AppError> {
    let rows = match pool {
        DbPool::Postgres(pool) => sqlx::query(
            "DELETE FROM role_assignments
                 WHERE guild_id = $1 AND user_id = $2",
        )
        .bind(guild_id)
        .bind(user_id)
        .execute(pool)
        .await
        .map(|result| result.rows_affected()),
        DbPool::Sqlite(pool) => sqlx::query(
            "DELETE FROM role_assignments
                 WHERE guild_id = ?1 AND user_id = ?2",
        )
        .bind(guild_id)
        .bind(user_id)
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
