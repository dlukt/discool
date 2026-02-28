# Story 6.3: Message History and Virtual Scrolling

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a **user**,
I want to scroll through message history smoothly,
so that I can read past conversations without lag or jank.

## Acceptance Criteria

1. **Given** a channel has hundreds or thousands of messages  
   **When** the user scrolls up in the message area  
   **Then** older messages load progressively via REST API pagination (cursor-based, not offset-based)

2. **Given** older history is being fetched  
   **When** a request is in flight  
   **Then** skeleton placeholders appear while messages load (matching message shape)

3. **Given** a high-volume channel timeline  
   **When** the user scrolls through history  
   **Then** scrolling is smooth at 60fps with 10,000+ messages in the channel (NFR8)

4. **Given** a large timeline is displayed  
   **When** the message list renders  
   **Then** only visible messages are rendered in the DOM (virtual scrolling)

5. **Given** the user is reading history  
   **When** new messages arrive in the channel  
   **Then** scrolling down past the newest loaded message shows new messages arriving in real-time

6. **Given** the user is far from newest messages  
   **When** they keep reading older content  
   **Then** a **Jump to present** button appears

7. **Given** a user switches away from and back to a channel  
   **When** timeline UI restores  
   **Then** the scroll position is preserved

8. **Given** multiple active guild contexts  
   **When** message history is used across channels  
   **Then** client memory usage stays under 200MB with 5 guilds active (NFR7)

## Tasks / Subtasks

- [x] Task 1: Add cursor-based message history contract on existing REST route (AC: 1, 5)
  - [x] Extend `GET /api/v1/guilds/{guild_slug}/channels/{channel_slug}/messages` query parsing in `server/src/handlers/messages.rs` to support cursor inputs (for example `before` + `limit`).
  - [x] Return list responses with envelope `{"data":[...], "cursor":"..."}` (no offset pagination), preserving existing error format.
  - [x] Keep default limit safety and clamp rules (1..200) while validating malformed cursor values with explicit `VALIDATION_ERROR`.

- [x] Task 2: Implement stable server-side pagination in model/service layers (AC: 1, 5)
  - [x] Extend `server/src/models/message.rs` with cursor-aware query functions (older-than cursor, deterministic order by `(created_at, id)`).
  - [x] Keep permission checks in `server/src/services/message_service.rs` (`VIEW_CHANNEL` + membership) before history reads.
  - [x] Add/adjust DB index migration only if required for deterministic cursor performance (avoid breaking existing 6.2 reads).

- [x] Task 3: Add chat history API client surface and typed cursor mapping (AC: 1, 5)
  - [x] Add `client/src/lib/features/chat/messageApi.ts` for history requests and cursor continuation.
  - [x] Add/extend chat wire/domain types (new `types.ts` or equivalent) to map `snake_case` API payloads to `messageStore` shape.
  - [x] Reuse `api.ts` conventions for auth/error handling and avoid ad-hoc `fetch()` in components.

- [x] Task 4: Extend message store for paginated history + per-channel state (AC: 1, 5, 7, 8)
  - [x] Evolve `client/src/lib/features/chat/messageStore.svelte.ts` to keep per-channel cursor, loading flags, hasMore state, and restore metadata.
  - [x] Merge paginated history with live WS `message_create` events without duplication or ordering regressions.
  - [x] Preserve/restore per-channel scroll anchor state when routing changes channels.
  - [x] Replace fixed `MAX_CHANNEL_TIMELINE_ITEMS` trimming with strategy that satisfies NFR7 while still allowing deep history traversal.

- [x] Task 5: Implement virtualized message rendering and loading UX in MessageArea (AC: 2, 3, 4, 6, 7)
  - [x] Refactor `client/src/lib/features/chat/MessageArea.svelte` to use virtual window rendering (visible rows + overscan), reusing the proven `MemberList` virtualization approach where possible.
  - [x] Add history load triggers on upward scroll boundary and render skeleton rows while loading (no “Load more” button).
  - [x] Add **Jump to present** CTA when user is sufficiently far from bottom; restore bottom-follow behavior when CTA is used.
  - [x] Keep accessibility semantics for message log (`role="log"`, `aria-live`) and maintain composer focus behavior on channel navigation.

