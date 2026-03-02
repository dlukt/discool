use crate::{AppError, db::DbPool};
use serde::Serialize;

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct User {
    pub id: String,
    pub did_key: String,
    pub public_key_multibase: String,
    pub username: String,
    pub display_name: Option<String>,
    pub avatar_color: Option<String>,
    pub avatar_storage_key: Option<String>,
    pub avatar_mime_type: Option<String>,
    pub avatar_size_bytes: Option<i64>,
    pub avatar_updated_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct UserResponse {
    pub id: String,
    pub did_key: String,
    pub username: String,
    pub display_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar_color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar_url: Option<String>,
    pub created_at: String,
}

impl From<User> for UserResponse {
    fn from(user: User) -> Self {
        let display_name = user
            .display_name
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or(&user.username)
            .to_string();
        let avatar_url = user
            .avatar_storage_key
            .as_ref()
            .map(|_| "/api/v1/users/me/avatar".to_string());

        Self {
            id: user.id,
            did_key: user.did_key,
            username: user.username,
            display_name,
            avatar_color: user.avatar_color,
            avatar_url,
            created_at: user.created_at,
        }
    }
}

pub async fn find_user_by_id(pool: &DbPool, user_id: &str) -> Result<Option<User>, AppError> {
    let user = match pool {
        DbPool::Postgres(pool) => {
            sqlx::query_as(
                "SELECT id, did_key, public_key_multibase, username, display_name, avatar_color, avatar_storage_key, avatar_mime_type, avatar_size_bytes, avatar_updated_at, created_at, updated_at
                 FROM users
                 WHERE id = $1
                 LIMIT 1",
            )
            .bind(user_id)
            .fetch_optional(pool)
            .await
        }
        DbPool::Sqlite(pool) => {
            sqlx::query_as(
                "SELECT id, did_key, public_key_multibase, username, display_name, avatar_color, avatar_storage_key, avatar_mime_type, avatar_size_bytes, avatar_updated_at, created_at, updated_at
                 FROM users
                 WHERE id = ?1
                 LIMIT 1",
            )
            .bind(user_id)
            .fetch_optional(pool)
            .await
        }
    }
    .map_err(|err| AppError::Internal(err.to_string()))?;

    Ok(user)
}
