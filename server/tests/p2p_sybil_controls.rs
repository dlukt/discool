use std::time::{Duration, Instant};

use discool_server::p2p::sybil::{IngressDecision, SybilGuard, SybilSettings};

fn test_settings() -> SybilSettings {
    SybilSettings {
        ingress_window: Duration::from_secs(5),
        ingress_per_peer_limit: 2,
        ingress_global_limit: 6,
        throttle_base: Duration::from_secs(4),
        throttle_max: Duration::from_secs(30),
        peer_retention_capacity: 16,
        stable_peer_min_age: Duration::from_secs(10),
        degraded_reject_ratio_threshold: 0.4,
        degraded_min_samples: 4,
        degraded_min_healthy_peers: 1,
        degraded_bootstrap_failure_threshold: 2,
    }
}

#[test]
fn flooding_peer_enters_cooldown_and_is_throttled() {
    let mut guard = SybilGuard::new(test_settings());
    let now = Instant::now();

    assert_eq!(guard.check_ingress("peer-a", now), IngressDecision::Allow);
    assert_eq!(
        guard.check_ingress("peer-a", now + Duration::from_millis(50)),
        IngressDecision::Allow
    );

    let first_rejection = guard.check_ingress("peer-a", now + Duration::from_millis(100));
    let cooldown_until = match first_rejection {
        IngressDecision::Reject(rejection) => {
            assert_eq!(rejection.reason, "peer-ingress-rate-limit");
            rejection.cooldown_until
        }
        other => panic!("expected first rejection, got {other:?}"),
    };

    match guard.check_ingress("peer-a", now + Duration::from_millis(200)) {
        IngressDecision::Reject(rejection) => {
            assert_eq!(rejection.reason, "peer-cooldown-active");
            assert_eq!(rejection.cooldown_until, cooldown_until);
        }
        other => panic!("expected cooldown rejection, got {other:?}"),
    }
}

#[test]
fn health_snapshot_tracks_rejected_and_throttled_traffic() {
    let mut guard = SybilGuard::new(test_settings());
    let now = Instant::now();

    let _ = guard.check_ingress("peer-a", now);
    let _ = guard.check_ingress("peer-a", now + Duration::from_millis(10));
    let _ = guard.check_ingress("peer-a", now + Duration::from_millis(20));
    let _ = guard.check_ingress("peer-a", now + Duration::from_millis(30));

    let snapshot = guard.health_snapshot(now + Duration::from_secs(1), 2, false);
    assert!(snapshot.ingress_total >= 4);
    assert!(snapshot.rejected_total >= 2);
    assert!(snapshot.throttled_total >= 1);
    assert!(snapshot.message_rate_per_minute > 0.0);

    let reason = guard.degraded_reason(&snapshot);
    assert!(
        reason.is_some(),
        "expected degraded reason for abusive traffic"
    );
}
