# Story 5.5: Channel-Level Permission Overrides

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a **guild owner**,
I want to set permission overrides for specific roles on specific channels,
so that I can create private channels or restrict actions per channel.

## Acceptance Criteria

1. **Given** the user has `MANAGE_CHANNELS` permission or is the guild owner  
   **When** they open channel settings for a specific channel  
   **Then** they see a **Permission Overrides** section listing roles

2. **Given** a role is selected in the overrides UI  
   **When** the user configures permission behavior  
   **Then** for each permission they can choose one of three states: **Allow / Deny / Inherit**

3. **Given** this story migration runs  
   **When** persistence is initialized  
   **Then** a `channel_permission_overrides` table exists with columns: `channel_id`, `role_id`, `allow_bitflag`, `deny_bitflag`

4. **Given** channel overrides are evaluated  
   **When** effective channel permissions are computed  
   **Then** they follow: `(role permissions | channel allow) & ~channel deny`

5. **Given** `@everyone` is denied on `VIEW` for a channel  
   **When** channel visibility is resolved  
   **Then** only explicitly allowed roles can view the channel (private channel behavior)

6. **Given** override entries are created, updated, or removed  
   **When** persistence succeeds  
   **Then** permission cache entries for affected guild members are invalidated immediately

7. **Given** override controls are shown in channel settings  
   **When** users interact with the three-state toggles  
   **Then** the UI clearly distinguishes allow/deny/inherit states and preserves keyboard accessibility

## Tasks / Subtasks

- [x] Task 1: Add channel override persistence and data-access layer (AC: 3, 4, 6)
  - [x] Add `server/migrations/0017_create_channel_permission_overrides.sql` with table, PK `(channel_id, role_id)`, FK constraints, and supporting indexes.
  - [x] Add `server/src/models/channel_permission_override.rs` with SQLite/Postgres parity for list/upsert/delete operations per channel and role.
  - [x] Wire new model module in `server/src/models/mod.rs`.

- [x] Task 2: Extend permission engine with channel-aware evaluation (AC: 4, 5, 6)
  - [x] Add channel-level permission resolution in `server/src/permissions/mod.rs` that applies override allow/deny masks on top of effective guild role permissions.
  - [x] Define and document `VIEW` semantics for private channels in code and tests (recommended: explicit `VIEW_CHANNEL` bit handling and compatibility backfill strategy).
  - [x] Ensure override mutations trigger immediate cache invalidation for affected members in the guild.

- [x] Task 3: Add backend API surface for channel overrides (AC: 1, 2, 6)
  - [x] Register routes in `server/src/handlers/mod.rs` under `/api/v1/guilds/{guild_slug}/channels/{channel_slug}/permission-overrides`.
  - [x] Add request/response DTOs and handlers in `server/src/handlers/channels.rs` (or a dedicated channel-permissions handler module if split).
  - [x] Enforce `MANAGE_CHANNELS` authorization with existing service guard patterns and validate guild/channel/role ownership boundaries.

- [x] Task 4: Apply override enforcement to channel visibility and operations (AC: 4, 5, 6)
  - [x] Update channel-read logic in `server/src/services/channel_service.rs` to return only channels viewable under computed effective channel permissions.
  - [x] Ensure permission-sensitive channel actions use channel-aware effective permissions rather than guild-only checks where appropriate.
  - [x] Preserve owner implicit access semantics and existing role hierarchy constraints.

- [x] Task 5: Build channel settings override UX with tri-state controls (AC: 1, 2, 7)
  - [x] Extend channel management UI in `client/src/lib/features/channel/ChannelList.svelte` (or extracted settings component) with a **Permission Overrides** section.
  - [x] Render role rows with tri-state controls per permission (allow/deny/inherit) and clear visual state.
  - [x] Keep keyboard and context-menu accessibility parity (`Shift+F10` / `Menu` key workflows, menu navigation, focus management).

- [x] Task 6: Add client channel override API/store/type plumbing (AC: 2, 7)
  - [x] Extend `client/src/lib/features/channel/types.ts` with channel-override wire/domain types and mappers.
  - [x] Add override endpoints in `client/src/lib/features/channel/channelApi.ts`.
  - [x] Extend `client/src/lib/features/channel/channelStore.svelte.ts` with load/update override actions and failure rollback patterns consistent with existing store behavior.

