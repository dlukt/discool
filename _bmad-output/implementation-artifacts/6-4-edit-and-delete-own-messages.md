# Story 6.4: Edit and Delete Own Messages

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a **user**,
I want to edit and delete my own messages,
so that I can correct mistakes or remove messages I no longer want visible.

## Acceptance Criteria

1. **Given** a user has sent a message  
   **When** they press Up arrow (with empty input) or right-click -> Edit  
   **Then** the message enters inline edit mode with the original text in the input

2. **Given** a message is in edit mode  
   **When** the user presses Enter  
   **Then** the edit is saved

3. **Given** a message is in edit mode  
   **When** the user presses Escape  
   **Then** edit mode is canceled without changing the message

4. **Given** a message was edited  
   **When** it renders in the timeline  
   **Then** it shows an **(edited)** label next to the timestamp

5. **Given** an edit is submitted  
   **When** the server accepts it  
   **Then** the edit is sent via WebSocket and broadcast as `message_update` to channel subscribers

6. **Given** a user right-clicks their own message and chooses Delete (or presses Del)  
   **When** they confirm the action in a confirmation dialog  
   **Then** the message is removed from view for all users via `message_delete`

7. **Given** a message row is hovered  
   **When** the pointer is over the message  
   **Then** a top-right hover action bar is shown with edit, delete, react, and reply actions

## Tasks / Subtasks

- [x] Task 1: Add server-side message edit/delete data operations (AC: 4, 5, 6)
  - [x] Extend `server/src/models/message.rs` with explicit update/delete operations for persisted messages (targeting message id + channel id + author id constraints).
  - [x] Keep deterministic timestamp behavior by updating `updated_at` on successful edits and preserving `created_at`.
  - [x] Keep query patterns/index usage compatible with Story 6.3 history pagination.

- [x] Task 2: Implement message edit/delete business rules in service layer (AC: 1, 2, 3, 4, 5, 6)
  - [x] Extend `server/src/services/message_service.rs` with edit/delete entry points that enforce guild membership + channel visibility and ownership checks for own-message operations.
  - [x] Reuse existing message normalization/sanitization guardrails for edited content (`MAX_MESSAGE_CHARS`, control-char validation, HTML escaping).
  - [x] Reject edits/deletes for non-owned messages with structured `FORBIDDEN` errors (moderator-wide delete remains future moderation scope in Epic 8).
  - [x] Return payload shape needed by WebSocket broadcast, including fields required to render edited status in client.

- [x] Task 3: Wire WebSocket protocol and gateway handling for update/delete ops (AC: 5, 6)
  - [x] Extend `server/src/ws/protocol.rs` client operation parsing with `c_message_update` and `c_message_delete` while preserving `c_` prefix and validation behavior.
  - [x] Extend `server/src/ws/gateway.rs` payload parsing and dispatch to new message service edit/delete flows.
  - [x] Broadcast `message_update` and `message_delete` using existing channel-targeted registry fanout and standard envelope fields (`op`, `d`, `s`, `t`).
  - [x] Keep rate-limit and structured error event behavior unchanged.

- [x] Task 4: Extend client WS protocol and chat store event ingestion (AC: 4, 5, 6)
  - [x] Add `c_message_update` and `c_message_delete` to `client/src/lib/ws/protocol.ts` `WsClientOp` union.
  - [x] Extend `client/src/lib/features/chat/messageStore.svelte.ts` to ingest and reconcile `message_update` and `message_delete` events without breaking ordering, virtualization, or pending counters.
  - [x] Ensure optimistic create reconciliation (`client_nonce`) remains intact when update/delete events arrive for nearby messages.

- [x] Task 5: Implement message edit/delete UX in chat components (AC: 1, 2, 3, 4, 6, 7)
  - [x] Extend `client/src/lib/features/chat/MessageBubble.svelte` with hover action affordances and edited label rendering.
  - [x] Extend `client/src/lib/features/chat/MessageArea.svelte` composer workflow to support:
    - [x] Up-arrow on empty input to edit latest own message.
    - [x] Enter to submit edit and Escape to cancel.
    - [x] Delete confirmation dialog flow before sending delete op.
  - [x] Add right-click/context-menu and keyboard context-menu support (`ContextMenu` key / `Shift+F10`) for message actions.
  - [x] Ensure destructive action labeling remains explicit (`Delete message`) and keyboard flows remain complete.

- [x] Task 6: Testing and quality gates (AC: all)
  - [x] Server integration tests in `server/tests/server_binds_to_configured_port.rs`:
    - [x] owner can edit own message and all subscribers receive `message_update`
    - [x] owner can delete own message and all subscribers receive `message_delete`
    - [x] non-owner edit/delete attempts return `FORBIDDEN`
    - [x] invalid edit payloads return `VALIDATION_ERROR`
  - [x] Server unit tests in message model/service modules for edit/delete ownership and validation behavior.
  - [x] Client tests:
    - [x] `MessageArea.test.ts` for Up/Enter/Escape edit flow and delete confirmation behavior
    - [x] `messageStore.test.ts` for `message_update` and `message_delete` ingestion correctness
    - [x] `MessageBubble` rendering assertions for edited label and action affordances
  - [x] Run quality gates:
    - [x] `cd client && npm run lint && npm run check && npm run test && npm run build`
    - [x] `cd server && cargo fmt --check && cargo clippy -- -D warnings && cargo test`

