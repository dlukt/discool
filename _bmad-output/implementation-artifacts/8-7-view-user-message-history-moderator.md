# Story 8.7: View User Message History (Moderator)

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a **moderator**,
I want to view a user's message history within the guild,
so that I can assess patterns of behavior before taking action.

## Acceptance Criteria

1. **Given** a user has `MANAGE_MESSAGES` or `KICK_MEMBERS` permission  
   **When** they right-click a member -> View message history  
   **Then** a panel opens showing that user's messages across all guild channels, sorted by recency.

2. **Given** a message-history panel has many entries  
   **When** moderators scroll through results  
   **Then** messages are paginated with virtual scrolling.

3. **Given** a message entry is shown in history  
   **When** the row is rendered  
   **Then** it includes channel, timestamp, and content.

4. **Given** a moderator needs full context for a historical message  
   **When** they click a message row in history  
   **Then** the app navigates to that message in its channel context.

5. **Given** a moderator needs to narrow the review scope  
   **When** they use history controls  
   **Then** results can be filtered by channel and date range.

## Tasks / Subtasks

- [x] Task 1: Add backend moderation API for user message history (AC: 1, 2, 3, 5)
  - [x] Add handler in `server/src/handlers/moderation.rs` for guild-scoped user history listing with `cursor`, `limit`, `channel_slug`, `from`, and `to` filters.
  - [x] Register route in `server/src/handlers/mod.rs` under existing moderation endpoint conventions.
  - [x] Keep response envelope and naming conventions: `{ "data": [...], "cursor": ... }` with `snake_case` fields.

- [x] Task 2: Extend message data/query layer for moderator history retrieval (AC: 2, 3, 5)
  - [x] Add model query support in `server/src/models/message.rs` for guild + author history with recency sort and cursor pagination.
  - [x] Ensure soft-deleted rows remain excluded (`deleted_at IS NULL`) and preserve PostgreSQL/SQLite parity.
  - [x] Reuse existing pagination and query-building patterns used by message history flows.

- [x] Task 3: Implement moderation service authorization and orchestration (AC: 1, 2, 3, 5)
  - [x] Add typed input/result models in `server/src/services/moderation_service.rs` for moderator message-history lookups.
  - [x] Enforce permission gate as `MANAGE_MESSAGES` **or** `KICK_MEMBERS` using existing permission helpers.
  - [x] Keep explicit validation/forbidden/not-found error behavior (`AppError` conventions), with no silent coercion.

- [x] Task 4: Implement client API and message-history panel UX (AC: 1, 2, 3, 5)
  - [x] Extend `client/src/lib/features/moderation/moderationApi.ts` (and tests) with fetch method + typed mapping for moderator user-history API.
  - [x] Add/update moderation panel component(s) under `client/src/lib/features/moderation/` for virtualized history list and filter controls.
  - [x] Follow existing loading rules: no loading UI <200ms, skeletons for 200ms-2s, skeleton + explanatory text when >2s.

- [x] Task 5: Wire member context action and jump-to-message flow (AC: 1, 4)
  - [x] Replace placeholder "Moderate messages (coming soon)" action in member context surfaces with real "View message history" flow.
  - [x] On history-row click, route to message channel context and trigger message-area history positioning logic.
  - [x] Preserve keyboard-accessible context-menu behavior and imperative action labels.

- [x] Task 6: Add regression coverage and run quality gates (AC: all)
  - [x] Server tests for permission allow/deny, cursor behavior, sorting/filtering, and soft-delete exclusion.
  - [x] Client tests for context action visibility, panel rendering, pagination/virtualization, filtering, and jump navigation trigger.
  - [x] Integration coverage for end-to-end moderator history access and 403 for unauthorized users.
  - [x] Run quality gates:
    - [x] `cd client && npm run lint && npm run check && npm run test && npm run build`
    - [x] `cd server && cargo fmt --check && cargo clippy -- -D warnings && cargo test`

## Dev Notes

### Developer Context

- Epic 8 moderation work (8.1-8.6) already established backend/service/model layering and moderation UI entry points; this story should extend those patterns.
- `8-6-message-deletion-by-moderators` introduced moderated message soft-delete behavior and related indexes that should be respected by history queries.
- Member list context surfaces still include a placeholder message-moderation action, which is the intended insertion point for this story's UX.
- This story is the FR49 bridge between real-time chat data and moderation decision workflows.

### Technical Requirements

