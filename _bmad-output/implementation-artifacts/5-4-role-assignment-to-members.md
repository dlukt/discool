# Story 5.4: Role Assignment to Members

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a **guild owner**,
I want to assign and remove roles from guild members,
so that members get the appropriate permissions for their responsibilities.

## Acceptance Criteria

1. **Given** the user has `MANAGE_ROLES` permission or is the guild owner  
   **When** they right-click a member in the member list or open their profile popover  
   **Then** an **Assign role** option is available showing all roles below the assigner's highest role

2. **Given** a permitted assigner opens role assignment  
   **When** they toggle roles on/off for a target member  
   **Then** assignments are persisted and reflected immediately

3. **Given** role assignments are changed  
   **When** persistence succeeds  
   **Then** computed permission cache entries for the target user are invalidated so effective permissions update immediately

4. **Given** assignments are written  
   **When** storage is queried  
   **Then** role links are stored in `role_assignments` for the `(guild_id, user_id, role_id)` tuple

5. **Given** assignments change a member's highest role  
   **When** member and chat identity surfaces render  
   **Then** username color reflects the member's highest-role color

6. **Given** a non-owner assigner attempts to assign protected or high-authority roles  
   **When** the server validates the request  
   **Then** roles equal to or above the assigner's highest role are rejected server-side

## Tasks / Subtasks

- [x] Task 1: Add backend role-assignment APIs and request validation (AC: 1, 2, 6)
  - [x] Add guild-member role assignment routes under `/api/v1/guilds/{guild_slug}/...` in `server/src/handlers/mod.rs` (recommended shape: member-role list + member-role update/toggle operations).
  - [x] Add handler DTOs and validation in `server/src/handlers/roles.rs` (or a dedicated members handler module if split during implementation).
  - [x] Reject malformed payloads and unknown role IDs with existing `ValidationError` patterns.

- [x] Task 2: Implement assignment business rules and hierarchy guards in services (AC: 1, 2, 6)
  - [x] Reuse `permissions::require_guild_permission(..., MANAGE_ROLES, ...)` for delegated role managers while preserving owner implicit access.
  - [x] Reuse `permissions::highest_guild_role_authority` and `permissions::actor_outranks_target_member` for strict outrank checks.
  - [x] Enforce assignable set constraints: no owner pseudo-role, no `@everyone`, and no role equal-to-or-above actor authority.
  - [x] Keep assignment toggles idempotent (re-applying existing assignment does not error).

- [x] Task 3: Extend role/guild-member model queries for assignment workflows (AC: 2, 4, 6)
  - [x] Add role assignment read/write helpers in `server/src/models/role.rs` for listing assigned role IDs per member and inserting/deleting assignments safely across SQLite/Postgres.
  - [x] Add guild-member listing/query helpers in `server/src/models/guild_member.rs` as needed for assignment targets in current guild scope.
  - [x] Keep transactional behavior for multi-toggle updates so partial writes do not leak.

- [x] Task 4: Invalidate permission cache and return deterministic updated state (AC: 2, 3)
  - [x] Invalidate cached effective permissions immediately after assignment mutation.
  - [x] Return updated member role state in deterministic role-order form so UI can render without stale recomputation.
  - [x] Preserve existing response envelopes (`{ "data": ... }`) and error envelope contracts.

- [x] Task 5: Implement member interaction UI for assign/remove roles (AC: 1, 2, 5)
  - [x] Replace the current `MemberList` placeholder with API-backed guild member rows in `client/src/lib/features/members/MemberList.svelte`.
  - [x] Add user context-menu/profile-popover action entry `Assign role` with role toggles filtered by assigner authority.
  - [x] Surface highest-role username colors in member list entries and wire color data to existing chat identity surfaces.
  - [x] Keep keyboard accessibility for context menus (Shift+F10/Menu key navigation and Enter activation).

