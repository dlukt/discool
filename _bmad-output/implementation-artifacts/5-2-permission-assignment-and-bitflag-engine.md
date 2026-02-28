# Story 5.2: Permission Assignment and Bitflag Engine

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a **guild owner**,
I want to assign specific permissions to each role,
so that I can control what each group of members can do.

## Acceptance Criteria

1. **Given** the guild owner is editing a role in Guild Settings -> Roles  
   **When** they open the permissions panel for that role  
   **Then** they see a list of all available permissions as toggles: `SEND_MESSAGES`, `MANAGE_CHANNELS`, `KICK_MEMBERS`, `BAN_MEMBERS`, `MANAGE_ROLES`, `MANAGE_GUILD`, `MANAGE_INVITES`, `MUTE_MEMBERS`, `VIEW_MOD_LOG`, `ATTACH_FILES`, `ADD_REACTIONS`, `MANAGE_MESSAGES`

2. **Given** the guild owner is editing role permissions  
   **When** they view each permission  
   **Then** each permission is a toggle switch (on/off)

3. **Given** role permissions are changed  
   **When** role data is persisted  
   **Then** permissions are stored as a bitflag integer on the role record

4. **Given** the owner role exists in role listings  
   **When** permissions are displayed  
   **Then** the owner role has all permissions implicitly and cannot be modified

5. **Given** the `@everyone` default role exists  
   **When** role defaults are initialized  
   **Then** `@everyone` defaults to `SEND_MESSAGES`, `ATTACH_FILES`, and `ADD_REACTIONS` enabled

6. **Given** a guild owner toggles a permission  
   **When** they switch it on or off  
   **Then** the change auto-saves immediately and a toast/status confirmation is shown

7. **Given** any protected API endpoint is called  
   **When** the server evaluates authorization  
   **Then** every API call validates the caller's computed permissions before executing (NFR16)

## Tasks / Subtasks

- [x] Task 1: Introduce canonical permission catalog and bitflag helpers (AC: 1, 2, 3, 4, 5)
  - [x] Add `server/src/permissions/mod.rs` with canonical bit definitions for the 12 permissions in AC order and helper masks (`all`, `default_everyone`).
  - [x] Keep DB storage as integer bitflags (`roles.permissions_bitflag`) while performing bit math in an unsigned type and checked conversion at boundaries.
  - [x] Add a shared client permission catalog (labels/order/help text) in `client/src/lib/features/guild/` so the UI and API payloads stay consistent.

- [x] Task 2: Extend role API/service/model flows for permission updates (AC: 1, 3, 4, 5, 6)
  - [x] Extend role update request/DTOs to accept `permissions_bitflag` without regressing existing name/color edit behavior.
  - [x] Allow permission updates for mutable persisted roles, but keep owner pseudo-role immutable and always computed as full permissions.
  - [x] Ensure default `@everyone` bitflag is set to `SEND_MESSAGES | ATTACH_FILES | ADD_REACTIONS` for newly created guilds and backfilled for existing defaults (new migration after `0015`).
  - [x] Return role payloads with updated `permissions_bitflag` in existing `{ "data": ... }` response envelopes.

- [x] Task 3: Implement computed permission evaluation and service-level authorization guards (AC: 7)
  - [x] Add a permission evaluation helper that resolves effective guild permissions from role assignments + default role + owner override.
  - [x] Introduce reusable service guard helpers (e.g., `require_guild_permission`) that return `AppError::Forbidden` with explicit messages on denial.
  - [x] Wire permission checks into relevant guild-scoped service mutations (roles/channels/categories/invites/guild settings) while preserving owner access semantics.
  - [x] Add invalidation hooks for computed permission cache entries on role permission mutations to avoid stale authorization decisions.

- [x] Task 4: Build Guild Settings -> Roles permission panel with immediate save UX (AC: 1, 2, 4, 6)
  - [x] Extend `GuildSettings.svelte` role controls with a permissions panel/dialog that lists all 12 toggles in the canonical order.
  - [x] Show owner role as read-only ("implicit all permissions"), and prevent modification attempts from UI actions.
  - [x] Implement immediate API persistence on toggle change with clear success/failure status messaging (toast/inline status consistent with existing role messages).
  - [x] Keep accessibility parity: labeled controls, keyboard-operable toggles, and error text announced via existing alert patterns.

