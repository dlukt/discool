# Story 5.1: Role CRUD and Default Roles

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a **guild owner**,
I want to create, edit, and delete custom roles for my guild,
so that I can organize members and control access to features.

## Acceptance Criteria

1. **Given** the user is the guild owner  
   **When** they navigate to Guild Settings -> Roles  
   **Then** they see an `@everyone` default role that cannot be deleted (applies to all members)

2. **Given** the user is the guild owner  
   **When** they create a role in Guild Settings -> Roles  
   **Then** they can create a new custom role with a name and display color

3. **Given** roles are shown in Guild Settings -> Roles  
   **When** the hierarchy is rendered  
   **Then** roles are listed in hierarchy order with owner role at top and `@everyone` at bottom

4. **Given** a custom role exists  
   **When** the owner edits that role  
   **Then** they can rename it and change its display color

5. **Given** a custom role exists  
   **When** the owner deletes it  
   **Then** a confirmation dialog is required  
   **And** members with that role lose it

6. **Given** this story migration runs  
   **When** role persistence is initialized  
   **Then** a `roles` table exists with columns: `guild_id`, `name`, `color`, `position`, `permissions_bitflag`, `created_at`

7. **Given** this story migration runs  
   **When** role assignment persistence is initialized  
   **Then** a `role_assignments` table exists for the many-to-many relationship between users and roles

## Tasks / Subtasks

- [x] Task 1: Add role persistence migrations and bootstrap defaults (AC: 1, 5, 6, 7)
  - [x] Add `server/migrations/0015_create_roles_and_role_assignments.sql` creating `roles` and `role_assignments` with required columns and cross-DB-compatible constraints/indexes.
  - [x] Backfill one `@everyone` role per existing guild in migration so legacy guilds satisfy AC 1 immediately.
  - [x] Ensure role assignment deletion behavior is explicit (FK cascade or equivalent) so deleting a custom role removes member links.

- [x] Task 2: Implement role model operations with SQLite + Postgres parity (AC: 2, 4, 5, 6, 7)
  - [x] Add `server/src/models/role.rs` with list/create/update/delete role queries and assignment cleanup helpers.
  - [x] Keep existing dual-backend query style (`DbPool::Postgres` + `DbPool::Sqlite`) used by channel/category/invite models.
  - [x] Add deterministic hierarchy query ordering and guards that preserve `@everyone` as non-deletable/non-editable at the model/service boundary.

- [x] Task 3: Implement role service layer with owner authorization and hierarchy shaping (AC: 1, 2, 3, 4, 5)
  - [x] Add `server/src/services/role_service.rs` and wire `server/src/services/mod.rs`.
  - [x] Reuse existing guild ownership authorization patterns (`load_owned_guild`-style checks) for all role mutations.
  - [x] Implement role list response shaping so hierarchy renders as owner role top (system/pseudo role) and `@everyone` bottom.
  - [x] Enforce server-side validation for name/color and block deletion/rename of `@everyone`.

- [x] Task 4: Expose role CRUD APIs under guild routes (AC: 1, 2, 4, 5)
  - [x] Add `server/src/handlers/roles.rs` and wire routes in `server/src/handlers/mod.rs`.
  - [x] Add endpoints under `/api/v1/guilds/{guild_slug}/roles` for list/create/update/delete role operations.
  - [x] Preserve API contracts: success envelope `{ "data": ... }`, error envelope from `AppError` with existing error codes/messages.

- [x] Task 5: Add client role domain/api wiring (AC: 1, 2, 4, 5)
  - [x] Extend guild feature client types with role wire/domain mappings (`snake_case` wire -> `camelCase` UI types).
  - [x] Add role API functions (list/create/update/delete) in guild feature API modules, following existing `guildApi.ts` and `channelApi.ts` patterns.
  - [x] Keep role requests scoped to guild slug routes and reuse existing `apiFetch` error handling semantics.

- [x] Task 6: Implement Guild Settings -> Roles UI flows (AC: 1, 2, 3, 4, 5)
  - [x] Extend `GuildSettings.svelte` with a Roles section/panel that lists hierarchy and highlights system roles.
  - [x] Add create/edit dialogs with name + color inputs, inline validation, and explicit save actions.
  - [x] Add delete confirmation dialog for custom roles with irreversible warning text; hide/disable delete for `@everyone`.
  - [x] Ensure UI behavior matches UX patterns for dialog/form layout, accessible labels, and destructive action placement.