- [x] Task 6: Add coverage and run quality gates (AC: all)
  - [x] Extend `server/tests/server_binds_to_configured_port.rs` for auth (`401`), forbidden (`403`), hierarchy violations (`422/403`), persistence success, and cache invalidation behavior.
  - [x] Add frontend tests for member context menu/profile popover assignment flows, filtered role visibility, and optimistic rollback/failure messaging.
  - [x] Run quality gates:
    - [x] `cd client && npm run lint && npm run check && npm run test && npm run build`
    - [x] `cd server && cargo fmt --check && cargo clippy -- -D warnings && cargo test`

## Dev Notes

### Developer Context

- Story 5.1 introduced `roles` + `role_assignments` tables and role CRUD APIs, and Story 5.2/5.3 established permission bitflags, `MANAGE_ROLES` service guards, hierarchy helpers, and role reorder behavior.
- Current runtime already computes effective permissions from default role + assigned roles in `permissions::effective_guild_permissions`, but there is no public API yet to assign/unassign roles per member.
- Current member UI is still a static placeholder (`client/src/lib/features/members/MemberList.svelte`) and must be upgraded for assignment interactions.
- Existing role mutation pattern and test style in `server/tests/server_binds_to_configured_port.rs` should be extended rather than replaced.

### Technical Requirements

- Assignment authorization must allow guild owner and delegated managers with `MANAGE_ROLES`.
- Delegated managers can only assign/remove roles strictly below their highest role authority.
- Assignment operations must validate that target users are guild members and role IDs belong to the same guild.
- Assignment changes must invalidate permission cache entries so permission-sensitive actions reflect new role state immediately.
- Username color selection must follow highest role precedence and fall back to default role color when no custom role applies.
- Keep strict server-side enforcement even if client filtering hides disallowed roles.

### Architecture Compliance

1. Keep Axum route registration in `server/src/handlers/mod.rs`, request parsing/validation in handlers, business rules in services, and SQL only in models.
2. Preserve response/error contracts (`{ "data": ... }` and `AppError` JSON error shape) and existing HTTP status semantics.
3. Maintain Postgres + SQLite parity for all assignment queries/mutations.
4. Reuse `permissions/` as the centralized authorization and hierarchy authority layer.
5. Keep frontend work inside current feature boundaries (`features/members` + existing guild API/store modules) and avoid introducing parallel state systems.

### Library & Framework Requirements

- Frontend: Svelte 5 runes + existing feature-state patterns.
- Backend: Axum 0.8 + sqlx 0.8 existing query and service patterns.
- No new framework/library adoption is required for Story 5.4.

### File Structure Requirements

Expected primary touch points:

- `server/src/handlers/mod.rs`
- `server/src/handlers/roles.rs` (or a dedicated member-role handler module if introduced)
- `server/src/services/role_service.rs`
- `server/src/models/role.rs`
- `server/src/models/guild_member.rs`
- `server/src/permissions/mod.rs`
- `server/tests/server_binds_to_configured_port.rs`
- `client/src/lib/features/members/MemberList.svelte`
- `client/src/lib/features/members/MemberList.test.ts` (new)
- `client/src/lib/features/guild/types.ts`
- `client/src/lib/features/guild/guildApi.ts`
- `client/src/lib/features/guild/guildStore.svelte.ts`
- `client/src/lib/features/shell/ShellRoute.svelte`
- `client/src/lib/features/chat/MessageArea.svelte` (if color rendering surfaces are added there)

### Testing Requirements

- Server integration tests:
  - unauthenticated member-role mutation requests return `401`,
  - callers without `MANAGE_ROLES` return `403`,
  - delegated manager cannot assign roles equal to or above own highest authority,
  - valid assignment toggle writes/clears `role_assignments`,
  - permission-sensitive endpoints reflect new permissions immediately after assignment updates.
- Server unit tests:
  - role assignability filtering from authority position,
  - idempotent assignment toggles,
  - target-member validation behavior.
