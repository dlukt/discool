use std::collections::{HashMap, VecDeque};
use std::time::{Duration, Instant};

use crate::config::P2pConfig;

#[derive(Debug, Clone)]
pub struct SybilSettings {
    pub ingress_window: Duration,
    pub ingress_per_peer_limit: usize,
    pub ingress_global_limit: usize,
    pub throttle_base: Duration,
    pub throttle_max: Duration,
    pub peer_retention_capacity: usize,
    pub stable_peer_min_age: Duration,
    pub degraded_reject_ratio_threshold: f64,
    pub degraded_min_samples: usize,
    pub degraded_min_healthy_peers: u32,
    pub degraded_bootstrap_failure_threshold: u32,
}

impl SybilSettings {
    pub fn from_config(config: &P2pConfig) -> Self {
        Self {
            ingress_window: Duration::from_secs(config.ingress_rate_window_secs),
            ingress_per_peer_limit: usize::try_from(config.ingress_rate_per_peer_limit)
                .unwrap_or(usize::MAX),
            ingress_global_limit: usize::try_from(config.ingress_rate_global_limit)
                .unwrap_or(usize::MAX),
            throttle_base: Duration::from_secs(config.throttle_base_secs),
            throttle_max: Duration::from_secs(config.throttle_max_secs),
            peer_retention_capacity: config.peer_retention_capacity,
            stable_peer_min_age: Duration::from_secs(config.stable_peer_min_age_secs),
            degraded_reject_ratio_threshold: config.degraded_reject_ratio_threshold,
            degraded_min_samples: usize::try_from(config.degraded_min_samples)
                .unwrap_or(usize::MAX),
            degraded_min_healthy_peers: config.degraded_min_healthy_peers,
            degraded_bootstrap_failure_threshold: config.degraded_bootstrap_failure_threshold,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IngressRejection {
    pub reason: &'static str,
    pub cooldown_until: Instant,
}

impl IngressRejection {
    pub fn throttle_expires_in_secs(&self, now: Instant) -> f64 {
        self.cooldown_until
            .saturating_duration_since(now)
            .as_secs_f64()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IngressDecision {
    Allow,
    Reject(IngressRejection),
}

#[derive(Debug, Clone)]
pub struct HealthSnapshot {
    pub message_rate_per_minute: f64,
    pub ingress_total: u64,
    pub rejected_total: u64,
    pub throttled_total: u64,
    pub healthy_peer_count: u32,
    pub bootstrap_failures: u32,
    pub standalone_mode: bool,
}

#[derive(Debug, Clone)]
struct PeerState {
    first_seen: Instant,
    last_seen: Instant,
    violation_score: u32,
    cooldown_until: Option<Instant>,
    accepted_events: u64,
    rejected_events: u64,
}

pub struct SybilGuard {
    settings: SybilSettings,
    global_window: VecDeque<Instant>,
    peer_windows: HashMap<String, VecDeque<Instant>>,
    peers: HashMap<String, PeerState>,
    ingress_total: u64,
    rejected_total: u64,
    throttled_total: u64,
    evicted_peers: VecDeque<String>,
}

impl SybilGuard {
    pub fn new(settings: SybilSettings) -> Self {
        Self {
            settings,
            global_window: VecDeque::new(),
            peer_windows: HashMap::new(),
            peers: HashMap::new(),
            ingress_total: 0,
            rejected_total: 0,
            throttled_total: 0,
            evicted_peers: VecDeque::new(),
        }
    }

    pub fn observe_peer(&mut self, peer_id: &str, now: Instant) {
        self.touch_peer(peer_id, now);
    }

    pub fn check_ingress(&mut self, peer_id: &str, now: Instant) -> IngressDecision {
        self.prune_windows(now);
        self.touch_peer(peer_id, now);

        self.ingress_total = self.ingress_total.saturating_add(1);

        if let Some(cooldown_until) = self.active_cooldown(peer_id, now) {
            self.rejected_total = self.rejected_total.saturating_add(1);
            self.throttled_total = self.throttled_total.saturating_add(1);
            if let Some(state) = self.peers.get_mut(peer_id) {
                state.rejected_events = state.rejected_events.saturating_add(1);
            }
            return IngressDecision::Reject(IngressRejection {
                reason: "peer-cooldown-active",
                cooldown_until,
            });
        }

        self.global_window.push_back(now);
        let peer_window_len = {
            let peer_window = self.peer_windows.entry(peer_id.to_string()).or_default();
            peer_window.push_back(now);
            peer_window.len()
        };

        if self.global_window.len() > self.settings.ingress_global_limit {
            return IngressDecision::Reject(self.register_violation(
                peer_id,
                now,
                "global-ingress-rate-limit",
            ));
        }
        if peer_window_len > self.settings.ingress_per_peer_limit {
            return IngressDecision::Reject(self.register_violation(
                peer_id,
                now,
                "peer-ingress-rate-limit",
            ));
        }

        if let Some(state) = self.peers.get_mut(peer_id) {
            state.accepted_events = state.accepted_events.saturating_add(1);
        }
        IngressDecision::Allow
    }

    pub fn register_invalid_message(&mut self, peer_id: &str, now: Instant) -> IngressRejection {
        self.touch_peer(peer_id, now);
        self.register_violation(peer_id, now, "invalid-gossip-message")
    }

    pub fn take_next_evicted_peer(&mut self) -> Option<String> {
        self.evicted_peers.pop_front()
    }

    pub fn healthy_peer_count(&self, now: Instant) -> u32 {
        let mut freshness_window = self.settings.ingress_window.saturating_mul(3);
        if freshness_window < Duration::from_secs(30) {
            freshness_window = Duration::from_secs(30);
        }

        u32::try_from(
            self.peers
                .values()
                .filter(|state| {
                    state.cooldown_until.is_none_or(|until| until <= now)
                        && now.saturating_duration_since(state.last_seen) <= freshness_window
                })
                .count(),
        )
        .unwrap_or(u32::MAX)
    }

    pub fn message_rate_per_minute(&mut self, now: Instant) -> f64 {
        self.prune_windows(now);
        if self.settings.ingress_window.is_zero() {
            return 0.0;
        }
        let seconds = self.settings.ingress_window.as_secs_f64();
        if seconds <= f64::EPSILON {
            return 0.0;
        }
        (self.global_window.len() as f64 / seconds) * 60.0
    }

    pub fn health_snapshot(
        &mut self,
        now: Instant,
        bootstrap_failures: u32,
        standalone_mode: bool,
    ) -> HealthSnapshot {
        HealthSnapshot {
            message_rate_per_minute: self.message_rate_per_minute(now),
            ingress_total: self.ingress_total,
            rejected_total: self.rejected_total,
            throttled_total: self.throttled_total,
            healthy_peer_count: self.healthy_peer_count(now),
            bootstrap_failures,
            standalone_mode,
        }
    }

    pub fn degraded_reason(&self, snapshot: &HealthSnapshot) -> Option<String> {
        if snapshot.ingress_total >= self.settings.degraded_min_samples as u64 {
            let reject_ratio = snapshot.rejected_total as f64 / snapshot.ingress_total as f64;
            if reject_ratio >= self.settings.degraded_reject_ratio_threshold {
                return Some(format!(
                    "reject ratio {:.2} exceeded threshold {:.2}; inspect abusive peers and tune ingress limits",
                    reject_ratio, self.settings.degraded_reject_ratio_threshold
                ));
            }
        }

        if snapshot.bootstrap_failures >= self.settings.degraded_bootstrap_failure_threshold {
            return Some(format!(
                "bootstrap failures ({}) exceeded threshold {}; verify bootstrap peers and network reachability",
                snapshot.bootstrap_failures, self.settings.degraded_bootstrap_failure_threshold
            ));
        }

        if !snapshot.standalone_mode
            && snapshot.healthy_peer_count < self.settings.degraded_min_healthy_peers
            && snapshot.ingress_total >= self.settings.degraded_min_samples as u64
        {
            return Some(format!(
                "healthy peer count ({}) below threshold {}; verify peer availability and bootstrap health",
                snapshot.healthy_peer_count, self.settings.degraded_min_healthy_peers
            ));
        }

        None
    }

    fn touch_peer(&mut self, peer_id: &str, now: Instant) {
        self.peers
            .entry(peer_id.to_string())
            .and_modify(|state| {
                state.last_seen = now;
            })
            .or_insert(PeerState {
                first_seen: now,
                last_seen: now,
                violation_score: 0,
                cooldown_until: None,
                accepted_events: 0,
                rejected_events: 0,
            });
        self.enforce_retention(now);
    }

    fn enforce_retention(&mut self, now: Instant) {
        while self.peers.len() > self.settings.peer_retention_capacity {
            let evict_peer = self
                .peers
                .iter()
                .min_by_key(|(_, state)| {
                    retention_score(state, now, self.settings.stable_peer_min_age)
                })
                .map(|(peer_id, _)| peer_id.clone());

            if let Some(peer_id) = evict_peer {
                self.peers.remove(&peer_id);
                self.peer_windows.remove(&peer_id);
                self.evicted_peers.push_back(peer_id);
            } else {
                break;
            }
        }
    }

    fn active_cooldown(&self, peer_id: &str, now: Instant) -> Option<Instant> {
        self.peers
            .get(peer_id)
            .and_then(|state| state.cooldown_until)
            .filter(|until| *until > now)
    }

    fn register_violation(
        &mut self,
        peer_id: &str,
        now: Instant,
        reason: &'static str,
    ) -> IngressRejection {
        let state = self.peers.entry(peer_id.to_string()).or_insert(PeerState {
            first_seen: now,
            last_seen: now,
            violation_score: 0,
            cooldown_until: None,
            accepted_events: 0,
            rejected_events: 0,
        });
        state.last_seen = now;
        state.violation_score = state.violation_score.saturating_add(1);
        state.rejected_events = state.rejected_events.saturating_add(1);

        let exp = state.violation_score.saturating_sub(1).min(16);
        let multiplier = 1u64 << exp;
        let base = self.settings.throttle_base.as_secs().max(1);
        let max = self.settings.throttle_max.as_secs().max(base);
        let cooldown_secs = base.saturating_mul(multiplier).min(max);
        let cooldown_until = now + Duration::from_secs(cooldown_secs);
        state.cooldown_until = Some(cooldown_until);

        self.rejected_total = self.rejected_total.saturating_add(1);
        self.throttled_total = self.throttled_total.saturating_add(1);

        IngressRejection {
            reason,
            cooldown_until,
        }
    }

    fn prune_windows(&mut self, now: Instant) {
        let window = self.settings.ingress_window;
        while self
            .global_window
            .front()
            .is_some_and(|seen| now.saturating_duration_since(*seen) > window)
        {
            self.global_window.pop_front();
        }

        self.peer_windows.retain(|_, events| {
            while events
                .front()
                .is_some_and(|seen| now.saturating_duration_since(*seen) > window)
            {
                events.pop_front();
            }
            !events.is_empty()
        });
    }
}

fn retention_score(state: &PeerState, now: Instant, stable_peer_min_age: Duration) -> i64 {
    let age_secs = i64::try_from(now.saturating_duration_since(state.first_seen).as_secs())
        .unwrap_or(i64::MAX);
    let staleness_secs =
        i64::try_from(now.saturating_duration_since(state.last_seen).as_secs()).unwrap_or(i64::MAX);
    let stable_bonus: i64 =
        if now.saturating_duration_since(state.first_seen) >= stable_peer_min_age {
            20_000
        } else {
            0
        };
    let accepted_bonus = i64::try_from(state.accepted_events.min(5_000)).unwrap_or(i64::MAX);
    let violation_penalty = i64::from(state.violation_score).saturating_mul(5_000)
        + i64::try_from(state.rejected_events.min(1_000)).unwrap_or(i64::MAX);

    stable_bonus
        .saturating_add(age_secs)
        .saturating_add(accepted_bonus)
        .saturating_sub(violation_penalty)
        .saturating_sub(staleness_secs)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_settings() -> SybilSettings {
        SybilSettings {
            ingress_window: Duration::from_secs(10),
            ingress_per_peer_limit: 3,
            ingress_global_limit: 6,
            throttle_base: Duration::from_secs(5),
            throttle_max: Duration::from_secs(60),
            peer_retention_capacity: 2,
            stable_peer_min_age: Duration::from_secs(20),
            degraded_reject_ratio_threshold: 0.5,
            degraded_min_samples: 4,
            degraded_min_healthy_peers: 1,
            degraded_bootstrap_failure_threshold: 3,
        }
    }

    #[test]
    fn per_peer_rate_limit_triggers_rejection() {
        let mut guard = SybilGuard::new(test_settings());
        let now = Instant::now();
        assert_eq!(guard.check_ingress("peer-a", now), IngressDecision::Allow);
        assert_eq!(
            guard.check_ingress("peer-a", now + Duration::from_secs(1)),
            IngressDecision::Allow
        );
        assert_eq!(
            guard.check_ingress("peer-a", now + Duration::from_secs(2)),
            IngressDecision::Allow
        );
        match guard.check_ingress("peer-a", now + Duration::from_secs(3)) {
            IngressDecision::Reject(rejection) => {
                assert_eq!(rejection.reason, "peer-ingress-rate-limit");
            }
            other => panic!("expected rejection, got {other:?}"),
        }
    }

    #[test]
    fn repeated_violations_activate_cooldown() {
        let mut guard = SybilGuard::new(test_settings());
        let now = Instant::now();
        let rejection = guard.register_invalid_message("peer-a", now);
        assert_eq!(rejection.reason, "invalid-gossip-message");

        match guard.check_ingress("peer-a", now + Duration::from_secs(1)) {
            IngressDecision::Reject(rejection) => {
                assert_eq!(rejection.reason, "peer-cooldown-active");
                assert!(rejection.cooldown_until > now + Duration::from_secs(1));
            }
            other => panic!("expected cooldown rejection, got {other:?}"),
        }
    }

    #[test]
    fn cooldown_rejections_do_not_exhaust_global_window() {
        let mut settings = test_settings();
        settings.ingress_per_peer_limit = 1;
        settings.ingress_global_limit = 3;
        settings.throttle_base = Duration::from_secs(30);
        settings.throttle_max = Duration::from_secs(30);

        let mut guard = SybilGuard::new(settings);
        let now = Instant::now();
        assert_eq!(guard.check_ingress("peer-a", now), IngressDecision::Allow);

        match guard.check_ingress("peer-a", now + Duration::from_millis(1)) {
            IngressDecision::Reject(rejection) => {
                assert_eq!(rejection.reason, "peer-ingress-rate-limit");
            }
            other => panic!("expected rate-limit rejection, got {other:?}"),
        }

        for offset in [2_u64, 3_u64] {
            match guard.check_ingress("peer-a", now + Duration::from_millis(offset)) {
                IngressDecision::Reject(rejection) => {
                    assert_eq!(rejection.reason, "peer-cooldown-active");
                }
                other => panic!("expected cooldown rejection, got {other:?}"),
            }
        }

        assert_eq!(
            guard.check_ingress("peer-b", now + Duration::from_millis(4)),
            IngressDecision::Allow
        );
    }

    #[test]
    fn retention_prefers_long_lived_well_behaved_peer() {
        let mut guard = SybilGuard::new(test_settings());
        let now = Instant::now();

        guard.observe_peer("stable-peer", now);
        for second in 0..3 {
            assert_eq!(
                guard.check_ingress("stable-peer", now + Duration::from_secs(second)),
                IngressDecision::Allow
            );
        }

        let newer = now + Duration::from_secs(21);
        guard.observe_peer("noisy-peer", newer);
        let _ = guard.register_invalid_message("noisy-peer", newer + Duration::from_secs(1));

        guard.observe_peer("third-peer", newer + Duration::from_secs(2));
        let evicted = guard.take_next_evicted_peer();
        assert_eq!(evicted.as_deref(), Some("noisy-peer"));
    }

    #[test]
    fn degraded_reason_flags_high_reject_ratio() {
        let mut guard = SybilGuard::new(test_settings());
        let now = Instant::now();
        for second in 0..6 {
            let _ = guard.check_ingress("peer-a", now + Duration::from_secs(second));
        }

        let snapshot = guard.health_snapshot(now + Duration::from_secs(7), 0, false);
        let reason = guard.degraded_reason(&snapshot);
        assert!(reason.is_some());
    }

    #[test]
    fn retention_reports_all_evictions_when_capacity_is_exceeded() {
        let mut settings = test_settings();
        settings.peer_retention_capacity = 1;
        settings.stable_peer_min_age = Duration::from_secs(1);
        let mut guard = SybilGuard::new(settings);
        let now = Instant::now();

        guard.observe_peer("stable-peer", now);
        let _ = guard.check_ingress("stable-peer", now + Duration::from_secs(1));
        guard.observe_peer("peer-b", now + Duration::from_secs(2));
        guard.observe_peer("peer-c", now + Duration::from_secs(3));

        let mut evicted = Vec::new();
        while let Some(peer) = guard.take_next_evicted_peer() {
            evicted.push(peer);
        }

        assert_eq!(evicted.len(), 2);
    }

    #[test]
    fn healthy_peer_count_excludes_stale_peers() {
        let mut guard = SybilGuard::new(test_settings());
        let now = Instant::now();
        guard.observe_peer("peer-a", now);

        assert_eq!(guard.healthy_peer_count(now + Duration::from_secs(5)), 1);
        assert_eq!(guard.healthy_peer_count(now + Duration::from_secs(31)), 0);
    }

    #[test]
    fn invalid_message_violation_counts_as_throttled() {
        let mut guard = SybilGuard::new(test_settings());
        let now = Instant::now();
        let _ = guard.register_invalid_message("peer-a", now);

        let snapshot = guard.health_snapshot(now + Duration::from_secs(1), 0, false);
        assert_eq!(snapshot.throttled_total, 1);
    }
}