- [x] Task 5: Add comprehensive tests and run quality gates (AC: all)
  - [x] Add server unit tests for bitflag encode/decode and default/all mask helpers.
  - [x] Extend `server/tests/server_binds_to_configured_port.rs` with permission update + enforcement scenarios (401/403, owner immutability, `@everyone` defaults, denied operations).
  - [x] Extend `client/src/lib/features/guild/GuildSettings.test.ts` for permissions panel rendering, toggle autosave calls, and failure handling.
  - [x] Run and capture quality gates:
    - [x] `cd client && npm run lint && npm run check && npm run test && npm run build`
    - [x] `cd server && cargo fmt --check && cargo clippy -- -D warnings && cargo test`

## Dev Notes

### Developer Context

- Story 5.1 delivered role CRUD + hierarchy rendering + default `@everyone` creation, but no permission editing UX and no cross-service permission engine yet.
- Current role update flow (`PATCH /guilds/{guild_slug}/roles/{role_id}`) supports only name/color, and default role permissions currently initialize to `0`.
- Guild-scoped service mutations are mostly owner-gated (`load_owned_guild`) today; Story 5.2 establishes the reusable RBAC foundation needed before delegated moderation stories.

### Technical Requirements

- Canonical permission bit assignments (keep stable across client/server):
  - `0: SEND_MESSAGES`
  - `1: MANAGE_CHANNELS`
  - `2: KICK_MEMBERS`
  - `3: BAN_MEMBERS`
  - `4: MANAGE_ROLES`
  - `5: MANAGE_GUILD`
  - `6: MANAGE_INVITES`
  - `7: MUTE_MEMBERS`
  - `8: VIEW_MOD_LOG`
  - `9: ATTACH_FILES`
  - `10: ADD_REACTIONS`
  - `11: MANAGE_MESSAGES`
- `@everyone` default mask must include only `SEND_MESSAGES`, `ATTACH_FILES`, `ADD_REACTIONS` (bitmask `1537` with the mapping above).
- Owner permissions are implicit full access and never persisted as mutable role state.
- Preserve API boundary conventions: snake_case wire fields, client-side camelCase transforms, `{ "data": ... }` success envelope and `AppError` error envelope.

### Architecture Compliance

1. Keep route topology under `/api/v1/guilds/{guild_slug}/roles...` and existing Axum handler registration in `server/src/handlers/mod.rs`.
2. Keep authorization/business rules in services and SQL in models (no SQL in handlers/services).
3. Implement permission evaluation as reusable domain logic (`server/src/permissions/`) to align with architecture mapping for FR24-29.
4. Preserve dual backend compatibility (Postgres + SQLite) for all new queries/migrations.
5. Keep frontend role-management work inside the existing `features/guild/` module pattern already used by this runtime codebase.

### Library & Framework Requirements

- Frontend: Svelte 5 runes, `@mateothegreat/svelte5-router`, existing fetch-based API layer (`apiFetch`) and current Guild Settings patterns.
- Backend: Axum 0.8 + sqlx 0.8 patterns already in use for handlers/services/models.
- No new framework adoption is required; implement within current stack and conventions.

### File Structure Requirements

Expected primary touch points:

- `server/migrations/0016_*_role_permission_*.sql` (new migration for default/backfill permission masks)
- `server/src/permissions/mod.rs` (new)
- `server/src/models/role.rs`
- `server/src/services/role_service.rs`
- `server/src/services/channel_service.rs`
- `server/src/services/category_service.rs`
- `server/src/services/guild_invite_service.rs`
- `server/src/services/guild_service.rs`
- `server/src/services/mod.rs`
- `server/src/handlers/roles.rs`
- `server/src/handlers/mod.rs` (if any route shape extension is required)
- `server/tests/server_binds_to_configured_port.rs`
- `client/src/lib/features/guild/types.ts`
- `client/src/lib/features/guild/guildApi.ts`
- `client/src/lib/features/guild/guildStore.svelte.ts`
- `client/src/lib/features/guild/GuildSettings.svelte`
- `client/src/lib/features/guild/GuildSettings.test.ts`