- [x] Task 7: Add integration/component tests and run quality gates (AC: all)
  - [x] Extend `server/tests/server_binds_to_configured_port.rs` with role auth checks (401/403), owner CRUD flow, `@everyone` protection, and role assignment removal on delete.
  - [x] Add client tests around role hierarchy rendering, create/edit/delete flows, validation, and owner-only visibility.
  - [x] Run quality gates:
    - [x] `cd client && npm run lint && npm run check && npm run test && npm run build`
    - [x] `cd server && cargo fmt --check && cargo clippy -- -D warnings && cargo test`

## Dev Notes

### Developer Context

- Epic 5 is not implemented yet in runtime code: no role routes, services, models, or migrations currently exist.
- Existing guild/channel/category/invite vertical slices provide the implementation baseline for layering, auth checks, and response envelopes.
- Guild settings surface already exists and is owner-gated, making it the natural placement for the first `Guild Settings -> Roles` UI slice.

### Technical Requirements

- Keep role mutation authorization owner-only for this story; full delegated role management belongs to later Epic 5 stories (FR29).
- Enforce `@everyone` invariants server-side (cannot delete, cannot be converted into a custom role by rename/update).
- Keep hierarchy deterministic:
  - owner role displayed first (system/pseudo entry),
  - custom roles in position order,
  - `@everyone` always last.
- Preserve migration compatibility for SQLite + Postgres and avoid backend-specific SQL features that break one dialect.
- Maintain strict API boundaries:
  - handlers validate payloads and map request DTOs,
  - services enforce business rules and authorization,
  - models perform SQL only.

### Architecture Compliance

1. Keep route structure under `/api/v1/guilds/{guild_slug}/...` and wire in `handlers/mod.rs` with current Axum registration style.
2. Preserve `AppError` taxonomy and JSON envelope contracts used by current handlers.
3. Follow existing service authorization patterns (`load_owned_guild` for mutations, explicit `Forbidden` errors).
4. Reuse existing model dual-backend patterns (`DbPool::Postgres` + `DbPool::Sqlite`) for every role query.
5. Keep role functionality scoped to Story 5.1 (CRUD + default-role guarantees); do not implement full permission-toggle engine yet (Story 5.2).

### Library & Framework Requirements

- Frontend: Svelte 5 runes and `@mateothegreat/svelte5-router`; no new routing/state libraries needed.
- Backend: Axum 0.8 and sqlx 0.8 patterns already used in this codebase.
- Runtime environment remains compatible with libp2p 0.56 line; this story does not require P2P stack changes.

### File Structure Requirements

Expected primary touch points:

- `server/migrations/0015_create_roles_and_role_assignments.sql` (new)
- `server/src/models/mod.rs`
- `server/src/models/role.rs` (new)
- `server/src/services/mod.rs`
- `server/src/services/role_service.rs` (new)
- `server/src/handlers/mod.rs`
- `server/src/handlers/roles.rs` (new)
- `server/src/services/guild_service.rs` (only if needed to guarantee default role creation for newly created guilds)
- `server/tests/server_binds_to_configured_port.rs`
- `client/src/lib/features/guild/GuildSettings.svelte`
- `client/src/lib/features/guild/GuildSettings.test.ts`
- `client/src/lib/features/guild/guildApi.ts` and `client/src/lib/features/guild/types.ts` (or equivalent guild feature role API/types files)
- `client/src/lib/features/guild/guildStore.svelte.ts` (if role state/actions are colocated here)
- `client/src/lib/features/shell/ShellRoute.svelte` (only if UI entrypoint changes are required)

### Testing Requirements

- Server integration tests:
  - unauthenticated role mutation endpoints return `401`,
  - non-owner mutations return `403`,
  - owner can create/list/update/delete custom roles,
  - deleting `@everyone` is rejected (`422` or `403` per rule design),
  - deleting a custom role removes member role assignments.
- Client tests:
  - owner sees Roles section; non-owner sees guardrail message/no mutation controls,
  - create/edit role dialogs validate required fields and submit expected payloads,
  - hierarchy ordering renders owner top and `@everyone` bottom,
  - delete custom role requires explicit confirmation and refreshes list deterministically.
