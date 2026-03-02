use axum::{
    Json,
    extract::rejection::JsonRejection,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Deserialize;
use serde_json::json;

use crate::{
    AppError, AppState,
    middleware::auth::AuthenticatedUser,
    services::moderation_service::{self, CreateMuteInput},
};

#[derive(Debug, Deserialize)]
pub struct CreateMuteRequest {
    pub target_user_id: Option<String>,
    pub reason: Option<String>,
    #[serde(default)]
    pub duration_seconds: Option<i64>,
    #[serde(default)]
    pub is_permanent: Option<bool>,
}

pub async fn create_mute(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(guild_slug): Path<String>,
    payload: Result<Json<CreateMuteRequest>, JsonRejection>,
) -> Result<Response, AppError> {
    let Json(req) =
        payload.map_err(|_| AppError::ValidationError("Invalid request body".to_string()))?;
    let input = to_create_mute_input(req)?;
    let created =
        moderation_service::create_mute(&state.pool, &user.user_id, &guild_slug, input).await?;
    Ok((StatusCode::CREATED, Json(json!({ "data": created }))).into_response())
}

pub async fn get_my_mute_status(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(guild_slug): Path<String>,
) -> Result<Response, AppError> {
    let status =
        moderation_service::get_my_mute_status(&state.pool, &user.user_id, &guild_slug).await?;
    Ok((StatusCode::OK, Json(json!({ "data": status }))).into_response())
}

fn to_create_mute_input(req: CreateMuteRequest) -> Result<CreateMuteInput, AppError> {
    let target_user_id = req.target_user_id.as_deref().unwrap_or("").trim();
    if target_user_id.is_empty() {
        return Err(AppError::ValidationError(
            "target_user_id is required".to_string(),
        ));
    }
    let reason = req.reason.as_deref().unwrap_or("").trim();
    if reason.is_empty() {
        return Err(AppError::ValidationError("reason is required".to_string()));
    }

    let is_permanent = req.is_permanent.unwrap_or(false);
    let duration_seconds = if is_permanent {
        None
    } else {
        Some(req.duration_seconds.ok_or_else(|| {
            AppError::ValidationError(
                "duration_seconds is required unless is_permanent is true".to_string(),
            )
        })?)
    };

    Ok(CreateMuteInput {
        target_user_id: target_user_id.to_string(),
        reason: reason.to_string(),
        duration_seconds,
    })
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::time::Instant;

    use axum::{body::to_bytes, extract::State};
    use dashmap::DashMap;
    use serde_json::Value;

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

    async fn seed_guild_fixture(state: &AppState) {
        let created_at = "2026-03-01T00:00:00Z";
        match &state.pool {
            crate::db::DbPool::Postgres(pool) => {
                sqlx::query("INSERT INTO users (id, did_key, public_key_multibase, username, avatar_color, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6, $7)")
                    .bind("owner-user-id")
                    .bind("did:key:owner")
                    .bind("pk-owner")
                    .bind("owner")
                    .bind("#99aab5")
                    .bind(created_at)
                    .bind(created_at)
                    .execute(pool)
                    .await
                    .unwrap();
                sqlx::query("INSERT INTO users (id, did_key, public_key_multibase, username, avatar_color, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6, $7)")
                    .bind("mod-user-id")
                    .bind("did:key:mod")
                    .bind("pk-mod")
                    .bind("mod")
                    .bind("#99aab5")
                    .bind(created_at)
                    .bind(created_at)
                    .execute(pool)
                    .await
                    .unwrap();
                sqlx::query("INSERT INTO users (id, did_key, public_key_multibase, username, avatar_color, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6, $7)")
                    .bind("target-user-id")
                    .bind("did:key:target")
                    .bind("pk-target")
                    .bind("target")
                    .bind("#99aab5")
                    .bind(created_at)
                    .bind(created_at)
                    .execute(pool)
                    .await
                    .unwrap();
                sqlx::query("INSERT INTO guilds (id, slug, name, description, owner_id, default_channel_slug, created_at, updated_at) VALUES ($1, $2, $3, NULL, $4, $5, $6, $7)")
                    .bind("guild-id")
                    .bind("test-guild")
                    .bind("Test Guild")
                    .bind("owner-user-id")
                    .bind("general")
                    .bind(created_at)
                    .bind(created_at)
                    .execute(pool)
                    .await
                    .unwrap();
                sqlx::query("INSERT INTO guild_members (guild_id, user_id, joined_at, joined_via_invite_code) VALUES ($1, $2, $3, NULL)")
                    .bind("guild-id")
                    .bind("mod-user-id")
                    .bind(created_at)
                    .execute(pool)
                    .await
                    .unwrap();
                sqlx::query("INSERT INTO guild_members (guild_id, user_id, joined_at, joined_via_invite_code) VALUES ($1, $2, $3, NULL)")
                    .bind("guild-id")
                    .bind("target-user-id")
                    .bind(created_at)
                    .execute(pool)
                    .await
                    .unwrap();
                sqlx::query("INSERT INTO roles (id, guild_id, name, color, position, permissions_bitflag, is_default, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)")
                    .bind("role-everyone")
                    .bind("guild-id")
                    .bind("@everyone")
                    .bind("#99aab5")
                    .bind(2_147_483_647_i64)
                    .bind(crate::permissions::default_everyone_permissions_i64())
                    .bind(1_i64)
                    .bind(created_at)
                    .bind(created_at)
                    .execute(pool)
                    .await
                    .unwrap();
                sqlx::query("INSERT INTO roles (id, guild_id, name, color, position, permissions_bitflag, is_default, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)")
                    .bind("role-mod")
                    .bind("guild-id")
                    .bind("Moderator")
                    .bind("#3366ff")
                    .bind(10_i64)
                    .bind(crate::permissions::MUTE_MEMBERS as i64)
                    .bind(0_i64)
                    .bind(created_at)
                    .bind(created_at)
                    .execute(pool)
                    .await
                    .unwrap();
                sqlx::query("INSERT INTO role_assignments (guild_id, user_id, role_id, assigned_at) VALUES ($1, $2, $3, $4)")
                    .bind("guild-id")
                    .bind("mod-user-id")
                    .bind("role-mod")
                    .bind(created_at)
                    .execute(pool)
                    .await
                    .unwrap();
            }
            crate::db::DbPool::Sqlite(pool) => {
                sqlx::query("INSERT INTO users (id, did_key, public_key_multibase, username, avatar_color, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)")
                    .bind("owner-user-id")
                    .bind("did:key:owner")
                    .bind("pk-owner")
                    .bind("owner")
                    .bind("#99aab5")
                    .bind(created_at)
                    .bind(created_at)
                    .execute(pool)
                    .await
                    .unwrap();
                sqlx::query("INSERT INTO users (id, did_key, public_key_multibase, username, avatar_color, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)")
                    .bind("mod-user-id")
                    .bind("did:key:mod")
                    .bind("pk-mod")
                    .bind("mod")
                    .bind("#99aab5")
                    .bind(created_at)
                    .bind(created_at)
                    .execute(pool)
                    .await
                    .unwrap();
                sqlx::query("INSERT INTO users (id, did_key, public_key_multibase, username, avatar_color, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)")
                    .bind("target-user-id")
                    .bind("did:key:target")
                    .bind("pk-target")
                    .bind("target")
                    .bind("#99aab5")
                    .bind(created_at)
                    .bind(created_at)
                    .execute(pool)
                    .await
                    .unwrap();
                sqlx::query("INSERT INTO guilds (id, slug, name, description, owner_id, default_channel_slug, created_at, updated_at) VALUES (?1, ?2, ?3, NULL, ?4, ?5, ?6, ?7)")
                    .bind("guild-id")
                    .bind("test-guild")
                    .bind("Test Guild")
                    .bind("owner-user-id")
                    .bind("general")
                    .bind(created_at)
                    .bind(created_at)
                    .execute(pool)
                    .await
                    .unwrap();
                sqlx::query("INSERT INTO guild_members (guild_id, user_id, joined_at, joined_via_invite_code) VALUES (?1, ?2, ?3, NULL)")
                    .bind("guild-id")
                    .bind("mod-user-id")
                    .bind(created_at)
                    .execute(pool)
                    .await
                    .unwrap();
                sqlx::query("INSERT INTO guild_members (guild_id, user_id, joined_at, joined_via_invite_code) VALUES (?1, ?2, ?3, NULL)")
                    .bind("guild-id")
                    .bind("target-user-id")
                    .bind(created_at)
                    .execute(pool)
                    .await
                    .unwrap();
                sqlx::query("INSERT INTO roles (id, guild_id, name, color, position, permissions_bitflag, is_default, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)")
                    .bind("role-everyone")
                    .bind("guild-id")
                    .bind("@everyone")
                    .bind("#99aab5")
                    .bind(2_147_483_647_i64)
                    .bind(crate::permissions::default_everyone_permissions_i64())
                    .bind(1_i64)
                    .bind(created_at)
                    .bind(created_at)
                    .execute(pool)
                    .await
                    .unwrap();
                sqlx::query("INSERT INTO roles (id, guild_id, name, color, position, permissions_bitflag, is_default, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)")
                    .bind("role-mod")
                    .bind("guild-id")
                    .bind("Moderator")
                    .bind("#3366ff")
                    .bind(10_i64)
                    .bind(crate::permissions::MUTE_MEMBERS as i64)
                    .bind(0_i64)
                    .bind(created_at)
                    .bind(created_at)
                    .execute(pool)
                    .await
                    .unwrap();
                sqlx::query("INSERT INTO role_assignments (guild_id, user_id, role_id, assigned_at) VALUES (?1, ?2, ?3, ?4)")
                    .bind("guild-id")
                    .bind("mod-user-id")
                    .bind("role-mod")
                    .bind(created_at)
                    .execute(pool)
                    .await
                    .unwrap();
            }
        }
    }

    fn mod_user() -> AuthenticatedUser {
        AuthenticatedUser {
            user_id: "mod-user-id".to_string(),
            session_id: "session-id".to_string(),
            username: "mod".to_string(),
            did_key: "did:key:mod".to_string(),
        }
    }

    #[test]
    fn to_create_mute_input_validates_required_fields() {
        let missing_target = to_create_mute_input(CreateMuteRequest {
            target_user_id: None,
            reason: Some("reason".to_string()),
            duration_seconds: Some(3600),
            is_permanent: Some(false),
        });
        assert!(matches!(missing_target, Err(AppError::ValidationError(_))));

        let missing_reason = to_create_mute_input(CreateMuteRequest {
            target_user_id: Some("target".to_string()),
            reason: Some("   ".to_string()),
            duration_seconds: Some(3600),
            is_permanent: Some(false),
        });
        assert!(matches!(missing_reason, Err(AppError::ValidationError(_))));

        let missing_duration = to_create_mute_input(CreateMuteRequest {
            target_user_id: Some("target".to_string()),
            reason: Some("reason".to_string()),
            duration_seconds: None,
            is_permanent: Some(false),
        });
        assert!(matches!(
            missing_duration,
            Err(AppError::ValidationError(_))
        ));
    }

    #[tokio::test]
    async fn create_mute_returns_data_envelope() {
        let state = test_state().await;
        seed_guild_fixture(&state).await;
        let response = create_mute(
            State(state),
            mod_user(),
            Path("test-guild".to_string()),
            Ok(Json(CreateMuteRequest {
                target_user_id: Some("target-user-id".to_string()),
                reason: Some("cooldown".to_string()),
                duration_seconds: Some(3600),
                is_permanent: Some(false),
            })),
        )
        .await
        .unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);

        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let payload: Value = serde_json::from_slice(&body).unwrap();
        assert!(payload.get("data").is_some());
        assert_eq!(
            payload["data"]["target_user_id"],
            Value::String("target-user-id".to_string())
        );
        assert_eq!(
            payload["data"]["duration_seconds"],
            Value::Number(3600.into())
        );
    }
}
