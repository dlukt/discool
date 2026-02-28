# Story 4.5: Invite Link Generation and Management

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a **guild owner**,
I want to generate invite links so people can join my guild,
so that my community can grow through shared links.

## Acceptance Criteria

1. **Given** the user is the guild owner or has invite management permissions  
   **When** they click the "Invite People" button in the guild header or settings  
   **Then** an invite modal appears with options to generate a reusable or single-use invite link

2. **Given** an invite link is generated  
   **When** it is shown in the modal  
   **Then** the link is displayed with a "Copy" button

3. **Given** the user clicks "Copy"  
   **When** clipboard copy succeeds  
   **Then** a success toast/message is shown with text: "Invite link copied"

4. **Given** this story migration runs  
   **When** invite persistence is initialized  
   **Then** an invite links table exists with columns: `guild_id`, `code`, `type` (`reusable`/`single_use`), `uses_remaining`, `created_by`, `created_at`, `revoked`

5. **Given** active invite links exist for a guild  
   **When** an authorized owner opens invite management  
   **Then** they can view all active links with type, creator, and creation date (InviteLinkCard pattern)

6. **Given** an active invite link is listed  
   **When** an authorized owner revokes it  
   **Then** the link is marked revoked and no longer appears as active

7. **Given** a single-use link is generated  
   **When** it is shown in invite management  
   **Then** it shows remaining uses and is represented as invalid after use exhaustion (consumed in Story 4.6)

8. **Given** a guild owner wants to invite someone quickly  
   **When** they open invite UI  
   **Then** invite generation can be completed in at most two clicks (generate → copy)

## Tasks / Subtasks

- [x] Task 1: Add guild invite persistence and migration primitives (AC: 4, 5, 6, 7)
  - [x] Add `server/migrations/0013_create_guild_invites.sql` with `guild_invites` table and constraints for code uniqueness, type enum/check, non-negative `uses_remaining`, and `revoked` boolean.
  - [x] Index by `guild_id`, `revoked`, and `created_at` for fast active-link listing in owner UI.
  - [x] Keep migration SQL compatible with both SQLite and Postgres conventions already used in this repo.

- [x] Task 2: Implement invite model + service logic with explicit authorization hooks (AC: 1, 4, 5, 6, 7)
  - [x] Add `server/src/models/guild_invite.rs` with create/list-active/revoke operations and creator metadata join.
  - [x] Add `server/src/services/guild_invite_service.rs` for validation, owner/permission checks (owner-only for now, Epic 5 hook point for invite permission), and response shaping.
  - [x] Generate URL-safe invite codes with strong randomness; ensure conflicts retry cleanly without silent fallback behavior.

- [x] Task 3: Expose authenticated invite APIs under guild routes (AC: 1, 2, 5, 6, 7)
  - [x] Add `server/src/handlers/invites.rs` and wire in `server/src/handlers/mod.rs`.
  - [x] Add endpoints under `/api/v1/guilds/{guild_slug}/invites`:
    - `GET` list active invites
    - `POST` create invite (`type`)
    - `DELETE` revoke invite by code/slug
  - [x] Preserve API envelope contract (`{ "data": ... }`) and existing `AppError` mapping (`UNAUTHORIZED`, `FORBIDDEN`, `VALIDATION_ERROR`, etc.).

- [x] Task 4: Add client invite types + API wiring in guild feature modules (AC: 2, 5, 6, 7)
  - [x] Extend `client/src/lib/features/guild/types.ts` with invite wire/domain types and snake_case ↔ camelCase converters.
  - [x] Extend `client/src/lib/features/guild/guildApi.ts` with list/create/revoke invite methods.
  - [x] Keep invite management state scoped to guild feature surfaces and avoid creating conflicting duplicate state containers.

