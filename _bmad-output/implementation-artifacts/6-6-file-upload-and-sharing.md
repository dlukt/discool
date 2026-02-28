# Story 6.6: File Upload and Sharing

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a **user**,
I want to upload and share files in channels,
so that I can share images, documents, and other files with my guild.

## Acceptance Criteria

1. **Given** a user is in a text channel  
   **When** they click the file upload button (paperclip icon) or drag-and-drop a file onto the message area  
   **Then** a file preview chip appears above the message input.

2. **Given** a user selected a file  
   **When** they compose a message  
   **Then** they can add optional text alongside the file.

3. **Given** a user selected a file  
   **When** they click Send  
   **Then** the file is uploaded and a message is created in the channel.

4. **Given** a file upload is in progress  
   **When** the upload starts  
   **Then** a progress bar shows upload percentage in the message input area.

5. **Given** an uploaded file is an image  
   **When** the message is rendered  
   **Then** the image renders inline at a reasonable size and can be clicked to view fullscreen.

6. **Given** an uploaded file is not an image  
   **When** the message is rendered  
   **Then** it renders as a download link card with file name, size, and type icon.

7. **Given** attachment uploads are accepted  
   **When** files are stored  
   **Then** they are written to the configured local filesystem path using a `FileStorageProvider` abstraction for future S3 support.

8. **Given** a user uploads a file  
   **When** file metadata is validated  
   **Then** file type and size are validated server-side using TOML-configured limits.

9. **Given** a user drags a file over the message area  
   **When** drag-over is active  
   **Then** a drag-drop overlay appears.

10. **Given** a user lacks `ATTACH_FILES` in the channel  
    **When** they attempt to upload  
    **Then** the action is denied with a permission error.

## Tasks / Subtasks

- [x] Task 1: Add attachment persistence schema and model operations (AC: 3, 5, 6, 7)
  - [x] Add migration `server/migrations/0021_create_message_attachments.sql` with FK to `messages(id)` and metadata columns (storage key, original filename, mime type, size bytes, created timestamp), plus indexes for timeline hydration.
  - [x] Add `server/src/models/message_attachment.rs` and export it via `server/src/models/mod.rs` with insert/list helpers for both single-message write and history-page hydration by message ids.
  - [x] Keep message deletes cascading to attachments through FK constraints to prevent orphan metadata.

- [x] Task 2: Add attachment config + local filesystem storage provider (AC: 7, 8)
  - [x] Extend config with an attachments section in `server/src/config/settings.rs` (for example `attachments.upload_dir` and `attachments.max_size_bytes`) and document defaults in `config.example.toml`.
  - [x] Mirror existing startup validation patterns: reject empty upload dir/zero max size and create directories during config validation.
  - [x] Introduce `server/src/services/file_storage_service.rs` with a `FileStorageProvider` abstraction and local filesystem implementation; wire into `server/src/services/mod.rs`.

- [x] Task 3: Implement authenticated message attachment upload endpoint (AC: 2, 3, 8, 10)
  - [x] Add `POST /api/v1/guilds/{guild_slug}/channels/{channel_slug}/messages/attachments` in `server/src/handlers/mod.rs`.
  - [x] Implement multipart parsing in `server/src/handlers/messages.rs` (`file` required, optional `content`, optional `client_nonce`) and return the standard `{ "data": ... }` envelope.
  - [x] Add message-service entrypoint that enforces guild membership + channel visibility, `SEND_MESSAGES`, and `ATTACH_FILES`, reusing existing effective permission resolution instead of duplicating checks.
  - [x] Validate declared MIME vs sniffed content, enforce max size, and return structured validation/forbidden errors on failure.

- [x] Task 4: Persist-before-broadcast attachment message flow (AC: 3, 5, 6)
  - [x] Keep writes server-authoritative: persist message + attachment metadata before any websocket fanout.
  - [x] Because websocket binary frames are intentionally unsupported, keep upload transport on REST and then broadcast `message_create` using existing channel fanout (`ws/registry.rs`) after successful persistence.
  - [x] Extend `MessageResponse` payloads to include attachment data so live events and history reads stay schema-consistent.

