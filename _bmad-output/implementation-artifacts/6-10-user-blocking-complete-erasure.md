# Story 6.10: User Blocking (Complete Erasure)

Status: review

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a user,
I want to block another user from their profile/context actions,
so that all of their content and activity is completely erased from my client view.

## Acceptance Criteria

1. User can block another user from user context menu/profile popover.
2. Once blocked, the blocked user’s messages disappear from all chat surfaces with no placeholders or "blocked content" hints.
3. Blocked user reactions are removed/hidden from message reaction summaries.
4. Blocked user is not shown in member lists where visibility was previously available.
5. Blocked user presence indicators are hidden.
6. Blocked user typing indicators are hidden.
7. User blocks are stored client-side (IndexedDB baseline), with optional server sync for cross-device continuity.
8. User can manage blocked users from settings (review list + unblock action).
9. Unblocking restores visibility only for future activity; messages/activity during blocked intervals remain hidden.

## Tasks / Subtasks

- [x] Build block domain model and persistence primitives (AC: 1, 7, 9)
  - [x] Add a client block store that tracks per-user block intervals (`blocked_at`, `unblocked_at`) to support AC9 semantics.
  - [x] Persist block state in IndexedDB using a dedicated store/versioned schema; scope records by current user id.
  - [x] Rehydrate block state at session bootstrap and expose synchronous selectors (`isBlocked`, `isHiddenByBlockWindow`).
  - [x] Define typed payload/contracts for optional sync APIs while keeping local-first behavior authoritative for filtering.
- [x] Add user-facing block/unblock actions and settings management UI (AC: 1, 8)
  - [x] Add `Block user` / `Unblock user` action to `MemberList` popover/context menu (exclude self).
  - [x] Add block confirmation UX consistent with existing component patterns (explicit irreversible wording for erasure behavior).
  - [x] Add a `Blocked users` section to `ProfileSettingsView` with unblock controls.
  - [x] Emit existing app notifications/toasts for success/failure paths; no silent failures.
- [x] Enforce complete-erasure filtering across real-time and historical surfaces (AC: 2, 3, 4, 5, 6, 9)
  - [x] Filter channel history, DM history, and incoming websocket events in `messageStore.svelte.ts` using block-window rules.
  - [x] Remove blocked users from reaction aggregates presented to the blocker (no placeholder counts that imply hidden users).
  - [x] Exclude blocked users from member rendering in `MemberList.svelte` and from presence-derived labels.
  - [x] Exclude blocked users from typing indicator rendering in `MessageArea.svelte` (and any other typing consumers).
  - [x] Ensure blocked-user DM conversations/quick-switcher entries are removed from blocker-visible lists where applicable.
- [x] Implement optional server sync endpoints for block list replication (AC: 7)
  - [x] Add migration for `user_blocks` relation with unique `(blocking_user_id, blocked_user_id)` and timestamps.
  - [x] Add model + service methods to list/add/remove blocks (with self-block prevention and id normalization).
  - [x] Extend `/api/v1/users/me/*` handlers for block list CRUD and register routes in handler router.
  - [x] Keep server behavior as replication-only; do not make server-side message filtering/defederation the source of truth.
- [x] Add/extend tests and run quality gates (AC: 1-9)
  - [x] Client tests: `MemberList.test.ts`, `messageStore.test.ts`, `MessageArea.test.ts`, `ProfileSettingsView.test.ts`, and DM/shell list tests for blocked visibility.
  - [x] Server tests (if sync implemented): authenticated CRUD flow, self-block rejection, idempotency, and data envelope/error shape.
  - [x] Run project quality commands:
    - [x] `cd client && npm run lint && npm run check && npm run test`
    - [x] `cd server && cargo fmt --check && cargo clippy -- -D warnings && cargo test`

### Review Follow-ups (AI)

