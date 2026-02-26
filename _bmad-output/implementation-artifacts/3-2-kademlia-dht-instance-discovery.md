# Story 3.2: Kademlia DHT Instance Discovery

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As an **instance operator**,
I want my instance to discover other Discool instances on the network,
so that users can find and join guilds across different instances.

## Acceptance Criteria

1. **Given** the P2P node is running and configured with bootstrap peer addresses (from config)  
   **When** the instance starts  
   **Then** it connects to bootstrap peers and joins the Kademlia DHT.

2. **Given** the instance is connected to the DHT  
   **When** startup completes  
   **Then** the instance registers itself in the DHT with its public address and metadata (instance name, version).

3. **Given** the instance has joined the DHT  
   **When** discovery runs  
   **Then** the instance discovers other registered instances within 60 seconds of coming online.

4. **Given** peers are discovered  
   **When** discovery updates are received  
   **Then** discovered instances are stored locally and periodically refreshed.

5. **Given** all bootstrap peers are unreachable  
   **When** discovery initialization fails  
   **Then** the instance operates in standalone mode and retries with exponential backoff.

6. **Given** the instance is running  
   **When** an admin checks health  
   **Then** P2P network status (discovered instances count, connection count) is exposed via the admin health dashboard.

## Tasks / Subtasks

- [x] Task 1: Extend discovery configuration and startup wiring (AC: 1, 5)
  - [x] Add P2P discovery config for bootstrap peers (multiaddrs) and retry/backoff tuning with safe defaults.
  - [x] Keep validation cross-backend safe (SQLite/Postgres unaffected) and preserve existing `p2p.enabled` behavior.
  - [x] Keep startup non-fatal: discovery/bootstrap failures must log warnings and continue HTTP service.

- [x] Task 2: Implement Kademlia discovery behavior (AC: 1, 2, 3)
  - [x] Create `server/src/p2p/discovery.rs` and compose Kademlia behavior into swarm runtime.
  - [x] Connect to configured bootstrap peers, trigger Kademlia bootstrap/query flow, and emit structured progress logs.
  - [x] Register local instance metadata (name, version, addresses) in DHT after join.
  - [x] Ensure discovery succeeds within 60 seconds under healthy bootstrap conditions.

- [x] Task 3: Persist discovered instances and refresh lifecycle (AC: 4)
  - [x] Add migration for discovered-instance storage with dedup/update semantics (`peer_id` unique, last_seen refresh).
  - [x] Store public peer metadata and addresses only (no private key material).
  - [x] Implement periodic refresh/revalidation of discovered peers.

- [x] Task 4: Expose P2P status in admin health surface (AC: 6)
  - [x] Extend admin health payload to include discovered instance count and active P2P connection count.
  - [x] Reuse existing `/api/v1/admin/health` envelope format (`{ "data": ... }`).
  - [x] Ensure values remain available when running standalone/fallback mode.

- [x] Task 5: Add tests and verification coverage (AC: all)
  - [x] Unit tests for discovery behavior state transitions and bootstrap/retry logic.
  - [x] Integration tests for: successful bootstrap, unreachable bootstrap fallback + retry, and health payload exposure.
  - [x] Regression tests ensuring server still starts and serves health endpoints when P2P discovery is degraded.
  - [x] Run server quality gates: `cd server && cargo fmt --check && cargo clippy -- -D warnings && cargo test`.

## Dev Notes

### Architecture Compliance

1. Keep P2P implementation isolated under `server/src/p2p/` and avoid handler-layer networking logic.  
2. Preserve startup ordering in `main.rs` (config validate → DB init/migrations → P2P bootstrap attempt → HTTP bind).  
3. Preserve graceful degradation: P2P discovery failure must never block local-instance operation.  
4. Keep API response envelopes and error format consistent with existing backend contracts.

### Technical Requirements (Developer Guardrails)

