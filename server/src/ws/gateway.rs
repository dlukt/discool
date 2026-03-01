use std::time::{Duration, Instant};

use axum::extract::ws::{Message, WebSocket};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tokio::sync::mpsc;

use crate::{
    AppError,
    config::AttachmentConfig,
    db::DbPool,
    services::{dm_service, message_service, presence_service},
};

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
    #[serde(default)]
    client_nonce: Option<String>,
}

#[derive(Debug, Deserialize)]
struct MessageUpdatePayload {
    guild_slug: String,
    channel_slug: String,
    message_id: String,
    #[serde(default)]
    content: String,
}

#[derive(Debug, Deserialize)]
struct MessageDeletePayload {
    guild_slug: String,
    channel_slug: String,
    message_id: String,
}

#[derive(Debug, Deserialize)]
struct MessageReactionTogglePayload {
    guild_slug: String,
    channel_slug: String,
    message_id: String,
    emoji: String,
}

#[derive(Debug, Deserialize)]
struct TypingStartPayload {
    guild_slug: String,
    channel_slug: String,
}

#[derive(Debug, Deserialize)]
struct DmSubscribePayload {
    #[serde(default)]
    dm_slug: Option<String>,
}

#[derive(Debug, Deserialize)]
struct DmMessageCreatePayload {
    dm_slug: String,
    #[serde(default)]
    content: String,
    #[serde(default)]
    client_nonce: Option<String>,
}

#[derive(Debug, Serialize)]
struct ChannelActivityPayload {
    guild_slug: String,
    channel_slug: String,
    actor_user_id: String,
    message_id: String,
}

