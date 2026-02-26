# Story 3.3: Gossipsub Inter-Instance Communication

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a **system**,
I want instances to exchange events via Gossipsub pub/sub,
so that cross-instance operations (identity verification, guild discovery) can function.

## Acceptance Criteria

1. **Given** two or more instances have discovered each other via Kademlia  
   **When** an instance publishes an event to a Gossipsub topic  
   **Then** all subscribed instances receive the event.

2. **Given** Gossipsub topics are used for inter-instance operations  
   **When** topics are created and subscribed  
   **Then** topics are namespaced by protocol version (`/discool/gossip/1.0.0/...`).

3. **Given** an instance publishes an inter-instance event  
   **When** the event is emitted over Gossipsub  
   **Then** messages are signed by the sending instance keypair and verified by receivers.

4. **Given** a receiver processes incoming Gossipsub messages  
   **When** a message is invalid or unsigned  
   **Then** it is rejected and logged.

5. **Given** events are propagated over the inter-instance mesh  
   **When** multiple peers are active  
   **Then** delivery is best-effort (eventual consistency, not guaranteed ordering).

6. **Given** operators deploy Discool across small-to-medium networks  
   **When** mesh defaults are applied  
   **Then** Gossipsub mesh parameters are tuned for tens to hundreds of instances.

## Tasks / Subtasks

- [x] Task 1: Extend P2P behavior with Gossipsub and protocol-versioned topics (AC: 1, 2, 6)
  - [x] Enable and wire `libp2p` Gossipsub behavior into existing discovery/runtime composition.
  - [x] Define canonical topic helpers/constants under `/discool/gossip/1.0.0/...`.
  - [x] Apply mesh defaults appropriate for small-to-medium deployments.

- [x] Task 2: Implement signed publish/subscribe event flow (AC: 1, 3, 5)
  - [x] Add typed event envelope(s) for inter-instance operations on Gossipsub.
  - [x] Publish with sender authenticity bound to the instance identity keypair.
  - [x] Consume subscribed messages and forward valid events into service-layer handlers.

- [x] Task 3: Enforce inbound validation and rejection behavior (AC: 3, 4)
  - [x] Verify message authenticity/signature on receive before processing.
  - [x] Reject and structured-log unsigned, malformed, or invalid messages with peer context.
  - [x] Keep invalid-message handling non-fatal to runtime stability.

- [x] Task 4: Preserve architecture boundaries and runtime resilience (AC: 1, 5)
  - [x] Keep P2P event ingress in `server/src/p2p/` and route domain effects through services (no direct DB access from P2P module).
  - [x] Maintain existing graceful degradation model: HTTP/API startup continues even with degraded P2P/gossip behavior.
  - [x] Ensure behavior remains best-effort and avoids introducing ordering guarantees.

- [x] Task 5: Add/adjust configuration and operator-facing defaults (AC: 2, 6)
  - [x] Add any required gossip settings to config model with safe defaults.
  - [x] Document new config in `config.example.toml`.
  - [x] Keep validation compatible with existing guardrails (`p2p.enabled` gating, port constraints).

- [x] Task 6: Add test coverage and run quality gates (AC: all)
  - [x] Unit tests for topic naming/versioning and message validation paths.
  - [x] Integration tests with at least two instances for valid propagation and invalid-message rejection.
  - [x] Regression test that server startup remains non-fatal when gossip initialization/dependencies are degraded.
  - [x] Run: `cd server && cargo fmt --check && cargo clippy -- -D warnings && cargo test`.

## Dev Notes

### Architecture Compliance

1. P2P is a first-class subsystem and must remain under `server/src/p2p/`.  
2. Respect data-flow boundary: incoming P2P event → `p2p` → service → model → ws/dispatcher.  
3. Do not allow `p2p/*` modules to query database tables directly.  
4. Keep protocol versioning explicit for inter-instance contracts.

### Technical Requirements (Developer Guardrails)