- Build on Story 3.1 foundations (`identity.rs`, `node.rs`, `P2pMetadata`) rather than replacing existing startup/runtime patterns.
- Do not keep discovery state as runtime-only ephemeral data; persist discovered peers for refresh and admin visibility.
- Ensure address handling uses valid multiaddrs and includes peer identity context where required.
- Bootstrap retry must be bounded and jittered exponential backoff (avoid tight loops).
- Log actionable discovery events (`bootstrap attempt`, `bootstrap failure`, `peer discovered`, `peer refreshed`) with `tracing`.
- Keep sensitive data out of logs (no private key bytes, no raw secret config values).

### Library & Framework Requirements

- Keep `rust-libp2p` on **0.56.x** line (already in repo) unless explicitly coordinated as separate upgrade work.
- Use Tokio runtime path (`SwarmBuilder::with_tokio`) consistently; async-std is removed from libp2p 0.56 release line.
- For Kademlia in rust-libp2p, explicitly wire peer addresses from Identify (or equivalent) via `Behaviour::add_address`; otherwise discovery can stall at boot nodes.
- Continue using `tracing` structured logging and existing error handling patterns (`Result<_, AppError>` at handler boundary).

### File Structure Requirements

Expected implementation touch points:

- `server/src/p2p/mod.rs` (export discovery module + metadata extensions)
- `server/src/p2p/node.rs` (compose behavior, event loop, retry path)
- `server/src/p2p/discovery.rs` (new; Kademlia bootstrap/register/discover lifecycle)
- `server/src/config/settings.rs` (bootstrap peer config + validation)
- `server/src/config/mod.rs` (if config exports/loader mapping changes)
- `config.example.toml` (document new `[p2p]` discovery keys)
- `server/src/handlers/admin.rs` (health payload includes P2P discovery status fields)
- `server/migrations/0009_*.sql` (discovered instance persistence)
- `server/tests/*.rs` and/or `server/src/p2p/*.rs` tests (unit + integration coverage)

### Testing Requirements

- Validate Kademlia bootstrap path with at least one deterministic integration scenario.
- Validate unreachable bootstrap peers result in standalone mode plus retry loop (without process crash).
- Validate admin health payload reports discovery/connection counters under both healthy and degraded states.
- Re-run:
  - `cd server && cargo fmt --check`
  - `cd server && cargo clippy -- -D warnings`
  - `cd server && cargo test`

### Previous Story Intelligence

- Story 3.1 already established: persistent instance identity, libp2p startup wiring, and graceful fallback to local-only mode.
- Reuse existing `P2pMetadata` sharing via `AppState` instead of introducing parallel global state.
- Keep prior review fixes intact:
  - P2P validation checks are gated behind `p2p.enabled`.
  - `p2p.listen_port` must remain different from `server.port`.

### Git Intelligence Summary

- Recent implementation pattern for major stories consistently includes:
  - story doc + sprint status updates,
  - config/settings updates,
  - focused module additions,
  - integration/unit tests in the same change set.
- Recent Epic 3.1 commit touched `server/src/p2p/*`, startup wiring in `main.rs`, and config surfaces; Story 3.2 should continue this structure for consistency.

### Latest Technical Information

1. Latest rust-libp2p stable release remains **libp2p-v0.56.0** (published 2025-06-28).  
2. libp2p v0.56.0 release notes explicitly removed async-std support; Tokio is the expected runtime path.  
3. rust-libp2p Kademlia docs note that Identify/address propagation must be explicitly connected to Kademlia (`add_address`) to discover beyond bootstrap peers.  
4. libp2p Kad-DHT bootstrap is periodic and routing-table health depends on repeated bootstrap/lookup cycles.

### Project Context Reference

- No `project-context.md` discovered via `**/project-context.md`.
- Primary context sources for this story are the planning artifacts (`epics.md`, `architecture.md`, `prd.md`, `ux-design-specification.md`) and Story 3.1 implementation history.

### Story Completion Status