- [x] Task 5: Implement InviteModal and integrate entry points in shell/guild surfaces (AC: 1, 2, 3, 8)
  - [x] Create `client/src/lib/features/guild/InviteModal.svelte` with reusable/single-use selector, generate action, copy action, and active invite list.
  - [x] Add "Invite people" triggers in `ShellRoute.svelte` desktop/mobile channel headers (alongside existing guild controls) and optionally in guild settings surface.
  - [x] Ensure copy feedback is user-visible and accessible (`aria-live`) with exact success copy requirement.

- [x] Task 6: Implement revoke and invite card UX details (AC: 5, 6, 7)
  - [x] Render invite cards with link preview/truncation, type badge, uses remaining (single-use), creator, and creation time.
  - [x] Add destructive revoke action using existing fire/destructive visual semantics.
  - [x] Ensure revoked invites are removed/refreshed deterministically without stale UI state.

- [x] Task 7: Add integration/unit tests and run quality gates (AC: all)
  - [x] Extend `server/tests/server_binds_to_configured_port.rs` for invite auth requirements, owner-only mutation checks, create/list/revoke flows, and single-use metadata behavior.
  - [x] Add/extend client tests (`InviteModal.test.ts`, `ShellRoute.test.ts`, and guild API/type tests as needed) for generate/copy/revoke UX and entry-point visibility.
  - [x] Run quality gates:
    - [x] `cd client && npm run lint && npm run check && npm run test`
    - [x] `cd client && npm run build`
    - [x] `cd server && cargo fmt --check && cargo clippy -- -D warnings && cargo test`

## Dev Notes

### Developer Context

- Story 4.4 completed category-aware channel management; guild/channel shell is already functional and owner-gated.
- There is currently no invite persistence, invite API route, or invite UI component in runtime code.
- `App.svelte` already parses `invite` and `guild_name` query params for cross-instance join prompts; Story 4.5 should generate links compatible with the upcoming Story 4.6 join flow without implementing membership join in this story.

### Technical Requirements

- Require authentication on invite endpoints via `AuthenticatedUser`; enforce owner authorization server-side until Epic 5 permissions are available.
- Keep API boundary fields in `snake_case`; map to `camelCase` in client types/helpers.
- Return only active invites in list endpoints (`revoked = false` and not consumed where applicable).
- Keep error handling explicit via `AppError`; no silent ignores for invalid invite type/revoke payloads.
- Support deterministic invite URL generation format (recommended canonical path: `/invite/{code}`) while allowing client-side share text context (`guild_name`) for existing join prompt compatibility.

### Architecture Compliance

1. Follow established layering:
   - handlers = HTTP boundary and payload validation
   - services = authorization + business rules
   - models = SQL access
2. Keep routes nested under `/api/v1/guilds/{guild_slug}/...` like channels/categories.
3. Implement dual SQL branches for Postgres and SQLite in new model queries.
4. Preserve response envelope format (`data`/`error`) and current error-code taxonomy.

### Library & Framework Requirements

- Frontend stays on Svelte 5 runes conventions (`$state`, `$derived`, `$effect`) and existing router patterns.
- Backend stays on Axum 0.8 + sqlx 0.8 patterns used by current guild/channel/category modules.
- Avoid introducing new UI/state dependencies unless strictly needed; existing modal and feedback patterns are sufficient for this story.

### File Structure Requirements

Expected primary touch points:

- `server/migrations/0013_create_guild_invites.sql` (new)
- `server/src/models/mod.rs`
- `server/src/models/guild_invite.rs` (new)
- `server/src/services/mod.rs`
- `server/src/services/guild_invite_service.rs` (new)
- `server/src/handlers/mod.rs`
- `server/src/handlers/invites.rs` (new)
- `server/tests/server_binds_to_configured_port.rs`
- `client/src/lib/features/guild/InviteModal.svelte` (new)
- `client/src/lib/features/guild/InviteModal.test.ts` (new)
- `client/src/lib/features/guild/guildApi.ts`
- `client/src/lib/features/guild/types.ts`
- `client/src/lib/features/shell/ShellRoute.svelte`
- `client/src/lib/features/shell/ShellRoute.test.ts`

