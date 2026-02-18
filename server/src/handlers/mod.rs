use axum::{
    Router,
    body::Body,
    http::{
        Request, StatusCode,
        header::{HeaderName, HeaderValue},
    },
    middleware::{self, Next},
    response::Response,
    routing::get,
};

use crate::AppState;

// Keep `connect-src` strict until we have a safe, non-Host-header-derived allow-list for WebSockets.
const DEFAULT_CSP: &str = "default-src 'self'; base-uri 'self'; object-src 'none'; frame-ancestors 'none'; img-src 'self' data:; script-src 'self'; style-src 'self'; connect-src 'self';";

pub fn router(state: AppState) -> Router {
    let api = Router::new()
        .route("/ping", get(ping))
        .fallback(get(api_not_found));

    Router::new()
        .nest("/api/v1", api)
        .route("/healthz", get(healthz))
        .route("/ws", get(ws_not_found))
        .fallback(get(crate::static_files::handler))
        .layer(middleware::from_fn(security_headers))
        .with_state(state)
}

async fn ping() -> axum::Json<serde_json::Value> {
    axum::Json(serde_json::json!({ "data": { "status": "ok" } }))
}

async fn healthz() -> StatusCode {
    StatusCode::OK
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
