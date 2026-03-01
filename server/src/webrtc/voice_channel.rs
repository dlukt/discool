use std::time::Instant;

use dashmap::DashMap;
use uuid::Uuid;

use crate::config::VoiceConfig;

use super::{
    signaling::{
        VoiceConnectionStatePayload, VoiceIceCandidatePayload, VoiceOfferPayload, build_offer_sdp,
        default_server_candidate,
    },
    turn::ice_servers_from_config,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum VoiceSessionState {
    Connecting,
    Connected,
}

#[derive(Debug, Clone)]
struct VoiceSession {
    state: VoiceSessionState,
    _id: String,
    _user_id: String,
    _guild_slug: String,
    _channel_slug: String,
    _created_at: Instant,
}

#[derive(Debug, Clone)]
pub struct SignalingStart {
    pub offer: VoiceOfferPayload,
    pub candidates: Vec<VoiceIceCandidatePayload>,
    pub connection_state: VoiceConnectionStatePayload,
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

    pub fn start_signaling(
        &self,
        connection_id: &str,
        user_id: &str,
        guild_slug: &str,
        channel_slug: &str,
    ) -> SignalingStart {
        let key = session_key(connection_id, guild_slug, channel_slug);
        let session_id = Uuid::new_v4().to_string();
        let ice_servers = ice_servers_from_config(&self.config);
        let offer_sdp = build_offer_sdp(&session_id, &ice_servers);

        self.sessions.insert(
            key,
            VoiceSession {
                state: VoiceSessionState::Connecting,
                _id: session_id,
                _user_id: user_id.to_string(),
                _guild_slug: guild_slug.to_string(),
                _channel_slug: channel_slug.to_string(),
                _created_at: Instant::now(),
            },
        );

        SignalingStart {
            offer: VoiceOfferPayload {
                guild_slug: guild_slug.to_string(),
                channel_slug: channel_slug.to_string(),
                sdp: offer_sdp,
                sdp_type: "offer",
            },
            candidates: vec![default_server_candidate(guild_slug, channel_slug)],
            connection_state: VoiceConnectionStatePayload {
                guild_slug: guild_slug.to_string(),
                channel_slug: channel_slug.to_string(),
                state: "connecting",
            },
        }
    }

    pub fn apply_answer(
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
        let mut session = self
            .sessions
            .get_mut(&key)
            .ok_or_else(|| "Voice session not found. Rejoin the voice channel.".to_string())?;
        session.state = VoiceSessionState::Connected;
        Ok(VoiceConnectionStatePayload {
            guild_slug: guild_slug.to_string(),
            channel_slug: channel_slug.to_string(),
            state: "connected",
        })
    }

    pub fn apply_remote_candidate(
        &self,
        connection_id: &str,
        guild_slug: &str,
        channel_slug: &str,
    ) -> Result<(), String> {
        let key = session_key(connection_id, guild_slug, channel_slug);
        if self.sessions.contains_key(&key) {
            return Ok(());
        }
        Err("Voice session not found. Rejoin the voice channel.".to_string())
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
}

fn session_key(connection_id: &str, guild_slug: &str, channel_slug: &str) -> String {
    format!("{connection_id}:{guild_slug}:{channel_slug}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn start_signaling_returns_offer_and_connecting_state() {
        let runtime = VoiceRuntime::new(VoiceConfig::default());
        let start = runtime.start_signaling("conn-1", "user-1", "guild", "voice-room");
        assert_eq!(start.offer.sdp_type, "offer");
        assert_eq!(start.connection_state.state, "connecting");
        assert_eq!(start.candidates.len(), 1);
    }

    #[test]
    fn apply_answer_requires_existing_session() {
        let runtime = VoiceRuntime::new(VoiceConfig::default());
        let err = runtime
            .apply_answer("conn-1", "guild", "voice-room", "v=0")
            .expect_err("answer should fail without voice session");
        assert!(err.contains("Voice session not found"));
    }
}
