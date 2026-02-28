# Story 4.7: Guild Navigation and Activity Indicators

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a **user**,
I want to see which guilds have unread activity and switch between them instantly,
so that I can stay aware of conversations across all my communities.

## Acceptance Criteria

1. **Given** a user has joined multiple guilds  
   **When** they view the GuildRail  
   **Then** each guild shows its icon (48px circle) with the guild name in a tooltip on hover

2. **Given** a guild is active  
   **When** the GuildRail renders  
   **Then** the active guild has an ice blue indicator bar on the left edge

3. **Given** guilds have unread activity  
   **When** the GuildRail renders  
   **Then** guilds with unread messages show a fire dot badge

4. **Given** a user clicks or keyboard-selects a guild icon  
   **When** guild switching occurs  
   **Then** the app switches to that guild's channel list and last-viewed channel

5. **Given** a user switches guilds frequently  
   **When** channel data is already available  
   **Then** guild switching feels instant (no global loading spinner; channel list swaps from cached data)

6. **Given** a user prefers a custom guild order  
   **When** they drag-and-drop guild icons in the GuildRail  
   **Then** the new order is applied and persisted

7. **Given** the GuildRail is visible  
   **When** the user looks at the top control  
   **Then** a Home button provides access to DMs (placeholder for Epic 6)

8. **Given** a keyboard-only user navigates the GuildRail  
   **When** arrow keys and Enter are used  
   **Then** arrow keys move focus across guild icons and Enter activates the selected guild

9. **Given** assistive technology users navigate guild controls  
   **When** guild icons render  
   **Then** each icon is a button with `aria-label="Guild Name"`

## Tasks / Subtasks

- [x] Task 1: Add guild activity signal and navigation metadata contracts (AC: 3, 4, 5)
  - [x] Extend `client/src/lib/features/guild/types.ts` and `guildApi.ts` with optional activity/navigation fields (snake_case wire ↔ camelCase domain), keeping backward compatibility when fields are absent.
  - [x] If backend contract expansion is needed, extend `server/src/models/guild.rs` and `server/src/services/guild_service.rs` with optional activity fields using safe defaults (no silent inference). *(Not required for this implementation; backend unchanged.)*
  - [x] Preserve `{ "data": ... }` envelope and existing authentication/authorization behavior on `/api/v1/guilds`.

- [x] Task 2: Persist per-guild navigation context (AC: 4, 5)
  - [x] Extend `client/src/lib/features/identity/navigationState.ts` from single-path persistence to per-guild last-viewed channel and persisted guild order (localStorage-backed, explicit getters/setters).
  - [x] Update route-resolution hooks in `client/src/App.svelte` and/or `client/src/lib/features/shell/ShellRoute.svelte` so every `/:guild/:channel` visit updates per-guild history.
  - [x] On guild switch, resolve target channel as: persisted last-viewed channel for guild → guild `defaultChannelSlug` fallback.

- [x] Task 3: Upgrade GuildRail visual states and semantics (AC: 1, 2, 3, 7, 9)
  - [x] Refactor `client/src/lib/features/guild/GuildRail.svelte` to 48px circular icons, hover/focus tooltip, left-edge active indicator bar, unread fire-dot badge, and top Home button treatment aligned with UX spec.
  - [x] Keep "Create guild" affordance at rail bottom and existing create flow behavior unchanged.
  - [x] Ensure all icon controls are button semantics with correct `aria-label`, `aria-current`, and predictable focus order.

- [x] Task 4: Implement instant-feel guild switching using existing cache primitives (AC: 4, 5)
  - [x] Reuse `channelState.loadedByGuild` cache behavior and request-token guards in `client/src/lib/features/channel/channelStore.svelte.ts` to avoid flicker/stale swaps on rapid guild changes.
  - [x] Avoid adding a global app/shell loading spinner for guild switches; keep loading indicators scoped to affected panels only.
  - [x] Ensure switching from GuildRail updates channel list/message context without breaking existing route persistence.

- [x] Task 5: Implement guild drag-and-drop reorder with persistence (AC: 6)
  - [x] Add drag start/over/drop/end handling in `GuildRail.svelte`, reusing ChannelList DnD interaction patterns where practical.
  - [x] Persist resulting guild slug order in navigation state and rehydrate order on `guildState.loadGuilds()`.
  - [x] Preserve deterministic fallback ordering for guilds not yet present in persisted order.

