# Story 6.5: Emoji Reactions

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a **user**,
I want to react to messages with emoji,
so that I can respond quickly without sending a full message.

## Acceptance Criteria

1. **Given** a user hovers over a message  
   **When** they click the react button (+) in the hover action bar or right-click -> React  
   **Then** an emoji picker popover appears.

2. **Given** a user selects an emoji in the picker  
   **When** the selection is submitted  
   **Then** that emoji is added as a reaction to the message.

3. **Given** reactions exist on a message  
   **When** the message renders  
   **Then** reaction badges render below the message with emoji and count.

4. **Given** a user clicks an existing reaction badge  
   **When** they already reacted with that emoji  
   **Then** their reaction is toggled off, and clicking again toggles it on.

5. **Given** multiple users react with the same emoji  
   **When** reactions are aggregated  
   **Then** the badge count increments/decrements correctly per unique user.

6. **Given** a reaction is added/removed  
   **When** channel subscribers are connected  
   **Then** the reaction change is broadcast to all subscribed channel members in real time.

7. **Given** message reactions must persist  
   **When** this story migration is applied  
   **Then** a `message_reactions` table exists with columns `message_id`, `user_id`, `emoji`, `created_at`.

8. **Given** a user lacks `ADD_REACTIONS` permission in a channel  
   **When** they try to add a reaction  
   **Then** the operation is rejected with a structured `FORBIDDEN` error.

## Tasks / Subtasks

- [x] Task 1: Add reaction persistence schema and model operations (AC: 5, 7)
  - [x] Add migration `server/migrations/0020_create_message_reactions.sql` with `message_id`, `user_id`, `emoji`, `created_at`, FK constraints to `messages` and `users`, and uniqueness for `(message_id, user_id, emoji)`.
  - [x] Add model operations for toggle upsert/delete semantics and aggregated reads by message id (`server/src/models/message_reaction.rs` or equivalent extension in `message.rs`).
  - [x] Add supporting indexes for fast badge aggregation in timeline reads (`message_id`, `emoji`, `created_at`).

- [x] Task 2: Implement reaction service logic and permission guardrails (AC: 2, 4, 5, 8)
  - [x] Extend `server/src/services/message_service.rs` with reaction toggle entry points that validate guild membership, channel visibility, message existence, and channel permission checks.
  - [x] Enforce `ADD_REACTIONS` for add behavior and keep server-authoritative checks (never trust client flags).
  - [x] Validate emoji payloads (non-empty, bounded length, no control chars) and return structured `VALIDATION_ERROR` when invalid.
  - [x] Return reaction snapshots suitable for UI rendering (`emoji`, `count`, `reacted`) while preserving existing message payload contracts.

- [x] Task 3: Extend WebSocket protocol and gateway dispatch for reactions (AC: 2, 4, 6, 8)
  - [x] Add client op parsing for a reaction toggle op (for example `c_message_reaction_toggle`) in `server/src/ws/protocol.rs` and client `WsClientOp` types.
  - [x] Add payload parsing and service delegation in `server/src/ws/gateway.rs` with existing structured error behavior.
  - [x] Broadcast a dedicated reaction event (for example `message_reaction_update`) through existing channel-targeted registry fanout.

- [x] Task 4: Wire reaction data into message read/write surfaces (AC: 3, 5)
  - [x] Include reaction aggregates in timeline-relevant payloads so badges render after reconnect/history loads (`message_create` and history API response path in `handlers/messages.rs` + `message_service`).
  - [x] Preserve Story 6.3 cursor semantics and avoid ordering regressions while enriching payload data.
  - [x] Keep JSON wire shape `snake_case` and map to client `camelCase` types in `client/src/lib/features/chat/types.ts`.

- [x] Task 5: Implement picker + badges UX in chat components (AC: 1, 2, 3, 4)
  - [x] Extend `client/src/lib/features/chat/MessageBubble.svelte` to render reaction badges under message content and expose active-state styling for current user.
  - [x] Add a lightweight emoji picker popover flow triggered from the existing react action in the hover bar and context menu.
  - [x] Wire `onReactRequest` from `MessageBubble` into `MessageArea.svelte` and `messageStore.svelte.ts` to send reaction toggle ops and reconcile incoming reaction events.
  - [x] Keep keyboard/context-menu accessibility behavior intact (`ContextMenu`, `Shift+F10`, focus/escape semantics).

