# Story 6.12: Quick Switcher

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a user,  
I want a keyboard shortcut to quickly search and navigate to any guild, channel, or DM,  
so that I can move around the app efficiently.

## Acceptance Criteria

1. Pressing `Ctrl+K` (or `Cmd+K` on macOS) opens the quick switcher overlay.
2. On open, the search input is focused and recently active items are listed before typing.
3. Typing performs fuzzy search across guild names, channel names, and DM usernames/display names.
4. Results are grouped by type: **Channels**, **DMs**, **Guilds**.
5. Keyboard controls work end-to-end: arrow keys move selection, `Enter` navigates, `Escape` closes.
6. The switcher is a centered modal overlay with focus trapped while open.
7. Selecting a result navigates instantly to the target guild/channel/DM route.

## Tasks / Subtasks

- [x] Extend the existing ShellRoute quick-switcher baseline instead of re-implementing from scratch (AC: 1, 2, 7)
  - [x] Keep the existing global `Ctrl/Cmd+K` hotkey integration in `ShellRoute.svelte` and preserve current lifecycle interactions.
  - [x] Preserve current route-close behavior and explicit close action while adding missing AC behavior.
  - [x] Keep the existing `goto(...)` routing integration for final navigation.

- [x] Implement proper quick-switcher data model for grouped results and recent-first defaults (AC: 2, 4)
  - [x] Build a unified result model that supports separate groups for Channels, DMs, and Guilds.
  - [x] Source recent-first defaults from existing store data (`dmState.conversations`, `guildState.guilds`, `channelState.orderedChannelsForGuild(...)`) before query input.
  - [x] For Guild results, always resolve to a valid channel route: `/${guildSlug}/${lastViewedChannelSlug || firstOrderedChannelSlug || defaultChannelSlug}` (never guild-only navigation).
  - [x] Add an explicit channel-hydration strategy for non-active guilds when opening the switcher (reuse cache first, then lazy preload via `channelState.loadChannels(guildSlug)` as needed) so cross-guild channel search is complete.
  - [x] Ensure DM coverage remains intact (already introduced in story 6.9 baseline behavior).

- [x] Replace substring filtering with deterministic fuzzy matching (AC: 3)
  - [x] Implement in-feature fuzzy ranking/scoring utility (no required new dependency unless clearly justified).
  - [x] Match against guild name/slug, channel name (+ guild context), and DM display name/username.
  - [x] Define and implement strict ranking/tie-break order for deterministic behavior: exact/prefix > fuzzy score > recency > stable key.
  - [x] Keep filtering token-efficient and fast for expected dataset size.

- [x] Add full keyboard navigation and selection semantics inside the switcher (AC: 5)
  - [x] Maintain active index state with arrow-key navigation across grouped result rows.
  - [x] Support `Enter` to activate highlighted item and `Escape` to close from anywhere in the dialog.
  - [x] Ensure `Cmd+K`/`Ctrl+K` does not interfere with existing unread-navigation shortcuts.

- [x] Implement accessibility-compliant modal behavior (AC: 2, 5, 6)
  - [x] Autofocus the search input on open and restore focus to prior element on close.
  - [x] Trap focus within the dialog while open and keep `aria-modal` / label semantics aligned.
  - [x] Keep list/result semantics screen-reader friendly and keyboard-only usable.

- [x] Expand test coverage and run quality gates (AC: 1-7)
  - [x] Extend `ShellRoute.test.ts` for open/focus behavior, grouped rendering, fuzzy query behavior, keyboard nav, Enter selection, and Escape close.
  - [x] Keep/extend existing DM quick-switcher assertion coverage.
  - [x] Run client quality checks: `cd client && npm run lint && npm run check && npm run test`.
  - [x] Run server quality checks for regression confidence: `cd server && cargo fmt --check && cargo clippy -- -D warnings && cargo test`.

## Dev Notes

### Developer Context

