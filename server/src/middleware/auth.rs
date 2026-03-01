use axum::{
    extract::FromRequestParts,
    http::{header::AUTHORIZATION, request::Parts},
};
use chrono::{DateTime, Utc};

use crate::{AppError, AppState, services::auth_service};

#[derive(Debug, Clone)]
pub struct AuthenticatedUser {
    pub user_id: String,
    pub session_id: String,
    pub username: String,
    pub did_key: String,
}

impl FromRequestParts<AppState> for AuthenticatedUser {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let header = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| AppError::Unauthorized("Missing Authorization header".to_string()))?;

        let mut header_parts = header.split_whitespace();
        let scheme = header_parts.next().unwrap_or("");
        let token = header_parts.next().unwrap_or("");
        if !scheme.eq_ignore_ascii_case("bearer")
            || token.is_empty()
            || header_parts.next().is_some()
        {
            return Err(AppError::Unauthorized(
                "Invalid Authorization header".to_string(),
            ));
        }

        let (session, user) = auth_service::validate_session(&state.pool, token).await?;

        if should_refresh_last_active_at(&session.last_active_at)? {
            auth_service::refresh_session(&state.pool, &session.id).await?;
        }

        Ok(Self {
            user_id: user.id,
            session_id: session.id,
            username: user.username,
            did_key: user.did_key,
        })
    }
}

fn should_refresh_last_active_at(last_active_at: &str) -> Result<bool, AppError> {
    let last_active_at = match DateTime::parse_from_rfc3339(last_active_at) {
        Ok(ts) => ts.with_timezone(&Utc),
        Err(_) => {
            tracing::warn!(
                last_active_at = %last_active_at,
                "Invalid session last_active_at timestamp; forcing refresh"
            );
            return Ok(true);
        }
    };

    Ok((Utc::now() - last_active_at) > chrono::Duration::seconds(60))
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::time::Instant;

    use axum::{http::StatusCode, response::IntoResponse};
    use dashmap::DashMap;
    use uuid::Uuid;

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
            challenges: Arc::new(DashMap::new()),
            p2p_metadata: Arc::new(std::sync::RwLock::new(crate::p2p::P2pMetadata::default())),
            voice_runtime: Arc::new(crate::webrtc::voice_channel::VoiceRuntime::new(
                crate::config::VoiceConfig::default(),
            )),
        }
    }

    async fn insert_user(state: &AppState) -> String {
        let user_id = Uuid::new_v4().to_string();
        let created_at = Utc::now().to_rfc3339();
        let updated_at = created_at.clone();

        match &state.pool {
            crate::db::DbPool::Postgres(pool) => {
                sqlx::query("INSERT INTO users (id, did_key, public_key_multibase, username, avatar_color, created_at, updated_at)\nVALUES ($1, $2, $3, $4, $5, $6, $7)")
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
                sqlx::query("INSERT INTO users (id, did_key, public_key_multibase, username, avatar_color, created_at, updated_at)\nVALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)")
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

        user_id
    }

    async fn request_with_auth(token: &str) -> Parts {
        request_with_auth_value(&format!("Bearer {token}")).await
    }

    async fn request_with_auth_value(value: &str) -> Parts {
        use axum::http::{HeaderValue, Request};

        let mut req = Request::builder().uri("/").body(()).unwrap();
        req.headers_mut()
            .insert(AUTHORIZATION, HeaderValue::from_str(value).unwrap());
        let (parts, _body) = req.into_parts();
        parts
    }

    #[tokio::test]
    async fn valid_token_extracts_user() {
        let state = test_state().await;
        let user_id = insert_user(&state).await;
        let session = auth_service::create_session(&state.pool, &user_id, 168)
            .await
            .unwrap();

        let mut parts = request_with_auth(&session.token).await;
        let user = AuthenticatedUser::from_request_parts(&mut parts, &state)
            .await
            .unwrap();
        assert_eq!(user.user_id, user_id);
        assert_eq!(user.session_id, session.id);
        assert_eq!(user.username, "liam");
        assert_eq!(user.did_key, "did:key:z6Mk-test");
    }

    #[tokio::test]
    async fn lowercase_bearer_scheme_is_accepted() {
        let state = test_state().await;
        let user_id = insert_user(&state).await;
        let session = auth_service::create_session(&state.pool, &user_id, 168)
            .await
            .unwrap();

        let mut parts = request_with_auth_value(&format!("bearer {}", session.token)).await;
        let user = AuthenticatedUser::from_request_parts(&mut parts, &state)
            .await
            .unwrap();
        assert_eq!(user.user_id, user_id);
    }

    #[tokio::test]
    async fn missing_header_returns_401() {
        use axum::http::Request;

        let state = test_state().await;
        let (mut parts, _body) = Request::builder().uri("/").body(()).unwrap().into_parts();

        let err = AuthenticatedUser::from_request_parts(&mut parts, &state)
            .await
            .unwrap_err();
        let res = err.into_response();
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn malformed_header_returns_401() {
        use axum::http::{HeaderValue, Request};

        let state = test_state().await;
        let mut req = Request::builder().uri("/").body(()).unwrap();
        req.headers_mut()
            .insert(AUTHORIZATION, HeaderValue::from_static("Basic abc"));
        let (mut parts, _body) = req.into_parts();

        let err = AuthenticatedUser::from_request_parts(&mut parts, &state)
            .await
            .unwrap_err();
        let res = err.into_response();
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn expired_token_returns_401() {
        let state = test_state().await;
        let user_id = insert_user(&state).await;
        let session = auth_service::create_session(&state.pool, &user_id, 168)
            .await
            .unwrap();

        match &state.pool {
            crate::db::DbPool::Postgres(pool) => {
                sqlx::query("UPDATE sessions SET expires_at = $1 WHERE id = $2")
                    .bind("2000-01-01T00:00:00Z")
                    .bind(&session.id)
                    .execute(pool)
                    .await
                    .unwrap();
            }
            crate::db::DbPool::Sqlite(pool) => {
                sqlx::query("UPDATE sessions SET expires_at = ?1 WHERE id = ?2")
                    .bind("2000-01-01T00:00:00Z")
                    .bind(&session.id)
                    .execute(pool)
                    .await
                    .unwrap();
            }
        }

        let mut parts = request_with_auth(&session.token).await;
        let err = AuthenticatedUser::from_request_parts(&mut parts, &state)
            .await
            .unwrap_err();
        let res = err.into_response();
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    }

    #[test]
    fn invalid_last_active_at_forces_refresh() {
        assert_eq!(
            should_refresh_last_active_at("not-a-timestamp").unwrap(),
            true
        );
    }
}
