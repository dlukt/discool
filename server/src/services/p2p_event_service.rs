use crate::p2p::gossip::{GossipEnvelope, GossipEvent, GossipTopic};

pub async fn handle_gossip_event(
    topic: GossipTopic,
    envelope: GossipEnvelope,
) -> Result<(), String> {
    match envelope.event {
        GossipEvent::IdentityVerificationRequested { subject_did } => {
            tracing::info!(
                sender_peer_id = %envelope.sender_peer_id,
                emitted_at = %envelope.emitted_at,
                topic = %topic.as_str(),
                subject_did = %subject_did,
                "Received identity verification gossip event"
            );
        }
        GossipEvent::GuildDiscoveryAnnounced {
            instance_name,
            instance_version,
            addresses,
        } => {
            tracing::info!(
                sender_peer_id = %envelope.sender_peer_id,
                emitted_at = %envelope.emitted_at,
                topic = %topic.as_str(),
                instance_name = %instance_name,
                instance_version = %instance_version,
                address_count = addresses.len(),
                "Received guild discovery gossip event"
            );
        }
    }
    Ok(())
}
