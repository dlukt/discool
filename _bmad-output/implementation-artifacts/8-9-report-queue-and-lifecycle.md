# Story 8.9: Report Queue and Lifecycle

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a **moderator**,
I want to review and act on user reports,
so that I can address community issues efficiently.

## Acceptance Criteria

1. **Given** a user has `VIEW_MOD_LOG` permission  
   **When** they open the report queue (via mod tools in the guild sidebar)  
   **Then** they see a list of reports as `ReportQueueItem` components, sorted by newest first.

2. **Given** the report queue renders items  
   **When** each item is displayed  
   **Then** it shows status badge (`pending` / `reviewed` / `actioned` / `dismissed`), content preview, reporter identity, reason, and timestamp.

3. **Given** reports are displayed  
   **When** status is `pending`  
   **Then** pending items are highlighted, while `reviewed` / `actioned` / `dismissed` use neutral or muted styling.

4. **Given** a moderator reviews a report  
   **When** they choose an action from the queue  
   **Then** they can dismiss, warn (send DM), mute, kick, or ban directly from the report item.

5. **Given** a moderator takes a moderation action from a report  
   **When** the action succeeds  
   **Then** report status updates from `pending` to `actioned`, and a corresponding mod-log entry exists for the action.

6. **Given** a moderator dismisses a report  
   **When** dismissal is confirmed  
   **Then** status changes to `dismissed` and an optional dismissal reason is persisted.

7. **Given** no pending reports exist  
   **When** the report queue is shown  
   **Then** the empty state text is exactly: **"No pending reports."**

8. **Given** report lifecycle is tracked  
   **When** transitions occur  
   **Then** lifecycle progression is represented as `pending -> reviewed -> actioned/dismissed` without invalid transitions.

## Tasks / Subtasks

- [x] Task 1: Add report lifecycle persistence and transition safety (AC: 2, 5, 6, 8)
  - [x] Add migration `server/migrations/0033_report_lifecycle_tracking.sql` (PostgreSQL + SQLite parity) to support lifecycle metadata needed for queue actions (reviewed/acted/dismissed timestamps/actors, optional dismissal reason, optional moderation-action linkage).
  - [x] Keep existing status enum constraints (`pending`, `reviewed`, `actioned`, `dismissed`) and add any new constraints needed to prevent inconsistent lifecycle data.
  - [x] Preserve/extend queue indexes so newest-first and pending-focused retrieval remains efficient.

- [x] Task 2: Extend moderation model layer for report queue and lifecycle transitions (AC: 1, 2, 5, 6, 8)
  - [x] Extend `server/src/models/moderation.rs` with report queue read models including reporter and target preview fields (message/user context).
  - [x] Add list/query helpers for report queue retrieval with filter + cursor support and newest-first ordering.
  - [x] Add lifecycle transition helpers that enforce allowed status transitions at write time (no silent invalid transitions).
  - [x] Add helper(s) to persist optional dismissal reason and optional moderation action linkage for actioned reports.

- [x] Task 3: Implement service-layer report queue orchestration and moderation actions (AC: 1, 4, 5, 6, 8)
  - [x] Add service input/output types in `server/src/services/moderation_service.rs` for report queue list/review/dismiss/action operations.
  - [x] Require `VIEW_MOD_LOG` permission for queue visibility and lifecycle operations.
  - [x] Reuse existing moderation flows for `mute`, `kick`, and `ban` to avoid duplicating business rules and hierarchy checks.
  - [x] Implement report-driven `warn` action (send DM + mod-log entry) in moderation service with explicit error behavior.
  - [x] Ensure successful action path updates report lifecycle consistently (`pending` -> `reviewed` / `actioned`) and captures linkage to the generated moderation action where applicable.

- [x] Task 4: Add moderation report queue HTTP endpoints and route wiring (AC: 1, 4, 5, 6, 8)
  - [x] Add handlers in `server/src/handlers/moderation.rs` for queue list and lifecycle operations (review, dismiss, act).
  - [x] Add route wiring in `server/src/handlers/mod.rs` under `/api/v1/guilds/{guild_slug}/moderation/reports/...`.
  - [x] Keep API contract conventions: `snake_case` wire fields, `{ "data": ... }` envelope, explicit `400/403/404/409/422` errors.
  - [x] Validate and normalize all lifecycle/action payload fields (including optional dismissal reason and action-specific input).

