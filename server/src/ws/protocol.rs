use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

pub const PROTOCOL_VERSION: &str = "1";

pub const STORY_6_1_SERVER_EVENTS: [&str; 7] = [
    "message_create",
    "message_update",
    "message_delete",
    "typing_start",
    "presence_update",
    "guild_update",
    "channel_update",
];

#[derive(Debug, Clone, Deserialize)]
pub struct ClientEnvelope {
    pub op: String,
    #[serde(default)]
    pub d: Value,
    #[serde(default)]
    pub s: Option<u64>,
    #[serde(default)]
    pub t: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ServerEnvelope<T> {
    pub op: &'static str,
    pub d: T,
    pub s: u64,
    pub t: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServerOp {
    Hello,
    Error,
    HeartbeatAck,
    ResumeAck,
    PresenceUpdate,
    MessageCreate,
    MessageUpdate,
    MessageDelete,
    TypingStart,
    GuildUpdate,
    ChannelUpdate,
}

impl ServerOp {
    pub const fn as_str(self) -> &'static str {
        match self {
            ServerOp::Hello => "hello",
            ServerOp::Error => "error",
            ServerOp::HeartbeatAck => "heartbeat_ack",
            ServerOp::ResumeAck => "resume_ack",
            ServerOp::PresenceUpdate => "presence_update",
            ServerOp::MessageCreate => "message_create",
            ServerOp::MessageUpdate => "message_update",
            ServerOp::MessageDelete => "message_delete",
            ServerOp::TypingStart => "typing_start",
            ServerOp::GuildUpdate => "guild_update",
            ServerOp::ChannelUpdate => "channel_update",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClientOp {
    Heartbeat,
    Subscribe,
    Unsubscribe,
    MessageCreate,
    MessageUpdate,
    MessageDelete,
    TypingStart,
    Resume,
}

#[derive(Debug, Clone)]
pub struct ProtocolError {
    pub code: &'static str,
    pub message: String,
    pub details: Value,
}

impl ProtocolError {
    pub fn validation(message: impl Into<String>) -> Self {
        Self {
            code: "VALIDATION_ERROR",
            message: message.into(),
            details: json!({}),
        }
    }

    pub fn rate_limited(op: &str, limit_per_second: u32) -> Self {
        Self {
            code: "RATE_LIMITED",
            message: "Rate limit exceeded for websocket operation".to_string(),
            details: json!({
                "op": op,
                "limit_per_second": limit_per_second,
            }),
        }
    }

    pub fn with_details(mut self, details: Value) -> Self {
        self.details = details;
        self
    }
}

pub fn parse_client_op(op: &str) -> Result<ClientOp, ProtocolError> {
    if !op.starts_with("c_") {
        return Err(
            ProtocolError::validation("Client operations must be prefixed with `c_`")
                .with_details(json!({ "op": op })),
        );
    }

    match op {
        "c_heartbeat" => Ok(ClientOp::Heartbeat),
        "c_subscribe" => Ok(ClientOp::Subscribe),
        "c_unsubscribe" => Ok(ClientOp::Unsubscribe),
        "c_message_create" => Ok(ClientOp::MessageCreate),
        "c_message_update" => Ok(ClientOp::MessageUpdate),
        "c_message_delete" => Ok(ClientOp::MessageDelete),
        "c_typing_start" => Ok(ClientOp::TypingStart),
        "c_resume" => Ok(ClientOp::Resume),
        _ => {
            Err(ProtocolError::validation("Unknown client operation")
                .with_details(json!({ "op": op })))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_client_op_requires_c_prefix() {
        let err = parse_client_op("typing_start").unwrap_err();
        assert_eq!(err.code, "VALIDATION_ERROR");
        assert_eq!(err.details["op"], json!("typing_start"));
    }

    #[test]
    fn parse_client_op_accepts_story_ops() {
        assert_eq!(parse_client_op("c_heartbeat").unwrap(), ClientOp::Heartbeat);
        assert_eq!(parse_client_op("c_subscribe").unwrap(), ClientOp::Subscribe);
        assert_eq!(
            parse_client_op("c_message_create").unwrap(),
            ClientOp::MessageCreate
        );
        assert_eq!(
            parse_client_op("c_message_update").unwrap(),
            ClientOp::MessageUpdate
        );
        assert_eq!(
            parse_client_op("c_message_delete").unwrap(),
            ClientOp::MessageDelete
        );
        assert_eq!(
            parse_client_op("c_typing_start").unwrap(),
            ClientOp::TypingStart
        );
        assert_eq!(parse_client_op("c_resume").unwrap(), ClientOp::Resume);
    }

    #[test]
    fn server_envelope_serializes_with_op_d_s_t_fields() {
        let serialized = serde_json::to_value(ServerEnvelope {
            op: ServerOp::Hello.as_str(),
            d: json!({ "ok": true }),
            s: 42,
            t: "2026-02-28T12:00:00Z".to_string(),
        })
        .unwrap();
        assert_eq!(serialized["op"], json!("hello"));
        assert_eq!(serialized["d"]["ok"], json!(true));
        assert_eq!(serialized["s"], json!(42));
        assert_eq!(serialized["t"], json!("2026-02-28T12:00:00Z"));
    }
}
