# Story 5.3: Role Hierarchy and Ordering

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a **guild owner**,
I want to set role hierarchy so higher roles take precedence,
so that permission conflicts are resolved predictably.

## Acceptance Criteria

1. **Given** the guild owner is viewing Guild Settings -> Roles  
   **When** they drag roles to reorder them  
   **Then** the role hierarchy is updated (higher position = higher authority)

2. **Given** a user has multiple roles  
   **When** effective permissions are computed  
   **Then** effective permissions are the union of all assigned role permissions

3. **Given** a user attempts moderation actions (kick, ban, mute)  
   **When** they target another member  
   **Then** the action is allowed only when the target member's highest role is lower than the actor's highest role

4. **Given** role hierarchy ordering is edited  
   **When** persisted and rendered  
   **Then** the guild owner remains at the top of the hierarchy and cannot be reordered

5. **Given** role order changes are submitted  
   **When** persistence succeeds  
   **Then** updates are immediate and permission cache entries for affected guild members are invalidated

## Tasks / Subtasks

- [x] Task 1: Add backend role reorder API and persistence flow (AC: 1, 4, 5)
  - [x] Add `PATCH /api/v1/guilds/{guild_slug}/roles/reorder` in `server/src/handlers/mod.rs` and `server/src/handlers/roles.rs`.
  - [x] Define request DTO (recommended: ordered custom-role IDs only) and validate required fields in the handler.
  - [x] Implement service-level validation mirroring channel/category reorder patterns: complete set, no unknown IDs, no duplicates.
  - [x] Persist custom-role positions transactionally for both Postgres and SQLite in `server/src/models/role.rs`.
  - [x] Keep owner pseudo-role fixed at top and `@everyone` fixed at bottom regardless of incoming reorder payload.

- [x] Task 2: Enforce hierarchy invariants and authority comparison helpers (AC: 1, 3, 4)
  - [x] Add reusable hierarchy helpers in `server/src/permissions/mod.rs` for resolving highest role authority per member.
  - [x] Encode actor-vs-target rule: actor must outrank target; equal rank must be denied.
  - [x] Keep owner shortcut behavior explicit (owner always outranks all guild members).
  - [x] Document and enforce the existing rank convention used by runtime code (ordered hierarchy drives authority; avoid ambiguous numeric assumptions).

- [x] Task 3: Preserve effective-permission union and cache correctness (AC: 2, 5)
  - [x] Keep `effective_guild_permissions` union behavior (default role + assigned roles) unchanged and covered by tests.
  - [x] Invalidate guild permission cache on reorder writes (`permissions::invalidate_guild_permission_cache`).
  - [x] Ensure reorder responses return deterministic hierarchy ordering used by UI immediately after mutation.

- [x] Task 4: Implement Guild Settings role drag-reorder UX (AC: 1, 4, 5)
  - [x] Add drag-and-drop interactions for custom roles in `client/src/lib/features/guild/GuildSettings.svelte`.
  - [x] Keep Owner and `@everyone` non-draggable and visibly fixed.
  - [x] Persist reorder immediately via guild store/API and show success/error status feedback using existing message patterns.
  - [x] Revert optimistic UI ordering on failure to prevent stale local hierarchy.

- [x] Task 5: Add client API/store wiring for role reorder (AC: 1, 5)
  - [x] Extend guild feature wire/domain types in `client/src/lib/features/guild/types.ts` with reorder payload typing.
  - [x] Add reorder API helper in `client/src/lib/features/guild/guildApi.ts`.
  - [x] Add `guildState.reorderRoles(...)` in `client/src/lib/features/guild/guildStore.svelte.ts` with cache refresh behavior consistent with existing role mutations.

- [x] Task 6: Add coverage and run quality gates (AC: all)
  - [x] Extend `server/tests/server_binds_to_configured_port.rs` with reorder endpoint auth/authorization/validation/success scenarios.
  - [x] Add server unit tests for highest-role resolution and actor-target hierarchy guard behavior.
  - [x] Extend `client/src/lib/features/guild/GuildSettings.test.ts` with drag reorder success and rollback-on-error behavior.
  - [x] Run quality gates:
    - [x] `cd client && npm run lint && npm run check && npm run test && npm run build`
    - [x] `cd server && cargo fmt --check && cargo clippy -- -D warnings && cargo test`

## Dev Notes

### Developer Context

- Story 5.2 already introduced canonical permission bitflags, union-based effective permission computation, and service-level `require_guild_permission` guards.
- Current role APIs support list/create/update/delete and permission editing, but there is no role reorder route yet.
- Current role list rendering already includes fixed system roles: Owner pseudo-role first and `@everyone` last.
- No moderation handlers/services exist yet in runtime code (`kick/ban/mute` not implemented), so hierarchy authority logic must be delivered as reusable guardrails now and consumed by upcoming moderation/member-management stories.

