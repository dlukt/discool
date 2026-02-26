pub mod discovery;
pub mod gossip;
pub mod identity;
pub mod node;

#[derive(Debug, Clone, Default)]
pub struct P2pMetadata {
    pub peer_id: Option<String>,
    pub listen_addrs: Vec<String>,
    pub discovered_instances: u32,
    pub connection_count: u32,
    pub standalone_mode: bool,
}
