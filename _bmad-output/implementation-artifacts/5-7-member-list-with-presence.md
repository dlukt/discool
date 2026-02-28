# Story 5.7: Member List with Presence

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a **user**,
I want to see who's in my guild and whether they're online,
so that I know who's available and the guild feels alive.

## Acceptance Criteria

1. **Given** a user is viewing a guild  
   **When** the member list panel is visible (right sidebar, 240px)  
   **Then** members are grouped by their highest role (role name as section header, colored)

2. **Given** members are grouped by role  
   **When** role sections render  
   **Then** online members appear first, then offline members within each group

3. **Given** each member entry is rendered  
   **When** the row is visible  
   **Then** it shows avatar (32px) + status dot (green=online, yellow=idle, gray=offline) + username colored by highest role

4. **Given** a member entry is clicked  
   **When** the profile popover opens  
   **Then** it shows avatar, username, roles, a **Send DM** button, and moderation actions if the viewer has permissions

5. **Given** users connect/disconnect  
   **When** presence changes occur  
   **Then** the member list updates in real time as users come online/offline

6. **Given** presence is tracked  
   **When** connection state changes  
   **Then** presence is derived from WebSocket connection state (connected=online, disconnect timeout=offline)

7. **Given** the guild has many members  
   **When** the list is scrolled  
   **Then** the member list uses virtual scrolling for performance

8. **Given** assistive technologies are used  
   **When** role groups and member rows are announced  
   **Then** screen readers announce member count and status for each role group

## Tasks / Subtasks

- [x] Task 1: Extend guild member contract with presence-ready fields (AC: 2, 3, 5, 6)
  - [x] Add presence status fields to server member DTOs in `server/src/services/role_service.rs` and wire types in `client/src/lib/features/guild/types.ts`.
  - [x] Keep role assignment payload shape and existing fields (`can_assign_roles`, `assignable_role_ids`, highest-role color) backward compatible.
  - [x] Preserve existing response envelope contracts (`{ "data": ... }` success, structured `{ "error": ... }` failures).

- [x] Task 2: Implement connection-state presence tracking with WebSocket compatibility (AC: 5, 6)
  - [x] Replace `/ws` not-found stub with authenticated WebSocket handling that can track per-user connection state without coupling to future message features.
  - [x] Track online/idle/offline transitions from connect/heartbeat/disconnect timeout events.
  - [x] Broadcast `presence_update` events using existing event naming conventions (`snake_case`, envelope with `op`/`d`) so Story 6.1 can extend rather than rewrite.
  - [x] Keep logic structured for reuse by Epic 6 real-time gateway implementation.

- [x] Task 3: Add client presence store and real-time merge path (AC: 2, 5, 6)
  - [x] Introduce a presence state source (feature-local or global store) that merges initial REST member data with live `presence_update` events.
  - [x] Handle reconnect and timeout transitions deterministically; avoid stale online states after disconnect.
  - [x] Keep state boundaries clear: REST in guild/member store, live presence in presence store.

- [x] Task 4: Upgrade member list UI for grouping, sorting, and virtualization (AC: 1, 2, 3, 7, 8)
  - [x] Refactor `client/src/lib/features/members/MemberList.svelte` to group members by highest role with role headers.
  - [x] Sort online first within each group, then stable username order to avoid row jitter.
  - [x] Render status dot and accessible status text per member while preserving highest-role username color rendering.
  - [x] Add virtualized/windowed rendering for large guilds while preserving 240px sidebar behavior across desktop/tablet/mobile surfaces.
  - [x] Announce role-group counts/status to screen readers and preserve keyboard/context-menu navigation.

- [x] Task 5: Expand profile popover actions without cross-epic regressions (AC: 4)
  - [x] Extend popover content to include avatar, username, and role chips/summary.
  - [x] Add **Send DM** action entrypoint compatible with Story 6.9 (navigates or opens DM intent; no duplicate DM data model in this story).
  - [x] Render moderation actions conditionally by permission (`MUTE_MEMBERS`, `KICK_MEMBERS`, `BAN_MEMBERS`, `MANAGE_MESSAGES`) with explicit non-available handling until Epic 8 endpoints land.
  - [x] Preserve Story 5.4/5.6 assign-role flow and delegated role constraints in the same popover/context menu surface.

