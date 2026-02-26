use std::path::Path;
use std::sync::{Arc, RwLock};
use std::time::Duration;

use libp2p::futures::StreamExt;
use libp2p::kad::{self, QueryResult, Quorum};
use libp2p::swarm::SwarmEvent;
use libp2p::{Multiaddr, SwarmBuilder, identify, noise, tcp, yamux};

use crate::config::P2pConfig;
use crate::db::DbPool;

use super::P2pMetadata;
use super::discovery::{
    DiscoveredInstance, DiscoveryEvent, LocalInstanceRecord, backoff_delay,
    build_discovery_behaviour, count_discovered_instances, decode_local_instance_record,
    load_instance_name, local_record_key, parse_bootstrap_peers, upsert_discovered_instance,
};
use super::identity::{IdentityError, load_or_create_identity};

pub struct P2pRuntime {
    pub peer_id: String,
    shutdown_tx: tokio::sync::watch::Sender<bool>,
    task: tokio::task::JoinHandle<()>,
}

impl P2pRuntime {
    pub async fn shutdown(self) {
        let _ = self.shutdown_tx.send(true);
        if let Err(err) = self.task.await
            && !err.is_cancelled()
        {
            tracing::warn!(error = %err, "P2P task failed during shutdown");
        }
    }
}

#[derive(Debug)]
pub enum NodeError {
    Identity(IdentityError),
    AddressParse(String),
    Transport(String),
    Listen(String),
    Discovery(String),
}

impl std::fmt::Display for NodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NodeError::Identity(err) => write!(f, "{err}"),
            NodeError::AddressParse(msg) => write!(f, "failed to parse listen address: {msg}"),
            NodeError::Transport(msg) => write!(f, "failed to build libp2p transport: {msg}"),
            NodeError::Listen(msg) => write!(f, "failed to start libp2p listener: {msg}"),
            NodeError::Discovery(msg) => write!(f, "failed to configure p2p discovery: {msg}"),
        }
    }
}

impl std::error::Error for NodeError {}

impl From<IdentityError> for NodeError {
    fn from(value: IdentityError) -> Self {
        NodeError::Identity(value)
    }
}

