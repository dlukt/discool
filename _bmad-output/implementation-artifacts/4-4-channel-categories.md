# Story 4.4: Channel Categories

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a **guild owner**,
I want to organize channels into collapsible categories,
so that my guild's channel list stays structured as it grows.

## Acceptance Criteria

1. **Given** the user is the guild owner or has channel management permissions  
   **When** they click "Create Category" in the channel list  
   **Then** a category is created with a name (uppercase, small text display)

2. **Given** channels and categories are visible in ChannelList  
   **When** the user drags channels  
   **Then** channels can be moved into categories and reordered predictably

3. **Given** category headers are visible  
   **When** the user clicks the chevron or category name  
   **Then** categories toggle between expanded/collapsed states

4. **Given** a category is collapsed  
   **When** the user returns to the guild  
   **Then** the category remains collapsed for that user and its channels stay hidden until expanded

5. **Given** category headers are rendered  
   **When** the viewer is a guild owner  
   **Then** the "+" button to create channels is always visible beside each category header  
   **And** for non-owners, it follows hover visibility rules

6. **Given** an existing category  
   **When** an authorized user renames or deletes it  
   **Then** rename is persisted  
   **And** deleting the category moves its channels to uncategorized instead of deleting channels

7. **Given** keyboard-only navigation in ChannelList  
   **When** a category header is focused  
   **Then** Enter toggles expand/collapse  
   **And** arrow keys navigate through children in logical order

## Tasks / Subtasks

- [x] Task 1: Add category persistence and channel-category relationship schema (AC: 1, 2, 4, 6)
  - [x] Add `server/migrations/0012_create_channel_categories.sql` with a `channel_categories` table (`id`, `guild_id`, `slug`, `name`, `position`, timestamps) and per-guild slug uniqueness/indexes.
  - [x] Extend `channels` persistence for category grouping (nullable category reference) while preserving existing Story 4.3 channel rows as uncategorized.
  - [x] Add per-user collapse persistence storage keyed by user + guild + category to satisfy collapsed-state retention.

- [x] Task 2: Implement category model/service operations with explicit validation (AC: 1, 2, 3, 4, 6)
  - [x] Add model queries for create/list/update/delete/reorder category behavior and channel assignment movement.
  - [x] Extend channel service logic to support category assignment updates while preserving deterministic channel ordering.
  - [x] Keep validation explicit (name required/length bounded, no duplicate slugs per guild, no silent fallbacks).

- [x] Task 3: Expose authenticated category APIs under guild routes (AC: 1, 2, 3, 4, 6)
  - [x] Add `server/src/handlers/categories.rs` and wire routes in `server/src/handlers/mod.rs` under `/api/v1/guilds/{guild_slug}/categories`.
  - [x] Preserve response envelope contract (`{ "data": ... }`) and existing `AppError` mapping patterns.
  - [x] Keep mutation authorization aligned with current channel-management enforcement and leave clear hook points for Epic 5 permission-engine integration.

- [x] Task 4: Extend channel client types/API/store for category-aware state (AC: 1, 2, 4, 6)
  - [x] Update `client/src/lib/features/channel/types.ts` with category domain/wire types and `snake_case` ↔ `camelCase` mapping.
  - [x] Extend `client/src/lib/features/channel/channelApi.ts` with category CRUD, channel-to-category movement, and collapse-state endpoints.
  - [x] Extend `client/src/lib/features/channel/channelStore.svelte.ts` to manage categories + grouped channels while retaining stale-response guard behavior.

- [x] Task 5: Refactor ChannelList to render category groups and interactions (AC: 1, 2, 3, 5, 7)
  - [x] Replace flat list rendering in `ChannelList.svelte` with category headers + grouped channel items + uncategorized section.
  - [x] Implement chevron/name toggle behavior with accessible `aria-expanded` state and logical focus order.
  - [x] Add/adjust create-channel trigger placement per category header visibility rules.

- [x] Task 6: Implement category rename/delete UX and uncategorized fallback behavior (AC: 6)
  - [x] Add category action affordances (rename/delete) consistent with existing dialog and context-menu patterns.
  - [x] Ensure delete flow clearly communicates that channels are moved to uncategorized (not removed).
  - [x] Keep error cases explicit and surfaced in UI alerts.