- Frontend tests:
  - member list renders API-backed entries and highest-role username color,
  - user context menu/profile popover shows `Assign role` only to eligible actors,
  - role toggle UI submits expected payload and updates UI state,
  - failure path restores UI and surfaces actionable error text.

### Previous Story Intelligence

- Story 5.3 already added `permissions::highest_guild_role_authority` and `permissions::actor_outranks_target_member` with strict outrank semantics; Story 5.4 should consume these helpers directly.
- Role list invariants are established and tested: Owner pseudo-role first, custom roles ordered by `position`, `@everyone` last and immutable.
- Role mutations currently invalidate guild permission cache via `permissions::invalidate_guild_permission_cache`; assignment mutations need equivalent invalidation coverage.
- Existing integration tests already seed `role_assignments` directly to verify behavior; extend this test style for assignment APIs.

### Git Intelligence Summary

- `f9149f0 feat: finalize story 5-3 role hierarchy and ordering` added role reorder endpoint, hierarchy helpers, and reorder cache invalidation patterns.
- `1f41931 feat: finalize story 5-2 permission bitflag engine` established canonical permission catalog and `MANAGE_ROLES`-based authorization.
- `aaf0082 feat: finalize story 5-1 role crud and default roles` introduced role/role_assignment persistence and role CRUD slices.
- `00d3274 feat: finalize story 4-7 guild navigation and activity indicators` confirms current shell/member panel layout and responsive member-list toggling behavior.
- `608adec feat: implement story 4-6 invite guild join` established guild membership persistence baseline (`guild_members`) used by role assignment targeting.

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
- Axum: `0.8.8` (`axum-v0.8.8`)
- SQLx: `0.8.6` (`v0.8.6`)
- rust-libp2p: `0.56.0` (`libp2p-v0.56.0`)

No dependency upgrade is required to implement Story 5.4; implement against currently pinned project versions.

### Project Context Reference

- No `project-context.md` was discovered via `**/project-context.md`.
- Story context is derived from planning artifacts, Stories 5.1–5.3 implementation artifacts, current runtime code, and recent commit history.

### Story Completion Status

- Ultimate context engine analysis completed - comprehensive developer guide created.
- Story status set to `ready-for-dev`.

### Project Structure Notes

