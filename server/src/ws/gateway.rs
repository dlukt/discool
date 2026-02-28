use std::time::{Duration, Instant};

use axum::extract::ws::{Message, WebSocket};
use serde::Deserialize;
use serde_json::{Value, json};
use tokio::sync::mpsc;

use crate::services::presence_service;

use super::{
    protocol::{
        ClientEnvelope, ClientOp, PROTOCOL_VERSION, ProtocolError, STORY_6_1_SERVER_EVENTS,
        ServerOp, parse_client_op,
    },
    registry,
};

const RATE_LIMIT_WINDOW: Duration = Duration::from_secs(1);
const RATE_LIMIT_MAX_OPS_PER_WINDOW: u32 = 30;

#[derive(Debug, Deserialize)]
struct SubscribePayload {
    guild_slug: String,
    #[serde(default)]
    channel_slug: Option<String>,
}

#[derive(Debug, Deserialize)]
struct MessageCreatePayload {
    guild_slug: String,
    channel_slug: String,
    #[serde(default)]
    content: String,
}

#[derive(Debug, Deserialize)]
struct TypingStartPayload {
    guild_slug: String,
    channel_slug: String,
}

#[derive(Debug, Deserialize)]
struct ResumePayload {
    #[serde(default)]
    last_sequence: Option<u64>,
}

#[derive(Debug, Clone)]
struct RateLimiter {
    window_started: Instant,
    op_count: u32,
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self {
            window_started: Instant::now(),
            op_count: 0,
        }
    }
}

impl RateLimiter {
    fn allow(&mut self) -> bool {
        if self.window_started.elapsed() >= RATE_LIMIT_WINDOW {
            self.window_started = Instant::now();
            self.op_count = 0;
        }

        if self.op_count >= RATE_LIMIT_MAX_OPS_PER_WINDOW {
            return false;
        }

        self.op_count = self.op_count.saturating_add(1);
        true
    }
}

fn protocol_error_payload(error: ProtocolError) -> Value {
    json!({
        "code": error.code,
        "message": error.message,
        "details": error.details,
    })
}

fn send_protocol_error(connection_id: &str, error: ProtocolError) {
    let payload = protocol_error_payload(error);
    registry::send_event(connection_id, ServerOp::Error, &payload);
}

fn parse_envelope(payload: &str) -> Result<ClientEnvelope, ProtocolError> {
    serde_json::from_str::<ClientEnvelope>(payload).map_err(|_| {
        ProtocolError::validation("Malformed websocket payload")
            .with_details(json!({ "reason": "invalid_json" }))
    })
}

fn parse_payload<T: for<'de> Deserialize<'de>>(value: Value, op: &str) -> Result<T, ProtocolError> {
    serde_json::from_value(value).map_err(|_| {
        ProtocolError::validation("Invalid operation payload")
            .with_details(json!({ "op": op, "reason": "invalid_payload_shape" }))
    })
}

fn handle_subscribe(connection_id: &str, payload: SubscribePayload, subscribe: bool) {
    if subscribe {
        registry::subscribe(
            connection_id,
            &payload.guild_slug,
            payload.channel_slug.as_deref(),
        );
    } else {
        registry::unsubscribe(
            connection_id,
            &payload.guild_slug,
            payload.channel_slug.as_deref(),
        );
    }

    let event_payload = json!({
        "guild_slug": payload.guild_slug,
        "channel_slug": payload.channel_slug,
        "subscribed": subscribe,
    });
    registry::send_event(connection_id, ServerOp::ChannelUpdate, &event_payload);
}

fn handle_message_create(connection_id: &str, user_id: &str, payload: MessageCreatePayload) {
    let event_payload = json!({
        "guild_slug": payload.guild_slug,
        "channel_slug": payload.channel_slug,
        "author_user_id": user_id,
        "content": payload.content,
        "connection_id": connection_id,
    });
    registry::broadcast_to_channel(
        event_payload["guild_slug"].as_str().unwrap_or_default(),
        event_payload["channel_slug"].as_str().unwrap_or_default(),
        ServerOp::MessageCreate,
        &event_payload,
    );
}

fn handle_typing_start(connection_id: &str, user_id: &str, payload: TypingStartPayload) {
    let event_payload = json!({
        "guild_slug": payload.guild_slug,
        "channel_slug": payload.channel_slug,
        "user_id": user_id,
        "connection_id": connection_id,
    });
    registry::broadcast_to_channel(
        event_payload["guild_slug"].as_str().unwrap_or_default(),
        event_payload["channel_slug"].as_str().unwrap_or_default(),
        ServerOp::TypingStart,
        &event_payload,
    );
}