- [x] Task 6: Implement keyboard navigation and activation behavior (AC: 8, 9)
  - [x] Add roving focus/arrow-key behavior for guild icons and Enter activation logic (Space optional parity if aligned with button semantics).
  - [x] Ensure active guild announcement and tooltip behavior remain accessible for keyboard and screen-reader users.
  - [x] Keep skip-link and main-content focus management in ShellRoute intact.

- [x] Task 7: Add/extend automated tests and run quality gates (AC: all)
  - [x] Extend `client/src/lib/features/guild/GuildRail.test.ts` for icon size/state classes, tooltip exposure, unread badge rendering, keyboard navigation, and reorder persistence behavior.
  - [x] Extend related tests (`ShellRoute.test.ts`, `guildApi.test.ts`, and/or navigation state unit tests) for last-viewed channel resolution and Home button behavior.
  - [x] If backend contract changes are introduced, add server integration coverage in `server/tests/server_binds_to_configured_port.rs` for guild payload compatibility. *(Not required; backend contract unchanged.)*
  - [x] Run quality gates:
    - [x] `cd client && npm run lint && npm run check && npm run test && npm run build`
    - [x] `cd server && cargo fmt --check && cargo clippy -- -D warnings && cargo test` (only if server files changed)

## Dev Notes

### Developer Context

- Story 4.6 delivered invite-join onboarding and membership-aware guild/channel/category reads; `guildState.loadGuilds(true)` refresh after invite join is already in place.
- Current `GuildRail.svelte` renders guild links and create-guild flow, but does **not** yet implement 48px circular icon styling, hover tooltip, unread badge, left-edge active bar, keyboard arrow navigation, or drag-drop ordering.
- Current route persistence stores a single `discool-last-location` value; story AC requires per-guild last-viewed channel behavior for switching.
- `channelState` already has per-guild cache tracking (`loadedByGuild`) and stale-request protection (`latestLoadRequestToken`) that should be reused for instant switching behavior.

### Technical Requirements

- Keep guild switching route shape as `/:guild/:channel`; do not introduce parallel route formats.
- Preserve existing invite/onboarding and shell bootstrap flows in `App.svelte`; guild navigation enhancements must not regress Story 4.6 behavior.
- Unread/activity display must be data-driven with explicit defaults when signal data is unavailable (no fabricated counts).
- Reorder persistence must be deterministic and resilient when guild membership changes (new guild joins or removed guilds).
- Maintain accessibility baselines: keyboard navigation, `aria-label`, focus visibility, and skip-link behavior.

### Architecture Compliance

1. Keep client layering consistent:
   - feature API/types in `features/guild`
   - reactive state in feature stores / navigation helpers
   - view logic in Svelte components.
2. If backend response fields are added, keep server layering:
   - handlers = HTTP boundary + envelope
   - services = business logic and authorization
   - models = SQL access / response projection.
3. Keep API contracts explicit and typed (`snake_case` wire, `camelCase` client mapping).
4. Preserve existing owner/member authorization boundaries for guild/channel/category mutations.

### Library & Framework Requirements

- Frontend: Svelte 5 runes + `@mateothegreat/svelte5-router` only; avoid introducing new state or DnD libraries unless strictly necessary.
- Backend (if touched): Axum 0.8 + sqlx 0.8 patterns used across current handlers/services/models.
- Styling: reuse existing Tailwind token system (`--primary`, `--fire`, `--channel-unread`, etc.) and existing sidebar layout primitives.

### File Structure Requirements

Expected primary touch points:

- `client/src/lib/features/guild/GuildRail.svelte`
- `client/src/lib/features/guild/GuildRail.test.ts`
- `client/src/lib/features/guild/guildStore.svelte.ts`
- `client/src/lib/features/guild/guildApi.ts`
- `client/src/lib/features/guild/types.ts`
- `client/src/lib/features/identity/navigationState.ts`
- `client/src/lib/features/shell/ShellRoute.svelte`
- `client/src/lib/features/shell/ShellRoute.test.ts`
- `client/src/App.svelte`
- `client/src/routes/routes.ts`
- `client/src/lib/features/channel/channelStore.svelte.ts`
- `client/src/app.css`
- `server/src/models/guild.rs` *(if payload extension required)*
- `server/src/services/guild_service.rs` *(if payload extension required)*
- `server/src/handlers/guilds.rs` *(if payload extension required)*
- `server/tests/server_binds_to_configured_port.rs` *(if payload extension required)*

