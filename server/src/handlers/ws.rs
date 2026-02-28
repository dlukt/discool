use axum::{
    extract::{
        Query, State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    http::{HeaderMap, header::AUTHORIZATION},
    response::{IntoResponse, Response},
};
use serde::Deserialize;
use tokio::sync::mpsc;

use crate::{
    AppError, AppState,
    services::{auth_service, presence_service},
};

#[derive(Debug, Deserialize)]
pub struct WsQuery {
    #[serde(default)]
    pub token: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ClientEnvelope {
    op: String,
}

pub async fn connect(
    State(state): State<AppState>,
    ws: WebSocketUpgrade,
    Query(query): Query<WsQuery>,
    headers: HeaderMap,
) -> Result<Response, AppError> {
    let token = resolve_session_token(&query, &headers)?;
    let (session, _user) = auth_service::validate_session(&state.pool, &token).await?;
    presence_service::ensure_watchdog_started();
    let user_id = session.user_id;

    Ok(ws
        .on_upgrade(move |socket| handle_socket(socket, user_id))
        .into_response())
}

fn resolve_session_token(query: &WsQuery, headers: &HeaderMap) -> Result<String, AppError> {
    if let Some(token) = query.token.as_deref() {
        let trimmed = token.trim();
        if !trimmed.is_empty() {
            return Ok(trimmed.to_string());
        }
    }

    if let Some(header_value) = headers.get(AUTHORIZATION)
        && let Ok(header) = header_value.to_str()
        && let Some(token) = parse_bearer_token(header)
    {
        return Ok(token.to_string());
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

fn handle_text_frame(user_id: &str, payload: &str) {
    let Ok(envelope) = serde_json::from_str::<ClientEnvelope>(payload) else {
        return;
    };
    if envelope.op.eq_ignore_ascii_case("heartbeat") {
        presence_service::mark_heartbeat(user_id);
    }
}

async fn handle_socket(mut socket: WebSocket, user_id: String) {
    let (sender, mut receiver) = mpsc::unbounded_channel();
    let connection_id = presence_service::register_connection(sender);
    presence_service::mark_connected(&user_id);

    loop {
        tokio::select! {
            outbound = receiver.recv() => {
                match outbound {
                    Some(message) => {
                        if socket.send(message).await.is_err() {
                            break;
                        }
                    }
                    None => break,
                }
            }
            inbound = socket.recv() => {
                match inbound {
                    Some(Ok(Message::Text(payload))) => {
                        handle_text_frame(&user_id, &payload);
                    }
                    Some(Ok(Message::Ping(payload))) => {
                        presence_service::mark_heartbeat(&user_id);
                        if socket.send(Message::Pong(payload)).await.is_err() {
                            break;
                        }
                    }
                    Some(Ok(Message::Pong(_))) => {
                        presence_service::mark_heartbeat(&user_id);
                    }
                    Some(Ok(Message::Close(_))) => break,
                    Some(Ok(Message::Binary(_))) => {}
                    Some(Err(err)) => {
                        tracing::debug!(error = %err, "WebSocket receive error");
                        break;
                    }
                    None => break,
                }
            }
        }
    }

    presence_service::mark_disconnected(&user_id);
    presence_service::unregister_connection(&connection_id);
}