- [x] Task 7: Implement category-aware drag/drop and keyboard navigation (AC: 2, 7)
  - [x] Extend current drag/drop logic to support moving channels across category boundaries and within category order.
  - [x] Preserve non-pointer fallback controls (move up/down) with category-aware semantics where practical.
  - [x] Implement Enter/arrow-key semantics for category expand/collapse and child traversal per UX spec.

- [x] Task 8: Add server/client test coverage and run quality gates (AC: all)
  - [x] Extend server integration tests in `server/tests/server_binds_to_configured_port.rs` for category CRUD, auth/forbidden checks, collapse persistence, and delete-to-uncategorized behavior.
  - [x] Extend client tests (`ChannelList.test.ts`, `channelStore.test.ts`) for grouped rendering, collapse persistence, category DnD moves, and keyboard behavior.
  - [x] Run quality gates:
    - [x] `cd client && npm run lint && npm run check && npm run test`
    - [x] `cd client && npm run build`
    - [x] `cd server && cargo fmt --check && cargo clippy -- -D warnings && cargo test`

## Dev Notes

### Developer Context

- Story 4.3 already shipped authenticated channel CRUD + reorder with a **flat** channel list and no category domain objects in server/client runtime code.
- Current channel schema (`0011_create_channels.sql`) has no category relationship and no per-user collapse state storage.
- Existing shell routing and channel fallback behavior (`/:guild/:channel`) must remain stable while introducing grouped channel rendering.

### Technical Requirements

- Maintain API boundary and naming conventions: JSON payloads in `snake_case`, client domain objects in `camelCase`.
- Category names must use current validation strictness patterns (required, bounded length, control-char rejection), while display formatting (uppercase/small text) is a presentation concern.
- Category collapse state must persist per user (not global per guild) and be restored on load.
- Deleting a category must re-home channels into uncategorized; no channel/message deletion side effects.
- Preserve explicit error handling through `AppError`; do not hide invalid reorder/move payload failures.

### Architecture Compliance

1. Follow existing server layering:
   - handlers = HTTP/request validation
   - services = authorization + business rules
   - models = SQL access
2. Keep routes under `/api/v1/guilds/{guild_slug}/...` with Axum path style already used by guild/channel endpoints.
3. Preserve dual-backend SQL patterns (Postgres and SQLite branches) for all new model queries.
4. Keep frontend state centralized in `features/channel` store/API modules; avoid parallel ad-hoc category state elsewhere.

### Library & Framework Requirements

- Frontend: stay on Svelte 5 runes patterns (`$state`, `$derived`, `$effect`) and existing `@mateothegreat/svelte5-router` navigation hooks.
- Backend: stay on Axum 0.8 + sqlx 0.8 conventions already used in channel/guild handlers/services/models.
- Avoid introducing new drag-drop dependencies unless native Svelte/DOM implementation proves insufficient and test coverage justifies the addition.

### File Structure Requirements

Expected primary touch points:

- `server/migrations/0012_create_channel_categories.sql` (new)
- `server/src/handlers/mod.rs`
- `server/src/handlers/categories.rs` (new)
- `server/src/services/mod.rs`
- `server/src/services/channel_service.rs`
- `server/src/models/mod.rs`
- `server/src/models/channel.rs`
- `server/src/models/category.rs` (new)
- `server/tests/server_binds_to_configured_port.rs`
- `client/src/lib/features/channel/ChannelList.svelte`
- `client/src/lib/features/channel/channelApi.ts`
- `client/src/lib/features/channel/channelStore.svelte.ts`
- `client/src/lib/features/channel/types.ts`
- `client/src/lib/features/channel/ChannelList.test.ts`
- `client/src/lib/features/channel/channelStore.test.ts`

### Testing Requirements

- Server:
  - Verify category mutation routes reject unauthenticated requests and non-owner writes (consistent with existing channel test style).
  - Verify category create/list/update/delete flows and move-to-uncategorized on delete.
  - Verify collapse state persistence is scoped to user + guild + category.