- [x] Task 5: Extend client moderation API contract for report queue and actions (AC: 1, 2, 4, 5, 6, 8)
  - [x] Add report queue lifecycle types and API methods in `client/src/lib/features/moderation/moderationApi.ts` (fetch queue, mark reviewed, dismiss, apply action).
  - [x] Preserve strict client-side validation and `ApiError` semantics consistent with existing moderation methods.
  - [x] Add/extend tests in `client/src/lib/features/moderation/moderationApi.test.ts` for paths, payload mapping, and error handling.

- [x] Task 6: Build report queue UI in moderation tools surface (AC: 1, 2, 3, 4, 7)
  - [x] Add `ReportQueuePanel.svelte` and `ReportQueueItem.svelte` under `client/src/lib/features/moderation/` following existing `ModLogPanel` component patterns.
  - [x] Render status badge, content preview, reporter identity, reason, timestamp, and queue actions.
  - [x] Show exact empty state copy **"No pending reports."** when pending list is empty.
  - [x] Keep status visuals aligned with UX spec (pending highlighted; non-pending muted/neutral).
  - [x] Ensure keyboard-accessible action controls and no regression in current member/moderation interactions.

- [x] Task 7: Integrate report queue tab into member moderation shell (AC: 1, 7)
  - [x] Extend `client/src/lib/features/members/MemberList.svelte` panel model to include a report queue tab for moderators.
  - [x] Keep permission gating aligned to `VIEW_MOD_LOG` and existing mod-log tab behavior.
  - [x] Preserve panel switching semantics and reset behavior (no dialog/panel state leaks across tabs).

- [x] Task 8: Add regression coverage and run quality gates (AC: all)
  - [x] Server tests for permission gating, queue listing order, lifecycle transitions, dismiss reason handling, and report actioned linkage to moderation actions.
  - [x] Handler/integration tests for full report action paths (`dismiss`, `warn`, `mute`, `kick`, `ban`) and status progression behavior.
  - [x] Client tests for tab visibility, report queue rendering, empty state, status highlighting, and queue action flows.
  - [x] Run project quality gates:
    - [x] `cd client && npm run lint && npm run check`
    - [x] `cd server && cargo fmt --check && cargo clippy -- -D warnings && cargo test`

## Dev Notes

### Developer Context

- Story 8.8 already established report creation (`message` / `user`) with `status = pending` and duplicate/self-report protections; Story 8.9 must build on this exact data model instead of creating a parallel reporting path.
- Existing moderation architecture and permissions flow are already stable and should be reused:
  - backend layering: handlers -> services -> models
  - permission checks via `permissions::require_guild_permission`
  - standardized API envelope and error behavior.
- Existing moderator shell surface is `MemberList` with `Members` and `Moderation log` tabs; report queue should integrate into this same surface for continuity.

### Technical Requirements

- Queue visibility and lifecycle actions require `VIEW_MOD_LOG`.
- Queue order for moderator UX is newest-first.
- Lifecycle transitions must be explicit and validated:
  - allow `pending -> reviewed`
  - allow `reviewed -> actioned|dismissed`
  - allow direct action flow from pending only when transition data remains consistent (implementation must not produce contradictory state).
- Dismiss flow must support optional moderator dismissal reason.
- Report actions available from queue: `dismiss`, `warn`, `mute`, `kick`, `ban`.
- `mute` / `kick` / `ban` report actions must reuse existing moderation service logic (hierarchy checks, permission behavior, side effects).
- `warn` must produce an auditable moderation action record and send a direct warning message to target user.
- Report queue response payload must include data required by `ReportQueueItem` UI:
  - status
  - reporter identity
  - target preview (message excerpt or user metadata)
  - reason
  - timestamp(s).

### Architecture Compliance

