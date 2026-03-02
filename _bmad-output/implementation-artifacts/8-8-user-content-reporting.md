# Story 8.8: User Content Reporting

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a **user**,
I want to report messages, files, or users to guild moderators,
so that I can flag harmful content for review.

## Acceptance Criteria

1. **Given** a user sees problematic content  
   **When** they right-click a message -> Report (or right-click a user -> Report)  
   **Then** a report dialog appears with a required reason field and optional category (`spam`, `harassment`, `rule_violation`, `other`).

2. **Given** a report is submitted  
   **When** the backend persists it  
   **Then** a report is created (new `reports` table migration in this story) with: `reporter_id`, target (`message`/`user`), reason, category, `status=pending`, and `created_at`.

3. **Given** a report is successfully created  
   **When** the client receives the response  
   **Then** a toast confirms: `"Report submitted. A moderator will review it."`.

4. **Given** a user attempts to report their own content  
   **When** the report is submitted  
   **Then** the request is rejected and users cannot report their own messages or user identity.

5. **Given** a user reports a target they already reported  
   **When** they submit another report for the same target in the same guild  
   **Then** the system prevents duplicate reports from the same reporter for that target.

## Tasks / Subtasks

- [x] Task 1: Add report persistence schema and constraints (AC: 2, 5)
  - [x] Add migration `server/migrations/0032_create_reports_table.sql` for report storage with PostgreSQL/SQLite parity.
  - [x] Include required fields: `id`, `guild_id`, `reporter_user_id`, `target_type`, target reference (`target_message_id` or `target_user_id`), `reason`, `category`, `status`, `created_at`, `updated_at`.
  - [x] Add constraints for valid target-type/target-id pairing and allowed status/category values.
  - [x] Add duplicate-prevention uniqueness for `(guild_id, reporter_user_id, target_type, target_message_id, target_user_id)`.
  - [x] Add queue-friendly indexes for future Story 8.9 reads (for example `guild_id + status + created_at`).

- [x] Task 2: Extend moderation model/service for report creation flow (AC: 2, 4, 5)
  - [x] Add report DB structs and insert/list helpers in `server/src/models/moderation.rs` (or a focused companion model if extracted, while keeping conventions aligned).
  - [x] Add typed input/output models in `server/src/services/moderation_service.rs` for user report creation.
  - [x] Require authenticated guild membership for reporter; reject non-members.
  - [x] Validate target existence and guild scope before insert (message belongs to guild; user is guild member when reporting user).
  - [x] Enforce self-report prevention for both message and user targets.
  - [x] Normalize/validate reason and category with explicit `AppError::ValidationError`/`Conflict` behavior.

- [x] Task 3: Expose report-create HTTP API endpoints and route wiring (AC: 1, 2, 4, 5)
  - [x] Add handlers in `server/src/handlers/moderation.rs` for:
    - [x] `POST /api/v1/guilds/{guild_slug}/moderation/reports/messages/{message_id}`
    - [x] `POST /api/v1/guilds/{guild_slug}/moderation/reports/users/{target_user_id}`
  - [x] Add request body parsing for required `reason` and optional `category`.
  - [x] Return API envelopes and naming consistent with project conventions (`snake_case`, `{ "data": ... }`).
  - [x] Map duplicates to explicit conflict responses (no silent success or duplicate inserts).
  - [x] Register routes in `server/src/handlers/mod.rs`.

- [x] Task 4: Add client moderation API support for report submission (AC: 1, 2, 3, 5)
  - [x] Extend `client/src/lib/features/moderation/moderationApi.ts` with report input/output types and methods:
    - [x] `createMessageReport(...)`
    - [x] `createUserReport(...)`
  - [x] Keep API-client validation patterns aligned with existing moderation methods (trim/required checks, `ApiError` semantics).
  - [x] Add tests in `client/src/lib/features/moderation/moderationApi.test.ts` for path mapping, payload mapping, and error handling.

- [x] Task 5: Implement message report UX from message context menu (AC: 1, 3, 4)
  - [x] Add a `Report` menu item in `client/src/lib/features/chat/MessageBubble.svelte` for non-own, non-system messages.
  - [x] Extend `MessageBubble` callback contract with `onReportRequest` to delegate orchestration to `MessageArea`.
  - [x] Add report dialog flow in `client/src/lib/features/chat/MessageArea.svelte` with:
    - [x] required reason field
    - [x] optional category selector
    - [x] disabled submit while in-flight
  - [x] Show success toast copy exactly: `"Report submitted. A moderator will review it."`.
  - [x] Ensure own-message report entry is hidden or disabled with explicit guardrails.

