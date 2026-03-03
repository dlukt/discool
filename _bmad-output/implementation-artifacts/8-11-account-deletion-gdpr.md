# Story 8.11: Account Deletion (GDPR)

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a **user**,
I want to permanently delete my account from an instance,
so that my personal data is removed as required by data protection regulations.

## Acceptance Criteria

1. **Given** a user is authenticated  
   **When** they navigate to User Settings -> Privacy -> Delete My Account  
   **Then** a confirmation dialog warns: **"This will permanently delete your identity and all associated data from this instance. This cannot be undone."**

2. **Given** the account deletion dialog is open  
   **When** the user has not typed their username exactly  
   **Then** the destructive confirmation action is disabled  
   **And** the user must type their username to confirm deletion.

3. **Given** account deletion is confirmed  
   **When** backend deletion executes successfully  
   **Then** the user record is deleted  
   **And** associated records are removed according to data policy: messages (or author anonymization policy), DMs, reactions, guild memberships, uploaded files, and session data.

4. **Given** account deletion touches multiple related tables  
   **When** any deletion step fails  
   **Then** no partial deletion is persisted  
   **And** the API returns a structured error response.

5. **Given** account deletion succeeds  
   **When** the response returns to the SPA  
   **Then** the user is logged out of the current instance immediately.

6. **Given** the user has a portable cryptographic identity in browser storage  
   **When** account deletion completes  
   **Then** identity material remains in browser storage so the user can still use that identity on other instances.

7. **Given** the confirmation UI includes destructive actions  
   **When** rendering the delete action  
   **Then** the delete button uses destructive styling (fire red) and is not the default action.

## Tasks / Subtasks

- [x] Task 1: Add authenticated account deletion API surface (AC: 1, 2, 5)
  - [x] Add `DELETE /api/v1/users/me` route in `server/src/handlers/mod.rs`.
  - [x] Add handler in `server/src/handlers/users.rs` with explicit payload validation for `confirm_username`.
  - [x] Keep response conventions: no silent failures, explicit `AppError` mapping, and consistent JSON error shape.

- [x] Task 2: Implement account deletion orchestration service (AC: 3, 4, 5, 6)
  - [x] Add `server/src/services/user_account_deletion_service.rs` and export from `server/src/services/mod.rs`.
  - [x] Implement transaction-scoped deletion flow (or equivalent atomic sequence) so failures roll back.
  - [x] Reuse existing models for user-related datasets (`user`, `guild_member`, `message`, `dm_message`, `message_reaction`, `message_attachment`, `user_block`, `recovery_email`, `session`).
  - [x] Ensure uploaded files are removed from file storage (`file_storage_service`) and not only from DB rows.
  - [x] Keep authenticated-session behavior deterministic: after deletion, the token must no longer authorize protected endpoints.

- [x] Task 3: Handle relational and ownership constraints explicitly (AC: 3, 4)
  - [x] Account for `guilds.owner_id` foreign key behavior before deleting a user row.
  - [x] Implement one explicit policy for owned guilds (no silent FK failures): either transfer ownership, remove owned guilds by policy, or return actionable `409 Conflict`.
  - [x] Document and test chosen ownership behavior so deletion never partially succeeds.

- [x] Task 4: Extend client identity API/store for deletion flow (AC: 5, 6)
  - [x] Add deletion request type and API function in `client/src/lib/features/identity/types.ts` and `identityApi.ts`.
  - [x] Add identity-store action to call deletion endpoint and clear only session/auth state, not portable identity keys.
  - [x] Ensure state cleanup reuses existing logout/session-clear patterns (`setSessionToken(null)`, storage cleanup for session, preserving identity).

- [x] Task 5: Add Privacy Danger Zone UI for deletion confirmation (AC: 1, 2, 7)
  - [x] Extend `client/src/lib/features/identity/ProfileSettingsView.svelte` Privacy section with a dedicated destructive block.
  - [x] Add confirmation dialog with exact warning copy and username-confirm input.
  - [x] Disable delete action until username matches current account username exactly.
  - [x] Use destructive visual token (`--destructive` / fire red), and keep cancel/non-destructive action as default focus path.
  - [x] Show clear success/error feedback through existing alert/toast patterns.

