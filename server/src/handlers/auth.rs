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
    identity::{challenge, did, keypair},
    models::user::UserResponse,
    services::auth_service,
};

const MAX_USERNAME_LEN: usize = 32;
const MAX_DID_KEY_LEN: usize = 128;
const MAX_DISPLAY_NAME_LEN: usize = 64;

#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub did_key: Option<String>,
    pub username: Option<String>,
    pub avatar_color: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ChallengeRequest {
    pub did_key: Option<String>,
    #[serde(default)]
    pub cross_instance: Option<CrossInstanceChallengeRequest>,
}

#[derive(Debug, Deserialize, Default)]
pub struct CrossInstanceChallengeRequest {
    #[serde(default)]
    pub enabled: bool,
    pub username: Option<String>,
    pub display_name: Option<String>,
    pub avatar_color: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct VerifyRequest {
    pub did_key: Option<String>,
    pub challenge: Option<String>,
    pub signature: Option<String>,
    #[serde(default)]
    pub cross_instance: Option<CrossInstanceVerifyRequest>,
}

#[derive(Debug, Deserialize, Default)]
pub struct CrossInstanceVerifyRequest {
    #[serde(default)]
    pub enabled: bool,
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

fn normalize_display_name_hint(value: Option<&str>) -> Result<Option<String>, AppError> {
    let Some(value) = value.map(str::trim) else {
        return Ok(None);
    };
    if value.is_empty() {
        return Ok(None);
    }
    if value.chars().count() > MAX_DISPLAY_NAME_LEN {
        return Err(AppError::ValidationError(format!(
            "display_name must be {MAX_DISPLAY_NAME_LEN} characters or less"
        )));
    }
    if value.chars().any(|ch| ch.is_control()) {
        return Err(AppError::ValidationError(
            "display_name contains invalid characters".to_string(),
        ));
    }
    Ok(Some(value.to_string()))
}

fn normalize_avatar_color_hint(value: Option<&str>) -> Result<Option<String>, AppError> {
    let Some(value) = value.map(str::trim) else {
        return Ok(None);
    };
    if value.is_empty() {
        return Ok(None);
    }
    if !is_hex_color(value) {
        return Err(AppError::ValidationError(
            "avatar_color must be a hex color like #3399ff".to_string(),
        ));
    }
    Ok(Some(value.to_string()))
}

fn cross_instance_onboarding_from_request(
    cross_instance: Option<&CrossInstanceChallengeRequest>,
) -> Result<Option<challenge::CrossInstanceOnboarding>, AppError> {
    let Some(cross_instance) = cross_instance else {
        return Ok(None);
    };
    if !cross_instance.enabled {
        return Ok(None);
    }

    let username = cross_instance.username.as_deref().unwrap_or("").trim();
    if username.is_empty() {
        return Err(AppError::ValidationError(
            "cross_instance.username is required".to_string(),
        ));
    }
    if !is_valid_username(username) {
        return Err(AppError::ValidationError(
            "username must be 1-32 chars and contain only letters, numbers, underscore, or hyphen"
                .to_string(),
        ));
    }

    let display_name = normalize_display_name_hint(cross_instance.display_name.as_deref())?;
    let avatar_color = normalize_avatar_color_hint(cross_instance.avatar_color.as_deref())?;

    Ok(Some(challenge::CrossInstanceOnboarding {
        username: username.to_string(),
        display_name,
        avatar_color,
    }))
}

fn validate_did_key_for_auth(did_key: &str) -> Result<[u8; 32], AppError> {
    let did_key = did_key.trim();
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

    Ok(public_key)
}

fn api_error_response(status: StatusCode, code: &str, message: &str) -> Response {
    (
        status,
        Json(json!({ "error": { "code": code, "message": message, "details": {} } })),
    )
        .into_response()
}

fn from_hex(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

fn decode_hex_64(value: &str) -> Result<[u8; 64], AppError> {
    let value = value.trim();
    if value.len() != 128 {
        return Err(AppError::ValidationError(
            "signature must be a 64-byte hex string".to_string(),
        ));
    }

    let bytes = value.as_bytes();
    let mut out = [0u8; 64];
    for i in 0..64 {
        let h1 = from_hex(bytes[i * 2])
            .ok_or_else(|| AppError::ValidationError("signature must be hex".to_string()))?;
        let h2 = from_hex(bytes[i * 2 + 1])
            .ok_or_else(|| AppError::ValidationError("signature must be hex".to_string()))?;
        out[i] = (h1 << 4) | h2;
    }
    Ok(out)
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
        display_name: username.to_string(),
        avatar_color,
        avatar_url: None,
        created_at,
    };

    Ok((StatusCode::CREATED, Json(json!({ "data": user }))).into_response())
}

pub async fn challenge(
    State(state): State<AppState>,
    payload: Result<Json<ChallengeRequest>, JsonRejection>,
) -> Result<Response, AppError> {
    let Json(req) =
        payload.map_err(|_| AppError::ValidationError("Invalid request body".to_string()))?;

    let did_key = req.did_key.as_deref().unwrap_or("").trim();
    let _ = validate_did_key_for_auth(did_key)?;
    let cross_instance = cross_instance_onboarding_from_request(req.cross_instance.as_ref())?;

    if cross_instance.is_none() {
        let user = auth_service::fetch_user_by_did(&state.pool, did_key).await;
        if matches!(user, Err(AppError::NotFound)) {
            return Ok(api_error_response(
                StatusCode::NOT_FOUND,
                "NOT_FOUND",
                "Identity not found on this instance",
            ));
        }
        user?;
    }

    let ttl = state.config.auth.challenge_ttl_seconds;
    let challenge =
        auth_service::create_challenge(state.challenges.as_ref(), did_key, cross_instance);

    Ok((
        StatusCode::OK,
        Json(json!({ "data": { "challenge": challenge, "expires_in": ttl } })),
    )
        .into_response())
}

pub async fn verify(
    State(state): State<AppState>,
    payload: Result<Json<VerifyRequest>, JsonRejection>,
) -> Result<Response, AppError> {
    let Json(req) =
        payload.map_err(|_| AppError::ValidationError("Invalid request body".to_string()))?;

    let did_key = req.did_key.as_deref().unwrap_or("").trim();
    let public_key = validate_did_key_for_auth(did_key)?;

    let challenge_str = req.challenge.as_deref().unwrap_or("").trim();
    if challenge_str.is_empty() {
        return Err(AppError::ValidationError(
            "challenge is required".to_string(),
        ));
    }
    if challenge_str.len() != 64 || !challenge_str.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(AppError::ValidationError(
            "challenge must be a 32-byte hex string".to_string(),
        ));
    }