- Permission requirement: only moderators with `MANAGE_MESSAGES` or `KICK_MEMBERS` can view user message history.
- History must be guild-scoped, recency-sorted, cursor-paginated, and virtualized in the client.
- Each row must include channel, timestamp, and message content.
- Filters must support both channel and date-range narrowing.
- Soft-deleted content must not reappear in moderator history responses.
- Click-through must navigate moderators into channel context for the selected message.

### Architecture Compliance

1. Keep strict layering: handlers parse/validate, services enforce auth + business rules, models own SQL.
2. Reuse existing permission engine and moderation helpers; do not introduce parallel authorization paths.
3. Preserve API contract conventions (`snake_case`, `{data}` envelope, cursor-based list shape).
4. Maintain PostgreSQL/SQLite parity for query semantics and tests.
5. Use explicit `AppError`-based error propagation (validation/forbidden/not-found).

### Library & Framework Requirements

- Backend stack remains Rust + Axum + SQLx + Tokio.
- Frontend stack remains Svelte 5 + Vite + existing store/query patterns.
- Continue using existing moderation/chat UI primitives and virtualized list approach.
- Do not introduce new state-management or data-fetching frameworks.

### File Structure Requirements

Expected primary touch points:

- `server/src/handlers/mod.rs`
- `server/src/handlers/moderation.rs`
- `server/src/services/moderation_service.rs`
- `server/src/models/message.rs`
- `server/tests/server_binds_to_configured_port.rs`
- `client/src/lib/features/moderation/moderationApi.ts`
- `client/src/lib/features/moderation/moderationApi.test.ts`
- `client/src/lib/features/members/MemberList.svelte`
- `client/src/lib/features/members/MemberList.test.ts`
- `client/src/lib/features/moderation/` (new or extended history panel components)
- `client/src/lib/features/chat/MessageArea.svelte` and/or `client/src/lib/features/chat/messageStore.svelte.ts` (jump-to-message wiring)

### Testing Requirements

- **Server**
  - Permission allow/deny for moderator history endpoint.
  - Cursor pagination correctness (no duplicates/skips across pages).
  - Recency ordering and filter correctness.
  - Soft-delete exclusion guarantees.
- **Client**
  - Moderator context action visibility + invocation.
  - Virtualized history panel behavior under pagination.
  - Channel/date filter behavior and empty/loading states.
  - Click-to-jump routing trigger for message context navigation.
- **Integration**
  - Authorized moderator can inspect user history end-to-end.
  - Unauthorized caller receives `403 Forbidden`.

### Previous Story Intelligence

- Story 8.6 established moderation message semantics and soft-delete persistence; history views must align with that visibility model.
- Story 8.6 used established moderation API/service conventions and explicit permission checks, which should be mirrored here.
- Story 8.5 delivered append-only moderation audit workflows; this story should provide the behavioral context moderators use before taking logged actions.

### Git Intelligence Summary

- `97a7516` feat: complete story 8-6 moderator message deletion
- `3210f9c` feat: complete story 8-5 moderation log
- `90a3b7f` feat: complete story 8-4 voice kick moderation flow
- `15e3cc8` feat: complete story 8.3 ban moderation flow
- `ee59039` fix: harden kick transaction review

### Latest Technical Information

