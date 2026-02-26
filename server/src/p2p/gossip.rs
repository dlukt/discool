use chrono::Utc;
use libp2p::PeerId;
use libp2p::gossipsub::{self, IdentTopic, TopicHash};
use serde::{Deserialize, Serialize};

pub const GOSSIP_PROTOCOL_VERSION: &str = "1.0.0";
pub const GOSSIP_TOPIC_PREFIX: &str = "/discool/gossip/1.0.0";
pub const IDENTITY_VERIFICATION_TOPIC: &str = "/discool/gossip/1.0.0/identity-verification";
pub const GUILD_DISCOVERY_TOPIC: &str = "/discool/gossip/1.0.0/guild-discovery";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GossipTopic {
    IdentityVerification,
    GuildDiscovery,
}

impl GossipTopic {
    pub fn as_str(self) -> &'static str {
        match self {
            GossipTopic::IdentityVerification => IDENTITY_VERIFICATION_TOPIC,
            GossipTopic::GuildDiscovery => GUILD_DISCOVERY_TOPIC,
        }
    }
}

pub fn gossip_topic(topic: GossipTopic) -> IdentTopic {
    IdentTopic::new(topic.as_str())
}

pub fn gossip_topic_from_hash(topic_hash: &TopicHash) -> Option<GossipTopic> {
    match topic_hash.to_string().as_str() {
        IDENTITY_VERIFICATION_TOPIC => Some(GossipTopic::IdentityVerification),
        GUILD_DISCOVERY_TOPIC => Some(GossipTopic::GuildDiscovery),
        _ => None,
    }
}

#[derive(Debug, Clone, Copy)]
pub struct GossipMeshSettings {
    pub mesh_n_low: usize,
    pub mesh_n: usize,
    pub mesh_n_high: usize,
}

impl GossipMeshSettings {
    pub fn new(mesh_n_low: usize, mesh_n: usize, mesh_n_high: usize) -> Result<Self, String> {
        if mesh_n_low == 0 {
            return Err("gossip mesh_n_low must be >= 1".to_string());
        }
        if mesh_n == 0 {
            return Err("gossip mesh_n must be >= 1".to_string());
        }
        if mesh_n_high == 0 {
            return Err("gossip mesh_n_high must be >= 1".to_string());
        }
        if !(mesh_n_low <= mesh_n && mesh_n <= mesh_n_high) {
            return Err(
                "gossip mesh values must satisfy mesh_n_low <= mesh_n <= mesh_n_high".to_string(),
            );
        }
        Ok(Self {
            mesh_n_low,
            mesh_n,
            mesh_n_high,
        })
    }
}

impl Default for GossipMeshSettings {
    fn default() -> Self {
        Self {
            mesh_n_low: 5,
            mesh_n: 6,
            mesh_n_high: 12,
        }
    }
}