### Technical Requirements

- Keep hierarchy ordering deterministic and server-authoritative.
- Preserve existing role invariants:
  - Owner pseudo-role remains immutable and always top.
  - `@everyone` remains immutable and always bottom.
  - Custom roles are the only reorderable set.
- Role hierarchy precedence must be enforced server-side; UI reorder is advisory only.
- Keep effective permission behavior as union across assigned roles plus default role (no regression from Story 5.2).
- Add actor-target authority comparison helper now so upcoming moderation/member-target operations can enforce AC3 consistently.
- Cache invalidation must occur after reorder mutation commits.

### Architecture Compliance

1. Keep route registration in `server/src/handlers/mod.rs`, request validation in handlers, business rules in services, and SQL in models.
2. Mirror existing reorder architecture used for channels/categories (`handlers/* -> services/* -> models/*` with strict payload validation).
3. Keep success/error contracts unchanged: `{ "data": ... }` envelopes and `AppError` error shape.
4. Maintain dual backend parity (Postgres + SQLite) for reorder writes using transaction-safe updates.
5. Keep guild settings role management inside `client/src/lib/features/guild/` to match current runtime module boundaries.

### Library & Framework Requirements

- Frontend: Svelte 5 runes and existing guild feature state/api modules.
- Backend: Axum 0.8 + sqlx 0.8 existing patterns.
- No new framework/library adoption required for this story.

### File Structure Requirements

Expected primary touch points:

- `server/src/handlers/mod.rs`
- `server/src/handlers/roles.rs`
- `server/src/services/role_service.rs`
- `server/src/models/role.rs`
- `server/src/permissions/mod.rs`
- `server/tests/server_binds_to_configured_port.rs`
- `client/src/lib/features/guild/GuildSettings.svelte`
- `client/src/lib/features/guild/GuildSettings.test.ts`
- `client/src/lib/features/guild/guildApi.ts`
- `client/src/lib/features/guild/guildStore.svelte.ts`
- `client/src/lib/features/guild/types.ts`

### Testing Requirements

- Server integration tests:
  - unauthenticated reorder requests return `401`,
  - unauthorized callers return `403`,
  - invalid payloads (missing/duplicate/unknown role IDs) return `422`,
  - successful reorder persists and returns expected role order,
  - Owner and `@everyone` remain fixed.
- Server unit tests:
  - highest-role resolution for owner/member/default-only cases,
  - actor-target hierarchy guard allows only strict outrank.
- Client tests:
  - custom role drag reorder triggers store/API call,
  - system roles are not draggable/reorderable,
  - failed reorder restores prior UI order and shows error feedback.

### Previous Story Intelligence

- Story 5.2 established permission bitflag catalog and `permissions::require_guild_permission`.
- Story 5.2 already invalidates permission cache on role permission mutations; reorder should follow the same pattern.
- Guild Settings already has role list/status messaging and permission modal patterns that reorder UX should reuse.
- Existing tests in `server_binds_to_configured_port.rs` already verify Owner-first and `@everyone`-last invariants; extend rather than duplicate style.

### Git Intelligence Summary

- `1f41931 feat: finalize story 5-2 permission bitflag engine` is the immediate baseline for role permissions, guards, and cache invalidation hooks.
- `aaf0082 feat: finalize story 5-1 role crud and default roles` introduced role persistence/service/handler/client slices that reorder should extend.
- `599b44e feat: finalize channel and category management` provides the nearest reorder validation/persistence pattern to replicate for roles.
- `00d3274 feat: finalize story 4-7 guild navigation and activity indicators` contains current drag-and-drop interaction style used in the client codebase.

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

No dependency upgrade is required to implement Story 5.3; implement against currently pinned project versions.

### Project Context Reference

- No `project-context.md` was discovered via `**/project-context.md`.
- Story context is derived from planning artifacts, Stories 5.1/5.2 implementation artifacts, current runtime code, and recent commit history.

### Story Completion Status

- Ultimate context engine analysis completed - comprehensive developer guide created.
- Story status set to `ready-for-dev`.

### Project Structure Notes

