use axum::{
    body::Body,
    extract::State,
    http::{HeaderValue, StatusCode, Uri, header},
    response::{IntoResponse, Response},
};
use rust_embed::{EmbeddedFile, RustEmbed};

use crate::{AppError, AppState, services::guild_invite_service};

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
    let path = normalize_path(&uri);
    serve_path(path)
}

pub async fn handler_with_state(State(state): State<AppState>, uri: Uri) -> Response {
    let path = normalize_path(&uri);
    if let Some(invite_code) = invite_code_from_path(path)
        && let Ok(metadata) =
            guild_invite_service::resolve_invite_metadata(&state.pool, invite_code).await
    {
        return invite_meta_response(invite_code, &metadata.guild_name, metadata.guild_icon_url);
    }
    serve_path(path)
}

fn normalize_path(uri: &Uri) -> &str {
    match uri.path().strip_prefix('/') {
        Some("") | None => "index.html",
        Some(path) if path.ends_with('/') => "index.html",
        Some(path) => path,
    }
}

fn serve_path(path: &str) -> Response {
    if let Some(file) = ClientDist::get(path) {
        return file_response(path, file);
    }

    // A request that looks like a file path (has an extension) should 404 if missing.
    if path != "index.html"
        && path
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

fn invite_code_from_path(path: &str) -> Option<&str> {
    let code = path.strip_prefix("invite/")?;
    if code.is_empty() || code.contains('/') {
        return None;
    }
    Some(code)
}

fn invite_meta_response(
    invite_code: &str,
    guild_name: &str,
    guild_icon_url: Option<String>,
) -> Response {
    let escaped_code = html_escape(invite_code);
    let escaped_guild_name = html_escape(guild_name);
    let redirect_target = format!("/?invite={escaped_code}");
    let escaped_redirect_target = html_escape(&redirect_target);
    let title = format!("Join {escaped_guild_name} on Discool");
    let description = format!("Join {escaped_guild_name} on Discool.");

    let mut html = format!(
        "<!doctype html>
<html lang=\"en\">
  <head>
    <meta charset=\"UTF-8\" />
    <meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\" />
    <title>{title}</title>
    <meta property=\"og:title\" content=\"{title}\" />
    <meta property=\"og:description\" content=\"{description}\" />
    <meta property=\"og:type\" content=\"website\" />
    <meta property=\"og:url\" content=\"/invite/{escaped_code}\" />
    <meta name=\"twitter:card\" content=\"summary\" />
    <meta name=\"twitter:title\" content=\"{title}\" />
    <meta name=\"twitter:description\" content=\"{description}\" />
    <meta http-equiv=\"refresh\" content=\"0;url={escaped_redirect_target}\" />
  </head>
  <body>
    <p>Redirecting…</p>
    <script>
      window.location.replace('{escaped_redirect_target}');
    </script>
  </body>
</html>",
    );

    if let Some(icon_url) = guild_icon_url {
        let escaped_icon_url = html_escape(&icon_url);
        let icon_meta = format!(
            "<meta property=\"og:image\" content=\"{escaped_icon_url}\" />
    <meta name=\"twitter:image\" content=\"{escaped_icon_url}\" />"
        );
        html = html.replace(
            "<meta name=\"twitter:card\" content=\"summary\" />",
            &format!("{icon_meta}\n    <meta name=\"twitter:card\" content=\"summary\" />"),
        );
    }

    let mut response = Response::new(Body::from(html));
    *response.status_mut() = StatusCode::OK;
    let headers = response.headers_mut();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/html; charset=utf-8"),
    );
    headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-cache"));
    response
}

fn html_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('\"', "&quot;")
        .replace('\'', "&#39;")
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
    use axum::{
        body::to_bytes,
        http::{StatusCode, header},
    };

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

    #[tokio::test]
    async fn invite_meta_response_includes_open_graph_tags() {
        let res = invite_meta_response(
            "invite123",
            "Makers Hub",
            Some("/api/v1/guilds/makers-hub/icon".to_string()),
        );
        assert_eq!(res.status(), StatusCode::OK);
        assert_eq!(
            res.headers().get(header::CONTENT_TYPE).unwrap(),
            "text/html; charset=utf-8"
        );
        assert_eq!(
            res.headers().get(header::CACHE_CONTROL).unwrap(),
            "no-cache"
        );

        let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
        let body = String::from_utf8(body.to_vec()).unwrap();
        assert!(body.contains("og:title"));
        assert!(body.contains("Join Makers Hub on Discool"));
        assert!(body.contains("og:image"));
        assert!(body.contains("/?invite=invite123"));
    }
}
