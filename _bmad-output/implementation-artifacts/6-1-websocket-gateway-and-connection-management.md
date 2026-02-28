# Story 6.1: WebSocket Gateway and Connection Management

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a **user**,
I want a persistent real-time connection to the server,
so that I receive messages and events instantly without polling.

## Acceptance Criteria

1. **Given** a user is authenticated  
   **When** the SPA establishes a WebSocket connection to `/ws`  
   **Then** the server authenticates the connection using the session token

2. **Given** a WebSocket message is exchanged  
   **When** protocol payloads are serialized/deserialized  
   **Then** the protocol uses JSON envelopes: `{"op": "...", "d": {...}, "s": 42, "t": ...}`

3. **Given** server-side events are emitted  
   **When** subscribed clients receive updates  
   **Then** supported server events include: `message_create`, `message_update`, `message_delete`, `typing_start`, `presence_update`, `guild_update`, `channel_update`

4. **Given** a client sends a real-time operation  
   **When** the gateway validates the operation name  
   **Then** client operations are prefixed with `c_` (for example `c_message_create`, `c_typing_start`)

5. **Given** multiple clients are connected across guilds/channels  
   **When** the server broadcasts an event  
   **Then** the server tracks connected clients per guild/channel for targeted event broadcasting

6. **Given** messages are delivered in real time  
   **When** latency is measured for same-region traffic  
   **Then** message delivery latency remains under 100ms (NFR1)

7. **Given** a WebSocket connection drops  
   **When** network connectivity is restored  
   **Then** the client auto-reconnects within 5 seconds using exponential backoff (NFR25)

8. **Given** the client is reconnecting  
   **When** the user is still interacting with the UI  
   **Then** the UI shows a non-blocking `"Reconnecting..."` status bar message instead of a blocking overlay

9. **Given** a client exceeds operation throughput limits  
   **When** too many messages are sent  
   **Then** per-connection rate limiting rejects excessive operations with an explicit error event (NFR15)

## Tasks / Subtasks

- [x] Task 1: Establish authenticated gateway lifecycle on `/ws` (AC: 1, 2, 4)
  - [x] Keep `server/src/handlers/ws.rs` as the upgrade entrypoint, but move frame/protocol logic into reusable gateway internals (`server/src/ws/*` or equivalent module split).
  - [x] Reuse existing session validation behavior (`auth_service::validate_session`) and preserve token sources already supported in runtime (`?token=` and `Authorization: Bearer ...`).
  - [x] Normalize handshake failures to structured unauthorized/validation responses and ensure malformed envelopes do not crash the connection task.

- [x] Task 2: Define the canonical WebSocket protocol contract (AC: 2, 3, 4)
  - [x] Add typed envelope and payload contracts for `{ op, d, s, t }` with explicit serialization/deserialization tests.
  - [x] Define server event op constants/enums for the Story 6.1 event set, including `presence_update` compatibility.
  - [x] Define client op constants/enums with enforced `c_` prefix and return explicit protocol errors for unknown/invalid operations.

- [x] Task 3: Implement connection registry + targeted fanout (AC: 3, 5)
  - [x] Track per-connection metadata (user id, guild subscriptions, active channel, last sequence) and expose safe registration/unregistration hooks.
  - [x] Introduce subscribe/unsubscribe semantics for guild/channel routing so events are not broadcast globally.
  - [x] Preserve and integrate existing presence broadcasting behavior from Story 5.7 while evolving to guild/channel-targeted dispatch.

- [x] Task 4: Add sequence, heartbeat, and reconnect-resume guardrails (AC: 6, 7)
  - [x] Include monotonically increasing sequence `s` and server timestamp `t` on outbound events.
  - [x] Add handshake/hello metadata (protocol version + connection/session metadata) and heartbeat acknowledgment behavior.
  - [x] Add resume hooks for reconnect scenarios (sequence-aware replay boundary), without implementing full message-history replay yet (Story 6.3 scope).

- [x] Task 5: Enforce per-connection rate limiting with explicit error events (AC: 9)
  - [x] Apply rate limiting for inbound `c_*` ops (especially message/typing paths) in the WS gateway path.
  - [x] Return machine-readable WS error events compatible with existing API error shape semantics (`code`, `message`, `details`).
  - [x] Add structured tracing for limit hits (user/session/op/connection id) without leaking sensitive payloads.