    let signature_hex = req.signature.as_deref().unwrap_or("").trim();
    if signature_hex.is_empty() {
        return Err(AppError::ValidationError(
            "signature is required".to_string(),
        ));
    }

    let cross_instance_enabled = req.cross_instance.as_ref().is_some_and(|cfg| cfg.enabled);
    let existing_user = if cross_instance_enabled {
        None
    } else {
        Some(
            match auth_service::fetch_user_by_did(&state.pool, did_key).await {
                Ok(user) => user,
                Err(AppError::NotFound) => {
                    return Ok(api_error_response(
                        StatusCode::NOT_FOUND,
                        "NOT_FOUND",
                        "Identity not found on this instance",
                    ));
                }
                Err(err) => return Err(err),
            },
        )
    };

    let signature_bytes = decode_hex_64(signature_hex)?;

    // Avoid doing expensive crypto verification if there is no pending challenge (or it already expired).
    let challenge_record = match auth_service::check_challenge(
        state.challenges.as_ref(),
        did_key,
        challenge_str,
        state.config.auth.challenge_ttl_seconds,
    ) {
        Ok(record) => record,
        Err(
            auth_service::AuthError::ChallengeNotFound | auth_service::AuthError::ChallengeExpired,
        ) => {
            return Err(AppError::Unauthorized(
                "Challenge expired or not found".to_string(),
            ));
        }
        Err(auth_service::AuthError::ChallengeMismatch) => {
            return Err(AppError::Unauthorized("Challenge mismatch".to_string()));
        }
    };

    let challenge_is_cross_instance = challenge_record.cross_instance.is_some();
    if challenge_is_cross_instance != cross_instance_enabled {
        return Err(AppError::ValidationError(
            "cross_instance mode must match challenge request".to_string(),
        ));
    }

    match challenge::verify_signature(&public_key, challenge_str, &signature_bytes) {
        Ok(()) => {}
        Err(challenge::VerifyError::InvalidSignature) => {
            return Err(AppError::Unauthorized("Invalid signature".to_string()));
        }
        Err(challenge::VerifyError::InvalidPublicKey) => {
            return Err(AppError::ValidationError(
                "Invalid Ed25519 public key".to_string(),
            ));
        }
    }

