# Story 3.5: Private Instance and Discovery Opt-Out

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As an **instance operator**,
I want to opt my instance out of P2P discovery,
so that my instance operates as an unlisted, private deployment.

## Acceptance Criteria

1. **Given** the operator sets `discovery.enabled = false` in configuration (or disables discovery during first-run setup)  
   **When** the server starts  
   **Then** the instance does not publish local metadata into Kademlia DHT.

2. **Given** discovery is disabled  
   **When** remote instances refresh discovered peers  
   **Then** this instance does not appear in remote discovered-instance lists.

3. **Given** discovery is disabled  
   **When** users access the instance directly by URL  
   **Then** core instance access and invite flows continue to work.

4. **Given** discovery is disabled  
   **When** users from other instances join via direct invite links  
   **Then** cross-instance identity verification still works without DHT discovery.

5. **Given** an operator re-enables discovery and restarts  
   **When** P2P runtime boots  
   **Then** DHT publishing/discovery resumes normally.

6. **Given** an operator opens Admin health view  
   **When** discovery is disabled  
   **Then** P2P status clearly shows `Discovery: Disabled (Unlisted)`.

## Tasks / Subtasks

- [x] Task 1: Define effective discovery toggle and precedence (AC: 1, 5)
  - [x] Add explicit discovery-mode config/settings support aligned with existing `p2p.enabled` semantics (runtime on/off remains separate from discovery advertisement on/off).
  - [x] Reuse existing `instance_settings.discovery_enabled` first-run/admin setting as an input for effective discovery mode (with deterministic precedence and sensible fallback).
  - [x] Document the toggle in `config.example.toml` and keep config validation behavior explicit.

- [x] Task 2: Gate DHT publication/discovery paths when unlisted (AC: 1, 2, 5)
  - [x] In `server/src/p2p/node.rs`, gate calls that advertise/discover via DHT (`put_record`, `get_closest_peers`, bootstrap/retry loops) on effective discovery mode.
  - [x] Keep libp2p runtime alive when `p2p.enabled=true` but discovery is disabled; do not silently downgrade into full `p2p.enabled=false`.
  - [x] Ensure logs clearly indicate discovery-disabled mode without treating it as an error condition.

- [x] Task 3: Preserve direct-access and direct-invite behavior without DHT (AC: 3, 4)
  - [x] Verify direct URL access and normal REST/WebSocket flows are unaffected.
  - [x] Verify invite-based join and identity verification paths do not depend on DHT advertisement.
  - [x] Avoid introducing regressions to graceful-degradation behavior from Epic 3.4.

- [x] Task 4: Surface unlisted discovery state in admin APIs/UI (AC: 6)
  - [x] Extend backend health/status payloads with explicit discovery mode fields (boolean + operator-facing label).
  - [x] Update client API typings and `AdminPanel.svelte` rendering to show `Discovery: Disabled (Unlisted)` when appropriate.
  - [x] Keep `{ "data": ... }` response envelopes and existing fields backward-compatible.

- [x] Task 5: Add regression coverage and run quality gates (AC: all)
  - [x] Add/update unit/integration tests around discovery toggle parsing/effective resolution and p2p behavior gating.
  - [x] Add/update tests validating admin health response + UI-visible status semantics.
  - [x] Run server quality gates: `cd server && cargo fmt --check && cargo clippy -- -D warnings && cargo test`.

## Dev Notes

### Developer Context

- Build directly on Epic 3 foundations already implemented in Stories 3.1–3.4 (`server/src/p2p/{identity,discovery,gossip,node,sybil}.rs`).
- Keep the distinction explicit:
  - `p2p.enabled=false` means runtime disabled.
  - Discovery disabled means runtime remains available but DHT advertisement/discovery is suppressed.
- Existing first-run setup already captures `discovery_enabled` in `instance_settings`; Story 3.5 should wire this into P2P runtime behavior and operator-visible status.

### Technical Requirements

