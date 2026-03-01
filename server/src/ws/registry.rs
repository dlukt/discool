use std::{
    collections::HashSet,
    sync::{
        Arc, OnceLock, RwLock,
        atomic::{AtomicU64, Ordering},
    },
    time::Instant,
};

use axum::extract::ws::Message;
use chrono::Utc;
use dashmap::DashMap;
use serde::Serialize;
use tokio::sync::mpsc;
use uuid::Uuid;

use super::protocol::{ServerEnvelope, ServerOp};

#[derive(Default)]
struct ConnectionRegistry {
    connections: DashMap<String, Arc<ConnectionState>>,
}

struct ConnectionState {
    user_id: String,
    session_id: String,
    sender: mpsc::UnboundedSender<Message>,
    guild_subscriptions: RwLock<HashSet<String>>,
    channel_subscriptions: RwLock<HashSet<String>>,
    active_channel: RwLock<Option<String>>,
    dm_subscriptions: RwLock<HashSet<String>>,
    active_dm: RwLock<Option<String>>,
    sequence: AtomicU64,
    last_heartbeat: RwLock<Instant>,
}

#[derive(Debug, Clone)]
pub struct ConnectionSnapshot {
    pub user_id: String,
    pub session_id: String,
    pub active_channel: Option<String>,
    pub active_dm: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ChannelConnectionTarget {
    pub connection_id: String,
    pub user_id: String,
}

static CONNECTION_REGISTRY: OnceLock<ConnectionRegistry> = OnceLock::new();

fn registry() -> &'static ConnectionRegistry {
    CONNECTION_REGISTRY.get_or_init(ConnectionRegistry::default)
}

fn channel_key(guild_slug: &str, channel_slug: &str) -> String {
    format!("{guild_slug}:{channel_slug}")
}

fn lock_read<T>(lock: &RwLock<T>) -> std::sync::RwLockReadGuard<'_, T> {
    lock.read().unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn lock_write<T>(lock: &RwLock<T>) -> std::sync::RwLockWriteGuard<'_, T> {
    lock.write()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

pub fn register_connection(
    user_id: &str,
    session_id: &str,
    sender: mpsc::UnboundedSender<Message>,
) -> String {
    let connection_id = Uuid::new_v4().to_string();
    let state = ConnectionState {
        user_id: user_id.to_string(),
        session_id: session_id.to_string(),
        sender,
        guild_subscriptions: RwLock::new(HashSet::new()),
        channel_subscriptions: RwLock::new(HashSet::new()),
        active_channel: RwLock::new(None),
        dm_subscriptions: RwLock::new(HashSet::new()),
        active_dm: RwLock::new(None),
        sequence: AtomicU64::new(0),
        last_heartbeat: RwLock::new(Instant::now()),
    };
    registry()
        .connections
        .insert(connection_id.clone(), Arc::new(state));
    connection_id
}

pub fn unregister_connection(connection_id: &str) {
    registry().connections.remove(connection_id);
}

pub fn websocket_connection_count() -> u32 {
    u32::try_from(registry().connections.len()).unwrap_or(u32::MAX)
}

pub fn mark_heartbeat(connection_id: &str) {
    if let Some(connection) = registry().connections.get(connection_id) {
        *lock_write(&connection.last_heartbeat) = Instant::now();
    }
}

pub fn connection_snapshot(connection_id: &str) -> Option<ConnectionSnapshot> {
    registry()
        .connections
        .get(connection_id)
        .map(|connection| ConnectionSnapshot {
            user_id: connection.user_id.clone(),
            session_id: connection.session_id.clone(),
            active_channel: lock_read(&connection.active_channel).clone(),
            active_dm: lock_read(&connection.active_dm).clone(),
        })
}

pub fn subscribe(connection_id: &str, guild_slug: &str, channel_slug: Option<&str>) {
    let Some(connection) = registry().connections.get(connection_id) else {
        return;
    };

    lock_write(&connection.guild_subscriptions).insert(guild_slug.to_string());

    if let Some(channel_slug) = channel_slug {
        let key = channel_key(guild_slug, channel_slug);
        lock_write(&connection.channel_subscriptions).insert(key.clone());
        *lock_write(&connection.active_channel) = Some(key);
    }
}

pub fn unsubscribe(connection_id: &str, guild_slug: &str, channel_slug: Option<&str>) {
    let Some(connection) = registry().connections.get(connection_id) else {
        return;
    };

    if let Some(channel_slug) = channel_slug {
        let key = channel_key(guild_slug, channel_slug);
        lock_write(&connection.channel_subscriptions).remove(&key);
        let should_clear_active = lock_read(&connection.active_channel)
            .as_ref()
            .map(|active| active == &key)
            .unwrap_or(false);
        if should_clear_active {
            *lock_write(&connection.active_channel) = None;
        }
    } else {
        lock_write(&connection.guild_subscriptions).remove(guild_slug);
        lock_write(&connection.channel_subscriptions)
            .retain(|key| !key.starts_with(&format!("{guild_slug}:")));
        let should_clear_active = lock_read(&connection.active_channel)
            .as_ref()
            .map(|active| active.starts_with(&format!("{guild_slug}:")))
            .unwrap_or(false);
        if should_clear_active {
            *lock_write(&connection.active_channel) = None;
        }
    }
}

