use crate::{AppError, db::DbPool};

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct GuildMemberProfile {
    pub user_id: String,
    pub username: String,
    pub display_name: Option<String>,
    pub avatar_color: Option<String>,
    pub joined_at: String,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct UserProfile {
    pub user_id: String,
    pub username: String,
    pub display_name: Option<String>,
    pub avatar_color: Option<String>,
}

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

pub async fn list_guild_member_profiles(
    pool: &DbPool,
    guild_id: &str,
) -> Result<Vec<GuildMemberProfile>, AppError> {
    let members = match pool {
        DbPool::Postgres(pool) => {
            sqlx::query_as(
                "SELECT gm.user_id AS user_id,
                        u.username AS username,
                        u.display_name AS display_name,
                        u.avatar_color AS avatar_color,
                        gm.joined_at AS joined_at
                 FROM guild_members gm
                 JOIN users u
                   ON u.id = gm.user_id
                 WHERE gm.guild_id = $1
                 ORDER BY gm.joined_at ASC",
            )
            .bind(guild_id)
            .fetch_all(pool)
            .await
        }
        DbPool::Sqlite(pool) => {
            sqlx::query_as(
                "SELECT gm.user_id AS user_id,
                        u.username AS username,
                        u.display_name AS display_name,
                        u.avatar_color AS avatar_color,
                        gm.joined_at AS joined_at
                 FROM guild_members gm
                 JOIN users u
                   ON u.id = gm.user_id
                 WHERE gm.guild_id = ?1
                 ORDER BY gm.joined_at ASC",
            )
            .bind(guild_id)
            .fetch_all(pool)
            .await
        }
    }
    .map_err(|err| AppError::Internal(err.to_string()))?;

    Ok(members)
}

pub async fn find_guild_member_profile(
    pool: &DbPool,
    guild_id: &str,
    user_id: &str,
) -> Result<Option<GuildMemberProfile>, AppError> {
    let member = match pool {
        DbPool::Postgres(pool) => {
            sqlx::query_as(
                "SELECT gm.user_id AS user_id,
                        u.username AS username,
                        u.display_name AS display_name,
                        u.avatar_color AS avatar_color,
                        gm.joined_at AS joined_at
                 FROM guild_members gm
                 JOIN users u
                   ON u.id = gm.user_id
                 WHERE gm.guild_id = $1
                   AND gm.user_id = $2
                 LIMIT 1",
            )
            .bind(guild_id)
            .bind(user_id)
            .fetch_optional(pool)
            .await
        }
        DbPool::Sqlite(pool) => {
            sqlx::query_as(
                "SELECT gm.user_id AS user_id,
                        u.username AS username,
                        u.display_name AS display_name,
                        u.avatar_color AS avatar_color,
                        gm.joined_at AS joined_at
                 FROM guild_members gm
                 JOIN users u
                   ON u.id = gm.user_id
                 WHERE gm.guild_id = ?1
                   AND gm.user_id = ?2
                 LIMIT 1",
            )
            .bind(guild_id)
            .bind(user_id)
            .fetch_optional(pool)
            .await
        }
    }
    .map_err(|err| AppError::Internal(err.to_string()))?;

    Ok(member)
}

pub async fn find_user_profile_by_id(
    pool: &DbPool,
    user_id: &str,
) -> Result<Option<UserProfile>, AppError> {
    let profile = match pool {
        DbPool::Postgres(pool) => {
            sqlx::query_as(
                "SELECT u.id AS user_id,
                        u.username AS username,
                        u.display_name AS display_name,
                        u.avatar_color AS avatar_color
                 FROM users u
                 WHERE u.id = $1
                 LIMIT 1",
            )
            .bind(user_id)
            .fetch_optional(pool)
            .await
        }
        DbPool::Sqlite(pool) => {
            sqlx::query_as(
                "SELECT u.id AS user_id,
                        u.username AS username,
                        u.display_name AS display_name,
                        u.avatar_color AS avatar_color
                 FROM users u
                 WHERE u.id = ?1
                 LIMIT 1",
            )
            .bind(user_id)
            .fetch_optional(pool)
            .await
        }
    }
    .map_err(|err| AppError::Internal(err.to_string()))?;

    Ok(profile)
}

pub async fn users_share_guild(
    pool: &DbPool,
    left_user_id: &str,
    right_user_id: &str,
) -> Result<bool, AppError> {
    let shares_guild = match pool {
        DbPool::Postgres(pool) => {
            sqlx::query_scalar::<_, bool>(
                "SELECT EXISTS(
                SELECT 1
                FROM guild_members left_member
                JOIN guild_members right_member
                  ON right_member.guild_id = left_member.guild_id
                WHERE left_member.user_id = $1
                  AND right_member.user_id = $2
            )",
            )
            .bind(left_user_id)
            .bind(right_user_id)
            .fetch_one(pool)
            .await
        }
        DbPool::Sqlite(pool) => sqlx::query_scalar::<_, i64>(
            "SELECT EXISTS(
                SELECT 1
                FROM guild_members left_member
                JOIN guild_members right_member
                  ON right_member.guild_id = left_member.guild_id
                WHERE left_member.user_id = ?1
                  AND right_member.user_id = ?2
            )",
        )
        .bind(left_user_id)
        .bind(right_user_id)
        .fetch_one(pool)
        .await
        .map(|value| value != 0),
    }
    .map_err(|err| AppError::Internal(err.to_string()))?;

    Ok(shares_guild)
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