- Story 6.12 is the final Epic 6 navigation/power-user story and should build directly on existing Epic 6 chat/navigation infrastructure.
- The quick switcher already exists in `client/src/lib/features/shell/ShellRoute.svelte`:
  - global `Ctrl/Cmd+K` hotkey toggles overlay
  - current search is substring-based (`includes`) over DM + channel-derived entries
  - current click-to-navigate behavior uses `goto(...)`
- Existing test coverage includes DM inclusion and click navigation, but not full AC behavior.
- Do not rewrite shell navigation architecture; extend current behavior surgically.

### Technical Requirements & Guardrails

- Reuse existing stores and routing primitives:
  - `dmState.conversations` (already sorted with activity semantics)
  - `guildState.guilds`
  - `channelState.orderedChannelsForGuild(...)`
  - `@mateothegreat/svelte5-router` `goto(...)`
- Preserve current global hotkeys and avoid conflicts:
  - keep `Alt+Shift+Up/Down` unread-channel navigation behavior intact
  - keep `Ctrl/Cmd+K` cross-platform and avoid browser-reserved shortcut collisions
- Guild result routing must always target a concrete channel route (`/:guild/:channel`), never `/:guild`.
- Ensure non-active guild channels are hydrated before or during quick-switcher indexing so channel search is not incomplete.
- No direct `fetch()` calls from UI components; consume existing store/API layers only.
- Avoid scope creep into unrelated message transport, error/toast infrastructure, or server APIs.
- Keep implementation deterministic and testable (stable ranking/grouping behavior for same inputs) with explicit tie-break ordering.

### Architecture Compliance Notes

- Follow frontend architecture:
  - Svelte 5 runes for UI state
  - `svelte5-router` for navigation
  - WebSocket-store/TanStack Query split for data sources
- Keep source-of-truth boundaries clear:
  - modal/query/highlight state in shell UI state
  - entity lists from existing feature stores
- Preserve feature-based organization and naming conventions (`PascalCase` components, `camelCase` TS symbols).
- Keep tests co-located with touched feature files.

### Library & Framework Requirements

- Current project baseline:
  - `svelte` `^5.45.2`
  - `@mateothegreat/svelte5-router` `^2.16.19`
  - `vite` `^7.3.1`
  - `vitest` `^4.0.18`
  - server stack includes `axum 0.8` and `sqlx 0.8`
- Latest-version awareness (researched):
  - Svelte latest: `5.53.6`
  - `@mateothegreat/svelte5-router` latest: `2.16.19`
  - Axum latest: `0.8.8`
  - SQLx latest tag: `0.8.6`
- No dependency upgrade is required for this story; prioritize AC correctness on current project versions.

### File Structure Requirements

- Primary implementation files (expected):
  - `client/src/lib/features/shell/ShellRoute.svelte`
  - `client/src/lib/features/shell/ShellRoute.test.ts`
- Supporting read/usage surfaces (reuse, avoid duplication):
  - `client/src/lib/features/dm/dmStore.svelte.ts`
  - `client/src/lib/features/guild/guildStore.svelte.ts`
  - `client/src/lib/features/channel/channelStore.svelte.ts`
  - `client/src/routes/routes.ts`
  - `client/src/lib/features/identity/navigationState.ts` (if needed for explicit recent tracking)
- If extraction is needed, keep helper(s) inside shell feature scope (or existing shared utility patterns) and add matching tests.

### Testing Requirements Summary

- Add/extend tests for:
  - `Ctrl+K` and `Cmd+K` open behavior
  - input autofocus on open
  - recent-first default rendering before typing
  - grouped result rendering (Channels / DMs / Guilds)
  - guild result navigation path resolution to `/:guild/:channel` fallback chain
  - non-active guild hydration path so channel results are available outside current guild
  - fuzzy matching correctness and stability
  - deterministic tie-break ordering for same-score matches
  - arrow-key selection navigation
  - `Enter` activation of highlighted item
  - `Escape` close behavior
  - focus trap + focus restoration
