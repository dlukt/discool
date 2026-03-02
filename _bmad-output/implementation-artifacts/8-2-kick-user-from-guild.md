# Story 8.2: Kick User from Guild

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a **moderator**,
I want to kick a user from the guild,
so that I can remove them while allowing them to rejoin via a new invite.

## Acceptance Criteria

1. **Given** a user has `KICK_MEMBERS` permission  
   **When** they right-click a member -> Kick from guild  
   **Then** a confirmation dialog appears with the user's name and a required reason field.

2. **Given** a moderator confirms kick  
   **When** the operation succeeds  
   **Then** the target user is removed from the guild.

3. **Given** a kicked user is currently active in the app  
   **When** kick is processed  
   **Then** the kicked user's guild disappears from their GuildRail.

4. **Given** a user was kicked from a guild  
   **When** they receive a new valid invite  
   **Then** they can rejoin the guild.

5. **Given** a moderator attempts kick  
   **When** role hierarchy is evaluated  
   **Then** target must have lower highest role than the moderator.

6. **Given** kick action succeeds  
   **When** moderator returns to member list  
   **Then** toast confirms action: "User kicked from guild".

7. **Given** kick action is executed  
   **When** audit data is recorded  
   **Then** action is persisted in a format consumable by Story 8.5 moderation log.

## Tasks / Subtasks

- [x] Task 1: Extend moderation persistence for kick actions (AC: 7)
  - [x] Add migration `server/migrations/0027_enable_kick_moderation_actions.sql` to allow `action_type IN ('mute', 'kick')` for `moderation_actions`.
  - [x] Keep backward compatibility with existing 8.1 mute records and indexes; do not drop existing moderation data.
  - [x] Update model constants in `server/src/models/moderation.rs` with `MODERATION_ACTION_TYPE_KICK`.

- [x] Task 2: Add guild-member removal primitives with cleanup (AC: 2, 4)
  - [x] Add `remove_guild_member` in `server/src/models/guild_member.rs` (Postgres + SQLite parity).
  - [x] Add helper to remove role assignments for a specific guild/user in `server/src/models/role.rs` so kicked users do not retain old role grants after rejoin.
  - [x] Keep owner membership invariant: guild owner cannot be kicked.

- [x] Task 3: Implement server kick workflow (AC: 1, 2, 4, 5, 7)
  - [x] Add `CreateKickInput` and `KickActionResponse` in `server/src/services/moderation_service.rs`.
  - [x] Implement `create_kick` flow: normalize inputs, require non-empty reason, require `KICK_MEMBERS`, require target membership, prevent self-kick, enforce `permissions::actor_outranks_target_member`.
  - [x] Persist kick moderation action through `moderation::insert_moderation_action` with `action_type='kick'`, `duration_seconds=NULL`, `expires_at=NULL`.
  - [x] Deactivate active mutes for target when kicking, so rejoin starts from clean moderation state unless a new mute is applied.
  - [x] Remove guild membership and stale role assignments in one transactional flow (or equivalent atomic behavior per backend).

- [x] Task 4: Add moderation kick API endpoint and routing (AC: 1, 7)
  - [x] Add `POST /api/v1/guilds/{guild_slug}/moderation/kicks` in `server/src/handlers/moderation.rs`.
  - [x] Wire route in `server/src/handlers/mod.rs`.
  - [x] Preserve API contract: `{ "data": ... }` response envelope and `snake_case` JSON fields.

- [x] Task 5: Revoke kicked user live access and refresh client state (AC: 3)
  - [x] Use WebSocket registry targets to remove kicked user's active guild/channel subscriptions for the affected guild.
  - [x] Emit `guild_update` event payload for impacted users and handle it client-side to refresh guild membership (`guildState.loadGuilds(true)`).
  - [x] Ensure active kicked route falls back safely (e.g., home or first available guild/channel) instead of leaving user on inaccessible guild path.

- [x] Task 6: Replace member-list kick placeholder with real action (AC: 1, 6)
  - [x] Extend `client/src/lib/features/moderation/moderationApi.ts` with `createKick`.
  - [x] In `client/src/lib/features/members/MemberList.svelte`, replace "Kick member (coming soon)" with a real confirmation dialog that includes:
    - [x] Target user's display name
    - [x] Required reason field
    - [x] Destructive confirmation action
  - [x] On success, show toast exactly "User kicked from guild" and refresh member/guild data.

