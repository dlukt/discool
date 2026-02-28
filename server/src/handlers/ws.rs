use axum::{
    extract::{Query, State, ws::WebSocketUpgrade},
    http::{HeaderMap, header::AUTHORIZATION},
    response::{IntoResponse, Response},
};
use serde::Deserialize;

use crate::{AppError, AppState, services::auth_service, ws::gateway};

#[derive(Debug, Deserialize)]
pub struct WsQuery {
    #[serde(default)]
    pub token: Option<String>,
}

pub async fn connect(
    State(state): State<AppState>,
    ws: WebSocketUpgrade,
    Query(query): Query<WsQuery>,
    headers: HeaderMap,
) -> Result<Response, AppError> {
    let token = resolve_session_token(&query, &headers)?;
    let (session, _user) = auth_service::validate_session(&state.pool, &token).await?;
    let user_id = session.user_id;
    let session_id = session.id;

    Ok(ws
        .on_upgrade(move |socket| gateway::handle_socket(socket, user_id, session_id))
        .into_response())
}

fn resolve_session_token(query: &WsQuery, headers: &HeaderMap) -> Result<String, AppError> {
    if let Some(token) = query.token.as_deref() {
        let trimmed = token.trim();
        if trimmed.is_empty() {
            return Err(AppError::ValidationError(
                "Websocket token cannot be empty".to_string(),
            ));
        }
        return Ok(trimmed.to_string());
    }

    if let Some(header_value) = headers.get(AUTHORIZATION) {
        let header = header_value.to_str().map_err(|_| {
            AppError::ValidationError("Invalid Authorization header encoding".to_string())
        })?;
        return parse_bearer_token(header)
            .map(ToString::to_string)
            .ok_or_else(|| {
                AppError::ValidationError("Invalid Authorization header for websocket".to_string())
            });
    }

    Err(AppError::Unauthorized(
        "Missing websocket session token".to_string(),
    ))
}

fn parse_bearer_token(header: &str) -> Option<&str> {
    let mut parts = header.split_whitespace();
    let scheme = parts.next()?;
    let token = parts.next()?;
    if !scheme.eq_ignore_ascii_case("bearer") || token.trim().is_empty() {
        return None;
    }
    if parts.next().is_some() {
        return None;
    }
    Some(token.trim())
}