pub fn subscribe_dm(connection_id: &str, dm_slug: &str) {
    let Some(connection) = registry().connections.get(connection_id) else {
        return;
    };

    let normalized = dm_slug.trim();
    if normalized.is_empty() {
        return;
    }
    {
        let mut subscriptions = lock_write(&connection.dm_subscriptions);
        subscriptions.clear();
        subscriptions.insert(normalized.to_string());
    }
    *lock_write(&connection.active_dm) = Some(normalized.to_string());
}

pub fn clear_dm_subscription(connection_id: &str) {
    let Some(connection) = registry().connections.get(connection_id) else {
        return;
    };
    lock_write(&connection.dm_subscriptions).clear();
    *lock_write(&connection.active_dm) = None;
}

pub fn send_event<T: Serialize>(connection_id: &str, op: ServerOp, payload: &T) {
    let Some(connection) = registry().connections.get(connection_id) else {
        return;
    };

    let sequence = connection.sequence.fetch_add(1, Ordering::Relaxed) + 1;
    let envelope = ServerEnvelope {
        op: op.as_str(),
        d: payload,
        s: sequence,
        t: Utc::now().to_rfc3339(),
    };

    match serde_json::to_string(&envelope) {
        Ok(serialized) => {
            if connection
                .sender
                .send(Message::Text(serialized.into()))
                .is_err()
            {
                drop(connection);
                unregister_connection(connection_id);
            }
        }
        Err(err) => {
            tracing::warn!(error = %err, connection_id, "Failed to serialize websocket envelope");
        }
    }
}

pub fn broadcast_global<T: Serialize>(op: ServerOp, payload: &T) {
    let targets: Vec<String> = registry()
        .connections
        .iter()
        .map(|entry| entry.key().clone())
        .collect();
    for connection_id in targets {
        send_event(&connection_id, op, payload);
    }
}

pub fn broadcast_to_guild<T: Serialize>(guild_slug: &str, op: ServerOp, payload: &T) {
    let targets: Vec<String> = registry()
        .connections
        .iter()
        .filter_map(|entry| {
            if lock_read(&entry.guild_subscriptions).contains(guild_slug) {
                Some(entry.key().clone())
            } else {
                None
            }
        })
        .collect();
    for connection_id in targets {
        send_event(&connection_id, op, payload);
    }
}

pub fn guild_connection_targets(guild_slug: &str) -> Vec<ChannelConnectionTarget> {
    registry()
        .connections
        .iter()
        .filter_map(|entry| {
            if lock_read(&entry.guild_subscriptions).contains(guild_slug) {
                Some(ChannelConnectionTarget {
                    connection_id: entry.key().clone(),
                    user_id: entry.user_id.clone(),
                })
            } else {
                None
            }
        })
        .collect()
}

pub fn broadcast_to_channel<T: Serialize>(
    guild_slug: &str,
    channel_slug: &str,
    op: ServerOp,
    payload: &T,
) {
    let key = channel_key(guild_slug, channel_slug);
    let targets: Vec<String> = registry()
        .connections
        .iter()
        .filter_map(|entry| {
            if lock_read(&entry.channel_subscriptions).contains(&key) {
                Some(entry.key().clone())
            } else {
                None
            }
        })
        .collect();
    for connection_id in targets {
        send_event(&connection_id, op, payload);
    }
}

pub fn broadcast_to_dm<T: Serialize>(dm_slug: &str, op: ServerOp, payload: &T) {
    let targets: Vec<String> = registry()
        .connections
        .iter()
        .filter_map(|entry| {
            if lock_read(&entry.dm_subscriptions).contains(dm_slug) {
                Some(entry.key().clone())
            } else {
                None
            }
        })
        .collect();
    for connection_id in targets {
        send_event(&connection_id, op, payload);
    }
}

pub fn channel_connection_targets(
    guild_slug: &str,
    channel_slug: &str,
) -> Vec<ChannelConnectionTarget> {
    let key = channel_key(guild_slug, channel_slug);
    registry()
        .connections
        .iter()
        .filter_map(|entry| {
            if lock_read(&entry.channel_subscriptions).contains(&key) {
                Some(ChannelConnectionTarget {
                    connection_id: entry.key().clone(),
                    user_id: entry.user_id.clone(),
                })
            } else {
                None
            }
        })
        .collect()
}