### Testing Requirements

- Client:
  - Verify GuildRail shows 48px circle icons, active indicator bar, unread fire-dot badge, and Home button.
  - Verify tooltip content and accessible labels for each guild icon.
  - Verify arrow-key navigation + Enter activation across guild icons.
  - Verify drag-drop reorder changes render order and persists across remount/reload.
  - Verify guild switch target channel selection prefers last-viewed channel for that guild.
  - Verify switching remains responsive without global spinner regressions.
- Server (only if contract changes):
  - Verify `/api/v1/guilds` response remains backward compatible and includes new optional fields as expected.
  - Verify existing auth and membership visibility behavior remains unchanged.

### Previous Story Intelligence

- Story 4.6 established invite resolution/join endpoints and made guild/channel/category reads member-aware while keeping mutations owner-gated.
- `App.svelte` currently orchestrates invite flow, guild refresh, and route persistence; Story 4.7 should extend, not duplicate, that orchestration.
- Current GuildRail test coverage is focused on create-guild flow; navigation-state and accessibility behaviors need targeted expansion for this story.

### Git Intelligence Summary

- `608adec feat: implement story 4-6 invite guild join` provides the immediate baseline for membership-aware guild visibility and invite-driven guild entry.
- `e735b72 feat: implement story 4-5 invite management` established GuildRail/Shell invite entry patterns and guild-level action placement conventions.
- `599b44e feat: finalize channel and category management` provides reusable drag-drop patterns and channel cache handling to mirror for guild reordering/switching.

### Latest Technical Information

1. Current repo pins:
   - `svelte`: `^5.45.2`
   - `@mateothegreat/svelte5-router`: `^2.16.19`
   - `axum`: `0.8`
   - `sqlx`: `0.8`
2. Latest stable lines researched:
   - Svelte: `5.53.6`
   - `@mateothegreat/svelte5-router`: `2.16.19`
   - Axum: `0.8.8`
   - SQLx: `0.8.6`
3. No dependency upgrade is required for Story 4.7; implement against current pinned versions.

### Project Context Reference

- No `project-context.md` was discovered via `**/project-context.md`.
- Story context is derived from planning artifacts, previous Epic 4 implementation artifacts, current runtime code, and recent commit history.

### Story Completion Status

- Ultimate context analysis completed — comprehensive developer implementation guide created.
- Story status set to `ready-for-dev`.

### Project Structure Notes

