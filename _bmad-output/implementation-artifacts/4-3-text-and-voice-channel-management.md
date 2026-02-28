# Story 4.3: Text and Voice Channel Management

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a **guild owner**,
I want to create, rename, reorder, and delete text and voice channels,
so that I can organize my guild's communication spaces.

## Acceptance Criteria

1. **Given** the user is the guild owner or has channel management permissions  
   **When** they click the "+" button next to a category header in the ChannelList  
   **Then** a channel creation dialog appears with name field and type selector (text/voice)

2. **Given** a valid channel create request  
   **When** creation succeeds  
   **Then** the channel is persisted and appears in ChannelList (channels table migration in this story)

3. **Given** a rendered channel list  
   **When** text and voice channels are displayed  
   **Then** text channels use a `#` icon and voice channels use a speaker icon

4. **Given** the user is authorized to manage channels  
   **When** they open the channel context menu and select "Edit channel"  
   **Then** they can rename the selected channel

5. **Given** the user is authorized to manage channels  
   **When** they drag-and-drop channels  
   **Then** channel ordering is persisted and reflected in ChannelList

6. **Given** the user is authorized to manage channels  
   **When** they select "Delete channel" from the context menu  
   **Then** a confirmation dialog is required before deletion

7. **Given** a channel delete confirmation is shown  
   **When** the user confirms deletion  
   **Then** the warning text includes: "This will permanently delete all messages in this channel"

8. **Given** a guild has many channels  
   **When** ChannelList is rendered  
   **Then** the channel area is scrollable and remains usable on desktop/tablet/mobile shell layouts

## Tasks / Subtasks

- [x] Task 1: Add channel persistence foundation and migration backfill (AC: 2, 5, 8)
  - [x] Create `server/migrations/0011_create_channels.sql` with a `channels` table (`id`, `guild_id`, `slug`, `name`, `channel_type`, `position`, timestamps), FK to `guilds`, and per-guild ordering/index constraints.
  - [x] Backfill one default channel row for existing guilds using each guild's `default_channel_slug` so Story 4.2 data stays valid.
  - [x] Keep schema naming/index conventions aligned with architecture (`snake_case`, plural tables, `idx_*` indexes).

- [x] Task 2: Implement channel model/service CRUD and reorder operations (AC: 2, 4, 5, 6, 7)
  - [x] Add `server/src/models/channel.rs` for DB queries (create/list/update/delete/reorder) with SQLite + PostgreSQL query branches.
  - [x] Add `server/src/services/channel_service.rs` for validation, owner/permission checks, and business rules (including safe deletion behavior and route-safe fallback handling for default channel).
  - [x] Enforce channel type constraints (`text` or `voice`) and normalized channel naming/slug rules.

- [x] Task 3: Expose authenticated channel endpoints under guild routes (AC: 1, 2, 4, 5, 6, 7)
  - [x] Add `server/src/handlers/channels.rs` and wire in `server/src/handlers/mod.rs`.
  - [x] Expose at minimum list/create/update/delete/reorder endpoints under `/api/v1/guilds/{guild_slug}/channels`.
  - [x] Preserve API envelope contract (`{ "data": ... }`) and `AppError` error envelope consistency.

- [x] Task 4: Add client channel API/types/store wiring (AC: 2, 5, 8)
  - [x] Add channel wire/domain types with explicit `snake_case` ↔ `camelCase` mapping.
  - [x] Add client channel API methods and a channel state holder in `client/src/lib/features/channel/`.
  - [x] Keep data flow aligned with existing guild patterns (`guildApi.ts` + `guildStore.svelte.ts`) to avoid parallel state models.

- [x] Task 5: Implement channel create UX in ChannelList (AC: 1, 2, 3, 8)
  - [x] Replace placeholder `ChannelList` channel rendering with server-backed channel list per guild.
  - [x] Add create-channel dialog with single-column form, blur-time validation, type selector (text/voice), Enter submit, and focus-safe close behavior.
  - [x] Ensure ChannelList remains scrollable with many channels and keeps active-channel highlighting.

- [x] Task 6: Implement channel context menu rename/delete flows (AC: 4, 6, 7)
  - [x] Add channel context menu actions: "Edit channel" and "Delete channel" with owner/permission gating.
  - [x] Implement rename flow (modal or inline edit) with explicit save/cancel behavior.
  - [x] Implement delete confirmation dialog with required permanent-message warning text exactly matching AC.

- [x] Task 7: Implement channel reordering interactions and persistence (AC: 5)
  - [x] Support drag-and-drop reorder in ChannelList with optimistic UI update + server persistence.
  - [x] Ensure reorder remains keyboard-accessible (focusable list items and non-pointer fallback actions where needed).
  - [x] Keep ordering behavior forward-compatible with Story 4.4 categories (do not block category introduction).