## Dev Notes

### Developer Context

- Story 6.3 completed cursor-based history, virtualization, and scroll restoration; message edit/delete must not regress these timeline behaviors.
- Current runtime handles message creation end-to-end (`c_message_create` -> persisted row -> `message_create` WS fanout), but has no edit/delete message operations yet.
- WebSocket server protocol already defines `message_update` and `message_delete` as server events, making this story primarily about implementing missing operation handling and client ingestion.
- Messages table already has `updated_at`, which should be leveraged to represent edited-state semantics without introducing ad-hoc client-only flags.

### Technical Requirements

- Enforce ownership for edit/delete own-message operations on the server (never trust client claims).
- Preserve existing sanitize-before-store policy when editing content.
- Keep `{"error":{"code","message","details":{}}}` semantics across failure paths.
- Keep message operations channel-scoped and subscription-scoped for broadcasts.
- Keep composer behavior predictable:
  - Up only triggers edit when composer is empty.
  - Enter submits in edit mode.
  - Escape cancels edit mode cleanly.
- Delete is destructive and must require explicit confirmation.

### Architecture Compliance

1. Keep strict layering: `ws/handlers -> services/message_service -> models/message`.
2. Keep WS naming conventions: client ops `c_*`, server ops snake_case.
3. Preserve JSON boundary consistency (`snake_case` on wire, mapped to `camelCase` in client).
4. Maintain virtualized timeline performance and memory constraints from Story 6.3.
5. Keep reconnect and non-blocking UX behavior from Story 6.1.

### Library & Framework Requirements

- Repo pinned versions:
  - `svelte`: `^5.45.2`
  - `@mateothegreat/svelte5-router`: `^2.16.19`
  - `axum`: `0.8`
  - `sqlx`: `0.8`
- Latest checks during story creation:
  - Svelte latest: `5.53.6`
  - `@mateothegreat/svelte5-router` latest: `2.16.19`
  - Axum latest release tag: `axum-v0.8.8`
  - SQLx latest tag: `v0.8.6`
- No dependency upgrade is required for this story.

### File Structure Requirements

Expected primary touch points:

- Server
  - `server/src/ws/protocol.rs`
  - `server/src/ws/gateway.rs`
  - `server/src/services/message_service.rs`
  - `server/src/models/message.rs`
  - `server/tests/server_binds_to_configured_port.rs`
- Client
  - `client/src/lib/ws/protocol.ts`
  - `client/src/lib/features/chat/messageStore.svelte.ts`
  - `client/src/lib/features/chat/MessageArea.svelte`
  - `client/src/lib/features/chat/MessageBubble.svelte`
  - `client/src/lib/features/chat/MessageArea.test.ts`
  - `client/src/lib/features/chat/messageStore.test.ts`

### Testing Requirements

- Validate AC keyboard/edit flows with component tests (Up, Enter, Escape).
- Validate destructive flow guardrail (confirmation required before delete send).
- Validate server ownership enforcement and error contracts for invalid/unauthorized edit/delete.
- Validate channel broadcast behavior (`message_update`, `message_delete`) for subscribed peers.
- Re-run full repo quality gate commands before marking story done.

### Previous Story Intelligence

- Story 6.3 introduced cursor-based message history and virtualization; edits/deletes must reconcile cleanly with partially loaded timelines.
- Story 6.3 hardened memory trimming behavior for active channels; update/delete ingestion must not reintroduce trimming regressions.
- Story 6.2 established sanitize-before-store and persist-before-broadcast guarantees; edited content must follow the same trust boundary.
- Story 6.1 established WS reconnect/lifecycle and `c_` operation patterns; edit/delete operations should follow the same protocol conventions.

### Git Intelligence Summary

Recent commit patterns to preserve:

- `8741b54` feat: finalize story 6-3 message history
- `3bff024` feat: finalize story 6-2 messaging and review
- `b00e451` feat: finalize story 6-1 websocket gateway and review
- `e1b2e8a` feat: finalize story 5-7 member list with presence
- `afa9352` feat: finalize story 5-6 role management delegation

### Latest Technical Information

- `svelte` latest package version: `5.53.6`
- `@mateothegreat/svelte5-router` latest package version: `2.16.19`
- `tokio-rs/axum` latest release tag: `axum-v0.8.8`
- `launchbadge/sqlx` latest tag: `v0.8.6`
- Current pinned repo versions already align with this story scope; no upgrade required.

### Project Context Reference

- No `project-context.md` file was discovered via `**/project-context.md`.
- Story context is derived from planning artifacts, existing implementation artifacts, current source code, and recent git history.

### Story Completion Status

- Ultimate context engine analysis completed - comprehensive developer guide created.
- Story status set to `ready-for-dev`.

### Project Structure Notes