- Architecture planning maps FR24-29 client work toward `features/members/*`; current runtime has `features/members/MemberList.svelte` as a placeholder while role management APIs/state live in `features/guild/*`.
- For Story 5.4, keep member interaction UI in `features/members/` while reusing existing guild API/store role plumbing to avoid duplicate domain logic.
- Story 5.7 (member list with presence) will expand this surface further; Story 5.4 should introduce assignment-ready foundations without blocking later presence work.

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Epic 5: Roles, Permissions & Member Management]
- [Source: _bmad-output/planning-artifacts/epics.md#Story 5.4: Role Assignment to Members]
- [Source: _bmad-output/planning-artifacts/epics.md#Story 5.3: Role Hierarchy and Ordering]
- [Source: _bmad-output/planning-artifacts/epics.md#Story 5.6: Role Management Delegation]
- [Source: _bmad-output/planning-artifacts/prd.md#Roles & Permissions]
- [Source: _bmad-output/planning-artifacts/prd.md#Security (NFR16)]
- [Source: _bmad-output/planning-artifacts/architecture.md#Feature Module Pattern (Frontend)]
- [Source: _bmad-output/planning-artifacts/architecture.md#REST API Response Format]
- [Source: _bmad-output/planning-artifacts/architecture.md#Error Handling]
- [Source: _bmad-output/planning-artifacts/architecture.md#Requirements to Structure Mapping]
- [Source: _bmad-output/planning-artifacts/architecture.md#Cross-Cutting Concerns Mapping]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#MemberListEntry]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#User context menu (in member list or on username)]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Context menu rules]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Desktop (≥1024px) — Primary experience]
- [Source: _bmad-output/implementation-artifacts/5-3-role-hierarchy-and-ordering.md]
- [Source: _bmad-output/implementation-artifacts/5-2-permission-assignment-and-bitflag-engine.md]
- [Source: _bmad-output/implementation-artifacts/5-1-role-crud-and-default-roles.md]
- [Source: _bmad-output/implementation-artifacts/sprint-status.yaml]
- [Source: server/migrations/0015_create_roles_and_role_assignments.sql]
- [Source: server/src/handlers/mod.rs]
- [Source: server/src/handlers/roles.rs]
- [Source: server/src/services/role_service.rs]
- [Source: server/src/models/role.rs]
- [Source: server/src/models/guild_member.rs]
- [Source: server/src/permissions/mod.rs]
- [Source: server/tests/server_binds_to_configured_port.rs]
- [Source: client/src/lib/features/members/MemberList.svelte]
- [Source: client/src/lib/features/shell/ShellRoute.svelte]
- [Source: client/src/lib/features/guild/GuildSettings.svelte]
- [Source: client/src/lib/features/guild/GuildSettings.test.ts]
- [Source: client/src/lib/features/guild/guildApi.ts]
- [Source: client/src/lib/features/guild/guildStore.svelte.ts]
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

- `cd server && cargo fmt --check`
- `cd server && cargo clippy -- -D warnings`
- `cd server && cargo test`
- `cd client && npm run lint && npm run check && npm run test && npm run build`
- `cd client && npm run lint && npm run check && npm run test && npm run build && cd ../server && cargo fmt --check && cargo clippy -- -D warnings && cargo test`

### Completion Notes List

- Added member-role API surface: `GET /api/v1/guilds/{guild_slug}/members` and `PATCH /api/v1/guilds/{guild_slug}/members/{member_user_id}/roles`.
- Implemented strict server-side assignment guards using `MANAGE_ROLES`, role hierarchy authority, and member outrank checks.
- Added transactional role-assignment model helpers with SQLite/Postgres parity and deterministic assigned-role ordering.
- Added permission-cache invalidation after assignment mutations and returned updated member role state payloads.
- Replaced MemberList placeholder with API-backed members, role-assignment popover flow, keyboard/context-menu access, optimistic updates, and failure rollback messaging.
- Added frontend API/store support for member-role data and mutation endpoints.
- Added/extended integration and frontend tests for auth/forbidden/hierarchy/idempotency/cache invalidation and assignment UI flows.

### File List

- server/src/handlers/mod.rs
- server/src/handlers/roles.rs
- server/src/models/guild_member.rs
- server/src/models/role.rs
- server/src/services/role_service.rs
- server/tests/server_binds_to_configured_port.rs
- client/src/lib/features/guild/types.ts
- client/src/lib/features/guild/guildApi.ts
- client/src/lib/features/guild/guildApi.test.ts
- client/src/lib/features/guild/guildStore.svelte.ts
- client/src/lib/features/members/MemberList.svelte
- client/src/lib/features/members/MemberList.test.ts
- client/src/lib/features/shell/ShellRoute.svelte

### Change Log

- 2026-02-28: Implemented Story 5.4 member role assignment APIs, model/service logic, member list UI interactions, and full quality-gate test coverage.
- 2026-02-28: Senior code review resolved one AC-4 data-model gap (`RoleAssignment` now includes `guild_id`) and re-ran server quality gates.

### Senior Developer Review (AI)

- Reviewer: Darko
- Date: 2026-02-28
- Outcome: Approve
- Findings fixed:
  - **HIGH** `server/src/models/role.rs`: `RoleAssignment` omitted `guild_id` even though storage tuple is `(guild_id, user_id, role_id)`; struct and assignment-list queries now include `guild_id`.
- Acceptance Criteria coverage:
  - AC1 ✅, AC2 ✅, AC3 ✅, AC4 ✅, AC5 ✅, AC6 ✅
- Git vs story file list: ✅ no discrepancies
- Validation rerun:
  - `cd server && cargo fmt --check && cargo clippy -- -D warnings && cargo test`