- [x] Task 6: Implement user report UX from member context actions (AC: 1, 3, 4)
  - [x] Add `Report user` action in `client/src/lib/features/members/MemberList.svelte` for non-self users.
  - [x] Reuse the same report dialog component/pattern used for message reporting where practical.
  - [x] Ensure dialog and action remain keyboard-accessible and consistent with existing context-action patterns.
  - [x] Preserve existing block/mute/kick/ban/history actions and avoid behavior regressions.

- [x] Task 7: Preserve architecture conventions and queue-readiness for Story 8.9 (AC: 2, 5)
  - [x] Keep strict handler/service/model layering and no direct SQL in handlers/services.
  - [x] Store report status as `pending` on creation and keep schema compatible with lifecycle states (`reviewed`, `actioned`, `dismissed`) required by Story 8.9.
  - [x] Keep `mod_log` integration decisions explicit (this story creates reports; Story 8.9 links moderator actions/lifecycle).

- [x] Task 8: Add regression coverage and run quality gates (AC: all)
  - [x] Server tests (unit/service/handler) for:
    - [x] message-report and user-report happy paths
    - [x] own-content rejection
    - [x] duplicate prevention
    - [x] invalid category/reason/target validation
  - [x] Integration tests in `server/tests/server_binds_to_configured_port.rs` for end-to-end report creation and rejection paths.
  - [x] Client tests in:
    - [x] `client/src/lib/features/chat/MessageBubble.test.ts`
    - [x] `client/src/lib/features/chat/MessageArea.test.ts`
    - [x] `client/src/lib/features/members/MemberList.test.ts`
    - [x] `client/src/lib/features/moderation/moderationApi.test.ts`
  - [x] Run quality gates:
    - [x] `cd client && npm run lint && npm run check && npm run test && npm run build`
    - [x] `cd server && cargo fmt --check && cargo clippy -- -D warnings && cargo test`

## Dev Notes

### Developer Context

- Epic 8 moderation foundation is already implemented through stories 8.1-8.7 (mute/kick/ban/voice-kick/mod-log/message-delete/history). Story 8.8 should extend this existing stack rather than introducing parallel moderation/reporting paths.
- Current moderation infrastructure already has:
  - stable backend layering in `handlers/moderation.rs` -> `services/moderation_service.rs` -> `models/moderation.rs`
  - consistent API envelope and cursor/list patterns in moderation endpoints
  - moderation client API and test scaffolding in `client/src/lib/features/moderation/moderationApi.ts`.
- Current user-facing action surfaces that should host reporting:
  - message actions in `MessageBubble`/`MessageArea`
  - member actions in `MemberList`.
- Story 8.9 (next) depends on high-quality report records created here, including reliable status initialization (`pending`) and dedupe behavior.

### Technical Requirements

- Reporter must be authenticated and in guild scope for the report target.
- Targets for this story are `message` and `user`; schema should remain compatible with future `file` target support without breaking API or data model consistency.
- Reason is required, non-empty, and should reuse established moderation reason length guardrails (500 chars max).
- Category is optional and constrained to known values (`spam`, `harassment`, `rule_violation`, `other`) when present.
- Duplicate prevention must be enforced at persistence level (unique index/constraint), not only at API layer.
- Self-report prevention must be enforced server-side:
  - message target: reporter cannot report own authored message
  - user target: reporter cannot report self.
- Response and error behavior must stay explicit:
  - success with `{ "data": ... }`
  - validation errors (`400/422`)
  - forbidden (`403`)
  - duplicate conflict (`409`).

### Architecture Compliance

1. Keep layer boundaries strict: handlers parse/validate; services enforce business rules/permissions; models own SQL.
2. Preserve API contract standards: `snake_case` wire fields and response envelope.
3. Keep PostgreSQL/SQLite query and migration parity.
4. Reuse existing moderation permission/normalization helpers and avoid copy-paste validation logic.
5. Avoid silent failure modes: invalid targets/categories/duplicates must return explicit errors.
6. Follow existing loading and feedback patterns in Svelte flows (clear action states, predictable dialogs/toasts).

### Library & Framework Requirements

- Backend stack remains Rust + Axum + SQLx + Tokio; continue extractor-first handler style and Tower middleware integration.
- Frontend stack remains Svelte 5 + existing feature/store patterns; no new global state/data frameworks.
- Keep existing API utilities (`apiFetch`, `apiFetchCursorList`, `ApiError`) and existing moderation UI conventions.

### File Structure Requirements

Expected primary touch points:

- `server/migrations/0032_create_reports_table.sql` (new)
- `server/src/models/moderation.rs`
- `server/src/services/moderation_service.rs`
- `server/src/handlers/moderation.rs`
- `server/src/handlers/mod.rs`
- `server/tests/server_binds_to_configured_port.rs`
- `client/src/lib/features/moderation/moderationApi.ts`
- `client/src/lib/features/moderation/moderationApi.test.ts`
- `client/src/lib/features/chat/MessageBubble.svelte`
- `client/src/lib/features/chat/MessageBubble.test.ts`
- `client/src/lib/features/chat/MessageArea.svelte`
- `client/src/lib/features/chat/MessageArea.test.ts`
- `client/src/lib/features/members/MemberList.svelte`
- `client/src/lib/features/members/MemberList.test.ts`
- `client/src/lib/features/moderation/` (new report dialog component if extracted for reuse)