- [x] Task 6: Add regression coverage and run quality gates (AC: all)
  - [x] Server integration tests in `server/tests/server_binds_to_configured_port.rs` for reaction add/toggle/broadcast and permission-denied behavior.
  - [x] Server unit tests for reaction model/service validation, uniqueness/toggle behavior, and error paths.
  - [x] Client tests (`MessageBubble.test.ts`, `MessageArea.test.ts`, `messageStore.test.ts`) for picker open/select, badge rendering/count updates, and toggle behavior.
  - [x] Run quality gates:
    - [x] `cd client && npm run lint && npm run check && npm run test && npm run build`
    - [x] `cd server && cargo fmt --check && cargo clippy -- -D warnings && cargo test`

## Dev Notes

### Developer Context

- Story 6.4 already delivered hover action controls with a visible **React** button in `MessageBubble`, but `onReactRequest` is not yet wired through `MessageArea`/`messageStore` into a real operation.
- Story 6.2 and 6.3 established message persistence, cursor history, optimistic reconciliation, and virtualized timeline rendering; reaction support must extend these paths without introducing parallel message pipelines.
- Permission infrastructure already defines `ADD_REACTIONS` on both server and client permission catalogs, so this story should reuse existing bitflag enforcement paths.
- Existing WebSocket protocol/gateway currently supports message create/update/delete and typing events only; reaction operations/events are net-new in this story.

### Technical Requirements

- Keep strict transport/domain boundaries:
  - `ws/*` handles protocol parsing, dispatch, and fanout.
  - `services/message_service.rs` owns validation, permission checks, and business logic.
  - `models/*` owns SQL operations.
- Persist reaction state server-side with deterministic toggle semantics per `(message_id, user_id, emoji)`.
- Preserve structured error contracts across WS and REST (`FORBIDDEN`, `VALIDATION_ERROR`, etc.).
- Keep reaction payloads channel-scoped and subscription-scoped; do not global-broadcast.
- Ensure reaction rendering is durable across:
  - live WS updates,
  - channel history REST loads,
  - reconnect/re-render in virtualized timelines.
- Follow existing content/input safety patterns (trim, control-char rejection, explicit bounds).

### Architecture Compliance

1. Keep backend layering: handlers/ws -> services -> models.
2. Keep WebSocket naming conventions: client ops `c_*`, server ops `snake_case`, envelope `{"op","d","s","t"}`.
3. Keep wire payload fields in `snake_case`; map to `camelCase` in client feature types/helpers.
4. Preserve Story 6.3 timeline performance constraints (virtualization + bounded memory behavior).
5. Preserve Story 6.1 reconnect + non-blocking UX behavior and Story 6.4 keyboard/context-menu interactions.

### Library & Framework Requirements

- Repo pinned versions:
  - `svelte`: `^5.45.2`
  - `@mateothegreat/svelte5-router`: `^2.16.19`
  - `axum`: `0.8`
  - `sqlx`: `0.8`
- Latest checks during story creation:
  - `svelte` latest: `5.53.6`
  - `@mateothegreat/svelte5-router` latest: `2.16.19`
  - `axum` latest release tag: `axum-v0.8.8`
  - `sqlx` latest tag: `v0.8.6`
- No dependency upgrade is required for this story.

### File Structure Requirements

Expected primary touch points:

- Server
  - `server/migrations/0020_create_message_reactions.sql` (new)
  - `server/src/models/message.rs` and/or `server/src/models/message_reaction.rs` (new)
  - `server/src/models/mod.rs` (if new model module is added)
  - `server/src/services/message_service.rs`
  - `server/src/ws/protocol.rs`
  - `server/src/ws/gateway.rs`
  - `server/src/handlers/messages.rs` (if history payload shape needs reaction hydration)
  - `server/tests/server_binds_to_configured_port.rs`

- Client
  - `client/src/lib/ws/protocol.ts`
  - `client/src/lib/features/chat/types.ts`
  - `client/src/lib/features/chat/messageStore.svelte.ts`
  - `client/src/lib/features/chat/MessageBubble.svelte`
  - `client/src/lib/features/chat/MessageArea.svelte`
  - `client/src/lib/features/chat/MessageBubble.test.ts`
  - `client/src/lib/features/chat/MessageArea.test.ts`
  - `client/src/lib/features/chat/messageStore.test.ts`

### Testing Requirements

- Server integration:
  - add reaction emits channel-scoped real-time event to subscribers,
  - toggling own reaction removes it and updates counts,
  - multi-user same-emoji aggregation increments/decrements correctly,
  - missing `ADD_REACTIONS` returns structured `FORBIDDEN`.
- Server unit:
  - reaction input validation,
  - uniqueness + toggle behavior,
  - message/channel scoping and not-found handling.
