# Story 8.3: Ban User from Guild

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a **moderator**,
I want to ban a user from the guild,
so that I can permanently remove them and prevent re-entry with the same identity.

## Acceptance Criteria

1. **Given** a user has `BAN_MEMBERS` permission  
   **When** they right-click a member -> Ban from guild  
   **Then** a confirmation dialog appears with the user's name, a required reason field, and an option to delete recent messages (`none` / `1 hour` / `24 hours` / `7 days`).

2. **Given** a moderator confirms ban  
   **When** the operation succeeds  
   **Then** the target user is removed from the guild and their identity is stored in a guild ban list (`guild_bans` migration introduced in this story).

3. **Given** a user is banned from a guild  
   **When** they attempt to join the same guild again with the same cryptographic identity via invite  
   **Then** the join is rejected with an explicit banned response.

4. **Given** a moderator executes a ban action  
   **When** the confirmation dialog is rendered  
   **Then** the destructive action button uses fire red styling (`bg-destructive`).

5. **Given** a moderator attempts to ban another member  
   **When** role hierarchy is evaluated  
   **Then** the target must have a lower highest role than the moderator.

6. **Given** a guild owner opens Guild Settings  
   **When** they navigate to moderation controls  
   **Then** they can view the guild ban list and unban users.

7. **Given** a ban action is executed  
   **When** audit data is recorded  
   **Then** the action is persisted in a format consumable by Story 8.5 moderation log.

## Tasks / Subtasks

- [x] Task 1: Add persistent ban data model and schema support (AC: 2, 3, 7)
  - [x] Add migration `server/migrations/0028_create_guild_bans_and_enable_ban_actions.sql`:
    - create `guild_bans` with `id`, `guild_id`, `target_user_id`, `actor_user_id`, `reason`, `delete_messages_window_seconds` (nullable), `is_active`, `created_at`, `updated_at`, `unbanned_by_user_id` (nullable), `unbanned_at` (nullable)
    - enforce one active ban per guild/user (`UNIQUE(guild_id, target_user_id, is_active)` equivalent)
    - add indexes for active ban checks (`guild_id`, `target_user_id`, `is_active`) and owner ban-list views (`guild_id`, `created_at`)
    - expand `moderation_actions` constraint to include `action_type='ban'` with SQLite/Postgres-compatible table rebuild pattern used in `0027`
  - [x] Add `server/src/models/guild_ban.rs` and export it from `server/src/models/mod.rs`.
  - [x] Extend `server/src/models/moderation.rs` with `MODERATION_ACTION_TYPE_BAN`.

- [x] Task 2: Implement transactional server-side ban workflow (AC: 2, 3, 5, 7)
  - [x] Add `CreateBanInput` / `BanActionResponse` in `server/src/services/moderation_service.rs`.
  - [x] Validate input: required `target_user_id`, required trimmed `reason`, allowed delete-window enum (`none|1h|24h|7d`).
  - [x] Enforce `BAN_MEMBERS` via `permissions::require_guild_permission(..., BAN_MEMBERS, "BAN_MEMBERS")`.
  - [x] Enforce hierarchy and safety: reject self-ban, reject owner-ban, require `permissions::actor_outranks_target_member`.
  - [x] Add transactional model helper (e.g., `moderation::apply_ban_action`) mirroring 8.2 kick hardening:
    - deactivate active mutes for target
    - clear role assignments for target in guild
    - remove guild membership
    - insert active `guild_bans` row
    - append `moderation_actions` record with `action_type='ban'` for mod-log compatibility
  - [x] Keep PG/SQLite behavior parity and rollback semantics consistent with 8.2.

- [x] Task 3: Support optional recent-message deletion window for ban flow (AC: 1, 2)
  - [x] Add message model/service helper(s) to delete target user's guild messages in selected time window (`1h`, `24h`, `7d`) while preserving `none` as default.
  - [x] Ensure related attachments are cleaned up consistently with existing message deletion behavior.
  - [x] Return deleted-message summary in ban response if available (or document explicitly in completion notes when intentionally omitted).

