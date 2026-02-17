use axum::{
    body::Body,
    http::{HeaderValue, StatusCode, Uri, header},
    response::{IntoResponse, Response},
};
use rust_embed::{EmbeddedFile, RustEmbed};

use crate::AppError;

#[derive(RustEmbed)]
#[folder = "../client/dist/"]
struct ClientDist;

const MISSING_CLIENT_DIST_HTML: &str = r#"<!doctype html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>Discool</title>
  </head>
  <body>
    <h1>Discool</h1>
    <p>Client assets not found.</p>
    <p>Run <code>cd client &amp;&amp; npm run build</code> and then rebuild the server.</p>
  </body>
</html>
"#;

pub async fn handler(uri: Uri) -> Response {
    let path = match uri.path().strip_prefix('/') {
        Some("") | None => "index.html",
        Some(path) if path.ends_with('/') => "index.html",
        Some(path) => path,
    };

    if let Some(file) = ClientDist::get(path) {
        return file_response(path, file);
    }

    // A request that looks like a file path (has an extension) should 404 if missing.
    if path
        .rsplit('/')
        .next()
        .is_some_and(|name| name.contains('.'))
    {
        return AppError::NotFound.into_response();
    }

    match ClientDist::get("index.html") {
        Some(index) => file_response("index.html", index),
        None => fallback_index_response(),
    }
}

fn file_response(path: &str, file: EmbeddedFile) -> Response {
    let mut response = Response::new(Body::from(file.data.into_owned()));
    *response.status_mut() = StatusCode::OK;

    let headers = response.headers_mut();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static(content_type(path)),
    );
    headers.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static(cache_control(path)),
    );

    response
}

fn cache_control(path: &str) -> &'static str {
    if path == "index.html" {
        "no-cache"
    } else if is_hashed_asset(path) {
        "public, max-age=31536000, immutable"
    } else {
        "public, max-age=3600"
    }
}

fn is_hashed_asset(path: &str) -> bool {
    let file_name = path.rsplit('/').next().unwrap_or(path);
    let Some((_, tail)) = file_name.rsplit_once('-') else {
        return false;
    };

    let hash = tail.split('.').next().unwrap_or("");
    hash.len() >= 8 && hash.chars().all(|c| c.is_ascii_alphanumeric())
}

fn content_type(path: &str) -> &'static str {
    let ext = path.rsplit('.').next().unwrap_or("");
    match ext {
        "html" => "text/html; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "js" | "mjs" => "application/javascript; charset=utf-8",
        "json" => "application/json",
        "map" => "application/json",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "ico" => "image/x-icon",
        "woff" => "font/woff",
        "woff2" => "font/woff2",
        "wasm" => "application/wasm",
        _ => "application/octet-stream",
    }
}

fn fallback_index_response() -> Response {
    let mut response = Response::new(Body::from(MISSING_CLIENT_DIST_HTML));
    *response.status_mut() = StatusCode::OK;

    let headers = response.headers_mut();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/html; charset=utf-8"),
    );
    headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-cache"));

    response
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::{StatusCode, header};

    #[tokio::test]
    async fn serves_index_for_root_and_spa_routes() {
        for uri in [Uri::from_static("/"), Uri::from_static("/guild/channel")] {
            let res = handler(uri).await;
            assert_eq!(res.status(), StatusCode::OK);
            assert_eq!(
                res.headers().get(header::CONTENT_TYPE).unwrap(),
                "text/html; charset=utf-8"
            );
            assert_eq!(
                res.headers().get(header::CACHE_CONTROL).unwrap(),
                "no-cache"
            );
        }
    }

    #[tokio::test]
    async fn missing_asset_returns_404() {
        let res = handler(Uri::from_static("/assets/does-not-exist.js")).await;
        assert_eq!(res.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn cache_control_sets_immutable_for_hashed_assets() {
        assert_eq!(
            cache_control("assets/index-abcdef12.js"),
            "public, max-age=31536000, immutable"
        );
    }
}
