use axum::{
    Json,
    extract::State,
    extract::rejection::JsonRejection,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

use crate::{AppError, AppState, db::DbPool};

#[derive(Debug, Deserialize)]
pub struct SetupRequest {
    pub admin_username: Option<String>,
    pub avatar_color: Option<String>,
    pub instance_name: Option<String>,
    pub instance_description: Option<String>,
    pub discovery_enabled: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct InstanceStatus {
    pub initialized: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub discovery_enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub admin: Option<AdminInfo>,
}

#[derive(Debug, Serialize)]
pub struct AdminInfo {
    pub username: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar_color: Option<String>,
}

fn is_hex_color(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.len() != 7 || bytes[0] != b'#' {
        return false;
    }

    bytes[1..].iter().all(|b| b.is_ascii_hexdigit())
}

pub(super) async fn is_initialized(pool: &DbPool) -> Result<bool, AppError> {
    let row = match pool {
        DbPool::Postgres(pool) => {
            sqlx::query_scalar::<_, String>(
                "SELECT value FROM instance_settings WHERE key = 'initialized_at' LIMIT 1",
            )
            .fetch_optional(pool)
            .await
        }
        DbPool::Sqlite(pool) => {
            sqlx::query_scalar::<_, String>(
                "SELECT value FROM instance_settings WHERE key = 'initialized_at' LIMIT 1",
            )
            .fetch_optional(pool)
            .await
        }
    };

    row.map(|row| row.is_some())
        .map_err(|err| AppError::Internal(err.to_string()))
}

pub async fn get_instance(State(state): State<AppState>) -> Result<Response, AppError> {
    let initialized = is_initialized(&state.pool).await?;
    if !initialized {
        let status = InstanceStatus {
            initialized: false,
            name: None,
            description: None,
            discovery_enabled: None,
            admin: None,
        };
        return Ok((StatusCode::OK, Json(json!({ "data": status }))).into_response());
    }

    let rows: Vec<(String, String)> = match &state.pool {
        DbPool::Postgres(pool) => {
            sqlx::query_as(
                "SELECT key, value FROM instance_settings WHERE key IN ('instance_name','instance_description','discovery_enabled')",
            )
            .fetch_all(pool)
            .await
        }
        DbPool::Sqlite(pool) => {
            sqlx::query_as(
                "SELECT key, value FROM instance_settings WHERE key IN ('instance_name','instance_description','discovery_enabled')",
            )
            .fetch_all(pool)
            .await
        }
    }
    .map_err(|err| AppError::Internal(err.to_string()))?;

    let mut name: Option<String> = None;
    let mut description: Option<String> = None;
    let mut discovery_enabled: Option<bool> = None;

    for (key, value) in rows {
        match key.as_str() {
            "instance_name" => name = Some(value),
            "instance_description" => {
                if !value.is_empty() {
                    description = Some(value);
                }
            }
            "discovery_enabled" => match value.to_ascii_lowercase().as_str() {
                "true" => discovery_enabled = Some(true),
                "false" => discovery_enabled = Some(false),
                _ => {}
            },
            _ => {}
        }
    }

    let admin_row: Option<(String, Option<String>)> = match &state.pool {
        DbPool::Postgres(pool) => {
            sqlx::query_as("SELECT username, avatar_color FROM admin_users LIMIT 1")
                .fetch_optional(pool)
                .await
        }
        DbPool::Sqlite(pool) => {
            sqlx::query_as("SELECT username, avatar_color FROM admin_users LIMIT 1")
                .fetch_optional(pool)
                .await
        }
    }
    .map_err(|err| AppError::Internal(err.to_string()))?;
    let admin = admin_row.map(|(username, avatar_color)| AdminInfo {
        username,
        avatar_color,
    });

    let status = InstanceStatus {
        initialized: true,
        name,
        description,
        discovery_enabled,
        admin,
    };

    Ok((StatusCode::OK, Json(json!({ "data": status }))).into_response())
}

pub async fn setup_instance(
    State(state): State<AppState>,
    payload: Result<Json<SetupRequest>, JsonRejection>,
) -> Result<Response, AppError> {
    if is_initialized(&state.pool).await? {
        return Err(AppError::Conflict(
            "Instance has already been initialized".to_string(),
        ));
    }

    let Json(req) =
        payload.map_err(|_| AppError::ValidationError("Invalid request body".to_string()))?;

    let admin_username = req.admin_username.as_deref().unwrap_or("").trim();
    if admin_username.is_empty() {
        return Err(AppError::ValidationError(
            "admin_username is required".to_string(),
        ));
    }

    let instance_name = req.instance_name.as_deref().unwrap_or("").trim();
    if instance_name.is_empty() {
        return Err(AppError::ValidationError(
            "instance_name is required".to_string(),
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
    let instance_description_value = req
        .instance_description
        .as_deref()
        .unwrap_or("")
        .trim()
        .to_string();
    let discovery_enabled_value = req.discovery_enabled.unwrap_or(true);

    match &state.pool {
        DbPool::Postgres(pool) => {
            let mut tx = pool
                .begin()
                .await
                .map_err(|err| AppError::Internal(err.to_string()))?;

            // Race-safe guard: if another request initializes between our initial check and now,
            // this insert will be ignored and we can return 409.
            // Store a string timestamp to keep the value TEXT-compatible across SQLite + Postgres.
            let initialized_at = Utc::now().to_rfc3339();
            let initialized = sqlx::query(
                "INSERT INTO instance_settings (key, value) VALUES ($1, $2) ON CONFLICT(key) DO NOTHING",
            )
            .bind("initialized_at")
            .bind(&initialized_at)
            .execute(&mut *tx)
            .await
            .map_err(|err| AppError::Internal(err.to_string()))?
            .rows_affected()
                == 1;
            if !initialized {
                return Err(AppError::Conflict(
                    "Instance has already been initialized".to_string(),
                ));
            }

            let admin_id = Uuid::new_v4().to_string();
            // Use $n placeholders: valid in Postgres, and also accepted by SQLite.
            sqlx::query("INSERT INTO admin_users (id, username, avatar_color) VALUES ($1, $2, $3)")
                .bind(&admin_id)
                .bind(admin_username)
                .bind(avatar_color.as_deref())
                .execute(&mut *tx)
                .await
                .map_err(|err| AppError::Internal(err.to_string()))?;

            let discovery_enabled_str = if discovery_enabled_value {
                "true"
            } else {
                "false"
            };
            for (key, value) in [
                ("instance_name", instance_name),
                ("instance_description", instance_description_value.as_str()),
                ("discovery_enabled", discovery_enabled_str),
            ] {
                sqlx::query("INSERT INTO instance_settings (key, value) VALUES ($1, $2)")
                    .bind(key)
                    .bind(value)
                    .execute(&mut *tx)
                    .await
                    .map_err(|err| AppError::Internal(err.to_string()))?;
            }

            tx.commit()
                .await
                .map_err(|err| AppError::Internal(err.to_string()))?;
        }
        DbPool::Sqlite(pool) => {
            let mut tx = pool
                .begin()
                .await
                .map_err(|err| AppError::Internal(err.to_string()))?;

            // Race-safe guard: if another request initializes between our initial check and now,
            // this insert will be ignored and we can return 409.
            // Store a string timestamp to keep the value TEXT-compatible across SQLite + Postgres.
            let initialized_at = Utc::now().to_rfc3339();
            let initialized = sqlx::query(
                "INSERT INTO instance_settings (key, value) VALUES ($1, $2) ON CONFLICT(key) DO NOTHING",
            )
            .bind("initialized_at")
            .bind(&initialized_at)
            .execute(&mut *tx)
            .await
            .map_err(|err| AppError::Internal(err.to_string()))?
            .rows_affected()
                == 1;
            if !initialized {
                return Err(AppError::Conflict(
                    "Instance has already been initialized".to_string(),
                ));
            }

            let admin_id = Uuid::new_v4().to_string();
            // Use $n placeholders: valid in Postgres, and also accepted by SQLite.
            sqlx::query("INSERT INTO admin_users (id, username, avatar_color) VALUES ($1, $2, $3)")
                .bind(&admin_id)
                .bind(admin_username)
                .bind(avatar_color.as_deref())
                .execute(&mut *tx)
                .await
                .map_err(|err| AppError::Internal(err.to_string()))?;

            let discovery_enabled_str = if discovery_enabled_value {
                "true"
            } else {
                "false"
            };
            for (key, value) in [
                ("instance_name", instance_name),
                ("instance_description", instance_description_value.as_str()),
                ("discovery_enabled", discovery_enabled_str),
            ] {
                sqlx::query("INSERT INTO instance_settings (key, value) VALUES ($1, $2)")
                    .bind(key)
                    .bind(value)
                    .execute(&mut *tx)
                    .await
                    .map_err(|err| AppError::Internal(err.to_string()))?;
            }

            tx.commit()
                .await
                .map_err(|err| AppError::Internal(err.to_string()))?;
        }
    }

    tracing::info!(
        admin_username = %admin_username,
        instance_name = %instance_name,
        "Instance setup completed"
    );

    let status = InstanceStatus {
        initialized: true,
        name: Some(instance_name.to_string()),
        description: if instance_description_value.is_empty() {
            None
        } else {
            Some(instance_description_value)
        },
        discovery_enabled: Some(discovery_enabled_value),
        admin: Some(AdminInfo {
            username: admin_username.to_string(),
            avatar_color,
        }),
    };

    Ok((StatusCode::OK, Json(json!({ "data": status }))).into_response())
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

    #[tokio::test]
    async fn is_initialized_is_false_on_fresh_db() {
        let state = test_state().await;
        assert!(!is_initialized(&state.pool).await.unwrap());
    }

    #[tokio::test]
    async fn get_instance_returns_uninitialized_on_fresh_db() {
        let state = test_state().await;
        let res = get_instance(State(state)).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        let value = json_value(res).await;
        assert_eq!(value, json!({ "data": { "initialized": false } }));
    }

    #[tokio::test]
    async fn setup_instance_creates_admin_and_settings() {
        let state = test_state().await;
        let req = SetupRequest {
            admin_username: Some("tomas".to_string()),
            avatar_color: None,
            instance_name: Some("My Instance".to_string()),
            instance_description: Some("A cool place to hang out".to_string()),
            discovery_enabled: Some(false),
        };

        let res = setup_instance(State(state.clone()), Ok(Json(req)))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        let value = json_value(res).await;
        assert_eq!(
            value,
            json!({
                "data": {
                    "initialized": true,
                    "name": "My Instance",
                    "description": "A cool place to hang out",
                    "discovery_enabled": false,
                    "admin": {
                        "username": "tomas"
                    }
                }
            })
        );
    }

    #[tokio::test]
    async fn get_instance_returns_admin_and_settings_after_setup() {
        let state = test_state().await;
        let req = SetupRequest {
            admin_username: Some("tomas".to_string()),
            avatar_color: Some("#3399ff".to_string()),
            instance_name: Some("My Instance".to_string()),
            instance_description: Some("A cool place to hang out".to_string()),
            discovery_enabled: Some(true),
        };

        let _ = setup_instance(State(state.clone()), Ok(Json(req)))
            .await
            .unwrap();
        let res = get_instance(State(state)).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        let value = json_value(res).await;
        assert_eq!(
            value,
            json!({
                "data": {
                    "initialized": true,
                    "name": "My Instance",
                    "description": "A cool place to hang out",
                    "discovery_enabled": true,
                    "admin": {
                        "username": "tomas",
                        "avatar_color": "#3399ff"
                    }
                }
            })
        );
    }

    #[tokio::test]
    async fn setup_instance_returns_409_when_already_initialized() {
        let state = test_state().await;

        let req = SetupRequest {
            admin_username: Some("tomas".to_string()),
            avatar_color: None,
            instance_name: Some("My Instance".to_string()),
            instance_description: None,
            discovery_enabled: None,
        };
        let _ = setup_instance(State(state.clone()), Ok(Json(req)))
            .await
            .unwrap();

        let err = setup_instance(
            State(state),
            Ok(Json(SetupRequest {
                admin_username: Some("other".to_string()),
                avatar_color: None,
                instance_name: Some("Other Instance".to_string()),
                instance_description: None,
                discovery_enabled: None,
            })),
        )
        .await
        .unwrap_err();

        let res = err.into_response();
        assert_eq!(res.status(), StatusCode::CONFLICT);

        let value = json_value(res).await;
        assert_eq!(
            value,
            json!({ "error": { "code": "CONFLICT", "message": "Instance has already been initialized", "details": {} } })
        );
    }

    #[tokio::test]
    async fn setup_instance_returns_422_for_empty_username() {
        let state = test_state().await;
        let err = setup_instance(
            State(state),
            Ok(Json(SetupRequest {
                admin_username: Some("   ".to_string()),
                avatar_color: None,
                instance_name: Some("My Instance".to_string()),
                instance_description: None,
                discovery_enabled: None,
            })),
        )
        .await
        .unwrap_err();

        let res = err.into_response();
        assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);

        let value = json_value(res).await;
        assert_eq!(
            value,
            json!({ "error": { "code": "VALIDATION_ERROR", "message": "admin_username is required", "details": {} } })
        );
    }

    #[tokio::test]
    async fn setup_instance_returns_422_for_empty_instance_name() {
        let state = test_state().await;
        let err = setup_instance(
            State(state),
            Ok(Json(SetupRequest {
                admin_username: Some("tomas".to_string()),
                avatar_color: None,
                instance_name: Some("   ".to_string()),
                instance_description: None,
                discovery_enabled: None,
            })),
        )
        .await
        .unwrap_err();

        let res = err.into_response();
        assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);

        let value = json_value(res).await;
        assert_eq!(
            value,
            json!({ "error": { "code": "VALIDATION_ERROR", "message": "instance_name is required", "details": {} } })
        );
    }

    #[tokio::test]
    async fn setup_instance_defaults_discovery_enabled_to_true_when_omitted() {
        let state = test_state().await;
        let req = SetupRequest {
            admin_username: Some("tomas".to_string()),
            avatar_color: None,
            instance_name: Some("My Instance".to_string()),
            instance_description: None,
            discovery_enabled: None,
        };

        let res = setup_instance(State(state), Ok(Json(req))).await.unwrap();
        let value = json_value(res).await;
        assert_eq!(value["data"]["discovery_enabled"], json!(true));
    }

    #[tokio::test]
    async fn setup_instance_returns_422_for_invalid_avatar_color() {
        let state = test_state().await;
        let err = setup_instance(
            State(state),
            Ok(Json(SetupRequest {
                admin_username: Some("tomas".to_string()),
                avatar_color: Some("javascript:alert(1)".to_string()),
                instance_name: Some("My Instance".to_string()),
                instance_description: None,
                discovery_enabled: None,
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