- [x] Task 8: Add server/client coverage and run quality gates (AC: all)
  - [x] Add server integration tests for auth requirements, create/list/update/delete/reorder success paths, and owner vs non-owner authorization.
  - [x] Add client tests for create dialog behavior, icon rendering by channel type, context menu rename/delete, delete warning text, and reorder state updates.
  - [x] Run quality gates:
    - [x] `cd client && npm run lint && npm run check && npm run test`
    - [x] `cd client && npm run build` (required so server tests can serve `client/dist/index.html`)
    - [x] `cd server && cargo fmt --check && cargo clippy -- -D warnings && cargo test`

## Dev Notes

### Developer Context

- Story 4.2 introduced guild persistence and owner authorization, but channel runtime data is still placeholder-only in `ChannelList.svelte` (derived from `defaultChannelSlug` + URL).
- Backend currently has guild endpoints but no `channels` handler/service/model modules yet.
- Shell routing (`/:guild/:channel`) and location persistence are already in place from Story 4.1 and must remain stable.

### Technical Requirements

- Require authentication on all channel mutation routes via `middleware::auth::AuthenticatedUser`.
- Enforce owner/permission checks server-side for create/update/delete/reorder operations; no client-side-only authorization.
- Keep REST request/response JSON fields `snake_case` at boundary and map to `camelCase` in client feature APIs.
- Preserve route-safe behavior when deleting/renaming channels currently in URL context (avoid leaving clients on invalid channel paths).
- Keep all error cases explicit with existing `AppError` contract; do not silently ignore invalid reorder/delete payloads.

### Architecture Compliance

1. Add channel domain code in architecture-aligned paths:
   - Server: `handlers/channels.rs`, `services/channel_service.rs`, `models/channel.rs`
   - Client: `features/channel/` for component + API + state
2. Keep REST structure under `/api/v1/guilds/{guild_slug}/channels` and preserve current Axum 0.8 path style already used in this repo.
3. Preserve layering discipline:
   - handlers = HTTP boundary
   - services = business logic/authorization
   - models = DB access
4. Keep frontend state boundaries coherent: channel server state via feature state/API helpers; avoid ad-hoc duplicated stores.

### Library & Framework Requirements

- Backend remains on Axum 0.8 + sqlx 0.8 with dual Postgres/SQLite support.
- Frontend remains on Svelte 5 runes + `@mateothegreat/svelte5-router` for SPA routing.
- No drag-and-drop dependency is currently present in `client/package.json`; prefer native implementation first unless a new dependency is justified and tested.
- Maintain existing fire/ice design language and dialog/context-menu behavior conventions.

### File Structure Requirements

Expected primary touch points:

- `server/migrations/0011_create_channels.sql` (new)
- `server/src/handlers/mod.rs`
- `server/src/handlers/channels.rs` (new)
- `server/src/services/mod.rs`
- `server/src/services/channel_service.rs` (new)
- `server/src/models/mod.rs`
- `server/src/models/channel.rs` (new)
- `server/tests/server_binds_to_configured_port.rs` and/or a new channel-focused integration test file
- `client/src/lib/features/channel/ChannelList.svelte`
- `client/src/lib/features/channel/channelApi.ts` (new)
- `client/src/lib/features/channel/channelStore.svelte.ts` (new)
- `client/src/lib/features/channel/types.ts` (new)
- `client/src/lib/features/channel/*.test.ts` (new/updated)
- `client/src/lib/features/shell/ShellRoute.svelte` (only if route fallback behavior requires shell integration updates)

### Testing Requirements

- Server:
  - Validate unauthenticated access is rejected for channel mutations.
  - Validate create/list/update/delete/reorder happy paths for owner.
  - Validate non-owner mutation rejection and consistent `FORBIDDEN` error envelope.
  - Validate ordering persistence and deterministic list order.
- Client:
  - Validate create dialog open/close, blur validation, Enter submit, and channel type selector behavior.
  - Validate channel icon rendering (`#` for text, speaker for voice).
  - Validate context menu rename/delete flows and mandatory delete warning copy.
  - Validate reorder UI state and persisted order after reload.
- Full quality gates must pass before moving to review.

### Previous Story Intelligence

- Story 4.2 established guild-domain implementation patterns: new migration + model/service/handler wiring with dual-DB SQL branches and `AuthenticatedUser` checks.
- Story 4.2 client patterns use Svelte 5 runes state objects (`guildState`) plus focused feature API modules and blur-time form validation in dialogs.
- Story 4.2 integration tests live in `server/tests/server_binds_to_configured_port.rs` and assert API envelopes/status codes directly; follow this style for channel tests.