- Architecture planning mentions future `features/members/` decomposition, but current runtime role management lives in `features/guild/`; keep Story 5.3 changes in `features/guild/` for consistency.
- Moderation target-action endpoints are not yet present; deliver reusable hierarchy guard logic now so future moderation/member stories can wire enforcement without redesign.

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Epic 5: Roles, Permissions & Member Management]
- [Source: _bmad-output/planning-artifacts/epics.md#Story 5.3: Role Hierarchy and Ordering]
- [Source: _bmad-output/planning-artifacts/prd.md#Roles & Permissions]
- [Source: _bmad-output/planning-artifacts/prd.md#Non-Functional Requirements (NFR16)]
- [Source: _bmad-output/planning-artifacts/architecture.md#Implementation Patterns & Consistency Rules]
- [Source: _bmad-output/planning-artifacts/architecture.md#Requirements to Structure Mapping]
- [Source: _bmad-output/planning-artifacts/architecture.md#Cross-Cutting Concerns Mapping]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Journey 2: Community Setup & First Guild]
- [Source: _bmad-output/implementation-artifacts/5-2-permission-assignment-and-bitflag-engine.md]
- [Source: _bmad-output/implementation-artifacts/5-1-role-crud-and-default-roles.md]
- [Source: _bmad-output/implementation-artifacts/sprint-status.yaml]
- [Source: server/src/handlers/mod.rs]
- [Source: server/src/handlers/roles.rs]
- [Source: server/src/services/role_service.rs]
- [Source: server/src/models/role.rs]
- [Source: server/src/permissions/mod.rs]
- [Source: server/tests/server_binds_to_configured_port.rs]
- [Source: client/src/lib/features/guild/GuildSettings.svelte]
- [Source: client/src/lib/features/guild/GuildSettings.test.ts]
- [Source: client/src/lib/features/guild/guildStore.svelte.ts]
- [Source: client/src/lib/features/guild/guildApi.ts]
- [Source: client/src/lib/features/guild/types.ts]
- [Source: client/src/lib/features/channel/ChannelList.svelte]
- [Source: client/package.json]
- [Source: server/Cargo.toml]
- [Source: https://registry.npmjs.org/svelte/latest]
- [Source: https://registry.npmjs.org/@mateothegreat/svelte5-router/latest]
- [Source: https://docs.rs/crate/axum/latest]
- [Source: https://docs.rs/crate/sqlx/latest]
- [Source: https://docs.rs/crate/libp2p/latest]
- [Source: https://api.github.com/repos/tokio-rs/axum/releases/latest]
- [Source: https://api.github.com/repos/launchbadge/sqlx/tags?per_page=5]
- [Source: https://api.github.com/repos/libp2p/rust-libp2p/releases/latest]

## Dev Agent Record

### Agent Model Used

GPT-5.3-Codex (model ID: gpt-5.3-codex)

### Debug Log References

- Workflow engine loaded: `_bmad/core/tasks/workflow.xml`
- Workflow config loaded: `_bmad/bmm/workflows/4-implementation/dev-story/workflow.yaml`
- Story source parsed from user input: `5-3` -> `5-3-role-hierarchy-and-ordering`
- Sprint status source loaded: `_bmad-output/implementation-artifacts/sprint-status.yaml`
- Core artifact analysis loaded: epics, architecture, prd, ux, prior stories 5.1/5.2, runtime code, and recent commits.

### Completion Notes List

- Added backend role reorder route/handler/service/model flow with complete-set validation, transactional writes (SQLite + Postgres), deterministic response ordering, and permission cache invalidation.
- Added reusable hierarchy helpers in `permissions` for highest member authority and strict actor-vs-target outrank checks, including explicit owner shortcut behavior.
- Added/updated server coverage for reorder auth/authorization/validation/success and kept effective permission union behavior covered by unit tests.
- Added role drag-and-drop UX for custom roles only, kept Owner/@everyone fixed, persisted role order through guild API/store wiring, and rolled back optimistic UI on failure.
- Ran full project quality gates successfully for client and server.

### Change Log

- 2026-02-28: Implemented Story 5.3 role hierarchy ordering end-to-end (backend API + persistence, permission hierarchy helpers, client DnD reorder UX, and test coverage).
- 2026-02-28: Senior developer code review completed (YOLO mode); no HIGH/MEDIUM findings; story moved to done.

### File List

- _bmad-output/implementation-artifacts/sprint-status.yaml
- _bmad-output/implementation-artifacts/5-3-role-hierarchy-and-ordering.md
- client/src/lib/features/guild/GuildSettings.svelte
- client/src/lib/features/guild/GuildSettings.test.ts
- client/src/lib/features/guild/guildApi.ts
- client/src/lib/features/guild/guildStore.svelte.ts
- client/src/lib/features/guild/types.ts
- server/src/handlers/mod.rs
- server/src/handlers/roles.rs
- server/src/models/role.rs
- server/src/permissions/mod.rs
- server/src/services/role_service.rs
- server/tests/server_binds_to_configured_port.rs

## Senior Developer Review (AI)

### Reviewer

Darko (GPT-5.3-Codex)

### Date

2026-02-28

### Outcome

Approve — no HIGH or MEDIUM findings.

### Notes

- Story File List matched actual git changes for implementation files.
- Acceptance Criteria 1-5 are implemented and covered by relevant tests.
- Quality gates re-run and passed:
  - `cd client && npm run lint && npm run check && npm run test && npm run build`
  - `cd server && cargo fmt --check && cargo clippy -- -D warnings && cargo test`