    match auth_service::validate_challenge(
        state.challenges.as_ref(),
        did_key,
        challenge_str,
        state.config.auth.challenge_ttl_seconds,
    ) {
        Ok(()) => {}
        Err(
            auth_service::AuthError::ChallengeNotFound | auth_service::AuthError::ChallengeExpired,
        ) => {
            return Err(AppError::Unauthorized(
                "Challenge expired or not found".to_string(),
            ));
        }
        Err(auth_service::AuthError::ChallengeMismatch) => {
            return Err(AppError::Unauthorized("Challenge mismatch".to_string()));
        }
    }

    let user = if challenge_is_cross_instance {
        let onboarding = challenge_record
            .cross_instance
            .ok_or_else(|| AppError::Unauthorized("Challenge expired or not found".to_string()))?;
        auth_service::fetch_existing_or_create_verified_user(&state.pool, did_key, &onboarding)
            .await?
    } else {
        existing_user
            .ok_or_else(|| AppError::Unauthorized("Challenge expired or not found".to_string()))?
    };

    let session =
        auth_service::create_session(&state.pool, &user.id, state.config.auth.session_ttl_hours)
            .await?;

    let response = json!({
        "token": session.token,
        "expires_at": session.expires_at,
        "user": UserResponse::from(user),
    });