fn handle_resume(connection_id: &str, envelope: &ClientEnvelope) {
    let payload: ResumePayload =
        parse_payload(envelope.d.clone(), &envelope.op).unwrap_or(ResumePayload {
            last_sequence: None,
        });
    let requested_sequence = payload.last_sequence.or(envelope.s).unwrap_or(0);
    let snapshot = registry::connection_snapshot(connection_id);
    let resume_payload = json!({
        "requested_sequence": requested_sequence,
        "active_channel": snapshot.and_then(|item| item.active_channel),
        "replay_supported": false,
    });
    registry::send_event(connection_id, ServerOp::ResumeAck, &resume_payload);
}

fn process_client_message(
    connection_id: &str,
    user_id: &str,
    envelope: ClientEnvelope,
    limiter: &mut RateLimiter,
) {
    let client_op = match parse_client_op(&envelope.op) {
        Ok(op) => op,
        Err(error) => {
            send_protocol_error(connection_id, error);
            return;
        }
    };

    if !limiter.allow() {
        tracing::warn!(
            connection_id,
            user_id,
            op = %envelope.op,
            "Websocket rate limit exceeded"
        );
        send_protocol_error(
            connection_id,
            ProtocolError::rate_limited(&envelope.op, RATE_LIMIT_MAX_OPS_PER_WINDOW),
        );
        return;
    }

    match client_op {
        ClientOp::Heartbeat => {
            presence_service::mark_heartbeat(user_id);
            registry::mark_heartbeat(connection_id);
            registry::send_event(
                connection_id,
                ServerOp::HeartbeatAck,
                &json!({ "ok": true }),
            );
        }
        ClientOp::Subscribe => match parse_payload::<SubscribePayload>(envelope.d, &envelope.op) {
            Ok(payload) => handle_subscribe(connection_id, payload, true),
            Err(error) => send_protocol_error(connection_id, error),
        },
        ClientOp::Unsubscribe => {
            match parse_payload::<SubscribePayload>(envelope.d, &envelope.op) {
                Ok(payload) => handle_subscribe(connection_id, payload, false),
                Err(error) => send_protocol_error(connection_id, error),
            }
        }
        ClientOp::MessageCreate => {
            match parse_payload::<MessageCreatePayload>(envelope.d, &envelope.op) {
                Ok(payload) => handle_message_create(connection_id, user_id, payload),
                Err(error) => send_protocol_error(connection_id, error),
            }
        }
        ClientOp::TypingStart => {
            match parse_payload::<TypingStartPayload>(envelope.d, &envelope.op) {
                Ok(payload) => handle_typing_start(connection_id, user_id, payload),
                Err(error) => send_protocol_error(connection_id, error),
            }
        }
        ClientOp::Resume => handle_resume(connection_id, &envelope),
    }
}

pub async fn handle_socket(mut socket: WebSocket, user_id: String, session_id: String) {
    let (sender, mut receiver) = mpsc::unbounded_channel();
    let connection_id = presence_service::register_connection(&user_id, &session_id, sender);
    presence_service::ensure_watchdog_started();
    presence_service::mark_connected(&user_id);

    let hello_payload = json!({
        "protocol_version": PROTOCOL_VERSION,
        "connection_id": connection_id,
        "session_id": session_id,
        "user_id": user_id,
        "resume_supported": true,
        "supported_events": STORY_6_1_SERVER_EVENTS,
    });
    registry::send_event(&connection_id, ServerOp::Hello, &hello_payload);

    let mut rate_limiter = RateLimiter::default();

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
                        match parse_envelope(payload.as_str()) {
                            Ok(envelope) => process_client_message(&connection_id, &user_id, envelope, &mut rate_limiter),
                            Err(error) => send_protocol_error(&connection_id, error),
                        }
                    }
                    Some(Ok(Message::Ping(payload))) => {
                        presence_service::mark_heartbeat(&user_id);
                        registry::mark_heartbeat(&connection_id);
                        registry::send_event(&connection_id, ServerOp::HeartbeatAck, &json!({ "ok": true }));
                        if socket.send(Message::Pong(payload)).await.is_err() {
                            break;
                        }
                    }
                    Some(Ok(Message::Pong(_))) => {
                        presence_service::mark_heartbeat(&user_id);
                        registry::mark_heartbeat(&connection_id);
                    }
                    Some(Ok(Message::Close(_))) => break,
                    Some(Ok(Message::Binary(_))) => {
                        send_protocol_error(
                            &connection_id,
                            ProtocolError::validation("Binary websocket frames are not supported"),
                        );
                    }
                    Some(Err(error)) => {
                        tracing::debug!(error = %error, connection_id, "WebSocket receive error");
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rate_limiter_resets_after_window() {
        let mut limiter = RateLimiter::default();
        for _ in 0..RATE_LIMIT_MAX_OPS_PER_WINDOW {
            assert!(limiter.allow());
        }
        assert!(!limiter.allow());

        limiter.window_started = Instant::now() - RATE_LIMIT_WINDOW - Duration::from_millis(1);
        assert!(limiter.allow());
    }
}
