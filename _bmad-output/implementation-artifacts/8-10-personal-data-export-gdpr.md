# Story 8.10: Personal Data Export (GDPR)

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a **user**,
I want to export all my personal data from an instance,
so that I have a copy of my information as required by data protection regulations.

## Acceptance Criteria

1. **Given** a user is authenticated  
   **When** they navigate to User Settings -> Privacy -> Export My Data  
   **Then** they can request a data export.

2. **Given** a user requests a data export  
   **When** export generation completes  
   **Then** the export is generated as a JSON file containing: profile data (username, avatar, email if associated), guild memberships, messages sent, DMs, reactions, files uploaded, and block list.

3. **Given** export generation succeeds  
   **When** the user remains in the SPA  
   **Then** the export is available for download within the SPA.

4. **Given** export generation takes more than 2 seconds  
   **When** the request is still in progress  
   **Then** progress is shown.

5. **Given** export generation completes successfully  
   **When** the file is ready  
   **Then** a toast confirms: **"Your data export is ready for download"**.

6. **Given** export data is assembled  
   **When** response payload is produced  
   **Then** it does not include other users' data or messages.

## Tasks / Subtasks

- [x] Task 1: Add personal data export API surface under existing users route group (AC: 1, 3)
  - [x] Add handler in `server/src/handlers/users.rs` for `POST /api/v1/users/me/data-export`.
  - [x] Wire route in `server/src/handlers/mod.rs` alongside existing `/users/me/*` endpoints.
  - [x] Preserve API contract conventions: `snake_case` fields and `{ "data": ... }` response envelope.

- [x] Task 2: Implement export orchestration service (AC: 2, 6)
  - [x] Add `server/src/services/user_data_export_service.rs` (or equivalent existing service extension) to build one export snapshot from authenticated `user_id`.
  - [x] Keep handler thin: validation/auth in handler, orchestration in service, SQL in models.
  - [x] Return deterministic, typed export payload with explicit sections: `profile`, `guild_memberships`, `messages`, `dm_messages`, `reactions`, `uploaded_files`, `block_list`, `exported_at`.

- [x] Task 3: Add/extend model helpers for export data gathering (AC: 2, 6)
  - [x] Extend `server/src/models/guild_member.rs` with user-centric membership listing helper.
  - [x] Reuse/extend `server/src/models/message.rs` user-author history helpers for full authored message export.
  - [x] Extend `server/src/models/dm_message.rs` with author-centric DM export query.
  - [x] Extend `server/src/models/message_reaction.rs` with user-centric reaction export query.
  - [x] Extend `server/src/models/message_attachment.rs` with helper to list attachments tied to messages authored by the user.
  - [x] Reuse `server/src/models/user_block.rs` owner block list helper and include only minimal block fields needed for personal export.
  - [x] Reuse recovery-email data (`user_recovery_email`) so email is included only when associated.

- [x] Task 4: Enforce strict privacy filtering during export assembly (AC: 2, 6)
  - [x] Ensure every exported list is filtered by authenticated `user_id` before serialization.
  - [x] Do not include other users' message content (including inbound DM messages not authored by requester).
  - [x] Avoid embedding unrelated profile snapshots for blocked users; keep block-list representation minimal.
  - [x] Add explicit tests for negative cases where other-user data exists in DB but must be excluded.

- [x] Task 5: Add client API contract and settings UX for export request/download (AC: 1, 3, 4, 5)
  - [x] Extend `client/src/lib/features/identity/types.ts` and `identityApi.ts` with typed personal-export request/response models.
  - [x] Add an export action to `client/src/lib/features/identity/ProfileSettingsView.svelte` in a privacy-focused section under settings.
  - [x] Implement in-SPA download via `Blob` + `URL.createObjectURL()` and clean up with `URL.revokeObjectURL()`.
  - [x] Add >2s progress behavior (indeterminate progress UI is acceptable) and disable duplicate submissions while pending.
  - [x] Emit exact success toast text: **"Your data export is ready for download"**.