- [x] Task 7: Add regression tests (AC: all)
  - [x] Server service tests for kick permission checks, hierarchy guardrails, owner/self protections, membership removal, and moderation action insert.
  - [x] Server handler tests for request validation and data-envelope responses.
  - [x] Client tests in `MemberList.test.ts` for kick dialog validation, API payload, success toast, and placeholder removal.
  - [x] Client/store tests for `guild_update` handling and kicked-user guild-list refresh behavior.
  - [x] Run quality gates:
    - [x] `cd client && npm run lint && npm run check && npm run test && npm run build`
    - [x] `cd server && cargo fmt --check && cargo clippy -- -D warnings && cargo test`

## Dev Notes

### Developer Context

- Story 8.1 already introduced moderation modules and APIs for mutes (`handlers/moderation.rs`, `services/moderation_service.rs`, `models/moderation.rs`) and integrated member-list moderation UI entry points.
- `MemberList.svelte` currently exposes a visible "Kick member (coming soon)" action for users with `KICK_MEMBERS`, so this story should replace that placeholder in-place rather than adding parallel moderation UI.
- Current `moderation_actions` schema is mute-only (`ck_moderation_actions_type CHECK (action_type = 'mute')`), so a migration is required before persisting kick records.
- There is currently no `DELETE FROM guild_members` model helper; guild membership removal must be introduced deliberately with DB parity and explicit error handling.
- Role assignments are stored separately from guild membership (`role_assignments`), so kick flow should explicitly clean those assignments to avoid unintentionally restoring old roles on rejoin.

### Technical Requirements

- Authorization:
  - Use `permissions::require_guild_permission(..., KICK_MEMBERS, "KICK_MEMBERS")`.
  - Enforce hierarchy with `permissions::actor_outranks_target_member`.
  - Reject self-kick and owner kick with explicit validation/forbidden errors.
- Input validation:
  - `target_user_id` required, trimmed non-empty.
  - `reason` required, trimmed non-empty, max 500 chars, no invalid control chars (reuse moderation reason normalization pattern).
- Data semantics:
  - Kick removes target from `guild_members`.
  - Kick action is appended to `moderation_actions` for Story 8.5 consumption.
  - Kick does not create guild-level ban state and must not block rejoin via invite.
- Live behavior:
  - Kicked user should lose guild-level real-time access immediately (not only after full page reload).
  - GuildRail should update via refresh/event handling so removed guild no longer appears.
- API conventions:
  - Keep `snake_case` wire format and `{ "data": ... }` envelopes.
  - Do not add silent fallbacks; return explicit validation/forbidden/not-found errors.

### Architecture Compliance

1. Keep layers strict: handlers validate/serialize, services own kick workflow and permission logic, models own SQL.
2. Maintain Postgres/SQLite parity for every new model/migration path.
3. Reuse existing moderation and permission helpers; do not duplicate hierarchy logic in handlers.
4. Preserve current real-time architecture; extend existing WebSocket op/event pathways rather than inventing side channels.
5. Preserve UX conventions from 8.1: context-menu action, explicit reason field, toast confirmation, keyboard compatibility.

### Library & Framework Requirements

- Backend: Rust + Axum + SQLx + Tokio (existing stack only; no new backend frameworks).
- Frontend: Svelte 5 + TypeScript + existing stores/utilities; no new global state framework.
- UI: keep dialog and destructive action patterns consistent with existing member moderation UX.

### File Structure Requirements

Expected primary touch points:

- `server/migrations/0027_enable_kick_moderation_actions.sql` (new)
- `server/src/models/guild_member.rs`
- `server/src/models/role.rs`
- `server/src/models/moderation.rs`
- `server/src/services/moderation_service.rs`
- `server/src/handlers/moderation.rs`
- `server/src/handlers/mod.rs`
- `server/src/ws/gateway.rs` and/or `server/src/ws/registry.rs` (kick-triggered live update wiring)
- `client/src/lib/features/moderation/moderationApi.ts`
- `client/src/lib/features/members/MemberList.svelte`
- `client/src/lib/features/members/MemberList.test.ts`
- `client/src/lib/features/guild/guildStore.svelte.ts` (guild refresh support for kick updates)
- `client/src/lib/features/shell/ShellRoute.svelte` (fallback routing when active guild disappears)

### Testing Requirements

- Server:
  - Kick endpoint/service validates required reason and required `KICK_MEMBERS`.
  - Kick rejects self-target, owner target, and equal/higher role target.
  - Successful kick removes membership and records moderation action.
  - Kicked user remains able to rejoin via invite path (not banned).