### Testing Requirements

- Server:
  - Verify unauthenticated invite mutations return `401`.
  - Verify non-owner invite create/revoke returns `403`.
  - Verify owner create/list/revoke workflows and active-link filtering.
  - Verify single-use link metadata (`type`, `uses_remaining`) is persisted and surfaced correctly.
- Client:
  - Verify invite modal opens from shell entry points in channel mode.
  - Verify reusable vs single-use generation payloads.
  - Verify copy action feedback text ("Invite link copied") and clipboard interaction handling.
  - Verify revoke action updates/removes link cards and surfaces API errors.

### Previous Story Intelligence

- Story 4.4 established category-aware channel rendering and server route patterns under `/api/v1/guilds/{guild_slug}/...`; invite implementation should follow the same vertical slice design.
- Existing server integration tests in `server_binds_to_configured_port.rs` already cover auth and owner checks for guild/channel/category mutations; mirror that style for invite scenarios.
- `ShellRoute.svelte` already hosts guild-level action buttons (`Guild settings`, `Settings`, `Log out`), making it the safest integration point for "Invite people" UX.

### Git Intelligence Summary

- `599b44e feat: finalize channel and category management` confirms the latest product surface in Epic 4 is complete and ready for invite-focused extension.
- `22cf056 feat: implement story 4-2 guild creation and settings` anchors guild-domain conventions (handlers/services/models/tests) that invite implementation should reuse.
- Recent BMAD/status commits (`e502990`, `d123729`) indicate sprint artifacts are maintained alongside implementation progress; keep sprint status transitions aligned with workflow.

### Latest Technical Information

1. Current repo pins:
   - `svelte`: `^5.45.2`
   - `@mateothegreat/svelte5-router`: `^2.16.19`
   - `axum`: `0.8`
   - `sqlx`: `0.8`
2. Current upstream stable lines (researched) are compatible with repo choices:
   - Svelte `5.53.6`
   - `@mateothegreat/svelte5-router` `2.16.19`
   - Axum `0.8.8`
   - SQLx `0.8.6`
3. No dependency upgrade is required for this story; implement invites against currently pinned versions.

### Project Context Reference

- No `project-context.md` was discovered via `**/project-context.md`.
- Story context is derived from planning artifacts, prior Epic 4 implementation artifacts, and current client/server runtime code.

### Story Completion Status

- Ultimate context analysis completed — comprehensive developer guide created.
- Story status set to `ready-for-dev`.

### Project Structure Notes

