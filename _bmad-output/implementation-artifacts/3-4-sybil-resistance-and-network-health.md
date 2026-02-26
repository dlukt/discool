# Story 3.4: Sybil Resistance and Network Health

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As an **instance operator**,
I want the P2P network to resist attacks from fake instances,
so that the discovery network remains trustworthy and functional.

## Acceptance Criteria

1. **Given** the P2P network is operational  
   **When** a new instance attempts to join the DHT  
   **Then** rate-based participation controls limit how quickly new peers can register.

2. **Given** the P2P network is operational  
   **When** peers flood the network with excessive requests  
   **Then** abusive peers are temporarily throttled.

3. **Given** the DHT is maintaining peer tables  
   **When** retention/eviction decisions are made  
   **Then** long-lived, well-behaved peers are favored over newly joined peers.

4. **Given** an operator opens the admin panel  
   **When** viewing P2P health  
   **Then** peer count, message rate, and rejected/throttled peer signals are visible.

5. **Given** the local instance detects degraded network conditions  
   **When** thresholds are exceeded  
   **Then** actionable warning logs are emitted without crashing local operation.

## Tasks / Subtasks

- [x] Task 1: Add rate-based participation controls for DHT/gossip ingress (AC: 1, 2)
  - [x] Add explicit anti-abuse settings to `P2pConfig` in `server/src/config/settings.rs` with validation rules that follow existing `p2p.enabled` guardrails.
  - [x] Implement bounded per-peer/global windows (new helper module under `server/src/p2p/`, e.g., `sybil.rs`) and wire checks into `server/src/p2p/node.rs`.
  - [x] Apply controls before processing high-frequency discovery/gossip activity.

- [x] Task 2: Implement temporary throttling for abusive peers (AC: 2)
  - [x] Track peer violations (invalid gossip, repeated flood behavior) and calculate cooldown periods.
  - [x] During active cooldown, reject/ignore abusive traffic and emit structured logs with peer id, reason, and throttle expiry.
  - [x] Keep throttling non-fatal and avoid global lockups.

- [x] Task 3: Favor long-lived well-behaved peers during retention decisions (AC: 3)
  - [x] Track peer stability signals (first_seen/last_seen/violation score) in runtime metadata.
  - [x] Implement a retention heuristic so stable peers are preferred when capacity pressure occurs.
  - [x] Ensure this policy integrates with existing Kademlia discovery flow instead of introducing a parallel peer-management path.

- [x] Task 4: Expose network health signals in admin health API (AC: 4)
  - [x] Extend `P2pMetadata` with network-health counters/rates needed by operators.
  - [x] Extend `AdminHealth` and `get_health` in `server/src/handlers/admin.rs` to include these values while preserving existing response envelope `{ "data": ... }`.
  - [x] Add/update handler tests for new response fields.

- [x] Task 5: Detect and log degraded network conditions (AC: 5)
  - [x] Define degraded-condition thresholds (e.g., sustained reject ratio, repeated bootstrap failures, low effective healthy peers).
  - [x] Emit warning-level logs with concise remediation hints.
  - [x] Preserve graceful degradation: server remains available even if P2P is degraded.

- [x] Task 6: Test coverage and quality gates (AC: all)
  - [x] Add unit tests for limiter/throttle/retention decision logic.
  - [x] Add integration coverage for flood/throttle behavior and admin health metric surfacing.
  - [x] Run: `cd server && cargo fmt --check && cargo clippy -- -D warnings && cargo test`.

## Dev Notes

### Developer Context

- Build directly on Epic 3 foundations already implemented in Stories 3.1-3.3 (`identity.rs`, `discovery.rs`, `gossip.rs`, `node.rs`, `P2pMetadata`).
- Do not introduce alternate networking stacks, duplicate discovery loops, or non-libp2p side channels.
- Keep this story scoped to P2P hardening + observability; do not pull in Epic 3.5 private-mode behavior beyond compatibility.

### Technical Requirements

- Maintain existing startup behavior where P2P failures degrade gracefully and do not block core server startup.
- Keep anti-abuse logic deterministic and testable; avoid ad-hoc global mutable state that cannot be unit tested.
- Reuse current structured logging patterns (`tracing`) and include peer context without leaking sensitive payloads.
- Preserve compatibility with both supported DB backends; avoid introducing persistence requirements unless strictly needed.

### Architecture Compliance

1. Keep layer boundaries intact: `p2p/` handles ingress/events, services handle business effects, models handle DB.  
2. Do not let `p2p/*` access the DB directly except through established discovery helpers already used by the runtime.  
3. Maintain protocol versioned namespaces (`/discool/kad/1.0.0`, `/discool/gossip/1.0.0/...`) and avoid breaking wire contracts.  
4. Keep server/runtime resilient: anti-abuse controls must reduce attack impact without turning normal transient churn into outages.

### Library & Framework Requirements

