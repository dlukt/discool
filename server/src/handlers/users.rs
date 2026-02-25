use axum::{
    Json,
    body::Body,
    extract::rejection::JsonRejection,
    extract::{Multipart, State},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
};
use serde::Deserialize;
use serde_json::json;

use crate::{
    AppError, AppState,
    middleware::auth::AuthenticatedUser,
    services::{
        email_service, recovery_email_service,
        user_profile_service::{self, UpdateProfileInput},
    },
};

#[derive(Debug, Deserialize)]
pub struct UpdateProfileRequest {
    #[serde(default)]
    pub display_name: Option<Option<String>>,
    #[serde(default)]
    pub avatar_color: Option<Option<String>>,
}

#[derive(Debug, Deserialize)]
pub struct StartRecoveryEmailRequest {
    pub email: Option<String>,
    pub encrypted_private_key: Option<String>,
    pub encryption_context: Option<RecoveryEmailEncryptionContextRequest>,
}

#[derive(Debug, Deserialize)]
pub struct RecoveryEmailEncryptionContextRequest {
    pub algorithm: Option<String>,
    pub version: Option<i64>,
}

pub async fn get_profile(
    State(state): State<AppState>,
    user: AuthenticatedUser,
) -> Result<Response, AppError> {
    let profile = user_profile_service::get_profile(&state.pool, &user.user_id).await?;
    Ok((StatusCode::OK, Json(json!({ "data": profile }))).into_response())
}

pub async fn update_profile(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    payload: Result<Json<UpdateProfileRequest>, JsonRejection>,
) -> Result<Response, AppError> {
    let Json(req) =
        payload.map_err(|_| AppError::ValidationError("Invalid request body".to_string()))?;
    let profile = user_profile_service::update_profile(
        &state.pool,
        &state.config.avatar,
        &user.user_id,
        UpdateProfileInput {
            display_name: req.display_name,
            avatar_color: req.avatar_color,
        },
    )
    .await?;

    Ok((StatusCode::OK, Json(json!({ "data": profile }))).into_response())
}

pub async fn get_recovery_email(
    State(state): State<AppState>,
    user: AuthenticatedUser,
) -> Result<Response, AppError> {
    let status =
        recovery_email_service::get_recovery_email_status(&state.pool, &user.user_id).await?;
    Ok((StatusCode::OK, Json(json!({ "data": status }))).into_response())
}

pub async fn start_recovery_email(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    headers: HeaderMap,
    payload: Result<Json<StartRecoveryEmailRequest>, JsonRejection>,
) -> Result<Response, AppError> {
    let Json(req) =
        payload.map_err(|_| AppError::ValidationError("Invalid request body".to_string()))?;
    let email = req.email.as_deref().unwrap_or("").trim();
    if email.is_empty() {
        return Err(AppError::ValidationError("email is required".to_string()));
    }
    let encrypted_private_key = req.encrypted_private_key.as_deref().unwrap_or("").trim();
    if encrypted_private_key.is_empty() {
        return Err(AppError::ValidationError(
            "encrypted_private_key is required".to_string(),
        ));
    }

    let encryption_context = req
        .encryption_context
        .ok_or_else(|| AppError::ValidationError("encryption_context is required".to_string()))?;
    let algorithm = encryption_context.algorithm.as_deref().unwrap_or("").trim();
    if algorithm.is_empty() {
        return Err(AppError::ValidationError(
            "encryption_context.algorithm is required".to_string(),
        ));
    }
    let version = encryption_context.version.unwrap_or(0);
    if version <= 0 {
        return Err(AppError::ValidationError(
            "encryption_context.version must be >= 1".to_string(),
        ));
    }

    let requester_ip = recovery_email_service::requester_ip_from_headers(&headers);
    let started = recovery_email_service::start_recovery_email_association(
        &state.pool,
        &state.config.email,
        &user.user_id,
        &recovery_email_service::StartRecoveryEmailInput {
            email: email.to_string(),
            encrypted_private_key: encrypted_private_key.to_string(),
            encryption_algorithm: algorithm.to_string(),
            encryption_version: version,
            requester_ip,
        },
    )
    .await?;

    email_service::send_recovery_verification_email(
        &state.config.email,
        &started.normalized_email,
        &started.token,
    )
    .await?;

    Ok((StatusCode::OK, Json(json!({ "data": started.status }))).into_response())
}