- Keep Story 4.5 scoped to invite generation/list/revoke only; actual join/redeem flow remains Story 4.6 scope.
- Design invite records/API so Story 4.6 can consume them directly for membership creation and invalid/expired handling without schema churn.

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Epic 4: Guilds, Channels & Invites]
- [Source: _bmad-output/planning-artifacts/epics.md#Story 4.5: Invite Link Generation and Management]
- [Source: _bmad-output/planning-artifacts/prd.md#Guild Management]
- [Source: _bmad-output/planning-artifacts/architecture.md#Implementation Patterns & Consistency Rules]
- [Source: _bmad-output/planning-artifacts/architecture.md#Complete Project Directory Structure]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#InviteLinkCard]
- [Source: _bmad-output/implementation-artifacts/4-4-channel-categories.md]
- [Source: client/src/lib/features/shell/ShellRoute.svelte]
- [Source: client/src/lib/features/guild/GuildSettings.svelte]
- [Source: client/src/lib/features/guild/guildApi.ts]
- [Source: client/src/lib/features/guild/types.ts]
- [Source: client/src/App.svelte]
- [Source: server/src/handlers/mod.rs]
- [Source: server/src/handlers/guilds.rs]
- [Source: server/src/services/guild_service.rs]
- [Source: server/src/services/category_service.rs]
- [Source: server/src/models/guild.rs]
- [Source: server/tests/server_binds_to_configured_port.rs]
- [Source: server/migrations/0010_create_guilds.sql]
- [Source: server/migrations/0011_create_channels.sql]
- [Source: server/migrations/0012_create_channel_categories.sql]
- [Source: https://registry.npmjs.org/svelte/latest]
- [Source: https://registry.npmjs.org/@mateothegreat/svelte5-router/latest]
- [Source: https://api.github.com/repos/tokio-rs/axum/releases/latest]
- [Source: https://docs.rs/crate/sqlx/latest]

## Dev Agent Record

### Agent Model Used

GPT-5.3-Codex (model ID: gpt-5.3-codex)

### Debug Log References

- Workflow engine loaded: `_bmad/core/tasks/workflow.xml`
- Workflow config loaded: `_bmad/bmm/workflows/4-implementation/dev-story/workflow.yaml`
- Implemented backend invite migration/model/service/handler stack and wired routes under `/api/v1/guilds/{guild_slug}/invites`.
- Implemented client invite types/API wiring, InviteModal UX, and shell invite entry points.
- Quality gates executed successfully:
  - `cd client && npm run lint && npm run check && npm run test`
  - `cd client && npm run build`
  - `cd server && cargo fmt --check && cargo clippy -- -D warnings && cargo test`

### Completion Notes List

- Added invite persistence (`guild_invites`) with cross-DB-compatible constraints and indexes for owner invite management queries.
- Added owner-gated invite service + API endpoints for list/create/revoke with explicit validation and deterministic canonical invite URLs.
- Added InviteModal UI with reusable/single-use generation, copy feedback (`Invite link copied`), and deterministic revoke removal behavior.
- Added server/client tests covering auth, owner authorization, metadata, copy/revoke UX, and shell invite entry-point visibility.

### File List

- client/src/lib/features/guild/guildApi.test.ts
- client/src/lib/features/guild/guildApi.ts
- client/src/lib/features/guild/InviteModal.svelte
- client/src/lib/features/guild/InviteModal.test.ts
- client/src/lib/features/guild/types.ts
- client/src/lib/features/shell/ShellRoute.svelte
- client/src/lib/features/shell/ShellRoute.test.ts
- _bmad-output/implementation-artifacts/sprint-status.yaml
- _bmad-output/implementation-artifacts/4-5-invite-link-generation-and-management.md
- server/migrations/0013_create_guild_invites.sql
- server/src/handlers/invites.rs
- server/src/handlers/mod.rs
- server/src/models/guild_invite.rs
- server/src/models/mod.rs
- server/src/services/guild_invite_service.rs
- server/src/services/mod.rs
- server/tests/server_binds_to_configured_port.rs

### Senior Developer Review (AI)

- Reviewer: Darko (AI) — 2026-02-28
- Outcome: Approved with no findings.
- Git vs Story Discrepancies: None.
- Findings:
  - None.
- Acceptance Criteria verification:
  - AC1 ✅ Invite modal entry point and reusable/single-use generation controls are implemented for channel header flows.
  - AC2 ✅ Generated links are rendered with a copy action in invite cards.
  - AC3 ✅ Copy success feedback is surfaced with exact text `Invite link copied` and `aria-live` announcement.
  - AC4 ✅ `0013_create_guild_invites.sql` creates invite persistence columns/constraints and indexes.
  - AC5 ✅ Authorized owners can list active invite links with type, creator, and creation timestamp metadata.
  - AC6 ✅ Revoke flow marks invites revoked server-side and removes revoked links from active list UI.
  - AC7 ✅ Single-use metadata (`uses_remaining`) is persisted/surfaced and represented in UI.
  - AC8 ✅ Invite generation flow in modal remains two-click (`Generate invite` → `Copy`) once open.

### Change Log

- 2026-02-28: Implemented Story 4.5 invite generation/management backend and frontend flows, added tests, and moved status to `review`.
- 2026-02-28: Senior developer adversarial review completed with no findings; story marked `done`.