pub async fn bootstrap(
    config: &P2pConfig,
    pool: DbPool,
    metadata: Arc<RwLock<P2pMetadata>>,
) -> Result<P2pRuntime, NodeError> {
    let identity = load_or_create_identity(Path::new(config.identity_key_path.as_str()))?;
    let peer_id = identity.peer_id;
    let bootstrap_peers =
        parse_bootstrap_peers(&config.bootstrap_peers).map_err(NodeError::Discovery)?;

    let listen_addr: Multiaddr = format!("/ip4/{}/tcp/{}", config.listen_host, config.listen_port)
        .parse()
        .map_err(|err: libp2p::multiaddr::Error| NodeError::AddressParse(err.to_string()))?;

    let mut swarm = SwarmBuilder::with_existing_identity(identity.keypair)
        .with_tokio()
        .with_tcp(
            tcp::Config::default(),
            noise::Config::new,
            yamux::Config::default,
        )
        .map_err(|err| NodeError::Transport(err.to_string()))?
        .with_behaviour(|keypair| {
            build_discovery_behaviour(keypair.public().to_peer_id(), keypair.public().clone())
        })
        .map_err(|err| NodeError::Transport(err.to_string()))?
        .build();

    swarm
        .listen_on(listen_addr.clone())
        .map_err(|err| NodeError::Listen(err.to_string()))?;

    let peer_id_text = peer_id.to_string();
    let discovered_instances = count_discovered_instances(&pool).await.unwrap_or(0);
    {
        let mut guard = metadata
            .write()
            .expect("p2p metadata lock poisoned before startup");
        guard.peer_id = Some(peer_id_text.clone());
        guard.listen_addrs = vec![listen_addr.to_string()];
        guard.discovered_instances = discovered_instances;
        guard.connection_count = 0;
        guard.standalone_mode = bootstrap_peers.is_empty();
    }

    for bootstrap_peer in &bootstrap_peers {
        swarm
            .behaviour_mut()
            .kademlia
            .add_address(&bootstrap_peer.peer_id, bootstrap_peer.kad_addr.clone());
        match swarm.dial(bootstrap_peer.dial_addr.clone()) {
            Ok(()) => tracing::info!(
                peer_id = %bootstrap_peer.peer_id,
                address = %bootstrap_peer.dial_addr,
                "Dialing bootstrap peer"
            ),
            Err(err) => tracing::warn!(
                peer_id = %bootstrap_peer.peer_id,
                address = %bootstrap_peer.dial_addr,
                error = %err,
                "Failed to dial bootstrap peer"
            ),
        }
    }

    if !bootstrap_peers.is_empty()
        && let Err(err) = swarm.behaviour_mut().kademlia.bootstrap()
    {
        tracing::warn!(
            error = %err,
            "Initial DHT bootstrap query failed; retry loop will continue"
        );
    }

    tracing::info!(
        peer_id = %peer_id_text,
        listen_addr = %listen_addr,
        "P2P startup initialized"
    );

    let (shutdown_tx, mut shutdown_rx) = tokio::sync::watch::channel(false);
    let task_metadata = Arc::clone(&metadata);
    let task_peer_id = peer_id_text.clone();
    let task_pool = pool.clone();
    let retry_initial_secs = config.discovery_retry_initial_secs;
    let retry_max_secs = config.discovery_retry_max_secs;
    let retry_jitter_millis = config.discovery_retry_jitter_millis;
    let refresh_interval = Duration::from_secs(config.discovery_refresh_interval_secs);
    let task = tokio::spawn(async move {
        let mut refresh_tick = tokio::time::interval(refresh_interval);
        refresh_tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        let mut retry_attempt = 0u32;
        let mut next_retry = tokio::time::Instant::now();
        let mut startup_discovery_logged = false;
        let startup_deadline = tokio::time::Instant::now() + Duration::from_secs(60);
        let mut instance_name = load_instance_name(&task_pool)
            .await
            .ok()
            .flatten()
            .unwrap_or_else(|| "Unnamed Discool Instance".to_string());

        loop {
            tokio::select! {
                changed = shutdown_rx.changed() => {
                    if changed.is_err() || *shutdown_rx.borrow() {
                        break;
                    }
                }
                _ = refresh_tick.tick() => {
                    if let Ok(Some(name)) = load_instance_name(&task_pool).await
                        && !name.trim().is_empty()
                    {
                        instance_name = name;
                    }

                    let addresses = task_metadata
                        .read()
                        .map(|guard| guard.listen_addrs.clone())
                        .unwrap_or_default();
                    if let Err(err) = put_local_instance_record(
                        &mut swarm,
                        &task_peer_id,
                        &instance_name,
                        &addresses,
                    ) {
                        tracing::warn!(peer_id = %task_peer_id, error = %err, "Failed to publish local instance metadata to DHT");
                    }

                    let local_peer_id = *swarm.local_peer_id();
                    swarm
                        .behaviour_mut()
                        .kademlia
                        .get_closest_peers(local_peer_id);

                    if !bootstrap_peers.is_empty()
                        && swarm.connected_peers().count() == 0
                        && tokio::time::Instant::now() >= next_retry
                    {
                        for bootstrap_peer in &bootstrap_peers {
                            swarm.behaviour_mut().kademlia.add_address(
                                &bootstrap_peer.peer_id,
                                bootstrap_peer.kad_addr.clone(),
                            );
                            if let Err(err) = swarm.dial(bootstrap_peer.dial_addr.clone()) {
                                tracing::warn!(
                                    peer_id = %bootstrap_peer.peer_id,
                                    address = %bootstrap_peer.dial_addr,
                                    error = %err,
                                    "Bootstrap retry dial failed"
                                );
                            }
                        }
                        retry_attempt = retry_attempt.saturating_add(1);
                        let delay = backoff_delay(
                            retry_attempt,
                            retry_initial_secs,
                            retry_max_secs,
                            retry_jitter_millis,
                        );
                        next_retry = tokio::time::Instant::now() + delay;
                        if let Ok(mut guard) = task_metadata.write() {
                            guard.standalone_mode = true;
                        }
                        if let Err(err) = swarm.behaviour_mut().kademlia.bootstrap() {
                            tracing::warn!(
                                peer_id = %task_peer_id,
                                error = %err,
                                retry_in_secs = delay.as_secs_f64(),
                                "DHT bootstrap retry failed; running in standalone mode"
                            );
                        } else {
                            tracing::info!(
                                peer_id = %task_peer_id,
                                retry_in_secs = delay.as_secs_f64(),
                                "DHT bootstrap retry started"
                            );
                        }
                    }
                }
                event = swarm.select_next_some() => {
                    match event {
                        SwarmEvent::NewListenAddr { address, .. } => {
                            let addr_text = address.to_string();
                            if let Ok(mut guard) = task_metadata.write()
                                && !guard.listen_addrs.iter().any(|a| a == &addr_text)
                            {
                                guard.listen_addrs.push(addr_text.clone());
                            }
                            tracing::info!(peer_id = %task_peer_id, listen_addr = %addr_text, "P2P listening on address");
                        }
                        SwarmEvent::ExpiredListenAddr { address, .. } => {
                            let addr_text = address.to_string();
                            if let Ok(mut guard) = task_metadata.write() {
                                guard.listen_addrs.retain(|a| a != &addr_text);
                            }
                            tracing::info!(peer_id = %task_peer_id, listen_addr = %addr_text, "P2P listener address expired");
                        }
                        SwarmEvent::ListenerError { error, .. } => {
                            tracing::warn!(peer_id = %task_peer_id, error = %error, "P2P listener error");
                        }
                        SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                            let connection_count = u32::try_from(swarm.connected_peers().count()).unwrap_or(u32::MAX);
                            retry_attempt = 0;
                            next_retry = tokio::time::Instant::now();
                            if let Ok(mut guard) = task_metadata.write() {
                                guard.connection_count = connection_count;
                                guard.standalone_mode = false;
                            }
                            tracing::info!(peer_id = %task_peer_id, remote_peer_id = %peer_id, connection_count, "P2P connection established");
                        }
                        SwarmEvent::ConnectionClosed { peer_id, .. } => {
                            let connection_count = u32::try_from(swarm.connected_peers().count()).unwrap_or(u32::MAX);
                            if let Ok(mut guard) = task_metadata.write() {
                                guard.connection_count = connection_count;
                            }
                            tracing::info!(peer_id = %task_peer_id, remote_peer_id = %peer_id, connection_count, "P2P connection closed");
                        }
                        SwarmEvent::OutgoingConnectionError { peer_id, error, .. } => {
                            tracing::warn!(peer_id = %task_peer_id, remote_peer_id = ?peer_id, error = %error, "P2P outgoing connection error");
                        }
                        SwarmEvent::Behaviour(DiscoveryEvent::Identify(event)) => {
                            if let identify::Event::Received { peer_id, info, .. } = *event {
                                let mut addresses = Vec::with_capacity(info.listen_addrs.len());
                                for address in &info.listen_addrs {
                                    swarm
                                        .behaviour_mut()
                                        .kademlia
                                        .add_address(&peer_id, address.clone());
                                    addresses.push(address.to_string());
                                }

                                let discovered = DiscoveredInstance {
                                    peer_id: peer_id.to_string(),
                                    instance_name: None,
                                    instance_version: Some(info.agent_version.clone()),
                                    addresses,
                                };

                                if let Err(err) =
                                    upsert_discovered_instance(&task_pool, &discovered).await
                                {
                                    tracing::warn!(peer_id = %task_peer_id, remote_peer_id = %peer_id, error = %err, "Failed to persist discovered peer");
                                } else if let Ok(count) =
                                    count_discovered_instances(&task_pool).await
                                    && let Ok(mut guard) = task_metadata.write()
                                {
                                    guard.discovered_instances = count;
                                }

                                tracing::info!(peer_id = %task_peer_id, remote_peer_id = %peer_id, "Peer discovered via Identify");
                            }
                        }
                        SwarmEvent::Behaviour(DiscoveryEvent::Kademlia(event)) => {
                            if let kad::Event::OutboundQueryProgressed { result, .. } = *event {
                                match result {
                                    QueryResult::Bootstrap(Ok(_)) => {
                                        if swarm.connected_peers().count() > 0 {
                                            retry_attempt = 0;
                                            next_retry = tokio::time::Instant::now();
                                            if let Ok(mut guard) = task_metadata.write() {
                                                guard.standalone_mode = false;
                                            }
                                        }
                                        tracing::info!(
                                            peer_id = %task_peer_id,
                                            "DHT bootstrap successful"
                                        );
                                    }
                                    QueryResult::Bootstrap(Err(err)) => {
                                        if let Ok(mut guard) = task_metadata.write() {
                                            guard.standalone_mode = true;
                                        }
                                        tracing::warn!(
                                            peer_id = %task_peer_id,
                                            error = %err,
                                            "DHT bootstrap query failed; staying in standalone mode"
                                        );
                                    }
                                    QueryResult::GetClosestPeers(Ok(closest)) => {
                                        for peer_info in closest.peers {
                                            if peer_info.peer_id == *swarm.local_peer_id() {
                                                continue;
                                            }
                                            let addresses = peer_info
                                                .addrs
                                                .iter()
                                                .map(ToString::to_string)
                                                .collect();
                                            let discovered = DiscoveredInstance {
                                                peer_id: peer_info.peer_id.to_string(),
                                                instance_name: None,
                                                instance_version: None,
                                                addresses,
                                            };
                                            if let Err(err) =
                                                upsert_discovered_instance(&task_pool, &discovered)
                                                    .await
                                            {
                                                tracing::warn!(peer_id = %task_peer_id, remote_peer_id = %peer_info.peer_id, error = %err, "Failed to persist closest peer discovery");
                                            }
                                        }
                                        if let Ok(count) = count_discovered_instances(&task_pool).await
                                            && let Ok(mut guard) = task_metadata.write()
                                        {
                                            guard.discovered_instances = count;
                                        }
                                    }
                                    QueryResult::GetRecord(Ok(kad::GetRecordOk::FoundRecord(
                                        record,
                                    ))) => {
                                        if let Some(decoded) =
                                            decode_local_instance_record(&record.record.value)
                                        {
                                            persist_local_record(&task_pool, decoded, &task_metadata)
                                                .await;
                                        }
                                    }
                                    QueryResult::PutRecord(Ok(_)) => {
                                        tracing::info!(peer_id = %task_peer_id, "Local instance metadata published to DHT");
                                    }
                                    _ => {}
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }

            if !startup_discovery_logged && tokio::time::Instant::now() >= startup_deadline {
                startup_discovery_logged = true;
                let discovered = task_metadata
                    .read()
                    .map(|guard| guard.discovered_instances)
                    .unwrap_or_default();
                tracing::info!(
                    peer_id = %task_peer_id,
                    discovered_instances = discovered,
                    "P2P discovery startup window reached (60s)"
                );
            }
        }

        tracing::info!(peer_id = %task_peer_id, "P2P swarm task stopped");
    });

    Ok(P2pRuntime {
        peer_id: peer_id_text,
        shutdown_tx,
        task,
    })
}

fn put_local_instance_record(
    swarm: &mut libp2p::Swarm<super::discovery::DiscoveryBehaviour>,
    peer_id: &str,
    instance_name: &str,
    addresses: &[String],
) -> Result<(), String> {
    let record = LocalInstanceRecord {
        peer_id: peer_id.to_string(),
        instance_name: instance_name.to_string(),
        instance_version: env!("CARGO_PKG_VERSION").to_string(),
        addresses: addresses.to_vec(),
    };
    let encoded = serde_json::to_vec(&record).map_err(|err| err.to_string())?;
    let record = kad::Record {
        key: local_record_key(swarm.local_peer_id()),
        value: encoded,
        publisher: Some(*swarm.local_peer_id()),
        expires: None,
    };
    swarm
        .behaviour_mut()
        .kademlia
        .put_record(record, Quorum::One)
        .map_err(|err| err.to_string())?;
    Ok(())
}

async fn persist_local_record(
    pool: &DbPool,
    decoded: LocalInstanceRecord,
    metadata: &Arc<RwLock<P2pMetadata>>,
) {
    let discovered = DiscoveredInstance {
        peer_id: decoded.peer_id,
        instance_name: Some(decoded.instance_name),
        instance_version: Some(decoded.instance_version),
        addresses: decoded.addresses,
    };
    if let Err(err) = upsert_discovered_instance(pool, &discovered).await {
        tracing::warn!(
            peer_id = %discovered.peer_id,
            error = %err,
            "Failed to persist DHT-discovered instance record"
        );
        return;
    }
    if let Ok(count) = count_discovered_instances(pool).await
        && let Ok(mut guard) = metadata.write()
    {
        guard.discovered_instances = count;
    }
}
