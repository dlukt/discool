# Story 5.6: Role Management Delegation

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a **guild owner**,
I want to allow moderators to assign roles,
so that I don't have to handle every role change myself.

## Acceptance Criteria

1. **Given** a role has the `MANAGE_ROLES` permission enabled  
   **When** a user with that role attempts to assign or remove roles from another member  
   **Then** they can only assign roles that are lower in the hierarchy than their own highest role.

2. **Given** a delegated role manager has `MANAGE_ROLES`  
   **When** they interact with role definitions  
   **Then** they cannot modify role permissions (only the guild owner can).

3. **Given** a delegated role manager has `MANAGE_ROLES`  
   **When** they interact with role definitions  
   **Then** they cannot delete roles (only the guild owner can).

4. **Given** role assignments are changed by delegated managers  
   **When** moderation logging is available  
   **Then** those actions are visible in the mod log (Epic 8 integration point).

5. **Given** any role assignment API call is executed  
   **When** authorization and hierarchy checks are evaluated  
   **Then** the server enforces delegation constraints on every request.

## Tasks / Subtasks

- [x] Task 1: Split role-definition authority from delegated assignment authority (AC: 1, 2, 3, 5)
  - [x] In `server/src/services/role_service.rs`, separate owner-only role-definition access from `MANAGE_ROLES` assignment access.
  - [x] Ensure `create_role`, `update_role`, `delete_role`, and `reorder_roles` remain owner-only regardless of role bitflags.
  - [x] Keep `update_member_roles` permission-based (`MANAGE_ROLES`) with strict role hierarchy guards.

- [x] Task 2: Harden delegated role assignment enforcement (AC: 1, 5)
  - [x] Preserve strict outrank checks for actor vs target member (`actor_outranks_target_member` path).
  - [x] Reject assignment of `Owner` pseudo-role, `@everyone`, unknown role IDs, and roles equal/above actor authority.
  - [x] Keep immediate permission-cache invalidation on assignment mutations.

- [x] Task 3: Keep UI delegation behavior explicit and safe (AC: 1, 2, 3)
  - [x] Keep role definition UI in `client/src/lib/features/guild/GuildSettings.svelte` owner-only.
  - [x] Keep delegated assignment controls in `client/src/lib/features/members/MemberList.svelte` (driven by `can_manage_roles`, `can_assign_roles`, `assignable_role_ids`).
  - [x] Update permission copy in `client/src/lib/features/guild/permissions.ts` so `MANAGE_ROLES` language matches delegated-assignment scope.

- [x] Task 4: Add audit-ready instrumentation for future mod log linkage (AC: 4)
  - [x] Add structured tracing around delegated `update_member_roles` actions (actor, target, guild, added roles, removed roles).
  - [x] Keep this as a non-breaking integration hook until Epic 8 mod-log persistence is implemented.

- [x] Task 5: Add/adjust test coverage for delegation boundaries (AC: all)
  - [x] Update server integration tests in `server/tests/server_binds_to_configured_port.rs`:
    - delegated manager with `MANAGE_ROLES` can assign/remove lower roles,
    - delegated manager cannot create/update/delete/reorder roles,
    - delegated manager cannot assign equal/higher roles,
    - owner behavior remains unchanged.
  - [x] Keep/extend member assignment cache-invalidation verification and downstream permission effect checks.
  - [x] Update frontend tests in `client/src/lib/features/members/MemberList.test.ts` and `client/src/lib/features/guild/GuildSettings.test.ts` for owner-only role-definition controls and delegated assignment behavior.
  - [x] Run quality gates:
    - [x] `cd client && npm run lint && npm run check && npm run test && npm run build`
    - [x] `cd server && cargo fmt --check && cargo clippy -- -D warnings && cargo test`

## Dev Notes

### Developer Context

- Story 5.4 implemented role assignment endpoints and hierarchy checks for delegated managers, including `assignable_role_ids`, `can_manage_roles`, and per-member `can_assign_roles`.
- Current `role_service` uses one permission-gated loader (`MANAGE_ROLES`) for both role-definition mutations and member-role assignment, which over-grants delegated managers for Story 5.6 requirements.
- Guild settings already render an owner-only guardrail in the client, while member role assignment UX is exposed in the member list.

