use axum::{
    Json,
    body::Body,
    extract::rejection::JsonRejection,
    extract::{Multipart, State},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};
use serde::Deserialize;
use serde_json::json;

use crate::{
    AppError, AppState,
    middleware::auth::AuthenticatedUser,
    services::user_profile_service::{self, UpdateProfileInput},
};

#[derive(Debug, Deserialize)]
pub struct UpdateProfileRequest {
    #[serde(default)]
    pub display_name: Option<Option<String>>,
    #[serde(default)]
    pub avatar_color: Option<Option<String>>,
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

    use axum::{Json, body::to_bytes, extract::State};
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
        }
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
}
