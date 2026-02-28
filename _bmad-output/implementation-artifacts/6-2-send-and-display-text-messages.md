# Story 6.2: Send and Display Text Messages

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a **user**,
I want to send and read text messages in a channel,
so that I can communicate with others in my guild.

## Acceptance Criteria

1. **Given** a user is in a text channel  
   **When** they type a message in the MessageInput and press Enter (or click the fire Send button)  
   **Then** the message appears immediately in the message area (optimistic UI)

2. **Given** a user sends a message  
   **When** the client emits `c_message_create` over WebSocket  
   **Then** the server persists the message to the database in this story before broadcasting/acknowledging it

3. **Given** a message is accepted by the server  
   **When** channel subscribers are connected  
   **Then** all other users in the same channel receive `message_create` in real time

4. **Given** a message is rendered in the timeline  
   **When** MessageBubble is shown  
   **Then** it includes avatar (32px), role-colored author name, timestamp, and content

5. **Given** consecutive messages from the same author  
   **When** they are adjacent in the timeline  
   **Then** compact mode collapses repeated avatar/name

6. **Given** system messages (join/leave)  
   **When** they are rendered  
   **Then** they appear centered and muted

7. **Given** a user navigates to a channel  
   **When** MessageInput mounts  
   **Then** input is auto-focused

8. **Given** a user composes in MessageInput  
   **When** they press Shift+Enter  
   **Then** a newline is inserted, and Enter sends

9. **Given** a channel has no messages  
   **When** timeline renders  
   **Then** show: `This is the beginning of #channel-name. Say something!`

10. **Given** user-generated message content  
    **When** the server processes it  
    **Then** content is sanitized before storage (NFR14) and persisted before ack/broadcast to preserve durability (NFR27)

## Tasks / Subtasks

- [x] Task 1: Add persistent message domain and migration (AC: 2, 10)
  - [x] Add a new SQL migration in `server/migrations/` for a `messages` table with:
    - [x] `id`, `guild_id`, `channel_id`, `author_user_id`, `content`, `is_system`, `created_at`, `updated_at`
    - [x] Foreign keys to `guilds`, `channels`, and `users`
    - [x] Index for timeline reads and durability/perf (`idx_messages_channel_id_created_at`)
  - [x] Add `server/src/models/message.rs` with db access functions (insert/create + minimal lookups needed by WS flow)
  - [x] Export the model in `server/src/models/mod.rs`

- [x] Task 2: Implement message business logic in services layer (AC: 2, 3, 10)
  - [x] Add `server/src/services/message_service.rs` and export it via `server/src/services/mod.rs`
  - [x] Validate membership + channel visibility and enforce `SEND_MESSAGES` (and `VIEW_CHANNEL`) using existing permissions helpers
  - [x] Add server-side content sanitization and validation guardrails:
    - [x] Trim/normalize, reject empty-after-sanitization
    - [x] Reject control-character payloads and oversized payloads
    - [x] Preserve safe plain text/markdown semantics while preventing stored XSS vectors
  - [x] Persist message before emitting any `message_create` event

- [x] Task 3: Wire message create through WS gateway without breaking Story 6.1 behavior (AC: 2, 3)
  - [x] Keep `c_message_create` as the client op in protocol (`server/src/ws/protocol.rs`, `client/src/lib/ws/protocol.ts`)
  - [x] Refactor `server/src/ws/gateway.rs` so message create delegates to `message_service` (no business logic inside `ws/`)
  - [x] Broadcast `message_create` with persisted message data (id, author identifiers needed for rendering, created_at, content, channel scope)
  - [x] Preserve existing sequence/timestamp envelope behavior (`op`, `d`, `s`, `t`) and targeted channel fanout from registry

- [x] Task 4: Add client chat data model + optimistic send flow (AC: 1, 3, 4, 5, 6, 9)
  - [x] Add chat feature types/store for timeline state (for example `client/src/lib/features/chat/messageStore.svelte.ts`)
  - [x] Send composer submissions via `wsClient.send('c_message_create', ...)` and create optimistic local entries immediately
  - [x] Reconcile optimistic entries when server `message_create` arrives (dedupe/replace optimistic item)
  - [x] Render channel timeline from message store (replace current static placeholder list)
  - [x] Support compact grouping for consecutive same-author messages and centered muted system rows
  - [x] Show empty-state copy exactly when timeline is empty