### Technical Requirements

- Preserve strict outrank semantics (`actor_position < target_position`) for assignment actions.
- Enforce owner-only role-definition mutations server-side (not just UI-gated).
- Keep API contracts unchanged (`{ "data": ... }` success envelope, structured error envelope).
- Maintain deterministic assignment mutation behavior (`set_role_assignments_for_user`) and cache invalidation.

### Architecture Compliance

1. Keep handlers as HTTP/DTO mapping only; business authorization rules stay in services.
2. Keep permission and hierarchy evaluation centralized in `server/src/permissions/mod.rs` and role service helpers.
3. Avoid duplicating role hierarchy logic in frontend; frontend should consume server-exposed capability fields.
4. Keep existing layered boundaries: handlers -> services -> models.

### Library & Framework Requirements

- Frontend: Svelte 5 + existing feature stores; no new client framework/library required.
- Backend: Axum 0.8 + sqlx 0.8 + existing permission helpers; no new backend framework/library required.
- Reuse existing tracing and test infrastructure.

### File Structure Requirements

Primary touch points expected for this story:

- `server/src/services/role_service.rs`
- `server/src/handlers/roles.rs`
- `server/src/permissions/mod.rs`
- `server/tests/server_binds_to_configured_port.rs`
- `client/src/lib/features/guild/permissions.ts`
- `client/src/lib/features/guild/GuildSettings.svelte`
- `client/src/lib/features/guild/GuildSettings.test.ts`
- `client/src/lib/features/members/MemberList.svelte`
- `client/src/lib/features/members/MemberList.test.ts`

### Testing Requirements

- Server integration tests must verify delegated role managers cannot mutate role definitions (`POST/PATCH/DELETE /roles`, `PATCH /roles/reorder`) even with `MANAGE_ROLES`.
- Server integration tests must verify delegated role managers can assign/remove only lower-ranked roles and cannot affect owner/equal-or-higher hierarchy targets.
- Server tests should retain permission-side effect assertions (e.g., granted/revoked invite capability) and cache invalidation behavior.
- Frontend tests should retain owner-only guardrail in guild settings and delegated assignment flows via member list (including keyboard context-menu paths).

### Previous Story Intelligence

- Story 5.4 established the delegated assignment data contract and strict outrank checks in role assignment flows.
- Story 5.5 reinforced permission-derived UI gating patterns for non-owner management actions.
- Story 5.3 and permission helpers established reusable hierarchy and authority primitives (`RoleAuthority`, strict outrank semantics).

### Git Intelligence Summary

- `2491aec` (`feat: finalize story 5-5 channel permission overrides`) confirms recent permission-gating and cache invalidation patterns.
- `68f87aa` (`feat: finalize story 5-4 role assignment to members`) introduced delegated member-role assignment endpoints and capability fields.
- `f9149f0` (`feat: finalize story 5-3 role hierarchy and ordering`) provides hierarchy and reorder baselines this story must preserve.
- `1f41931` (`feat: finalize story 5-2 permission bitflag engine`) defines canonical permission masks and role permission catalog behavior.
- `aaf0082` (`feat: finalize story 5-1 role crud and default roles`) created role persistence and default role foundations.

### Latest Technical Information

Current project pins:

- `svelte`: `^5.45.2`
- `@mateothegreat/svelte5-router`: `^2.16.19`
- `axum`: `0.8`
- `sqlx`: `0.8`
- `libp2p`: `0.56`

Latest lines checked:

- Svelte latest: `5.53.6`
- `@mateothegreat/svelte5-router` latest: `2.16.19`
- Axum latest release tag: `axum-v0.8.8`
- SQLx latest tag: `v0.8.6`
- rust-libp2p latest release tag: `libp2p-v0.56.0`

No upgrade is required for this story; implementation should target current repo versions.

### Project Context Reference

- No `project-context.md` file was found via `**/project-context.md`.
- Story context is derived from planning artifacts, implementation artifacts, current source code, and recent git history.

### Story Completion Status