- [x] Task 6: Build shared client WS lifecycle + non-blocking reconnect UX (AC: 7, 8)
  - [x] Introduce a shared client WS module (`client/src/lib/ws/*`) with single-connection lifecycle states: `connecting`, `connected`, `reconnecting`, `disconnected`.
  - [x] Cap exponential reconnect timing to satisfy the <=5s reconnect expectation for restored connectivity.
  - [x] Surface reconnect state in chat shell UI (`MessageArea.svelte`/`ShellRoute.svelte`) using plain-language status text and no blocking overlay.
  - [x] Avoid dual-socket drift by integrating existing `presenceStore.svelte.ts` with shared WS lifecycle (adapter or migration path) instead of independent long-term sockets.

- [x] Task 7: Add coverage and run quality gates (AC: all)
  - [x] Extend `server/tests/server_binds_to_configured_port.rs` with authenticated WS handshake, protocol validation, and targeted-broadcast/rate-limit cases.
  - [x] Add server unit tests for WS protocol parsing, sequence progression, subscription routing, and reconnect timeout behavior.
  - [x] Add/extend client tests for reconnect backoff state transitions and visible `"Reconnecting..."` status rendering.
  - [x] Run quality gates:
    - [x] `cd client && npm run lint && npm run check && npm run test && npm run build`
    - [x] `cd server && cargo fmt --check && cargo clippy -- -D warnings && cargo test`

## Dev Notes

### Developer Context

- Story 5.7 already replaced the `/ws` stub with authenticated WebSocket handling and introduced presence heartbeat/timeouts; Story 6.1 must extend that foundation, not rebuild from scratch.
- Current chat surface (`MessageArea.svelte`) is still placeholder-based and does not yet consume message events, so Story 6.1 should deliver transport/gateway infrastructure without prematurely implementing full message CRUD scope from Story 6.2+.
- Current client presence flow opens `/ws` directly from `presenceStore.svelte.ts`; Story 6.1 should avoid permanent parallel socket architecture and define a shared WS lifecycle path.
- Current server integration tests already assert unauthorized WS upgrades return `401`, which should remain true after gateway refactor.

### Technical Requirements

- Keep `/ws` as the real-time entrypoint and preserve authenticated token resolution behavior already implemented in `server/src/handlers/ws.rs`.
- Use the architecture-defined WS envelope contract (`op`, `d`, `s`, `t`) and `snake_case` op naming, with `c_` prefix for client-sent ops.
- Include Story 6.1 server event names in protocol definitions even if some events are emitted by later stories.
- Implement targeted event routing by guild/channel subscription tracking; global fanout is insufficient for Epic 6 scale.
- Enforce per-connection rate limiting with explicit error events and keep server/client messaging plain and actionable.
- Meet reconnect UX requirements: automatic reconnect with exponential backoff and visible non-blocking `"Reconnecting..."` state text.
- Track delivery latency metrics for NFR1 validation readiness; instrumentation should be added as part of gateway events.
- Out of scope for this story: message persistence schema + timeline rendering behavior from Stories 6.2/6.3.

### Architecture Compliance

1. Keep Axum layering intact: handlers handle HTTP upgrade boundaries, WS gateway modules handle frame/protocol state, services hold business logic.
2. Preserve API/WS error consistency with structured error payload semantics (`code`, `message`, `details`) and sanitized messages.
3. Keep `ws` transport logic separate from domain logic (`services/*`), consistent with architecture boundary guidance.
4. Preserve frontend state boundaries: real-time event state from WS store, REST data in existing feature stores/APIs.
5. Preserve existing permission and auth models; WS session validation must remain server-authoritative.
6. Preserve existing Story 5.7 presence behavior while introducing targeted subscriptions and sequence-aware gateway flows.

### Library & Framework Requirements

- Backend: Rust + Axum `0.8` with built-in WS support, Tokio async runtime, existing tracing/error infrastructure.
- Frontend: Svelte `5.x` runes with existing feature stores; no new state library required.
- Keep existing pinned dependencies unless a blocker is proven; Story 6.1 does not require dependency upgrades.

### File Structure Requirements

Expected primary touch points:

- `server/src/handlers/ws.rs`
- `server/src/handlers/mod.rs`
- `server/src/services/presence_service.rs`
- `server/src/services/mod.rs`
- `server/src/lib.rs` (if shared gateway state wiring is needed)
- `server/src/middleware/auth.rs` (token parsing reuse/alignment)
- `server/tests/server_binds_to_configured_port.rs`
- `client/src/lib/features/members/presenceStore.svelte.ts`
- `client/src/lib/features/chat/MessageArea.svelte`
- `client/src/lib/features/shell/ShellRoute.svelte`
- `client/src/lib/features/shell/ShellRoute.test.ts`
- `client/src/lib/features/members/MemberList.svelte` (presence integration touchpoint if needed)
- `client/src/lib/ws/client.ts` (new)
- `client/src/lib/ws/protocol.ts` (new)