- Build on existing Story 3.1/3.2 foundations (`identity.rs`, `discovery.rs`, `node.rs`, `P2pMetadata`) instead of replacing runtime patterns.
- Reuse existing Kademlia-discovered peers; do not introduce an alternate peer discovery path for gossip.
- Treat delivery as eventual consistency; do not add product assumptions about total ordering.
- Reject unsigned/invalid inbound messages and log actionable diagnostics (remote peer, reason) without leaking sensitive data.
- Keep startup resilient: gossip feature issues must not block local-only instance operation.

### Library & Framework Requirements

- Keep `rust-libp2p` on the existing **0.56.x** line and Tokio runtime path.
- Extend libp2p features to include Gossipsub support in the current dependency baseline.
- Use explicit topic versioning (`/discool/gossip/1.0.0/...`) to isolate incompatible protocol revisions.
- Apply libp2p Gossipsub validation/peer-scoring patterns for invalid-message control.

### File Structure Requirements

Expected implementation touch points:

- `server/Cargo.toml` (libp2p gossipsub feature wiring)
- `server/src/p2p/mod.rs` (module exports/state)
- `server/src/p2p/discovery.rs` (behavior composition and shared protocol config)
- `server/src/p2p/node.rs` (swarm event loop, publish/subscribe handling)
- `server/src/p2p/gossip.rs` (new, if extracting topic/envelope/validation logic)
- `server/src/p2p/protocol.rs` (new, if centralizing protocol IDs/topics)
- `server/src/config/settings.rs` (optional gossip config defaults/validation)
- `config.example.toml` (operator documentation for gossip tunables)
- `server/src/p2p/*.rs` unit tests and `server/tests/*.rs` integration coverage

### Testing Requirements

- Verify valid event propagation across at least two instances.
- Verify invalid/unsigned messages are rejected and logged.
- Verify topic namespace/version usage is consistent and enforced.
- Verify existing Kademlia/discovery behavior remains functional after gossip integration.
- Re-run:
  - `cd server && cargo fmt --check`
  - `cd server && cargo clippy -- -D warnings`
  - `cd server && cargo test`

### Previous Story Intelligence

- Story 3.1 established stable instance identity and startup graceful-degradation behavior; preserve both.
- Story 3.2 established discovery persistence, retry/backoff, and admin P2P status surfaces; integrate gossip into this runtime rather than parallel paths.
- Keep prior validation fixes intact:
  - P2P-specific config validation should be gated by `p2p.enabled`.
  - `p2p.listen_port` must remain distinct from `server.port`.

### Git Intelligence Summary

- Recent work follows scoped Conventional Commits (`feat:`) and story-by-story progression.
- Keep implementation scoped to Story 3.3 deliverables and include tests in the same change set.

### Latest Technical Information

1. Latest stable `rust-libp2p` release remains **v0.56.0** and runtime guidance is Tokio-first.  
2. Gossipsub best practice is explicit message validation with accept/reject outcomes and peer-scoring penalties for invalid traffic.  
3. Topic versioning in names (e.g., `/discool/gossip/1.0.0/...`) is recommended to avoid cross-version protocol ambiguity.  
4. Gossipsub relies on external peer discovery (already provided here by Kademlia).

### Project Context Reference

- No `project-context.md` discovered via `**/project-context.md`.
- Authoritative context for this story comes from planning artifacts and completed Epic 3 story documents.

### Story Completion Status