- [x] Task 4: Add moderation ban/unban API endpoints and route wiring (AC: 1, 6, 7)
  - [x] Extend `server/src/handlers/moderation.rs`:
    - `POST /api/v1/guilds/{guild_slug}/moderation/bans`
    - `GET /api/v1/guilds/{guild_slug}/moderation/bans`
    - `DELETE /api/v1/guilds/{guild_slug}/moderation/bans/{ban_id}` (unban)
  - [x] Register routes in `server/src/handlers/mod.rs`.
  - [x] Preserve response conventions: `{ "data": ... }` envelope and `snake_case` payload fields.
  - [x] Emit `guild_update` websocket event with `action_type: "ban"` and unsubscribe the banned user's connection from the guild.

- [x] Task 5: Prevent banned identities from rejoining via invite (AC: 3)
  - [x] Add active-ban guard in `server/src/models/guild_invite.rs::join_guild_via_invite` before membership insert.
  - [x] Ensure single-use invite counters are not consumed when join is denied due to active ban.
  - [x] Return explicit join error message from `server/src/services/guild_invite_service.rs` (not generic invalid-invite text).

- [x] Task 6: Replace member-list ban placeholder with real UX flow (AC: 1, 4, 5)
  - [x] Extend `client/src/lib/features/moderation/moderationApi.ts` with:
    - `createBan`
    - optional `listBans` / `unban` calls shared with Guild Settings
  - [x] Update `client/src/lib/features/members/MemberList.svelte` to replace `"Ban member (coming soon)"` with:
    - destructive confirmation dialog
    - required reason field
    - delete-message window selector (`none|1h|24h|7d`)
    - fire red confirm button (`bg-destructive`)
  - [x] Show success toast: `"User banned from guild"` and refresh guild/member state.
  - [x] Preserve keyboard context-menu accessibility (`ContextMenu`, `Shift+F10`).

- [x] Task 7: Add guild-owner ban list and unban management in settings (AC: 6)
  - [x] Extend guild types/store/api in:
    - `client/src/lib/features/guild/types.ts`
    - `client/src/lib/features/guild/guildApi.ts`
    - `client/src/lib/features/guild/guildStore.svelte.ts`
  - [x] Add a ban-management section in `client/src/lib/features/guild/GuildSettings.svelte` for owners:
    - list banned users + reason + actor + created timestamp
    - unban action with destructive confirmation
  - [x] Refresh ban list after ban/unban actions and keep error handling explicit.

- [x] Task 8: Add regression coverage for ban flow (AC: all)
  - [x] Server unit tests in `server/src/services/moderation_service.rs` for permission checks, hierarchy guards, self/owner protection, transactional rollback, and ban persistence.
  - [x] Handler tests in `server/src/handlers/moderation.rs` for ban payload validation and response envelope shape.
  - [x] Invite-join regression tests in `server/tests/server_binds_to_configured_port.rs` for banned-user rejoin rejection and invite-use semantics.
  - [x] Client tests in `client/src/lib/features/members/MemberList.test.ts` for ban dialog validation, delete-window payload mapping, and destructive styling.
  - [x] Client tests in `client/src/lib/features/guild/GuildSettings.test.ts` and `guildStore.test.ts` for ban list rendering and unban refresh behavior.
  - [x] Run quality gates:
    - [x] `cd client && npm run lint && npm run check && npm run test && npm run build`
    - [x] `cd server && cargo fmt --check && cargo clippy -- -D warnings && cargo test`

## Dev Notes

### Developer Context

- Story 8.1 and 8.2 already established moderation scaffolding:
  - `server/src/models/moderation.rs`
  - `server/src/services/moderation_service.rs`
  - `server/src/handlers/moderation.rs`
  - `client/src/lib/features/moderation/moderationApi.ts`
  - `client/src/lib/features/members/MemberList.svelte`
- `MemberList.svelte` currently shows `Ban member (coming soon)` in the moderation action area; this story should replace that placeholder in-place.
- Invite joins currently run through `guild_invite_service::join_guild_by_invite` and `guild_invite::join_guild_via_invite` without active-ban checks.
- `GuildSettings.svelte` currently focuses on guild profile + roles/permissions and has no ban-list management UI.

### Technical Requirements

- Authorization and hierarchy:
  - Require `BAN_MEMBERS` on the server for ban actions.
  - Enforce `permissions::actor_outranks_target_member` and reject equal/higher-role targets.
  - Reject self-ban and guild-owner ban with explicit errors.