### Testing Requirements

- Server integration tests:
  - unauthenticated WS upgrades still return `401`,
  - authenticated upgrades establish WS protocol handshake successfully,
  - invalid envelopes / unknown ops return explicit protocol errors,
  - targeted fanout reaches subscribed clients only,
  - per-connection rate limiting rejects excessive ops deterministically.
- Server unit tests:
  - envelope parsing/serialization,
  - sequence counter progression and timestamp inclusion,
  - heartbeat + reconnect state handling,
  - guild/channel subscription registry behavior.
- Frontend tests:
  - reconnect state transitions and capped backoff behavior,
  - `"Reconnecting..."` status visibility without blocking interaction,
  - presence update flow remains functional after shared WS integration.

### Previous Story Intelligence

- Story 5.7 introduced authenticated `/ws` handling, presence status transitions (`online`/`idle`/`offline`), and client-side heartbeat/reconnect logic.
- Story 5.7 also established `presence_update` event shape and envelope conventions that Story 6.1 must preserve for backward compatibility.
- Story 5.7 quality gates and review fixes hardened heartbeat behavior; Story 6.1 should not regress those edge-case protections.

### Git Intelligence Summary

- `e1b2e8a` feat: finalize story 5-7 member list with presence
- `afa9352` feat: finalize story 5-6 role management delegation
- `2491aec` feat: finalize story 5-5 channel permission overrides
- `68f87aa` feat: finalize story 5-4 role assignment to members
- `f9149f0` feat: finalize story 5-3 role hierarchy and ordering

### Latest Technical Information

Current pinned runtime lines in this repo:

- `svelte`: `^5.45.2`
- `@mateothegreat/svelte5-router`: `^2.16.19`
- `axum`: `0.8`
- `sqlx`: `0.8`
- `libp2p`: `0.56`

Latest stable lines checked during story creation:

- Svelte latest: `5.53.6`
- `@mateothegreat/svelte5-router` latest: `2.16.19`
- Axum latest release tag: `axum-v0.8.8`
- SQLx latest tag: `v0.8.6`
- rust-libp2p latest release tag: `libp2p-v0.56.0`

No dependency upgrade is required for Story 6.1; implementation should target currently pinned project versions.

### Project Context Reference

- No `project-context.md` file was discovered via `**/project-context.md`.
- Story context is derived from planning artifacts, existing implementation artifacts, current source code, and recent git history.

### Story Completion Status

- Ultimate context engine analysis completed - comprehensive developer guide created.
- Story status set to `ready-for-dev`.

### Project Structure Notes