1. Preserve strict layering: handlers parse/validate, services orchestrate policy, models own SQL.
2. Keep API boundary conventions: `snake_case` JSON and `{ "data": ... }` envelope.
3. Reuse existing moderation primitives rather than introducing duplicate moderation logic.
4. Maintain PostgreSQL/SQLite parity for migrations and queries.
5. Keep explicit error behavior; no silent fallback transitions or no-op lifecycle writes.
6. Follow established moderation UI loading/empty/error states and keyboard accessibility patterns.

### Library & Framework Requirements

- Backend remains Axum + Tower middleware patterns with extractor-driven handlers.
- Persistence remains SQLx with runtime/TLS feature discipline already used in project.
- Frontend remains Svelte 5 component-first architecture; moderation queue UI should follow existing feature component patterns.
- Testing remains Vitest on client and Rust unit/integration tests on server.

### File Structure Requirements

Expected primary touch points:

- `server/migrations/0033_report_lifecycle_tracking.sql` (new)
- `server/src/models/moderation.rs`
- `server/src/services/moderation_service.rs`
- `server/src/handlers/moderation.rs`
- `server/src/handlers/mod.rs`
- `server/tests/server_binds_to_configured_port.rs`
- `client/src/lib/features/moderation/moderationApi.ts`
- `client/src/lib/features/moderation/moderationApi.test.ts`
- `client/src/lib/features/moderation/ReportQueuePanel.svelte` (new)
- `client/src/lib/features/moderation/ReportQueuePanel.test.ts` (new)
- `client/src/lib/features/moderation/ReportQueueItem.svelte` (new)
- `client/src/lib/features/members/MemberList.svelte`
- `client/src/lib/features/members/MemberList.test.ts`

### Testing Requirements

- **Server**
  - Queue listing requires `VIEW_MOD_LOG`.
  - Queue sorting and filtering produce expected lifecycle ordering.
  - Lifecycle transition validation rejects invalid transitions.
  - Dismiss action persists optional dismissal reason and status.
  - Action flows (`warn`, `mute`, `kick`, `ban`) update report state and create moderation action/log records.
- **Client**
  - Report queue tab visibility follows permission gating.
  - Queue list renders required fields and correct status styling semantics.
  - Empty pending queue copy exactly matches **"No pending reports."**.
  - Queue actions dispatch correct API calls and refresh state predictably.
  - Existing member list and moderation log behavior do not regress.

### Previous Story Intelligence

- Story 8.8 implemented report intake with these key constraints now depended on by 8.9:
  - `reports` table and status constants already include `pending/reviewed/actioned/dismissed`.
  - Report creation endpoints:
    - `POST /api/v1/guilds/{guild_slug}/moderation/reports/messages/{message_id}`
    - `POST /api/v1/guilds/{guild_slug}/moderation/reports/users/{target_user_id}`
  - Deduplication and self-report prevention are already enforced server-side.
- Existing UI report dialogs and API clients are in place (`ReportDialog.svelte`, `createMessageReport`, `createUserReport`), so 8.9 should extend these moderation flows, not replace them.

### Git Intelligence Summary

- `8793165` feat: complete story 8-8 user content reporting
- `58f294f` feat: complete story 8-7 moderator history
- `97a7516` feat: complete story 8-6 moderator message deletion
- `3210f9c` feat: complete story 8-5 moderation log
- `90a3b7f` feat: complete story 8-4 voice kick moderation flow

Recent history confirms moderation work is concentrated in:
- `server/src/handlers/moderation.rs`
- `server/src/services/moderation_service.rs`
- `server/src/models/moderation.rs`
- `client/src/lib/features/moderation/`
- `client/src/lib/features/members/MemberList.svelte`

### Latest Technical Information