- [x] Task 6: Add regression tests and run quality gates (AC: all)
  - [x] Server unit tests in `server/src/handlers/users.rs` for validation and handler response behavior.
  - [x] Server integration tests in `server/tests/server_binds_to_configured_port.rs` for auth requirement, successful deletion, session invalidation, and ownership-policy behavior.
  - [x] Client API tests in `client/src/lib/features/identity/identityApi.test.ts` for endpoint/method/wire-format.
  - [x] Client UI tests in `client/src/lib/features/identity/ProfileSettingsView.test.ts` for confirmation dialog flow, username gate, destructive button state, and success/error behavior.
  - [x] Run project quality commands:
    - [x] `cd server && cargo fmt --check && cargo clippy -- -D warnings && cargo test`
    - [x] `cd client && npm run lint && npm run check`

## Dev Notes

### Developer Context

- Story 8.11 follows Story 8.10 (`_bmad-output/implementation-artifacts/8-10-personal-data-export-gdpr.md`) and should reuse the same user-settings privacy surface and identity API layering.
- Existing user-facing privacy flows are already in `ProfileSettingsView.svelte` (export + blocked users). Add deletion in the same settings context rather than introducing a new route.
- Existing backend user endpoints live under `/api/v1/users/me/*` in `handlers/users.rs`; this story should stay inside that boundary.

### Technical Requirements

- Backend endpoint recommendation: `DELETE /api/v1/users/me` with confirmation payload (`confirm_username`) validated against authenticated user profile.
- Keep auth source of truth as `AuthenticatedUser`; never trust client-sent user IDs.
- Deletion flow must be atomic from the API perspective: either full success or a clear error with no partial data loss.
- Account deletion must remove instance-specific session state and ensure removed token cannot access protected routes.
- Portable identity requirement: do not call identity-clearing behavior that deletes cryptographic identity from browser storage.
- Uploaded files need physical storage cleanup using `file_storage_service::delete`, not just DB row deletion.

### Architecture Compliance

1. Keep separation of concerns: handler validates/authenticates, service orchestrates deletion, models encapsulate SQL.
2. Preserve API boundary conventions (`snake_case` wire fields, explicit errors, no broad catch-and-ignore).
3. Keep Postgres/SQLite parity for all SQL behavior, especially around foreign keys and transaction semantics.
4. Maintain `AppError`-driven responses and avoid exposing internal DB/file-system details.
5. Reuse established identity/profile modules and avoid duplicating session teardown logic.

### Library & Framework Requirements

- **Backend:** Axum + SQLx + Tokio; follow existing handler/service/model patterns.
- **Storage:** Reuse `file_storage_service` for attachment/avatar deletion side effects.
- **Frontend:** Svelte 5 + existing identity store/API patterns; no direct `fetch` from component.
- **UX primitives:** Use existing dialog/button/toast patterns and destructive color tokens from the UX spec.

### File Structure Requirements

Expected primary touch points:

- `server/src/handlers/mod.rs`
- `server/src/handlers/users.rs`
- `server/src/services/mod.rs`
- `server/src/services/user_account_deletion_service.rs` (new)
- `server/src/models/user.rs`
- `server/src/models/guild.rs`
- `server/src/models/message_attachment.rs`
- `server/src/models/session.rs`
- `server/tests/server_binds_to_configured_port.rs`
- `client/src/lib/features/identity/types.ts`
- `client/src/lib/features/identity/identityApi.ts`
- `client/src/lib/features/identity/identityApi.test.ts`
- `client/src/lib/features/identity/identityStore.svelte.ts`
- `client/src/lib/features/identity/ProfileSettingsView.svelte`
- `client/src/lib/features/identity/ProfileSettingsView.test.ts`

### Testing Requirements

- **Server**
  - Unauthenticated deletion call returns `401` with standard error envelope.
  - Username-confirm mismatch returns validation error (`422`) with clear message.
  - Successful deletion removes/invalidates user session and blocks further authenticated access with old token.
  - Deletion behavior across related data is verified (messages/DMs/reactions/memberships/blocks/recovery records and file storage cleanup).
  - Owned-guild behavior is explicitly tested (chosen policy must not produce FK partial-failure).

- **Client**
  - API test verifies `DELETE /api/v1/users/me` request shape.
  - UI test verifies confirmation dialog warning text, username typing requirement, and disabled destructive action until valid.
  - UI test verifies success path logs user out from instance while retaining local identity state.
  - UI test verifies API errors surface in alert/toast patterns.