- Preserve and expand existing DM quick-switcher test scenario.
- Run full client quality gates; run server checks for regression confidence.

### Previous Story Intelligence (6.11)

- Story 6.11 established strict UX quality patterns for status clarity and accessibility; maintain that quality bar for keyboard-first quick-switcher UX.
- Existing Epic 6 implementation already centralizes live chat/guild/dm state in stores; leverage those stores instead of introducing parallel state models.
- Recent implementation patterns favor additive, focused changes in existing feature files with co-located tests.
- Keep this story scoped to quick-switcher behavior; do not absorb unrelated error/toast work.

### Git Intelligence Summary

- Recent commits indicate direct continuity across Epic 6:
  - `e91d6ff` feat: include reaction actors for block filtering
  - `e6d031d` feat: finalize story 6-10 user blocking complete erasure
  - `0cc2c2c` feat: finalize story 6-9 direct messages
  - `aef45a9` feat: finalize story 6-8 typing indicators and channel activity
  - `46525a7` feat: finalize story 6-7 rich embeds and markdown rendering
- Story 6.9 commit already touched `ShellRoute` and introduced DM quick-switcher presence; this story should complete the full 6.12 AC set on top of that baseline.

### Latest Technical Information

- Router package is already on latest published version (`2.16.19`); no router upgrade pressure.
- Svelte has newer minor releases available (`5.53.6` latest), but this story should avoid framework upgrades unless a blocker is identified.
- Backend latest awareness (Axum `0.8.8`, SQLx `0.8.6`) is informational only for this client-focused story.

### Project Context Reference

- `project-context.md` was not found via configured pattern `**/project-context.md` during discovery.

### Story Completion Status

- Story file created: `_bmad-output/implementation-artifacts/6-12-quick-switcher.md`
- Sprint status target for this story: `ready-for-dev`
- Completion note: Ultimate context engine analysis completed - comprehensive developer guide created.

### Project Structure Notes

- Planned work aligns with existing feature boundaries under `client/src/lib/features/shell`.
- Architecture artifact references a shared `accessibility.ts` helper path, but current repo structure does not include that file; if focus-trap helpers are extracted, place them in an existing in-repo location consistent with current conventions.

### References

- Story definition and ACs:
  - `_bmad-output/planning-artifacts/epics.md` (Story 6.12, Epic 6 section)
  - `_bmad-output/planning-artifacts/epics.md` (Story 6.9 quick-switcher DM inclusion note)
- UX requirements:
  - `_bmad-output/planning-artifacts/ux-design-specification.md` (Keyboard Shortcut Patterns; Navigation Patterns; focus trap guidance)
  - `_bmad-output/planning-artifacts/ux-design-specification.md` (Quick switcher behavior: fuzzy/grouped/arrow/enter/escape/recent-first)
- Product requirements context:
  - `_bmad-output/planning-artifacts/prd.md` (FR59-FR62 navigation/resilience context)
- Architecture constraints:
  - `_bmad-output/planning-artifacts/architecture.md` (routing, state model, anti-patterns, test organization, feature structure)
- Existing implementation baseline:
  - `client/src/lib/features/shell/ShellRoute.svelte`
  - `client/src/lib/features/shell/ShellRoute.test.ts`
  - `client/src/lib/features/dm/dmStore.svelte.ts`
  - `client/src/lib/features/guild/guildStore.svelte.ts`
  - `client/src/lib/features/channel/channelStore.svelte.ts`
  - `client/src/routes/routes.ts`
- Latest version checks:
  - https://registry.npmjs.org/svelte/latest
  - https://registry.npmjs.org/@mateothegreat/svelte5-router/latest
  - https://api.github.com/repos/tokio-rs/axum/releases/latest
  - https://api.github.com/repos/launchbadge/sqlx/tags?per_page=1

## Dev Agent Record

### Agent Model Used

GPT-5.3-Codex (model ID: gpt-5.3-codex)

### Debug Log References