- Axum documentation continues to emphasize extractor-based handlers and Tower middleware composition, which matches current moderation handler architecture and should be preserved.  
  [Source: https://docs.rs/axum/latest/axum/]
- SQLx documentation continues to require explicit runtime/TLS feature configuration and warns about runtime feature mismatches; maintain existing project runtime/TLS configuration discipline for new queue queries.  
  [Source: https://docs.rs/sqlx/latest/sqlx/]
- Svelte documentation remains component-first and compile-time optimized; report queue UI should remain focused, typed Svelte components in existing feature structure.  
  [Source: https://svelte.dev/docs/svelte/overview]
- Vitest guidance confirms Vite-aligned testing workflow and `.test` naming conventions; keep moderation queue tests co-located in feature test files.  
  [Source: https://vitest.dev/guide/]

### Project Context Reference

- No `project-context.md` was found via `**/project-context.md`.
- Context was derived from planning artifacts, current implementation artifacts, existing moderation code, and git history.

### Story Completion Status

- Story context file: `_bmad-output/implementation-artifacts/8-9-report-queue-and-lifecycle.md`
- Status set to: `ready-for-dev`
- Sprint status target: `ready-for-dev`
- Completion note: Ultimate context engine analysis completed - comprehensive developer guide created.

### Project Structure Notes

- Keep report queue implementation in existing moderation boundaries (`handlers/moderation.rs`, `services/moderation_service.rs`, `models/moderation.rs`).
- Keep moderator UI integration in existing member/moderation feature area (`MemberList` + `features/moderation/*`).
- Avoid introducing a separate moderator app surface or parallel permission system.

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Epic 8: Moderation, Reporting & Data Privacy]
- [Source: _bmad-output/planning-artifacts/epics.md#Story 8.9: Report Queue and Lifecycle]
- [Source: _bmad-output/planning-artifacts/epics.md#Story 8.8: User Content Reporting]
- [Source: _bmad-output/planning-artifacts/prd.md#UGC Reporting & Content Moderation]
- [Source: _bmad-output/planning-artifacts/prd.md#FR53]
- [Source: _bmad-output/planning-artifacts/prd.md#FR54]
- [Source: _bmad-output/planning-artifacts/prd.md#FR55]
- [Source: _bmad-output/planning-artifacts/architecture.md#API & Communication Patterns]
- [Source: _bmad-output/planning-artifacts/architecture.md#API Naming Conventions]
- [Source: _bmad-output/planning-artifacts/architecture.md#REST API Response Format]
- [Source: _bmad-output/planning-artifacts/architecture.md#Process Patterns]
- [Source: _bmad-output/planning-artifacts/architecture.md#FR Category: Moderation & Safety (FR45-55)]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Journey 4: Moderation Workflow (Rico)]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#ReportQueueItem]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Context Menu Patterns]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Empty report queue]
- [Source: _bmad-output/implementation-artifacts/8-8-user-content-reporting.md]
- [Source: _bmad-output/implementation-artifacts/sprint-status.yaml]
- [Source: server/migrations/0032_create_reports_table.sql]
- [Source: server/src/models/moderation.rs]
- [Source: server/src/services/moderation_service.rs]
- [Source: server/src/handlers/moderation.rs]
- [Source: server/src/handlers/mod.rs]
- [Source: client/src/lib/features/moderation/moderationApi.ts]
- [Source: client/src/lib/features/moderation/ModLogPanel.svelte]
- [Source: client/src/lib/features/moderation/ReportDialog.svelte]
- [Source: client/src/lib/features/members/MemberList.svelte]
- [Source: client/src/lib/features/guild/permissions.ts]
- [Source: https://docs.rs/axum/latest/axum/]
- [Source: https://docs.rs/sqlx/latest/sqlx/]
- [Source: https://svelte.dev/docs/svelte/overview]
- [Source: https://vitest.dev/guide/]

## Dev Agent Record

### Agent Model Used

GPT-5.3-Codex (model ID: gpt-5.3-codex)

### Debug Log References

- `cd server && cargo fmt --check && cargo test -q`
- `cd client && npm run lint && npm run check && npm run test -- --run`
- `cd server && cargo fmt --check && cargo clippy -- -D warnings && cargo test -q`

### Completion Notes List

- Added report lifecycle migration/model/service support for queue listing, review, dismiss, and action transitions with report-to-moderation action linkage.
- Wired backend HTTP endpoints under `/api/v1/guilds/{guild_slug}/moderation/reports/...` for list/review/dismiss/actions.
- Added client report queue API methods plus new `ReportQueuePanel`/`ReportQueueItem` moderation UI and integrated report queue tab in `MemberList`.
- Added client coverage (`moderationApi`, `ReportQueuePanel`, `MemberList`) and stabilized moderation service tests for permission-cache isolation.
- Ran lint/check/tests for client and fmt/clippy/tests for server.
- Senior review fix: tightened lifecycle consistency so `actioned` reports cannot persist `dismissal_reason`.
- Senior review fix: hardened `act_on_report` to reserve lifecycle status before action execution, roll back to `reviewed` on action failure, and reconcile stale reservation rows older than the reservation TTL.
- Added service coverage for rollback and stale-reservation reconciliation paths.

### File List

- `server/migrations/0033_report_lifecycle_tracking.sql`
- `server/src/models/moderation.rs`
- `server/src/services/moderation_service.rs`
- `server/src/handlers/moderation.rs`
- `server/src/handlers/mod.rs`
- `client/src/lib/features/moderation/moderationApi.ts`
- `client/src/lib/features/moderation/moderationApi.test.ts`
- `client/src/lib/features/moderation/ReportQueuePanel.svelte`
- `client/src/lib/features/moderation/ReportQueuePanel.test.ts`
- `client/src/lib/features/moderation/ReportQueueItem.svelte`
- `client/src/lib/features/members/MemberList.svelte`
- `client/src/lib/features/members/MemberList.test.ts`
- `_bmad-output/implementation-artifacts/sprint-status.yaml`
- `_bmad-output/implementation-artifacts/8-9-report-queue-and-lifecycle.md`

### Change Log

- Added report queue lifecycle implementation across migration, model/service/handlers, and moderation UI integration.
- Added/updated moderation queue client and UI tests (`moderationApi`, `ReportQueuePanel`, `MemberList`).
- Senior review fix: enforced `dismissal_reason IS NULL` for `status = 'actioned'` in `server/migrations/0033_report_lifecycle_tracking.sql`.
- Senior review fix: made report action transitions consistency-safe with pre-action reservation, rollback-on-failure, and stale reservation reconciliation.
- Re-ran and passed quality gates:
  - `cd server && cargo fmt --check && cargo clippy -- -D warnings && cargo test`
  - `cd client && npm run lint && npm run check && npm run test -- --run`

### Senior Developer Review (AI)

- Outcome: Approved after one high-severity data-consistency fix.
- Fixed finding:
  - **High**: lifecycle constraint allowed `actioned` reports to retain a dismissal reason, permitting contradictory state.
  - **Fix**: added `AND dismissal_reason IS NULL` in `ck_reports_lifecycle_consistency` actioned branch (`server/migrations/0033_report_lifecycle_tracking.sql`).
- Additional hardening review outcome: Approved after report action reservation consistency updates.
- Fixed findings:
  - **High**: moderation actions could succeed before report status transition finalized, allowing action-without-state under optimistic-lock conflicts.
  - **Fix**: reserve report action state first, execute moderation action, then finalize linkage; on action failure, explicitly roll back to `reviewed`.
  - **Medium**: crash/interruption during reserved action window could leave stale `actioned` rows without linked moderation actions.
  - **Fix**: reconcile stale reservations (`status = actioned`, `moderation_action_id IS NULL`) to `reviewed` after TTL, before queue/action lifecycle operations.
- Validation:
  - `cd server && cargo fmt --check && cargo clippy -- -D warnings && cargo test`
  - `cd server && cargo test -- --test-threads=1`
  - `cd server && cargo test act_on_report_restores_review_state_when_action_fails_after_reserve -- --test-threads=1`
  - `cd server && cargo test list_report_queue_reconciles_stale_action_reservations -- --test-threads=1`
  - `cd server && cargo test act_on_report_reconciles_stale_action_reservations_before_processing -- --test-threads=1`
  - `cd client && npm run lint && npm run check && npm run test -- --run`
