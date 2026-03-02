# Story 8.6: Message Deletion by Moderators

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a **moderator**,
I want to delete any user's messages,
so that I can remove inappropriate content from channels.

## Acceptance Criteria

1. **Given** a user has `MANAGE_MESSAGES` permission  
   **When** they right-click a message -> Delete message  
   **Then** the message is removed from view for all users.

2. **Given** a moderator deletes a message  
   **When** the action is submitted  
   **Then** the moderator must provide a reason, and that reason is logged in the moderation log.

3. **Given** a moderated message deletion is performed  
   **When** the backend persists the change  
   **Then** the message is soft-deleted in the database (not permanently erased) to preserve audit trail requirements.

4. **Given** a user has blocked another user  
   **When** blocked-user messages are moderated/deleted and history is reloaded  
   **Then** blocked content remains invisible to the blocking user (block erasure precedence is preserved).

5. **Given** a message deletion succeeds  
   **When** the UI receives the successful result  
   **Then** a toast confirms exactly: `"Message deleted"`.

## Tasks / Subtasks

- [x] Task 1: Add moderated soft-delete persistence for messages (AC: 2, 3, 4)
  - [x] Add migration `server/migrations/0031_add_moderated_message_soft_delete_fields.sql` for message soft-delete metadata (for example: `deleted_at`, `deleted_by_user_id`, `deleted_reason`, `deleted_moderation_action_id`) while preserving existing constraints and DB parity.
  - [x] Keep PostgreSQL/SQLite compatibility using the established migration patterns from `0027`/`0028`/`0029`/`0030`.
  - [x] Add/adjust indexes used by channel history queries so visibility filtering for non-deleted rows remains performant.

- [x] Task 2: Extend message model/service logic for moderated delete behavior (AC: 1, 3, 4)
  - [x] Add model methods for moderated soft-delete (`server/src/models/message.rs`) without breaking existing owner-initiated hard delete paths.
  - [x] Ensure message list/read paths used by channel history do not return soft-deleted rows to normal users.
  - [x] Ensure attachment retrieval for soft-deleted parent messages returns `NotFound` to prevent deleted-content access.

- [x] Task 3: Add moderation service operation for message deletion (AC: 1, 2, 3)
  - [x] Add typed input/output models in `server/src/services/moderation_service.rs` for deleting a specific message with required `reason`.
  - [x] Enforce `MANAGE_MESSAGES` via `permissions::require_guild_permission(..., MANAGE_MESSAGES, "MANAGE_MESSAGES")`.
  - [x] Reuse existing moderation reason validation (`normalize_reason`) and role-safety checks consistent with existing kick/ban patterns.
  - [x] Record the action in `moderation_actions` with `action_type = message_delete` and include moderator/target/reason/timestamp.

- [x] Task 4: Expose moderation HTTP API and broadcast channel delete events (AC: 1, 5)
  - [x] Add handler in `server/src/handlers/moderation.rs` for `POST /api/v1/guilds/{guild_slug}/moderation/messages/{message_id}/delete` with required `channel_slug` + `reason`.
  - [x] Register route in `server/src/handlers/mod.rs`.
  - [x] On successful moderation delete, broadcast `ServerOp::MessageDelete` to channel subscribers (same UX effect as websocket owner-delete path).
  - [x] Keep API contract conventions: `snake_case` fields and `{ "data": ... }` envelope.

- [x] Task 5: Implement moderator message-delete UX in chat (AC: 1, 2, 5)
  - [x] Update message action visibility in `client/src/lib/features/chat/MessageBubble.svelte` so delete is available for moderators with `MANAGE_MESSAGES` (not only own messages).
  - [x] Add reason-required moderator delete dialog flow in `client/src/lib/features/chat/MessageArea.svelte` (reuse moderation reason UX conventions from existing mute/kick/ban dialogs).
  - [x] Extend `client/src/lib/features/moderation/moderationApi.ts` with a message-delete moderation API method.
  - [x] Preserve exact success copy: `"Message deleted"` and consistent error/toast behavior.

- [x] Task 6: Preserve block erasure precedence explicitly (AC: 4)
  - [x] Confirm `client/src/lib/features/chat/messageStore.svelte.ts` blocked-content filtering behavior remains authoritative after moderated deletes and history refresh.
  - [x] Avoid introducing deleted-message placeholders/tombstones that could reveal blocked-user content metadata.

