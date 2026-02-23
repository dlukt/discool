use std::sync::OnceLock;
use std::time::Instant;

use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use axum_prometheus::metrics::{describe_gauge, gauge};
use serde_json::json;

use crate::{AppState, db::DbPool};

static CUSTOM_METRICS_REGISTERED: OnceLock<()> = OnceLock::new();

pub async fn healthz() -> StatusCode {
    StatusCode::OK
}

pub async fn readyz(State(state): State<AppState>) -> impl IntoResponse {
    let res = match &state.pool {
        DbPool::Postgres(pool) => {
            sqlx::query_scalar::<_, String>(
                "SELECT value FROM schema_metadata WHERE key = 'initialized_at' LIMIT 1",
            )
            .fetch_optional(pool)
            .await
        }
        DbPool::Sqlite(pool) => {
            sqlx::query_scalar::<_, String>(
                "SELECT value FROM schema_metadata WHERE key = 'initialized_at' LIMIT 1",
            )
            .fetch_optional(pool)
            .await
        }
    };

    match res {
        Ok(Some(_)) => (
            StatusCode::OK,
            Json(json!({
                "status": "ready",
                "checks": {
                    "database": "connected",
                    "migrations": "applied"
                }
            })),
        ),
        Ok(None) => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({
                "status": "not_ready",
                "checks": {
                    "database": "connected",
                    "migrations": "pending"
                }
            })),
        ),
        Err(err) if looks_like_missing_schema(&err) => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({
                "status": "not_ready",
                "checks": {
                    "database": "connected",
                    "migrations": "pending"
                }
            })),
        ),
        Err(_) => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({
                "status": "not_ready",
                "checks": {
                    "database": "unavailable"
                }
            })),
        ),
    }
}

pub fn register_custom_metrics() {
    CUSTOM_METRICS_REGISTERED.get_or_init(|| {
        describe_gauge!("discool_info", "Discool server information");
        describe_gauge!("discool_uptime_seconds", "Discool server uptime in seconds");
        describe_gauge!(
            "discool_db_pool_connections",
            "Discool database pool connection counts"
        );

        gauge!("discool_info", "version" => env!("CARGO_PKG_VERSION")).set(1.0);
    });
}

pub fn update_custom_metrics(pool: &DbPool, start_time: Instant, db_max_connections: u32) {
    gauge!("discool_uptime_seconds").set(start_time.elapsed().as_secs_f64());

    let idle = pool.num_idle() as f64;
    let total = pool.size() as f64;
    let active = (total - idle).max(0.0);

    gauge!("discool_db_pool_connections", "state" => "active").set(active);
    gauge!("discool_db_pool_connections", "state" => "idle").set(idle);
    gauge!("discool_db_pool_connections", "state" => "max").set(db_max_connections as f64);
}

fn looks_like_missing_schema(err: &sqlx::Error) -> bool {
    let sqlx::Error::Database(db_err) = err else {
        return false;
    };

    let msg = db_err.message().to_ascii_lowercase();
    msg.contains("schema_metadata")
        && (msg.contains("no such table")
            || msg.contains("does not exist")
            || msg.contains("unknown table"))
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use axum::{body::to_bytes, extract::State, response::IntoResponse};
    use dashmap::DashMap;

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

    async fn test_state_without_migrations() -> AppState {
        let mut cfg = crate::config::Config::default();
        cfg.database = Some(crate::config::DatabaseConfig {
            url: "sqlite::memory:".to_string(),
            max_connections: 1,
        });

        let pool = crate::db::init_pool(cfg.database.as_ref().unwrap())
            .await
            .unwrap();

        AppState {
            config: Arc::new(cfg),
            pool,
            start_time: Instant::now(),
            challenges: Arc::new(DashMap::new()),
        }
    }

    #[tokio::test]
    async fn healthz_returns_200() {
        assert_eq!(healthz().await, StatusCode::OK);
    }

    #[tokio::test]
    async fn readyz_returns_200_when_pool_is_healthy() {
        let state = test_state().await;
        let res = readyz(State(state)).await.into_response();
        assert_eq!(res.status(), StatusCode::OK);

        let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
        let value: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(
            value,
            json!({
                "status": "ready",
                "checks": {
                    "database": "connected",
                    "migrations": "applied"
                }
            })
        );
    }

    #[tokio::test]
    async fn readyz_returns_503_when_migrations_pending_no_row() {
        let state = test_state().await;
        match &state.pool {
            DbPool::Postgres(pool) => {
                sqlx::query("DELETE FROM schema_metadata WHERE key = 'initialized_at'")
                    .execute(pool)
                    .await
                    .unwrap();
            }
            DbPool::Sqlite(pool) => {
                sqlx::query("DELETE FROM schema_metadata WHERE key = 'initialized_at'")
                    .execute(pool)
                    .await
                    .unwrap();
            }
        }

        let res = readyz(State(state)).await.into_response();
        assert_eq!(res.status(), StatusCode::SERVICE_UNAVAILABLE);

        let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
        let value: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(
            value,
            json!({
                "status": "not_ready",
                "checks": {
                    "database": "connected",
                    "migrations": "pending"
                }
            })
        );
    }

    #[tokio::test]
    async fn readyz_returns_503_when_schema_metadata_table_missing() {
        let state = test_state_without_migrations().await;
        let res = readyz(State(state)).await.into_response();
        assert_eq!(res.status(), StatusCode::SERVICE_UNAVAILABLE);

        let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
        let value: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(
            value,
            json!({
                "status": "not_ready",
                "checks": {
                    "database": "connected",
                    "migrations": "pending"
                }
            })
        );
    }

    #[tokio::test]
    async fn readyz_returns_503_when_pool_query_fails() {
        let state = test_state().await;
        state.pool.close().await;

        let res = readyz(State(state)).await.into_response();
        assert_eq!(res.status(), StatusCode::SERVICE_UNAVAILABLE);

        let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
        let value: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(
            value,
            json!({
                "status": "not_ready",
                "checks": {
                    "database": "unavailable"
                }
            })
        );
    }
}