### Git Intelligence Summary

- Recent implementation commit for Epic 4 (`22cf056`) touched guild handler/service/model plus `ChannelList.svelte` and shell integration; Story 4.3 should extend this same vertical slice pattern instead of introducing new architecture.
- A follow-up commit (`d123729`) only moved sprint status, confirming story-state changes are tracked in `_bmad-output/implementation-artifacts/sprint-status.yaml`.
- Latest repo commit (`e502990`) is BMAD/tooling metadata heavy; avoid coupling runtime channel implementation work to those automation files.

### Latest Technical Information

1. `@mateothegreat/svelte5-router` latest stable remains `2.16.19` and the repo already pins it; no router upgrade is required for this story.
2. Svelte latest stable is in the 5.53.x line, while this repo is pinned at `^5.45.2`; keep story work compatible with current pinned Svelte 5 runes usage (`$state`, `$derived`, `$effect`) unless an explicit dependency-upgrade story is approved.
3. Axum latest stable in 0.8 line is 0.8.8 and sqlx latest stable in 0.8 line is 0.8.6; current `0.8` constraints remain valid. Continue using current Axum 0.8 route parameter style in this codebase and existing sqlx runtime/TLS feature configuration.

### Project Context Reference

- No `project-context.md` was discovered via `**/project-context.md`.
- Story context is derived from epics, PRD, architecture, UX artifacts, recent Epic 4 story files, and current client/server source.

### Story Completion Status

- Ultimate context engine analysis completed — comprehensive developer guide created.
- Story status set to `ready-for-dev`.

### Project Structure Notes

