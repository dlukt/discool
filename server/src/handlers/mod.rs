use axum::{
    Router,
    body::Body,
    http::{
        Request, StatusCode,
        header::{HeaderName, HeaderValue},
    },
    middleware::{self, Next},
    response::Response,
    routing::{delete, get, patch, post},
};
use axum_prometheus::PrometheusMetricLayer;

use crate::AppState;

mod admin;
mod auth;
mod categories;
mod channels;
mod guilds;
mod health;
mod instance;
mod users;

// Keep `connect-src` strict until we have a safe, non-Host-header-derived allow-list for WebSockets.
// Keep `style-src` strict (no 'unsafe-inline') to reduce XSS blast radius.
const DEFAULT_CSP: &str = "default-src 'self'; base-uri 'self'; object-src 'none'; frame-ancestors 'none'; img-src 'self' data:; script-src 'self'; style-src 'self'; connect-src 'self';";

pub fn router(state: AppState) -> Router {
    let api = Router::new()
        .route("/ping", get(ping))
        .route("/auth/register", post(auth::register))
        .route("/auth/challenge", post(auth::challenge))
        .route("/auth/verify", post(auth::verify))
        .route(
            "/auth/recovery-email/start",
            post(auth::start_identity_recovery),
        )
        .route(
            "/auth/recovery-email/verify",
            get(auth::verify_recovery_email),
        )
        .route("/auth/recovery-email/recover", get(auth::recover_identity))
        .route("/auth/logout", delete(auth::logout))
        .route(
            "/guilds",
            get(guilds::list_guilds).post(guilds::create_guild),
        )
        .route(
            "/guilds/{guild_slug}/channels",
            get(channels::list_channels).post(channels::create_channel),
        )
        .route(
            "/guilds/{guild_slug}/categories",
            get(categories::list_categories).post(categories::create_category),
        )
        .route(
            "/guilds/{guild_slug}/categories/reorder",
            patch(categories::reorder_categories),
        )
        .route(
            "/guilds/{guild_slug}/categories/{category_slug}",
            patch(categories::update_category).delete(categories::delete_category),
        )
        .route(
            "/guilds/{guild_slug}/categories/{category_slug}/collapse",
            patch(categories::update_category_collapse),
        )
        .route(
            "/guilds/{guild_slug}/channels/reorder",
            patch(channels::reorder_channels),
        )
        .route(
            "/guilds/{guild_slug}/channels/{channel_slug}",
            patch(channels::update_channel).delete(channels::delete_channel),
        )
        .route("/guilds/{guild_slug}", patch(guilds::update_guild))
        .route(
            "/guilds/{guild_slug}/icon",
            get(guilds::get_guild_icon).post(guilds::upload_guild_icon),
        )
        .route(
            "/users/me/profile",
            get(users::get_profile).patch(users::update_profile),
        )
        .route(
            "/users/me/recovery-email",
            get(users::get_recovery_email).post(users::start_recovery_email),
        )
        .route(
            "/users/me/avatar",
            get(users::get_avatar).post(users::upload_avatar),
        )
        .route("/admin/health", get(admin::get_health))
        .route("/admin/backup", post(admin::create_backup))
        .route("/instance", get(instance::get_instance))
        .route("/instance/setup", post(instance::setup_instance))
        .fallback(get(api_not_found));

    let mut tracked = Router::new()
        .nest("/api/v1", api)
        .route("/ws", get(ws_not_found))
        .fallback(get(crate::static_files::handler));

    // /metrics should not be tracked (it is scraped frequently like /healthz and /readyz).
    let mut metrics = Router::new().route("/metrics", get(metrics_not_found));

    if state.config.metrics_enabled() {
        let start_time = state.start_time;
        let pool = state.pool.clone();
        let db_max_connections = state
            .config
            .database
            .as_ref()
            .map(|db| db.max_connections)
            .unwrap_or(pool.size());

        let (prometheus_layer, metric_handle) = PrometheusMetricLayer::pair();
        health::register_custom_metrics();

        tracked = tracked.layer(prometheus_layer);
        metrics = Router::new().route(
            "/metrics",
            get(move || {
                let pool = pool.clone();
                async move {
                    health::update_custom_metrics(&pool, start_time, db_max_connections);
                    metric_handle.render()
                }
            }),
        );
    }

    Router::new()
        .route("/healthz", get(health::healthz))
        .route("/readyz", get(health::readyz))
        .merge(metrics)
        .merge(tracked)
        .layer(middleware::from_fn(security_headers))
        .with_state(state)
}

async fn ping() -> axum::Json<serde_json::Value> {
    axum::Json(serde_json::json!({ "data": { "status": "ok" } }))
}

async fn metrics_not_found() -> StatusCode {
    StatusCode::NOT_FOUND
}

async fn security_headers(req: Request<Body>, next: Next) -> Response {
    let mut res = next.run(req).await;
    let headers = res.headers_mut();

    headers.insert(
        HeaderName::from_static("x-content-type-options"),
        HeaderValue::from_static("nosniff"),
    );
    headers.insert(
        HeaderName::from_static("x-frame-options"),
        HeaderValue::from_static("DENY"),
    );
    headers.insert(
        HeaderName::from_static("referrer-policy"),
        HeaderValue::from_static("same-origin"),
    );
    headers.insert(
        HeaderName::from_static("content-security-policy"),
        HeaderValue::from_static(DEFAULT_CSP),
    );

    res
}

async fn api_not_found() -> crate::AppError {
    crate::AppError::NotFound
}

async fn ws_not_found() -> crate::AppError {
    crate::AppError::NotFound
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn ping_returns_ok() {
        let axum::Json(value) = ping().await;
        assert_eq!(value, serde_json::json!({ "data": { "status": "ok" } }));
    }
}