- [x] Task 7: Add coverage and run quality gates (AC: all)
  - [x] Extend `server/tests/server_binds_to_configured_port.rs` for 401/403/422 checks, override persistence, private-channel visibility behavior, and cache invalidation behavior.
  - [x] Add backend unit tests for channel-permission computation and override validation edge cases in `server/src/permissions/mod.rs` and model tests.
  - [x] Add frontend tests in `client/src/lib/features/channel/ChannelList.test.ts` (or new settings test file) for tri-state UI behavior, keyboard access, and save/error flows.
  - [x] Run quality gates:
    - [x] `cd client && npm run lint && npm run check && npm run test && npm run build`
    - [x] `cd server && cargo fmt --check && cargo clippy -- -D warnings && cargo test`

## Dev Notes

### Developer Context

- Stories 5.1-5.4 delivered role CRUD, permission bitflags, role hierarchy, and member-role assignment with strict outrank semantics and cache invalidation.
- Current backend has guild-level permission evaluation in `server/src/permissions/mod.rs`, but no channel-specific override persistence or channel-level permission computation yet.
- Current channel mutations already use `MANAGE_CHANNELS` via `permissions::require_guild_permission`, and channel/category read paths are member-visible (`can_view_guild`) without per-channel filtering.
- Current channel UI (`client/src/lib/features/channel/ChannelList.svelte`) supports create/edit/delete/reorder and contextual actions, but has no override editor section.

### Technical Requirements

- Persist channel override state in a dedicated table keyed by `(channel_id, role_id)` and keep SQL parity across SQLite/Postgres.
- Enforce `MANAGE_CHANNELS` (or owner implicit access) for all override mutations server-side; never rely on client-only gating.
- Compute effective channel permissions with the canonical formula from AC4 and apply it consistently where channel visibility/action checks are performed.
- Explicitly implement private-channel visibility semantics for `VIEW` override behavior (AC5) and document how this maps to the permission bitflag catalog.
- Invalidate cached effective permissions immediately after override writes so access decisions reflect changes without restart.
- Maintain API response envelope contracts: success `{ "data": ... }`, errors `{ "error": { "code", "message", "details" } }`.

### Architecture Compliance

1. Keep route wiring in handlers, business rules in services, and SQL in models (no SQL in handlers/services).
2. Reuse `permissions/` as the single authorization/permission computation layer.
3. Preserve existing HTTP and `AppError` semantics (`401/403/422` patterns already used by role/channel services).
4. Keep frontend work within existing feature boundaries (`features/channel`, `features/guild`, shared API/types helpers).
5. Maintain deterministic, regression-safe behavior for existing channel list/order/category flows while adding overrides.

### Library & Framework Requirements

- Frontend: Svelte 5 runes, existing feature-store patterns, and current channel UI architecture.
- Backend: Axum 0.8 + sqlx 0.8 with current model/service conventions.
- No new framework/library adoption is required for Story 5.5.

### File Structure Requirements

Expected primary touch points:

- `server/migrations/0017_create_channel_permission_overrides.sql` (new)
- `server/src/models/mod.rs`
- `server/src/models/channel_permission_override.rs` (new)
- `server/src/permissions/mod.rs`
- `server/src/services/channel_service.rs`
- `server/src/handlers/channels.rs`
- `server/src/handlers/mod.rs`
- `server/tests/server_binds_to_configured_port.rs`
- `client/src/lib/features/channel/types.ts`
- `client/src/lib/features/channel/channelApi.ts`
- `client/src/lib/features/channel/channelStore.svelte.ts`
- `client/src/lib/features/channel/ChannelList.svelte`
- `client/src/lib/features/channel/ChannelList.test.ts`
- `client/src/lib/features/guild/permissions.ts`

### Testing Requirements

- Server integration tests:
  - unauthenticated channel-override mutations return `401`,
  - callers lacking `MANAGE_CHANNELS` return `403`,
  - invalid payloads (unknown role/channel, invalid bitflags, cross-guild role IDs) return `422`,
  - successful override writes persist and are returned deterministically,
  - private-channel visibility behavior matches AC5,
  - cache invalidation is observable immediately after override updates.
- Server unit tests:
  - channel effective-permission computation (allow/deny/inherit combinations),
  - `VIEW` override behavior and strict precedence,
  - bitflag validation for override allow/deny masks.
- Frontend tests:
  - channel settings render override roles and tri-state controls,
  - toggle sequences emit expected payload transitions (allow ↔ deny ↔ inherit),
  - keyboard/context-menu access works and remains accessible,
  - failed saves surface actionable error messages and rollback local optimistic state.

### Previous Story Intelligence