- The runtime currently uses `server/src/handlers/ws.rs` + `services/presence_service.rs` for basic real-time presence; Story 6.1 should evolve this into full gateway scaffolding rather than replacing it with parallel patterns.
- `client/vite.config.ts` already proxies `/ws` to the backend during development; new client WS modules should keep this assumption.
- `ShellRoute` and `MessageArea` already provide stable surfaces for introducing non-blocking connection status feedback.

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Epic 6: Real-Time Text Communication]
- [Source: _bmad-output/planning-artifacts/epics.md#Story 6.1: WebSocket Gateway and Connection Management]
- [Source: _bmad-output/planning-artifacts/epics.md#Story 6.2 through Story 6.12]
- [Source: _bmad-output/planning-artifacts/prd.md#FR30-FR38]
- [Source: _bmad-output/planning-artifacts/prd.md#FR62]
- [Source: _bmad-output/planning-artifacts/prd.md#NFR1]
- [Source: _bmad-output/planning-artifacts/prd.md#NFR15]
- [Source: _bmad-output/planning-artifacts/prd.md#NFR25]
- [Source: _bmad-output/planning-artifacts/architecture.md#API & Communication Patterns]
- [Source: _bmad-output/planning-artifacts/architecture.md#WebSocket Event Naming]
- [Source: _bmad-output/planning-artifacts/architecture.md#Communication Patterns]
- [Source: _bmad-output/planning-artifacts/architecture.md#Project Structure & Boundaries]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Emotional Journey Mapping]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Journey Patterns]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Feedback Patterns]
- [Source: _bmad-output/implementation-artifacts/5-7-member-list-with-presence.md]
- [Source: _bmad-output/implementation-artifacts/sprint-status.yaml]
- [Source: server/src/handlers/ws.rs]
- [Source: server/src/services/presence_service.rs]
- [Source: server/src/handlers/mod.rs]
- [Source: server/src/middleware/auth.rs]
- [Source: server/src/error.rs]
- [Source: server/tests/server_binds_to_configured_port.rs]
- [Source: client/src/lib/features/members/presenceStore.svelte.ts]
- [Source: client/src/lib/features/chat/MessageArea.svelte]
- [Source: client/src/lib/features/shell/ShellRoute.svelte]
- [Source: client/vite.config.ts]
- [Source: client/package.json]
- [Source: server/Cargo.toml]
- [Source: https://registry.npmjs.org/svelte/latest]
- [Source: https://registry.npmjs.org/@mateothegreat/svelte5-router/latest]
- [Source: https://api.github.com/repos/tokio-rs/axum/releases/latest]
- [Source: https://api.github.com/repos/launchbadge/sqlx/tags?per_page=5]
- [Source: https://api.github.com/repos/libp2p/rust-libp2p/releases/latest]

## Dev Agent Record

### Agent Model Used

GPT-5.3-Codex (model ID: gpt-5.3-codex)

### Debug Log References

- Workflow engine loaded: `_bmad/core/tasks/workflow.xml`
- Workflow config loaded: `_bmad/bmm/workflows/4-implementation/dev-story/workflow.yaml`
- Story source parsed from user input: `6-1` -> `6-1-websocket-gateway-and-connection-management`.
- Quality gates executed:
  - `cd client && npm run lint && npm run check && npm run test && npm run build`
  - `cd server && cargo fmt --check && cargo clippy -- -D warnings && cargo test`
- Story and sprint status updated to `done`.

### Completion Notes List

- Added a modular WebSocket gateway (`server/src/ws/*`) with typed protocol envelopes (`op`, `d`, `s`, `t`), `c_*` client-op validation, hello/heartbeat/resume hooks, and explicit error events.
- Added connection registry targeting (guild/channel subscribe/unsubscribe fanout) and per-connection rate limiting with structured tracing and machine-readable error payloads.
- Integrated presence broadcasting into shared WS transport so presence updates remain compatible while gaining sequence/timestamp envelope metadata.
- Added shared client WS lifecycle module with capped reconnect backoff (<=5s), routed subscription sync, and non-blocking `"Reconnecting..."` UX in `MessageArea`.
- Migrated presence store networking to the shared WS client to avoid long-term parallel sockets and added reconnect lifecycle tests.
- Added integration and unit coverage for authenticated handshake, protocol validation errors, targeted fanout, rate limiting, sequence progression, and reconnect behavior.

## Senior Developer Review (AI)

### Reviewer

Darko (AI-assisted review) on 2026-02-28

### Outcome

Adversarial review found no actionable HIGH/MEDIUM/LOW issues. Final decision: **Approve**.

### Findings

- None.

### Validation

- ✅ `cd client && npm run lint && npm run check && npm run test && npm run build`
- ✅ `cd server && cargo fmt --check && cargo clippy -- -D warnings && cargo test`

## Change Log

- 2026-02-28: Implemented Story 6.1 WebSocket gateway foundation, shared client lifecycle, targeted broadcast/rate-limit behavior, and full quality gates for review readiness.
- 2026-02-28: Senior code review completed with no actionable findings; story approved and moved to done.

### File List

- client/src/App.svelte
- client/src/lib/features/chat/MessageArea.svelte
- client/src/lib/features/members/presenceStore.svelte.ts
- client/src/lib/features/shell/ShellRoute.svelte
- client/src/lib/features/shell/ShellRoute.test.ts
- client/src/lib/ws/client.test.ts
- client/src/lib/ws/client.ts
- client/src/lib/ws/protocol.ts
- server/src/handlers/ws.rs
- server/src/lib.rs
- server/src/services/presence_service.rs
- server/src/ws/gateway.rs
- server/src/ws/mod.rs
- server/src/ws/protocol.rs
- server/src/ws/registry.rs
- server/tests/server_binds_to_configured_port.rs
- _bmad-output/implementation-artifacts/6-1-websocket-gateway-and-connection-management.md
- _bmad-output/implementation-artifacts/sprint-status.yaml
