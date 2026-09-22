use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
    sync::Arc,
    time::Duration,
};

use dashmap::DashMap;
use tokio::sync::Notify;
use webrtc::peer_connection::{
    MediaEngine, PeerConnection, PeerConnectionBuilder, PeerConnectionEventHandler,
    RTCConfigurationBuilder, RTCIceCandidateInit, RTCIceGatheringState, RTCIceServer,
    RTCSessionDescription, Registry, register_default_interceptors,
};

use crate::config::VoiceConfig;

use super::{
    signaling::{VoiceConnectionStatePayload, VoiceIceCandidatePayload, VoiceOfferPayload},
    turn::ice_servers_from_config,
};

/// How long an offer waits for ICE gathering before it goes out with the
/// candidates found so far, so a STUN or TURN server that never answers cannot
/// stall a join.
const ICE_GATHERING_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Clone)]
struct VoiceSession {
    peer_connection: Arc<dyn PeerConnection>,
    user_id: String,
    guild_slug: String,
    channel_slug: String,
    is_muted: bool,
    is_deafened: bool,
    is_speaking: bool,
}

// Hand-written because `dyn PeerConnection` has no `Debug` impl.
impl fmt::Debug for VoiceSession {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("VoiceSession")
            .field("user_id", &self.user_id)
            .field("guild_slug", &self.guild_slug)
            .field("channel_slug", &self.channel_slug)
            .field("is_muted", &self.is_muted)
            .field("is_deafened", &self.is_deafened)
            .field("is_speaking", &self.is_speaking)
            .finish_non_exhaustive()
    }
}

/// Relays the one peer-connection event signaling waits on. webrtc 0.21
/// reports events through a handler; 0.17 had `gathering_complete_promise`.
struct VoiceEventHandler {
    gathering_complete: Arc<Notify>,
}

#[async_trait::async_trait]
impl PeerConnectionEventHandler for VoiceEventHandler {
    async fn on_ice_gathering_state_change(&self, state: RTCIceGatheringState) {
        if state == RTCIceGatheringState::Complete {
            self.gathering_complete.notify_one();
        }
    }
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
        self.close_sessions_for_connection_guild(connection_id, guild_slug)
            .await;
        let key = session_key(connection_id, guild_slug, channel_slug);
        let gathering_complete = Arc::new(Notify::new());
        let peer_connection = create_peer_connection(
            ice_servers_from_config(&self.config),
            Arc::clone(&gathering_complete),
        )
        .await?;
        let local_description =
            match create_gathered_offer(peer_connection.as_ref(), &gathering_complete).await {
                Ok(local_description) => local_description,
                Err(error) => {
                    // Dropping a peer connection leaves its driver task and sockets
                    // running; only `close` stops them.
                    if let Err(close_error) = peer_connection.close().await {
                        tracing::debug!(
                            %connection_id,
                            error = %close_error,
                            "Failed to close voice peer connection"
                        );
                    }
                    return Err(error);
                }
            };
        let offer_sdp = local_description.sdp.trim().to_string();

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
            ..Default::default()
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

