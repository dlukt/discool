use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

use dashmap::DashMap;
use webrtc::{
    api::{
        APIBuilder, interceptor_registry::register_default_interceptors, media_engine::MediaEngine,
    },
    ice_transport::{ice_candidate::RTCIceCandidateInit, ice_server::RTCIceServer},
    interceptor::registry::Registry,
    peer_connection::{
        RTCPeerConnection, configuration::RTCConfiguration,
        sdp::session_description::RTCSessionDescription,
    },
    rtp_transceiver::rtp_codec::RTPCodecType,
};

use crate::config::VoiceConfig;

use super::{
    signaling::{VoiceConnectionStatePayload, VoiceIceCandidatePayload, VoiceOfferPayload},
    turn::ice_servers_from_config,
};

#[derive(Debug, Clone)]
struct VoiceSession {
    peer_connection: Arc<RTCPeerConnection>,
    user_id: String,
    guild_slug: String,
    channel_slug: String,
    is_muted: bool,
    is_deafened: bool,
    is_speaking: bool,
}

#[derive(Debug, Clone)]
pub struct SignalingStart {
    pub offer: VoiceOfferPayload,
    pub candidates: Vec<VoiceIceCandidatePayload>,
    pub connection_state: VoiceConnectionStatePayload,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VoiceParticipantState {
    pub user_id: String,
    pub is_muted: bool,
    pub is_deafened: bool,
    pub is_speaking: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VoiceParticipantStateUpdate {
    pub is_muted: bool,
    pub is_deafened: bool,
    pub is_speaking: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VoiceChannelRef {
    pub guild_slug: String,
    pub channel_slug: String,
}

#[derive(Debug)]
pub struct VoiceRuntime {
    config: VoiceConfig,
    sessions: DashMap<String, VoiceSession>,
}

impl VoiceRuntime {
    pub fn new(config: VoiceConfig) -> Self {
        Self {
            config,
            sessions: DashMap::new(),
        }
    }

    pub async fn start_signaling(
        &self,
        connection_id: &str,
        user_id: &str,
        guild_slug: &str,
        channel_slug: &str,
    ) -> Result<SignalingStart, String> {
        let key = session_key(connection_id, guild_slug, channel_slug);
        self.close_session_by_key(&key, connection_id).await;
        let peer_connection = create_peer_connection(ice_servers_from_config(&self.config)).await?;
        peer_connection
            .add_transceiver_from_kind(RTPCodecType::Audio, None)
            .await
            .map_err(|error| format!("failed to configure server audio transceiver: {error}"))?;
        let offer = peer_connection
            .create_offer(None)
            .await
            .map_err(|error| format!("failed to create voice offer: {error}"))?;
        peer_connection
            .set_local_description(offer)
            .await
            .map_err(|error| format!("failed to set voice local description: {error}"))?;
        let mut gathering_complete = peer_connection.gathering_complete_promise().await;
        let _ = gathering_complete.recv().await;
        let local_description = peer_connection
            .local_description()
            .await
            .ok_or_else(|| "voice offer SDP is unavailable".to_string())?;
        let offer_sdp = local_description.sdp.trim().to_string();
        if offer_sdp.is_empty() {
            return Err("voice offer SDP is empty".to_string());
        }

        self.sessions.insert(
            key,
            VoiceSession {
                peer_connection,
                user_id: user_id.to_string(),
                guild_slug: guild_slug.to_string(),
                channel_slug: channel_slug.to_string(),
                is_muted: false,
                is_deafened: false,
                is_speaking: false,
            },
        );

        Ok(SignalingStart {
            offer: VoiceOfferPayload {
                guild_slug: guild_slug.to_string(),
                channel_slug: channel_slug.to_string(),
                sdp: offer_sdp,
                sdp_type: "offer",
            },
            candidates: extract_candidates(guild_slug, channel_slug, &local_description.sdp),
            connection_state: VoiceConnectionStatePayload {
                guild_slug: guild_slug.to_string(),
                channel_slug: channel_slug.to_string(),
                state: "connecting",
            },
        })
    }

    pub async fn apply_answer(
        &self,
        connection_id: &str,
        guild_slug: &str,
        channel_slug: &str,
        sdp: &str,
    ) -> Result<VoiceConnectionStatePayload, String> {
        if sdp.trim().is_empty() {
            return Err("voice answer SDP cannot be empty".to_string());
        }
        let key = session_key(connection_id, guild_slug, channel_slug);
        let peer_connection = self
            .sessions
            .get(&key)
            .map(|session| Arc::clone(&session.peer_connection))
            .ok_or_else(|| "Voice session not found. Rejoin the voice channel.".to_string())?;
        let answer = RTCSessionDescription::answer(sdp.trim().to_string())
            .map_err(|error| format!("invalid voice answer SDP: {error}"))?;
        peer_connection
            .set_remote_description(answer)
            .await
            .map_err(|error| format!("failed to apply voice answer SDP: {error}"))?;
        Ok(VoiceConnectionStatePayload {
            guild_slug: guild_slug.to_string(),
            channel_slug: channel_slug.to_string(),
            state: "connecting",
        })
    }

    pub async fn apply_remote_candidate(
        &self,
        connection_id: &str,
        guild_slug: &str,
        channel_slug: &str,
        candidate: &str,
        sdp_mid: Option<&str>,
        sdp_mline_index: Option<u16>,
    ) -> Result<(), String> {
        let trimmed_candidate = candidate.trim();
        if trimmed_candidate.is_empty() {
            return Err("candidate is required".to_string());
        }
        let key = session_key(connection_id, guild_slug, channel_slug);
        let peer_connection = self
            .sessions
            .get(&key)
            .map(|session| Arc::clone(&session.peer_connection))
            .ok_or_else(|| "Voice session not found. Rejoin the voice channel.".to_string())?;
        let candidate = RTCIceCandidateInit {
            candidate: trimmed_candidate.to_string(),
            sdp_mid: sdp_mid.map(ToString::to_string),
            sdp_mline_index,
            username_fragment: None,
        };
        peer_connection
            .add_ice_candidate(candidate)
            .await
            .map_err(|error| format!("failed to apply remote ICE candidate: {error}"))?;
        Ok(())
    }

    pub async fn leave_session(&self, connection_id: &str, guild_slug: &str, channel_slug: &str) {
        let key = session_key(connection_id, guild_slug, channel_slug);
        self.close_session_by_key(&key, connection_id).await;
    }

    pub fn update_participant_state(
        &self,
        connection_id: &str,
        user_id: &str,
        guild_slug: &str,
        channel_slug: &str,
        next_state: VoiceParticipantStateUpdate,
    ) -> Result<(), String> {
        let key = session_key(connection_id, guild_slug, channel_slug);
        let mut session = self
            .sessions
            .get_mut(&key)
            .ok_or_else(|| "Voice session not found. Rejoin the voice channel.".to_string())?;
        if session.user_id != user_id {
            return Err("Voice session user mismatch".to_string());
        }
        session.is_muted = next_state.is_muted || next_state.is_deafened;
        session.is_deafened = next_state.is_deafened;
        session.is_speaking = next_state.is_speaking && !session.is_muted;
        Ok(())
    }

    pub fn participants_for_channel(
        &self,
        guild_slug: &str,
        channel_slug: &str,
    ) -> Vec<VoiceParticipantState> {
        let mut by_user: BTreeMap<String, VoiceParticipantState> = BTreeMap::new();
        for session in self.sessions.iter() {
            let value = session.value();
            if value.guild_slug != guild_slug || value.channel_slug != channel_slug {
                continue;
            }
            by_user
                .entry(value.user_id.clone())
                .and_modify(|participant| {
                    participant.is_muted &= value.is_muted;
                    participant.is_deafened &= value.is_deafened;
                    participant.is_speaking |= value.is_speaking;
                })
                .or_insert_with(|| VoiceParticipantState {
                    user_id: value.user_id.clone(),
                    is_muted: value.is_muted,
                    is_deafened: value.is_deafened,
                    is_speaking: value.is_speaking,
                });
        }
        by_user.into_values().collect()
    }

    pub fn channels_for_connection(&self, connection_id: &str) -> Vec<VoiceChannelRef> {
        let prefix = format!("{connection_id}:");
        let mut channels = BTreeSet::new();
        for session in self.sessions.iter() {
            if !session.key().starts_with(&prefix) {
                continue;
            }
            let value = session.value();
            channels.insert((value.guild_slug.clone(), value.channel_slug.clone()));
        }
        channels
            .into_iter()
            .map(|(guild_slug, channel_slug)| VoiceChannelRef {
                guild_slug,
                channel_slug,
            })
            .collect()
    }

    pub async fn clear_connection(&self, connection_id: &str) {
        let prefix = format!("{connection_id}:");
        let keys = self
            .sessions
            .iter()
            .filter_map(|entry| {
                if entry.key().starts_with(&prefix) {
                    Some(entry.key().clone())
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        for key in keys {
            self.close_session_by_key(&key, connection_id).await;
        }
    }

    pub fn retry_initial_millis(&self) -> u64 {
        self.config.retry_initial_millis
    }

    pub fn retry_max_millis(&self) -> u64 {
        self.config.retry_max_millis
    }

    pub fn retry_max_attempts(&self) -> u32 {
        self.config.retry_max_attempts
    }

    async fn close_session_by_key(&self, key: &str, connection_id: &str) {
        let Some((_, session)) = self.sessions.remove(key) else {
            return;
        };
        if let Err(error) = session.peer_connection.close().await {
            tracing::debug!(%connection_id, %error, "Failed to close voice peer connection");
        }
    }
}

async fn create_peer_connection(
    ice_servers: Vec<RTCIceServer>,
) -> Result<Arc<RTCPeerConnection>, String> {
    let mut media_engine = MediaEngine::default();
    media_engine
        .register_default_codecs()
        .map_err(|error| format!("failed to register voice codecs: {error}"))?;
    let mut interceptor_registry = Registry::new();
    interceptor_registry =
        register_default_interceptors(interceptor_registry, &mut media_engine)
            .map_err(|error| format!("failed to register voice interceptors: {error}"))?;
    let api = APIBuilder::new()
        .with_media_engine(media_engine)
        .with_interceptor_registry(interceptor_registry)
        .build();
    api.new_peer_connection(RTCConfiguration {
        ice_servers,
        ..Default::default()
    })
    .await
    .map(Arc::new)
    .map_err(|error| format!("failed to create voice peer connection: {error}"))
}

fn extract_candidates(
    guild_slug: &str,
    channel_slug: &str,
    sdp: &str,
) -> Vec<VoiceIceCandidatePayload> {
    let sdp_mid = sdp
        .lines()
        .find_map(|line| line.strip_prefix("a=mid:"))
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
        .or_else(|| Some("0".to_string()));
    sdp.lines()
        .filter_map(|line| line.strip_prefix("a=candidate:"))
        .map(str::trim)
        .filter(|candidate| !candidate.is_empty())
        .map(|candidate| VoiceIceCandidatePayload {
            guild_slug: guild_slug.to_string(),
            channel_slug: channel_slug.to_string(),
            candidate: format!("candidate:{candidate}"),
            sdp_mid: sdp_mid.clone(),
            sdp_mline_index: Some(0),
        })
        .collect()
}

fn session_key(connection_id: &str, guild_slug: &str, channel_slug: &str) -> String {
    format!("{connection_id}:{guild_slug}:{channel_slug}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn start_signaling_returns_offer_and_connecting_state() {
        let runtime = VoiceRuntime::new(VoiceConfig::default());
        let start = runtime
            .start_signaling("conn-1", "user-1", "guild", "voice-room")
            .await
            .expect("voice signaling should start");
        assert_eq!(start.offer.sdp_type, "offer");
        assert_eq!(start.connection_state.state, "connecting");
        assert!(
            !start.candidates.is_empty(),
            "voice signaling should emit at least one ICE candidate"
        );
    }

    #[tokio::test]
    async fn apply_answer_requires_existing_session() {
        let runtime = VoiceRuntime::new(VoiceConfig::default());
        let err = runtime
            .apply_answer("conn-1", "guild", "voice-room", "v=0")
            .await
            .expect_err("answer should fail without voice session");
        assert!(err.contains("Voice session not found"));
    }

    #[tokio::test]
    async fn leave_session_is_idempotent() {
        let runtime = VoiceRuntime::new(VoiceConfig::default());
        runtime.leave_session("conn-1", "guild", "voice-room").await;
        runtime.leave_session("conn-1", "guild", "voice-room").await;
        assert!(runtime.sessions.is_empty());
    }

    #[tokio::test]
    async fn participant_state_updates_clear_speaking_when_muted() {
        let runtime = VoiceRuntime::new(VoiceConfig::default());
        runtime
            .start_signaling("conn-1", "user-1", "guild", "voice-room")
            .await
            .expect("voice signaling should start");
        runtime
            .update_participant_state(
                "conn-1",
                "user-1",
                "guild",
                "voice-room",
                VoiceParticipantStateUpdate {
                    is_muted: true,
                    is_deafened: true,
                    is_speaking: true,
                },
            )
            .expect("voice participant state update should work");
        let participants = runtime.participants_for_channel("guild", "voice-room");
        assert_eq!(participants.len(), 1);
        assert_eq!(participants[0].user_id, "user-1");
        assert!(participants[0].is_muted);
        assert!(participants[0].is_deafened);
        assert!(!participants[0].is_speaking);
    }
}