- The FR traceability matrix maps FR56/FR59 under Epic 6; to avoid scope creep, Story 4.7 should deliver GuildRail UX/navigation plumbing and activity-indicator display wiring while leaving full unread signal generation to Epic 6 message/event infrastructure.
- Keep Home button scope as DM entry placeholder only (Epic 6 will provide full DM surfaces and unread semantics).

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Epic 4: Guilds, Channels & Invites]
- [Source: _bmad-output/planning-artifacts/epics.md#Story 4.7: Guild Navigation and Activity Indicators]
- [Source: _bmad-output/planning-artifacts/prd.md#User Experience & Navigation]
- [Source: _bmad-output/planning-artifacts/prd.md#Non-Functional Requirements]
- [Source: _bmad-output/planning-artifacts/architecture.md#Tech Stack]
- [Source: _bmad-output/planning-artifacts/architecture.md#State Boundaries (Frontend)]
- [Source: _bmad-output/planning-artifacts/architecture.md#Process Patterns]
- [Source: _bmad-output/planning-artifacts/architecture.md#Requirements to Structure Mapping]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Critical Success Moments]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Transferable UX Patterns]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#GuildRail]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Design tokens to establish early]
- [Source: _bmad-output/implementation-artifacts/4-6-join-guild-via-invite-link.md]
- [Source: client/src/lib/features/guild/GuildRail.svelte]
- [Source: client/src/lib/features/guild/GuildRail.test.ts]
- [Source: client/src/lib/features/guild/guildStore.svelte.ts]
- [Source: client/src/lib/features/guild/guildApi.ts]
- [Source: client/src/lib/features/guild/types.ts]
- [Source: client/src/lib/features/channel/channelStore.svelte.ts]
- [Source: client/src/lib/features/channel/ChannelList.svelte]
- [Source: client/src/lib/features/shell/ShellRoute.svelte]
- [Source: client/src/lib/features/shell/ShellRoute.test.ts]
- [Source: client/src/lib/features/identity/navigationState.ts]
- [Source: client/src/routes/routes.ts]
- [Source: client/src/App.svelte]
- [Source: client/src/app.css]
- [Source: server/src/models/guild.rs]
- [Source: server/src/services/guild_service.rs]
- [Source: server/src/handlers/guilds.rs]
- [Source: server/src/handlers/mod.rs]
- [Source: server/migrations/0010_create_guilds.sql]
- [Source: server/migrations/0011_create_channels.sql]
- [Source: server/migrations/0014_create_guild_members.sql]
- [Source: client/package.json]
- [Source: server/Cargo.toml]
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
- Story source parsed from user input: `4-7` → `4-7-guild-navigation-and-activity-indicators`
- Sprint status transitioned: `ready-for-dev` → `in-progress` → `review` → `done`
- Quality gates executed: `cd client && npm run lint && npm run check && npm run test && npm run build`

### Completion Notes List

- Added optional guild activity/navigation contract fields (`hasUnreadActivity`, `lastViewedChannelSlug`) with backward-compatible wire mapping.
- Extended navigation persistence with per-guild last-viewed channels and persisted guild order, and wired route resolution updates in `App.svelte`.
- Refactored `GuildRail.svelte` to implement 48px circular buttons, tooltip, active indicator bar, unread fire-dot badge, Home button, drag-drop reorder persistence, and keyboard arrow/Enter behavior.
- Upgraded channel store with per-guild cached channel/category snapshots for instant-feel switching and stale request invalidation.
- Expanded tests across GuildRail, navigation state, guild API mapping, shell Home control presence, and channel-store cache reuse.

### File List

- _bmad-output/implementation-artifacts/4-7-guild-navigation-and-activity-indicators.md
- _bmad-output/implementation-artifacts/sprint-status.yaml
- client/src/App.svelte
- client/src/lib/features/channel/channelStore.svelte.ts
- client/src/lib/features/channel/channelStore.test.ts
- client/src/lib/features/guild/GuildRail.svelte
- client/src/lib/features/guild/GuildRail.test.ts
- client/src/lib/features/guild/guildApi.test.ts
- client/src/lib/features/guild/guildStore.svelte.ts
- client/src/lib/features/guild/types.ts
- client/src/lib/features/identity/navigationState.test.ts
- client/src/lib/features/identity/navigationState.ts
- client/src/lib/features/shell/ShellRoute.test.ts

### Change Log

- 2026-02-28: Implemented Story 4.7 GuildRail navigation/activity enhancements, per-guild navigation persistence, guild-order persistence, cached guild switching, and related automated tests.
- 2026-02-28: Completed adversarial code review; fixed GuildRail ARIA list/tooltip semantics and channel cache-switch stale-loading behavior; added regression coverage in `channelStore.test.ts`.

## Senior Developer Review (AI)

### Reviewer

Darko (AI)

### Outcome

Approved after fixes

### Findings and Resolutions

1. **HIGH — Invalid list semantics in GuildRail (`client/src/lib/features/guild/GuildRail.svelte`)**
   - Issue: `role="listitem"` entries were not contained by a list role.
   - Fix: Added `role="list"` to the guild navigation container.

2. **MEDIUM — Tooltip ARIA relationship missing (`client/src/lib/features/guild/GuildRail.svelte`)**
   - Issue: `role="tooltip"` content was not explicitly associated with its trigger.
   - Fix: Added stable tooltip IDs and `aria-describedby` linkage on guild buttons.

3. **MEDIUM — In-flight channel load could overwrite cached guild switch (`client/src/lib/features/channel/channelStore.svelte.ts`)**
   - Issue: Switching to cached guild data during another guild's in-flight request could leave stale loading state and later replace active guild channels unexpectedly.
   - Fix: Invalidate in-flight request tokens whenever serving cached guild data while loading, and clear loading/error state when cache activates.
   - Regression Test: Added `clears loading when switching from an in-flight guild to cached guild data` in `client/src/lib/features/channel/channelStore.test.ts`.

### Verification

- `cd client && npm run lint && npm run check && npm run test` (pass)