- Keep `rust-libp2p` on the project baseline (`0.56.x`) and Tokio runtime integration.
- Continue using Gossipsub strict validation + explicit `report_message_validation_result(...)` acceptance/rejection signaling.
- Prefer existing libp2p primitives (peer scoring/validation/rate controls where applicable) over custom protocol forks.
- Keep dependencies minimal; add new crates only if clearly justified and aligned with existing architecture.

### File Structure Requirements

Primary touch points (expected):

- `server/src/p2p/node.rs` (event-loop enforcement and health aggregation)
- `server/src/p2p/discovery.rs` (behaviour-level configuration hooks)
- `server/src/p2p/gossip.rs` (validation/scoring-related helpers if needed)
- `server/src/p2p/mod.rs` (`P2pMetadata` expansion)
- `server/src/handlers/admin.rs` (health payload extension)
- `server/src/config/settings.rs` + `config.example.toml` (new anti-abuse config knobs and validation)
- `server/src/p2p/*.rs` unit tests and `server/tests/*.rs` integration tests

### Testing Requirements

- Verify normal peer join/discovery still works under non-abusive traffic.
- Verify burst/flood traffic triggers throttling and rejection behavior.
- Verify long-lived peers are retained/preferred according to policy.
- Verify admin health endpoint reports required network-health fields.
- Re-run full server quality gates:
  - `cd server && cargo fmt --check`
  - `cd server && cargo clippy -- -D warnings`
  - `cd server && cargo test`

### Previous Story Intelligence

- Story 3.3 already established:
  - versioned gossip topics under `/discool/gossip/1.0.0/...`
  - signed gossip messages with source/sender validation
  - strict accept/reject signaling for inbound gossip validation
  - service-layer routing for valid gossip events
- Preserve these patterns; extend them with anti-abuse controls instead of replacing them.

### Git Intelligence Summary

- Recent commits show Epic 3 work centered in `server/src/p2p/*`, `server/src/config/settings.rs`, and `server/src/handlers/admin.rs`.
- Keep Story 3.4 implementation similarly scoped and include tests/config/docs in the same change set.
- Continue conventional-commit style and story-scoped file changes.

### Latest Technical Information

1. rust-libp2p stable baseline remains **v0.56.0** with Tokio-first runtime guidance.
2. Production guidance continues to favor composed `NetworkBehaviour` (Identify + Kademlia + Gossipsub) for robust peer discovery/messaging.
3. Gossipsub hardening best practice includes strict validation, explicit reject reporting, and peer-behavior-aware scoring/throttling.
4. Operational observability for P2P commonly tracks connected peer count, message throughput, reject/drop rates, and churn/degraded-state indicators.

### Project Context Reference

- No `project-context.md` discovered via `**/project-context.md`.
- Authoritative context comes from planning artifacts + Epic 3 completed stories + current server P2P modules.

### Story Completion Status

- Ultimate context engine analysis completed - comprehensive developer guide created.
- Story marked `ready-for-dev`.

### Project Structure Notes