### Testing Requirements

- **Server**
  - Report create endpoint accepts valid message/user targets and returns expected envelope.
  - Reporter cannot report own message or own user.
  - Duplicate reports by same reporter for same target are rejected.
  - Invalid/unknown target and invalid category are rejected with explicit errors.
  - Guild-scope checks prevent cross-guild target reporting.
- **Client**
  - Message context menu shows `Report` only for eligible messages (not own/system).
  - Member action surface shows `Report user` only for non-self members.
  - Report dialog enforces required reason and submits optional category.
  - Success toast copy exactly matches AC.
  - API methods build expected paths and payloads.
- **Integration**
  - End-to-end message report and user report creation succeed for eligible users.
  - Duplicate and self-report scenarios fail with expected status codes/messages.

### Previous Story Intelligence

- Story 8.7 introduced moderator message history and expanded moderation client/backend contracts; reuse these patterns for pagination, target validation style, and panel wiring.
- Story 8.6 established moderated message soft-delete semantics and `message_delete` moderation event handling; message-report target validation should treat deleted messages consistently (not reportable once effectively removed from normal visibility).
- Story 8.5 established moderation log/read patterns and added future-facing action types (`warn`), which indicates moderation workflows are designed to evolve via additive schema/service extensions.

### Git Intelligence Summary

- `58f294f` feat: complete story 8-7 moderator history
- `97a7516` feat: complete story 8-6 moderator message deletion
- `3210f9c` feat: complete story 8-5 moderation log
- `90a3b7f` feat: complete story 8-4 voice kick moderation flow
- `15e3cc8` feat: complete story 8.3 ban moderation flow

Recent commit history confirms stable implementation patterns in:
- `server/src/handlers/moderation.rs`
- `server/src/services/moderation_service.rs`
- `server/src/models/moderation.rs`
- `client/src/lib/features/moderation/moderationApi.ts`
- member/chat feature surfaces for moderation actions.

### Latest Technical Information

