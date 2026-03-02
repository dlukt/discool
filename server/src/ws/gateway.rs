use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};

use axum::extract::ws::{Message, WebSocket};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tokio::sync::mpsc;

use crate::{
    AppError,
    config::AttachmentConfig,
    db::DbPool,
    models::{channel, guild, guild_member},
    services::{dm_service, message_service, moderation_service, presence_service},
    webrtc::{
        signaling::{
            VoiceParticipantPayload, VoiceStateUpdatePayload as ServerVoiceStateUpdatePayload,
        },
        voice_channel::{VoiceChannelRef, VoiceParticipantStateUpdate, VoiceRuntime},
    },
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

#[derive(Debug, Deserialize)]
struct VoiceJoinPayload {
    guild_slug: String,
    channel_slug: String,
}

#[derive(Debug, Deserialize)]
struct VoiceLeavePayload {
    guild_slug: String,
    channel_slug: String,
}

#[derive(Debug, Deserialize)]
struct VoiceAnswerPayload {
    guild_slug: String,
    channel_slug: String,
    sdp: String,
    sdp_type: String,
}

#[derive(Debug, Deserialize)]
struct VoiceIceCandidatePayload {
    guild_slug: String,
    channel_slug: String,
    candidate: String,
    #[serde(default)]
    sdp_mid: Option<String>,
    #[serde(default)]
    sdp_mline_index: Option<u16>,
}

#[derive(Debug, Deserialize)]
struct VoiceStateUpdateClientPayload {
    guild_slug: String,
    channel_slug: String,
    is_muted: bool,
    is_deafened: bool,
    is_speaking: bool,
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

fn parse_required_non_empty_field(
    value: &str,
    op: &str,
    field: &'static str,
) -> Result<String, ProtocolError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(
            ProtocolError::validation(format!("{field} is required")).with_details(json!({
                "op": op,
                "field": field,
                "reason": "invalid_payload_field",
            })),
        );
    }
    Ok(trimmed.to_string())
}

fn parse_voice_join_payload(value: Value, op: &str) -> Result<VoiceJoinPayload, ProtocolError> {
    let parsed = parse_payload::<VoiceJoinPayload>(value, op)?;
    Ok(VoiceJoinPayload {
        guild_slug: parse_required_non_empty_field(&parsed.guild_slug, op, "guild_slug")?,
        channel_slug: parse_required_non_empty_field(&parsed.channel_slug, op, "channel_slug")?,
    })
}

fn parse_voice_leave_payload(value: Value, op: &str) -> Result<VoiceLeavePayload, ProtocolError> {
    let parsed = parse_payload::<VoiceLeavePayload>(value, op)?;
    Ok(VoiceLeavePayload {
        guild_slug: parse_required_non_empty_field(&parsed.guild_slug, op, "guild_slug")?,
        channel_slug: parse_required_non_empty_field(&parsed.channel_slug, op, "channel_slug")?,
    })
}

fn parse_voice_answer_payload(value: Value, op: &str) -> Result<VoiceAnswerPayload, ProtocolError> {
    let parsed = parse_payload::<VoiceAnswerPayload>(value, op)?;
    let sdp_type = parse_required_non_empty_field(&parsed.sdp_type, op, "sdp_type")?;
    if sdp_type != "answer" {
        return Err(
            ProtocolError::validation("sdp_type must be `answer`").with_details(json!({
                "op": op,
                "field": "sdp_type",
                "reason": "invalid_payload_field",
            })),
        );
    }
    Ok(VoiceAnswerPayload {
        guild_slug: parse_required_non_empty_field(&parsed.guild_slug, op, "guild_slug")?,
        channel_slug: parse_required_non_empty_field(&parsed.channel_slug, op, "channel_slug")?,
        sdp: parse_required_non_empty_field(&parsed.sdp, op, "sdp")?,
        sdp_type,
    })
}