- [x] Task 5: Implement message UI components and keyboard behavior (AC: 4, 5, 6, 7, 8, 9)
  - [x] Evolve `client/src/lib/features/chat/MessageArea.svelte` from placeholder into live timeline + composer shell
  - [x] Add `MessageBubble.svelte` and compose author/timestamp/content rendering with accessibility-friendly markup
  - [x] Keep auto-focus on channel navigation and preserve reconnect status treatment from Story 6.1
  - [x] Implement Enter send / Shift+Enter newline behavior in composer

- [x] Task 6: Add route surfaces for upcoming message history work without scope creep (AC: 2, future 6.3 compatibility)
  - [x] Add message module boundaries now (`handlers/messages.rs`, service/model wiring) even if full history REST pagination lands in Story 6.3
  - [x] Ensure naming and contracts remain compatible with architecture path shape (`/guilds/{guild_slug}/channels/{channel_slug}/messages`)

- [x] Task 7: Test coverage + quality gates (AC: all)
  - [x] Server integration tests in `server/tests/server_binds_to_configured_port.rs`:
    - [x] `c_message_create` persists and then broadcasts `message_create` only to subscribed channel peers
    - [x] unauthorized/forbidden/invalid payload paths return structured error events
    - [x] sanitization + validation cases are enforced
  - [x] Server unit tests:
    - [x] message service sanitization/validation and permission checks
    - [x] model insert/query behavior for sqlite + postgres query branches
  - [x] Client tests:
    - [x] optimistic append + reconciliation behavior
    - [x] Enter/Shift+Enter composer behavior
    - [x] empty-state + compact grouping rendering
  - [x] Run quality gates:
    - [x] `cd client && npm run lint && npm run check && npm run test && npm run build`
    - [x] `cd server && cargo fmt --check && cargo clippy -- -D warnings && cargo test`

## Dev Notes

### Developer Context

- Story 6.1 already established the authenticated WS gateway, op envelope contract, subscription fanout, reconnect lifecycle, and `c_message_create` op parsing; build on that foundation, do not bypass it.
- Current message flow in `server/src/ws/gateway.rs` broadcasts `message_create` directly from payload and does not persist or sanitize message content yet; this story closes that gap.
- Current `MessageArea.svelte` still renders placeholder timeline rows and a basic input; story 6.2 is the first real chat timeline/composer implementation.
- Current server routes do not include message REST handlers yet; architecture expects dedicated message modules for FR30-38.

### Technical Requirements

- Keep WS transport/business boundaries strict:
  - `ws/` handles protocol/frame dispatch only.
  - `services/message_service.rs` owns validation, permissions, sanitization, persistence, and broadcast payload shaping.
  - `models/message.rs` owns SQL.
- Permission checks must remain server-authoritative and use existing permission bitmasks (`SEND_MESSAGES`, `VIEW_CHANNEL`) with guild membership checks.
- Preserve API and WS error contract shape: `{"error":{"code","message","details":{}}}` for HTTP and equivalent structured payload in WS `error` events.
- Message persistence must complete before acknowledgment/broadcast to satisfy NFR27 durability intent.
- Sanitization must happen before storage and must not introduce regressions for future markdown rendering work (Story 6.7).
- Maintain targeted WS delivery via guild/channel subscriptions and existing sequence/timestamp envelope behavior.
- Keep UX behavior aligned with spec: optimistic send, clear status text, empty-state copy, keyboard conventions.

### Architecture Compliance

1. Respect layer boundaries from architecture:
   - handlers -> services -> models for REST/domain logic
   - ws -> services for real-time operations
2. Keep JSON naming and contracts in snake_case on the wire; map to camelCase in client types/helpers.
3. Keep route design consistent with existing API structure and nested-resource constraints.
4. Keep feature-module pattern on frontend: chat types/store/api/components co-located under `features/chat/`.
5. Preserve existing reconnect/status UX and non-blocking behavior from Story 6.1.

### Library & Framework Requirements

- Use existing stack already pinned in this repo:
  - Backend: Rust + Axum `0.8`, Tokio, sqlx `0.8`
  - Frontend: Svelte `5.45.2` line, `@mateothegreat/svelte5-router` `2.16.19`, Vitest
- No dependency upgrade is required to deliver this story.
- If introducing a sanitization dependency becomes necessary, keep it minimal and justify with security need + test coverage; otherwise prefer explicit server-side normalization/sanitization helper logic in service layer.

### File Structure Requirements

Expected primary touch points:

- Server
  - `server/migrations/00xx_create_messages.sql` (new)
  - `server/src/models/message.rs` (new)
  - `server/src/models/mod.rs`
  - `server/src/services/message_service.rs` (new)
  - `server/src/services/mod.rs`
  - `server/src/handlers/messages.rs` (new, at least scaffold for message domain)
  - `server/src/handlers/mod.rs` (route wiring)
  - `server/src/ws/gateway.rs` (delegate `c_message_create` to service)
  - `server/src/ws/protocol.rs` (only if payload contract needs expansion)
  - `server/src/permissions/mod.rs` (reuse existing bits; avoid inventing new ad-hoc checks)
  - `server/tests/server_binds_to_configured_port.rs`
- Client
  - `client/src/lib/features/chat/MessageArea.svelte`
  - `client/src/lib/features/chat/MessageBubble.svelte` (new)
  - `client/src/lib/features/chat/messageStore.svelte.ts` (new)
  - `client/src/lib/ws/client.ts` (send + event integration, if needed)
  - `client/src/lib/ws/protocol.ts` (only if type payloads are expanded)
  - `client/src/lib/features/shell/ShellRoute.svelte` (only if minimal wiring needed)
  - related tests under `client/src/lib/features/chat/*.test.ts` and/or `client/src/lib/ws/*.test.ts`

### Testing Requirements

- Server integration tests (extend existing WS integration suite):
  - authenticated clients can create messages with `c_message_create`
  - persisted message is broadcast as `message_create` only to subscribed channel members
  - invalid payloads produce `VALIDATION_ERROR` WS events
  - permission-denied paths produce explicit errors
  - rate-limit behavior from Story 6.1 remains intact
- Server unit tests:
  - sanitization/validation helper behavior
  - permission + membership checks in message service
  - model query/insert behavior and index-dependent ordering assumptions
- Client tests:
  - optimistic insertion and reconciliation with server events
  - Enter sends, Shift+Enter inserts newline
  - empty channel state text and message rendering variants (standard/compact/system)
  - composer autofocus on channel navigation remains true

### Previous Story Intelligence

- Story 6.1 introduced shared WS lifecycle state and reconnect behavior (`connecting`, `connected`, `reconnecting`, `disconnected`); message UX must integrate with this and remain non-blocking.
- Story 6.1 enforced `c_` client op prefixes and structured WS error events; message create must follow the same protocol validation and error handling.
- Story 6.1 established channel-targeted fanout via subscribe/unsubscribe; use the same subscription model for `message_create`.
- Story 6.1 added tests in `server/tests/server_binds_to_configured_port.rs` and `client/src/lib/ws/client.test.ts`; extend those patterns instead of introducing unrelated test harnesses.

### Git Intelligence Summary

Recent commits and implementation patterns to preserve:

- `b00e451` feat: finalize story 6-1 websocket gateway and review
- `e1b2e8a` feat: finalize story 5-7 member list with presence
- `afa9352` feat: finalize story 5-6 role management delegation
- `2491aec` feat: finalize story 5-5 channel permission overrides
- `68f87aa` feat: finalize story 5-4 role assignment to members

### Latest Technical Information

Current pinned lines in repo:

- `svelte`: `^5.45.2`
- `@mateothegreat/svelte5-router`: `^2.16.19`
- `axum`: `0.8`
- `sqlx`: `0.8`

Latest stable lines checked during story creation:

- Svelte latest: `5.53.6`
- `@mateothegreat/svelte5-router` latest: `2.16.19`
- Axum latest release tag: `axum-v0.8.8`
- SQLx latest tag observed: `v0.8.6`

No upgrades are required for this story; implement against currently pinned project versions.

### Project Context Reference

- No `project-context.md` file was discovered via `**/project-context.md`.
- Story context is derived from planning artifacts, existing implementation artifacts, source code, and recent git history.

### Story Completion Status

- Ultimate context engine analysis completed - comprehensive developer guide created.
- Story status set to `ready-for-dev`.

### Project Structure Notes

