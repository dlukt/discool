use serde::Serialize;

use super::user::UserResponse;

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct Session {
    pub id: String,
    pub user_id: String,
    pub token: String,
    pub created_at: String,
    pub expires_at: String,
    pub last_active_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SessionResponse {
    pub token: String,
    pub expires_at: String,
    pub user: UserResponse,
}