- [x] Task 6: Add coverage and run quality gates (AC: all)
  - [x] Extend `server/tests/server_binds_to_configured_port.rs` for member-list response shape, presence transitions, and guild-member authorization constraints.
  - [x] Extend `client/src/lib/features/members/MemberList.test.ts` for grouping order, status-dot rendering, popover action visibility, and keyboard/screen-reader behavior.
  - [x] Add/extend ShellRoute tests (`client/src/lib/features/shell/ShellRoute.test.ts`) to keep responsive member panel behavior stable.
  - [x] Run quality gates:
    - [x] `cd client && npm run lint && npm run check && npm run test && npm run build`
    - [x] `cd server && cargo fmt --check && cargo clippy -- -D warnings && cargo test`

## Dev Notes

### Developer Context

- Story 5.4 replaced the MemberList placeholder with API-backed rows and delegated role assignment controls, and Story 5.6 tightened owner/delegated authorization boundaries.
- Current member endpoint (`GET /api/v1/guilds/{guild_slug}/members`) returns role/assignment data but does not expose presence state yet.
- Current server route wiring still maps `/ws` to `ws_not_found`, so this story must introduce presence-capable real-time plumbing without conflicting with Epic 6 gateway plans.
- Shell layout already reserves a 240px member panel in desktop/tablet flows; this story should preserve that UX contract while improving content density and performance.

### Technical Requirements

- Presence statuses must be explicit and deterministic (`online`, `idle`, `offline`) with offline fallback when no active connection is known.
- Connection-state presence must be server-authoritative (not just client heuristics) and derived from WebSocket lifecycle + timeout.
- Member list grouping must use highest role authority/color semantics already established in role service outputs.
- Existing delegated role assignment behavior (`can_manage_roles`, `can_assign_roles`, assignable role filtering) must remain unchanged.
- `Send DM` and moderation actions must be integrated as action entrypoints, not parallel feature implementations that duplicate Epic 6/8 scopes.
- Keep accessibility parity: keyboard context menu (`ContextMenu` / `Shift+F10`), focus management, and announced status/group counts.

### Architecture Compliance

1. Keep Axum layering intact: route wiring in handlers, business rules in services, persistence in models.
2. Keep permission checks centralized in `server/src/permissions/mod.rs`; do not duplicate permission logic in UI.
3. Preserve API contracts and naming patterns (snake_case wire, typed client mappers in `features/guild/types.ts`).
4. Keep real-time event naming aligned with architecture guidance (`presence_update`, envelope with operation/data metadata).
5. Preserve feature boundaries in client (`features/members`, `features/guild`, `features/shell`) and avoid state duplication.

### Library & Framework Requirements

- Frontend: Svelte 5 runes + existing feature-store patterns.
- Backend: Axum 0.8 + sqlx 0.8 + existing permission/cache helpers.
- Use existing UI primitives and scroll patterns first; avoid introducing new virtualization/state libraries unless existing primitives cannot satisfy AC7.
- No dependency upgrade is required for this story; implement against currently pinned versions.

### File Structure Requirements

Expected primary touch points:

- `server/src/handlers/mod.rs`
- `server/src/services/role_service.rs`
- `server/src/models/guild_member.rs`
- `server/src/permissions/mod.rs`
- `server/src/lib.rs` (if shared presence tracker state is introduced)
- `server/tests/server_binds_to_configured_port.rs`
- `client/src/lib/features/members/MemberList.svelte`
- `client/src/lib/features/members/MemberList.test.ts`
- `client/src/lib/features/guild/types.ts`
- `client/src/lib/features/guild/guildApi.ts`
- `client/src/lib/features/guild/guildStore.svelte.ts`
- `client/src/lib/features/shell/ShellRoute.svelte`
- `client/src/lib/features/shell/ShellRoute.test.ts`
- `client/src/lib/stores/presenceStore.svelte.ts` (new, if created)

