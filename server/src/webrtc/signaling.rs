use serde::Serialize;
use webrtc::ice_transport::ice_server::RTCIceServer;

#[derive(Debug, Clone, Serialize)]
pub struct VoiceOfferPayload {
    pub guild_slug: String,
    pub channel_slug: String,
    pub sdp: String,
    pub sdp_type: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub struct VoiceIceCandidatePayload {
    pub guild_slug: String,
    pub channel_slug: String,
    pub candidate: String,
    pub sdp_mid: Option<String>,
    pub sdp_mline_index: Option<u16>,
}

#[derive(Debug, Clone, Serialize)]
pub struct VoiceConnectionStatePayload {
    pub guild_slug: String,
    pub channel_slug: String,
    pub state: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub struct VoiceParticipantPayload {
    pub user_id: String,
    pub username: String,
    pub display_name: Option<String>,
    pub avatar_color: Option<String>,
    pub is_muted: bool,
    pub is_deafened: bool,
    pub is_speaking: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct VoiceStateUpdatePayload {
    pub guild_slug: String,
    pub channel_slug: String,
    pub participant_count: u32,
    pub participants: Vec<VoiceParticipantPayload>,
}

pub fn build_offer_sdp(session_id: &str, ice_servers: &[RTCIceServer]) -> String {
    let ice_ufrag = format!("dc{:08x}", crc32(session_id.as_bytes()));
    let ice_pwd = format!("discool{:016x}", crc32(session_id.as_bytes()) as u64);
    let relay_hint = if ice_servers.iter().any(|server| {
        server
            .urls
            .iter()
            .any(|url| url.starts_with("turn:") || url.starts_with("turns:"))
    }) {
        "a=ice-options:trickle renomination\r\n"
    } else {
        "a=ice-options:trickle\r\n"
    };
    format!(
        "v=0\r\n\
o=- 0 2 IN IP4 127.0.0.1\r\n\
s=Discool Voice\r\n\
t=0 0\r\n\
a=group:BUNDLE 0\r\n\
a=msid-semantic: WMS\r\n\
m=audio 9 UDP/TLS/RTP/SAVPF 111\r\n\
c=IN IP4 0.0.0.0\r\n\
a=rtcp:9 IN IP4 0.0.0.0\r\n\
a=mid:0\r\n\
a=sendrecv\r\n\
a=rtcp-mux\r\n\
a=rtpmap:111 opus/48000/2\r\n\
a=fmtp:111 minptime=10;useinbandfec=1\r\n\
a=setup:actpass\r\n\
a=ice-ufrag:{ice_ufrag}\r\n\
a=ice-pwd:{ice_pwd}\r\n\
{relay_hint}\
a=fingerprint:sha-256 59:28:07:C8:AF:6E:B7:1A:2D:9C:43:7A:3F:7A:86:66:73:15:F7:56:2F:77:BC:11:5E:68:4F:41:27:BC:95:3A\r\n"
    )
}

pub fn default_server_candidate(guild_slug: &str, channel_slug: &str) -> VoiceIceCandidatePayload {
    VoiceIceCandidatePayload {
        guild_slug: guild_slug.to_string(),
        channel_slug: channel_slug.to_string(),
        candidate: "candidate:1 1 udp 2130706431 127.0.0.1 9 typ host".to_string(),
        sdp_mid: Some("0".to_string()),
        sdp_mline_index: Some(0),
    }
}

fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = 0xffff_ffff_u32;
    for byte in bytes {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            let mask = (crc & 1).wrapping_neg();
            crc = (crc >> 1) ^ (0xedb8_8320 & mask);
        }
    }
    !crc
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offer_sdp_contains_secure_transport_markers() {
        let sdp = build_offer_sdp("session-1", &[]);
        assert!(sdp.contains("UDP/TLS/RTP/SAVPF"));
        assert!(sdp.contains("a=fingerprint:sha-256"));
        assert!(sdp.contains("a=setup:actpass"));
        assert!(sdp.contains("a=ice-ufrag:"));
        assert!(sdp.contains("a=ice-pwd:"));
    }
}