- [x] Task 6: Add regression tests and run project quality gates (AC: all)
  - [x] Server handler/service tests in `server/src/handlers/users.rs` (and/or service test module) for success shape, auth gating, and privacy filtering.
  - [x] Client API tests in `client/src/lib/features/identity/identityApi.test.ts` for endpoint path, wire format, and mapping.
  - [x] UI tests in `client/src/lib/features/identity/ProfileSettingsView.test.ts` for progress state (>2s), download trigger, and success/error messaging.
  - [x] Run quality gates:
    - [x] `cd server && cargo fmt --check && cargo clippy -- -D warnings && cargo test` *(1 existing unrelated moderation test failure remains in baseline: `create_voice_kick_records_audit_action_without_removing_membership`)*
    - [x] `cd client && npm run lint && npm run check`

## Dev Notes

### Developer Context

- Story 8.10 sits after Story 8.9 (`_bmad-output/implementation-artifacts/8-9-report-queue-and-lifecycle.md`) and should keep the same engineering discipline: strict layering, explicit validation/error behavior, and complete test coverage.
- This story is user-privacy focused (FR63) and should be implemented in user/profile pathways, not moderation pathways.
- Current user settings surface is `ProfileSettingsView` and already hosts recovery email + block list controls, making it the correct UX location for **Export My Data**.

### Technical Requirements

- Endpoint should require authenticated session (`AuthenticatedUser`) and operate on `user.user_id` only.
- Export payload must contain the required sections from AC2:
  - Profile data (username, avatar fields, associated email if present)
  - Guild memberships
  - Messages sent
  - DMs (user-owned authored DM records only)
  - Reactions (user-owned)
  - Files uploaded (attachments belonging to user-authored messages)
  - Block list
- Preserve response envelope contract: `{"data": { ...export payload... }}`.
- Download should happen inside SPA, without navigating away or requiring separate admin tooling.
- Progress UX requirement is time-based (>2 seconds). Implement client-side timer-triggered progress state while request remains pending.

### Architecture Compliance

1. Keep boundaries: handlers parse/request-shape -> services orchestrate -> models own SQL.
2. Keep API naming/shape conventions (`snake_case` wire keys, `{"data": ...}` envelope, explicit errors).
3. Keep PostgreSQL + SQLite parity for every new/changed query.
4. Avoid broad catch-and-ignore behavior; surface explicit actionable failures.
5. Treat privacy filtering as a hard requirement, not best-effort.

### Library & Framework Requirements

- Backend: Axum response patterns (`IntoResponse`) for status/body/headers.
- Persistence: SQLx queries with existing runtime/TLS feature discipline; no runtime-feature mismatches.
- Serialization: `serde`/`serde_json` typed payload structures for deterministic JSON export shape.
- Frontend: Svelte 5 settings view patterns and existing identity API/store layering.
- Browser download mechanism: `Blob` + `URL.createObjectURL()` + `URL.revokeObjectURL()` cleanup.

### File Structure Requirements

Expected primary touch points:

- `server/src/handlers/mod.rs`
- `server/src/handlers/users.rs`
- `server/src/services/mod.rs`
- `server/src/services/user_data_export_service.rs` (new)
- `server/src/models/guild_member.rs`
- `server/src/models/message.rs`
- `server/src/models/dm_message.rs`
- `server/src/models/message_reaction.rs`
- `server/src/models/message_attachment.rs`
- `server/src/models/user_block.rs`
- `client/src/lib/features/identity/types.ts`
- `client/src/lib/features/identity/identityApi.ts`
- `client/src/lib/features/identity/identityApi.test.ts`
- `client/src/lib/features/identity/ProfileSettingsView.svelte`
- `client/src/lib/features/identity/ProfileSettingsView.test.ts`

### Testing Requirements

- **Server**
  - Auth required for export endpoint (unauthenticated requests are rejected).
  - Response contains all required sections and envelope contract.
  - Privacy filter tests prove other-user messages/data are excluded.
  - Optional associated email appears only when present.
- **Client**
  - API call path and payload mapping for export endpoint.
  - Settings export action shows progress when request exceeds 2 seconds.
  - Success toast exact text matches AC.
  - Download flow triggers object URL creation and cleanup.

### Previous Story Intelligence

- Story 8.9 reinforced guardrails that still apply: no duplicate business logic, explicit transitions/validation, and high-confidence regression tests before completion.
- Recent moderation stories heavily touched moderation modules; keep this story scoped to user/profile/export surfaces to avoid unnecessary coupling.

### Git Intelligence Summary