### Testing Requirements

- Server unit tests:
  - bitmask helper correctness (set/unset/contains),
  - default and full permission masks,
  - owner implicit-all behavior.
- Server integration tests:
  - unauthenticated permission updates return `401`,
  - non-authorized callers receive `403`,
  - owner can update role permissions successfully,
  - owner pseudo-role cannot be modified,
  - `@everyone` defaults are applied and returned correctly,
  - representative guild-scoped endpoints enforce computed permissions.
- Client tests:
  - permissions panel lists all 12 toggles in canonical order,
  - toggling triggers immediate API call and success messaging,
  - failed toggle persistence surfaces clear error and re-sync behavior.

### Previous Story Intelligence

- Story 5.1 established role storage and routing slices (`models/role.rs`, `services/role_service.rs`, `handlers/roles.rs`) that should be extended, not replaced.
- Role list ordering and system-role invariants are already enforced and tested (Owner first, custom roles by position, `@everyone` last).
- Existing role UX patterns already provide dialogs/status messaging; permission panel should reuse these patterns for consistency.

### Git Intelligence Summary

- `aaf0082 feat: finalize story 5-1 role crud and default roles` provides the immediate baseline for role service/model/UI patterns.
- `00d3274 feat: finalize story 4-7 guild navigation and activity indicators` confirms current shell/navigation behavior that Guild Settings updates must not regress.
- `608adec feat: implement story 4-6 invite guild join` and `e735b72 feat: implement story 4-5 invite management` reinforce current guild-scoped API and owner-mutation patterns.

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
- Axum: `0.8.8`
- SQLx: `0.8.6`
- rust-libp2p: `0.56.0`

No dependency upgrade is required to implement Story 5.2; build against currently pinned project versions.

### Project Context Reference

- No `project-context.md` was discovered via `**/project-context.md`.
- Story context is derived from planning artifacts, Story 5.1 implementation artifact, current runtime code, and recent commit history.

### Story Completion Status

- Ultimate context engine analysis completed - comprehensive developer guide created.
- Story status set to `ready-for-dev`.

### Project Structure Notes