- Ban semantics:
  - Active ban state lives in `guild_bans`; moderation log/audit remains append-only in `moderation_actions`.
  - Ban must block invite-based rejoin for the same `user_id` (cryptographic identity holder).
  - Unban deactivates the active ban record without mutating historical audit entries.
- Request validation:
  - `reason`: required, trimmed, bounded (reuse moderation reason limits/patterns).
  - `delete_message_window`: enum-validated (`none|1h|24h|7d`), no silent coercion.
- UX:
  - Ban confirmation uses destructive color/token (`bg-destructive`) and explicit wording.
  - Context-menu flow remains keyboard-accessible.
- Contracts:
  - Keep `snake_case` API wire fields and `{ "data": ... }` envelope.
  - Preserve explicit error responses (no silent fallbacks).

### Architecture Compliance

1. Keep boundaries strict: handlers parse/serialize, services own business rules, models own SQL.
2. Keep Postgres/SQLite parity for migration and model behavior.
3. Reuse existing moderation and permission helpers; do not duplicate hierarchy logic in handlers.
4. Keep ban flow transactional like hardened 8.2 kick implementation.
5. Preserve websocket-driven guild refresh behavior (`guild_update`) for live client consistency.
6. Keep audit trails append-only; do not overwrite prior moderation entries.

### Library & Framework Requirements

- Backend: Rust + Axum + Tokio + SQLx (existing stack only).
- Frontend: Svelte 5 + TypeScript + existing stores and API utilities.
- No new global state framework or background job scheduler for this story.
- Continue existing routing and extractor conventions used in Axum handlers.

### File Structure Requirements

Expected primary touch points:

- `server/migrations/0028_create_guild_bans_and_enable_ban_actions.sql` (new)
- `server/src/models/mod.rs`
- `server/src/models/moderation.rs`
- `server/src/models/guild_ban.rs` (new)
- `server/src/models/guild_invite.rs`
- `server/src/models/message.rs` (if bulk message window deletion helper is added)
- `server/src/services/moderation_service.rs`
- `server/src/services/guild_invite_service.rs`
- `server/src/handlers/moderation.rs`
- `server/src/handlers/mod.rs`
- `server/tests/server_binds_to_configured_port.rs`
- `client/src/lib/features/moderation/moderationApi.ts`
- `client/src/lib/features/members/MemberList.svelte`
- `client/src/lib/features/members/MemberList.test.ts`
- `client/src/lib/features/guild/types.ts`
- `client/src/lib/features/guild/guildApi.ts`
- `client/src/lib/features/guild/guildStore.svelte.ts`
- `client/src/lib/features/guild/guildStore.test.ts`
- `client/src/lib/features/guild/GuildSettings.svelte`
- `client/src/lib/features/guild/GuildSettings.test.ts`

### Testing Requirements

- Server:
  - Ban rejects missing permission, self-target, owner-target, and equal/higher-role target.
  - Successful ban removes membership, stores active guild ban, and appends moderation action.
  - Banned user cannot rejoin via invite; invite usage semantics remain correct.
  - Unban restores join eligibility without deleting audit history.
  - Optional message-window deletion obeys selected window and does not affect unrelated users.
- Client:
  - Ban dialog validates required reason and delete-window selection mapping.
  - Destructive ban button uses destructive styling and labels.
  - Successful ban dispatches API request and shows expected toast.
  - Guild Settings ban list loads, renders, and unban updates list/state.

### Previous Story Intelligence

- Story 8.2 introduced hardened transactional moderation mutation (`apply_kick_action`) with in-transaction hierarchy revalidation and rollback protection; reuse this pattern for `apply_ban_action`.
- Story 8.2 established websocket guild-update behavior for removed members; ban should follow the same live-refresh/unsubscribe pattern.
- Story 8.1 and 8.2 established moderation UI conventions in `MemberList.svelte` (required reason, explicit confirmation, toast feedback, keyboard-friendly context menu flow).
- Keep existing 8.1/8.2 behavior stable: no regression to mute duration options, mute status timing logic, or kick workflows.

### Git Intelligence Summary

