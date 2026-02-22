use serde::Serialize;

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct User {
    pub id: String,
    pub did_key: String,
    pub public_key_multibase: String,
    pub username: String,
    pub avatar_color: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct UserResponse {
    pub id: String,
    pub did_key: String,
    pub username: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar_color: Option<String>,
    pub created_at: String,
}

impl From<User> for UserResponse {
    fn from(user: User) -> Self {
        Self {
            id: user.id,
            did_key: user.did_key,
            username: user.username,
            avatar_color: user.avatar_color,
            created_at: user.created_at,
        }
    }
}