- Message domain files (`handlers/messages.rs`, `services/message_service.rs`, `models/message.rs`) do not yet exist and should be added following existing channel/category/role patterns.
- Existing migrations stop at `0017_create_channel_permission_overrides.sql`; message persistence migration should continue numbering from that sequence.
- `MessageArea.svelte` currently contains explicit placeholder timeline content that should be replaced by real message store-backed rendering in this story.

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Epic 6: Real-Time Text Communication]
- [Source: _bmad-output/planning-artifacts/epics.md#Story 6.2: Send and Display Text Messages]
- [Source: _bmad-output/planning-artifacts/prd.md#Text Communication FR30-FR38]
- [Source: _bmad-output/planning-artifacts/prd.md#Data & Privacy FR65]
- [Source: _bmad-output/planning-artifacts/prd.md#NFR14 Input sanitization]
- [Source: _bmad-output/planning-artifacts/prd.md#NFR27 Data durability]
- [Source: _bmad-output/planning-artifacts/architecture.md#API & Communication Patterns]
- [Source: _bmad-output/planning-artifacts/architecture.md#WebSocket Event Naming]
- [Source: _bmad-output/planning-artifacts/architecture.md#Loading State Pattern]
- [Source: _bmad-output/planning-artifacts/architecture.md#Requirements to Structure Mapping]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#MessageBubble]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#MessageInput]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Status messages (async operations)]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Empty States]
- [Source: _bmad-output/implementation-artifacts/6-1-websocket-gateway-and-connection-management.md]
- [Source: _bmad-output/implementation-artifacts/sprint-status.yaml]
- [Source: server/src/ws/gateway.rs]
- [Source: server/src/ws/protocol.rs]
- [Source: server/src/ws/registry.rs]
- [Source: server/src/handlers/mod.rs]
- [Source: server/src/services/presence_service.rs]
- [Source: server/src/permissions/mod.rs]
- [Source: client/src/lib/features/chat/MessageArea.svelte]
- [Source: client/src/lib/features/members/presenceStore.svelte.ts]
- [Source: client/src/lib/ws/client.ts]
- [Source: client/src/lib/ws/protocol.ts]
- [Source: client/src/lib/features/shell/ShellRoute.svelte]
- [Source: client/src/lib/features/shell/ShellRoute.test.ts]
- [Source: client/src/lib/ws/client.test.ts]
- [Source: https://registry.npmjs.org/svelte/latest]
- [Source: https://registry.npmjs.org/@mateothegreat/svelte5-router/latest]
- [Source: https://api.github.com/repos/tokio-rs/axum/releases/latest]
- [Source: https://api.github.com/repos/launchbadge/sqlx/tags?per_page=5]

## Dev Agent Record

### Agent Model Used

GPT-5.3-Codex (model ID: gpt-5.3-codex)

### Debug Log References

- Workflow engine loaded: `_bmad/core/tasks/workflow.xml`
- Workflow config loaded: `_bmad/bmm/workflows/4-implementation/dev-story/workflow.yaml`
- Story source parsed from user input: `6-2` -> `_bmad-output/implementation-artifacts/6-2-send-and-display-text-messages.md`
- Implemented server message persistence/service/route scaffolding and delegated WS `c_message_create` through `message_service`.
- Implemented client timeline/message store with optimistic send + reconciliation and live timeline rendering.
- Quality gates executed successfully:
  - `cd client && npm run lint && npm run check && npm run test && npm run build`
  - `cd server && cargo fmt --check && cargo clippy -- -D warnings && cargo test`

### Completion Notes List

- Added persistent message domain (`messages` migration + model) with indexed timeline reads.
- Added `message_service` with permission enforcement, sanitization/validation, persistence-before-broadcast, and response shaping.
- Refactored websocket gateway to delegate `c_message_create` to `message_service` and emit structured WS errors.
- Added `/api/v1/guilds/{guild_slug}/channels/{channel_slug}/messages` route surface and handler wiring for 6.3-compatible boundaries.
- Replaced placeholder chat UI with message store-backed timeline, `MessageBubble`, empty state, compact grouping, auto-focus, and Enter/Shift+Enter composer behavior.
- Added server and client tests for persistence/broadcast/error paths plus optimistic reconciliation and timeline/composer behavior.

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

### Change Log

- 2026-02-28: Completed Story 6.2 implementation; status moved to `review`.
- 2026-02-28: Senior code review completed with no actionable findings; story approved and moved to done.

### File List

- _bmad-output/implementation-artifacts/6-2-send-and-display-text-messages.md
- _bmad-output/implementation-artifacts/sprint-status.yaml
- client/src/lib/features/chat/MessageArea.svelte
- client/src/lib/features/chat/MessageArea.test.ts
- client/src/lib/features/chat/MessageBubble.svelte
- client/src/lib/features/chat/messageStore.svelte.ts
- client/src/lib/features/chat/messageStore.test.ts
- server/migrations/0018_create_messages.sql
- server/src/handlers/messages.rs
- server/src/handlers/mod.rs
- server/src/handlers/ws.rs
- server/src/models/message.rs
- server/src/models/mod.rs
- server/src/services/message_service.rs
- server/src/services/mod.rs
- server/src/ws/gateway.rs
- server/tests/server_binds_to_configured_port.rs