    Ok((StatusCode::OK, Json(json!({ "data": response }))).into_response())
}

pub async fn logout(
    State(state): State<AppState>,
    user: crate::middleware::auth::AuthenticatedUser,
) -> Result<Response, AppError> {
    auth_service::delete_session_by_id(&state.pool, &user.session_id).await?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::time::Duration;
    use std::time::Instant;

    use axum::{Json, body::to_bytes, extract::State, response::IntoResponse};
    use dashmap::DashMap;
    use ed25519_dalek::Signer;

    use super::*;

    fn bytes_to_hex(bytes: &[u8]) -> String {
        const HEX: &[u8; 16] = b"0123456789abcdef";
        let mut out = String::with_capacity(bytes.len() * 2);
        for &b in bytes {
            out.push(HEX[(b >> 4) as usize] as char);
            out.push(HEX[(b & 0x0f) as usize] as char);
        }
        out
    }

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
            challenges: Arc::new(DashMap::new()),
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

    fn cross_instance_challenge_request(did_key: String, username: &str) -> ChallengeRequest {
        ChallengeRequest {
            did_key: Some(did_key),
            cross_instance: Some(CrossInstanceChallengeRequest {
                enabled: true,
                username: Some(username.to_string()),
                display_name: Some(username.to_string()),
                avatar_color: Some("#3B82F6".to_string()),
            }),
        }
    }

    fn cross_instance_verify_request(
        did_key: String,
        challenge: String,
        signature: String,
    ) -> VerifyRequest {
        VerifyRequest {
            did_key: Some(did_key),
            challenge: Some(challenge),
            signature: Some(signature),
            cross_instance: Some(CrossInstanceVerifyRequest { enabled: true }),
        }
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

    #[tokio::test]
    async fn challenge_then_verify_returns_session() {
        let state = test_state().await;
        let secret = [1u8; 32];
        let did_key = did_for_signing_key(secret);

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

        let res = challenge(
            State(state.clone()),
            Ok(Json(ChallengeRequest {
                did_key: Some(did_key.clone()),
                cross_instance: None,
            })),
        )
        .await
        .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let value = json_value(res).await;
        let challenge_hex = value["data"]["challenge"].as_str().unwrap().to_string();

        let signing = ed25519_dalek::SigningKey::from_bytes(&secret);
        let sig = signing.sign(challenge_hex.as_bytes()).to_bytes();
        let signature = bytes_to_hex(&sig);

        let res = verify(
            State(state),
            Ok(Json(VerifyRequest {
                did_key: Some(did_key),
                challenge: Some(challenge_hex),
                signature: Some(signature),
                cross_instance: None,
            })),
        )
        .await
        .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let value = json_value(res).await;
        assert!(value["data"]["token"].as_str().is_some());
        assert!(value["data"]["expires_at"].as_str().is_some());
    }

    #[tokio::test]
    async fn challenge_returns_404_for_unregistered_did() {
        let state = test_state().await;
        let did_key = did_for_signing_key([1u8; 32]);

        let res = challenge(
            State(state),
            Ok(Json(ChallengeRequest {
                did_key: Some(did_key),
                cross_instance: None,
            })),
        )
        .await
        .unwrap();
        assert_eq!(res.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn challenge_cross_instance_returns_200_for_unregistered_did() {
        let state = test_state().await;
        let did_key = did_for_signing_key([9u8; 32]);

        let res = challenge(
            State(state.clone()),
            Ok(Json(cross_instance_challenge_request(
                did_key.clone(),
                "liam",
            ))),
        )
        .await
        .unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        let value = json_value(res).await;
        assert!(value["data"]["challenge"].as_str().is_some());
        let record = state.challenges.get(did_key.as_str()).unwrap();
        let onboarding = record.cross_instance.as_ref().unwrap();
        assert_eq!(onboarding.username, "liam");
    }

    #[tokio::test]
    async fn verify_cross_instance_creates_user_after_valid_signature() {
        let state = test_state().await;
        let secret = [6u8; 32];
        let did_key = did_for_signing_key(secret);

        let res = challenge(
            State(state.clone()),
            Ok(Json(cross_instance_challenge_request(
                did_key.clone(),
                "liam",
            ))),
        )
        .await
        .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let value = json_value(res).await;
        let challenge_hex = value["data"]["challenge"].as_str().unwrap().to_string();

        let signing = ed25519_dalek::SigningKey::from_bytes(&secret);
        let sig = signing.sign(challenge_hex.as_bytes()).to_bytes();
        let signature = bytes_to_hex(&sig);

        let res = verify(
            State(state.clone()),
            Ok(Json(cross_instance_verify_request(
                did_key.clone(),
                challenge_hex,
                signature,
            ))),
        )
        .await
        .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let value = json_value(res).await;
        assert_eq!(value["data"]["user"]["did_key"], json!(did_key));

        let created = auth_service::fetch_user_by_did(&state.pool, &did_key).await;
        assert!(created.is_ok());
    }

    #[tokio::test]
    async fn verify_cross_instance_invalid_signature_does_not_create_user() {
        let state = test_state().await;
        let secret = [7u8; 32];
        let did_key = did_for_signing_key(secret);

        let res = challenge(
            State(state.clone()),
            Ok(Json(cross_instance_challenge_request(
                did_key.clone(),
                "liam",
            ))),
        )
        .await
        .unwrap();
        let value = json_value(res).await;
        let challenge_hex = value["data"]["challenge"].as_str().unwrap().to_string();

        let wrong_signing = ed25519_dalek::SigningKey::from_bytes(&[8u8; 32]);
        let wrong_sig = wrong_signing.sign(challenge_hex.as_bytes()).to_bytes();

        let err = verify(
            State(state.clone()),
            Ok(Json(cross_instance_verify_request(
                did_key.clone(),
                challenge_hex,
                bytes_to_hex(&wrong_sig),
            ))),
        )
        .await
        .unwrap_err();
        assert_eq!(err.into_response().status(), StatusCode::UNAUTHORIZED);
        assert!(matches!(
            auth_service::fetch_user_by_did(&state.pool, &did_key).await,
            Err(AppError::NotFound)
        ));
    }

    #[tokio::test]
    async fn verify_cross_instance_username_conflict_is_deterministic_and_safe() {
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

        let secret = [10u8; 32];
        let did_key = did_for_signing_key(secret);

        let res = challenge(
            State(state.clone()),
            Ok(Json(cross_instance_challenge_request(
                did_key.clone(),
                "liam",
            ))),
        )
        .await
        .unwrap();
        let value = json_value(res).await;
        let challenge_hex = value["data"]["challenge"].as_str().unwrap().to_string();
        let signing = ed25519_dalek::SigningKey::from_bytes(&secret);
        let sig = signing.sign(challenge_hex.as_bytes()).to_bytes();

        let res = verify(
            State(state.clone()),
            Ok(Json(cross_instance_verify_request(
                did_key.clone(),
                challenge_hex,
                bytes_to_hex(&sig),
            ))),
        )
        .await
        .unwrap();
        let first_value = json_value(res).await;
        let first_user_id = first_value["data"]["user"]["id"]
            .as_str()
            .unwrap()
            .to_string();
        let first_username = first_value["data"]["user"]["username"]
            .as_str()
            .unwrap()
            .to_string();
        assert_ne!(first_username, "liam");

        let res = challenge(
            State(state.clone()),
            Ok(Json(cross_instance_challenge_request(
                did_key.clone(),
                "liam",
            ))),
        )
        .await
        .unwrap();
        let value = json_value(res).await;
        let second_challenge_hex = value["data"]["challenge"].as_str().unwrap().to_string();
        let second_sig = signing.sign(second_challenge_hex.as_bytes()).to_bytes();

        let res = verify(
            State(state),
            Ok(Json(cross_instance_verify_request(
                did_key,
                second_challenge_hex,
                bytes_to_hex(&second_sig),
            ))),
        )
        .await
        .unwrap();
        let second_value = json_value(res).await;
        assert_eq!(
            second_value["data"]["user"]["id"].as_str().unwrap(),
            first_user_id
        );
        assert_eq!(
            second_value["data"]["user"]["username"].as_str().unwrap(),
            first_username
        );
    }

    #[tokio::test]
    async fn verify_returns_401_for_invalid_signature() {
        let state = test_state().await;
        let secret = [1u8; 32];
        let did_key = did_for_signing_key(secret);

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

        let res = challenge(
            State(state.clone()),
            Ok(Json(ChallengeRequest {
                did_key: Some(did_key.clone()),
                cross_instance: None,
            })),
        )
        .await
        .unwrap();
        let value = json_value(res).await;
        let challenge_hex = value["data"]["challenge"].as_str().unwrap().to_string();

        let signing = ed25519_dalek::SigningKey::from_bytes(&[2u8; 32]); // wrong key
        let sig = signing.sign(challenge_hex.as_bytes()).to_bytes();

        let err = verify(
            State(state),
            Ok(Json(VerifyRequest {
                did_key: Some(did_key),
                challenge: Some(challenge_hex),
                signature: Some(bytes_to_hex(&sig)),
                cross_instance: None,
            })),
        )
        .await
        .unwrap_err();

        let res = err.into_response();
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn verify_returns_401_for_expired_challenge() {
        let state = test_state().await;
        let secret = [1u8; 32];
        let did_key = did_for_signing_key(secret);

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

        let res = challenge(
            State(state.clone()),
            Ok(Json(ChallengeRequest {
                did_key: Some(did_key.clone()),
                cross_instance: None,
            })),
        )
        .await
        .unwrap();
        let value = json_value(res).await;
        let challenge_hex = value["data"]["challenge"].as_str().unwrap().to_string();

        // Force expiry.
        if let Some(mut record) = state.challenges.get_mut(did_key.as_str()) {
            record.created_at = Instant::now() - Duration::from_secs(301);
        }

        let signing = ed25519_dalek::SigningKey::from_bytes(&secret);
        let sig = signing.sign(challenge_hex.as_bytes()).to_bytes();

        let err = verify(
            State(state),
            Ok(Json(VerifyRequest {
                did_key: Some(did_key),
                challenge: Some(challenge_hex),
                signature: Some(bytes_to_hex(&sig)),
                cross_instance: None,
            })),
        )
        .await
        .unwrap_err();

        let res = err.into_response();
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn verify_rejects_replay() {
        let state = test_state().await;
        let secret = [1u8; 32];
        let did_key = did_for_signing_key(secret);

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

        let res = challenge(
            State(state.clone()),
            Ok(Json(ChallengeRequest {
                did_key: Some(did_key.clone()),
                cross_instance: None,
            })),
        )
        .await
        .unwrap();
        let value = json_value(res).await;
        let challenge_hex = value["data"]["challenge"].as_str().unwrap().to_string();

        let signing = ed25519_dalek::SigningKey::from_bytes(&secret);
        let sig = signing.sign(challenge_hex.as_bytes()).to_bytes();
        let signature = bytes_to_hex(&sig);

        let _ = verify(
            State(state.clone()),
            Ok(Json(VerifyRequest {
                did_key: Some(did_key.clone()),
                challenge: Some(challenge_hex.clone()),
                signature: Some(signature.clone()),
                cross_instance: None,
            })),
        )
        .await
        .unwrap();

        let err = verify(
            State(state),
            Ok(Json(VerifyRequest {
                did_key: Some(did_key),
                challenge: Some(challenge_hex),
                signature: Some(signature),
                cross_instance: None,
            })),
        )
        .await
        .unwrap_err();

        let res = err.into_response();
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    }
}