- Keep all existing guild/channel/category/invite behavior regression-safe while adding role functionality.

### Previous Story Intelligence

- Story 4.7 established per-guild navigation persistence and cached guild switching behavior; new settings/roles UI must not regress shell routing and sidebar behavior.
- Story 4.6 made guild/channel/category reads member-aware while keeping mutations owner-only; Story 5.1 should keep this mutation model unless explicitly changed.
- Story 4.5/4.6/4.7 all reinforce the same vertical pattern (handler -> service -> model, typed client API -> store -> component, integration tests in `server_binds_to_configured_port.rs`).

### Git Intelligence Summary

- `00d3274 feat: finalize story 4-7 guild navigation and activity indicators` confirms the latest baseline for guild shell/navigation state.
- `608adec feat: implement story 4-6 invite guild join` confirms owner-only mutation boundaries and member-read behavior currently enforced.
- `e735b72 feat: implement story 4-5 invite management` confirms current guild-level modal/action integration patterns and test style.
- `599b44e feat: finalize channel and category management` provides the closest existing CRUD + reorder backend/client implementation pattern to mirror.

### Latest Technical Information

1. Current repo dependency lines:
   - `svelte`: `^5.45.2`
   - `@mateothegreat/svelte5-router`: `^2.16.19`
   - `axum`: `0.8`
   - `sqlx`: `0.8`
   - `libp2p`: `0.56`
2. Latest stable lines researched:
   - Svelte: `5.53.6`
   - `@mateothegreat/svelte5-router`: `2.16.19`
   - Axum: `0.8.8`
   - SQLx: `0.8.6`
   - rust-libp2p: `0.56.0`
3. No dependency upgrade is required for Story 5.1; implement against currently pinned versions.

### Project Context Reference

- No `project-context.md` was discovered via `**/project-context.md`.
- Story context is derived from planning artifacts, Epic 4 implementation artifacts, current runtime code, and recent commit history.

### Story Completion Status

- Ultimate context engine analysis completed - comprehensive developer guide created.
- Story status set to `ready-for-dev`.

### Project Structure Notes