- Client:
  - Kick dialog appears from member context actions for eligible moderator permissions.
  - Reason field required; invalid submit shows user-facing error.
  - Successful submit calls kick API with expected payload and shows exact toast text.
  - Kicked-user guild list refresh logic removes kicked guild from rail state.

### Previous Story Intelligence

- Story 8.1 established moderation foundations that should be reused directly:
  - `moderationApi.createMute` request normalization and wire-mapping patterns.
  - `moderation_service::create_mute` input normalization + hierarchy guard pattern.
  - Member-list modal/action UX conventions and toast integration.
- Story 8.1 review fixes should remain intact:
  - Keep existing mute duration options unchanged (1h/24h/1w/custom/permanent).
  - Preserve JS timeout safety fix in `MessageArea.svelte` (do not regress unrelated behavior while adding kick support).

### Git Intelligence Summary

- Most recent commit (`8966e28`) implemented Story 8.1 and touched the exact moderation surfaces needed for 8.2 (models/services/handlers/member UI/tests).
- Existing codebase patterns indicate:
  - New moderation actions are introduced first in service + model, then wired through handler, then surfaced in member UI.
  - Feature completion includes both server and client tests in the same story scope.

### Latest Technical Information

- Axum current guidance emphasizes extractor-based handlers and explicit routing composition; kick endpoint should follow the same handler contract style used in existing moderation handlers.  
  [Source: https://docs.rs/axum/latest/axum/]
- SQLx runtime features remain explicit (`runtime-tokio` etc.), and async APIs require runtime features enabled; keep parity with existing Cargo/sqlx setup when adding migration/model operations.  
  [Source: https://docs.rs/sqlx/latest/sqlx/]
- ARIA menu pattern guidance confirms context-menu keyboard entry (`Shift+F10` / Menu key), which should remain valid while replacing placeholder kick action with real destructive flow.  
  [Source: https://www.w3.org/WAI/ARIA/apg/patterns/menu/]
- Browser guidance recommends using UI dialogs intentionally for confirmations; this aligns with implementing explicit kick confirmation dialog rather than implicit single-click destructive action.  
  [Source: https://developer.mozilla.org/en-US/docs/Web/API/Window/confirm]

### Project Context Reference

- No `project-context.md` file was discovered via `**/project-context.md`.
- Context for this story is derived from planning artifacts, current moderation implementation (8.1), sprint tracking, and current runtime code.

### Story Completion Status

- Story context is fully prepared for `dev-story` implementation.
- Story file created at `_bmad-output/implementation-artifacts/8-2-kick-user-from-guild.md`.
- Sprint status target after this workflow: `review`.
- Completion note: Kick workflow implementation includes backend persistence updates, membership/role cleanup, real-time guild access revocation, client UX/state refresh, and regression-test coverage.

### Project Structure Notes

- Keep moderation API expansion inside `handlers/moderation.rs` and `services/moderation_service.rs`; do not scatter kick logic into unrelated handlers.
- Reuse existing member-list moderation action area in `MemberList.svelte`; do not add duplicate kick controls elsewhere.
- Because guild/channel state is cached by guild slug on client, include explicit refresh/invalidations when processing kick updates to avoid stale guild visibility.

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Story 8.2: Kick User from Guild]
- [Source: _bmad-output/planning-artifacts/epics.md#Story 8.1: Mute User (Timed and Permanent)]
- [Source: _bmad-output/planning-artifacts/prd.md#Moderation & Safety]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Journey 4: Moderation Workflow (Rico)]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Context Menu Patterns]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Flow Optimization Principles]
- [Source: _bmad-output/planning-artifacts/architecture.md#Project Structure & Boundaries]
- [Source: _bmad-output/planning-artifacts/architecture.md#Requirements to Structure Mapping]
- [Source: _bmad-output/implementation-artifacts/sprint-status.yaml]
- [Source: _bmad-output/implementation-artifacts/8-1-mute-user-timed-and-permanent.md]
- [Source: server/migrations/0026_create_moderation_actions.sql]
- [Source: server/src/services/moderation_service.rs]
- [Source: server/src/models/moderation.rs]
- [Source: server/src/models/guild_member.rs]
- [Source: server/src/models/role.rs]
- [Source: server/src/handlers/mod.rs]
- [Source: server/src/handlers/moderation.rs]
- [Source: server/src/ws/gateway.rs]
- [Source: server/src/ws/registry.rs]
- [Source: client/src/lib/features/members/MemberList.svelte]
- [Source: client/src/lib/features/members/MemberList.test.ts]
- [Source: client/src/lib/features/moderation/moderationApi.ts]
- [Source: client/src/lib/features/guild/guildStore.svelte.ts]
- [Source: client/src/lib/features/shell/ShellRoute.svelte]
- [Source: https://docs.rs/axum/latest/axum/]
- [Source: https://docs.rs/sqlx/latest/sqlx/]
- [Source: https://www.w3.org/WAI/ARIA/apg/patterns/menu/]
- [Source: https://developer.mozilla.org/en-US/docs/Web/API/Window/confirm]

## Dev Agent Record

### Agent Model Used

GPT-5.3-Codex (model ID: gpt-5.3-codex)

### Debug Log References

- Story key resolved from sprint status: `8-2-kick-user-from-guild`.
- Previous-story intelligence loaded from 8.1 implementation story and recent commit history.
- Architecture, PRD, and UX moderation sections cross-referenced for constraints and UI behavior.

### Completion Notes List

- Implemented backend kick support end to end: schema migration, moderation model constants, membership/role cleanup helpers, `create_kick` service flow, and moderation kick API route.
- Added live kick access revocation via WebSocket registry unsubscribe + `guild_update` event emission from moderation handler.
- Implemented client kick UX with required-reason dialog, API integration, exact success toast text, and guild/member refresh behavior.
- Added client guild-update handling in guild store and safe ShellRoute fallback when active guild becomes inaccessible.
- Added/updated regression tests for backend kick flow, handler validation/envelope behavior, member list kick UX, guild update refresh behavior, and ShellRoute fallback; quality gates passed for client and server.

### File List

- server/migrations/0027_enable_kick_moderation_actions.sql
- server/src/models/moderation.rs
- server/src/models/guild_member.rs
- server/src/models/role.rs
- server/src/services/moderation_service.rs
- server/src/handlers/moderation.rs
- server/src/handlers/mod.rs
- client/src/lib/features/moderation/moderationApi.ts
- client/src/lib/features/members/MemberList.svelte
- client/src/lib/features/members/MemberList.test.ts
- client/src/lib/features/guild/guildStore.svelte.ts
- client/src/lib/features/guild/guildStore.test.ts
- client/src/lib/features/shell/ShellRoute.svelte
- client/src/lib/features/shell/ShellRoute.test.ts
- _bmad-output/implementation-artifacts/8-2-kick-user-from-guild.md
- _bmad-output/implementation-artifacts/sprint-status.yaml

## Senior Developer Review (AI)

### Outcome

Approved after fixes.

### Findings and Resolutions

- **[HIGH][Fixed]** Kick execution in `create_kick` was not atomic: moderation action insert, role cleanup, and membership removal happened in separate operations, allowing partial persistence on failure/race paths.  
  **Fix:** Added transactional `moderation::apply_kick_action` in `server/src/models/moderation.rs` and switched `create_kick` to use it so mute deactivation, role cleanup, membership delete, and kick audit insert commit or roll back together.
- **[HIGH][Fixed]** Kick hierarchy checks were vulnerable to time-of-check/time-of-use races because role hierarchy validation happened before entering the transactional kick mutation path.  
  **Fix:** Added hierarchy/ownership revalidation inside `moderation::apply_kick_action` transaction for both Postgres and SQLite so role changes between pre-check and mutation cannot bypass `actor_outranks_target_member` constraints.

### Verification

- `cd server && cargo test create_kick && cargo test apply_kick_action_rolls_back_when_membership_delete_fails`
- `cd server && cargo test apply_kick_action_revalidates_hierarchy_within_transaction`
- `cd server && cargo fmt --check && cargo clippy -- -D warnings && cargo test`

## Change Log

- 2026-03-02: Senior AI code review found and fixed non-atomic kick persistence in `create_kick` by introducing transactional `moderation::apply_kick_action`.
- 2026-03-02: Added rollback regression test `apply_kick_action_rolls_back_when_membership_delete_fails` in `server/src/services/moderation_service.rs`.
- 2026-03-02: Revalidated kick hierarchy inside transaction to close TOCTOU race windows, with regression test `apply_kick_action_revalidates_hierarchy_within_transaction`.
- 2026-03-02: Story status updated to `done` and sprint status synced for `8-2-kick-user-from-guild`.