- Client:
  - Verify grouped category rendering, collapse toggling, and persisted collapsed restoration.
  - Verify drag/drop channel movement across categories and fallback move actions.
  - Verify keyboard behavior for category toggle/traversal and accessible semantics (`aria-expanded`, focus order).

### Previous Story Intelligence

- Story 4.3 established `channelState` as the source of truth; it now includes stale-response request-token guarding and must remain race-safe when category loads are introduced.
- Story 4.3 integration tests (`server_binds_to_configured_port.rs` channel block) already verify auth, owner-only mutation, reorder correctness, and default-channel fallback on delete; category tests should mirror this pattern.
- Story 4.3 `ChannelList.svelte` already includes create/rename/delete/reorder controls and non-pointer reorder actions; category UX should extend these interactions rather than replacing with a separate parallel surface.

### Git Intelligence Summary

- Recent commit history shows one product commit for Epic 4 foundation (`22cf056` for Story 4.2) followed by status/tooling commits; this suggests implementation continuity should stay anchored in existing guild/channel modules rather than broad restructuring.
- Sprint status tracking is updated in dedicated commits (`d123729`), confirming workflow expectation that story-state transitions are persisted in `_bmad-output/implementation-artifacts/sprint-status.yaml`.

### Latest Technical Information

1. Current repo pins:
   - `svelte`: `^5.45.2`
   - `@mateothegreat/svelte5-router`: `^2.16.19`
   - `axum`: `0.8`
   - `sqlx`: `0.8`
2. Current upstream stable lines remain compatible with repo choices (Svelte 5.x, svelte5-router 2.16.x, Axum 0.8.x, sqlx 0.8.x); no dependency upgrade is required for this story.
3. Keep implementation compatible with existing runes-native frontend and Tokio-native backend stack; treat dependency upgrades as separate scoped work.

### Project Context Reference

- No `project-context.md` was discovered via `**/project-context.md`.
- Story context was derived from epics, PRD, architecture, UX specification, prior Epic 4 story artifacts, and current runtime source.

### Story Completion Status

- Ultimate context analysis completed — comprehensive developer implementation guide created.
- Story status set to `ready-for-dev`.

### Project Structure Notes

