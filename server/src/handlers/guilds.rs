use axum::{
    Json,
    body::Body,
    extract::rejection::JsonRejection,
    extract::{Multipart, Path, State},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};
use serde::Deserialize;
use serde_json::json;

use crate::{
    AppError, AppState,
    middleware::auth::AuthenticatedUser,
    services::guild_service::{self, CreateGuildInput, UpdateGuildInput},
};

#[derive(Debug, Deserialize)]
pub struct CreateGuildRequest {
    pub name: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateGuildRequest {
    #[serde(default)]
    pub name: Option<Option<String>>,
    #[serde(default)]
    pub description: Option<Option<String>>,
}

pub async fn list_guilds(
    State(state): State<AppState>,
    user: AuthenticatedUser,
) -> Result<Response, AppError> {
    let guilds = guild_service::list_guilds(&state.pool, &user.user_id).await?;
    Ok((StatusCode::OK, Json(json!({ "data": guilds }))).into_response())
}

pub async fn create_guild(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    payload: Result<Json<CreateGuildRequest>, JsonRejection>,
) -> Result<Response, AppError> {
    let Json(req) =
        payload.map_err(|_| AppError::ValidationError("Invalid request body".to_string()))?;
    let guild = guild_service::create_guild(
        &state.pool,
        &user.user_id,
        CreateGuildInput {
            name: req.name.unwrap_or_default(),
            description: req.description,
        },
    )
    .await?;
    Ok((StatusCode::CREATED, Json(json!({ "data": guild }))).into_response())
}

pub async fn update_guild(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(guild_slug): Path<String>,
    payload: Result<Json<UpdateGuildRequest>, JsonRejection>,
) -> Result<Response, AppError> {
    let Json(req) =
        payload.map_err(|_| AppError::ValidationError("Invalid request body".to_string()))?;
    let guild = guild_service::update_guild(
        &state.pool,
        &user.user_id,
        &guild_slug,
        UpdateGuildInput {
            name: req.name,
            description: req.description,
        },
    )
    .await?;
    Ok((StatusCode::OK, Json(json!({ "data": guild }))).into_response())
}

pub async fn delete_guild(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(guild_slug): Path<String>,
) -> Result<Response, AppError> {
    guild_service::delete_guild(&state.pool, &state.config.avatar, &user.user_id, &guild_slug)
        .await?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

pub async fn upload_guild_icon(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(guild_slug): Path<String>,
    mut multipart: Multipart,
) -> Result<Response, AppError> {
    let mut icon_bytes: Option<Vec<u8>> = None;
    let mut icon_content_type: Option<String> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|_| AppError::ValidationError("Invalid multipart payload".to_string()))?
    {
        if field.name() == Some("icon") {
            icon_content_type = field.content_type().map(str::to_string);
            let bytes = field
                .bytes()
                .await
                .map_err(|_| AppError::ValidationError("Invalid icon payload".to_string()))?;
            icon_bytes = Some(bytes.to_vec());
            break;
        }
    }

    let icon_bytes = icon_bytes
        .ok_or_else(|| AppError::ValidationError("icon field is required".to_string()))?;
    let guild = guild_service::save_guild_icon(
        &state.pool,
        &state.config.avatar,
        &user.user_id,
        &guild_slug,
        icon_content_type.as_deref(),
        &icon_bytes,
    )
    .await?;
    Ok((StatusCode::OK, Json(json!({ "data": guild }))).into_response())
}

pub async fn get_guild_icon(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(guild_slug): Path<String>,
) -> Result<Response, AppError> {
    let (bytes, mime) = guild_service::load_guild_icon(
        &state.pool,
        &state.config.avatar,
        &user.user_id,
        &guild_slug,
    )
    .await?;

    let mut response = Response::new(Body::from(bytes));
    *response.status_mut() = StatusCode::OK;
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        mime.parse()
            .map_err(|_| AppError::Internal("Invalid guild icon MIME type".to_string()))?,
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

    use dashmap::DashMap;
    use uuid::Uuid;

    use crate::db::DbPool;

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
            p2p_metadata: Arc::new(std::sync::RwLock::new(
                crate::p2p::P2pMetadata::default(),
            )),
            voice_runtime: Arc::new(crate::webrtc::voice_channel::VoiceRuntime::new(
                crate::config::VoiceConfig::default(),
            )),
        }
    }

    async fn insert_user(state: &AppState, username: &str) -> AuthenticatedUser {
        let user_id = Uuid::new_v4().to_string();
        let did_key = format!("did:key:{}", Uuid::new_v4());
        let now = chrono::Utc::now().to_rfc3339();
        match &state.pool {
            DbPool::Postgres(pool) => {
                sqlx::query(
                    "INSERT INTO users (id, did_key, public_key_multibase, username, avatar_color, created_at, updated_at) VALUES ($1, $2, $3, $4, NULL, $5, $6)",
                )
                .bind(&user_id)
                .bind(&did_key)
                .bind("z6Mk-test")
                .bind(username)
                .bind(&now)
                .bind(&now)
                .execute(pool)
                .await
                .unwrap();
            }
            DbPool::Sqlite(pool) => {
                sqlx::query(
                    "INSERT INTO users (id, did_key, public_key_multibase, username, avatar_color, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, NULL, ?5, ?6)",
                )
                .bind(&user_id)
                .bind(&did_key)
                .bind("z6Mk-test")
                .bind(username)
                .bind(&now)
                .bind(&now)
                .execute(pool)
                .await
                .unwrap();
            }
        }
        AuthenticatedUser {
            user_id,
            session_id: "session-id".to_string(),
            username: username.to_string(),
            did_key,
        }
    }

    async fn insert_guild(state: &AppState, owner_id: &str, slug: &str, name: &str) -> String {
        let guild_id = Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();
        match &state.pool {
            DbPool::Postgres(pool) => {
                sqlx::query(
                    "INSERT INTO guilds (id, slug, name, owner_id, default_channel_slug, created_at, updated_at) VALUES ($1, $2, $3, $4, 'general', $5, $6)",
                )
                .bind(&guild_id)
                .bind(slug)
                .bind(name)
                .bind(owner_id)
                .bind(&now)
                .bind(&now)
                .execute(pool)
                .await
                .unwrap();
            }
            DbPool::Sqlite(pool) => {
                sqlx::query(
                    "INSERT INTO guilds (id, slug, name, owner_id, default_channel_slug, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, 'general', ?5, ?6)",
                )
                .bind(&guild_id)
                .bind(slug)
                .bind(name)
                .bind(owner_id)
                .bind(&now)
                .bind(&now)
                .execute(pool)
                .await
                .unwrap();
            }
        }
        guild_id
    }

    async fn insert_channel(state: &AppState, guild_id: &str, slug: &str) {
        let channel_id = Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();
        match &state.pool {
            DbPool::Postgres(pool) => {
                sqlx::query(
                    "INSERT INTO channels (id, guild_id, slug, name, channel_type, position, created_at, updated_at) VALUES ($1, $2, $3, $3, 'text', 0, $4, $5)",
                )
                .bind(&channel_id)
                .bind(guild_id)
                .bind(slug)
                .bind(&now)
                .bind(&now)
                .execute(pool)
                .await
                .unwrap();
            }
            DbPool::Sqlite(pool) => {
                sqlx::query(
                    "INSERT INTO channels (id, guild_id, slug, name, channel_type, position, created_at, updated_at) VALUES (?1, ?2, ?3, ?3, 'text', 0, ?4, ?5)",
                )
                .bind(&channel_id)
                .bind(guild_id)
                .bind(slug)
                .bind(&now)
                .bind(&now)
                .execute(pool)
                .await
                .unwrap();
            }
        }
    }

    async fn count_guilds_by_id(state: &AppState, guild_id: &str) -> i64 {
        match &state.pool {
            DbPool::Postgres(pool) => {
                sqlx::query_scalar("SELECT COUNT(*) FROM guilds WHERE id = $1")
                    .bind(guild_id)
                    .fetch_one(pool)
                    .await
                    .unwrap()
            }
            DbPool::Sqlite(pool) => {
                sqlx::query_scalar("SELECT COUNT(*) FROM guilds WHERE id = ?1")
                    .bind(guild_id)
                    .fetch_one(pool)
                    .await
                    .unwrap()
            }
        }
    }

    async fn count_channels_by_guild(state: &AppState, guild_id: &str) -> i64 {
        match &state.pool {
            DbPool::Postgres(pool) => {
                sqlx::query_scalar("SELECT COUNT(*) FROM channels WHERE guild_id = $1")
                    .bind(guild_id)
                    .fetch_one(pool)
                    .await
                    .unwrap()
            }
            DbPool::Sqlite(pool) => {
                sqlx::query_scalar("SELECT COUNT(*) FROM channels WHERE guild_id = ?1")
                    .bind(guild_id)
                    .fetch_one(pool)
                    .await
                    .unwrap()
            }
        }
    }

    #[tokio::test]
    async fn delete_guild_returns_204_and_cascades_to_channels() {
        let state = test_state().await;
        let owner = insert_user(&state, "owner").await;
        let guild_id = insert_guild(&state, &owner.user_id, "my-guild", "My Guild").await;
        insert_channel(&state, &guild_id, "general").await;
        assert_eq!(
            count_channels_by_guild(&state, &guild_id).await,
            1
        );

        let res = delete_guild(
            State(state.clone()),
            owner.clone(),
            Path("my-guild".to_string()),
        )
        .await
        .unwrap();
        assert_eq!(res.status(), StatusCode::NO_CONTENT);

        assert_eq!(count_guilds_by_id(&state, &guild_id).await, 0);
        assert_eq!(count_channels_by_guild(&state, &guild_id).await, 0);
    }

    #[tokio::test]
    async fn delete_guild_returns_403_for_non_owner() {
        let state = test_state().await;
        let owner = insert_user(&state, "owner").await;
        let intruder = insert_user(&state, "intruder").await;
        let guild_id = insert_guild(&state, &owner.user_id, "owner-guild", "Owner Guild").await;

        let err = delete_guild(
            State(state.clone()),
            intruder,
            Path("owner-guild".to_string()),
        )
        .await
        .unwrap_err();
        assert_eq!(err.into_response().status(), StatusCode::FORBIDDEN);
        assert_eq!(count_guilds_by_id(&state, &guild_id).await, 1);
    }

    #[tokio::test]
    async fn delete_guild_returns_404_for_unknown_slug() {
        let state = test_state().await;
        let owner = insert_user(&state, "owner").await;

        let err = delete_guild(State(state), owner, Path("does-not-exist".to_string()))
            .await
            .unwrap_err();
        assert_eq!(err.into_response().status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn update_guild_changes_slug_when_renamed() {
        let state = test_state().await;
        let owner = insert_user(&state, "owner").await;
        insert_guild(&state, &owner.user_id, "my-guild", "My Guild").await;

        let updated = guild_service::update_guild(
            &state.pool,
            &owner.user_id,
            "my-guild",
            UpdateGuildInput {
                name: Some(Some("Renamed Guild".to_string())),
                description: None,
            },
        )
        .await
        .unwrap();

        assert_eq!(updated.slug, "renamed-guild");
        assert!(
            crate::models::guild::find_guild_by_slug(&state.pool, "my-guild")
                .await
                .unwrap()
                .is_none()
        );
        assert!(
            crate::models::guild::find_guild_by_slug(&state.pool, "renamed-guild")
                .await
                .unwrap()
                .is_some()
        );
    }

    #[tokio::test]
    async fn update_guild_dedups_slug_on_collision() {
        let state = test_state().await;
        let owner = insert_user(&state, "owner").await;
        insert_guild(&state, &owner.user_id, "lobby", "Lobby").await;
        insert_guild(&state, &owner.user_id, "lobby-2", "Lobby Two").await;

        // Rename lobby-2 to "Lobby": base slug "lobby" is taken by the other
        // guild, so it falls back to its own "lobby-2".
        let updated = guild_service::update_guild(
            &state.pool,
            &owner.user_id,
            "lobby-2",
            UpdateGuildInput {
                name: Some(Some("Lobby".to_string())),
                description: None,
            },
        )
        .await
        .unwrap();
        assert_eq!(updated.name, "Lobby");
        assert_eq!(updated.slug, "lobby-2");
    }

    #[tokio::test]
    async fn update_guild_keeps_slug_when_only_description_changes() {
        let state = test_state().await;
        let owner = insert_user(&state, "owner").await;
        insert_guild(&state, &owner.user_id, "my-guild", "My Guild").await;

        let updated = guild_service::update_guild(
            &state.pool,
            &owner.user_id,
            "my-guild",
            UpdateGuildInput {
                name: None,
                description: Some(Some("new description".to_string())),
            },
        )
        .await
        .unwrap();
        assert_eq!(updated.slug, "my-guild");
        assert_eq!(updated.description.as_deref(), Some("new description"));
    }
}