pub fn dm_connection_targets(dm_slug: &str) -> Vec<ChannelConnectionTarget> {
    registry()
        .connections
        .iter()
        .filter_map(|entry| {
            if lock_read(&entry.dm_subscriptions).contains(dm_slug) {
                Some(ChannelConnectionTarget {
                    connection_id: entry.key().clone(),
                    user_id: entry.user_id.clone(),
                })
            } else {
                None
            }
        })
        .collect()
}

pub fn user_connection_targets(user_ids: &[String]) -> Vec<ChannelConnectionTarget> {
    let target_users: HashSet<&str> = user_ids
        .iter()
        .map(String::as_str)
        .filter(|value| !value.trim().is_empty())
        .collect();
    if target_users.is_empty() {
        return Vec::new();
    }
    registry()
        .connections
        .iter()
        .filter_map(|entry| {
            if target_users.contains(entry.user_id.as_str()) {
                Some(ChannelConnectionTarget {
                    connection_id: entry.key().clone(),
                    user_id: entry.user_id.clone(),
                })
            } else {
                None
            }
        })
        .collect()
}

#[cfg(test)]
pub fn reset_for_tests() {
    registry().connections.clear();
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use tokio::sync::mpsc;

    use super::*;

    fn test_lock() -> &'static std::sync::Mutex<()> {
        static LOCK: std::sync::OnceLock<std::sync::Mutex<()>> = std::sync::OnceLock::new();
        LOCK.get_or_init(|| std::sync::Mutex::new(()))
    }

    fn read_text_message(receiver: &mut mpsc::UnboundedReceiver<Message>) -> serde_json::Value {
        let message = receiver.try_recv().expect("expected websocket message");
        let Message::Text(payload) = message else {
            panic!("expected text websocket message");
        };
        serde_json::from_str(payload.as_str()).expect("payload should be valid json")
    }

    #[test]
    fn send_event_assigns_monotonic_sequence_per_connection() {
        let _guard = test_lock().lock().expect("registry test lock poisoned");
        reset_for_tests();

        let (sender, mut receiver) = mpsc::unbounded_channel();
        let connection_id = register_connection("user-a", "session-a", sender);

        send_event(&connection_id, ServerOp::Hello, &json!({ "ok": true }));
        send_event(
            &connection_id,
            ServerOp::HeartbeatAck,
            &json!({ "ok": true }),
        );

        let first = read_text_message(&mut receiver);
        let second = read_text_message(&mut receiver);

        assert_eq!(first["op"], json!("hello"));
        assert_eq!(first["s"], json!(1));
        assert!(first["t"].as_str().is_some());
        assert_eq!(second["op"], json!("heartbeat_ack"));
        assert_eq!(second["s"], json!(2));
        assert!(second["t"].as_str().is_some());
    }

    #[test]
    fn channel_broadcast_targets_only_subscribed_connections() {
        let _guard = test_lock().lock().expect("registry test lock poisoned");
        reset_for_tests();

        let (sender_a, mut receiver_a) = mpsc::unbounded_channel();
        let (sender_b, mut receiver_b) = mpsc::unbounded_channel();
        let connection_a = register_connection("user-a", "session-a", sender_a);
        let connection_b = register_connection("user-b", "session-b", sender_b);

        subscribe(&connection_a, "lobby", Some("general"));
        subscribe(&connection_b, "lobby", Some("random"));

        broadcast_to_channel(
            "lobby",
            "general",
            ServerOp::TypingStart,
            &json!({ "guild_slug": "lobby", "channel_slug": "general" }),
        );

        let targeted = read_text_message(&mut receiver_a);
        assert_eq!(targeted["op"], json!("typing_start"));
        assert!(
            receiver_b.try_recv().is_err(),
            "non-subscribed connection received a channel event"
        );
    }

    #[test]
    fn dm_broadcast_targets_only_dm_subscribed_connections() {
        let _guard = test_lock().lock().expect("registry test lock poisoned");
        reset_for_tests();

        let (sender_a, mut receiver_a) = mpsc::unbounded_channel();
        let (sender_b, mut receiver_b) = mpsc::unbounded_channel();
        let connection_a = register_connection("user-a", "session-a", sender_a);
        let connection_b = register_connection("user-b", "session-b", sender_b);

        subscribe_dm(&connection_a, "dm-abc");
        subscribe_dm(&connection_b, "dm-def");

        broadcast_to_dm(
            "dm-abc",
            ServerOp::DmActivity,
            &json!({ "dm_slug": "dm-abc" }),
        );

        let targeted = read_text_message(&mut receiver_a);
        assert_eq!(targeted["op"], json!("dm_activity"));
        assert!(
            receiver_b.try_recv().is_err(),
            "non-subscribed connection received a dm event"
        );
    }
}
