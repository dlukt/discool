use axum::{
    Json,
    extract::State,
    extract::rejection::JsonRejection,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use chrono::Utc;
use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;

use crate::{
    AppError, AppState,
    db::DbPool,
    identity::{did, keypair},
    models::user::UserResponse,
};

const MAX_USERNAME_LEN: usize = 32;
const MAX_DID_KEY_LEN: usize = 128;

#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub did_key: Option<String>,
    pub username: Option<String>,
    pub avatar_color: Option<String>,
}

fn is_hex_color(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.len() != 7 || bytes[0] != b'#' {
        return false;
    }

    bytes[1..].iter().all(|b| b.is_ascii_hexdigit())
}

fn is_valid_username(username: &str) -> bool {
    let len = username.chars().count();
    if len == 0 || len > MAX_USERNAME_LEN {
        return false;
    }

    username
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

async fn user_exists_by_did(pool: &DbPool, did_key: &str) -> Result<bool, AppError> {
    let exists: Option<i32> = match pool {
        DbPool::Postgres(pool) => {
            sqlx::query_scalar("SELECT 1 FROM users WHERE did_key = $1 LIMIT 1")
                .bind(did_key)
                .fetch_optional(pool)
                .await
        }
        DbPool::Sqlite(pool) => {
            sqlx::query_scalar("SELECT 1 FROM users WHERE did_key = ?1 LIMIT 1")
                .bind(did_key)
                .fetch_optional(pool)
                .await
        }
    }
    .map_err(|err| AppError::Internal(err.to_string()))?;

    Ok(exists.is_some())
}

async fn user_exists_by_username(pool: &DbPool, username: &str) -> Result<bool, AppError> {
    let exists: Option<i32> = match pool {
        DbPool::Postgres(pool) => {
            sqlx::query_scalar("SELECT 1 FROM users WHERE username = $1 LIMIT 1")
                .bind(username)
                .fetch_optional(pool)
                .await
        }
        DbPool::Sqlite(pool) => {
            sqlx::query_scalar("SELECT 1 FROM users WHERE username = ?1 LIMIT 1")
                .bind(username)
                .fetch_optional(pool)
                .await
        }
    }
    .map_err(|err| AppError::Internal(err.to_string()))?;

    Ok(exists.is_some())
}

pub async fn register(
    State(state): State<AppState>,
    payload: Result<Json<RegisterRequest>, JsonRejection>,
) -> Result<Response, AppError> {
    let Json(req) =
        payload.map_err(|_| AppError::ValidationError("Invalid request body".to_string()))?;

    let username = req.username.as_deref().unwrap_or("").trim();
    if username.is_empty() {
        return Err(AppError::ValidationError(
            "username is required".to_string(),
        ));
    }
    if !is_valid_username(username) {
        return Err(AppError::ValidationError(
            "username must be 1-32 chars and contain only letters, numbers, underscore, or hyphen"
                .to_string(),
        ));
    }

    let avatar_color = req
        .avatar_color
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());
    if avatar_color.is_some_and(|color| !is_hex_color(color)) {
        return Err(AppError::ValidationError(
            "avatar_color must be a hex color like #3399ff".to_string(),
        ));
    }
    let avatar_color = avatar_color.map(str::to_string);

    let did_key = req.did_key.as_deref().unwrap_or("").trim();
    if did_key.is_empty() {
        return Err(AppError::ValidationError("did_key is required".to_string()));
    }
    if did_key.len() > MAX_DID_KEY_LEN {
        return Err(AppError::ValidationError("did_key is too long".to_string()));
    }
    if !did_key.starts_with("did:key:z6Mk") {
        return Err(AppError::ValidationError(
            "Invalid DID format: must start with did:key:z6Mk".to_string(),
        ));
    }

    let public_key = did::parse_did_key(did_key)
        .map_err(|_| AppError::ValidationError("Invalid DID format".to_string()))?;
    keypair::validate_ed25519_public_key(&public_key)
        .map_err(|_| AppError::ValidationError("Invalid Ed25519 public key".to_string()))?;

    let public_key_multibase = did_key
        .strip_prefix("did:key:")
        .ok_or_else(|| AppError::ValidationError("Invalid DID format".to_string()))?
        .to_string();

    let id = Uuid::new_v4().to_string();
    let created_at = Utc::now().to_rfc3339();
    let updated_at = created_at.clone();

    let inserted = match &state.pool {
        DbPool::Postgres(pool) => sqlx::query(
            "INSERT INTO users (id, did_key, public_key_multibase, username, avatar_color, created_at, updated_at)\n             VALUES ($1, $2, $3, $4, $5, $6, $7)\n             ON CONFLICT DO NOTHING",
        )
        .bind(&id)
        .bind(did_key)
        .bind(&public_key_multibase)
        .bind(username)
        .bind(avatar_color.as_deref())
        .bind(&created_at)
        .bind(&updated_at)
        .execute(pool)
        .await
        .map_err(|err| AppError::Internal(err.to_string()))?
        .rows_affected()
            == 1,
        DbPool::Sqlite(pool) => sqlx::query(
            "INSERT INTO users (id, did_key, public_key_multibase, username, avatar_color, created_at, updated_at)\n             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)\n             ON CONFLICT DO NOTHING",
        )
        .bind(&id)
        .bind(did_key)
        .bind(&public_key_multibase)
        .bind(username)
        .bind(avatar_color.as_deref())
        .bind(&created_at)
        .bind(&updated_at)
        .execute(pool)
        .await
        .map_err(|err| AppError::Internal(err.to_string()))?
        .rows_affected()
            == 1,
    };

    if !inserted {
        if user_exists_by_did(&state.pool, did_key).await? {
            return Err(AppError::Conflict(
                "Identity already registered on this instance".to_string(),
            ));
        }
        if user_exists_by_username(&state.pool, username).await? {
            return Err(AppError::Conflict("Username already taken".to_string()));
        }
        return Err(AppError::Conflict("Registration conflict".to_string()));
    }

    let user = UserResponse {
        id,
        did_key: did_key.to_string(),
        username: username.to_string(),
        avatar_color,
        created_at,
    };

    Ok((StatusCode::CREATED, Json(json!({ "data": user }))).into_response())
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::time::Instant;

    use axum::{Json, body::to_bytes, extract::State, response::IntoResponse};

    use super::*;

    async fn test_state() -> AppState {
        let mut cfg = crate::config::Config::default();
        cfg.database = Some(crate::config::DatabaseConfig {
            url: "sqlite::memory:".to_string(),
            max_connections: 1,
        });

        let pool = crate::db::init_pool(cfg.database.as_ref().unwrap())
            .await
            .unwrap();
        crate::db::run_migrations(&pool).await.unwrap();

        AppState {
            config: Arc::new(cfg),
            pool,
            start_time: Instant::now(),
        }
    }

    async fn json_value(res: Response) -> serde_json::Value {
        let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
        serde_json::from_slice(&body).unwrap()
    }

    fn did_for_signing_key(secret: [u8; 32]) -> String {
        let signing = ed25519_dalek::SigningKey::from_bytes(&secret);
        let public = signing.verifying_key().to_bytes();

        let mut bytes = Vec::with_capacity(34);
        bytes.extend_from_slice(&[0xed, 0x01]);
        bytes.extend_from_slice(&public);
        format!("did:key:z{}", bs58::encode(bytes).into_string())
    }

    #[tokio::test]
    async fn register_creates_user() {
        let state = test_state().await;
        let did_key = did_for_signing_key([1u8; 32]);

        let req = RegisterRequest {
            did_key: Some(did_key.clone()),
            username: Some("liam".to_string()),
            avatar_color: Some("#3B82F6".to_string()),
        };

        let res = register(State(state), Ok(Json(req))).await.unwrap();
        assert_eq!(res.status(), StatusCode::CREATED);

        let value = json_value(res).await;
        assert_eq!(value["data"]["did_key"], json!(did_key));
        assert_eq!(value["data"]["username"], json!("liam"));
        assert_eq!(value["data"]["avatar_color"], json!("#3B82F6"));
        assert!(value["data"]["id"].as_str().is_some());
        assert!(value["data"]["created_at"].as_str().is_some());
    }

    #[tokio::test]
    async fn register_returns_409_for_duplicate_did() {
        let state = test_state().await;
        let did_key = did_for_signing_key([1u8; 32]);

        let _ = register(
            State(state.clone()),
            Ok(Json(RegisterRequest {
                did_key: Some(did_key.clone()),
                username: Some("liam".to_string()),
                avatar_color: None,
            })),
        )
        .await
        .unwrap();

        let err = register(
            State(state),
            Ok(Json(RegisterRequest {
                did_key: Some(did_key),
                username: Some("other".to_string()),
                avatar_color: None,
            })),
        )
        .await
        .unwrap_err();

        let res = err.into_response();
        assert_eq!(res.status(), StatusCode::CONFLICT);
        let value = json_value(res).await;
        assert_eq!(
            value,
            json!({ "error": { "code": "CONFLICT", "message": "Identity already registered on this instance", "details": {} } })
        );
    }

    #[tokio::test]
    async fn register_returns_409_for_duplicate_username() {
        let state = test_state().await;

        let _ = register(
            State(state.clone()),
            Ok(Json(RegisterRequest {
                did_key: Some(did_for_signing_key([1u8; 32])),
                username: Some("liam".to_string()),
                avatar_color: None,
            })),
        )
        .await
        .unwrap();

        let err = register(
            State(state),
            Ok(Json(RegisterRequest {
                did_key: Some(did_for_signing_key([2u8; 32])),
                username: Some("liam".to_string()),
                avatar_color: None,
            })),
        )
        .await
        .unwrap_err();

        let res = err.into_response();
        assert_eq!(res.status(), StatusCode::CONFLICT);
        let value = json_value(res).await;
        assert_eq!(
            value,
            json!({ "error": { "code": "CONFLICT", "message": "Username already taken", "details": {} } })
        );
    }

    #[tokio::test]
    async fn register_returns_422_for_invalid_did_prefix() {
        let state = test_state().await;
        let err = register(
            State(state),
            Ok(Json(RegisterRequest {
                did_key: Some("nope".to_string()),
                username: Some("liam".to_string()),
                avatar_color: None,
            })),
        )
        .await
        .unwrap_err();

        let res = err.into_response();
        assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
        let value = json_value(res).await;
        assert_eq!(
            value,
            json!({ "error": { "code": "VALIDATION_ERROR", "message": "Invalid DID format: must start with did:key:z6Mk", "details": {} } })
        );
    }

    #[tokio::test]
    async fn register_returns_422_for_too_long_did_key() {
        let state = test_state().await;
        let did_key = format!("did:key:z6Mk{}", "a".repeat(256));
        let err = register(
            State(state),
            Ok(Json(RegisterRequest {
                did_key: Some(did_key),
                username: Some("liam".to_string()),
                avatar_color: None,
            })),
        )
        .await
        .unwrap_err();

        let res = err.into_response();
        assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
        let value = json_value(res).await;
        assert_eq!(
            value,
            json!({ "error": { "code": "VALIDATION_ERROR", "message": "did_key is too long", "details": {} } })
        );
    }

    #[tokio::test]
    async fn register_returns_422_for_invalid_username_chars() {
        let state = test_state().await;
        let err = register(
            State(state),
            Ok(Json(RegisterRequest {
                did_key: Some(did_for_signing_key([1u8; 32])),
                username: Some("liam!".to_string()),
                avatar_color: None,
            })),
        )
        .await
        .unwrap_err();

        let res = err.into_response();
        assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
        let value = json_value(res).await;
        assert_eq!(value["error"]["code"], json!("VALIDATION_ERROR"));
    }

    #[tokio::test]
    async fn register_returns_422_for_empty_username() {
        let state = test_state().await;
        let err = register(
            State(state),
            Ok(Json(RegisterRequest {
                did_key: Some(did_for_signing_key([1u8; 32])),
                username: Some("   ".to_string()),
                avatar_color: None,
            })),
        )
        .await
        .unwrap_err();

        let res = err.into_response();
        assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
        let value = json_value(res).await;
        assert_eq!(
            value,
            json!({ "error": { "code": "VALIDATION_ERROR", "message": "username is required", "details": {} } })
        );
    }

    #[tokio::test]
    async fn register_returns_422_for_username_too_long() {
        let state = test_state().await;
        let too_long = "a".repeat(33);
        let err = register(
            State(state),
            Ok(Json(RegisterRequest {
                did_key: Some(did_for_signing_key([1u8; 32])),
                username: Some(too_long),
                avatar_color: None,
            })),
        )
        .await
        .unwrap_err();

        let res = err.into_response();
        assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
        let value = json_value(res).await;
        assert_eq!(value["error"]["code"], json!("VALIDATION_ERROR"));
    }

    #[tokio::test]
    async fn register_returns_422_for_invalid_avatar_color() {
        let state = test_state().await;
        let err = register(
            State(state),
            Ok(Json(RegisterRequest {
                did_key: Some(did_for_signing_key([1u8; 32])),
                username: Some("liam".to_string()),
                avatar_color: Some("javascript:alert(1)".to_string()),
            })),
        )
        .await
        .unwrap_err();

        let res = err.into_response();
        assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
        let value = json_value(res).await;
        assert_eq!(
            value,
            json!({ "error": { "code": "VALIDATION_ERROR", "message": "avatar_color must be a hex color like #3399ff", "details": {} } })
        );
    }
}