- [x] Task 7: Add regression coverage and execute quality gates (AC: all)
  - [x] Server unit/service/handler tests for permission checks, reason validation, soft-delete persistence, mod-log insertion, and websocket delete-event propagation.
  - [x] Integration test coverage in `server/tests/server_binds_to_configured_port.rs` for moderator delete happy-path + forbidden-path.
  - [x] Client tests in `MessageArea.test.ts`, `MessageBubble.test.ts`, `moderationApi.test.ts`, and `MemberList.test.ts` for visibility, reason requirement, and toast copy.
  - [x] Run quality gates:
    - [x] `cd client && npm run lint && npm run check && npm run test && npm run build`
    - [x] `cd server && cargo fmt --check && cargo clippy -- -D warnings && cargo test`

## Dev Notes

### Developer Context

- Current message delete path is websocket-only (`c_message_delete`) and enforces owner-only deletion (`"You can only delete your own messages"`), with hard delete from `messages` table.  
  [Source: server/src/ws/gateway.rs#handle_message_delete, server/src/services/message_service.rs#delete_message, server/tests/server_binds_to_configured_port.rs#websocket_message_update_delete_reject_non_owner_and_invalid_payloads]
- Moderation log infrastructure is already in place and includes `message_delete` as a valid moderation action type.
  [Source: server/migrations/0030_expand_moderation_actions_for_log_view.sql, server/src/models/moderation.rs]
- `MANAGE_MESSAGES` permission exists server/client, but member-panel action is still placeholder `"Moderate messages (coming soon)"`; this story should implement real message-level moderation behavior.
  [Source: server/src/permissions/mod.rs, client/src/lib/features/guild/permissions.ts, client/src/lib/features/members/MemberList.svelte, client/src/lib/features/members/MemberList.test.ts]
- Client block erasure currently filters messages/reactions by blocked author and block window; moderated delete changes must preserve this behavior.
  [Source: client/src/lib/features/chat/messageStore.svelte.ts]

### Technical Requirements

- Require `MANAGE_MESSAGES` for deleting others' messages; owners remain implicitly authorized by existing permission system.
- Moderation delete must require non-empty reason and respect max moderation reason length (`500`) using existing validation patterns.
- Soft-delete moderated messages in DB; do not permanently erase content needed for audit trails.
- Ensure moderated-deleted messages are absent from normal channel history and active timeline for all users.
- Ensure attachment downloads tied to soft-deleted messages are not retrievable through public attachment endpoints.
- Persist mod-log entry for each moderated delete (`action_type = message_delete`) with actor, target, reason, timestamp.
- Preserve existing websocket owner-delete path for self-deletes unless intentionally replaced in implementation plan.

### Architecture Compliance

1. Keep boundaries strict:
   - Handlers parse/validate request payloads.
   - Services enforce authorization/business rules.
   - Models own SQL and DB-specific query branches.
2. Preserve API conventions:
   - `snake_case` wire fields.
   - `{ "data": ... }` envelope shape.
3. Maintain PostgreSQL/SQLite parity for schema and query behavior.
4. Reuse existing permission and moderation helpers; avoid parallel auth logic.
5. Keep explicit error handling (`AppError::ValidationError`, `AppError::Forbidden`, `AppError::NotFound`) and avoid silent coercion.

### Library & Framework Requirements

- Backend stack remains Rust + Axum + SQLx + Tokio (existing dependencies in `server/Cargo.toml`).
- Frontend stack remains Svelte 5 + Vite + existing state/store patterns (`client/package.json`).
- Do not introduce alternate state management or API abstraction frameworks for this story.
- Continue using existing websocket event semantics (`message_delete`) for client timeline removal synchronization.

### File Structure Requirements

Expected primary touch points:

- `server/migrations/0031_add_moderated_message_soft_delete_fields.sql` (new)
- `server/src/models/message.rs`
- `server/src/models/moderation.rs`
- `server/src/services/message_service.rs`
- `server/src/services/moderation_service.rs`
- `server/src/handlers/moderation.rs`
- `server/src/handlers/mod.rs`
- `server/src/ws/gateway.rs` (only if delete-event publishing path requires shared helper updates)
- `server/tests/server_binds_to_configured_port.rs`
- `client/src/lib/features/moderation/moderationApi.ts`
- `client/src/lib/features/moderation/moderationApi.test.ts`
- `client/src/lib/features/chat/MessageArea.svelte`
- `client/src/lib/features/chat/MessageArea.test.ts`
- `client/src/lib/features/chat/MessageBubble.svelte`
- `client/src/lib/features/chat/MessageBubble.test.ts`
- `client/src/lib/features/chat/messageStore.svelte.ts` (only if payload handling/visibility logic needs updates)
- `client/src/lib/features/members/MemberList.svelte` / `.test.ts` (if "Moderate messages" action wiring is updated in this story)

### Testing Requirements

- **Server**
  - Moderator with `MANAGE_MESSAGES` can delete another user's message with required reason.
  - User without `MANAGE_MESSAGES` receives `403 Forbidden`.
  - Soft-deleted messages are not returned by channel history APIs.
  - Soft-deleted message attachments are not downloadable.
  - Moderation log includes `message_delete` entry with correct fields.
  - Channel participants receive `message_delete` event after moderation delete.
- **Client**
  - Message context menu shows "Delete message" for moderators on non-owned messages.
  - Moderator delete requires reason before submit.
  - Success toast copy is exactly `"Message deleted"`.
  - Existing own-message delete flow continues working.
  - Blocked-content invisibility remains correct after deletion/history refresh.

### Previous Story Intelligence

- Story 8.5 established the moderation log query model/UI and explicitly included `message_delete` action type; this story should feed that pipeline rather than inventing new audit storage.
- Story 8.5 validated `VIEW_MOD_LOG` gating and append-only expectations; do not introduce mutation of historical moderation entries.
- Existing moderation flows (8.1-8.4) standardized required reason input and explicit success/error messaging; message deletion should follow the same UX and validation standards.

### Git Intelligence Summary

- Recent implementation sequence in this area:
  - `3210f9c` feat: complete story 8-5 moderation log
  - `90a3b7f` feat: complete story 8-4 voice kick moderation flow
  - `15e3cc8` feat: complete story 8.3 ban moderation flow
  - `ee59039` fix: harden kick transaction review
  - `a272645` feat: implement story 8.2 kick moderation flow
- Practical implication: implement 8.6 as an extension of the existing moderation service/handler/model architecture and established tests, not as a parallel subsystem.

### Latest Technical Information

- Axum guidance continues to favor extractor-based handlers and Tower middleware composition, matching current handler/service patterns.  
  [Source: https://docs.rs/axum/latest/axum/]
- SQLx runtime/tls feature configuration remains important; current project config (`runtime-tokio`, rustls tls) should be preserved for compatibility and connection behavior consistency.  
  [Source: https://docs.rs/sqlx/latest/sqlx/, server/Cargo.toml]
- Svelte documentation continues to emphasize compiled component patterns and lean runtime output; story UI changes should stay within existing Svelte component/state conventions.  
  [Source: https://svelte.dev/docs/svelte/overview]

### Project Context Reference

- No `project-context.md` file found via `**/project-context.md`.
- Context derived from planning artifacts, existing Epic 8 implementation artifacts, and current codebase state.

### Story Completion Status

- Story implementation completed at `_bmad-output/implementation-artifacts/8-6-message-deletion-by-moderators.md`.
- Story status set to `review`.
- Sprint status target for this story: `review`.
- Completion note: Moderator message-delete backend/frontend flow, soft-delete persistence, moderation-log integration, and regression coverage were implemented and validated with full quality gates.

### Project Structure Notes

- Message-level moderation action belongs in chat/message surfaces plus existing moderation backend modules.
- Keep moderation control points consolidated in `services/moderation_service.rs` + `handlers/moderation.rs`.
- Reuse existing websocket event contract (`message_delete`) for real-time timeline consistency.

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Epic 8: Moderation, Reporting & Data Privacy]
- [Source: _bmad-output/planning-artifacts/epics.md#Story 8.6: Message Deletion by Moderators]
- [Source: _bmad-output/planning-artifacts/prd.md#Moderation & Safety]
- [Source: _bmad-output/planning-artifacts/prd.md#FR50]
- [Source: _bmad-output/planning-artifacts/architecture.md#Cross-Cutting Concerns Identified]
- [Source: _bmad-output/planning-artifacts/architecture.md#Requirements to Structure Mapping]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Journey 4: Moderation Workflow (Rico)]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Message context menu]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Context menu rules]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#MessageBubble]
- [Source: _bmad-output/implementation-artifacts/8-5-moderation-log.md]
- [Source: _bmad-output/implementation-artifacts/sprint-status.yaml]
- [Source: server/src/permissions/mod.rs]
- [Source: server/src/models/message.rs]
- [Source: server/src/models/moderation.rs]
- [Source: server/src/services/message_service.rs]
- [Source: server/src/services/moderation_service.rs]
- [Source: server/src/handlers/moderation.rs]
- [Source: server/src/handlers/mod.rs]
- [Source: server/src/ws/gateway.rs]
- [Source: server/tests/server_binds_to_configured_port.rs]
- [Source: client/src/lib/features/chat/MessageArea.svelte]
- [Source: client/src/lib/features/chat/MessageBubble.svelte]
- [Source: client/src/lib/features/chat/messageStore.svelte.ts]
- [Source: client/src/lib/features/guild/permissions.ts]
- [Source: client/src/lib/features/members/MemberList.svelte]
- [Source: client/src/lib/features/moderation/moderationApi.ts]
- [Source: https://docs.rs/axum/latest/axum/]
- [Source: https://docs.rs/sqlx/latest/sqlx/]
- [Source: https://svelte.dev/docs/svelte/overview]

## Dev Agent Record

### Agent Model Used

GPT-5.3-Codex (model ID: gpt-5.3-codex)

### Debug Log References

- Full quality gates executed successfully:
  - `cd client && npm run lint && npm run check && npm run test -- --run && npm run build`
  - `cd server && cargo fmt --check && cargo clippy -- -D warnings && cargo test`
- Stabilized moderation handler test fixture permissions to avoid cross-test permission-cache conflicts during parallel test execution.

### Completion Notes List

- Added migration `0031_add_moderated_message_soft_delete_fields.sql` with moderated-delete metadata and history-oriented indexes.
- Added moderated delete service/handler flow requiring `MANAGE_MESSAGES` + reason validation, persisting a moderation log action and soft-deleting the message atomically.
- Updated message reads/history/attachment paths to hide soft-deleted messages while preserving owner websocket hard-delete behavior.
- Implemented moderator delete UX (reason-required dialog and API path) with exact success toast `"Message deleted"`.
- Added server/client regression coverage for permission checks, API mapping, websocket delete propagation, history visibility, and attachment 404 behavior after moderated deletes.

### File List

- server/migrations/0031_add_moderated_message_soft_delete_fields.sql
- server/src/models/message.rs
- server/src/models/moderation.rs
- server/src/services/moderation_service.rs
- server/src/handlers/moderation.rs
- server/src/handlers/mod.rs
- server/tests/server_binds_to_configured_port.rs
- client/src/lib/features/moderation/moderationApi.ts
- client/src/lib/features/moderation/moderationApi.test.ts
- client/src/lib/features/chat/MessageBubble.svelte
- client/src/lib/features/chat/MessageBubble.test.ts
- client/src/lib/features/chat/MessageArea.svelte
- client/src/lib/features/chat/MessageArea.test.ts
- _bmad-output/implementation-artifacts/sprint-status.yaml
- _bmad-output/implementation-artifacts/8-6-message-deletion-by-moderators.md

## Senior Developer Review (AI)

- Outcome: **Approved**
- Findings addressed:
  - **MEDIUM (fixed):** `services::moderation_service::tests::create_message_delete_soft_deletes_message_and_records_audit_action` was nondeterministic under shared permission-cache state and could fail with `Missing MANAGE_MESSAGES permission in this guild`. Updated the test actor to `owner-user-id` so it validates soft-delete/audit persistence without cache-order coupling.
  - **LOW (fixed):** Message delete affordance in `client/src/lib/features/chat/MessageBubble.svelte` now explicitly excludes system messages to keep UI capability checks aligned with delete handler guards.
- Validation rerun:
  - `cd client && npm run lint && npm run check && npm run test -- --run src/lib/features/chat/MessageArea.test.ts src/lib/features/chat/MessageBubble.test.ts`
  - `cd client && npm run lint && npm run check && npm run test -- --run && npm run build`
  - `cd server && cargo fmt --check && cargo clippy -- -D warnings && cargo test`

## Change Log

- 2026-03-02: Senior code review completed, one medium test-stability issue fixed in `server/src/services/moderation_service.rs`, full quality gates rerun, story moved to `done`.
- 2026-03-02: Follow-up review tightened system-message delete affordance guard in `client/src/lib/features/chat/MessageBubble.svelte` to keep UI capability checks aligned with delete handler guards.
