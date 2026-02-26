use std::time::Duration;

use chrono::Utc;
use libp2p::multiaddr::Protocol;
use libp2p::swarm::NetworkBehaviour;
use libp2p::{Multiaddr, PeerId, StreamProtocol, identify, kad};
use rand::Rng;
use serde::{Deserialize, Serialize};

use crate::db::DbPool;

#[derive(NetworkBehaviour)]
#[behaviour(to_swarm = "DiscoveryEvent")]
pub struct DiscoveryBehaviour {
    pub identify: identify::Behaviour,
    pub kademlia: kad::Behaviour<kad::store::MemoryStore>,
}

#[derive(Debug)]
pub enum DiscoveryEvent {
    Identify(Box<identify::Event>),
    Kademlia(Box<kad::Event>),
}

impl From<identify::Event> for DiscoveryEvent {
    fn from(value: identify::Event) -> Self {
        DiscoveryEvent::Identify(Box::new(value))
    }
}

impl From<kad::Event> for DiscoveryEvent {
    fn from(value: kad::Event) -> Self {
        DiscoveryEvent::Kademlia(Box::new(value))
    }
}

#[derive(Debug, Clone)]
pub struct BootstrapPeer {
    pub peer_id: PeerId,
    pub dial_addr: Multiaddr,
    pub kad_addr: Multiaddr,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalInstanceRecord {
    pub peer_id: String,
    pub instance_name: String,
    pub instance_version: String,
    pub addresses: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct DiscoveredInstance {
    pub peer_id: String,
    pub instance_name: Option<String>,
    pub instance_version: Option<String>,
    pub addresses: Vec<String>,
}

pub fn build_discovery_behaviour(
    local_peer_id: PeerId,
    local_public_key: libp2p::identity::PublicKey,
) -> DiscoveryBehaviour {
    let identify_config = identify::Config::new("/discool/1.0.0".to_string(), local_public_key)
        .with_agent_version(format!("discool/{}", env!("CARGO_PKG_VERSION")));
    let identify = identify::Behaviour::new(identify_config);

    let mut kad_config = kad::Config::new(StreamProtocol::new("/discool/kad/1.0.0"));
    kad_config.set_query_timeout(Duration::from_secs(30));
    let store = kad::store::MemoryStore::new(local_peer_id);
    let mut kademlia = kad::Behaviour::with_config(local_peer_id, store, kad_config);
    kademlia.set_mode(Some(kad::Mode::Server));

    DiscoveryBehaviour { identify, kademlia }
}

pub fn parse_bootstrap_peers(values: &[String]) -> Result<Vec<BootstrapPeer>, String> {
    values
        .iter()
        .map(|value| parse_bootstrap_peer(value))
        .collect()
}

pub fn parse_bootstrap_peer(value: &str) -> Result<BootstrapPeer, String> {
    let trimmed = value.trim();
    let dial_addr: Multiaddr = trimmed.parse().map_err(|err: libp2p::multiaddr::Error| {
        format!("invalid bootstrap multiaddr '{trimmed}': {err}")
    })?;
    let mut kad_addr = Multiaddr::empty();
    let mut peer_id: Option<PeerId> = None;

    for protocol in dial_addr.iter() {
        match protocol {
            Protocol::P2p(parsed_peer_id) => {
                peer_id = Some(parsed_peer_id);
            }
            other => kad_addr.push(other),
        }
    }

    let peer_id = peer_id.ok_or_else(|| {
        format!("bootstrap multiaddr '{trimmed}' must include /p2p/<peer-id> protocol")
    })?;
    if kad_addr.is_empty() {
        return Err(format!(
            "bootstrap multiaddr '{trimmed}' is missing transport address before /p2p"
        ));
    }

    Ok(BootstrapPeer {
        peer_id,
        dial_addr,
        kad_addr,
    })
}

pub fn local_record_key(peer_id: &PeerId) -> kad::RecordKey {
    kad::RecordKey::new(&format!("/discool/instances/{peer_id}"))
}

pub fn decode_local_instance_record(value: &[u8]) -> Option<LocalInstanceRecord> {
    serde_json::from_slice(value).ok()
}

pub fn backoff_delay(
    attempt: u32,
    initial_secs: u64,
    max_secs: u64,
    jitter_millis: u64,
) -> Duration {
    let exp = attempt.saturating_sub(1).min(16);
    let multiplier = 1u64 << exp;
    let initial = initial_secs.max(1);
    let max = max_secs.max(initial);
    let base_secs = initial.saturating_mul(multiplier).min(max);
    let jitter = if jitter_millis == 0 {
        0
    } else {
        rand::rng().random_range(0..=jitter_millis)
    };
    Duration::from_secs(base_secs).saturating_add(Duration::from_millis(jitter))
}

pub async fn load_instance_name(pool: &DbPool) -> Result<Option<String>, String> {
    match pool {
        DbPool::Postgres(pool) => sqlx::query_scalar::<_, String>(
            "SELECT value FROM instance_settings WHERE key = 'instance_name' LIMIT 1",
        )
        .fetch_optional(pool)
        .await
        .map_err(|err| err.to_string()),
        DbPool::Sqlite(pool) => sqlx::query_scalar::<_, String>(
            "SELECT value FROM instance_settings WHERE key = 'instance_name' LIMIT 1",
        )
        .fetch_optional(pool)
        .await
        .map_err(|err| err.to_string()),
    }
}

pub async fn upsert_discovered_instance(
    pool: &DbPool,
    discovered: &DiscoveredInstance,
) -> Result<(), String> {
    let addresses_json =
        serde_json::to_string(&discovered.addresses).map_err(|err| err.to_string())?;
    let now = Utc::now().to_rfc3339();

    let query = "INSERT INTO discovered_instances (peer_id, instance_name, instance_version, addresses_json, first_seen, last_seen)\nVALUES ($1, $2, $3, $4, $5, $5)\nON CONFLICT (peer_id) DO UPDATE SET instance_name = excluded.instance_name, instance_version = excluded.instance_version, addresses_json = excluded.addresses_json, last_seen = excluded.last_seen";

    match pool {
        DbPool::Postgres(pool) => {
            sqlx::query(query)
                .bind(&discovered.peer_id)
                .bind(discovered.instance_name.as_deref())
                .bind(discovered.instance_version.as_deref())
                .bind(&addresses_json)
                .bind(&now)
                .execute(pool)
                .await
                .map_err(|err| err.to_string())?;
        }
        DbPool::Sqlite(pool) => {
            sqlx::query(query)
                .bind(&discovered.peer_id)
                .bind(discovered.instance_name.as_deref())
                .bind(discovered.instance_version.as_deref())
                .bind(&addresses_json)
                .bind(&now)
                .execute(pool)
                .await
                .map_err(|err| err.to_string())?;
        }
    }

    Ok(())
}

pub async fn count_discovered_instances(pool: &DbPool) -> Result<u32, String> {
    let count = match pool {
        DbPool::Postgres(pool) => {
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM discovered_instances")
                .fetch_one(pool)
                .await
                .map_err(|err| err.to_string())?
        }
        DbPool::Sqlite(pool) => {
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM discovered_instances")
                .fetch_one(pool)
                .await
                .map_err(|err| err.to_string())?
        }
    };

    Ok(u32::try_from(count).unwrap_or(u32::MAX))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_bootstrap_peer() -> String {
        let peer_id = libp2p::identity::Keypair::generate_ed25519()
            .public()
            .to_peer_id();
        format!("/ip4/127.0.0.1/tcp/4001/p2p/{peer_id}")
    }

    #[test]
    fn parse_bootstrap_peer_requires_peer_id() {
        let err = parse_bootstrap_peer("/ip4/127.0.0.1/tcp/4001").unwrap_err();
        assert!(err.contains("/p2p/"), "unexpected error: {err}");
    }

    #[test]
    fn parse_bootstrap_peer_extracts_peer_and_kad_addr() {
        let value = valid_bootstrap_peer();
        let parsed = parse_bootstrap_peer(&value).unwrap();
        assert_eq!(
            parsed.peer_id.to_string(),
            value.rsplit('/').next().unwrap().to_string()
        );
        assert_eq!(parsed.kad_addr.to_string(), "/ip4/127.0.0.1/tcp/4001");
        assert_eq!(parsed.dial_addr.to_string(), value);
    }

    #[test]
    fn parse_bootstrap_peer_trims_whitespace() {
        let value = format!("  {}  ", valid_bootstrap_peer());
        let parsed = parse_bootstrap_peer(&value).unwrap();
        assert!(
            parsed
                .dial_addr
                .to_string()
                .starts_with("/ip4/127.0.0.1/tcp/4001/p2p/")
        );
    }

    #[test]
    fn backoff_delay_increases_and_caps() {
        let first = backoff_delay(1, 2, 30, 0);
        let second = backoff_delay(2, 2, 30, 0);
        let sixth = backoff_delay(6, 2, 30, 0);

        assert_eq!(first, Duration::from_secs(2));
        assert_eq!(second, Duration::from_secs(4));
        assert_eq!(sixth, Duration::from_secs(30));
    }
}