fn parse_voice_ice_candidate_payload(
    value: Value,
    op: &str,
) -> Result<VoiceIceCandidatePayload, ProtocolError> {
    let parsed = parse_payload::<VoiceIceCandidatePayload>(value, op)?;
    let candidate = parse_required_non_empty_field(&parsed.candidate, op, "candidate")?;
    if let Some(mid) = parsed.sdp_mid.as_ref()
        && mid.trim().is_empty()
    {
        return Err(
            ProtocolError::validation("sdp_mid must not be empty").with_details(json!({
                "op": op,
                "field": "sdp_mid",
                "reason": "invalid_payload_field",
            })),
        );
    }
    Ok(VoiceIceCandidatePayload {
        guild_slug: parse_required_non_empty_field(&parsed.guild_slug, op, "guild_slug")?,
        channel_slug: parse_required_non_empty_field(&parsed.channel_slug, op, "channel_slug")?,
        candidate,
        sdp_mid: parsed.sdp_mid.map(|value| value.trim().to_string()),
        sdp_mline_index: parsed.sdp_mline_index,
    })
}

fn parse_voice_state_update_payload(
    value: Value,
    op: &str,
) -> Result<VoiceStateUpdateClientPayload, ProtocolError> {
    let parsed = parse_payload::<VoiceStateUpdateClientPayload>(value, op)?;
    Ok(VoiceStateUpdateClientPayload {
        guild_slug: parse_required_non_empty_field(&parsed.guild_slug, op, "guild_slug")?,
        channel_slug: parse_required_non_empty_field(&parsed.channel_slug, op, "channel_slug")?,
        is_muted: parsed.is_muted,
        is_deafened: parsed.is_deafened,
        is_speaking: parsed.is_speaking,
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

async fn handle_typing_start(
    pool: &DbPool,
    connection_id: &str,
    user_id: &str,
    payload: TypingStartPayload,
) -> Result<(), AppError> {
    moderation_service::assert_member_can_start_typing(pool, &payload.guild_slug, user_id).await?;
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
    Ok(())
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

async fn build_voice_state_update_payload(
    pool: &DbPool,
    voice_runtime: &VoiceRuntime,
    guild_slug: &str,
    channel_slug: &str,
) -> Result<Option<ServerVoiceStateUpdatePayload>, AppError> {
    let Some(guild_record) = guild::find_guild_by_slug(pool, guild_slug).await? else {
        return Ok(None);
    };
    let member_profiles = guild_member::list_guild_member_profiles(pool, &guild_record.id).await?;
    let mut profile_by_user: HashMap<String, (String, Option<String>, Option<String>)> =
        HashMap::with_capacity(member_profiles.len());
    for profile in member_profiles {
        profile_by_user.insert(
            profile.user_id,
            (profile.username, profile.display_name, profile.avatar_color),
        );
    }

    let participant_states = voice_runtime.participants_for_channel(guild_slug, channel_slug);
    let mut participants = Vec::with_capacity(participant_states.len());
    for participant in participant_states {
        let profile = profile_by_user.get(&participant.user_id);
        let (username, display_name, avatar_color) = if let Some(profile) = profile {
            (profile.0.clone(), profile.1.clone(), profile.2.clone())
        } else if let Some(profile) =
            guild_member::find_user_profile_by_id(pool, &participant.user_id).await?
        {
            (profile.username, profile.display_name, profile.avatar_color)
        } else {
            (participant.user_id.clone(), None, None)
        };
        participants.push(VoiceParticipantPayload {
            user_id: participant.user_id,
            username,
            display_name,
            avatar_color,
            is_muted: participant.is_muted,
            is_deafened: participant.is_deafened,
            is_speaking: participant.is_speaking,
        });
    }

    Ok(Some(ServerVoiceStateUpdatePayload {
        guild_slug: guild_slug.to_string(),
        channel_slug: channel_slug.to_string(),
        participant_count: u32::try_from(participants.len()).unwrap_or(u32::MAX),
        participants,
    }))
}

async fn broadcast_voice_state_update(
    pool: &DbPool,
    voice_runtime: &VoiceRuntime,
    guild_slug: &str,
    channel_slug: &str,
) -> Result<(), AppError> {
    let Some(payload) =
        build_voice_state_update_payload(pool, voice_runtime, guild_slug, channel_slug).await?
    else {
        return Ok(());
    };
    registry::broadcast_to_guild(guild_slug, ServerOp::VoiceStateUpdate, &payload);
    Ok(())
}

fn switched_channels_to_rebroadcast(
    previous_channels: Vec<VoiceChannelRef>,
    target_guild_slug: &str,
    target_channel_slug: &str,
) -> Vec<VoiceChannelRef> {
    previous_channels
        .into_iter()
        .filter(|previous_channel| {
            previous_channel.guild_slug == target_guild_slug
                && previous_channel.channel_slug != target_channel_slug
        })
        .collect()
}

async fn handle_voice_join(
    pool: &DbPool,
    voice_runtime: &VoiceRuntime,
    connection_id: &str,
    user_id: &str,
    payload: VoiceJoinPayload,
) -> Result<(), AppError> {
    let guild = guild::find_guild_by_slug(pool, &payload.guild_slug)
        .await?
        .ok_or(AppError::NotFound)?;
    let channel = channel::find_channel_by_slug(pool, &guild.id, &payload.channel_slug)
        .await?
        .ok_or(AppError::NotFound)?;
    if channel.channel_type != "voice" {
        return Err(AppError::ValidationError(
            "voice join requires a voice channel".to_string(),
        ));
    }

    let allowed = message_service::filter_channel_viewer_user_ids(
        pool,
        &payload.guild_slug,
        &payload.channel_slug,
        &[user_id.to_string()],
    )
    .await?;
    if !allowed.contains(user_id) {
        return Err(AppError::Forbidden(
            "Missing VIEW_CHANNEL permission in this channel".to_string(),
        ));
    }

    let previous_channels = switched_channels_to_rebroadcast(
        voice_runtime.channels_for_connection(connection_id),
        &payload.guild_slug,
        &payload.channel_slug,
    );
    let start = voice_runtime
        .start_signaling(
            connection_id,
            user_id,
            &payload.guild_slug,
            &payload.channel_slug,
        )
        .await
        .map_err(AppError::ValidationError)?;
    registry::send_event(
        connection_id,
        ServerOp::VoiceConnectionState,
        &start.connection_state,
    );
    registry::send_event(connection_id, ServerOp::VoiceOffer, &start.offer);
    for candidate in start.candidates {
        registry::send_event(connection_id, ServerOp::VoiceIceCandidate, &candidate);
    }
    for previous_channel in previous_channels {
        broadcast_voice_state_update(
            pool,
            voice_runtime,
            &previous_channel.guild_slug,
            &previous_channel.channel_slug,
        )
        .await?;
    }
    broadcast_voice_state_update(
        pool,
        voice_runtime,
        &payload.guild_slug,
        &payload.channel_slug,
    )
    .await?;
    Ok(())
}

async fn handle_voice_leave(
    pool: &DbPool,
    voice_runtime: &VoiceRuntime,
    connection_id: &str,
    payload: VoiceLeavePayload,
) -> Result<(), AppError> {
    voice_runtime
        .leave_session(connection_id, &payload.guild_slug, &payload.channel_slug)
        .await;
    registry::send_event(
        connection_id,
        ServerOp::VoiceConnectionState,
        &json!({
            "guild_slug": payload.guild_slug,
            "channel_slug": payload.channel_slug,
            "state": "disconnected",
        }),
    );
    broadcast_voice_state_update(
        pool,
        voice_runtime,
        &payload.guild_slug,
        &payload.channel_slug,
    )
    .await?;
    Ok(())
}

pub async fn disconnect_user_from_voice_channel(
    pool: &DbPool,
    voice_runtime: &VoiceRuntime,
    guild_slug: &str,
    channel_slug: &str,
    target_user_id: &str,
) -> Result<(), AppError> {
    let target_user_id = target_user_id.trim();
    if target_user_id.is_empty() {
        return Err(AppError::ValidationError(
            "target_user_id is required".to_string(),
        ));
    }

    let disconnected_connections = voice_runtime
        .disconnect_user_from_channel(guild_slug, channel_slug, target_user_id)
        .await;
    for connection_id in disconnected_connections {
        registry::send_event(
            &connection_id,
            ServerOp::VoiceConnectionState,
            &json!({
                "guild_slug": guild_slug,
                "channel_slug": channel_slug,
                "state": "disconnected",
            }),
        );
    }

    broadcast_voice_state_update(pool, voice_runtime, guild_slug, channel_slug).await
}

async fn handle_voice_answer(
    voice_runtime: &VoiceRuntime,
    connection_id: &str,
    payload: VoiceAnswerPayload,
) -> Result<(), AppError> {
    if payload.sdp_type != "answer" {
        return Err(AppError::ValidationError(
            "sdp_type must be `answer`".to_string(),
        ));
    }
    let state = voice_runtime
        .apply_answer(
            connection_id,
            &payload.guild_slug,
            &payload.channel_slug,
            &payload.sdp,
        )
        .await
        .map_err(AppError::ValidationError)?;
    registry::send_event(connection_id, ServerOp::VoiceConnectionState, &state);
    Ok(())
}

async fn handle_voice_ice_candidate(
    voice_runtime: &VoiceRuntime,
    connection_id: &str,
    payload: VoiceIceCandidatePayload,
) -> Result<(), AppError> {
    if payload.candidate.trim().is_empty() {
        return Err(AppError::ValidationError(
            "candidate is required".to_string(),
        ));
    }
    voice_runtime
        .apply_remote_candidate(
            connection_id,
            &payload.guild_slug,
            &payload.channel_slug,
            &payload.candidate,
            payload.sdp_mid.as_deref(),
            payload.sdp_mline_index,
        )
        .await
        .map_err(AppError::ValidationError)
}

async fn handle_voice_state_update(
    pool: &DbPool,
    voice_runtime: &VoiceRuntime,
    connection_id: &str,
    user_id: &str,
    payload: VoiceStateUpdateClientPayload,
) -> Result<(), AppError> {
    voice_runtime
        .update_participant_state(
            connection_id,
            user_id,
            &payload.guild_slug,
            &payload.channel_slug,
            VoiceParticipantStateUpdate {
                is_muted: payload.is_muted,
                is_deafened: payload.is_deafened,
                is_speaking: payload.is_speaking,
            },
        )
        .map_err(AppError::ValidationError)?;
    broadcast_voice_state_update(
        pool,
        voice_runtime,
        &payload.guild_slug,
        &payload.channel_slug,
    )
    .await
}

async fn process_client_message(
    pool: &DbPool,
    attachment_config: &AttachmentConfig,
    voice_runtime: &VoiceRuntime,
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
                Ok(payload) => {
                    if let Err(error) =
                        handle_typing_start(pool, connection_id, user_id, payload).await
                    {
                        send_app_error(connection_id, error);
                    }
                }
                Err(error) => send_protocol_error(connection_id, error),
            }
        }
        ClientOp::Resume => handle_resume(connection_id, &envelope),
        ClientOp::VoiceJoin => match parse_voice_join_payload(envelope.d, &envelope.op) {
            Ok(payload) => {
                if let Err(error) =
                    handle_voice_join(pool, voice_runtime, connection_id, user_id, payload).await
                {
                    send_app_error(connection_id, error);
                }
            }
            Err(error) => send_protocol_error(connection_id, error),
        },
        ClientOp::VoiceLeave => match parse_voice_leave_payload(envelope.d, &envelope.op) {
            Ok(payload) => {
                if let Err(error) =
                    handle_voice_leave(pool, voice_runtime, connection_id, payload).await
                {
                    send_app_error(connection_id, error);
                }
            }
            Err(error) => send_protocol_error(connection_id, error),
        },
        ClientOp::VoiceAnswer => match parse_voice_answer_payload(envelope.d, &envelope.op) {
            Ok(payload) => {
                if let Err(error) = handle_voice_answer(voice_runtime, connection_id, payload).await
                {
                    send_app_error(connection_id, error);
                }
            }
            Err(error) => send_protocol_error(connection_id, error),
        },
        ClientOp::VoiceIceCandidate => {
            match parse_voice_ice_candidate_payload(envelope.d, &envelope.op) {
                Ok(payload) => {
                    if let Err(error) =
                        handle_voice_ice_candidate(voice_runtime, connection_id, payload).await
                    {
                        send_app_error(connection_id, error);
                    }
                }
                Err(error) => send_protocol_error(connection_id, error),
            }
        }
        ClientOp::VoiceStateUpdate => {
            match parse_voice_state_update_payload(envelope.d, &envelope.op) {
                Ok(payload) => {
                    if let Err(error) = handle_voice_state_update(
                        pool,
                        voice_runtime,
                        connection_id,
                        user_id,
                        payload,
                    )
                    .await
                    {
                        send_app_error(connection_id, error);
                    }
                }
                Err(error) => send_protocol_error(connection_id, error),
            }
        }
    }
}

pub async fn handle_socket(
    mut socket: WebSocket,
    user_id: String,
    session_id: String,
    pool: DbPool,
    attachment_config: AttachmentConfig,
    voice_runtime: Arc<VoiceRuntime>,
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
                                    voice_runtime.as_ref(),
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

    let disconnected_channels = voice_runtime.channels_for_connection(&connection_id);
    voice_runtime.clear_connection(&connection_id).await;
    for channel in disconnected_channels {
        if let Err(error) = broadcast_voice_state_update(
            &pool,
            voice_runtime.as_ref(),
            &channel.guild_slug,
            &channel.channel_slug,
        )
        .await
        {
            tracing::debug!(
                connection_id,
                guild_slug = %channel.guild_slug,
                channel_slug = %channel.channel_slug,
                error = ?error,
                "Failed to broadcast voice state update during websocket cleanup"
            );
        }
    }
    presence_service::mark_disconnected(&user_id);
    presence_service::unregister_connection(&connection_id);
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

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

    #[test]
    fn parse_voice_join_payload_requires_non_empty_slugs() {
        let err = parse_voice_join_payload(
            json!({
                "guild_slug": " ",
                "channel_slug": "voice-room",
            }),
            "c_voice_join",
        )
        .expect_err("empty guild slug should be rejected");
        assert_eq!(err.code, "VALIDATION_ERROR");
        assert_eq!(err.details["field"], json!("guild_slug"));
    }

    #[test]
    fn parse_voice_answer_payload_requires_answer_type() {
        let err = parse_voice_answer_payload(
            json!({
                "guild_slug": "guild",
                "channel_slug": "voice-room",
                "sdp": "v=0",
                "sdp_type": "offer",
            }),
            "c_voice_answer",
        )
        .expect_err("non-answer type should be rejected");
        assert_eq!(err.code, "VALIDATION_ERROR");
        assert_eq!(err.details["field"], json!("sdp_type"));
    }

    #[test]
    fn parse_voice_candidate_payload_requires_candidate() {
        let err = parse_voice_ice_candidate_payload(
            json!({
                "guild_slug": "guild",
                "channel_slug": "voice-room",
                "candidate": "",
                "sdp_mid": "0",
                "sdp_mline_index": 0,
            }),
            "c_voice_ice_candidate",
        )
        .expect_err("candidate is required");
        assert_eq!(err.code, "VALIDATION_ERROR");
        assert_eq!(err.details["field"], json!("candidate"));
    }

    #[test]
    fn parse_voice_leave_payload_requires_non_empty_slugs() {
        let err = parse_voice_leave_payload(
            json!({
                "guild_slug": "guild",
                "channel_slug": " ",
            }),
            "c_voice_leave",
        )
        .expect_err("empty channel slug should be rejected");
        assert_eq!(err.code, "VALIDATION_ERROR");
        assert_eq!(err.details["field"], json!("channel_slug"));
    }

    #[test]
    fn parse_voice_state_update_payload_requires_boolean_flags() {
        let err = parse_voice_state_update_payload(
            json!({
                "guild_slug": "guild",
                "channel_slug": "voice-room",
                "is_muted": "yes",
                "is_deafened": false,
                "is_speaking": false,
            }),
            "c_voice_state_update",
        )
        .expect_err("invalid voice state update payload should be rejected");
        assert_eq!(err.code, "VALIDATION_ERROR");
        assert_eq!(err.details["reason"], json!("invalid_payload_shape"));
    }

    #[test]
    fn switched_channels_to_rebroadcast_filters_to_previous_same_guild_channels() {
        let channels = vec![
            VoiceChannelRef {
                guild_slug: "guild".to_string(),
                channel_slug: "voice-a".to_string(),
            },
            VoiceChannelRef {
                guild_slug: "guild".to_string(),
                channel_slug: "voice-b".to_string(),
            },
            VoiceChannelRef {
                guild_slug: "other-guild".to_string(),
                channel_slug: "voice-z".to_string(),
            },
        ];

        assert_eq!(
            switched_channels_to_rebroadcast(channels, "guild", "voice-b"),
            vec![VoiceChannelRef {
                guild_slug: "guild".to_string(),
                channel_slug: "voice-a".to_string(),
            }]
        );
    }
}