- [x] Task 5: Extend message read surfaces and client models for attachments (AC: 3, 5, 6)
  - [x] Update server history path (`list_messages`) to hydrate attachments in the same page query flow used for reactions.
  - [x] Extend chat wire/domain types in `client/src/lib/features/chat/types.ts` to map attachment fields (`snake_case` -> `camelCase`).
  - [x] Update `messageStore.svelte.ts` parsing and reconciliation so optimistic/live/history flows preserve attachment arrays without regressing existing reaction behavior.

- [x] Task 6: Implement composer upload UX (paperclip + drag/drop + preview) (AC: 1, 2, 4, 9, 10)
  - [x] Add paperclip trigger + hidden file input in `MessageArea.svelte`; show preview chip(s) with remove action before send.
  - [x] Add drag-over state and overlay for drop targets in the message area with accessible keyboard/mouse behavior.
  - [x] Add progress UI bound to upload lifecycle and disable send while upload is actively in flight.
  - [x] Derive `canAttachFiles` using existing role bitflag patterns (as done in ChannelList/MemberList) for client-side affordance only; keep server checks authoritative.

- [x] Task 7: Implement attachment rendering in message bubbles (AC: 5, 6)
  - [x] Extend `MessageBubble.svelte` to render image attachments inline with click-to-fullscreen behavior.
  - [x] Render non-image attachments as file cards/links showing filename, formatted size, and type icon.
  - [x] Resolve virtualized-row height regressions in `MessageArea.svelte` (current fixed row-height assumptions are not attachment-safe); use measured/dynamic heights or an equivalent deterministic strategy.

- [x] Task 8: Add regression coverage and run quality gates (AC: all)
  - [x] Server integration tests in `server/tests/server_binds_to_configured_port.rs` for multipart upload success, permission-denied (`ATTACH_FILES`), invalid type, and oversize rejection paths.
  - [x] Server unit tests for attachment validation, storage-key safety, and service-level permission/content rules (including text-optional-with-file behavior).
  - [x] Client tests (`MessageArea.test.ts`, `MessageBubble.test.ts`, `messageStore.test.ts`) for file selection preview, drag overlay, progress UI, and attachment rendering.
  - [x] Run full quality gates before moving story beyond ready-for-dev:
    - [x] `cd client && npm run lint && npm run check && npm run test && npm run build`
    - [x] `cd server && cargo fmt --check && cargo clippy -- -D warnings && cargo test`

## Dev Notes

### Developer Context

- Current message writes are websocket op-driven (`c_message_create`, `c_message_update`, `c_message_delete`, `c_message_reaction_toggle`), while REST currently only serves message history reads.
- Websocket binary frames are explicitly rejected in gateway handling, so file bytes must not be sent through websocket ops.
- The `messages` schema currently has no attachment columns/table; attachment persistence is net-new.
- Upload prior art already exists for avatar and guild icon multipart flows with MIME sniffing, declared MIME checks, local filesystem writes, and old-file cleanup.
- Permission catalogs already include `ATTACH_FILES` on both server and client, but message creation currently enforces only send/view checks.
- Timeline virtualization in `MessageArea.svelte` currently assumes fixed row heights; inline attachments introduce variable heights that must be handled explicitly.

### Technical Requirements

- Preserve backend layering (`handlers -> services -> models`) and avoid bypassing service validation from handlers.
- Keep websocket contract stable (`c_*` client ops, `snake_case` server ops, `{op,d,s,t}` envelope); do not introduce binary websocket message upload.
- Attachment send path should be: multipart REST upload -> service validation + persistence -> websocket `message_create` fanout with attachment payload.
- Maintain existing sanitize-before-store behavior for text content; for attachment messages, allow empty text only when at least one valid attachment is present.
- Enforce both permission and membership checks server-side (`VIEW_CHANNEL`, `SEND_MESSAGES`, `ATTACH_FILES`) with existing channel override-aware permission resolution.
- Validate file MIME via sniffing and declared MIME consistency; enforce configurable max size; reject unsupported types with explicit `VALIDATION_ERROR`.
- Keep file storage safe: generate server-side storage keys, prevent path traversal, and clean up on partial failures (no silent failures).
- Keep payload compatibility: existing text-only messages should continue returning stable fields, with attachments defaulting to an empty list.