    pub async fn disconnect_user_from_channel(
        &self,
        guild_slug: &str,
        channel_slug: &str,
        user_id: &str,
    ) -> Vec<String> {
        let keys_and_connections = self
            .sessions
            .iter()
            .filter_map(|entry| {
                let value = entry.value();
                if value.guild_slug != guild_slug
                    || value.channel_slug != channel_slug
                    || value.user_id != user_id
                {
                    return None;
                }
                let key = entry.key().clone();
                let connection_id = connection_id_from_session_key(&key)?.to_string();
                Some((key, connection_id))
            })
            .collect::<Vec<_>>();

        let mut disconnected_connections = BTreeSet::new();
        for (key, connection_id) in keys_and_connections {
            self.close_session_by_key(&key, &connection_id).await;
            disconnected_connections.insert(connection_id);
        }

        disconnected_connections.into_iter().collect()
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

    async fn close_sessions_for_connection_guild(&self, connection_id: &str, guild_slug: &str) {
        let prefix = format!("{connection_id}:{guild_slug}:");
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
    gathering_complete: Arc<Notify>,
) -> Result<Arc<dyn PeerConnection>, String> {
    let mut media_engine = MediaEngine::default();
    media_engine
        .register_default_codecs()
        .map_err(|error| format!("failed to register voice codecs: {error}"))?;
    let interceptor_registry = register_default_interceptors(Registry::new(), &mut media_engine)
        .map_err(|error| format!("failed to register voice interceptors: {error}"))?;
    let peer_connection = PeerConnectionBuilder::new()
        .with_configuration(
            RTCConfigurationBuilder::new()
                .with_ice_servers(ice_servers)
                .build(),
        )
        .with_media_engine(media_engine)
        .with_interceptor_registry(interceptor_registry)
        .with_handler(Arc::new(VoiceEventHandler { gathering_complete }))
        // webrtc 0.17 gathered on every interface by itself; 0.21 binds only
        // what it is given. The wildcard expands to one socket per usable IPv4
        // interface. `[::]:0` stays out: on a host with no usable IPv6 address
        // (Docker's default bridge) it is bound verbatim, which either logs a
        // bind error on every join or advertises `::` as a host candidate.
        .with_udp_addrs(vec!["0.0.0.0:0"])
        .build()
        .await
        .map_err(|error| format!("failed to create voice peer connection: {error}"))?;
    Ok(Arc::new(peer_connection))
}

/// Creates the audio offer and waits for ICE gathering, so the returned SDP
/// already carries the server's candidates.
async fn create_gathered_offer(
    peer_connection: &dyn PeerConnection,
    gathering_complete: &Notify,
) -> Result<RTCSessionDescription, String> {
    // `RtpCodecKind` comes from `rtc`, which webrtc depends on privately and
    // does not re-export, so it is built from its W3C kind string instead.
    peer_connection
        .add_transceiver_from_kind("audio".into(), None)
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
    if tokio::time::timeout(ICE_GATHERING_TIMEOUT, gathering_complete.notified())
        .await
        .is_err()
    {
        tracing::warn!(
            timeout_secs = ICE_GATHERING_TIMEOUT.as_secs(),
            "ICE gathering did not complete in time; sending the voice offer with the candidates gathered so far"
        );
    }
    let local_description = peer_connection
        .local_description()
        .await
        .ok_or_else(|| "voice offer SDP is unavailable".to_string())?;
    if local_description.sdp.trim().is_empty() {
        return Err("voice offer SDP is empty".to_string());
    }
    Ok(local_description)
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

fn connection_id_from_session_key(session_key: &str) -> Option<&str> {
    session_key
        .split(':')
        .next()
        .filter(|value| !value.is_empty())
}

#[cfg(test)]
mod tests {
    use tokio::sync::mpsc;
    use webrtc::peer_connection::{RTCPeerConnectionIceEvent, RTCPeerConnectionState};

    use super::*;

    /// Plays the browser: forwards its gathered candidates for trickling and
    /// reports when the connection comes up.
    struct BrowserPeerHandler {
        candidates: mpsc::UnboundedSender<RTCIceCandidateInit>,
        connected: Arc<Notify>,
    }

    #[async_trait::async_trait]
    impl PeerConnectionEventHandler for BrowserPeerHandler {
        async fn on_ice_candidate(&self, event: RTCPeerConnectionIceEvent) {
            if let Ok(candidate) = event.candidate.to_json() {
                let _ = self.candidates.send(candidate);
            }
        }

        async fn on_connection_state_change(&self, state: RTCPeerConnectionState) {
            if state == RTCPeerConnectionState::Connected {
                self.connected.notify_one();
            }
        }
    }

    #[tokio::test]
    async fn answer_and_trickled_candidates_complete_the_handshake() {
        // No STUN: host candidates suffice on one machine, and the test must
        // not depend on reaching a public server.
        let runtime = VoiceRuntime::new(VoiceConfig {
            stun_urls: Vec::new(),
            ..VoiceConfig::default()
        });
        let start = runtime
            .start_signaling("conn-1", "user-1", "guild", "voice-room")
            .await
            .expect("voice signaling should start");

        let (candidate_tx, mut candidate_rx) = mpsc::unbounded_channel();
        let connected = Arc::new(Notify::new());
        let mut media_engine = MediaEngine::default();
        media_engine
            .register_default_codecs()
            .expect("browser codecs should register");
        let browser = PeerConnectionBuilder::new()
            .with_media_engine(media_engine)
            .with_handler(Arc::new(BrowserPeerHandler {
                candidates: candidate_tx,
                connected: Arc::clone(&connected),
            }))
            .with_udp_addrs(vec!["0.0.0.0:0"])
            .build()
            .await
            .expect("browser peer connection should build");
        browser
            .set_remote_description(
                RTCSessionDescription::offer(start.offer.sdp).expect("server offer should parse"),
            )
            .await
            .expect("browser should accept the server offer");
        let answer = browser
            .create_answer(None)
            .await
            .expect("browser should create an answer");
        let answer_sdp = answer.sdp.clone();
        browser
            .set_local_description(answer)
            .await
            .expect("browser should apply its answer");

        // Like a browser, answer before gathering finishes, so the server only
        // learns the browser's candidates by trickle.
        runtime
            .apply_answer("conn-1", "guild", "voice-room", &answer_sdp)
            .await
            .expect("server should accept the browser answer");
        let handshake = async {
            loop {
                tokio::select! {
                    () = connected.notified() => return,
                    Some(candidate) = candidate_rx.recv() => {
                        runtime
                            .apply_remote_candidate(
                                "conn-1",
                                "guild",
                                "voice-room",
                                &candidate.candidate,
                                Some("0"),
                                Some(0),
                            )
                            .await
                            .expect("server should accept a trickled candidate");
                    }
                }
            }
        };
        tokio::time::timeout(Duration::from_secs(10), handshake)
            .await
            .expect("browser should connect to the server peer");

        let _ = browser.close().await;
        runtime.leave_session("conn-1", "guild", "voice-room").await;
    }

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
    async fn start_signaling_switches_connection_to_latest_channel_within_guild() {
        let runtime = VoiceRuntime::new(VoiceConfig::default());
        runtime
            .start_signaling("conn-1", "user-1", "guild", "voice-a")
            .await
            .expect("initial signaling should start");
        runtime
            .start_signaling("conn-1", "user-1", "guild", "voice-b")
            .await
            .expect("switch signaling should start");

        assert!(
            runtime
                .participants_for_channel("guild", "voice-a")
                .is_empty()
        );
        let participants = runtime.participants_for_channel("guild", "voice-b");
        assert_eq!(participants.len(), 1);
        assert_eq!(participants[0].user_id, "user-1");
        assert_eq!(
            runtime.channels_for_connection("conn-1"),
            vec![VoiceChannelRef {
                guild_slug: "guild".to_string(),
                channel_slug: "voice-b".to_string(),
            }]
        );
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

    #[tokio::test]
    async fn disconnect_user_from_channel_removes_only_target_sessions() {
        let runtime = VoiceRuntime::new(VoiceConfig::default());
        runtime
            .start_signaling("conn-1", "user-1", "guild", "voice-a")
            .await
            .expect("first user session should start");
        runtime
            .start_signaling("conn-2", "user-1", "guild", "voice-a")
            .await
            .expect("second user session should start");
        runtime
            .start_signaling("conn-3", "user-2", "guild", "voice-a")
            .await
            .expect("other user session should start");
        runtime
            .start_signaling("conn-4", "user-1", "guild", "voice-b")
            .await
            .expect("other channel session should start");

        let disconnected = runtime
            .disconnect_user_from_channel("guild", "voice-a", "user-1")
            .await;
        assert_eq!(
            disconnected,
            vec!["conn-1".to_string(), "conn-2".to_string()]
        );

        let channel_a_participants = runtime.participants_for_channel("guild", "voice-a");
        assert_eq!(channel_a_participants.len(), 1);
        assert_eq!(channel_a_participants[0].user_id, "user-2");

        let channel_b_participants = runtime.participants_for_channel("guild", "voice-b");
        assert_eq!(channel_b_participants.len(), 1);
        assert_eq!(channel_b_participants[0].user_id, "user-1");

        let repeated_disconnect = runtime
            .disconnect_user_from_channel("guild", "voice-a", "user-1")
            .await;
        assert!(repeated_disconnect.is_empty());
    }
}
