use std::path::Path;
use std::sync::{Arc, RwLock};

use libp2p::futures::StreamExt;
use libp2p::swarm::SwarmEvent;
use libp2p::{Multiaddr, SwarmBuilder, noise, swarm::dummy, tcp, yamux};

use crate::config::P2pConfig;

use super::P2pMetadata;
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
}

impl std::fmt::Display for NodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NodeError::Identity(err) => write!(f, "{err}"),
            NodeError::AddressParse(msg) => write!(f, "failed to parse listen address: {msg}"),
            NodeError::Transport(msg) => write!(f, "failed to build libp2p transport: {msg}"),
            NodeError::Listen(msg) => write!(f, "failed to start libp2p listener: {msg}"),
        }
    }
}

impl std::error::Error for NodeError {}

impl From<IdentityError> for NodeError {
    fn from(value: IdentityError) -> Self {
        NodeError::Identity(value)
    }
}

pub fn bootstrap(
    config: &P2pConfig,
    metadata: Arc<RwLock<P2pMetadata>>,
) -> Result<P2pRuntime, NodeError> {
    let identity = load_or_create_identity(Path::new(config.identity_key_path.as_str()))?;
    let peer_id = identity.peer_id;

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
        .with_behaviour(|_| dummy::Behaviour)
        .map_err(|err| NodeError::Transport(err.to_string()))?
        .build();

    swarm
        .listen_on(listen_addr.clone())
        .map_err(|err| NodeError::Listen(err.to_string()))?;

    let peer_id_text = peer_id.to_string();
    {
        let mut guard = metadata
            .write()
            .expect("p2p metadata lock poisoned before startup");
        guard.peer_id = Some(peer_id_text.clone());
        guard.listen_addrs = vec![listen_addr.to_string()];
    }

    tracing::info!(
        peer_id = %peer_id_text,
        listen_addr = %listen_addr,
        "P2P startup initialized"
    );

    let (shutdown_tx, mut shutdown_rx) = tokio::sync::watch::channel(false);
    let task_metadata = Arc::clone(&metadata);
    let task_peer_id = peer_id_text.clone();
    let task = tokio::spawn(async move {
        loop {
            tokio::select! {
                changed = shutdown_rx.changed() => {
                    if changed.is_err() || *shutdown_rx.borrow() {
                        break;
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
                        _ => {}
                    }
                }
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