pub fn build_gossipsub_behaviour(
    local_keypair: libp2p::identity::Keypair,
    mesh_settings: GossipMeshSettings,
) -> gossipsub::Behaviour {
    let config = gossipsub::ConfigBuilder::default()
        .validation_mode(gossipsub::ValidationMode::Strict)
        .mesh_n_low(mesh_settings.mesh_n_low)
        .mesh_n(mesh_settings.mesh_n)
        .mesh_n_high(mesh_settings.mesh_n_high)
        .validate_messages()
        .build()
        .expect("gossipsub config should be valid");
    let mut behaviour = gossipsub::Behaviour::new(
        gossipsub::MessageAuthenticity::Signed(local_keypair),
        config,
    )
    .expect("gossipsub behaviour should build");

    for topic in [
        GossipTopic::IdentityVerification,
        GossipTopic::GuildDiscovery,
    ] {
        behaviour
            .subscribe(&gossip_topic(topic))
            .expect("gossipsub topic subscription should succeed");
    }

    behaviour
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GossipEnvelope {
    pub sender_peer_id: String,
    pub emitted_at: String,
    pub event: GossipEvent,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum GossipEvent {
    IdentityVerificationRequested {
        subject_did: String,
    },
    GuildDiscoveryAnnounced {
        instance_name: String,
        instance_version: String,
        addresses: Vec<String>,
    },
}

impl GossipEvent {
    pub fn topic(&self) -> GossipTopic {
        match self {
            GossipEvent::IdentityVerificationRequested { .. } => GossipTopic::IdentityVerification,
            GossipEvent::GuildDiscoveryAnnounced { .. } => GossipTopic::GuildDiscovery,
        }
    }
}

pub fn encode_gossip_envelope(envelope: &GossipEnvelope) -> Result<Vec<u8>, String> {
    serde_json::to_vec(envelope).map_err(|err| err.to_string())
}

pub fn decode_and_validate_envelope(
    data: &[u8],
    source_peer_id: Option<&PeerId>,
    topic: GossipTopic,
) -> Result<GossipEnvelope, String> {
    let envelope: GossipEnvelope =
        serde_json::from_slice(data).map_err(|err| format!("invalid gossip payload: {err}"))?;
    if envelope.sender_peer_id.trim().is_empty() {
        return Err("missing sender_peer_id in gossip payload".to_string());
    }

    let source_peer_id = source_peer_id
        .ok_or_else(|| "missing source peer id (message is likely unsigned)".to_string())?;
    if envelope.sender_peer_id != source_peer_id.to_string() {
        return Err(format!(
            "sender peer mismatch: payload='{}' source='{}'",
            envelope.sender_peer_id, source_peer_id
        ));
    }
    if envelope.event.topic() != topic {
        return Err(format!(
            "event/topic mismatch: event={:?} topic={}",
            envelope.event.topic(),
            topic.as_str()
        ));
    }

    Ok(envelope)
}

pub fn build_guild_discovery_announcement(
    sender_peer_id: &str,
    instance_name: &str,
    addresses: &[String],
) -> Result<Vec<u8>, String> {
    let envelope = GossipEnvelope {
        sender_peer_id: sender_peer_id.to_string(),
        emitted_at: Utc::now().to_rfc3339(),
        event: GossipEvent::GuildDiscoveryAnnounced {
            instance_name: instance_name.to_string(),
            instance_version: env!("CARGO_PKG_VERSION").to_string(),
            addresses: addresses.to_vec(),
        },
    };
    encode_gossip_envelope(&envelope)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn topic_constants_are_protocol_versioned() {
        assert_eq!(GOSSIP_PROTOCOL_VERSION, "1.0.0");
        assert!(IDENTITY_VERIFICATION_TOPIC.starts_with(GOSSIP_TOPIC_PREFIX));
        assert!(GUILD_DISCOVERY_TOPIC.starts_with(GOSSIP_TOPIC_PREFIX));
    }

    #[test]
    fn mesh_settings_require_valid_bounds() {
        assert!(GossipMeshSettings::new(5, 6, 12).is_ok());
        assert!(GossipMeshSettings::new(6, 5, 12).is_err());
        assert!(GossipMeshSettings::new(5, 6, 4).is_err());
    }

    #[test]
    fn envelope_decode_rejects_sender_mismatch() {
        let sender = libp2p::identity::Keypair::generate_ed25519()
            .public()
            .to_peer_id();
        let other = libp2p::identity::Keypair::generate_ed25519()
            .public()
            .to_peer_id();
        let payload = build_guild_discovery_announcement(
            &sender.to_string(),
            "node-a",
            &["/ip4/127.0.0.1/tcp/4001".to_string()],
        )
        .unwrap();

        let err = decode_and_validate_envelope(&payload, Some(&other), GossipTopic::GuildDiscovery)
            .unwrap_err();
        assert!(
            err.contains("sender peer mismatch"),
            "unexpected error: {err}"
        );
    }
}