- [x] Task 6: Ensure real-time continuity while reading history (AC: 5, 6)
  - [x] Keep existing `wsClient` subscription flow unchanged; ingest incoming messages while user is scrolled back.
  - [x] Do not force auto-scroll when user is reading older history; surface unobtrusive “new messages” affordance tied to jump action.
  - [x] Ensure optimistic sends still reconcile correctly when timeline is virtualized and partially rendered.

- [x] Task 7: Test coverage and quality gates (AC: all)
  - [x] Server integration tests in `server/tests/server_binds_to_configured_port.rs` for cursor pagination semantics, permission-denied paths, and response envelope shape with cursor.
  - [x] Server unit tests for cursor parsing/validation and model query ordering stability (same timestamp tie-breaks).
  - [x] Client tests (`MessageArea.test.ts`, `messageStore.test.ts`) for virtualized windowing behavior, skeleton load states, jump-to-present UX, and scroll restoration across channel switches.
  - [x] Run quality gates:
    - [x] `cd client && npm run lint && npm run check && npm run test && npm run build`
    - [x] `cd server && cargo fmt --check && cargo clippy -- -D warnings && cargo test`

## Dev Notes

### Developer Context

- Story 6.2 already introduced persisted messages, `c_message_create` -> `message_create` flow, and baseline chat timeline rendering. Story 6.3 must build on that path, not replace it.
- The REST route for channel messages already exists, but currently supports only basic limit-based reads (no cursor contract yet).
- `messageStore` currently appends/reconciles live messages and trims timeline length, but it does not manage paginated history state or per-channel scroll restoration.
- `MessageArea` currently renders all timeline rows in a simple loop and needs virtualization for 10k+ history performance.

### Technical Requirements

- Use cursor-based pagination only (no offset anywhere).
- Preserve API envelope and naming conventions: `{"data": ...}` / `{"error": ...}` and `snake_case` on wire.
- Keep permission checks server-authoritative for every history fetch (`VIEW_CHANNEL` and guild membership gates).
- Maintain deterministic ordering with cursor tie-break safety for messages sharing timestamps.
- Keep real-time updates (`message_create`) working while history is partially loaded and user is not at bottom.
- Keep loading UX per UX spec: skeleton placeholders for history loads, no blocking overlay, and no “load more” button.

### Architecture Compliance

1. Keep backend boundaries strict: handlers -> services -> models.
2. Keep WebSocket transport logic in `ws/` and business logic in `services/`.
3. Keep list endpoints cursor-based and avoid offset anti-patterns.
4. Keep frontend boundary separation: REST history via API layer, live updates via WS store ingestion.
5. Keep JSON boundary in `snake_case` with frontend mapping to `camelCase`.
6. Keep tests co-located on client and integration-heavy on server for behavior safety.

### Library & Framework Requirements

- Backend: Rust + Axum `0.8`, sqlx `0.8`, Tokio async runtime.
- Frontend: Svelte `^5.45.2`, `@mateothegreat/svelte5-router` `^2.16.19`, Vitest.
- Latest checked lines during story generation:
  - Svelte latest: `5.53.6`
  - `@mateothegreat/svelte5-router` latest: `2.16.19`
  - Axum latest release tag: `axum-v0.8.8`
  - SQLx latest tag: `v0.8.6`
- No dependency upgrade is required for this story; implement against current pinned versions.

### File Structure Requirements

Expected primary touch points:

- Server
  - `server/src/handlers/messages.rs`
  - `server/src/services/message_service.rs`
  - `server/src/models/message.rs`
  - `server/src/handlers/mod.rs` (only if route signature adjustments are needed)
  - `server/migrations/` (optional index migration if required by query plan)
  - `server/tests/server_binds_to_configured_port.rs`
- Client
  - `client/src/lib/features/chat/MessageArea.svelte`
  - `client/src/lib/features/chat/messageStore.svelte.ts`
  - `client/src/lib/features/chat/messageStore.test.ts`
  - `client/src/lib/features/chat/MessageArea.test.ts`
  - `client/src/lib/features/chat/messageApi.ts` (new)
  - `client/src/lib/features/chat/types.ts` (new or extended)
  - `client/src/lib/api.ts` (only if list+cursor helper extraction is needed)