### Testing Requirements

- Server integration tests:
  - unauthorized member list/presence access remains `401/403`-correct,
  - member list payload includes expected presence fields and role-group data inputs,
  - connect/disconnect timeout transitions update emitted presence state correctly.
- Server unit tests:
  - presence transition state machine (connect -> online, inactivity -> idle/offline),
  - timeout and reconnection edge cases.
- Frontend tests:
  - grouped render by role and online-first ordering within groups,
  - status-dot color/label semantics for online/idle/offline,
  - popover includes DM/moderation action visibility by permission,
  - virtualization renders only visible windows while preserving navigation/accessibility semantics.

### Previous Story Intelligence

- Story 5.4 established member list API integration, context-menu access patterns, and assign-role flows that must remain intact.
- Story 5.5 reinforced keyboard context-menu parity (`ContextMenu`, `Shift+F10`) and permission-gated channel/member interaction patterns.
- Story 5.6 enforced owner-only role-definition mutations while preserving delegated assignment controls; this story must not loosen those boundaries.
- Story 5.4 explicitly noted Story 5.7 would expand the member surface with presence, so this implementation should extend—not replace—the existing member list foundation.

### Git Intelligence Summary

- `afa9352 feat: finalize story 5-6 role management delegation`
- `2491aec feat: finalize story 5-5 channel permission overrides`
- `68f87aa feat: finalize story 5-4 role assignment to members`
- `f9149f0 feat: finalize story 5-3 role hierarchy and ordering`
- `1f41931 feat: finalize story 5-2 permission bitflag engine`

### Latest Technical Information

Current pinned runtime lines in this repo:

- `svelte`: `^5.45.2`
- `@mateothegreat/svelte5-router`: `^2.16.19`
- `axum`: `0.8`
- `sqlx`: `0.8`
- `libp2p`: `0.56`

Latest stable lines checked during story creation:

- Svelte latest: `5.53.6`
- `@mateothegreat/svelte5-router` latest: `2.16.19`
- Axum latest release tag: `axum-v0.8.8`
- SQLx latest tag: `v0.8.6`
- rust-libp2p latest release tag: `libp2p-v0.56.0`

No dependency upgrade is required for Story 5.7; implementation should target current repo versions.

### Project Context Reference

- No `project-context.md` file was discovered via `**/project-context.md`.
- Story context is derived from planning artifacts, previous implementation artifacts in Epic 5, current source code, and recent git history.

### Story Completion Status

- Ultimate context engine analysis completed - comprehensive developer guide created.
- Story status set to `ready-for-dev`.

### Project Structure Notes