- Axum continues to emphasize extractor-based request handling and Tower middleware composition, matching existing moderation handler conventions.  
  [Source: https://docs.rs/axum/latest/axum/]
- SQLx runtime and TLS feature selection remains critical (`runtime-tokio` and rustls/native TLS choices); preserve current project SQLx runtime/TLS setup.  
  [Source: https://docs.rs/sqlx/latest/sqlx/]
- Svelte continues to compile declarative components into optimized runtime output; stay within current Svelte component/store architecture for panel and navigation behavior.  
  [Source: https://svelte.dev/docs/svelte/overview]

### Project Context Reference

- No `project-context.md` file was found via `**/project-context.md`.
- Context was derived from planning artifacts, sprint status, existing Epic 8 implementation artifacts, and current git history.

### Story Completion Status

- Story context created at `_bmad-output/implementation-artifacts/8-7-view-user-message-history-moderator.md`.
- Story status is `done`.
- Sprint status target for this story is `done`.
- Completion note: Ultimate context engine analysis completed - comprehensive developer guide created.

### Project Structure Notes

- Keep moderation read workflow centered in existing moderation modules (`handlers/moderation.rs`, `services/moderation_service.rs`), with message query additions in `models/message.rs`.
- Keep UI entry point in existing member context menu patterns; avoid introducing separate navigation paths for this moderation function.
- Follow existing virtualized-list and loading-state standards already used in chat/member/moderation surfaces.

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Epic 8: Moderation, Reporting & Data Privacy]
- [Source: _bmad-output/planning-artifacts/epics.md#Story 8.7: View User Message History (Moderator)]
- [Source: _bmad-output/planning-artifacts/prd.md#Moderation & Safety]
- [Source: _bmad-output/planning-artifacts/prd.md#FR49]
- [Source: _bmad-output/planning-artifacts/architecture.md#Cross-Cutting Concerns Identified]
- [Source: _bmad-output/planning-artifacts/architecture.md#API & Communication Patterns]
- [Source: _bmad-output/planning-artifacts/architecture.md#Requirements to Structure Mapping]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Journey 4: Moderation Workflow (Rico)]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Context Menu Patterns]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Loading rules]
- [Source: _bmad-output/implementation-artifacts/8-6-message-deletion-by-moderators.md]
- [Source: _bmad-output/implementation-artifacts/sprint-status.yaml]
- [Source: https://docs.rs/axum/latest/axum/]
- [Source: https://docs.rs/sqlx/latest/sqlx/]
- [Source: https://svelte.dev/docs/svelte/overview]

## Dev Agent Record

### Agent Model Used

GPT-5.3-Codex (model ID: gpt-5.3-codex)

### Debug Log References

- `cd server && cargo test list_user_message_history -- --nocapture && cargo test sqlite_message_history_page_filters_and_skips_soft_deleted_rows -- --nocapture`
- `cd server && cargo test moderation_user_message_history_lists_entries_and_enforces_permissions -- --nocapture`
- `cd server && cargo clippy -- -D warnings && cargo test`
- `cd client && npm run test -- src/lib/features/moderation/moderationApi.test.ts src/lib/features/members/MemberList.test.ts src/lib/features/shell/ShellRoute.test.ts src/lib/features/chat/messageStore.test.ts`
- `cd client && npm run lint && npm run check && npm run test && npm run build`

### Completion Notes List

- Implemented guild-scoped moderator user-message-history endpoint with cursor pagination, channel/date filters, and `MANAGE_MESSAGES` or `KICK_MEMBERS` authorization.
- Added model/service/handler + route wiring with soft-delete exclusion and RFC3339/cursor validation.
- Replaced member-context placeholder with real "View message history" action and added virtualized history panel with filter controls.
- Wired history-row click to global intent, channel navigation, and in-channel jump/highlight behavior via message store pending-jump coordination.
- Added server unit/service/integration coverage and client API/store/member/shell coverage; full client/server quality gates are passing.

### File List

- `server/src/models/message.rs`
- `server/src/services/moderation_service.rs`
- `server/src/handlers/moderation.rs`
- `server/src/handlers/mod.rs`
- `server/tests/server_binds_to_configured_port.rs`
- `client/src/lib/features/moderation/moderationApi.ts`
- `client/src/lib/features/moderation/moderationApi.test.ts`
- `client/src/lib/features/moderation/UserMessageHistoryPanel.svelte`
- `client/src/lib/features/members/MemberList.svelte`
- `client/src/lib/features/members/MemberList.test.ts`
- `client/src/lib/features/chat/messageStore.svelte.ts`
- `client/src/lib/features/chat/messageStore.test.ts`
- `client/src/lib/features/chat/MessageArea.svelte`
- `client/src/lib/features/chat/MessageArea.test.ts`
- `client/src/lib/features/shell/ShellRoute.svelte`
- `client/src/lib/features/shell/ShellRoute.test.ts`

### Change Log

- Added moderator user-message-history API (`GET /api/v1/guilds/{guild_slug}/moderation/users/{target_user_id}/messages`).
- Implemented backend history query path (model/service/handler/route) including cursor + channel/date filters and OR permission gate.
- Added client moderation history panel, context action wiring, and jump-to-message flow into channel context.
- Extended tests across Rust and Vitest suites and validated with full quality gates.
- Senior code review (YOLO) completed with no blocking findings; story approved and status moved to `done`.

## Senior Developer Review (AI)

### Reviewer

Darko

### Date

2026-03-02

### Outcome

Approved

### Findings Summary

- High: 0
- Medium: 0
- Low: 0

### Notes

- Acceptance criteria and completed tasks were validated against implementation across `client/` and `server/`.
- Full quality gates passed:
  - `cd client && npm run lint && npm run check && npm run test && npm run build`
  - `cd server && cargo fmt --check && cargo clippy -- -D warnings && cargo test`
