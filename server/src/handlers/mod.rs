use axum::{
    Router,
    body::Body,
    http::{
        Request,
        header::{HeaderName, HeaderValue},
    },
    middleware::{self, Next},
    response::Response,
    routing::get,
};

pub fn router() -> Router {
    let api = Router::new()
        .route("/ping", get(ping))
        .fallback(get(api_not_found));

    Router::new()
        .nest("/api/v1", api)
        .route("/ws", get(ws_not_found))
        .fallback(get(crate::static_files::handler))
        .layer(middleware::from_fn(security_headers))
}

async fn ping() -> axum::Json<serde_json::Value> {
    axum::Json(serde_json::json!({ "data": { "status": "ok" } }))
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
        HeaderValue::from_static(
            "default-src 'self'; base-uri 'self'; object-src 'none'; frame-ancestors 'none'; img-src 'self' data:; script-src 'self'; style-src 'self'; connect-src 'self' ws: wss:;",
        ),
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