- `c982e01` feat: complete story 8-9 report queue lifecycle
- `8793165` feat: complete story 8-8 user content reporting
- `58f294f` feat: complete story 8-7 moderator history
- `97a7516` feat: complete story 8-6 moderator message deletion
- `3210f9c` feat: complete story 8-5 moderation log

### Latest Technical Information

- Axum response docs confirm `IntoResponse` tuple patterns for status + headers + body, useful for clean download responses.
- SQLx docs emphasize explicit runtime/TLS feature correctness and note async APIs panic without runtime features.
- Serde JSON docs emphasize typed conversion/serialization paths for robust JSON payload generation.
- MDN guidance for `URL.createObjectURL()` confirms Blob URL flow and need to revoke object URLs after use.

### Project Context Reference

- No `project-context.md` was found via `**/project-context.md`.
- Context derived from planning artifacts, implementation artifacts, schema/migrations, and current user/profile code paths.

### Story Completion Status

- Story context file: `_bmad-output/implementation-artifacts/8-10-personal-data-export-gdpr.md`
- Status set to: `ready-for-dev`
- Sprint status target: `ready-for-dev`
- Completion note: Ultimate context engine analysis completed - comprehensive developer guide created.

### Project Structure Notes

- Keep user-facing export flows within identity/user modules (`handlers/users.rs`, identity API, profile settings view).
- Do not route personal export through admin backup flow (`handlers/admin.rs`); that endpoint is instance-operator scope, not end-user personal-data scope.
- Preserve existing UX composition by adding a privacy/export section to `ProfileSettingsView` rather than introducing a separate settings route.

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Epic 8: Moderation, Reporting & Data Privacy]
- [Source: _bmad-output/planning-artifacts/epics.md#Story 8.10: Personal Data Export (GDPR)]
- [Source: _bmad-output/planning-artifacts/epics.md#Story 8.11: Account Deletion (GDPR)]
- [Source: _bmad-output/planning-artifacts/prd.md#Compliance & Regulatory]
- [Source: _bmad-output/planning-artifacts/prd.md#Data & Privacy]
- [Source: _bmad-output/planning-artifacts/architecture.md#API Naming Conventions]
- [Source: _bmad-output/planning-artifacts/architecture.md#REST API Response Format]
- [Source: _bmad-output/planning-artifacts/architecture.md#Process Patterns]
- [Source: _bmad-output/planning-artifacts/architecture.md#FR Category: Data & Privacy (FR63-66)]
- [Source: _bmad-output/implementation-artifacts/sprint-status.yaml]
- [Source: _bmad-output/implementation-artifacts/8-9-report-queue-and-lifecycle.md]
- [Source: server/src/handlers/mod.rs]
- [Source: server/src/handlers/users.rs]
- [Source: server/src/services/user_profile_service.rs]
- [Source: server/src/models/guild_member.rs]
- [Source: server/src/models/message.rs]
- [Source: server/src/models/dm_channel.rs]
- [Source: server/src/models/dm_message.rs]
- [Source: server/src/models/message_reaction.rs]
- [Source: server/src/models/message_attachment.rs]
- [Source: server/src/models/user_block.rs]
- [Source: server/migrations/0003_create_users.sql]
- [Source: server/migrations/0005_user_profile_avatar_metadata.sql]
- [Source: server/migrations/0006_recovery_email_association.sql]
- [Source: server/migrations/0014_create_guild_members.sql]
- [Source: server/migrations/0018_create_messages.sql]
- [Source: server/migrations/0020_create_message_reactions.sql]
- [Source: server/migrations/0021_create_message_attachments.sql]
- [Source: server/migrations/0023_create_dm_channels.sql]
- [Source: server/migrations/0024_create_dm_messages.sql]
- [Source: server/migrations/0025_create_user_blocks.sql]
- [Source: client/src/lib/features/identity/identityApi.ts]
- [Source: client/src/lib/features/identity/identityStore.svelte.ts]
- [Source: client/src/lib/features/identity/ProfileSettingsView.svelte]
- [Source: client/src/lib/features/identity/ProfileSettingsView.test.ts]
- [Source: https://docs.rs/axum/latest/axum/response/index.html]
- [Source: https://docs.rs/sqlx/latest/sqlx/]
- [Source: https://docs.rs/serde_json/latest/serde_json/]
- [Source: https://developer.mozilla.org/en-US/docs/Web/API/URL/createObjectURL_static]

## Dev Agent Record

### Agent Model Used

GPT-5.3-Codex (model ID: gpt-5.3-codex)

### Debug Log References

- `cd server && cargo test --lib create_data_export_returns_enveloped_sections`
- `cd server && cargo test --lib build_user_data_export_filters_to_requester_and_includes_required_sections`
- `cd server && cargo test --lib build_user_data_export_omits_email_when_not_associated`
- `cd server && cargo test --lib sqlite_list_messages_by_author_user_id_filters_to_author`
- `cd server && cargo test --lib sqlite_list_message_reactions_by_user_id_filters_to_requester`
- `cd server && cargo test --test server_binds_to_configured_port users_data_export_requires_authentication`
- `cd client && npm run test -- src/lib/features/identity/identityApi.test.ts src/lib/features/identity/ProfileSettingsView.test.ts`
- `cd client && npm run lint && npm run check`
- `cd server && cargo fmt --check && cargo clippy -- -D warnings && cargo test` *(fails on one pre-existing moderation test)*
- `cd server && cargo test --lib sqlite_list_message_reactions_by_user_id_filters_to_requester && cargo test --lib sqlite_list_message_reactions_by_user_id_excludes_soft_deleted_messages && cargo test --lib build_user_data_export_filters_to_requester_and_includes_required_sections`

### Completion Notes List

- Added authenticated `POST /api/v1/users/me/data-export` endpoint returning `{ "data": ... }` with typed export sections.
- Implemented `user_data_export_service` orchestration and user-scoped model helpers for memberships, messages, DMs, reactions, attachments, and optional associated recovery email.
- Enforced privacy filtering by querying only requester-owned rows and exporting a minimal block-list representation (`blocked_user_id`, `blocked_at`, `unblocked_at`).
- Added backend tests for response shape, privacy filtering, optional email behavior, and route-level auth gating.
- Added client API/types support plus profile settings privacy section with in-SPA JSON download, >2s progress state, duplicate-submit guard, and exact success toast copy.
- Added client tests for API mapping, slow-progress UX, download trigger + URL cleanup, and success toast text.
- Senior review fix: tightened reaction export query to exclude reactions tied to soft-deleted messages and added regression coverage.

### File List

- `_bmad-output/implementation-artifacts/8-10-personal-data-export-gdpr.md`
- `_bmad-output/implementation-artifacts/sprint-status.yaml`
- `server/src/handlers/mod.rs`
- `server/src/handlers/users.rs`
- `server/src/models/dm_message.rs`
- `server/src/models/guild_member.rs`
- `server/src/models/message.rs`
- `server/src/models/message_attachment.rs`
- `server/src/models/message_reaction.rs`
- `server/src/models/recovery_email.rs`
- `server/src/models/user.rs`
- `server/src/services/mod.rs`
- `server/src/services/user_data_export_service.rs`
- `server/tests/server_binds_to_configured_port.rs`
- `client/src/lib/features/identity/ProfileSettingsView.svelte`
- `client/src/lib/features/identity/ProfileSettingsView.test.ts`
- `client/src/lib/features/identity/identityApi.test.ts`
- `client/src/lib/features/identity/identityApi.ts`
- `client/src/lib/features/identity/identityStore.svelte.ts`
- `client/src/lib/features/identity/types.ts`

### Change Log

- 2026-03-02: Implemented Story 8.10 personal data export endpoint, service/model data assembly, client export UX/download flow, and regression coverage.
- 2026-03-02: Senior review fix — filtered exported reactions to exclude soft-deleted messages and added regression test coverage.

### Senior Developer Review (AI)

- Outcome: Approved after one medium-severity privacy hardening fix.
- Fixed finding:
  - **Medium**: reaction export included reactions referencing soft-deleted messages, which could leak moderated/deleted message IDs in personal exports.
  - **Fix**: updated `list_message_reactions_by_user_id` to join `messages` and require `m.deleted_at IS NULL` (`server/src/models/message_reaction.rs`), plus added `sqlite_list_message_reactions_by_user_id_excludes_soft_deleted_messages`.
- Validation:
  - `cd server && cargo test --lib sqlite_list_message_reactions_by_user_id_filters_to_requester`
  - `cd server && cargo test --lib sqlite_list_message_reactions_by_user_id_excludes_soft_deleted_messages`
  - `cd server && cargo test --lib build_user_data_export_filters_to_requester_and_includes_required_sections`