- Architecture planning references a future `features/members/` module, but current runtime code uses `features/guild/` for guild configuration workflows; implement the first role-management UI where existing guild settings patterns already live, then extract later if needed.
- Keep Story 5.1 focused on role CRUD/default-role persistence and hierarchy display; permission toggles/bitflag UX belong to Story 5.2.

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Epic 5: Roles, Permissions & Member Management]
- [Source: _bmad-output/planning-artifacts/epics.md#Story 5.1: Role CRUD and Default Roles]
- [Source: _bmad-output/planning-artifacts/prd.md#Roles & Permissions]
- [Source: _bmad-output/planning-artifacts/prd.md#Security]
- [Source: _bmad-output/planning-artifacts/prd.md#Non-Functional Requirements]
- [Source: _bmad-output/planning-artifacts/architecture.md#Feature Module Pattern (Frontend)]
- [Source: _bmad-output/planning-artifacts/architecture.md#Error Handling]
- [Source: _bmad-output/planning-artifacts/architecture.md#Complete Project Directory Structure]
- [Source: _bmad-output/planning-artifacts/architecture.md#Requirements to Structure Mapping]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Component Library Mapping (shadcn-svelte primitives)]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#MemberListEntry]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Settings forms]
- [Source: _bmad-output/implementation-artifacts/4-7-guild-navigation-and-activity-indicators.md]
- [Source: _bmad-output/implementation-artifacts/4-6-join-guild-via-invite-link.md]
- [Source: _bmad-output/implementation-artifacts/4-5-invite-link-generation-and-management.md]
- [Source: _bmad-output/implementation-artifacts/sprint-status.yaml]
- [Source: server/src/handlers/mod.rs]
- [Source: server/src/handlers/channels.rs]
- [Source: server/src/handlers/categories.rs]
- [Source: server/src/services/channel_service.rs]
- [Source: server/src/services/category_service.rs]
- [Source: server/src/models/mod.rs]
- [Source: server/src/models/channel.rs]
- [Source: server/src/services/mod.rs]
- [Source: server/src/error.rs]
- [Source: server/src/db/pool.rs]
- [Source: server/migrations/0011_create_channels.sql]
- [Source: server/migrations/0012_create_channel_categories.sql]
- [Source: server/migrations/0014_create_guild_members.sql]
- [Source: server/tests/server_binds_to_configured_port.rs]
- [Source: client/src/lib/api.ts]
- [Source: client/src/lib/features/guild/GuildSettings.svelte]
- [Source: client/src/lib/features/guild/GuildSettings.test.ts]
- [Source: client/src/lib/features/guild/guildApi.ts]
- [Source: client/src/lib/features/guild/guildStore.svelte.ts]
- [Source: client/src/lib/features/guild/types.ts]
- [Source: client/src/lib/features/channel/channelApi.ts]
- [Source: client/src/lib/features/channel/ChannelList.svelte]
- [Source: client/src/lib/features/channel/ChannelList.test.ts]
- [Source: client/src/lib/features/shell/ShellRoute.svelte]
- [Source: client/package.json]
- [Source: server/Cargo.toml]
- [Source: https://registry.npmjs.org/svelte/latest]
- [Source: https://registry.npmjs.org/@mateothegreat/svelte5-router/latest]
- [Source: https://api.github.com/repos/tokio-rs/axum/releases/latest]
- [Source: https://docs.rs/crate/sqlx/latest]
- [Source: https://api.github.com/repos/libp2p/rust-libp2p/releases/latest]

## Dev Agent Record

### Agent Model Used

GPT-5.3-Codex (model ID: gpt-5.3-codex)

### Debug Log References

- Workflow engine loaded: `_bmad/core/tasks/workflow.xml`
- Workflow config loaded: `_bmad/bmm/workflows/4-implementation/dev-story/workflow.yaml`
- Story source parsed from user input: `5-1` -> `5-1-role-crud-and-default-roles`
- Added migration `0015_create_roles_and_role_assignments.sql` with `@everyone` backfill and assignment cascade semantics.
- Implemented backend role vertical slice (model/service/handler/routes) and wired guild bootstrap default role creation.
- Implemented guild role client domain/api/store wiring and Guild Settings role create/edit/delete dialogs.
- Added server integration coverage for role auth, hierarchy, `@everyone` protection, and assignment cleanup.
- Added client component coverage for role hierarchy and role CRUD dialog flows.
- Executed quality gates successfully for client and server.

### Completion Notes List

- Added complete Story 5.1 role CRUD implementation with SQLite/Postgres parity and owner-only mutation authorization.
- Ensured deterministic hierarchy rendering with owner pseudo-role first, custom roles in order, and `@everyone` always last.
- Enforced `@everyone` invariants and assignment cleanup behavior on role deletion at service/model boundaries.
- Shipped Guild Settings role management UI flows with accessible create/edit/delete dialogs and destructive confirmation copy.
- Verified behavior through new server + client tests and full repository quality gates.

### File List

- _bmad-output/implementation-artifacts/5-1-role-crud-and-default-roles.md
- server/migrations/0015_create_roles_and_role_assignments.sql
- server/src/models/mod.rs
- server/src/models/role.rs
- server/src/services/mod.rs
- server/src/services/role_service.rs
- server/src/services/guild_service.rs
- server/src/handlers/mod.rs
- server/src/handlers/roles.rs
- server/tests/server_binds_to_configured_port.rs
- client/src/lib/features/guild/types.ts
- client/src/lib/features/guild/guildApi.ts
- client/src/lib/features/guild/guildStore.svelte.ts
- client/src/lib/features/guild/GuildSettings.svelte
- client/src/lib/features/guild/GuildSettings.test.ts
- _bmad-output/implementation-artifacts/sprint-status.yaml

## Senior Developer Review (AI)

### Reviewer

Darko (GPT-5.3-Codex) on 2026-02-28

### Outcome

Approve - no actionable HIGH/MEDIUM findings.

### Notes

- Acceptance Criteria 1-7 were cross-checked against implementation and tests.
- Git/source changes align with the story File List for application code.
- Quality gates passed:
  - `cd client && npm run lint && npm run check && npm run test && npm run build`
  - `cd server && cargo fmt --check && cargo clippy -- -D warnings && cargo test`

### Change Log

- 2026-02-28: Implemented Story 5.1 role CRUD/default-role functionality across migration, backend APIs/services/models, guild settings UI, and automated tests.
- 2026-02-28: Senior AI code review completed (YOLO); no actionable findings; story marked `done`.