- Do not remove/replace existing sybil-health and degraded-network logic; Story 3.5 is mode-gating, not anti-abuse redesign.
- Preserve startup resilience: P2P failures remain non-fatal for core server startup.
- Keep DB compatibility across PostgreSQL + SQLite (current dual-backend pattern in handlers/config/discovery).
- Maintain explicit, structured logs (`tracing`) for discovery mode transitions and suppressed advertisement behavior.

### Architecture Compliance

1. Keep layer boundaries intact: handlers expose HTTP state, p2p runtime handles network behavior, DB access through existing query helpers.  
2. Reuse discovery helper patterns in `server/src/p2p/discovery.rs` instead of duplicating DB lookup logic in multiple places.  
3. Preserve current protocol namespaces and wire contracts (`/discool/kad/1.0.0`, `/discool/gossip/1.0.0/...`).  
4. Preserve API envelope contract (`{ "data": ... }`) and snake_case boundary fields.

### Library & Framework Requirements

- Keep `rust-libp2p` on project baseline (`0.56.x`) with Tokio runtime usage.
- Use existing Kademlia/Gossipsub behavior composition; avoid introducing alternate discovery stacks.
- Avoid new dependencies unless absolutely required; prefer existing crate capabilities and internal helpers.

### File Structure Requirements

Primary touch points (expected):

- `server/src/config/settings.rs`
- `config.example.toml`
- `server/src/handlers/instance.rs`
- `server/src/handlers/admin.rs`
- `server/src/p2p/mod.rs`
- `server/src/p2p/discovery.rs`
- `server/src/p2p/node.rs`
- `client/src/lib/api.ts`
- `client/src/lib/components/AdminPanel.svelte`
- `server/tests/*.rs` and targeted client tests for changed API/UI behavior

### Testing Requirements

- Validate discovery-disabled mode suppresses DHT advertisement/query activity while keeping direct connectivity paths operational.
- Validate admin health payload includes discovery mode semantics and remains backward-compatible for existing fields.
- Validate setup/config toggles correctly propagate to effective runtime discovery mode.
- Re-run server quality gates:
  - `cd server && cargo fmt --check`
  - `cd server && cargo clippy -- -D warnings`
  - `cd server && cargo test`

### Previous Story Intelligence

- Story 3.4 established anti-abuse enforcement and health telemetry in `node.rs`; Story 3.5 should integrate by gating discovery behavior, not bypassing these controls.
- Story 3.4 expanded `P2pMetadata` and admin health payloads; extend these patterns consistently for discovery-mode visibility.
- Keep implementation scope tight around config, p2p runtime, admin status surfacing, and tests (matching recent Epic 3 change patterns).

### Git Intelligence Summary

- Recent Epic 3 commits concentrated changes in `server/src/p2p/*`, `server/src/config/settings.rs`, `server/src/handlers/admin.rs`, and `config.example.toml`.
- Follow the same file-locality pattern for Story 3.5 to reduce regression risk and preserve architectural consistency.

### Latest Technical Information

1. `rust-libp2p` stable baseline remains **v0.56.0**; runtime support is Tokio-first in current release guidance.
2. Kademlia peer discovery in rust-libp2p still requires explicit behavior wiring; avoid assumptions about automatic discovery coupling.
3. Kademlia `put_record`/provider publication APIs are explicit advertisement points; suppress these calls in unlisted mode.
4. Unlisted/private operation should rely on explicit direct peer dialing/known addresses rather than public discovery advertisement.

### Project Context Reference

- No `project-context.md` discovered via `**/project-context.md`.
- Authoritative context comes from planning artifacts, Epic 3 completed story artifacts, and current server/client implementation files.

### Story Completion Status

- Ultimate context engine analysis completed - comprehensive developer guide created.
- Story marked `ready-for-dev`.

### Project Structure Notes