- Story 4.3 should deliver channel CRUD/reorder foundations without prematurely implementing full category lifecycle (Story 4.4) or role bitflag engine (Epic 5).
- Keep changes cohesive around existing Epic 4 foundation files to reduce integration risk across shell routing and guild state.

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Epic 4: Guilds, Channels & Invites]
- [Source: _bmad-output/planning-artifacts/epics.md#Story 4.3: Text and Voice Channel Management]
- [Source: _bmad-output/planning-artifacts/prd.md#Guild Management]
- [Source: _bmad-output/planning-artifacts/prd.md#User Experience & Navigation]
- [Source: _bmad-output/planning-artifacts/prd.md#Non-Functional Requirements]
- [Source: _bmad-output/planning-artifacts/architecture.md#Implementation Patterns & Consistency Rules]
- [Source: _bmad-output/planning-artifacts/architecture.md#Requirements to Structure Mapping]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Journey 2: Guild Setup (Maya)]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#ChannelListItem]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Context Menu Patterns]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Form Patterns]
- [Source: _bmad-output/implementation-artifacts/4-2-guild-creation-and-settings.md]
- [Source: _bmad-output/implementation-artifacts/4-1-spa-navigation-shell-and-routing.md]
- [Source: client/src/lib/features/channel/ChannelList.svelte]
- [Source: client/src/lib/features/guild/GuildRail.svelte]
- [Source: client/src/lib/features/guild/guildStore.svelte.ts]
- [Source: client/src/lib/features/shell/ShellRoute.svelte]
- [Source: client/src/routes/routes.ts]
- [Source: server/src/handlers/mod.rs]
- [Source: server/src/handlers/guilds.rs]
- [Source: server/src/services/guild_service.rs]
- [Source: server/src/models/guild.rs]
- [Source: server/tests/server_binds_to_configured_port.rs]
- [Source: client/package.json]
- [Source: server/Cargo.toml]
- [Source: https://github.com/sveltejs/svelte/releases]
- [Source: https://www.npmjs.com/package/@mateothegreat/svelte5-router]
- [Source: https://github.com/mateothegreat/svelte5-router/releases]
- [Source: https://github.com/tokio-rs/axum/releases]
- [Source: https://crates.io/crates/axum]
- [Source: https://docs.rs/sqlx/latest/sqlx/]

## Dev Agent Record

### Agent Model Used

GPT-5.3-Codex (model ID: gpt-5.3-codex)

### Debug Log References

- Workflow engine loaded: `_bmad/core/tasks/workflow.xml`
- Workflow config loaded: `_bmad/bmm/workflows/4-implementation/dev-story/workflow.yaml`
- Validation run: `cd client && npm run lint && npm run check && npm run test && npm run build`
- Validation run: `cd server && cargo fmt --check && cargo clippy -- -D warnings && cargo test`

### Completion Notes List

- Implemented channel migration/model/service/handler wiring with authenticated CRUD + reorder endpoints and owner authorization enforcement.
- Replaced placeholder ChannelList with server-backed channel UI including create dialog, text/voice icons, context menu rename/delete, delete warning copy, and drag-drop + keyboard reorder flows.
- Added ChannelList client test coverage for create validation, icon rendering, rename/delete flows, warning text, and reorder controls.
- Added channel state cleanup on logout path to avoid stale cross-session channel data.
- Full quality gates passed for client and server.

### File List

- client/src/App.svelte
- client/src/lib/features/channel/ChannelList.svelte
- client/src/lib/features/channel/ChannelList.test.ts
- client/src/lib/features/channel/channelApi.ts
- client/src/lib/features/channel/channelStore.svelte.ts
- client/src/lib/features/channel/channelStore.test.ts
- client/src/lib/features/channel/types.ts
- server/migrations/0011_create_channels.sql
- server/src/handlers/channels.rs
- server/src/handlers/mod.rs
- server/src/models/channel.rs
- server/src/models/guild.rs
- server/src/models/mod.rs
- server/src/services/channel_service.rs
- server/src/services/guild_service.rs
- server/src/services/mod.rs
- server/tests/server_binds_to_configured_port.rs
- _bmad-output/implementation-artifacts/4-3-text-and-voice-channel-management.md
- _bmad-output/implementation-artifacts/sprint-status.yaml

## Senior Developer Review (AI)

### Reviewer

- Darko (AI)

### Review Date

- 2026-02-28

### Outcome

- Approve with fix applied

### Findings

1. **MEDIUM** — Channel list loading had a stale-response race risk: if guild A and guild B loads overlap, the older response could overwrite newer state and show wrong channels for the current route.
   - Proof: `client/src/lib/features/channel/channelStore.svelte` previously accepted whichever `listChannels` response completed last.
2. **MEDIUM** — Stale load failures could still bubble to UI callers: if an older guild request failed after a newer request won, the stale failure was re-thrown and could surface misleading errors.
   - Proof: `client/src/lib/features/channel/channelStore.svelte` previously re-threw stale request errors when `requestToken !== latestLoadRequestToken`.
3. **MEDIUM** — Legacy `channel_slugs` reorder requests assigned global positions, which can leave category-local position gaps after categories exist and break category-local move affordances.
   - Proof: `server/src/services/channel_service.rs` previously delegated `channel_slugs` to `channel::reorder_channels` without re-compacting positions per category.
4. **MEDIUM** — Drag-and-drop channel targeting in `ChannelList` used raw target position, which produced incorrect downward placement when dragging within the same category.
   - Proof: `client/src/lib/features/channel/ChannelList.svelte` previously called `moveChannel(..., target.position)` even though `moveChannel` removes the source before inserting.

### Fixes Applied

- Added load request token guarding in `channelStore` so stale `listChannels` responses are ignored.
- Added regression test `client/src/lib/features/channel/channelStore.test.ts` covering overlapped guild-load ordering.
- Updated stale-request error handling to ignore outdated failures instead of throwing them to active callers.
- Added regression test coverage for stale failure behavior when guild context changes.
- Updated `channel_slugs` reorder handling to convert slug order into per-category `channel_positions` updates before persistence.
- Added server integration coverage proving `channel_slugs` reorder keeps category-local positions compact (`server/tests/server_binds_to_configured_port.rs`).
- Adjusted same-category drag/drop target index calculation so downward drops land at the intended position after source removal.
- Added `ChannelList` regression coverage for downward same-category drag/drop targeting (`client/src/lib/features/channel/ChannelList.test.ts`).

### Validation

- `cd client && npm run lint && npm run check && npm run test && npm run build`
- `cd server && cargo fmt --check && cargo clippy -- -D warnings && cargo test`
- `cd server && cargo fmt --check && cargo clippy -- -D warnings && cargo test channels_owner_crud_reorder_and_default_fallback_work && cargo test categories_owner_crud_collapse_and_delete_move_channels_to_uncategorized`
- `cd client && npm run lint && npm run check && npm run test && npm run build` (post-follow-up verification)

## Change Log

- 2026-02-28: Story created and marked ready-for-dev with comprehensive implementation context for text and voice channel management.
- 2026-02-28: Implemented channel persistence + API + ChannelList management UX (create/rename/delete/reorder), added client coverage, and passed full client/server quality gates.
- 2026-02-28: Senior code review completed; fixed channel-store stale response race and added regression test coverage.
- 2026-02-28: Follow-up code review fixed stale-request error propagation in channel store and added regression test coverage.
- 2026-02-28: Follow-up YOLO review fixed `channel_slugs` reorder to preserve category-local contiguous positions and added regression integration coverage.
- 2026-02-28: Follow-up YOLO review fixed downward same-category drag/drop target positioning and added ChannelList regression coverage.
