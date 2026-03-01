use webrtc::ice_transport::ice_server::RTCIceServer;

use crate::config::{VoiceConfig, VoiceTurnConfig};

pub fn ice_servers_from_config(config: &VoiceConfig) -> Vec<RTCIceServer> {
    let mut servers = Vec::with_capacity(2);
    let stun_urls = config
        .stun_urls
        .iter()
        .map(|url| url.trim())
        .filter(|url| !url.is_empty())
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    if !stun_urls.is_empty() {
        servers.push(RTCIceServer {
            urls: stun_urls,
            ..Default::default()
        });
    }
    if let Some(turn) = config.turn.as_ref() {
        servers.push(turn_server(turn));
    }
    servers
}

fn turn_server(config: &VoiceTurnConfig) -> RTCIceServer {
    RTCIceServer {
        urls: vec![config.url.trim().to_string()],
        username: config.username.trim().to_string(),
        credential: config.credential.trim().to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ice_servers_include_stun_and_turn_when_configured() {
        let mut config = VoiceConfig::default();
        config.turn = Some(VoiceTurnConfig {
            url: "turn:relay.example.com:3478".to_string(),
            username: "relay-user".to_string(),
            credential: "relay-secret".to_string(),
        });

        let servers = ice_servers_from_config(&config);
        assert_eq!(servers.len(), 2);
        assert_eq!(servers[0].urls, vec!["stun:stun.l.google.com:19302"]);
        assert_eq!(servers[1].urls, vec!["turn:relay.example.com:3478"]);
        assert_eq!(servers[1].username, "relay-user");
        assert_eq!(servers[1].credential, "relay-secret");
    }
}
