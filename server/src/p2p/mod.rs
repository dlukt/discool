pub mod identity;
pub mod node;

#[derive(Debug, Clone, Default)]
pub struct P2pMetadata {
    pub peer_id: Option<String>,
    pub listen_addrs: Vec<String>,
}