- Story 5.4 established role-assignment API patterns (`/guilds/{guild_slug}/members/{member_user_id}/roles`) and strict authority enforcement using `highest_guild_role_authority` + `actor_outranks_target_member`.
- Story 5.3 established deterministic role ordering, reorder endpoint patterns, and guild permission cache invalidation on hierarchy mutations.
- Story 5.2 introduced the canonical bitflag permission catalog and `require_guild_permission` helper used by channel/category/invite/guild mutation services.
- Existing channel/context-menu UI and tests provide baseline accessibility and interaction patterns that override UI should extend rather than replace.

### Git Intelligence Summary

- `68f87aa feat: finalize story 5-4 role assignment to members` adds member-role APIs, hierarchy guards, and cache invalidation behavior that channel overrides should mirror.
- `f9149f0 feat: finalize story 5-3 role hierarchy and ordering` provides reorder validation patterns and role authority helpers.
- `1f41931 feat: finalize story 5-2 permission bitflag engine` establishes canonical permission constants and service-level guard helpers.
- `aaf0082 feat: finalize story 5-1 role crud and default roles` introduced role/role_assignment persistence foundations.
- `00d3274 feat: finalize story 4-7 guild navigation and activity indicators` confirms current shell/channel/member panel interaction baseline.

### Latest Technical Information

Current pinned runtime lines in this repo:
- `svelte`: `^5.45.2`
- `@mateothegreat/svelte5-router`: `^2.16.19`
- `axum`: `0.8`
- `sqlx`: `0.8`
- `libp2p`: `0.56`

Latest stable lines checked during story creation:
- Svelte: `5.53.6`
- `@mateothegreat/svelte5-router`: `2.16.19`
- Axum: `axum-v0.8.8`
- SQLx: `v0.8.6`
- rust-libp2p: `libp2p-v0.56.0`

No dependency upgrade is required to implement Story 5.5; implement against currently pinned project versions.

### Project Context Reference

- No `project-context.md` was discovered via `**/project-context.md`.
- Story context is derived from planning artifacts, Stories 5.1-5.4 implementation artifacts, current runtime code, recent commit history, and current upstream version checks.

### Story Completion Status

- Ultimate context engine analysis completed - comprehensive developer guide created.
- Story status set to `ready-for-dev`.

### Project Structure Notes