- Ultimate context engine analysis completed - comprehensive developer guide created.
- Story marked `ready-for-dev` for implementation handoff.

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Epic 3: P2P Discovery & Federation Foundation]
- [Source: _bmad-output/planning-artifacts/epics.md#Story 3.2: Kademlia DHT Instance Discovery]
- [Source: _bmad-output/planning-artifacts/architecture.md#Core Architectural Decisions]
- [Source: _bmad-output/planning-artifacts/architecture.md#Project Structure & Boundaries]
- [Source: _bmad-output/planning-artifacts/architecture.md#Implementation Patterns & Consistency Rules]
- [Source: _bmad-output/planning-artifacts/prd.md#Functional Requirements]
- [Source: _bmad-output/planning-artifacts/prd.md#P2P & Distributed System Constraints]
- [Source: _bmad-output/planning-artifacts/prd.md#Non-Functional Requirements]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#User Journey Flows]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Component Strategy]
- [Source: server/src/p2p/node.rs]
- [Source: server/src/p2p/identity.rs]
- [Source: server/src/config/settings.rs]
- [Source: server/src/main.rs]
- [Source: server/src/handlers/admin.rs]
- [Source: server/src/handlers/mod.rs]
- [Source: config.example.toml]
- [Source: https://github.com/libp2p/rust-libp2p/releases/tag/libp2p-v0.56.0]
- [Source: https://docs.rs/libp2p/latest/libp2p/kad/]
- [Source: https://libp2p.io/guides/kademlia-dht/]

## Dev Agent Record

### Agent Model Used

GPT-5.3-Codex (Copilot CLI 0.0.418)

### Debug Log References

- Story context generation workflow (create-story) artifact analysis pass.
- `cd server && cargo fmt --check` ✅
- `cd server && cargo clippy -- -D warnings` ✅
- `cd server && cargo test` ✅ (125 unit, 2 DB integration, 42 server integration tests)

### Completion Notes List

- Added P2P discovery config keys for bootstrap peers and retry/refresh tuning with validation-safe defaults.
- Implemented `server/src/p2p/discovery.rs` for Kademlia + Identify behavior wiring, bootstrap peer parsing, backoff logic, and discovered-instance persistence helpers.
- Reworked `server/src/p2p/node.rs` runtime to dial bootstrap peers, publish local metadata records to DHT, discover peers, persist discoveries, and keep standalone retry behavior non-fatal.
- Added migration `0009_discovered_instances.sql` with peer-id dedup and refresh timestamps.
- Extended admin health payload with `p2p_discovered_instances` and `p2p_connection_count`.
- Added integration tests for unreachable-bootstrap fallback and successful bootstrap discovery within startup window.

### File List

- `_bmad-output/implementation-artifacts/3-2-kademlia-dht-instance-discovery.md`
- `_bmad-output/implementation-artifacts/sprint-status.yaml`
- `config.example.toml`
- `server/Cargo.toml`
- `server/Cargo.lock`
- `server/migrations/0009_discovered_instances.sql`
- `server/src/config/settings.rs`
- `server/src/handlers/admin.rs`
- `server/src/main.rs`
- `server/src/p2p/discovery.rs`
- `server/src/p2p/mod.rs`
- `server/src/p2p/node.rs`
- `server/tests/server_binds_to_configured_port.rs`

## Change Log

- 2026-02-26: Implemented Story 3.2 Kademlia discovery runtime, discovered-peer persistence, admin health P2P metrics, and full quality-gate verification.
- 2026-02-26: Senior code review found and fixed retry backoff scheduling for unreachable bootstrap peers and bootstrap peer whitespace parsing consistency.

## Senior Developer Review (AI)

- Outcome: Changes requested and fixed in this review pass.
- High fixed:
  - Retry backoff scheduling did not advance when bootstrap attempts were accepted but not connected, causing overly frequent retry dials in unreachable-peer scenarios.
    - Fixed in `server/src/p2p/node.rs` by scheduling `next_retry` for every retry attempt and resetting retry state only on established connections.
- Medium fixed:
  - Bootstrap peer runtime parsing used raw strings while validation accepted trimmed values, allowing runtime/validation mismatch with whitespace-padded multiaddrs.
    - Fixed in `server/src/p2p/discovery.rs` by trimming input in `parse_bootstrap_peer`; added unit test `parse_bootstrap_peer_trims_whitespace`.
- Remaining HIGH/MEDIUM findings: none.