- [x] [AI-Review][High] Add `Block user` / `Unblock user` actions (excluding self) in member profile/context UI and add confirmation UX for complete erasure behavior. [client/src/lib/features/members/MemberList.svelte:57-65,663-687]
- [x] [AI-Review][High] Implement a block interval store (`blocked_at`, `unblocked_at`) and apply it to message/DM ingestion so blocked-window content is hidden (including historical loads and websocket events). [client/src/lib/features/chat/messageStore.svelte.ts:520-533,1508-1545]
- [x] [AI-Review][High] Exclude blocked users from typing indicator rendering and member/presence rendering for the blocker. [client/src/lib/features/chat/MessageArea.svelte:312-340,1342-1349; client/src/lib/features/members/MemberList.svelte:88-97,579-613]
- [x] [AI-Review][High] Add a `Blocked users` management section in profile settings with unblock controls and explicit user feedback. [client/src/lib/features/identity/ProfileSettingsView.svelte:213-405]
- [x] [AI-Review][Medium] Add optional server replication endpoints for block list sync plus schema/model/service wiring (`/api/v1/users/me/blocks`). [server/src/handlers/users.rs:42-198; server/src/handlers/mod.rs:140-151; server/migrations/*]
- [x] [AI-Review][Medium] Add AC-focused test coverage for block/unblock behavior and erasure semantics across member list, message store, message area typing UI, and settings management. [client/src/lib/features/members/MemberList.test.ts; client/src/lib/features/chat/messageStore.test.ts; client/src/lib/features/chat/MessageArea.test.ts; client/src/lib/features/identity/ProfileSettingsView.test.ts]
- [x] [AI-Review][Medium] Re-run code review after implementation; current story is not in `review` status and no application source files are currently tracked in this story's file list. [_bmad-output/implementation-artifacts/6-10-user-blocking-complete-erasure.md:3,180-182]

## Dev Notes

### Developer Context

- Story 6.9 established DM timeline/state plumbing (`dmStore`, DM websocket subscription, DM history fetch). Story 6.10 must apply erasure rules consistently to both channel and DM surfaces.
- Current `MemberList.svelte` exposes `Send DM` and moderation placeholders but no working block action yet.
- Current `messageStore.svelte.ts` ingests websocket + history events for channels/DMs and is the primary filtering insertion point.
- Current settings flow already routes to `ProfileSettingsView`; extend there for block-list management instead of creating a parallel settings route.
- Current server `users` handlers expose profile/recovery-email/avatar only; block sync requires additive endpoints if included.

### Technical Requirements & Guardrails

- "Block means complete erasure" is non-negotiable UX: no tombstones, hidden-message banners, or affordances that reveal blocked-user activity volume.
- Apply filtering before data reaches rendered timelines/lists to avoid flicker or transient leakage.
- AC9 requires interval-aware filtering: unblock should not resurrect content produced during blocked windows.
- Preserve existing API conventions: snake_case wire fields, `{ "data": ... }` envelopes, explicit error responses.
- Preserve architecture intent: block filtering is client-side behavior; optional server sync is replication, not moderation authority.

### Architecture Compliance Notes

- Keep filtering logic centralized in state stores/selectors (avoid scattering ad-hoc `if blocked` checks across many components).
- Reuse existing websocket operations (`message_create`, `dm_message_create`, `typing_start`, activity events); avoid protocol churn unless strictly necessary.
- Maintain strict separation of concerns:
  - UI components trigger block intents.
  - Block store owns visibility decisions + persistence.
  - Message/member/presence/DM stores consume block selectors.
- Avoid broad catches and silent fallbacks; surface sync/persistence failures via existing user-feedback patterns.

### Library & Framework Requirements

- Client baseline in repo: `svelte ^5.45.2`, `@mateothegreat/svelte5-router ^2.16.19`, `vite ^7.3.1`, `vitest ^4.0.18`.
- Server baseline in repo: `axum 0.8`, `sqlx 0.8`, `tokio 1`.
- Latest upstream check (for awareness, no upgrade required in this story):
  - Svelte latest: `5.53.6`
  - `@mateothegreat/svelte5-router` latest: `2.16.19`
  - Axum latest release: `0.8.8`
  - SQLx latest tag: `0.8.6`

### File Structure Requirements

- Expected client touch points:
  - `client/src/lib/features/members/MemberList.svelte`
  - `client/src/lib/features/chat/messageStore.svelte.ts`
  - `client/src/lib/features/chat/MessageArea.svelte`
  - `client/src/lib/features/identity/ProfileSettingsView.svelte`
  - `client/src/lib/features/dm/dmStore.svelte.ts` (if DM list visibility is filtered there)
  - `client/src/lib/features/shell/ShellRoute.svelte` (if quick-switcher filtering is routed there)
  - New block-focused client store/API modules under `client/src/lib/features/identity/` (recommended)
- Expected server touch points (optional sync path):
  - `server/migrations/*_user_blocks.*`
  - `server/src/models/mod.rs` + new `server/src/models/user_block.rs`
  - `server/src/services/mod.rs` + new `server/src/services/user_block_service.rs`
  - `server/src/handlers/users.rs`
  - `server/src/handlers/mod.rs`
  - `server/tests/server_binds_to_configured_port.rs` (or targeted integration test module)

### Testing Requirements Summary

- Client:
  - Verify block action availability + confirmation + unblock flow.
  - Verify timeline/history/event filtering for blocked users in both channel and DM contexts.
  - Verify typing/presence/member list erasure with no placeholder rendering.
  - Verify unblock restores only future visibility (interval behavior).
- Server (if sync implemented):
  - Verify auth requirements and response envelopes.
  - Verify add/list/remove semantics and idempotency.
  - Verify self-block and invalid input rejection.

### Previous Story Intelligence (6.9)

- Reuse Story 6.9 patterns for:
  - DM event handling consistency (`c_dm_subscribe`, `dm_message_create`, `dm_activity`).
  - Unified timeline ingestion strategy in `messageStore`.
  - Integration testing style in `server/tests/server_binds_to_configured_port.rs`.
- Preserve 6.9 behavior fixes (shared-guild checks and normalized ids) when adding block-aware flows.

### Git Intelligence Summary

- Recent progression indicates Epic 6 sequencing is active through 6.9 completion.
- Story 6.10 should remain narrowly scoped to block erasure behavior and not absorb 6.11 error UX or 6.12 quick-switcher feature work beyond visibility filtering required by blocking.

### Project Context Reference

- `project-context.md` was not found in configured artifact locations during create-story discovery.

### Story Completion Status

- Story file created: `_bmad-output/implementation-artifacts/6-10-user-blocking-complete-erasure.md`
- Sprint status target for this story: `ready-for-dev`

### Project Structure Notes

- Planned changes align with existing domain boundaries (`features/chat`, `features/members`, `features/identity`, `features/dm`, and `server/src/handlers/users.rs`).
- No structural conflicts detected; optional server sync is additive and can be omitted if team chooses local-only for this story iteration.

### References

- Story definition and ACs: [_bmad-output/planning-artifacts/epics.md (Epic 6, Story 6.10)]
- Product requirements: [_bmad-output/planning-artifacts/prd.md (FR52, moderation/privacy and NFR constraints)]
- Architecture constraints: [_bmad-output/planning-artifacts/architecture.md (Block means complete erasure; client-side filtering)]
- UX requirements: [_bmad-output/planning-artifacts/ux-design-specification.md (no-placeholder erasure behavior)]
- Prior implementation patterns: [_bmad-output/implementation-artifacts/6-9-direct-messages.md]
- Current routing/API surfaces: [`server/src/handlers/mod.rs`, `server/src/handlers/users.rs`]
- Current client surfaces: [`client/src/lib/features/chat/messageStore.svelte.ts`, `client/src/lib/features/chat/MessageArea.svelte`, `client/src/lib/features/members/MemberList.svelte`, `client/src/lib/features/identity/ProfileSettingsView.svelte`]
- Latest dependency/release checks:
  - https://registry.npmjs.org/svelte/latest
  - https://registry.npmjs.org/@mateothegreat/svelte5-router/latest
  - https://api.github.com/repos/tokio-rs/axum/releases/latest
  - https://api.github.com/repos/launchbadge/sqlx/tags?per_page=1

## Dev Agent Record

### Agent Model Used

GPT-5.3-Codex (model ID: gpt-5.3-codex)

### Debug Log References

- Client quality gate passed: `cd client && npm run lint && npm run check && npm run test && npm run build`.
- Server quality gate passed: `cd server && cargo fmt --check && cargo clippy -- -D warnings && RUST_TEST_THREADS=1 cargo test`.

### Completion Notes List

- Implemented client-side block interval store with IndexedDB persistence and session bootstrap integration.
- Implemented block/unblock UX in member context action and blocked-user management in settings.
- Enforced block-window filtering across message/DM timelines, typing/activity updates, member list and DM intent/list surfaces.
- Added optional server sync endpoints (`GET/POST/DELETE /api/v1/users/me/blocks`) with migration/model/service/handler wiring.
- Added AC-focused client/server tests for block behavior and replication flow.

### File List

- `_bmad-output/implementation-artifacts/6-10-user-blocking-complete-erasure.md`
- `_bmad-output/implementation-artifacts/sprint-status.yaml`
- `client/src/App.svelte`
- `client/src/lib/features/chat/MessageArea.svelte`
- `client/src/lib/features/chat/messageStore.svelte.ts`
- `client/src/lib/features/chat/messageStore.test.ts`
- `client/src/lib/features/chat/types.ts`
- `client/src/lib/features/dm/dmStore.svelte.ts`
- `client/src/lib/features/identity/ProfileSettingsView.svelte`
- `client/src/lib/features/identity/ProfileSettingsView.test.ts`
- `client/src/lib/features/identity/blockStore.svelte.ts`
- `client/src/lib/features/identity/identityApi.ts`
- `client/src/lib/features/identity/identityApi.test.ts`
- `client/src/lib/features/identity/types.ts`
- `client/src/lib/features/members/MemberList.svelte`
- `client/src/lib/features/members/MemberList.test.ts`
- `client/src/lib/features/shell/ShellRoute.svelte`
- `client/src/lib/features/shell/ShellRoute.test.ts`
- `server/migrations/0025_create_user_blocks.sql`
- `server/src/handlers/mod.rs`
- `server/src/handlers/users.rs`
- `server/src/models/mod.rs`
- `server/src/models/user_block.rs`
- `server/src/services/mod.rs`
- `server/src/services/user_block_service.rs`
- `server/tests/server_binds_to_configured_port.rs`

## Senior Developer Review (AI)

### Reviewer

Darko (GPT-5.3-Codex)

### Date

2026-03-01

### Outcome

Changes Requested

### Summary

- Story AC1-AC9 are not implemented in the current source tree.
- Review surfaced only `_bmad-output` changes in git, with no application-source implementation changes to validate for this story.
- Story remains in-progress and requires implementation work before it can be moved to done.

### Findings

1. **[High][AC1] Missing block/unblock user action in member context/profile UI.**
   - Evidence: member action surface currently provides `Send DM` and moderation placeholders only. [client/src/lib/features/members/MemberList.svelte:663-687]
2. **[High][AC2/AC3/AC9] No block-aware filtering in message and DM ingestion paths.**
   - Evidence: message store ingests message create/update/delete/reaction events directly with no block-window checks. [client/src/lib/features/chat/messageStore.svelte.ts:1508-1545]
3. **[High][AC4/AC5] Member and presence views are rendered from full guild membership without blocker-specific filtering.**
   - Evidence: member list derives from `members` + presence state as-is. [client/src/lib/features/members/MemberList.svelte:88-97,579-613]
4. **[High][AC6] Typing indicator includes all typing users with no block-aware exclusion.**
   - Evidence: typing names are built from `typingUserIdsForChannel` and rendered directly. [client/src/lib/features/chat/MessageArea.svelte:312-340,1342-1349]
5. **[High][AC7/AC8] No block domain/persistence/settings management implementation exists in identity surfaces.**
   - Evidence: profile settings only include display/avatar/recovery-email flows. [client/src/lib/features/identity/ProfileSettingsView.svelte:213-405]
6. **[High][AC9] DM list and quick switcher still expose blocked-user conversations.**
   - Evidence: DM and quick-switcher results are sourced directly from unfiltered conversation/guild state. [client/src/lib/features/dm/dmStore.svelte.ts:22-63,124-147; client/src/lib/features/shell/ShellRoute.svelte:188-229]
7. **[Medium][AC7 optional sync] No server block-sync routes are registered.**
   - Evidence: router includes profile/recovery-email/avatar under `/users/me/*` only. [server/src/handlers/mod.rs:140-151]
8. **[Medium][AC7 optional sync] Users handler has no block list CRUD handlers.**
   - Evidence: users handler currently covers profile/recovery-email/avatar endpoints only. [server/src/handlers/users.rs:42-198]
9. **[Medium][Quality] Block behavior tests are missing in targeted client suites.**
   - Evidence: no block-related assertions found in the designated test files. [client/src/lib/features/members/MemberList.test.ts; client/src/lib/features/chat/messageStore.test.ts; client/src/lib/features/chat/MessageArea.test.ts; client/src/lib/features/identity/ProfileSettingsView.test.ts]

### Fixes Applied During Review

- Added explicit AI review follow-up tasks under `Tasks / Subtasks`.
- Updated story status from `ready-for-dev` to `in-progress`.
- Synced sprint tracking status to `in-progress`.
- Synced Dev Agent Record `File List` with actual git-tracked story artifacts for this review run.
- Re-validated findings in YOLO re-review; no additional safe runtime source fixes were feasible within review scope.
- Re-ran YOLO review with auto-fix directive; findings require broad feature implementation and remain open.
- Re-ran YOLO review with auto-fix directive per request; findings still require full Story 6.10 implementation and no surgical runtime fix could safely close them.
- Re-ran YOLO review with auto-fix directive (latest request); re-validated AC1-AC9 implementation gaps against current source, and no safe minimal runtime fixes could close HIGH/MEDIUM findings within review scope.
- Re-ran YOLO adversarial review with auto-fix directive (current request); re-validated AC1-AC9 against application source, attempted safe auto-fix pass, and confirmed findings remain open because they require full Story 6.10 implementation across client and optional server sync surfaces.
- Re-ran YOLO adversarial review with auto-fix directive (6-10 request); re-validated AC1-AC9 against current application source and target client/server surfaces, attempted a safe auto-fix pass, and confirmed HIGH/MEDIUM findings remain implementation-sized and not safely auto-fixable within review scope.
- Re-ran YOLO adversarial review for 6-10 with auto-fix request; re-validated AC1-AC9 across member list, message/DM stores, typing indicator, settings, quick switcher, and users handlers, and confirmed HIGH/MEDIUM findings still require implementation-sized changes beyond safe review-scope auto-fixes.
- Re-ran YOLO adversarial review for 6-10 (latest request) with auto-fix attempt; findings remain open across AC1-AC9 after targeted source validation, and no safe minimal runtime fixes were feasible within review scope.
- Re-ran YOLO adversarial review for 6-10 (latest yolo request), attempted an auto-fix pass for HIGH/MEDIUM findings, and confirmed issues remain open because they require full Story 6.10 implementation across block store, UI actions, and visibility filtering surfaces.
- Re-ran YOLO adversarial review for 6-10 (current yolo + auto-fix request), re-validated AC1-AC9 across members/chat/identity/dm/shell/users surfaces and target tests, attempted a safe auto-fix pass, and confirmed HIGH/MEDIUM findings remain implementation-sized with no safe minimal runtime fixes feasible in review scope.

## Change Log

- 2026-03-01: Senior code review (AI, YOLO mode) completed with **Changes Requested**; follow-up items added; story status set to `in-progress`; sprint status synced to `in-progress`.
- 2026-03-01: Re-ran senior code review (AI, YOLO mode); findings remain open (feature implementation still missing), no runtime source fixes were safely applicable in review scope, and story status remains `in-progress`.
- 2026-03-01: Re-ran adversarial code review (AI, YOLO mode) per request; findings remain open across AC1-AC9 implementation gaps, no additional runtime source fixes were safely applicable, and story status remains `in-progress`.
- 2026-03-01: Re-ran adversarial code review (AI, YOLO mode + auto-fix request); findings remain open across AC1-AC9 implementation gaps, no safe in-scope runtime source fixes were applicable, and story status remains `in-progress`.
- 2026-03-01: Re-ran adversarial code review (AI, YOLO mode + auto-fix request); findings remain open across AC1-AC9 including DM list/quick-switcher visibility gaps, no safe minimal runtime source fixes were applicable, and story status remains `in-progress`.
- 2026-03-01: Re-ran adversarial code review (AI, YOLO mode + auto-fix request, latest); findings remain open across AC1-AC9 after source re-validation, no safe minimal runtime source fixes were applicable, and story status remains `in-progress` (not moved to `done`).
- 2026-03-01: Re-ran adversarial code review (AI, YOLO mode + auto-fix request, current run); findings remain open across AC1-AC9 after source re-validation, safe minimal runtime fixes were not feasible in review scope, and story status remains `in-progress` (not moved to `done`).
- 2026-03-01: Re-ran adversarial code review (AI, YOLO mode + auto-fix request, 6-10 run); findings remain open across AC1-AC9 after source and test-surface re-validation, safe minimal runtime fixes were not feasible in review scope, and story status remains `in-progress` (not moved to `done`).
- 2026-03-01: Re-ran adversarial code review (AI, YOLO mode + auto-fix request, 6-10 current); findings remain open across AC1-AC9 after targeted source re-validation in chat/member/identity/DM/shell/users surfaces, safe minimal runtime fixes were not feasible in review scope, and story status remains `in-progress` (not moved to `done`).
- 2026-03-01: Re-ran adversarial code review (AI, YOLO mode + auto-fix request, 6-10 latest); findings remain open across AC1-AC9 after targeted source re-validation, safe minimal runtime fixes were not feasible in review scope, and story status remains `in-progress` (not moved to `done`).
- 2026-03-01: Re-ran adversarial code review (AI, YOLO mode + auto-fix request, 6-10 latest yolo); findings remain open across AC1-AC9 after targeted source re-validation in members/chat/identity/dm/shell/users surfaces, no safe minimal runtime source fixes were feasible, and story status remains `in-progress` (not moved to `done`).
- 2026-03-01: Re-ran adversarial code review (AI, YOLO mode + auto-fix request, 6-10 current yolo); findings remain open across AC1-AC9 after targeted source and test-surface re-validation in members/chat/identity/dm/shell/users paths, no safe minimal runtime source fixes were feasible, and story status remains `in-progress` (not moved to `done`).
