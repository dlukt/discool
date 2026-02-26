pub mod discovery;
pub mod gossip;
pub mod identity;
pub mod node;
pub mod sybil;

#[derive(Debug, Clone, Default)]
pub struct P2pMetadata {
    pub peer_id: Option<String>,
    pub discovery_enabled: Option<bool>,
    pub listen_addrs: Vec<String>,
    pub discovered_instances: u32,
    pub connection_count: u32,
    pub standalone_mode: bool,
    pub message_rate_per_minute: f64,
    pub ingress_total: u64,
    pub rejected_total: u64,
    pub throttled_total: u64,
    pub healthy_peer_count: u32,
    pub bootstrap_failures: u32,
    pub degraded: bool,
    pub degraded_reason: Option<String>,
}