- Story creation workflow execution: create-story (epic 6, story 12).
- Story implementation: updated quick switcher model, keyboard semantics, and dialog accessibility behavior in `ShellRoute.svelte`.
- Verification runs:
  - `cd client && npm run lint && npm run check && npm run test`
  - `cd server && cargo fmt --check && cargo clippy -- -D warnings && cargo test`

### Completion Notes List

- Parsed requested story identifier `6-12` to `6-12-quick-switcher`.
- Loaded and analyzed epics, architecture, PRD, UX, sprint status, previous story (6.11), and recent git history.
- Generated implementation guidance emphasizing reuse of current ShellRoute quick-switcher baseline and closure of remaining AC gaps.
- Implemented grouped quick-switcher results (Channels/DMs/Guilds), recent-first defaults, deterministic fuzzy ranking, and guild route fallback resolution.
- Added non-active guild channel hydration on quick-switcher open using `channelState.loadChannels(guildSlug)` and active-guild restoration.
- Implemented arrow-key selection, Enter activation, Escape close, focus trap, autofocus on open, and focus restoration on close.
- Expanded `ShellRoute.test.ts` coverage for Ctrl/Cmd+K behavior, grouped rendering, fuzzy ranking determinism, keyboard navigation, focus handling, and hydration behavior.
- Senior AI code review completed; fixed keyboard Enter handling so non-input controls in the quick-switcher dialog do not trigger unintended navigation, and aligned overlay positioning with centered-modal AC language.

### File List

- `_bmad-output/implementation-artifacts/6-12-quick-switcher.md`
- `_bmad-output/implementation-artifacts/6-11-error-handling-and-status-communication.md`
- `_bmad-output/implementation-artifacts/sprint-status.yaml`
- `client/src/App.svelte`
- `client/src/lib/components/ToastViewport.svelte`
- `client/src/lib/components/ToastViewport.test.ts`
- `client/src/lib/features/chat/MessageArea.svelte`
- `client/src/lib/features/chat/MessageArea.test.ts`
- `client/src/lib/feedback/toastStore.svelte.ts`
- `client/src/lib/feedback/userFacingError.ts`
- `client/src/lib/feedback/userFacingError.test.ts`
- `client/src/lib/features/shell/ShellRoute.svelte`
- `client/src/lib/features/shell/ShellRoute.test.ts`
- `server/src/error.rs`

## Senior Developer Review (AI)

### Outcome

Approved after fixes.

### Findings and Resolutions

- **[HIGH][Fixed]** Quick-switcher keyboard handler intercepted `Enter` globally at dialog scope, so pressing `Enter` on non-input controls (for example the Close button) could navigate to the active result unexpectedly.  
  **Fix:** Scoped `ArrowUp`/`ArrowDown`/`Enter` quick-switcher navigation handling to the search input target.
- **[HIGH][Fixed]** Quick-switcher overlay was top-aligned (`items-start`) rather than centered, conflicting with AC6 wording ("centered modal overlay").  
  **Fix:** Updated overlay layout classes to center the dialog vertically and horizontally.

### Verification

- `cd client && npm run test -- src/lib/features/shell/ShellRoute.test.ts`
- `cd client && npm run lint && npm run check && npm run test`

### Follow-up Review (2026-03-01)

- Re-reviewed Story 6.12 in YOLO mode with no additional HIGH/MEDIUM/LOW findings in the quick-switcher implementation.
- Re-validated quality gates:
  - `cd client && npm run lint && npm run check && npm run test`
  - `cd server && cargo fmt --check && cargo clippy -- -D warnings && cargo test`

## Change Log

- 2026-03-01: Completed Story 6.12 quick-switcher implementation; added grouped and fuzzy-search behavior, keyboard/accessibility semantics, expanded tests, and passed client/server quality gates.
- 2026-03-01: Senior AI code review completed; fixed quick-switcher Enter-key scope and centered overlay positioning, then re-ran client quality gates.
- 2026-03-01: Follow-up AI review found no additional issues; synced sprint status for `6-12-quick-switcher` to `done`.