#[derive(Debug, Serialize)]
struct DmActivityPayload {
    dm_slug: String,
    actor_user_id: String,
    message_id: String,
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

fn app_error_payload(error: AppError) -> Value {
    match error {
        AppError::NotFound => json!({
            "code": "NOT_FOUND",
            "message": "Resource not found",
            "details": {},
        }),
        AppError::Unauthorized(message) => json!({
            "code": "UNAUTHORIZED",
            "message": message,
            "details": {},
        }),
        AppError::Forbidden(message) => json!({
            "code": "FORBIDDEN",
            "message": message,
            "details": {},
        }),
        AppError::Conflict(message) => json!({
            "code": "CONFLICT",
            "message": message,
            "details": {},
        }),
        AppError::ValidationError(message) => json!({
            "code": "VALIDATION_ERROR",
            "message": message,
            "details": {},
        }),
        AppError::Internal(message) => {
            tracing::error!(%message, "Websocket message handling failed");
            json!({
                "code": "INTERNAL_ERROR",
                "message": "An internal error occurred",
                "details": {},
            })
        }
    }
}

fn send_app_error(connection_id: &str, error: AppError) {
    let payload = app_error_payload(error);
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

async fn handle_dm_subscribe(
    pool: &DbPool,
    user_id: &str,
    connection_id: &str,
    payload: DmSubscribePayload,
) -> Result<(), AppError> {
    let Some(dm_slug) = payload.dm_slug.as_deref() else {
        registry::clear_dm_subscription(connection_id);
        return Ok(());
    };
    let dm_slug = dm_slug.trim();
    if dm_slug.is_empty() {
        registry::clear_dm_subscription(connection_id);
        return Ok(());
    }
    dm_service::assert_dm_participant(pool, user_id, dm_slug).await?;
    registry::subscribe_dm(connection_id, dm_slug);
    Ok(())
}

async fn handle_message_create(
    pool: &DbPool,
    user_id: &str,
    payload: MessageCreatePayload,
) -> Result<(), AppError> {
    let created = message_service::create_message(
        pool,
        user_id,
        message_service::CreateMessageInput {
            guild_slug: payload.guild_slug,
            channel_slug: payload.channel_slug,
            content: payload.content,
            client_nonce: payload.client_nonce,
        },
    )
    .await?;
    registry::broadcast_to_channel(
        &created.guild_slug,
        &created.channel_slug,
        ServerOp::MessageCreate,
        &created,
    );
    emit_channel_activity_event(pool, &created).await?;
    Ok(())
}

async fn handle_dm_message_create(
    pool: &DbPool,
    user_id: &str,
    payload: DmMessageCreatePayload,
) -> Result<(), AppError> {
    let created = dm_service::create_dm_message(
        pool,
        user_id,
        dm_service::CreateDmMessageInput {
            dm_slug: payload.dm_slug,
            content: payload.content,
            client_nonce: payload.client_nonce,
        },
    )
    .await?;

    let dm_targets = registry::dm_connection_targets(&created.message.dm_slug);
    for target in dm_targets {
        if created.participant_user_ids.contains(&target.user_id) {
            registry::send_event(
                &target.connection_id,
                ServerOp::DmMessageCreate,
                &created.message,
            );
        }
    }

    let activity_payload = DmActivityPayload {
        dm_slug: created.message.dm_slug.clone(),
        actor_user_id: created.message.author_user_id.clone(),
        message_id: created.message.id.clone(),
    };
    for target in registry::user_connection_targets(&created.participant_user_ids) {
        if target.user_id == created.message.author_user_id {
            continue;
        }
        registry::send_event(
            &target.connection_id,
            ServerOp::DmActivity,
            &activity_payload,
        );
    }

    Ok(())
}

pub async fn emit_channel_activity_event(
    pool: &DbPool,
    message: &message_service::MessageResponse,
) -> Result<(), AppError> {
    let targets = registry::guild_connection_targets(&message.guild_slug);
    if targets.is_empty() {
        return Ok(());
    }
    let viewer_user_ids = targets
        .iter()
        .map(|target| target.user_id.clone())
        .collect::<Vec<_>>();
    let allowed_viewers = message_service::filter_channel_viewer_user_ids(
        pool,
        &message.guild_slug,
        &message.channel_slug,
        &viewer_user_ids,
    )
    .await?;
    if allowed_viewers.is_empty() {
        return Ok(());
    }

    let payload = ChannelActivityPayload {
        guild_slug: message.guild_slug.clone(),
        channel_slug: message.channel_slug.clone(),
        actor_user_id: message.author_user_id.clone(),
        message_id: message.id.clone(),
    };
    for target in targets {
        if allowed_viewers.contains(&target.user_id) {
            registry::send_event(&target.connection_id, ServerOp::ChannelActivity, &payload);
        }
    }
    Ok(())
}

async fn handle_message_update(
    pool: &DbPool,
    user_id: &str,
    payload: MessageUpdatePayload,
) -> Result<(), AppError> {
    let updated = message_service::update_message(
        pool,
        user_id,
        message_service::UpdateMessageInput {
            guild_slug: payload.guild_slug,
            channel_slug: payload.channel_slug,
            message_id: payload.message_id,
            content: payload.content,
        },
    )
    .await?;
    registry::broadcast_to_channel(
        &updated.guild_slug,
        &updated.channel_slug,
        ServerOp::MessageUpdate,
        &updated,
    );
    Ok(())
}

async fn handle_message_delete(
    pool: &DbPool,
    attachment_config: &AttachmentConfig,
    user_id: &str,
    payload: MessageDeletePayload,
) -> Result<(), AppError> {
    let deleted = message_service::delete_message(
        pool,
        attachment_config,
        user_id,
        message_service::DeleteMessageInput {
            guild_slug: payload.guild_slug,
            channel_slug: payload.channel_slug,
            message_id: payload.message_id,
        },
    )
    .await?;
    registry::broadcast_to_channel(
        &deleted.guild_slug,
        &deleted.channel_slug,
        ServerOp::MessageDelete,
        &deleted,
    );
    Ok(())
}

async fn handle_message_reaction_toggle(
    pool: &DbPool,
    user_id: &str,
    payload: MessageReactionTogglePayload,
) -> Result<(), AppError> {
    let updated = message_service::toggle_message_reaction(
        pool,
        user_id,
        message_service::ToggleMessageReactionInput {
            guild_slug: payload.guild_slug,
            channel_slug: payload.channel_slug,
            message_id: payload.message_id,
            emoji: payload.emoji,
        },
    )
    .await?;
    let targets = registry::channel_connection_targets(&updated.guild_slug, &updated.channel_slug);
    if targets.is_empty() {
        return Ok(());
    }
    let viewer_user_ids = targets
        .iter()
        .map(|target| target.user_id.clone())
        .collect::<Vec<_>>();
    let reactions_by_viewer = message_service::list_message_reaction_summaries_for_viewers(
        pool,
        &updated.message_id,
        &viewer_user_ids,
    )
    .await?;
    for target in targets {
        let reactions = reactions_by_viewer
            .get(&target.user_id)
            .cloned()
            .unwrap_or_default();
        let payload = message_service::MessageReactionUpdateResponse {
            guild_slug: updated.guild_slug.clone(),
            channel_slug: updated.channel_slug.clone(),
            message_id: updated.message_id.clone(),
            actor_user_id: updated.actor_user_id.clone(),
            reactions,
        };
        registry::send_event(
            &target.connection_id,
            ServerOp::MessageReactionUpdate,
            &payload,
        );
    }
    Ok(())
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
    let active_channel = snapshot
        .as_ref()
        .and_then(|item| item.active_channel.clone());
    let active_dm = snapshot.as_ref().and_then(|item| item.active_dm.clone());
    let resume_payload = json!({
        "requested_sequence": requested_sequence,
        "active_channel": active_channel,
        "active_dm": active_dm,
        "replay_supported": false,
    });
    registry::send_event(connection_id, ServerOp::ResumeAck, &resume_payload);
}

async fn process_client_message(
    pool: &DbPool,
    attachment_config: &AttachmentConfig,
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
                Ok(payload) => {
                    if let Err(error) = handle_message_create(pool, user_id, payload).await {
                        send_app_error(connection_id, error);
                    }
                }
                Err(error) => send_protocol_error(connection_id, error),
            }
        }
        ClientOp::MessageUpdate => {
            match parse_payload::<MessageUpdatePayload>(envelope.d, &envelope.op) {
                Ok(payload) => {
                    if let Err(error) = handle_message_update(pool, user_id, payload).await {
                        send_app_error(connection_id, error);
                    }
                }
                Err(error) => send_protocol_error(connection_id, error),
            }
        }
        ClientOp::MessageDelete => {
            match parse_payload::<MessageDeletePayload>(envelope.d, &envelope.op) {
                Ok(payload) => {
                    if let Err(error) =
                        handle_message_delete(pool, attachment_config, user_id, payload).await
                    {
                        send_app_error(connection_id, error);
                    }
                }
                Err(error) => send_protocol_error(connection_id, error),
            }
        }
        ClientOp::MessageReactionToggle => {
            match parse_payload::<MessageReactionTogglePayload>(envelope.d, &envelope.op) {
                Ok(payload) => {
                    if let Err(error) = handle_message_reaction_toggle(pool, user_id, payload).await
                    {
                        send_app_error(connection_id, error);
                    }
                }
                Err(error) => send_protocol_error(connection_id, error),
            }
        }
        ClientOp::DmSubscribe => {
            match parse_payload::<DmSubscribePayload>(envelope.d, &envelope.op) {
                Ok(payload) => {
                    if let Err(error) =
                        handle_dm_subscribe(pool, user_id, connection_id, payload).await
                    {
                        send_app_error(connection_id, error);
                    }
                }
                Err(error) => send_protocol_error(connection_id, error),
            }
        }
        ClientOp::DmMessageCreate => {
            match parse_payload::<DmMessageCreatePayload>(envelope.d, &envelope.op) {
                Ok(payload) => {
                    if let Err(error) = handle_dm_message_create(pool, user_id, payload).await {
                        send_app_error(connection_id, error);
                    }
                }
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

pub async fn handle_socket(
    mut socket: WebSocket,
    user_id: String,
    session_id: String,
    pool: DbPool,
    attachment_config: AttachmentConfig,
) {
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
                            Ok(envelope) => {
                                process_client_message(
                                    &pool,
                                    &attachment_config,
                                    &connection_id,
                                    &user_id,
                                    envelope,
                                    &mut rate_limiter,
                                )
                                .await
                            }
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