- Client:
  - react action opens picker,
  - picker selection sends correct WS payload,
  - incoming reaction updates render badges/counts,
  - clicking existing badge toggles own reaction.
- Run full quality gates before moving story beyond ready-for-dev.

### Previous Story Intelligence

- Story 6.4 established `message_update`/`message_delete` mutation flows, ownership checks, and action-bar/context-menu patterns; reaction behavior should mirror that event and UX consistency.
- Story 6.3 established cursor-based history and virtualized rendering; any reaction payload additions must preserve sorting, memory limits, and scroll restoration.
- Story 6.2 established sanitize-before-store and persist-before-broadcast principles; reaction mutations should follow the same trust boundary.
- Story 6.1 established WS operation parsing, rate-limiting, and channel-targeted fanout; reaction ops must reuse that transport path.

### Git Intelligence Summary

Recent commit patterns to preserve:

- `bfe2ad1` feat: finalize story 6-4 edit/delete own messages
- `8741b54` feat: finalize story 6-3 message history
- `3bff024` feat: finalize story 6-2 messaging and review
- `b00e451` feat: finalize story 6-1 websocket gateway and review
- `e1b2e8a` feat: finalize story 5-7 member list with presence

### Latest Technical Information

- `svelte` latest package version: `5.53.6`
- `@mateothegreat/svelte5-router` latest package version: `2.16.19`
- `tokio-rs/axum` latest release tag: `axum-v0.8.8`
- `launchbadge/sqlx` latest tag: `v0.8.6`
- Current pinned project versions are suitable for this story scope.

### Project Context Reference

- No `project-context.md` file was discovered via `**/project-context.md`.
- Story context is derived from planning artifacts, implementation artifacts, current source code, and recent git history.

### Story Completion Status

- Ultimate context engine analysis completed - comprehensive developer guide created.
- Story status set to `ready-for-dev`.

### Project Structure Notes