### Testing Requirements

- Server integration:
  - first-page message history returns ordered data and cursor,
  - follow-up cursor fetch returns older items without overlap,
  - invalid cursor returns structured validation error,
  - unauthorized/forbidden readers are rejected.
- Server unit:
  - cursor decode/encode validation,
  - deterministic ordering with timestamp ties,
  - limit clamp behavior.
- Client:
  - virtual window renders a subset of large timelines,
  - upward scroll triggers history fetch and skeleton display,
  - jump-to-present appears and restores bottom-follow behavior,
  - per-channel scroll restore works after route changes,
  - live WS arrivals while scrolled back do not yank scroll.

### Previous Story Intelligence

- Story 6.2 established `messages` persistence and `idx_messages_channel_id_created_at` index.
- Story 6.2 enforces sanitize-before-store and persist-before-broadcast; do not regress these guarantees while adding history read paths.
- Story 6.2 introduced optimistic send/reconciliation via `client_nonce`; this must remain stable in a virtualized list.
- Story 6.1/6.2 established shared WS lifecycle and targeted channel subscriptions; history work should not fork socket logic.

### Git Intelligence Summary

Recent commit patterns to preserve:

- `3bff024` feat: finalize story 6-2 messaging and review
- `b00e451` feat: finalize story 6-1 websocket gateway and review
- `e1b2e8a` feat: finalize story 5-7 member list with presence
- `afa9352` feat: finalize story 5-6 role management delegation
- `2491aec` feat: finalize story 5-5 channel permission overrides

### Latest Technical Information

Current repo pins:

- `svelte`: `^5.45.2`
- `@mateothegreat/svelte5-router`: `^2.16.19`
- `axum`: `0.8`
- `sqlx`: `0.8`

Latest stable checks:

- Svelte latest: `5.53.6`
- `@mateothegreat/svelte5-router` latest: `2.16.19`
- Axum latest release: `axum-v0.8.8`
- SQLx latest tag: `v0.8.6`

No upgrades required for Story 6.3.

### Project Context Reference

- No `project-context.md` file was discovered via `**/project-context.md`.
- Story context is derived from planning artifacts, current implementation artifacts, current source code, and recent git history.

### Story Completion Status

- Ultimate context engine analysis completed - comprehensive developer guide created.
- Story status set to `ready-for-dev`.

### Project Structure Notes

