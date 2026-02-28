use std::{
    sync::{
        OnceLock,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

use axum::extract::ws::Message;
use chrono::Utc;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use uuid::Uuid;

const IDLE_TIMEOUT: Duration = Duration::from_secs(30);
const OFFLINE_TIMEOUT: Duration = Duration::from_secs(60);
const WATCHDOG_TICK: Duration = Duration::from_secs(5);

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PresenceStatus {
    Online,
    Idle,
    Offline,
}

#[derive(Debug, Clone)]
struct PresenceEntry {
    status: PresenceStatus,
    connection_count: usize,
    last_seen_at: Instant,
}

#[derive(Default)]
struct PresenceRegistry {
    users: DashMap<String, PresenceEntry>,
    sockets: DashMap<String, mpsc::UnboundedSender<Message>>,
    watchdog_started: AtomicBool,
}

static PRESENCE_REGISTRY: OnceLock<PresenceRegistry> = OnceLock::new();

#[derive(Serialize)]
struct PresenceEnvelope<'a> {
    op: &'a str,
    d: PresencePayload<'a>,
}

#[derive(Serialize)]
struct PresencePayload<'a> {
    user_id: &'a str,
    status: PresenceStatus,
    updated_at: String,
}

fn registry() -> &'static PresenceRegistry {
    PRESENCE_REGISTRY.get_or_init(PresenceRegistry::default)
}

fn broadcast_presence_update(user_id: &str, status: PresenceStatus) {
    let message = match serde_json::to_string(&PresenceEnvelope {
        op: "presence_update",
        d: PresencePayload {
            user_id,
            status,
            updated_at: Utc::now().to_rfc3339(),
        },
    }) {
        Ok(value) => value,
        Err(err) => {
            tracing::warn!(error = %err, "Failed to serialize presence update");
            return;
        }
    };

    let mut stale_connection_ids = Vec::new();
    for entry in registry().sockets.iter() {
        if entry
            .value()
            .send(Message::Text(message.clone().into()))
            .is_err()
        {
            stale_connection_ids.push(entry.key().clone());
        }
    }

    for connection_id in stale_connection_ids {
        registry().sockets.remove(&connection_id);
    }
}

fn evaluate_status(entry: &PresenceEntry, now: Instant) -> PresenceStatus {
    let elapsed = now.saturating_duration_since(entry.last_seen_at);
    if entry.connection_count == 0 {
        if elapsed >= OFFLINE_TIMEOUT {
            PresenceStatus::Offline
        } else {
            PresenceStatus::Idle
        }
    } else if elapsed >= IDLE_TIMEOUT {
        PresenceStatus::Idle
    } else {
        PresenceStatus::Online
    }
}

fn tick_timeouts(now: Instant) {
    let mut updates = Vec::new();
    for mut entry in registry().users.iter_mut() {
        let next_status = evaluate_status(entry.value(), now);
        if next_status != entry.status {
            entry.status = next_status;
            updates.push((entry.key().clone(), next_status));
        }
    }

    for (user_id, status) in updates {
        broadcast_presence_update(&user_id, status);
    }
}

pub fn ensure_watchdog_started() {
    if registry()
        .watchdog_started
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        return;
    }

    tokio::spawn(async {
        let mut interval = tokio::time::interval(WATCHDOG_TICK);
        loop {
            interval.tick().await;
            tick_timeouts(Instant::now());
        }
    });
}

pub fn register_connection(sender: mpsc::UnboundedSender<Message>) -> String {
    let connection_id = Uuid::new_v4().to_string();
    registry().sockets.insert(connection_id.clone(), sender);
    connection_id
}

pub fn unregister_connection(connection_id: &str) {
    registry().sockets.remove(connection_id);
}

pub fn websocket_connection_count() -> u32 {
    u32::try_from(registry().sockets.len()).unwrap_or(u32::MAX)
}

pub fn current_presence_status(user_id: &str) -> PresenceStatus {
    registry()
        .users
        .get(user_id)
        .map(|entry| entry.status)
        .unwrap_or(PresenceStatus::Offline)
}

pub fn mark_connected(user_id: &str) {
    let now = Instant::now();
    let mut should_broadcast_online = false;

    registry()
        .users
        .entry(user_id.to_string())
        .and_modify(|entry| {
            entry.connection_count = entry.connection_count.saturating_add(1);
            entry.last_seen_at = now;
            if entry.status != PresenceStatus::Online {
                entry.status = PresenceStatus::Online;
                should_broadcast_online = true;
            }
        })
        .or_insert_with(|| {
            should_broadcast_online = true;
            PresenceEntry {
                status: PresenceStatus::Online,
                connection_count: 1,
                last_seen_at: now,
            }
        });

    if should_broadcast_online {
        broadcast_presence_update(user_id, PresenceStatus::Online);
    }
}