- Chat feature boundaries currently split by `MessageArea.svelte` (timeline/composer shell), `MessageBubble.svelte` (row rendering), `messageStore.svelte.ts` (state + WS ingestion), and `messageApi.ts` (history fetch).
- Current server message mutation path is WebSocket-driven (`ws/gateway.rs` -> `message_service`) and should remain the primary write path for reactions.
- `message_reactions` migration should follow sequential migration numbering after `0019_add_messages_cursor_index.sql`.

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Epic 6: Real-Time Text Communication]
- [Source: _bmad-output/planning-artifacts/epics.md#Story 6.5: Emoji Reactions]
- [Source: _bmad-output/planning-artifacts/prd.md#Text Communication]
- [Source: _bmad-output/planning-artifacts/prd.md#Functional Requirements]
- [Source: _bmad-output/planning-artifacts/prd.md#Non-Functional Requirements]
- [Source: _bmad-output/planning-artifacts/architecture.md#Implementation Patterns & Consistency Rules]
- [Source: _bmad-output/planning-artifacts/architecture.md#API & Communication Patterns]
- [Source: _bmad-output/planning-artifacts/architecture.md#Frontend Architecture]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#MessageBubble]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Keyboard Shortcut Patterns]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Loading Patterns]
- [Source: _bmad-output/implementation-artifacts/6-4-edit-and-delete-own-messages.md]
- [Source: _bmad-output/implementation-artifacts/6-3-message-history-and-virtual-scrolling.md]
- [Source: _bmad-output/implementation-artifacts/6-2-send-and-display-text-messages.md]
- [Source: _bmad-output/implementation-artifacts/sprint-status.yaml]
- [Source: server/src/permissions/mod.rs]
- [Source: server/src/models/message.rs]
- [Source: server/src/services/message_service.rs]
- [Source: server/src/ws/protocol.rs]
- [Source: server/src/ws/gateway.rs]
- [Source: server/src/handlers/messages.rs]
- [Source: server/migrations/0018_create_messages.sql]
- [Source: server/migrations/0019_add_messages_cursor_index.sql]
- [Source: client/src/lib/features/chat/MessageBubble.svelte]
- [Source: client/src/lib/features/chat/MessageArea.svelte]
- [Source: client/src/lib/features/chat/messageStore.svelte.ts]
- [Source: client/src/lib/features/chat/types.ts]
- [Source: client/src/lib/ws/protocol.ts]
- [Source: client/package.json]
- [Source: server/Cargo.toml]
- [Source: https://registry.npmjs.org/svelte/latest]
- [Source: https://registry.npmjs.org/@mateothegreat/svelte5-router/latest]
- [Source: https://api.github.com/repos/tokio-rs/axum/releases/latest]
- [Source: https://api.github.com/repos/launchbadge/sqlx/tags?per_page=5]

## Dev Agent Record

### Agent Model Used

GPT-5.3-Codex (gpt-5.3-codex)

### Debug Log References

- Full quality gates command run (exit 0): `cd client && npm run lint && npm run check && npm run test && npm run build && cd ../server && cargo fmt --check && cargo clippy -- -D warnings && cargo test`
- Copilot command output: `/tmp/copilot-tool-output-1772310051146-hhyjxq.txt`

### Completion Notes List

- Added full emoji reaction backend flow: migration, model, reaction toggle service, `ADD_REACTIONS` guard for add, validation, and reaction snapshots on message payloads.
- Extended WS protocol/gateway with `c_message_reaction_toggle` and `message_reaction_update`, including channel-scoped fanout and structured error handling.
- Implemented client reaction UX and state flow (picker popover, badges with counts/toggle, store op send + update ingestion, and message type mapping).
- Added/updated server and client tests for reaction persistence, aggregation, permission denial, picker/badge behavior, and history hydration with reactions.
- Senior review follow-up: made reaction toggle mutation idempotent under concurrent requests by removing transient `CONFLICT`/`NOT_FOUND` branches after insert/delete attempts.

### File List

- _bmad-output/implementation-artifacts/6-5-emoji-reactions.md
- server/migrations/0020_create_message_reactions.sql
- server/src/models/message_reaction.rs
- server/src/models/mod.rs
- server/src/services/message_service.rs
- server/src/ws/protocol.rs
- server/src/ws/gateway.rs
- server/tests/server_binds_to_configured_port.rs
- client/src/lib/ws/protocol.ts
- client/src/lib/features/chat/types.ts
- client/src/lib/features/chat/messageStore.svelte.ts
- client/src/lib/features/chat/MessageBubble.svelte
- client/src/lib/features/chat/MessageArea.svelte
- client/src/lib/features/chat/MessageBubble.test.ts
- client/src/lib/features/chat/MessageArea.test.ts
- client/src/lib/features/chat/messageStore.test.ts
- _bmad-output/implementation-artifacts/sprint-status.yaml

### Change Log

- 2026-02-28: Implemented Story 6.5 emoji reactions end-to-end and passed full client/server quality gates.
- 2026-02-28: Senior Developer Review (AI) completed in YOLO mode; fixed a reaction-toggle race path returning transient `CONFLICT`/`NOT_FOUND` errors under concurrent toggles, re-ran full quality gates, and moved story to `done`.
- 2026-02-28: Follow-up adversarial review fixed per-subscriber reaction snapshots in websocket fanout so `reacted` flags are computed for each viewer connection.
- 2026-02-28: Follow-up code review fixed reaction fanout N+1 queries by batching reaction data retrieval per toggle and materializing per-viewer summaries in-memory.
- 2026-02-28: Follow-up code review aligned actor and subscriber reaction snapshots so all recipients see the same fresh per-viewer counts from one broadcast query.

### Senior Developer Review (AI)

- Reviewer: Darko (GPT-5.3-Codex)
- Date: 2026-02-28
- Outcome: **Approve**
- Findings:
  - Fixed 1 MEDIUM issue in `server/src/services/message_service.rs`: concurrent toggle requests could return transient `CONFLICT`/`NOT_FOUND` due check-then-act race behavior.
  - Fixed 1 HIGH issue across `server/src/ws/gateway.rs` + `server/src/ws/registry.rs`: reaction broadcast previously reused actor-view `reacted` flags for all subscribers; fanout now computes per-subscriber snapshots.
  - Fixed 1 MEDIUM issue across `server/src/ws/gateway.rs` + `server/src/services/message_service.rs` + `server/src/models/message_reaction.rs`: reaction fanout no longer runs per-subscriber DB queries (`N+1`) and now computes personalized views from a single reaction fetch.
  - Fixed 1 MEDIUM issue in `server/src/ws/gateway.rs`: actor and non-actor subscribers now both use the same fresh per-viewer broadcast snapshot to avoid divergent reaction counts under concurrency.
  - No remaining actionable HIGH or MEDIUM issues after fix and re-review.
  - Source file claims and implementation coverage for Story 6.5 ACs were revalidated.
- Validation: `cd client && npm run lint && npm run check && npm run test && npm run build && cd ../server && cargo fmt --check && cargo clippy -- -D warnings && RUST_TEST_THREADS=1 cargo test`
