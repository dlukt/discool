use crate::{AppError, db::DbPool};

pub async fn insert_guild_member(
    pool: &DbPool,
    guild_id: &str,
    user_id: &str,
    joined_at: &str,
    joined_via_invite_code: Option<&str>,
) -> Result<bool, AppError> {
    let rows = match pool {
        DbPool::Postgres(pool) => sqlx::query(
            "INSERT INTO guild_members (guild_id, user_id, joined_at, joined_via_invite_code)
                 VALUES ($1, $2, $3, $4)
                 ON CONFLICT DO NOTHING",
        )
        .bind(guild_id)
        .bind(user_id)
        .bind(joined_at)
        .bind(joined_via_invite_code)
        .execute(pool)
        .await
        .map(|result| result.rows_affected()),
        DbPool::Sqlite(pool) => sqlx::query(
            "INSERT INTO guild_members (guild_id, user_id, joined_at, joined_via_invite_code)
                 VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT DO NOTHING",
        )
        .bind(guild_id)
        .bind(user_id)
        .bind(joined_at)
        .bind(joined_via_invite_code)
        .execute(pool)
        .await
        .map(|result| result.rows_affected()),
    }
    .map_err(|err| AppError::Internal(err.to_string()))?;

    Ok(rows == 1)
}

pub async fn is_guild_member(
    pool: &DbPool,
    guild_id: &str,
    user_id: &str,
) -> Result<bool, AppError> {
    let is_member = match pool {
        DbPool::Postgres(pool) => {
            sqlx::query_scalar::<_, bool>(
                "SELECT EXISTS(
                    SELECT 1
                    FROM guild_members
                    WHERE guild_id = $1 AND user_id = $2
                )",
            )
            .bind(guild_id)
            .bind(user_id)
            .fetch_one(pool)
            .await
        }
        DbPool::Sqlite(pool) => sqlx::query_scalar::<_, i64>(
            "SELECT EXISTS(
                    SELECT 1
                    FROM guild_members
                    WHERE guild_id = ?1 AND user_id = ?2
                )",
        )
        .bind(guild_id)
        .bind(user_id)
        .fetch_one(pool)
        .await
        .map(|value| value != 0),
    }
    .map_err(|err| AppError::Internal(err.to_string()))?;

    Ok(is_member)
}