- Chat feature boundaries currently split by `MessageArea.svelte` (timeline/composer shell), `MessageBubble.svelte` (row rendering), `messageStore.svelte.ts` (state + WS ingestion), and `messageApi.ts` (REST history); keep edit/delete work within this feature module.
- Current server message routing for REST is read-only (`GET .../messages`) while write behavior is WebSocket-driven in `ws/gateway.rs`; this story should extend that established write path instead of introducing parallel mutation channels.

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Epic 6: Real-Time Text Communication]
- [Source: _bmad-output/planning-artifacts/epics.md#Story 6.4: Edit and Delete Own Messages]
- [Source: _bmad-output/planning-artifacts/prd.md#Text Communication]
- [Source: _bmad-output/planning-artifacts/prd.md#Non-Functional Requirements]
- [Source: _bmad-output/planning-artifacts/architecture.md#Implementation Patterns & Consistency Rules]
- [Source: _bmad-output/planning-artifacts/architecture.md#Communication Patterns]
- [Source: _bmad-output/planning-artifacts/architecture.md#State Boundaries (Frontend)]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#MessageBubble]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Interaction Patterns]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Keyboard Shortcut Patterns]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Loading Patterns]
- [Source: _bmad-output/implementation-artifacts/6-3-message-history-and-virtual-scrolling.md]
- [Source: _bmad-output/implementation-artifacts/6-2-send-and-display-text-messages.md]
- [Source: _bmad-output/implementation-artifacts/sprint-status.yaml]
- [Source: server/src/ws/protocol.rs]
- [Source: server/src/ws/gateway.rs]
- [Source: server/src/services/message_service.rs]
- [Source: server/src/models/message.rs]
- [Source: server/src/handlers/messages.rs]
- [Source: client/src/lib/features/chat/MessageArea.svelte]
- [Source: client/src/lib/features/chat/MessageBubble.svelte]
- [Source: client/src/lib/features/chat/messageStore.svelte.ts]
- [Source: client/src/lib/features/chat/MessageArea.test.ts]
- [Source: client/src/lib/features/chat/messageStore.test.ts]
- [Source: client/src/lib/features/members/MemberList.svelte]
- [Source: client/src/lib/features/channel/ChannelList.svelte]
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
- Story source parsed from user input: `6-4` -> `6-4-edit-and-delete-own-messages`
- Implemented server edit/delete operations in model, service, and websocket gateway/protocol layers
- Implemented client edit/delete operations in chat store and UI keyboard/context-menu workflows
- Added server and client coverage for update/delete success and validation/authorization failures
- Quality gates executed successfully after implementation:
  - `cd client && npm run lint && npm run check && npm run test && npm run build`
  - `cd server && cargo fmt --check && cargo clippy -- -D warnings && cargo test`

### Completion Notes List

- Added persisted message update/delete model operations with author+channel scoping and deterministic `updated_at` handling.
- Added `update_message`/`delete_message` service flows enforcing membership, channel visibility, own-message ownership, and sanitize-before-store rules.
- Added websocket client operation handling (`c_message_update`, `c_message_delete`) with `message_update` / `message_delete` channel broadcasts.
- Added chat store support for update/delete sends and event ingestion while preserving existing timeline ordering and optimistic create reconciliation.
- Added chat UX support for Up-arrow edit start, Enter save, Escape cancel, delete confirmation dialog, context-menu/keyboard message actions, and hover action bar.
- Added tests for server model/service ownership/validation behavior, websocket update/delete fanout + forbidden/validation failures, and client edit/delete UI/store rendering behavior.

### File List

- _bmad-output/implementation-artifacts/6-4-edit-and-delete-own-messages.md
- client/src/lib/features/chat/MessageArea.svelte
- client/src/lib/features/chat/MessageArea.test.ts
- client/src/lib/features/chat/MessageBubble.svelte
- client/src/lib/features/chat/MessageBubble.test.ts
- client/src/lib/features/chat/messageStore.svelte.ts
- client/src/lib/features/chat/messageStore.test.ts
- client/src/lib/features/chat/types.ts
- client/src/lib/ws/protocol.ts
- server/src/models/message.rs
- server/src/services/message_service.rs
- server/src/ws/gateway.rs
- server/src/ws/protocol.rs
- server/tests/server_binds_to_configured_port.rs
- _bmad-output/implementation-artifacts/sprint-status.yaml

### Change Log

- 2026-02-28: Implemented Story 6.4 edit/delete own-message flow end-to-end across server, websocket protocol, chat store, chat UI, and automated tests; story advanced to `review`.
- 2026-02-28: Senior Developer Review (AI) completed in YOLO mode; no HIGH/MEDIUM findings, no code fixes required, and story moved to `done`.

### Senior Developer Review (AI)

- Reviewer: Darko (GPT-5.3-Codex)
- Date: 2026-02-28
- Outcome: **Approve**
- Findings:
  - No actionable HIGH or MEDIUM issues found.
  - Git changes and story File List match exactly (0 discrepancies).
  - Acceptance Criteria and completed tasks were validated against implementation and automated tests.
- Validation: `cd client && npm run lint && npm run check && npm run test && npm run build && cd ../server && cargo fmt --check && cargo clippy -- -D warnings && cargo test`