- Fully aligned with documented structure (`server/src/p2p`, `server/src/handlers`, `server/src/config`).
- No architecture variance required for this story.

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Epic 3: P2P Discovery & Federation Foundation]
- [Source: _bmad-output/planning-artifacts/epics.md#Story 3.4: Sybil Resistance and Network Health]
- [Source: _bmad-output/planning-artifacts/architecture.md#Core Architectural Decisions]
- [Source: _bmad-output/planning-artifacts/architecture.md#Project Organization]
- [Source: _bmad-output/planning-artifacts/architecture.md#Implementation Handoff]
- [Source: _bmad-output/planning-artifacts/prd.md#P2P & Distributed System Constraints]
- [Source: _bmad-output/planning-artifacts/prd.md#Instance Management]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Journey 3: Instance Deployment & First Run (Tomas)]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Journey Patterns]
- [Source: _bmad-output/implementation-artifacts/3-3-gossipsub-inter-instance-communication.md]
- [Source: server/src/p2p/node.rs]
- [Source: server/src/p2p/gossip.rs]
- [Source: server/src/p2p/discovery.rs]
- [Source: server/src/handlers/admin.rs]
- [Source: server/src/config/settings.rs]
- [Source: https://github.com/libp2p/rust-libp2p/releases]
- [Source: https://libp2p.io/releases/2025-06-28-rust-libp2p/]
- [Source: https://libp2p.github.io/rust-libp2p/libp2p/kad/index.html]
- [Source: https://libp2p.github.io/rust-libp2p/libp2p/gossipsub/index.html]

## Dev Agent Record

### Agent Model Used

GPT-5.3-Codex (model ID: gpt-5.3-codex)

### Debug Log References

- Baseline quality gates (pre-change): `cd server && cargo fmt --check && cargo clippy -- -D warnings && cargo test` ✅
- Final quality gates (post-change): `cd server && cargo fmt --check && cargo clippy -- -D warnings && cargo test` ✅

### Completion Notes List

- Added explicit anti-abuse config knobs and validation under `p2p.enabled` guardrails (ingress windows/limits, throttle bounds, retention capacity, degraded thresholds).
- Implemented `server/src/p2p/sybil.rs` with deterministic per-peer/global ingress limiting, violation-driven cooldown throttling, retention heuristics favoring stable peers, and degraded health evaluation.
- Wired anti-abuse controls into `server/src/p2p/node.rs` before identify/gossip processing, including structured throttle/rejection logs and non-fatal enforcement behavior.
- Extended `P2pMetadata` and admin health payload with message-rate, ingress/reject/throttle counters, healthy peer count, bootstrap failure count, and degraded status/reason.
- Added/updated tests for sybil logic and admin health surfacing, including new integration coverage in `server/tests/p2p_sybil_controls.rs` and updated health assertions in `server/tests/server_binds_to_configured_port.rs`.
- Verified server quality gates pass after implementation.

### File List

- _bmad-output/implementation-artifacts/3-4-sybil-resistance-and-network-health.md
- _bmad-output/implementation-artifacts/sprint-status.yaml
- config.example.toml
- server/src/config/settings.rs
- server/src/handlers/admin.rs
- server/src/p2p/mod.rs
- server/src/p2p/node.rs
- server/src/p2p/sybil.rs
- server/tests/p2p_sybil_controls.rs
- server/tests/server_binds_to_configured_port.rs

## Change Log

- 2026-02-26: Implemented Story 3.4 sybil resistance hardening with ingress controls, temporary throttling, retention heuristics, degraded-network warning signals, admin health observability extensions, and full server quality-gate validation.
- 2026-02-26: Senior code review follow-up fixed cooldown-window spillover that could trigger collateral global throttling, enforced runtime disconnect attempts for retention evictions, and added targeted regression coverage.
- 2026-02-26: YOLO review follow-up fixed multi-eviction disconnect gaps, stale healthy-peer counting in degraded-health signals, and throttle telemetry undercounting; added targeted regression tests.

## Senior Developer Review (AI)

### Outcome

- **Changes Requested → Fixed in this pass**

### Findings

1. **[HIGH][Fixed] Cooldown traffic polluted the global ingress window**
   - Proof: `server/src/p2p/sybil.rs` previously pushed every ingress event into global/per-peer windows before checking active cooldown.
   - Impact: A peer already in cooldown could still consume global-window capacity and trigger collateral `global-ingress-rate-limit` rejections for healthy peers.
   - Fix: `check_ingress` now checks active cooldown before adding events to rate-limit windows.

2. **[HIGH][Fixed] Retention eviction did not affect active swarm peers**
   - Proof: `server/src/p2p/node.rs` previously only logged `take_last_evicted_peer()` and never acted on the evicted peer.
   - Impact: Retention heuristics remained metadata-only and did not influence live peer set pressure.
   - Fix: Added `handle_retention_eviction(...)` to attempt `swarm.disconnect_peer_id(...)` for evicted peers and log disconnect outcome.

3. **[MEDIUM][Fixed] Missing regression coverage for cooldown spillover behavior**
   - Proof: No unit test asserted that repeated cooldown rejections avoid exhausting the global ingress window.
   - Impact: The collateral-throttling regression could reappear unnoticed.
   - Fix: Added `cooldown_rejections_do_not_exhaust_global_window` unit test in `server/src/p2p/sybil.rs`.

### YOLO Follow-up Findings

1. **[HIGH][Fixed] Retention queue dropped all but the last evicted peer**
   - Proof: `server/src/p2p/sybil.rs` tracked only one `last_evicted_peer`, and repeated evictions in one retention pass overwrote prior entries before `node.rs` drained them.
   - Impact: Under burst capacity pressure, only one peer disconnect was attempted while other evicted peers could remain connected, weakening AC3 retention enforcement.
   - Fix: Replaced single-slot eviction tracking with FIFO queue semantics (`evicted_peers`) and switched runtime draining to `take_next_evicted_peer()`.

2. **[MEDIUM][Fixed] Healthy-peer metric counted stale peers indefinitely**
   - Proof: `healthy_peer_count` previously filtered only cooldown state and ignored peer recency.
   - Impact: Long-stale peers could inflate `p2p_healthy_peer_count`, masking degraded network conditions and delaying AC5 warning visibility.
   - Fix: `healthy_peer_count` now applies a bounded freshness window before considering peers healthy.

3. **[MEDIUM][Fixed] Throttle telemetry undercounted first violation events**
   - Proof: `register_violation` incremented `rejected_total` but not `throttled_total`.
   - Impact: Admin health could show active rejections with understated throttling signals, reducing operator trust in AC4 observability.
   - Fix: `register_violation` now increments `throttled_total`, with regression coverage in `invalid_message_violation_counts_as_throttled`.

### Validation

- `cd server && cargo fmt --check && cargo clippy -- -D warnings && cargo test` ✅