pub fn mark_heartbeat(user_id: &str) {
    let now = Instant::now();
    let mut should_broadcast_online = false;

    if let Some(mut entry) = registry().users.get_mut(user_id) {
        if entry.connection_count == 0 {
            return;
        }
        entry.last_seen_at = now;
        if entry.status != PresenceStatus::Online {
            entry.status = PresenceStatus::Online;
            should_broadcast_online = true;
        }
    } else {
        return;
    }

    if should_broadcast_online {
        broadcast_presence_update(user_id, PresenceStatus::Online);
    }
}

pub fn mark_disconnected(user_id: &str) {
    let now = Instant::now();
    let mut should_broadcast_idle = false;

    if let Some(mut entry) = registry().users.get_mut(user_id) {
        if entry.connection_count > 0 {
            entry.connection_count -= 1;
        }
        entry.last_seen_at = now;
        if entry.connection_count == 0 && entry.status != PresenceStatus::Idle {
            entry.status = PresenceStatus::Idle;
            should_broadcast_idle = true;
        }
    }

    if should_broadcast_idle {
        broadcast_presence_update(user_id, PresenceStatus::Idle);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_lock() -> &'static std::sync::Mutex<()> {
        static LOCK: std::sync::OnceLock<std::sync::Mutex<()>> = std::sync::OnceLock::new();
        LOCK.get_or_init(|| std::sync::Mutex::new(()))
    }

    fn reset_presence_state() {
        registry().users.clear();
        registry().sockets.clear();
    }

    #[test]
    fn unknown_users_default_to_offline() {
        let _guard = test_lock().lock().expect("presence test lock poisoned");
        reset_presence_state();
        assert_eq!(
            current_presence_status("missing-user"),
            PresenceStatus::Offline
        );
    }

    #[test]
    fn connect_heartbeat_and_idle_timeout_transitions_work() {
        let _guard = test_lock().lock().expect("presence test lock poisoned");
        reset_presence_state();
        let user_id = "user-presence-online";

        mark_connected(user_id);
        assert_eq!(current_presence_status(user_id), PresenceStatus::Online);

        if let Some(mut entry) = registry().users.get_mut(user_id) {
            entry.last_seen_at = Instant::now() - IDLE_TIMEOUT - Duration::from_secs(1);
        }
        tick_timeouts(Instant::now());
        assert_eq!(current_presence_status(user_id), PresenceStatus::Idle);

        mark_heartbeat(user_id);
        assert_eq!(current_presence_status(user_id), PresenceStatus::Online);
    }

    #[test]
    fn disconnect_timeout_transitions_to_offline() {
        let _guard = test_lock().lock().expect("presence test lock poisoned");
        reset_presence_state();
        let user_id = "user-presence-disconnect";

        mark_connected(user_id);
        mark_disconnected(user_id);
        assert_eq!(current_presence_status(user_id), PresenceStatus::Idle);

        if let Some(mut entry) = registry().users.get_mut(user_id) {
            entry.last_seen_at = Instant::now() - OFFLINE_TIMEOUT - Duration::from_secs(1);
        }
        tick_timeouts(Instant::now());
        assert_eq!(current_presence_status(user_id), PresenceStatus::Offline);
    }

    #[test]
    fn heartbeat_does_not_revive_disconnected_or_unknown_users() {
        let _guard = test_lock().lock().expect("presence test lock poisoned");
        reset_presence_state();
        let user_id = "user-presence-heartbeat-guard";

        mark_connected(user_id);
        mark_disconnected(user_id);
        assert_eq!(current_presence_status(user_id), PresenceStatus::Idle);

        mark_heartbeat(user_id);
        assert_eq!(current_presence_status(user_id), PresenceStatus::Idle);
        let connection_count = registry()
            .users
            .get(user_id)
            .map(|entry| entry.connection_count)
            .unwrap_or_default();
        assert_eq!(connection_count, 0);

        mark_heartbeat("missing-heartbeat-user");
        assert_eq!(
            current_presence_status("missing-heartbeat-user"),
            PresenceStatus::Offline
        );
    }
}