- Axum continues to emphasize extractor-driven handlers and Tower middleware composition; this aligns with existing moderation handler architecture and should remain the baseline for new report endpoints.  
  [Source: https://docs.rs/axum/latest/axum/]
- SQLx runtime/TLS feature configuration remains explicit; keep existing runtime/TLS strategy and DB parity conventions for new report queries and migrations.  
  [Source: https://docs.rs/sqlx/latest/sqlx/]
- Svelte remains compile-time optimized and component-first; reporting UI should be implemented as focused, typed components/stores consistent with existing chat/member/moderation modules.  
  [Source: https://svelte.dev/docs/svelte/overview]

### Project Context Reference

- No `project-context.md` file was found via `**/project-context.md`.
- Context was derived from planning artifacts, sprint status, existing Epic 8 implementation artifacts, current repository code, and recent git history.

### Story Completion Status

- Story context created at `_bmad-output/implementation-artifacts/8-8-user-content-reporting.md`.
- Story status is `ready-for-dev`.
- Sprint status target for this story is `ready-for-dev`.
- Completion note: Ultimate context engine analysis completed - comprehensive developer guide created.

### Project Structure Notes

- Keep report submission logic in existing moderation modules, not in ad-hoc feature-specific server stacks.
- Keep message report entry at message context menu level (`MessageBubble` + `MessageArea`), and user report entry in existing member action surface (`MemberList`).
- Preserve UX consistency with existing moderation dialogs (required reason, clear confirmation, explicit error text, keyboard accessibility).
- Keep schema and API future-ready for Story 8.9 queue/lifecycle without prematurely implementing moderator queue UI in this story.

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Epic 8: Moderation, Reporting & Data Privacy]
- [Source: _bmad-output/planning-artifacts/epics.md#Story 8.8: User Content Reporting]
- [Source: _bmad-output/planning-artifacts/epics.md#Story 8.9: Report Queue and Lifecycle]
- [Source: _bmad-output/planning-artifacts/prd.md#UGC Reporting & Content Moderation]
- [Source: _bmad-output/planning-artifacts/prd.md#Moderation & Safety]
- [Source: _bmad-output/planning-artifacts/prd.md#FR53]
- [Source: _bmad-output/planning-artifacts/prd.md#FR54]
- [Source: _bmad-output/planning-artifacts/prd.md#FR55]
- [Source: _bmad-output/planning-artifacts/architecture.md#Cross-Cutting Concerns Identified]
- [Source: _bmad-output/planning-artifacts/architecture.md#API & Communication Patterns]
- [Source: _bmad-output/planning-artifacts/architecture.md#Format Patterns]
- [Source: _bmad-output/planning-artifacts/architecture.md#Process Patterns]
- [Source: _bmad-output/planning-artifacts/architecture.md#Structure Patterns]
- [Source: _bmad-output/planning-artifacts/architecture.md#Requirements to Structure Mapping]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Journey 4: Moderation Workflow (Rico)]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#ReportQueueItem]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Context Menu Patterns]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Loading Patterns]
- [Source: _bmad-output/implementation-artifacts/8-5-moderation-log.md]
- [Source: _bmad-output/implementation-artifacts/8-6-message-deletion-by-moderators.md]
- [Source: _bmad-output/implementation-artifacts/8-7-view-user-message-history-moderator.md]
- [Source: _bmad-output/implementation-artifacts/sprint-status.yaml]
- [Source: server/src/handlers/mod.rs]
- [Source: server/src/handlers/moderation.rs]
- [Source: server/src/services/moderation_service.rs]
- [Source: server/src/models/moderation.rs]
- [Source: server/migrations/0026_create_moderation_actions.sql]
- [Source: server/migrations/0030_expand_moderation_actions_for_log_view.sql]
- [Source: client/src/lib/features/moderation/moderationApi.ts]
- [Source: client/src/lib/features/chat/MessageBubble.svelte]
- [Source: client/src/lib/features/chat/MessageArea.svelte]
- [Source: client/src/lib/features/members/MemberList.svelte]
- [Source: https://docs.rs/axum/latest/axum/]
- [Source: https://docs.rs/sqlx/latest/sqlx/]
- [Source: https://svelte.dev/docs/svelte/overview]

## Dev Agent Record

### Agent Model Used

GPT-5.3-Codex (model ID: gpt-5.3-codex)

### Debug Log References

- `cd client && npm run test -- src/lib/features/chat/MessageBubble.test.ts src/lib/features/chat/MessageArea.test.ts src/lib/features/members/MemberList.test.ts`
- `cd client && npm run lint && npm run check && npm run test && npm run build`
- `cd server && cargo fmt && cargo clippy -- -D warnings && cargo test`

### Completion Notes List

- Added `reports` persistence with target-type constraints, category/status validation, dedupe prevention, and queue-ready indexing.
- Implemented backend report-create flows (model/service/handlers/routes) for message and user targets with membership checks, self-report rejection, and conflict mapping.
- Implemented client reporting API helpers and reusable `ReportDialog` UX, and wired message/member reporting actions.
- Added regression coverage across backend and frontend reporting flows, then passed full client and server quality gates.

### File List

- `server/migrations/0032_create_reports_table.sql`
- `server/src/models/moderation.rs`
- `server/src/services/moderation_service.rs`
- `server/src/handlers/moderation.rs`
- `server/src/handlers/mod.rs`
- `server/tests/server_binds_to_configured_port.rs`
- `client/src/lib/features/moderation/moderationApi.ts`
- `client/src/lib/features/moderation/moderationApi.test.ts`
- `client/src/lib/features/moderation/ReportDialog.svelte`
- `client/src/lib/features/chat/MessageBubble.svelte`
- `client/src/lib/features/chat/MessageBubble.test.ts`
- `client/src/lib/features/chat/MessageArea.svelte`
- `client/src/lib/features/chat/MessageArea.test.ts`
- `client/src/lib/features/members/MemberList.svelte`
- `client/src/lib/features/members/MemberList.test.ts`
- `_bmad-output/implementation-artifacts/sprint-status.yaml`
- `_bmad-output/implementation-artifacts/8-8-user-content-reporting.md`

### Change Log

- Added Story 8.8 reporting schema, backend APIs, and validation/error behavior.
- Added frontend report API contracts and reusable report dialog component.
- Wired report actions into message and member surfaces with required reason and optional category.
- Added/updated tests for message reporting, user reporting, and API behavior.
- Ran and passed client (`lint`, `check`, `test`, `build`) and server (`fmt`, `clippy`, `test`) quality gates.
- Senior review fix: invalidated guild permission cache in moderation service test setup to prevent cross-test cache contamination (`list_bans_and_unban_round_trip` regression).

### Senior Developer Review (AI)

- Outcome: Approved after one medium-severity test reliability fix.
- Fixed finding:
  - **Medium**: global permission cache could leak stale guild permissions between moderation service tests sharing `guild-id`, causing ban-flow test failures.
  - **Fix**: added `permissions::invalidate_guild_permission_cache("guild-id")` in `setup_service_pool()` before fixture seeding (`server/src/services/moderation_service.rs`).
- Validation:
  - `cd server && cargo test services::moderation_service::tests::list_bans_and_unban_round_trip`
  - `cd server && cargo fmt --check && cargo clippy -- -D warnings && cargo test`