### Previous Story Intelligence

- Story 8.10 established the privacy section and export workflow in `ProfileSettingsView`; build deletion flow adjacent to this UX, not in moderation/admin areas.
- Story 8.10 also established user-data traversal patterns in backend services; reuse these data-surface assumptions when verifying deletion coverage.
- Keep AC discipline from Story 8.10: explicit payload shapes, explicit auth gating, and privacy-first regression tests.

### Git Intelligence Summary

Recent commits indicate active Epic 8 patterns and relevant files:

- `934b9f7` feat: complete story 8-10 data export (users handlers, identity API/store, ProfileSettingsView, user data export service)
- `c982e01` feat: complete story 8-9 report queue lifecycle
- `8793165` feat: complete story 8-8 user content reporting
- `58f294f` feat: complete story 8-7 moderator history
- `97a7516` feat: complete story 8-6 moderator message deletion

Actionable implication: keep privacy-account work in identity/users surfaces and avoid coupling to moderation/report queue modules.

### Latest Technical Information

- Axum `IntoResponse` patterns support clean `StatusCode`/headers/body tuple composition for deletion responses and consistent handler returns.  
  [Source: https://docs.rs/axum/latest/axum/response/index.html]
- SQLx requires proper runtime feature configuration for async APIs and transaction-backed database operations; keep feature discipline aligned with existing project config.  
  [Source: https://docs.rs/sqlx/latest/sqlx/]
- SQLite foreign-key behavior is enforced only when enabled; deletion behavior relying on FK cascades should be covered by integration tests to prevent drift.  
  [Source: https://www.sqlite.org/foreignkeys.html]
- Browser-native `window.confirm()` can be suppressed/blocked and is less controllable for accessibility; custom dialog flow is preferable for destructive account deletion UX.  
  [Source: https://developer.mozilla.org/en-US/docs/Web/API/Window/confirm]

### Project Context Reference

- No `project-context.md` was found via `**/project-context.md`.
- Context derived from planning artifacts, implementation artifacts, migrations, and current identity/profile code paths.

### Story Completion Status

- Story context file: `_bmad-output/implementation-artifacts/8-11-account-deletion-gdpr.md`
- Status set to: `ready-for-dev`
- Sprint status target: `ready-for-dev`
- Completion note: Ultimate context engine analysis completed - comprehensive developer guide created.

### Project Structure Notes

- Deletion flow should remain in user identity/profile modules, not moderation/admin pathways.
- Preserve current API and state-management conventions already used by profile, block-list, and data-export flows.
- Keep changes additive and bounded to Story 8.11 scope; avoid introducing unrelated refactors.

### Questions / Clarifications (for PM/Architect follow-up)

- `guilds.owner_id` currently references `users(id)` without explicit `ON DELETE` behavior in migration 0010. Confirm product policy for account deletion when the user owns one or more guilds (transfer ownership, delete owned guilds, or block with actionable conflict).

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Epic 8: Moderation, Reporting & Data Privacy]
- [Source: _bmad-output/planning-artifacts/epics.md#Story 8.11: Account Deletion (GDPR)]
- [Source: _bmad-output/planning-artifacts/prd.md#Compliance & Regulatory]
- [Source: _bmad-output/planning-artifacts/prd.md#Data & Privacy]
- [Source: _bmad-output/planning-artifacts/architecture.md#API Naming Conventions]
- [Source: _bmad-output/planning-artifacts/architecture.md#REST API Response Format]
- [Source: _bmad-output/planning-artifacts/architecture.md#Process Patterns]
- [Source: _bmad-output/planning-artifacts/architecture.md#FR Category: Data & Privacy (FR63-66)]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Recoverable by default]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Semantic color mapping]
- [Source: _bmad-output/implementation-artifacts/sprint-status.yaml]
- [Source: _bmad-output/implementation-artifacts/8-10-personal-data-export-gdpr.md]
- [Source: server/src/handlers/mod.rs]
- [Source: server/src/handlers/users.rs]
- [Source: server/src/services/user_data_export_service.rs]
- [Source: server/src/services/file_storage_service.rs]
- [Source: server/src/models/user.rs]
- [Source: server/src/models/guild.rs]
- [Source: server/src/models/session.rs]
- [Source: server/migrations/0004_create_sessions.sql]
- [Source: server/migrations/0010_create_guilds.sql]
- [Source: server/migrations/0018_create_messages.sql]
- [Source: server/migrations/0020_create_message_reactions.sql]
- [Source: server/migrations/0021_create_message_attachments.sql]
- [Source: server/migrations/0023_create_dm_channels.sql]
- [Source: server/migrations/0024_create_dm_messages.sql]
- [Source: server/migrations/0025_create_user_blocks.sql]
- [Source: server/tests/server_binds_to_configured_port.rs]
- [Source: client/src/lib/features/identity/identityApi.ts]
- [Source: client/src/lib/features/identity/identityApi.test.ts]
- [Source: client/src/lib/features/identity/identityStore.svelte.ts]
- [Source: client/src/lib/features/identity/ProfileSettingsView.svelte]
- [Source: client/src/lib/features/identity/ProfileSettingsView.test.ts]

## Dev Agent Record

### Agent Model Used

GPT-5.3-Codex (model ID: gpt-5.3-codex)

### Debug Log References

- `cd server && cargo fmt --check && cargo clippy -- -D warnings && cargo test`
- `cd client && npm run lint && npm run check && npm test`

### Completion Notes List

- Added authenticated `DELETE /api/v1/users/me` API route and handler with strict `confirm_username` validation and explicit error responses.
- Implemented `user_account_deletion_service` with transaction-scoped user deletion and owned-guild `409 Conflict` policy.
- Added attachment/avatar storage cleanup in the deletion flow and ensured stale tokens are rejected after deletion via session cascade.
- Extended identity client API/store with account deletion request + session-only cleanup that preserves portable browser identity.
- Added Privacy danger zone UI and modal confirmation flow with exact warning copy, username gate, destructive action styling, and toast/error feedback.
- Added backend and frontend regression tests covering auth, validation, success path, session invalidation, and ownership conflict behavior.

### File List

- _bmad-output/implementation-artifacts/sprint-status.yaml
- server/src/handlers/mod.rs
- server/src/handlers/users.rs
- server/src/models/message_attachment.rs
- server/src/services/mod.rs
- server/src/services/user_account_deletion_service.rs
- server/tests/server_binds_to_configured_port.rs
- client/src/lib/features/identity/types.ts
- client/src/lib/features/identity/identityApi.ts
- client/src/lib/features/identity/identityApi.test.ts
- client/src/lib/features/identity/identityStore.svelte.ts
- client/src/lib/features/identity/identityStore.test.ts
- client/src/lib/features/identity/ProfileSettingsView.svelte
- client/src/lib/features/identity/ProfileSettingsView.test.ts

### Change Log

- 2026-03-02: Completed Story 8.11 implementation with backend account deletion API/service, owned-guild conflict policy, client deletion flow, privacy danger-zone UI, and regression tests.
- 2026-03-03: Senior review hardening — reject whitespace-only `confirmUsername` client-side before issuing deletion requests.
- 2026-03-03: Adversarial review hardening — moved file cleanup and attachment-key discovery into DB transaction-safe flow, moved owned-guild checks into DB transactions, mapped FK delete races to conflict responses, and fixed clippy warnings in account-deletion test helpers.

### Senior Developer Review (AI)

- Outcome: Approved after adversarial review hardening and follow-up fixes.
- Fixed finding:
  - **Low**: `deleteMyAccount` accepted whitespace-only `confirmUsername`, causing avoidable API round-trips that always fail server-side validation.
  - **Fix**: updated `deleteMyAccount` to require `confirmUsername?.trim()` and added a regression test in `identityApi.test.ts`.
- Validation:
  - `cd server && cargo fmt --check && cargo clippy -- -D warnings && cargo test`
  - `cd client && npm run lint && npm run check`
- Follow-up review pass (YOLO):
  - **High**: Moved storage-file deletion to run only after successful DB transaction commit in `user_account_deletion_service` to avoid irreversible filesystem deletion before commit.
  - **High**: Moved owned-guild precheck into the active transaction for both DB backends and mapped FK delete races to explicit `409 Conflict`.
  - **Medium**: Moved attachment storage-key discovery into the active transaction to reduce stale-read cleanup gaps during account deletion.
  - **Medium**: Replaced `json_body.as_bytes().len()` with `json_body.len()` in integration-test HTTP helpers to satisfy `clippy -D warnings`.