- Ultimate context engine analysis completed - comprehensive developer guide created.
- Story marked `ready-for-dev` for implementation handoff.

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Story 3.3: Gossipsub Inter-Instance Communication]
- [Source: _bmad-output/planning-artifacts/architecture.md#Core Architectural Decisions]
- [Source: _bmad-output/planning-artifacts/architecture.md#Project Structure & Boundaries]
- [Source: _bmad-output/planning-artifacts/architecture.md#Implementation Patterns & Consistency Rules]
- [Source: _bmad-output/planning-artifacts/prd.md#Instance Management]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Journey 3: Tomás Deploys and Launches His Instance]
- [Source: _bmad-output/implementation-artifacts/3-1-libp2p-node-bootstrap-and-instance-identity.md]
- [Source: _bmad-output/implementation-artifacts/3-2-kademlia-dht-instance-discovery.md]
- [Source: server/src/p2p/mod.rs]
- [Source: server/src/p2p/discovery.rs]
- [Source: server/src/p2p/node.rs]
- [Source: server/Cargo.toml]
- [Source: https://github.com/libp2p/rust-libp2p/releases]
- [Source: https://libp2p.github.io/rust-libp2p/libp2p/gossipsub/index.html]
- [Source: https://github.com/libp2p/specs/blob/master/pubsub/README.md#message-validation-and-peer-scoring]

## Senior Developer Review (AI)

Reviewer: Darko  
Date: 2026-02-26  
Outcome: Approve

- Git vs Story discrepancies: none in source/test/config files relevant to Story 3.3.
- Acceptance Criteria verification: all ACs implemented.
- Findings: 0 HIGH, 0 MEDIUM, 0 LOW.
- Quality gates re-run during review: `cd server && cargo fmt --check && cargo clippy -- -D warnings && cargo test` ✅

## Dev Agent Record

### Agent Model Used

GPT-5.3-Codex (model ID: gpt-5.3-codex)

### Debug Log References

- Baseline quality gates (pre-change): `cd server && cargo fmt --check && cargo clippy -- -D warnings && cargo test` ✅
- Final quality gates (post-change): `cd server && cargo fmt --check && cargo clippy -- -D warnings && cargo test` ✅

### Completion Notes List

- Added `server/src/p2p/gossip.rs` with canonical versioned topics under `/discool/gossip/1.0.0/...`, mesh defaults, typed envelope/event definitions, and inbound validation helpers.
- Extended `DiscoveryBehaviour` to compose Gossipsub with Identify+Kademlia and subscribe to inter-instance topics at startup.
- Updated `node.rs` to publish periodic signed guild-discovery gossip announcements, validate inbound messages (including source/sender matching), reject invalid payloads, and route valid events through service-layer handling.
- Added `server/src/services/p2p_event_service.rs` and wired `server/src/services/mod.rs` so P2P ingress hands off domain effects through services.
- Added configurable gossip mesh defaults (`gossip_mesh_n_low`, `gossip_mesh_n`, `gossip_mesh_n_high`) in config model + validation and documented them in `config.example.toml`.
- Added tests for topic/versioning and validation paths plus two-instance propagation/rejection integration coverage in `server/tests/p2p_gossip_inter_instance.rs`.
- Verified non-fatal degraded startup behavior remains covered by existing integration tests (`server_stays_up_when_p2p_startup_fails`, `server_stays_up_with_unreachable_bootstrap_peer`).

### File List

- `_bmad-output/implementation-artifacts/3-3-gossipsub-inter-instance-communication.md`
- `_bmad-output/implementation-artifacts/sprint-status.yaml`
- `config.example.toml`
- `server/Cargo.toml`
- `server/Cargo.lock`
- `server/src/config/settings.rs`
- `server/src/p2p/mod.rs`
- `server/src/p2p/discovery.rs`
- `server/src/p2p/node.rs`
- `server/src/p2p/gossip.rs`
- `server/src/services/mod.rs`
- `server/src/services/p2p_event_service.rs`
- `server/tests/p2p_gossip_inter_instance.rs`

## Change Log

- 2026-02-26: Implemented Story 3.3 Gossipsub inter-instance messaging with versioned topics, signed publish/subscribe flow, strict inbound validation/rejection, mesh config defaults, service routing, and full server quality-gate verification.
- 2026-02-26: Senior Developer Review (AI) completed in YOLO mode; no HIGH/MEDIUM/LOW findings, no code fixes required, and story approved as done.