### Architecture Compliance

1. Keep REST response envelopes and websocket payload naming aligned with current API conventions (`snake_case` wire, typed mapping in client feature layer).
2. Reuse existing registry fanout and sequence handling for `message_create` broadcasts instead of inventing a parallel event transport.
3. Preserve Story 6.3 cursor pagination and memory-budget behavior while enriching message payloads with attachments.
4. Keep attachment storage implementation behind a provider abstraction consistent with architecture guidance for future backend swap (local now, S3 later).
5. Reuse established multipart and validation patterns from avatar/guild icon uploads instead of duplicating divergent logic.

### Library & Framework Requirements

- Pinned project dependencies remain:
  - `svelte`: `^5.45.2`
  - `@mateothegreat/svelte5-router`: `^2.16.19`
  - `axum`: `0.8`
  - `sqlx`: `0.8`
- Latest checks during story creation:
  - `svelte` latest: `5.53.6`
  - `@mateothegreat/svelte5-router` latest: `2.16.19`
  - `axum` latest release tag: `axum-v0.8.8`
  - `sqlx` latest tag: `v0.8.6`
- No dependency upgrade is required to implement this story.

### File Structure Requirements

Expected primary touch points:

- Server
  - `server/migrations/0021_create_message_attachments.sql` (new)
  - `server/src/models/message_attachment.rs` (new)
  - `server/src/models/mod.rs`
  - `server/src/services/file_storage_service.rs` (new)
  - `server/src/services/mod.rs`
  - `server/src/services/message_service.rs`
  - `server/src/handlers/messages.rs`
  - `server/src/handlers/mod.rs`
  - `server/src/config/settings.rs`
  - `config.example.toml`
  - `server/tests/server_binds_to_configured_port.rs`

- Client
  - `client/src/lib/features/chat/messageApi.ts`
  - `client/src/lib/features/chat/types.ts`
  - `client/src/lib/features/chat/messageStore.svelte.ts`
  - `client/src/lib/features/chat/MessageArea.svelte`
  - `client/src/lib/features/chat/MessageBubble.svelte`
  - `client/src/lib/features/chat/MessageArea.test.ts`
  - `client/src/lib/features/chat/MessageBubble.test.ts`
  - `client/src/lib/features/chat/messageStore.test.ts`

### Testing Requirements

- Server integration coverage should verify:
  - multipart attachment upload success with optional text,
  - real-time channel fanout for attachment message creates,
  - `ATTACH_FILES` permission-denied behavior,
  - oversize and unsupported MIME validation failures.
- Server unit coverage should verify:
  - content normalization rules for file-only vs text-only messages,
  - storage key/path safety and cleanup-on-failure semantics,
  - attachment metadata mapping into message responses.
- Client coverage should verify:
  - paperclip selection and preview chip behavior,
  - drag-over overlay visibility and drop handling,
  - progress UI transitions and disabled-send states,
  - inline image and non-image attachment rendering.

### Previous Story Intelligence

- Story 6.5 already added message-adjacent persistence and payload enrichment via reactions; attachment hydration should follow the same timeline/live consistency pattern.
- Story 6.4 established composer/message action patterns and ownership-safe mutation behavior that should remain intact when adding upload affordances.
- Story 6.3 established cursor history + virtualized timeline constraints; attachment rendering must not break scroll restoration or memory limits.
- Story 6.2 established persist-before-broadcast and sanitize-before-store behavior for messages; attachment writes should retain that trust boundary.

### Git Intelligence Summary

Recent commit conventions and implementation patterns to preserve:

- `46550ff` feat: finalize story 6-5 emoji reactions
- `bfe2ad1` feat: finalize story 6-4 edit/delete own messages
- `8741b54` feat: finalize story 6-3 message history
- `3bff024` feat: finalize story 6-2 messaging and review
- `b00e451` feat: finalize story 6-1 websocket gateway and review

### Latest Technical Information

- `svelte` latest package version: `5.53.6`
- `@mateothegreat/svelte5-router` latest package version: `2.16.19`
- `tokio-rs/axum` latest release tag: `axum-v0.8.8`
- `launchbadge/sqlx` latest tag: `v0.8.6`

### Project Context Reference

- No `project-context.md` file was discovered via `**/project-context.md`.
- Story context is derived from planning artifacts, implementation artifacts, source code, and recent git history.

### Story Completion Status

- Story 6.6 attachment upload and sharing scope has been implemented across server, client, and tests.
- Story status transitioned to `review` after passing quality gates.

### Project Structure Notes

- Keep message mutations centralized in `message_service`; do not split attachment business rules into handler-local SQL.
- Keep chat feature boundaries unchanged: API adapters in `messageApi.ts`, state/reconciliation in `messageStore.svelte.ts`, composer/timeline in `MessageArea.svelte`, row rendering in `MessageBubble.svelte`.
- Route additions should remain under existing guild/channel message namespace to preserve API discoverability and permission middleware behavior.

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Story 6.6: File Upload and Sharing]
- [Source: _bmad-output/planning-artifacts/epics.md#Epic 6: Real-Time Text Communication]
- [Source: _bmad-output/planning-artifacts/prd.md#Functional Requirements]
- [Source: _bmad-output/planning-artifacts/prd.md#Text Communication]
- [Source: _bmad-output/planning-artifacts/prd.md#Non-Functional Requirements]
- [Source: _bmad-output/planning-artifacts/architecture.md#Implementation Patterns & Consistency Rules]
- [Source: _bmad-output/planning-artifacts/architecture.md#API & Communication Patterns]
- [Source: _bmad-output/planning-artifacts/architecture.md#Frontend Architecture]
- [Source: _bmad-output/planning-artifacts/architecture.md#Gap Analysis Results]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#MessageInput]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#MessageBubble]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Feedback Patterns]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Loading Patterns]
- [Source: _bmad-output/implementation-artifacts/6-5-emoji-reactions.md]
- [Source: _bmad-output/implementation-artifacts/6-4-edit-and-delete-own-messages.md]
- [Source: _bmad-output/implementation-artifacts/6-3-message-history-and-virtual-scrolling.md]
- [Source: _bmad-output/implementation-artifacts/6-2-send-and-display-text-messages.md]
- [Source: _bmad-output/implementation-artifacts/sprint-status.yaml]
- [Source: server/src/ws/protocol.rs]
- [Source: server/src/ws/gateway.rs]
- [Source: server/src/ws/registry.rs]
- [Source: server/src/handlers/mod.rs]
- [Source: server/src/handlers/messages.rs]
- [Source: server/src/handlers/users.rs]
- [Source: server/src/handlers/guilds.rs]
- [Source: server/src/services/message_service.rs]
- [Source: server/src/services/user_profile_service.rs]
- [Source: server/src/services/guild_service.rs]
- [Source: server/src/services/mod.rs]
- [Source: server/src/models/message.rs]
- [Source: server/src/models/mod.rs]
- [Source: server/src/permissions/mod.rs]
- [Source: server/src/config/settings.rs]
- [Source: server/migrations/0018_create_messages.sql]
- [Source: server/migrations/0019_add_messages_cursor_index.sql]
- [Source: server/migrations/0020_create_message_reactions.sql]
- [Source: server/tests/server_binds_to_configured_port.rs]
- [Source: client/src/lib/ws/protocol.ts]
- [Source: client/src/lib/features/chat/messageApi.ts]
- [Source: client/src/lib/features/chat/types.ts]
- [Source: client/src/lib/features/chat/messageStore.svelte.ts]
- [Source: client/src/lib/features/chat/MessageArea.svelte]
- [Source: client/src/lib/features/chat/MessageBubble.svelte]
- [Source: client/src/lib/features/channel/ChannelList.svelte]
- [Source: client/src/lib/features/members/MemberList.svelte]
- [Source: client/src/lib/features/guild/permissions.ts]
- [Source: client/package.json]
- [Source: server/Cargo.toml]
- [Source: https://registry.npmjs.org/svelte/latest]
- [Source: https://registry.npmjs.org/@mateothegreat/svelte5-router/latest]
- [Source: https://api.github.com/repos/tokio-rs/axum/releases/latest]
- [Source: https://api.github.com/repos/launchbadge/sqlx/tags?per_page=5]

## Dev Agent Record

### Agent Model Used

GPT-5.3-Codex (model ID: gpt-5.3-codex)

### Debug Log References

- `_bmad/core/tasks/workflow.xml` loaded and applied with `_bmad/bmm/workflows/4-implementation/dev-story/workflow.yaml`.
- Client quality gate executed and passed: `cd client && npm run lint && npm run check && npm run test && npm run build`.
- Server quality gate executed and passed: `cd server && cargo fmt --check && cargo clippy -- -D warnings && RUST_TEST_THREADS=1 cargo test`.

### Completion Notes List

- Implemented attachment persistence, upload/download endpoints, config + storage provider abstraction, and message payload hydration.
- Implemented composer paperclip/drag-drop/progress UX and attachment rendering (inline images + file cards + preview modal) in chat UI.
- Added/updated server and client tests for success, permission, validation, upload flow, and rendering behavior; quality gates are green.

### File List

- server/migrations/0021_create_message_attachments.sql
- server/src/models/message_attachment.rs
- server/src/models/mod.rs
- server/src/services/file_storage_service.rs
- server/src/services/mod.rs
- server/src/services/message_service.rs
- server/src/handlers/messages.rs
- server/src/handlers/mod.rs
- server/src/handlers/ws.rs
- server/src/ws/gateway.rs
- server/src/config/mod.rs
- server/src/config/settings.rs
- config.example.toml
- server/tests/server_binds_to_configured_port.rs
- client/src/lib/features/chat/types.ts
- client/src/lib/features/chat/messageApi.ts
- client/src/lib/features/chat/messageStore.svelte.ts
- client/src/lib/features/chat/MessageArea.svelte
- client/src/lib/features/chat/MessageBubble.svelte
- client/src/lib/features/chat/messageStore.test.ts
- client/src/lib/features/chat/MessageArea.test.ts
- client/src/lib/features/chat/MessageBubble.test.ts
- _bmad-output/implementation-artifacts/6-6-file-upload-and-sharing.md
- _bmad-output/implementation-artifacts/sprint-status.yaml

### Change Log

- 2026-02-28: Implemented Story 6.6 file upload and sharing end-to-end and passed full client/server quality gates.
- 2026-02-28: Senior Developer Review (AI) completed in YOLO mode; fixed attachment-write rollback cleanup in `server/src/services/message_service.rs`, updated story file list to match git changes, re-ran full quality gates, and moved story to `done`.

### Senior Developer Review (AI)

- Reviewer: Darko (GPT-5.3-Codex)
- Date: 2026-02-28
- Outcome: **Approve**
- Findings:
  - Fixed 1 HIGH issue in `server/src/services/message_service.rs`: when attachment storage write failed, rollback removed DB rows but could leave a partially written attachment file; failure path now performs file cleanup after successful rollback and logs cleanup failures.
  - Fixed 1 MEDIUM issue in this story document: Dev Agent Record `File List` missed files that were modified in git (`server/src/handlers/ws.rs`, `server/src/ws/gateway.rs`, `server/src/config/mod.rs`).
  - No remaining actionable HIGH or MEDIUM issues after fix and re-review.
  - Acceptance Criteria and task claims were cross-checked against implementation and test coverage.
- Validation: `cd client && npm run lint && npm run check && npm run test && npm run build && cd ../server && cargo fmt --check && cargo clippy -- -D warnings && RUST_TEST_THREADS=1 cargo test`
