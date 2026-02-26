use std::time::Duration;

use discool_server::p2p::discovery::{
    DiscoveryBehaviour, DiscoveryEvent, build_discovery_behaviour,
};
use discool_server::p2p::gossip::{
    GossipMeshSettings, GossipTopic, build_guild_discovery_announcement,
    decode_and_validate_envelope, gossip_topic,
};
use libp2p::futures::StreamExt;
use libp2p::gossipsub::MessageAcceptance;
use libp2p::multiaddr::Protocol;
use libp2p::swarm::SwarmEvent;
use libp2p::{Multiaddr, PeerId, Swarm, SwarmBuilder, noise, tcp, yamux};

fn build_swarm() -> (Swarm<DiscoveryBehaviour>, PeerId) {
    let keypair = libp2p::identity::Keypair::generate_ed25519();
    let peer_id = keypair.public().to_peer_id();
    let mesh_settings = GossipMeshSettings::default();
    let swarm = SwarmBuilder::with_existing_identity(keypair)
        .with_tokio()
        .with_tcp(
            tcp::Config::default(),
            noise::Config::new,
            yamux::Config::default,
        )
        .expect("swarm transport should build")
        .with_behaviour(move |identity| build_discovery_behaviour(identity.clone(), mesh_settings))
        .expect("swarm behaviour should build")
        .build();
    (swarm, peer_id)
}

async fn wait_for_listen_addr(swarm: &mut Swarm<DiscoveryBehaviour>) -> Multiaddr {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        assert!(
            tokio::time::Instant::now() < deadline,
            "timed out waiting for listen address"
        );
        if let SwarmEvent::NewListenAddr { address, .. } = swarm.select_next_some().await {
            return address;
        }
    }
}

async fn connect_swarms(
    swarm_a: &mut Swarm<DiscoveryBehaviour>,
    peer_id_a: PeerId,
    listen_addr_a: Multiaddr,
    swarm_b: &mut Swarm<DiscoveryBehaviour>,
) {
    let dial_addr = listen_addr_a.with(Protocol::P2p(peer_id_a));
    swarm_b
        .dial(dial_addr)
        .expect("dialing first swarm should succeed");

    let deadline = tokio::time::Instant::now() + Duration::from_secs(8);
    let mut a_connected = false;
    let mut b_connected = false;
    while !(a_connected && b_connected) {
        assert!(
            tokio::time::Instant::now() < deadline,
            "timed out waiting for swarm connection"
        );
        tokio::select! {
            event = swarm_a.select_next_some() => {
                if matches!(event, SwarmEvent::ConnectionEstablished { .. }) {
                    a_connected = true;
                }
            }
            event = swarm_b.select_next_some() => {
                if matches!(event, SwarmEvent::ConnectionEstablished { .. }) {
                    b_connected = true;
                }
            }
        }
    }
}

async fn publish_when_ready(
    publisher: &mut Swarm<DiscoveryBehaviour>,
    peer: &mut Swarm<DiscoveryBehaviour>,
    topic: GossipTopic,
    payload: Vec<u8>,
) {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        match publisher
            .behaviour_mut()
            .gossipsub
            .publish(gossip_topic(topic), payload.clone())
        {
            Ok(_) => return,
            Err(err) => {
                assert!(
                    tokio::time::Instant::now() < deadline,
                    "timed out waiting for publish readiness: {err}"
                );
            }
        }
        tokio::select! {
            _ = publisher.select_next_some() => {}
            _ = peer.select_next_some() => {}
        }
    }
}

#[tokio::test]
async fn gossip_event_propagates_between_two_instances() {
    let (mut swarm_a, peer_id_a) = build_swarm();
    let (mut swarm_b, _peer_id_b) = build_swarm();

    swarm_a
        .listen_on("/ip4/127.0.0.1/tcp/0".parse().unwrap())
        .unwrap();
    let listen_addr_a = wait_for_listen_addr(&mut swarm_a).await;

    swarm_b
        .listen_on("/ip4/127.0.0.1/tcp/0".parse().unwrap())
        .unwrap();
    let _listen_addr_b = wait_for_listen_addr(&mut swarm_b).await;

    connect_swarms(&mut swarm_a, peer_id_a, listen_addr_a, &mut swarm_b).await;

    let payload = build_guild_discovery_announcement(
        swarm_a.local_peer_id().to_string().as_str(),
        "node-a",
        &["/ip4/127.0.0.1/tcp/4001".to_string()],
    )
    .unwrap();
    publish_when_ready(
        &mut swarm_a,
        &mut swarm_b,
        GossipTopic::GuildDiscovery,
        payload,
    )
    .await;

    let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
    loop {
        assert!(
            tokio::time::Instant::now() < deadline,
            "timed out waiting for propagated gossip message"
        );
        tokio::select! {
            _ = swarm_a.select_next_some() => {}
            event = swarm_b.select_next_some() => {
                if let SwarmEvent::Behaviour(DiscoveryEvent::Gossipsub(event)) = event
                    && let libp2p::gossipsub::Event::Message { propagation_source, message_id, message } = *event
                {
                    let envelope = decode_and_validate_envelope(
                        &message.data,
                        message.source.as_ref(),
                        GossipTopic::GuildDiscovery,
                    )
                    .unwrap();
                    assert_eq!(envelope.sender_peer_id, swarm_a.local_peer_id().to_string());
                    let _ = swarm_b.behaviour_mut().gossipsub.report_message_validation_result(
                        &message_id,
                        &propagation_source,
                        MessageAcceptance::Accept,
                    );
                    return;
                }
            }
        }
    }
}

#[tokio::test]
async fn invalid_gossip_payload_is_rejected() {
    let (mut swarm_a, peer_id_a) = build_swarm();
    let (mut swarm_b, _peer_id_b) = build_swarm();

    swarm_a
        .listen_on("/ip4/127.0.0.1/tcp/0".parse().unwrap())
        .unwrap();
    let listen_addr_a = wait_for_listen_addr(&mut swarm_a).await;

    swarm_b
        .listen_on("/ip4/127.0.0.1/tcp/0".parse().unwrap())
        .unwrap();
    let _listen_addr_b = wait_for_listen_addr(&mut swarm_b).await;

    connect_swarms(&mut swarm_a, peer_id_a, listen_addr_a, &mut swarm_b).await;

    publish_when_ready(
        &mut swarm_a,
        &mut swarm_b,
        GossipTopic::GuildDiscovery,
        b"not-json".to_vec(),
    )
    .await;

    let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
    loop {
        assert!(
            tokio::time::Instant::now() < deadline,
            "timed out waiting for invalid gossip message"
        );
        tokio::select! {
            _ = swarm_a.select_next_some() => {}
            event = swarm_b.select_next_some() => {
                if let SwarmEvent::Behaviour(DiscoveryEvent::Gossipsub(event)) = event
                    && let libp2p::gossipsub::Event::Message { propagation_source, message_id, message } = *event
                {
                    let err = decode_and_validate_envelope(
                        &message.data,
                        message.source.as_ref(),
                        GossipTopic::GuildDiscovery,
                    )
                    .unwrap_err();
                    assert!(
                        err.contains("invalid gossip payload"),
                        "unexpected error: {err}"
                    );
                    let _ = swarm_b.behaviour_mut().gossipsub.report_message_validation_result(
                        &message_id,
                        &propagation_source,
                        MessageAcceptance::Reject,
                    );
                    return;
                }
            }
        }
    }
}