- `client/src/lib/features/chat/` currently has `MessageArea`, `MessageBubble`, and `messageStore`; there is no dedicated `messageApi.ts` yet.
- `client/src/lib/features/members/MemberList.svelte` already contains working virtual list math and rendering strategy that should be reused as prior art.
- `server/src/handlers/messages.rs` and `message_service::list_channel_messages` currently support only limit-based reads; cursor behavior is net-new in this story.

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Story 6.3: Message History and Virtual Scrolling]
- [Source: _bmad-output/planning-artifacts/epics.md#Story 6.2: Send and Display Text Messages]
- [Source: _bmad-output/planning-artifacts/prd.md#FR31]
- [Source: _bmad-output/planning-artifacts/prd.md#FR38]
- [Source: _bmad-output/planning-artifacts/prd.md#NFR7]
- [Source: _bmad-output/planning-artifacts/prd.md#NFR8]
- [Source: _bmad-output/planning-artifacts/architecture.md#UX-Driven Architectural Implications]
- [Source: _bmad-output/planning-artifacts/architecture.md#Format Patterns]
- [Source: _bmad-output/planning-artifacts/architecture.md#Enforcement Guidelines]
- [Source: _bmad-output/planning-artifacts/architecture.md#State Boundaries (Frontend)]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#MessageBubble]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Loading Patterns]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Implementation Guidelines]
- [Source: _bmad-output/implementation-artifacts/6-2-send-and-display-text-messages.md]
- [Source: _bmad-output/implementation-artifacts/sprint-status.yaml]
- [Source: server/src/handlers/messages.rs]
- [Source: server/src/services/message_service.rs]
- [Source: server/src/models/message.rs]
- [Source: server/src/ws/gateway.rs]
- [Source: client/src/lib/features/chat/MessageArea.svelte]
- [Source: client/src/lib/features/chat/messageStore.svelte.ts]
- [Source: client/src/lib/features/members/MemberList.svelte]
- [Source: client/src/lib/ws/client.ts]
- [Source: client/src/lib/ws/protocol.ts]
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
- Story source parsed from user input: `6-3` -> `6-3-message-history-and-virtual-scrolling`
- Implemented backend cursor pagination across handler/service/model with deterministic `(created_at, id)` ordering and cursor encoding/decoding.
- Added migration `0019_add_messages_cursor_index.sql` for cursor-read performance.
- Added history API surface and chat wire/domain mapping in `messageApi.ts` and `types.ts`.
- Reworked `messageStore.svelte.ts` for per-channel history cursor/loading/restore state, bounded memory strategy, and live-event merge safety.
- Refactored `MessageArea.svelte` for virtualized rendering, top-boundary history fetch, loading skeletons, jump-to-present CTA, and scroll restoration.
- Expanded server/client tests for cursor pagination semantics and virtualized timeline UX.
- Quality gates run successfully:
  - `cd client && npm run lint && npm run check && npm run test && npm run build`
  - `cd server && cargo fmt --check && cargo clippy -- -D warnings && cargo test`

### Completion Notes List

- Implemented cursor-based message history contract (`before` + clamped `limit`) with explicit validation and response envelope `{ "data": [...], "cursor": "..." }`.
- Added deterministic cursor pagination in message model/service layers and preserved VIEW_CHANNEL + membership authorization checks.
- Added chat history API client and typed `snake_case` to `camelCase` mapping for REST payloads and live timeline shape consistency.
- Upgraded message store to maintain per-channel cursor/loading/restore metadata, merge paginated + real-time events safely, and bound memory with deep-history support.
- Refactored MessageArea to virtual window rendering with overscan, skeleton history loading, jump-to-present/new-message affordance, and role/log accessibility semantics.
- Added/updated server and client tests for cursor behavior, malformed cursor handling, virtualization, skeleton states, jump CTA, and scroll restoration.
- Executed full client and server quality gates with all checks passing.

### File List

- client/src/lib/api.ts
- client/src/lib/features/chat/messageApi.ts
- client/src/lib/features/chat/types.ts
- client/src/lib/features/chat/messageStore.svelte.ts
- client/src/lib/features/chat/MessageArea.svelte
- client/src/lib/features/chat/MessageBubble.svelte
- client/src/lib/features/chat/messageStore.test.ts
- client/src/lib/features/chat/MessageArea.test.ts
- server/src/handlers/messages.rs
- server/src/services/message_service.rs
- server/src/models/message.rs
- server/migrations/0019_add_messages_cursor_index.sql
- server/tests/server_binds_to_configured_port.rs
- _bmad-output/implementation-artifacts/6-3-message-history-and-virtual-scrolling.md
- _bmad-output/implementation-artifacts/sprint-status.yaml

## Change Log

- 2026-02-28: Implemented Story 6.3 end-to-end (cursor history API, deterministic pagination, client history/store/message virtualization UX, and full regression coverage updates).
- 2026-02-28: Senior code review found and fixed active-channel memory trimming that could discard newly loaded older history once channel cache hit cap.

## Senior Developer Review (AI)

### Reviewer

Darko

### Outcome

Approved

### Findings

1. **HIGH** — Active channel memory trimming always dropped oldest rows, which could immediately evict newly fetched older pages once the active cache reached 4,000 items, blocking deep history traversal.

### Fixes Applied

- Updated `messageStore` memory enforcement to use trim direction based on operation:
  - default path keeps recent messages,
  - older-history pagination path keeps newly fetched older pages.
- Added regression coverage in `messageStore.test.ts` (`keeps older pages visible when active timeline hits memory cap`).

### Validation

- `cd client && npm run lint && npm run check && npm run test && npm run build`
- `cd server && cargo fmt --check && cargo clippy -- -D warnings && cargo test`