- Ultimate context engine analysis completed - comprehensive developer guide created.
- Story status set to `ready-for-dev`.

### Project Structure Notes

- Role definition surfaces are currently in guild settings (`features/guild`) and role assignment surfaces are in member list (`features/members`).
- Server role routes are centralized under `/api/v1/guilds/{guild_slug}/roles` and `/api/v1/guilds/{guild_slug}/members/{member_user_id}/roles`.

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Story 5.6: Role Management Delegation]
- [Source: _bmad-output/planning-artifacts/epics.md#Story 5.4: Role Assignment to Members]
- [Source: _bmad-output/planning-artifacts/prd.md#Roles & Permissions]
- [Source: _bmad-output/planning-artifacts/architecture.md#Requirements to Structure Mapping]
- [Source: _bmad-output/planning-artifacts/architecture.md#REST API Response Format]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#User context menu (in member list or on username)]
- [Source: _bmad-output/implementation-artifacts/5-5-channel-level-permission-overrides.md]
- [Source: server/src/services/role_service.rs]
- [Source: server/src/handlers/roles.rs]
- [Source: server/src/handlers/mod.rs]
- [Source: server/src/permissions/mod.rs]
- [Source: server/src/models/role.rs]
- [Source: server/tests/server_binds_to_configured_port.rs]
- [Source: client/src/lib/features/guild/GuildSettings.svelte]
- [Source: client/src/lib/features/guild/GuildSettings.test.ts]
- [Source: client/src/lib/features/guild/permissions.ts]
- [Source: client/src/lib/features/members/MemberList.svelte]
- [Source: client/src/lib/features/members/MemberList.test.ts]
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

- Workflow engine loaded: `_bmad/core/tasks/workflow.xml`
- Workflow config loaded: `_bmad/bmm/workflows/4-implementation/dev-story/workflow.yaml`
- Story source parsed from user input: `5-6` -> `5-6-role-management-delegation`.
- Story marked `in-progress` in `_bmad-output/implementation-artifacts/sprint-status.yaml` before implementation.
- Implemented owner-only role-definition guards and delegated assignment tracing in `server/src/services/role_service.rs`.
- Extended integration/frontend tests for delegated manager boundaries and owner-only controls.
- Quality gates passing:
  - `cd client && npm run lint && npm run check && npm run test && npm run build`
  - `cd server && cargo fmt --check && cargo clippy -- -D warnings && cargo test`

### Completion Notes List

- Split server authorization so role definitions are owner-only while `update_member_roles` stays `MANAGE_ROLES`-gated with strict hierarchy checks.
- Added structured delegated-assignment tracing fields (`actor_user_id`, `target_user_id`, `guild_id`, `guild_slug`, `added_role_ids`, `removed_role_ids`) as Epic 8 mod-log hook points.
- Updated `MANAGE_ROLES` catalog copy to delegated-assignment scope and strengthened UI regression tests for owner-only settings and delegated assignment controls.
- Added server integration assertions that delegated managers with `MANAGE_ROLES` cannot create/update/delete/reorder roles and cannot assign equal/higher roles.

### File List

- server/src/services/role_service.rs
- server/tests/server_binds_to_configured_port.rs
- client/src/lib/features/guild/permissions.ts
- client/src/lib/features/guild/GuildSettings.test.ts
- client/src/lib/features/members/MemberList.test.ts
- _bmad-output/implementation-artifacts/5-6-role-management-delegation.md
- _bmad-output/implementation-artifacts/sprint-status.yaml

## Senior Developer Review (AI)

- Reviewer: Darko (GPT-5.3-Codex)
- Date: 2026-02-28
- Outcome: **Approve**
- Findings: No actionable HIGH/MEDIUM issues identified in implementation or tests.
- Validation: `cd client && npm run lint && npm run check && npm run test && npm run build && cd ../server && cargo fmt --check && cargo clippy -- -D warnings && cargo test`

## Change Log

- 2026-02-28: Implemented Story 5.6 delegation hardening, added delegated assignment tracing, expanded backend/frontend coverage, and passed full quality gates.
- 2026-02-28: Senior Developer Review (AI) completed in YOLO mode; no additional fixes required and story moved to done.