- Fully aligned with current structure (`server/src/p2p`, `server/src/handlers`, `server/src/config`, `client/src/lib/components`, `client/src/lib/api`).
- No structural variance required for this story.

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Epic 3: P2P Discovery & Federation Foundation]
- [Source: _bmad-output/planning-artifacts/epics.md#Story 3.5: Private Instance and Discovery Opt-Out]
- [Source: _bmad-output/planning-artifacts/prd.md#P2P & Distributed System Constraints]
- [Source: _bmad-output/planning-artifacts/architecture.md#Core Architectural Decisions]
- [Source: _bmad-output/planning-artifacts/architecture.md#Project Structure & Boundaries]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Journey 3: Instance Deployment & First Run (Tomas)]
- [Source: _bmad-output/implementation-artifacts/3-4-sybil-resistance-and-network-health.md]
- [Source: server/src/p2p/node.rs]
- [Source: server/src/p2p/discovery.rs]
- [Source: server/src/config/settings.rs]
- [Source: server/src/handlers/instance.rs]
- [Source: server/src/handlers/admin.rs]
- [Source: client/src/lib/components/SetupPage.svelte]
- [Source: client/src/lib/components/AdminPanel.svelte]
- [Source: client/src/lib/api.ts]
- [Source: config.example.toml]
- [Source: https://libp2p.io/releases/2025-06-28-rust-libp2p/]
- [Source: https://github.com/libp2p/rust-libp2p/releases]
- [Source: https://libp2p.github.io/rust-libp2p/libp2p/kad/struct.Behaviour.html]
- [Source: https://docs.rs/libp2p/latest/libp2p/]

## Dev Agent Record

### Agent Model Used

GPT-5.3-Codex (model ID: gpt-5.3-codex)

### Debug Log References

- Workflow engine loaded: `_bmad/core/tasks/workflow.xml`
- Workflow config loaded: `_bmad/bmm/workflows/4-implementation/dev-story/workflow.yaml`
- Implementation + validation run: `cd server && cargo fmt --check && cargo clippy -- -D warnings && cargo test`
- Client validation run: `cd client && npm run lint && npm run check && npm run test`

### Completion Notes List

- Added explicit `p2p.discovery.enabled` config support with defaults, config docs, and tests.
- Added discovery helper functions for parsing/loading `instance_settings.discovery_enabled`, effective-mode resolution, and operator-facing mode labels.
- Gated DHT bootstrap/refresh advertisement paths in `server/src/p2p/node.rs` on effective discovery mode while preserving runtime startup.
- Extended admin health payload with `p2p_discovery_enabled` and `p2p_discovery_label` and wired client API + AdminPanel rendering.
- Added/updated regression coverage for config parsing, admin health payload shape, UI rendering, and discovery-disabled unlisted runtime behavior.

### File List

- _bmad-output/implementation-artifacts/3-5-private-instance-and-discovery-opt-out.md
- _bmad-output/implementation-artifacts/sprint-status.yaml
- config.example.toml
- server/src/config/settings.rs
- server/src/p2p/discovery.rs
- server/src/p2p/node.rs
- server/src/handlers/instance.rs
- server/src/handlers/admin.rs
- server/tests/server_binds_to_configured_port.rs
- client/src/lib/api.ts
- client/src/lib/components/AdminPanel.svelte
- client/src/lib/components/AdminPanel.test.ts

## Change Log

- 2026-02-26: Story created and marked ready-for-dev with comprehensive implementation context and guardrails.
- 2026-02-26: Implemented discovery opt-out mode gating, admin/UI discovery status surfacing, and regression tests; story moved to review.
- 2026-02-26: Senior developer review completed; fixed standalone-mode health tracking in `server/src/p2p/node.rs` and added unit coverage.

## Senior Developer Review (AI)

- Outcome: **Approved after fixes** (no remaining HIGH/MEDIUM findings).
- Findings fixed:
  - Corrected standalone mode initialization to reflect actual runtime state when discovery is disabled or no peers are connected.
  - Recomputed standalone mode on connection established/closed events to prevent stale health state.
  - Added focused unit tests for standalone-mode derivation.
- Validation rerun:
  - `cd server && cargo fmt --check && cargo clippy -- -D warnings && cargo test`