- `MemberList` is already mounted in desktop, tablet overlay, and mobile panel flows from `ShellRoute`; all surfaces must stay consistent after refactor.
- Current backend member APIs do not include presence and current `/ws` is a stub; Story 5.7 should add foundational presence plumbing that Story 6.1 can build on.
- DM and moderation action entrypoints should align with Story 6.9 and Epic 8 scopes without duplicating those domain implementations here.

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Story 5.7: Member List with Presence]
- [Source: _bmad-output/planning-artifacts/epics.md#Story 6.9: Direct Messages]
- [Source: _bmad-output/planning-artifacts/epics.md#Epic 8 (moderation stories)]
- [Source: _bmad-output/planning-artifacts/prd.md#User Experience & Navigation (FR57, FR58)]
- [Source: _bmad-output/planning-artifacts/architecture.md#WebSocket Event Naming]
- [Source: _bmad-output/planning-artifacts/architecture.md#State Boundaries (Frontend)]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Layout structure (desktop)]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#MemberListEntry]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#User context menu (in member list or on username)]
- [Source: _bmad-output/implementation-artifacts/5-4-role-assignment-to-members.md]
- [Source: _bmad-output/implementation-artifacts/5-6-role-management-delegation.md]
- [Source: _bmad-output/implementation-artifacts/sprint-status.yaml]
- [Source: server/src/handlers/mod.rs]
- [Source: server/src/services/role_service.rs]
- [Source: server/src/permissions/mod.rs]
- [Source: client/src/lib/features/members/MemberList.svelte]
- [Source: client/src/lib/features/members/MemberList.test.ts]
- [Source: client/src/lib/features/guild/types.ts]
- [Source: client/src/lib/features/guild/guildApi.ts]
- [Source: client/src/lib/features/guild/guildStore.svelte.ts]
- [Source: client/src/lib/features/shell/ShellRoute.svelte]
- [Source: client/src/lib/features/shell/ShellRoute.test.ts]
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
- Implemented authenticated `/ws` handler and reusable server presence tracker (`online`/`idle`/`offline`) with heartbeat + timeout transitions and `presence_update` broadcasts.
- Added client presence store merge path and refactored member list rendering for role-grouped sorting, virtualized rendering, status indicators, and expanded popover actions.
- Ran full repo quality gate: `cd client && npm run lint && npm run check && npm run test && npm run build && cd ../server && cargo fmt --check && cargo clippy -- -D warnings && cargo test`.

### Completion Notes List

- ✅ Extended member payload contracts with `presence_status` while preserving existing role-assignment fields and API envelopes.
- ✅ Replaced `/ws` stub with authenticated WebSocket handling and presence lifecycle tracking/broadcast foundations reusable for Epic 6 gateway work.
- ✅ Added `presenceStore` and merged live presence events with REST member data without duplicating guild/member store ownership.
- ✅ Upgraded `MemberList` to grouped, online-first, virtualized rendering with accessibility labels and keyboard/context-menu parity preserved.
- ✅ Expanded profile popover with avatar/user details, role chips, Send DM intent dispatch, permission-gated moderation placeholders, and preserved assign-role flow.
- ✅ Expanded tests for member-list grouping/status/action behavior, ShellRoute member-rail stability, and server presence/auth contract coverage.

### File List

- client/src/lib/features/channel/ChannelList.test.ts
- client/src/lib/features/guild/guildApi.test.ts
- client/src/lib/features/guild/types.ts
- client/src/lib/features/members/MemberList.svelte
- client/src/lib/features/members/MemberList.test.ts
- client/src/lib/features/members/presenceStore.svelte.ts
- client/src/lib/features/shell/ShellRoute.test.ts
- server/src/handlers/admin.rs
- server/src/handlers/mod.rs
- server/src/handlers/ws.rs
- server/src/services/mod.rs
- server/src/services/presence_service.rs
- server/src/services/role_service.rs
- server/tests/server_binds_to_configured_port.rs
- _bmad-output/implementation-artifacts/5-7-member-list-with-presence.md
- _bmad-output/implementation-artifacts/sprint-status.yaml

## Senior Developer Review (AI)

### Reviewer

Darko (AI-assisted review) on 2026-02-28

### Outcome

Changes requested during adversarial review were fixed in this pass. Final decision: **Approve**.

### Findings

- [MEDIUM][Fixed] Removed duplicate presence seeding in `client/src/lib/features/members/MemberList.svelte` to avoid redundant state churn during member load.
- [MEDIUM][Fixed] Hardened heartbeat handling in `server/src/services/presence_service.rs` so disconnected/unknown sessions cannot revive presence state.

### Validation

- ✅ `cd client && npm run lint && npm run check && npm run test && npm run build && cd ../server && cargo fmt --check && cargo clippy -- -D warnings && cargo test`

## Change Log

- 2026-02-28: Implemented Story 5.7 member presence foundation across server and client, added virtualization/grouped member UI updates, and passed full client/server quality gates.
- 2026-02-28: Senior code review completed; fixed duplicate presence seeding and heartbeat revival edge case; story approved and moved to done.