pub async fn upload_avatar(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    mut multipart: Multipart,
) -> Result<Response, AppError> {
    let mut avatar_bytes: Option<Vec<u8>> = None;
    let mut avatar_content_type: Option<String> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|_| AppError::ValidationError("Invalid multipart payload".to_string()))?
    {
        if field.name() == Some("avatar") {
            avatar_content_type = field.content_type().map(str::to_string);
            let bytes = field
                .bytes()
                .await
                .map_err(|_| AppError::ValidationError("Invalid avatar payload".to_string()))?;
            avatar_bytes = Some(bytes.to_vec());
            break;
        }
    }

    let avatar_bytes = avatar_bytes
        .ok_or_else(|| AppError::ValidationError("avatar field is required".to_string()))?;

    let profile = user_profile_service::save_avatar(
        &state.pool,
        &state.config.avatar,
        &user.user_id,
        avatar_content_type.as_deref(),
        &avatar_bytes,
    )
    .await?;

    Ok((StatusCode::OK, Json(json!({ "data": profile }))).into_response())
}

pub async fn get_avatar(
    State(state): State<AppState>,
    user: AuthenticatedUser,
) -> Result<Response, AppError> {
    let (bytes, mime) =
        user_profile_service::load_avatar(&state.pool, &state.config.avatar, &user.user_id).await?;

    let mut response = Response::new(Body::from(bytes));
    *response.status_mut() = StatusCode::OK;
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        mime.parse()
            .map_err(|_| AppError::Internal("Invalid avatar MIME type".to_string()))?,
    );
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        header::HeaderValue::from_static("private, max-age=0, must-revalidate"),
    );
    Ok(response)
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::time::Instant;

    use axum::{Json, body::to_bytes, extract::State, http::HeaderMap};
    use dashmap::DashMap;
    use uuid::Uuid;

    use super::*;

    async fn test_state_with_email_limits(
        start_rate_limit_per_hour: u32,
        verify_rate_limit_per_hour: u32,
    ) -> AppState {
        let mut cfg = crate::config::Config::default();
        cfg.database = Some(crate::config::DatabaseConfig {
            url: "sqlite::memory:".to_string(),
            max_connections: 1,
        });
        cfg.email.start_rate_limit_per_hour = start_rate_limit_per_hour;
        cfg.email.verify_rate_limit_per_hour = verify_rate_limit_per_hour;

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

    async fn test_state() -> AppState {
        let defaults = crate::config::EmailConfig::default();
        test_state_with_email_limits(
            defaults.start_rate_limit_per_hour,
            defaults.verify_rate_limit_per_hour,
        )
        .await
    }

    async fn insert_user(state: &AppState) -> AuthenticatedUser {
        let user_id = Uuid::new_v4().to_string();
        let created_at = chrono::Utc::now().to_rfc3339();
        let updated_at = created_at.clone();
        match &state.pool {
            crate::db::DbPool::Postgres(pool) => {
                sqlx::query("INSERT INTO users (id, did_key, public_key_multibase, username, avatar_color, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6, $7)")
                    .bind(&user_id)
                    .bind("did:key:z6Mk-test")
                    .bind("z6Mk-test")
                    .bind("liam")
                    .bind(Option::<String>::None)
                    .bind(&created_at)
                    .bind(&updated_at)
                    .execute(pool)
                    .await
                    .unwrap();
            }
            crate::db::DbPool::Sqlite(pool) => {
                sqlx::query("INSERT INTO users (id, did_key, public_key_multibase, username, avatar_color, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)")
                    .bind(&user_id)
                    .bind("did:key:z6Mk-test")
                    .bind("z6Mk-test")
                    .bind("liam")
                    .bind(Option::<String>::None)
                    .bind(&created_at)
                    .bind(&updated_at)
                    .execute(pool)
                    .await
                    .unwrap();
            }
        }

        AuthenticatedUser {
            user_id,
            session_id: "session-id".to_string(),
            username: "liam".to_string(),
            did_key: "did:key:z6Mk-test".to_string(),
        }
    }

    async fn json_value(res: Response) -> serde_json::Value {
        let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
        serde_json::from_slice(&body).unwrap()
    }

    #[tokio::test]
    async fn get_profile_returns_username_as_default_display_name() {
        let state = test_state().await;
        let user = insert_user(&state).await;

        let res = get_profile(State(state), user).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        let value = json_value(res).await;
        assert_eq!(value["data"]["username"], json!("liam"));
        assert_eq!(value["data"]["display_name"], json!("liam"));
    }

    #[tokio::test]
    async fn update_profile_returns_422_when_payload_has_no_fields() {
        let state = test_state().await;
        let user = insert_user(&state).await;

        let err = update_profile(
            State(state),
            user,
            Ok(Json(UpdateProfileRequest {
                display_name: None,
                avatar_color: None,
            })),
        )
        .await
        .unwrap_err();

        let res = err.into_response();
        assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[tokio::test]
    async fn update_profile_persists_display_name_and_avatar_color() {
        let state = test_state().await;
        let user = insert_user(&state).await;

        let res = update_profile(
            State(state.clone()),
            user.clone(),
            Ok(Json(UpdateProfileRequest {
                display_name: Some(Some("Liam".to_string())),
                avatar_color: Some(Some("#3B82F6".to_string())),
            })),
        )
        .await
        .unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        let value = json_value(res).await;
        assert_eq!(value["data"]["display_name"], json!("Liam"));
        assert_eq!(value["data"]["avatar_color"], json!("#3B82F6"));

        let res = get_profile(State(state), user).await.unwrap();
        let value = json_value(res).await;
        assert_eq!(value["data"]["display_name"], json!("Liam"));
        assert_eq!(value["data"]["avatar_color"], json!("#3B82F6"));
    }

    #[tokio::test]
    async fn get_recovery_email_returns_unassociated_by_default() {
        let state = test_state().await;
        let user = insert_user(&state).await;

        let res = get_recovery_email(State(state), user).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        let value = json_value(res).await;
        assert_eq!(value["data"]["associated"], json!(false));
        assert_eq!(value["data"]["verified"], json!(false));
        assert!(value["data"]["email_masked"].is_null());
    }

    #[tokio::test]
    async fn start_recovery_email_returns_unverified_status_and_persists_token() {
        let state = test_state().await;
        let user = insert_user(&state).await;

        let res = start_recovery_email(
            State(state.clone()),
            user.clone(),
            HeaderMap::new(),
            Ok(Json(StartRecoveryEmailRequest {
                email: Some("liam@example.com".to_string()),
                encrypted_private_key: Some("c2VjcmV0".to_string()),
                encryption_context: Some(RecoveryEmailEncryptionContextRequest {
                    algorithm: Some("aes-256-gcm".to_string()),
                    version: Some(1),
                }),
            })),
        )
        .await
        .unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        let value = json_value(res).await;
        assert_eq!(value["data"]["associated"], json!(true));
        assert_eq!(value["data"]["verified"], json!(false));
        assert_eq!(value["data"]["email_masked"], json!("l***@example.com"));

        let token_count: i64 = match &state.pool {
            crate::db::DbPool::Postgres(pool) => sqlx::query_scalar(
                "SELECT COUNT(*) FROM email_verification_tokens WHERE user_id = $1",
            )
            .bind(&user.user_id)
            .fetch_one(pool)
            .await
            .unwrap(),
            crate::db::DbPool::Sqlite(pool) => sqlx::query_scalar(
                "SELECT COUNT(*) FROM email_verification_tokens WHERE user_id = ?1",
            )
            .bind(&user.user_id)
            .fetch_one(pool)
            .await
            .unwrap(),
        };
        assert_eq!(token_count, 1);
    }

    #[tokio::test]
    async fn start_recovery_email_returns_422_for_malformed_email() {
        let state = test_state().await;
        let user = insert_user(&state).await;

        let err = start_recovery_email(
            State(state),
            user,
            HeaderMap::new(),
            Ok(Json(StartRecoveryEmailRequest {
                email: Some("not-an-email".to_string()),
                encrypted_private_key: Some("c2VjcmV0".to_string()),
                encryption_context: Some(RecoveryEmailEncryptionContextRequest {
                    algorithm: Some("aes-256-gcm".to_string()),
                    version: Some(1),
                }),
            })),
        )
        .await
        .unwrap_err();
        assert_eq!(
            err.into_response().status(),
            StatusCode::UNPROCESSABLE_ENTITY
        );
    }

    #[tokio::test]
    async fn start_recovery_email_enforces_hourly_limit_across_retries() {
        let state = test_state_with_email_limits(2, 20).await;
        let user = insert_user(&state).await;

        let request_payload = || {
            Ok(Json(StartRecoveryEmailRequest {
                email: Some("liam@example.com".to_string()),
                encrypted_private_key: Some("c2VjcmV0".to_string()),
                encryption_context: Some(RecoveryEmailEncryptionContextRequest {
                    algorithm: Some("aes-256-gcm".to_string()),
                    version: Some(1),
                }),
            }))
        };

        let first = start_recovery_email(
            State(state.clone()),
            user.clone(),
            HeaderMap::new(),
            request_payload(),
        )
        .await
        .unwrap();
        assert_eq!(first.status(), StatusCode::OK);

        let second = start_recovery_email(
            State(state.clone()),
            user.clone(),
            HeaderMap::new(),
            request_payload(),
        )
        .await
        .unwrap();
        assert_eq!(second.status(), StatusCode::OK);

        let err = start_recovery_email(State(state), user, HeaderMap::new(), request_payload())
            .await
            .unwrap_err();
        assert_eq!(
            err.into_response().status(),
            StatusCode::UNPROCESSABLE_ENTITY
        );
    }
}