- Recent commits indicate moderation work sequencing and patterns:
  - `ee59039 fix: harden kick transaction review`
  - `a272645 feat: implement story 8.2 kick moderation flow`
  - `8966e28 feat: complete story 8-1 mute moderation`
- Current codebase already has moderation routes/services and should be extended incrementally for ban rather than introducing parallel subsystems.

### Latest Technical Information

- Axum guidance emphasizes extractor-based handlers and Router composition; ban endpoints should follow existing handler patterns and response conversion.  
  [Source: https://docs.rs/axum/latest/axum/]
- SQLx runtime behavior depends on enabled runtime features; keep current runtime/TLS feature choices and parity between SQLite and Postgres query paths.  
  [Source: https://docs.rs/sqlx/latest/sqlx/]
- ARIA menu patterns reinforce keyboard context menu behavior (`Shift+F10`, arrow navigation, Enter activation), which must remain intact for moderation actions.  
  [Source: https://www.w3.org/WAI/ARIA/apg/patterns/menu/]

### Project Context Reference

- No `project-context.md` was discovered via `**/project-context.md`.
- Story context is derived from planning artifacts, current moderation implementation (8.1/8.2), sprint tracking, and current runtime code.

### Story Completion Status

- Story context is fully prepared for `dev-story` implementation.
- Story file created at `_bmad-output/implementation-artifacts/8-3-ban-user-from-guild.md`.
- Sprint status target after this workflow: `ready-for-dev`.
- Completion note: Comprehensive ban workflow guidance prepared, including persistence, API, join-blocking, UI, and testing guardrails.

### Project Structure Notes

- Keep moderation API expansion centralized in `handlers/moderation.rs` and `services/moderation_service.rs`.
- Extend existing member context action surfaces (`MemberList.svelte`) instead of creating parallel ban entry points.
- Keep invite rejoin enforcement in invite join flow (`guild_invite_service`/`guild_invite`), not in ad-hoc caller checks.
- Keep guild-owner ban management in `GuildSettings.svelte` to match AC6 and current settings information architecture.

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Epic 8: Moderation, Reporting & Data Privacy]
- [Source: _bmad-output/planning-artifacts/epics.md#Story 8.3: Ban User from Guild]
- [Source: _bmad-output/planning-artifacts/prd.md#Moderation & Safety]
- [Source: _bmad-output/planning-artifacts/architecture.md#Project Structure & Boundaries]
- [Source: _bmad-output/planning-artifacts/architecture.md#Cross-Cutting Concerns Mapping]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Journey 4: Moderation Workflow (Rico)]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Context Menu Rules]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Color Usage Rules]
- [Source: _bmad-output/implementation-artifacts/sprint-status.yaml]
- [Source: _bmad-output/implementation-artifacts/8-1-mute-user-timed-and-permanent.md]
- [Source: _bmad-output/implementation-artifacts/8-2-kick-user-from-guild.md]
- [Source: server/migrations/0026_create_moderation_actions.sql]
- [Source: server/migrations/0027_enable_kick_moderation_actions.sql]
- [Source: server/src/services/moderation_service.rs]
- [Source: server/src/models/moderation.rs]
- [Source: server/src/models/guild_invite.rs]
- [Source: server/src/services/guild_invite_service.rs]
- [Source: server/src/handlers/moderation.rs]
- [Source: server/src/handlers/mod.rs]
- [Source: server/src/permissions/mod.rs]
- [Source: client/src/lib/features/members/MemberList.svelte]
- [Source: client/src/lib/features/members/MemberList.test.ts]
- [Source: client/src/lib/features/moderation/moderationApi.ts]
- [Source: client/src/lib/features/guild/GuildSettings.svelte]
- [Source: client/src/lib/features/guild/GuildSettings.test.ts]
- [Source: client/src/lib/features/guild/guildStore.svelte.ts]
- [Source: client/src/lib/features/guild/guildStore.test.ts]
- [Source: client/src/lib/features/guild/guildApi.ts]
- [Source: server/tests/server_binds_to_configured_port.rs]
- [Source: https://docs.rs/axum/latest/axum/]
- [Source: https://docs.rs/sqlx/latest/sqlx/]
- [Source: https://www.w3.org/WAI/ARIA/apg/patterns/menu/]

## Dev Agent Record

### Agent Model Used

GPT-5.3-Codex (model ID: gpt-5.3-codex)

### Debug Log References

- Story key resolved from sprint status: `8-3-ban-user-from-guild`.
- Baseline quality gates passed before implementation (`client` lint/check/test/build and `server` fmt/clippy/test).
- Post-change quality gates passed after ban/unban implementation and regression coverage updates.
- Invite rejoin semantics for already-member single-use links were preserved while enforcing banned-user rejection.

### Completion Notes List

- Implemented guild ban persistence and moderation action support for `action_type='ban'` with migration + model wiring.
- Added transactional ban flow with hierarchy/safety enforcement, optional message-window deletion, attachment cleanup, and moderation audit persistence.
- Added moderation ban APIs (`create/list/unban`), websocket guild updates for ban/unban, and invite-join banned identity enforcement.
- Replaced member-list ban placeholder with destructive ban dialog (required reason + delete-window), success toast, and state refresh.
- Added guild-settings owner ban list and unban controls with store/api/type wiring and cache invalidation.
- Added/updated server and client regression tests for ban flow, invite semantics, ban list management, and API mapping; all project quality gates passed.

### File List

- server/migrations/0028_create_guild_bans_and_enable_ban_actions.sql
- server/src/models/mod.rs
- server/src/models/guild_ban.rs
- server/src/models/moderation.rs
- server/src/models/message.rs
- server/src/models/guild_invite.rs
- server/src/services/moderation_service.rs
- server/src/services/guild_invite_service.rs
- server/src/handlers/moderation.rs
- server/src/handlers/mod.rs
- server/tests/server_binds_to_configured_port.rs
- client/src/lib/features/moderation/moderationApi.ts
- client/src/lib/features/members/MemberList.svelte
- client/src/lib/features/members/MemberList.test.ts
- client/src/lib/features/guild/types.ts
- client/src/lib/features/guild/guildApi.ts
- client/src/lib/features/guild/guildApi.test.ts
- client/src/lib/features/guild/guildStore.svelte.ts
- client/src/lib/features/guild/guildStore.test.ts
- client/src/lib/features/guild/GuildSettings.svelte
- client/src/lib/features/guild/GuildSettings.test.ts
- _bmad-output/implementation-artifacts/8-3-ban-user-from-guild.md
- _bmad-output/implementation-artifacts/sprint-status.yaml

## Senior Developer Review (AI)

### Reviewer

- Darko (AI) — 2026-03-02

### Findings & Fixes

- HIGH: `server/src/models/guild_invite.rs` committed transactions on banned invite-join rejection paths before returning `Ok(None)`; removed those commits so rejection exits without committing the transaction in both Postgres and SQLite paths.
- HIGH: `server/src/services/moderation_service.rs::create_ban` returned an error when post-commit message cleanup failed, causing false-negative ban failures; converted cleanup to logged best-effort so committed bans still return success and added regression coverage.

### Validation

- `cd server && cargo fmt --check`
- `cd server && cargo test invite_join_rejects_banned_identity_without_consuming_single_use_invite`
- `cd server && cargo test create_ban_delete_window_removes_recent_messages_and_attachments`
- `cd server && cargo test create_ban_succeeds_when_message_cleanup_fails_after_commit`

### Outcome

- Approved after fix; all identified HIGH/MEDIUM review findings are resolved.

## Change Log

- 2026-03-02: Implemented Story 8.3 ban workflow end-to-end across migration/models/services/handlers, invite enforcement, and member/guild settings UX.
- 2026-03-02: Added regression tests for server moderation + invite rejoin ban semantics and client ban dialog/ban list behaviors.
- 2026-03-02: Ran quality gates successfully (`cd client && npm run lint && npm run check && npm run test && npm run build`; `cd server && cargo fmt --check && cargo clippy -- -D warnings && cargo test`) and moved story to `review`.
- 2026-03-02: Senior review fixed banned invite-join transaction handling in `server/src/models/guild_invite.rs` and moved story to `done`.
- 2026-03-02: Senior review fixed ban success/error mismatch by handling post-commit message cleanup failures as warned best-effort and added regression coverage.