- This story extends Story 4.3 channel management by introducing grouping and per-user collapse behavior; it should not pull invite lifecycle scope from Stories 4.5/4.6.
- Keep permission architecture forward-compatible with Epic 5 role/override work while preserving existing owner-based enforcement semantics in the current codebase.

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Epic 4: Guilds, Channels & Invites]
- [Source: _bmad-output/planning-artifacts/epics.md#Story 4.4: Channel Categories]
- [Source: _bmad-output/planning-artifacts/prd.md#Functional Requirements]
- [Source: _bmad-output/planning-artifacts/prd.md#Accessibility]
- [Source: _bmad-output/planning-artifacts/architecture.md#Frontend Architecture]
- [Source: _bmad-output/planning-artifacts/architecture.md#Naming Patterns]
- [Source: _bmad-output/planning-artifacts/architecture.md#Structure Patterns]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#ChannelCategory]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Journey 2: Maya — Guild Setup]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Accessibility Considerations]
- [Source: _bmad-output/implementation-artifacts/4-3-text-and-voice-channel-management.md]
- [Source: client/src/lib/features/channel/ChannelList.svelte]
- [Source: client/src/lib/features/channel/channelStore.svelte.ts]
- [Source: client/src/lib/features/channel/channelApi.ts]
- [Source: client/src/lib/features/channel/types.ts]
- [Source: server/src/handlers/mod.rs]
- [Source: server/src/handlers/channels.rs]
- [Source: server/src/services/channel_service.rs]
- [Source: server/src/models/channel.rs]
- [Source: server/migrations/0011_create_channels.sql]
- [Source: server/tests/server_binds_to_configured_port.rs]
- [Source: client/package.json]
- [Source: server/Cargo.toml]
- [Source: https://github.com/sveltejs/svelte/releases]
- [Source: https://www.npmjs.com/package/@mateothegreat/svelte5-router]
- [Source: https://github.com/tokio-rs/axum/releases]
- [Source: https://crates.io/crates/sqlx]

## Dev Agent Record

### Agent Model Used

GPT-5.3-Codex (model ID: gpt-5.3-codex)

### Debug Log References

- Workflow engine loaded: `_bmad/core/tasks/workflow.xml`
- Workflow config loaded: `_bmad/bmm/workflows/4-implementation/dev-story/workflow.yaml`
- Implemented server category persistence + services + handlers with per-user collapse storage and channel reassignment on category delete.
- Implemented client category-aware ChannelList rendering, category CRUD/collapse interactions, channel movement between categories, and keyboard navigation/toggling.
- Quality gates executed successfully:
  - `cd client && npm run lint && npm run check && npm run test`
  - `cd client && npm run build`
  - `cd server && cargo fmt --check && cargo clippy -- -D warnings && cargo test`

### Completion Notes List

- Implemented category schema and category/channel persistence updates (including nullable `channels.category_id` and `channel_category_collapses` keyed by `user_id + guild_id + category_id`).
- Added authenticated category APIs under `/api/v1/guilds/{guild_slug}/categories` (list/create/update/delete/reorder/collapse) and extended channel reorder/create payload support for category assignment.
- Refactored channel client feature modules to include category domain mapping, category CRUD endpoints, category collapse persistence calls, and category-aware channel movement state updates.
- Refactored `ChannelList.svelte` to render grouped categories + uncategorized channels, support category create/rename/delete, category collapse toggling, drag/drop movement across categories, and keyboard Enter/arrow navigation behavior.
- Added/updated server and client tests for category auth/forbidden flows, category CRUD/collapse/delete-to-uncategorized behavior, grouped rendering, and category collapse state updates.

### File List

- _bmad-output/implementation-artifacts/4-4-channel-categories.md
- _bmad-output/implementation-artifacts/sprint-status.yaml
- server/migrations/0012_create_channel_categories.sql
- server/src/models/mod.rs
- server/src/models/category.rs
- server/src/models/channel.rs
- server/src/services/mod.rs
- server/src/services/category_service.rs
- server/src/services/channel_service.rs
- server/src/services/guild_service.rs
- server/src/handlers/mod.rs
- server/src/handlers/categories.rs
- server/src/handlers/channels.rs
- server/tests/server_binds_to_configured_port.rs
- client/src/lib/features/channel/types.ts
- client/src/lib/features/channel/channelApi.ts
- client/src/lib/features/channel/channelStore.svelte.ts
- client/src/lib/features/channel/ChannelList.svelte
- client/src/App.svelte
- client/src/lib/features/channel/channelStore.test.ts
- client/src/lib/features/channel/ChannelList.test.ts

### Senior Developer Review (AI)

- Reviewer: Darko (AI) — 2026-02-28
- Outcome: Approved after one medium issue was fixed.
- Findings:
  - **MEDIUM (fixed)** `client/src/lib/features/channel/ChannelList.svelte`: keyboard ArrowLeft/ArrowRight category collapse path called async persistence without error handling, causing unhandled promise rejections and missing user-visible feedback on failure.
- Fixes applied:
  - Added shared `persistCategoryCollapsed(...)` helper with consistent error handling and reused it for mouse toggle and keyboard ArrowLeft/ArrowRight actions.
- Acceptance Criteria verification:
  - AC1 ✅ Category creation implemented with validation and uppercase/small-text rendering.
  - AC2 ✅ Channels can move across categories and reorder predictably (drag/drop and fallback controls).
  - AC3 ✅ Chevron/name toggles expanded/collapsed state.
  - AC4 ✅ Collapse state persists per user and is restored.
  - AC5 ✅ Owner category header "+" create-channel control is present.
  - AC6 ✅ Rename/delete persists and delete moves channels to uncategorized.
  - AC7 ✅ Enter/arrow keyboard traversal behavior implemented.

## Change Log

- 2026-02-28: Story created and marked ready-for-dev with comprehensive context for channel categories.
- 2026-02-28: Implemented category persistence, category APIs, category-aware channel UI/store/API updates, and server/client tests; story moved to review.
- 2026-02-28: Senior developer adversarial review completed, fixed keyboard collapse persistence error handling path, and marked story done.