- Architecture planning mentions `features/members/RoleManager.svelte`, but current runtime channel administration is centered in `features/channel/ChannelList.svelte`; Story 5.5 should extend existing channel management surfaces.
- Existing guild permission checks are guild-scoped and cache keyed by `(guild_id, user_id)`; Story 5.5 must introduce channel-level evaluation without regressing established guild-level behavior.

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Epic 5: Roles, Permissions & Member Management]
- [Source: _bmad-output/planning-artifacts/epics.md#Story 5.5: Channel-Level Permission Overrides]
- [Source: _bmad-output/planning-artifacts/epics.md#Story 5.4: Role Assignment to Members]
- [Source: _bmad-output/planning-artifacts/prd.md#Roles & Permissions]
- [Source: _bmad-output/planning-artifacts/prd.md#Security]
- [Source: _bmad-output/planning-artifacts/architecture.md#Authentication & Security]
- [Source: _bmad-output/planning-artifacts/architecture.md#REST API Response Format]
- [Source: _bmad-output/planning-artifacts/architecture.md#Error Handling]
- [Source: _bmad-output/planning-artifacts/architecture.md#Requirements to Structure Mapping]
- [Source: _bmad-output/planning-artifacts/architecture.md#Cross-Cutting Concerns Mapping]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#MemberListEntry]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#User context menu (in member list or on username)]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Context menu rules]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Desktop (≥1024px) — Primary experience]
- [Source: _bmad-output/implementation-artifacts/5-4-role-assignment-to-members.md]
- [Source: _bmad-output/implementation-artifacts/5-3-role-hierarchy-and-ordering.md]
- [Source: _bmad-output/implementation-artifacts/5-2-permission-assignment-and-bitflag-engine.md]
- [Source: _bmad-output/implementation-artifacts/5-1-role-crud-and-default-roles.md]
- [Source: _bmad-output/implementation-artifacts/sprint-status.yaml]
- [Source: server/src/handlers/mod.rs]
- [Source: server/src/handlers/channels.rs]
- [Source: server/src/handlers/roles.rs]
- [Source: server/src/services/channel_service.rs]
- [Source: server/src/services/role_service.rs]
- [Source: server/src/services/category_service.rs]
- [Source: server/src/models/channel.rs]
- [Source: server/src/models/role.rs]
- [Source: server/src/permissions/mod.rs]
- [Source: server/migrations/0011_create_channels.sql]
- [Source: server/migrations/0012_create_channel_categories.sql]
- [Source: server/migrations/0015_create_roles_and_role_assignments.sql]
- [Source: server/tests/server_binds_to_configured_port.rs]
- [Source: client/src/lib/features/channel/ChannelList.svelte]
- [Source: client/src/lib/features/channel/ChannelList.test.ts]
- [Source: client/src/lib/features/channel/channelApi.ts]
- [Source: client/src/lib/features/channel/channelStore.svelte.ts]
- [Source: client/src/lib/features/channel/types.ts]
- [Source: client/src/lib/features/guild/GuildSettings.svelte]
- [Source: client/src/lib/features/guild/GuildSettings.test.ts]
- [Source: client/src/lib/features/guild/guildApi.ts]
- [Source: client/src/lib/features/guild/guildStore.svelte.ts]
- [Source: client/src/lib/features/guild/permissions.ts]
- [Source: client/src/lib/features/guild/types.ts]
- [Source: client/package.json]
- [Source: server/Cargo.toml]
- [Source: https://registry.npmjs.org/svelte/latest]
- [Source: https://registry.npmjs.org/@mateothegreat/svelte5-router/latest]
- [Source: https://api.github.com/repos/tokio-rs/axum/releases/latest]
- [Source: https://api.github.com/repos/launchbadge/sqlx/tags?per_page=5]
- [Source: https://api.github.com/repos/libp2p/rust-libp2p/releases/latest]

## Dev Agent Record

### Agent Model Used

GPT-5.3-Codex (model ID: gpt-5.3-codex)

### Debug Log References

- Workflow engine loaded: `_bmad/core/tasks/workflow.xml`
- Workflow config loaded: `_bmad/bmm/workflows/4-implementation/dev-story/workflow.yaml`
- Story source parsed from user input: `5-5` -> `5-5-channel-level-permission-overrides`
- Quality gates run and passing:
  - `cd client && npm run lint && npm run check && npm run test && npm run build`
  - `cd server && cargo fmt --check && cargo clippy -- -D warnings && cargo test`

### Completion Notes List

- Implemented channel permission override persistence, model wiring, API routes/handlers, and channel-aware visibility checks with strict allow/deny precedence.
- Added `VIEW_CHANNEL` permission bit plus migration backfill to keep default/effective role permissions aligned with private channel behavior.
- Added frontend channel override API/store/type plumbing and a tri-state (Allow/Deny/Inherit) permission override editor in channel actions.
- Added backend and frontend test coverage for override flows, validation, private visibility behavior, and cache invalidation behavior.

### File List

- server/migrations/0017_create_channel_permission_overrides.sql
- server/src/models/channel_permission_override.rs
- server/src/models/mod.rs
- server/src/permissions/mod.rs
- server/src/services/channel_service.rs
- server/src/handlers/channels.rs
- server/src/handlers/mod.rs
- server/src/services/role_service.rs
- server/tests/server_binds_to_configured_port.rs
- client/src/lib/features/channel/types.ts
- client/src/lib/features/channel/channelApi.ts
- client/src/lib/features/channel/channelStore.svelte.ts
- client/src/lib/features/channel/channelStore.test.ts
- client/src/lib/features/channel/ChannelList.svelte
- client/src/lib/features/channel/ChannelList.test.ts
- client/src/lib/features/guild/permissions.ts
- client/src/lib/features/guild/GuildSettings.test.ts
- client/src/lib/features/members/MemberList.test.ts
- _bmad-output/implementation-artifacts/5-5-channel-level-permission-overrides.md
- _bmad-output/implementation-artifacts/sprint-status.yaml

## Senior Developer Review (AI)

### Reviewer

Darko (AI-assisted) — 2026-02-28

### Findings Summary

- Medium: Channel action permissions were owner-only in the UI; fixed by deriving `MANAGE_CHANNELS` capability from current member role bitflags.
- Low: Keyboard context-menu parity (`Shift+F10` / `ContextMenu`) was missing for channel actions; fixed.

### Fixes Applied

- Updated `client/src/lib/features/channel/ChannelList.svelte` to compute `canManageChannels` from owner or role-derived `MANAGE_CHANNELS`, and added keyboard context-menu handlers for channel/category action menus.
- Updated `client/src/lib/features/channel/ChannelList.test.ts` to cover non-owner `MANAGE_CHANNELS` access and keyboard shortcut behavior.

### Validation

- `cd client && npm run test -- ChannelList.test.ts` ✅
- Adversarial re-review of uncommitted source changes: no remaining findings.

### Outcome

- Approved after fixes; no open HIGH/MEDIUM findings.

## Change Log

- 2026-02-28: Senior AI review completed and follow-up fixes applied for channel action gating + keyboard context-menu accessibility.