- Architecture planning maps FR24-29 to `server/src/permissions/` and `features/members/`, while current runtime codebase has no `server/src/permissions/` yet and places role management in `features/guild/`.
- For this story, add backend permission engine modules under `server/src/permissions/` and keep UI work in `features/guild/` to match existing implemented module boundaries.

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Epic 5: Roles, Permissions & Member Management]
- [Source: _bmad-output/planning-artifacts/epics.md#Story 5.2: Permission Assignment and Bitflag Engine]
- [Source: _bmad-output/planning-artifacts/prd.md#Roles & Permissions]
- [Source: _bmad-output/planning-artifacts/prd.md#Non-Functional Requirements (NFR16)]
- [Source: _bmad-output/planning-artifacts/architecture.md#Cross-Cutting Concerns Identified]
- [Source: _bmad-output/planning-artifacts/architecture.md#Core Architectural Decisions]
- [Source: _bmad-output/planning-artifacts/architecture.md#Feature Module Pattern (Frontend)]
- [Source: _bmad-output/planning-artifacts/architecture.md#REST API Response Format]
- [Source: _bmad-output/planning-artifacts/architecture.md#Requirements to Structure Mapping]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Journey 2: Community Setup & First Guild]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Settings forms]
- [Source: _bmad-output/implementation-artifacts/5-1-role-crud-and-default-roles.md]
- [Source: _bmad-output/implementation-artifacts/sprint-status.yaml]
- [Source: server/src/handlers/mod.rs]
- [Source: server/src/handlers/roles.rs]
- [Source: server/src/services/role_service.rs]
- [Source: server/src/models/role.rs]
- [Source: server/src/error.rs]
- [Source: server/src/middleware/auth.rs]
- [Source: server/tests/server_binds_to_configured_port.rs]
- [Source: client/src/lib/features/guild/GuildSettings.svelte]
- [Source: client/src/lib/features/guild/GuildSettings.test.ts]
- [Source: client/src/lib/features/guild/guildApi.ts]
- [Source: client/src/lib/features/guild/guildStore.svelte.ts]
- [Source: client/src/lib/features/guild/types.ts]
- [Source: client/src/lib/api.ts]
- [Source: client/package.json]
- [Source: server/Cargo.toml]
- [Source: https://registry.npmjs.org/svelte/latest]
- [Source: https://registry.npmjs.org/@mateothegreat/svelte5-router/latest]
- [Source: https://docs.rs/crate/axum/latest]
- [Source: https://docs.rs/crate/sqlx/latest]
- [Source: https://api.github.com/repos/libp2p/rust-libp2p/releases/latest]

## Dev Agent Record

### Agent Model Used

GPT-5.3-Codex (model ID: gpt-5.3-codex)

### Debug Log References

- Workflow engine loaded: `_bmad/core/tasks/workflow.xml`
- Workflow config loaded: `_bmad/bmm/workflows/4-implementation/dev-story/workflow.yaml`
- Story source parsed from user input: `5-2` -> `5-2-permission-assignment-and-bitflag-engine`
- Sprint status transitioned: `ready-for-dev` -> `in-progress` -> `review`
- Quality gates executed successfully:
  - `cd client && npm run lint && npm run check && npm run test && npm run build`
  - `cd server && cargo fmt --check && cargo clippy -- -D warnings && cargo test`

### Completion Notes List

- Implemented canonical server permission bitflags and conversion helpers in `server/src/permissions/mod.rs`, including default/all masks (`1537`/`4095`).
- Extended role update flows to support `permissions_bitflag`, enforced owner pseudo-role immutability, and ensured `@everyone` defaults via migration `0016`.
- Added reusable permission guards (`require_guild_permission`) and wired mutation authorization across roles, channels, categories, invites, and guild settings services.
- Added Guild Settings permissions panel with 12 canonical toggles, owner read-only UX, immediate persistence, and inline success/error feedback.
- Expanded tests for permission updates, owner/default invariants, guard enforcement, cache invalidation behavior, and UI autosave/failure handling.

### File List

- _bmad-output/implementation-artifacts/5-2-permission-assignment-and-bitflag-engine.md
- _bmad-output/implementation-artifacts/sprint-status.yaml
- client/src/lib/features/guild/GuildSettings.svelte
- client/src/lib/features/guild/GuildSettings.test.ts
- client/src/lib/features/guild/permissions.ts
- client/src/lib/features/guild/types.ts
- server/migrations/0016_backfill_default_role_permission_bitflags.sql
- server/src/handlers/roles.rs
- server/src/lib.rs
- server/src/models/role.rs
- server/src/permissions/mod.rs
- server/src/services/category_service.rs
- server/src/services/channel_service.rs
- server/src/services/guild_invite_service.rs
- server/src/services/guild_service.rs
- server/src/services/role_service.rs
- server/tests/server_binds_to_configured_port.rs

### Change Log

- 2026-02-28: Implemented Story 5.2 permission assignment and bitflag engine end-to-end (backend guards, API updates, UI permissions panel, and test coverage).
- 2026-02-28: Senior developer review completed in YOLO mode; no blocking findings remained after validation.

## Senior Developer Review (AI)

### Reviewer

Darko

### Date

2026-02-28

### Outcome

Approved

### Summary

- Acceptance criteria and completed tasks were validated against implementation.
- Changed-file coverage matched the story File List for application source files.
- Targeted quality checks passed:
  - `cd client && npm run lint && npm run check && npm run test -- --run GuildSettings.test.ts`
  - `cd server && cargo fmt --check && cargo clippy -- -D warnings && cargo test guild_permission_bitflags_authorize_member_mutations_and_invalidate_cache && cargo test roles_owner_crud_hierarchy_and_delete_cleanup_work`

### Findings

- No blocking findings.
