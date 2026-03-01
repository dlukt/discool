# Story 6.8: Typing Indicators and Channel Activity

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a **user**,
I want to see who's typing and which channels have activity,
so that conversations feel alive and I know where to look.

## Acceptance Criteria

1. **Given** a user is typing in a channel  
   **When** they start typing  
   **Then** a `c_typing_start` event is sent via WebSocket.

2. **Given** a user is typing in a channel  
   **When** `c_typing_start` is received by other subscribed users  
   **Then** they see "`[Username]` is typing..." above the message input.

3. **Given** multiple users are typing in the same channel  
   **When** typing indicators are rendered  
   **Then** text formats as "`[User1]` and `[User2]` are typing..." for two users, up to three names max, then "Several people are typing...".

4. **Given** typing activity stops  
   **When** no new typing signal is received for 5 seconds  
   **Then** the typing indicator disappears automatically.

5. **Given** a channel has unread messages  
   **When** the channel list is rendered  
   **Then** the channel shows a bold channel name and an ice blue dot.

6. **Given** unread activity exists in one or more channels  
   **When** guild and channel navigation UI renders  
   **Then** activity indicators are visible at channel and guild surfaces without exposing hidden channel content.

7. **Given** the user opens/views an unread channel  
   **When** that channel becomes active and read in the UI flow  
   **Then** its unread indicator clears.

8. **Given** the user presses `Alt+Shift+Up/Down`  
   **When** unread channels exist  
   **Then** navigation jumps to previous/next unread channel.

## Tasks / Subtasks

- [x] Task 1: Emit typing start signals from composer input (AC: 1)
  - [x] Add client typing emission in `MessageArea.svelte` so `wsClient.send('c_typing_start', { guild_slug, channel_slug })` fires when user starts typing in channel mode.
  - [x] Throttle typing emission (for example once every ~2 seconds while input remains non-empty) to avoid websocket spam and rate-limit collisions.
  - [x] Never emit for empty/whitespace-only drafts, non-channel modes, or missing guild/channel context.

- [x] Task 2: Track and render typing indicators with 5-second expiry (AC: 2, 3, 4)
  - [x] Extend realtime client state (existing `messageStore` or a dedicated typing store) to parse `typing_start` envelopes from `wsClient.subscribe`.
  - [x] Track typing users per `guild:channel` keyed by `user_id` with `lastSeenAt` timestamps; ignore self typing events.
  - [x] Resolve display names from already-loaded member/session data; fallback deterministically when profile metadata is unavailable.
  - [x] Render indicator text above composer in `MessageArea.svelte` with required grammar rules (1 user, 2 users, 3 users, >3 users).
  - [x] Expire each typing user entry after 5 seconds of inactivity and clear stale entries on channel switch/disconnect.

- [x] Task 3: Add permission-safe channel activity signaling for unread state (AC: 5, 6, 7)
  - [x] Introduce a lightweight server websocket event for unread activity (for example `channel_activity`) in `server/src/ws/protocol.rs`; include minimal payload (`guild_slug`, `channel_slug`, actor/message identifiers if needed).
  - [x] In `server/src/ws/gateway.rs`, emit this activity event on message creation in addition to existing `message_create` channel fanout.
  - [x] Ensure activity fanout is permission-safe: do not leak activity for channels where recipient lacks `VIEW_CHANNEL` (reuse existing permission resolution patterns from `message_service`).
  - [x] Extend client websocket protocol typing to parse the new event and map it into unread state updates.

- [x] Task 4: Implement unread activity state and channel/guild derivation (AC: 5, 6, 7)
  - [x] Add unread tracking keyed by `guild:channel` in client state (feature store), independent from `pendingNewCount` timeline mechanics.
  - [x] Mark channels unread when activity arrives for non-active channels; do not mark active channel unread while user is effectively reading.
  - [x] Clear unread state when channel becomes active/read, and derive per-guild unread boolean for GuildRail integration.
  - [x] Wire derived guild activity into `guildState.guilds` updates so existing `GuildRail.svelte` unread badge rendering remains the display surface.

- [x] Task 5: Render unread visuals in ChannelList and preserve accessibility semantics (AC: 5, 6)
  - [x] Extend channel domain/wire models as needed to carry computed unread state to `ChannelList.svelte`.
  - [x] Render unread text channels with bold label + ice blue dot while preserving active/hover/current-page styling.
  - [x] Keep keyboard/tree navigation behavior intact (`data-channel-nav`, arrow navigation, context menu keys) after unread UI changes.
  - [x] Ensure unread indicator is conveyed beyond color where possible (semantic labels/test IDs), consistent with UX accessibility guidance.

- [x] Task 6: Implement `Alt+Shift+Up/Down` unread-channel navigation (AC: 8)
  - [x] Add global shortcut handling in shell routing layer (`ShellRoute.svelte`) so the combination works from anywhere in-app.
  - [x] Navigate through ordered channel list positions (category-aware order) to previous/next unread channel in active guild.
  - [x] No-op safely when no unread targets exist; do not interfere with composer/editor shortcuts that use other modifier combinations.
  - [x] Use existing `goto('/{guild}/{channel}')` route transitions so route persistence and channel activation remain consistent.

- [x] Task 7: Add regression coverage and run quality gates (AC: all)
  - [x] Server integration tests in `server/tests/server_binds_to_configured_port.rs` for typing fanout, activity fanout targeting, and rate-limit/error behavior.
  - [x] Client tests:
    - [x] Typing indicator lifecycle and copy rules (`MessageArea.test.ts` and/or store tests), including 5-second expiry.
    - [x] Unread channel visual state in `ChannelList.test.ts`.
    - [x] Guild unread badge derivation updates in `GuildRail.test.ts` (where state wiring is affected).
    - [x] `Alt+Shift+Up/Down` unread navigation behavior in `ShellRoute.test.ts`.
  - [x] Run:
    - [x] `cd client && npm run lint && npm run check && npm run test && npm run build`
    - [x] `cd server && cargo fmt --check && cargo clippy -- -D warnings && RUST_TEST_THREADS=1 cargo test`

## Dev Notes

### Developer Context

- Server already supports `c_typing_start` parsing and `typing_start` channel broadcast (`ws/protocol.rs`, `ws/gateway.rs`, `ws/registry.rs`), but payload currently carries user/channel IDs only and no inactivity lifecycle.
- Client websocket op types already include `c_typing_start`, but message state only consumes `message_create`, `message_update`, `message_delete`, and `message_reaction_update` events.
- `MessageArea.svelte` has rich composer shortcuts and timeline `pendingNewCount` handling, but no typing indicator UI/state integration yet.
- `ChannelList.svelte` currently renders active vs non-active styles only; unread channel visual treatment is not implemented.
- `GuildRail.svelte` already renders unread badge when `guild.hasUnreadActivity` is true, but server guild responses currently do not provide unread fields.
- Current websocket subscription model tracks one active channel via `presenceState.setRouting(...)` -> `wsClient.setSubscription(guild, channel)`; message fanout is channel-targeted, so cross-channel unread state requires a distinct activity signal.

### Technical Requirements

- Preserve websocket envelope and naming conventions: client ops prefixed `c_`, server ops snake_case, payload fields snake_case.
- Maintain backend layering (`handlers -> services -> models`) and keep permission-sensitive logic in services, not handlers/gateway glue.
- Do not leak restricted channel activity: unread signaling must respect `VIEW_CHANNEL` checks and channel overrides.
- Typing lifecycle must be deterministic: per-channel/per-user timers, 5-second expiry, and cleanup on route/disconnect transitions.
- Keep existing timeline behavior stable (`pendingNewCount`, scroll restoration, optimistic messages, memory budget) while adding unread activity state.
- Respect UX and accessibility requirements: typing indicator placement above input, unread dot + bold channel label, keyboard shortcut behavior, and assistive-friendly signaling.
- Keep performance constraints in mind (NFR1/NFR8): avoid per-keystroke event storms and expensive recomputation on every render.

### Architecture Compliance

1. Keep wire contracts explicit (`snake_case` on the wire, typed camelCase transforms in client feature types/stores).
2. Extend websocket protocol through `server/src/ws/protocol.rs` and `client/src/lib/ws/protocol.ts` instead of ad-hoc string literals across components.
3. Reuse existing permission computation flows from message/channel services; avoid duplicating inconsistent authorization logic in gateway/registry.
4. Keep state boundaries clear: event ingestion in stores, presentation in Svelte components, routing/shortcut orchestration in shell.
5. Preserve existing guild/channel navigation persistence (`saveLastLocation`, `saveLastViewedChannel`) while adding unread navigation behavior.

### Library & Framework Requirements

- Current project baseline:
  - `svelte`: `^5.45.2`
  - `@mateothegreat/svelte5-router`: `^2.16.19`
  - `axum`: `0.8`
  - `sqlx`: `0.8`
- Latest checks during story creation:
  - `svelte` latest: `5.53.6`
  - `@mateothegreat/svelte5-router` latest: `2.16.19`
  - `axum` latest release tag: `axum-v0.8.8`
  - `sqlx` latest tag: `v0.8.6`
- No dependency upgrade is required to implement this story.

### File Structure Requirements

Expected primary touch points:

- Server
  - `server/src/ws/protocol.rs`
  - `server/src/ws/gateway.rs`
  - `server/src/ws/registry.rs`
  - `server/src/services/message_service.rs` (permission-aware activity dispatch helpers, if extracted)
  - `server/tests/server_binds_to_configured_port.rs`

- Client
  - `client/src/lib/ws/protocol.ts`
  - `client/src/lib/ws/client.ts` (only if subscription/event flow updates are required)
  - `client/src/lib/features/members/presenceStore.svelte.ts` (routing/subscription interplay)
  - `client/src/lib/features/chat/messageStore.svelte.ts` (realtime event ingestion + unread/typing state coordination)
  - `client/src/lib/features/chat/MessageArea.svelte` (typing indicator UI + emit behavior)
  - `client/src/lib/features/channel/types.ts`
  - `client/src/lib/features/channel/channelStore.svelte.ts`
  - `client/src/lib/features/channel/ChannelList.svelte`
  - `client/src/lib/features/guild/guildStore.svelte.ts`
  - `client/src/lib/features/guild/GuildRail.svelte`
  - `client/src/lib/features/shell/ShellRoute.svelte`
  - `client/src/lib/features/shell/ShellRoute.test.ts`
  - `client/src/lib/features/chat/MessageArea.test.ts`
  - `client/src/lib/features/chat/messageStore.test.ts`
  - `client/src/lib/features/channel/ChannelList.test.ts`
  - `client/src/lib/features/guild/GuildRail.test.ts`

### Testing Requirements

- Server integration coverage should verify:
  - typing events are accepted with `c_typing_start` and only delivered to subscribed channel recipients,
  - unread activity event fanout is scoped to authorized viewers,
  - websocket rate-limit and protocol error behavior remains intact.
- Client coverage should verify:
  - typing indicator text grammar and 5-second inactivity expiry (use fake timers),
  - self typing events are ignored,
  - unread channel indicator style/state transitions,
  - unread clear-on-view behavior when channel becomes active/read,
  - `Alt+Shift+Up/Down` unread navigation semantics.
- Preserve and re-run full quality gates before moving past implementation/review.

### Previous Story Intelligence

- Story 6.7 reinforced consistent REST/WS payload shape handling through typed transform layers; unread/typing event additions should follow the same contract discipline.
- Story 6.7 emphasized sanitize-safe rendering and performance-safe message rendering in `MessageArea`/`MessageBubble`; typing/unread UI must not regress those paths.
- Story 6.6 reinforced persist-before-broadcast and service-layer validation boundaries for realtime message behavior; new activity fanout should preserve those boundaries.
- Story 6.6 and 6.7 both used full client/server quality gates (with `RUST_TEST_THREADS=1` for shared-env stability) before status promotion.

### Git Intelligence Summary

Recent commit sequence and conventions:

- `46525a7` feat: finalize story 6-7 rich embeds and markdown rendering
- `b73f308` feat: finalize story 6-6 file upload and sharing
- `46550ff` feat: finalize story 6-5 emoji reactions
- `bfe2ad1` feat: finalize story 6-4 edit/delete own messages
- `8741b54` feat: finalize story 6-3 message history

### Latest Technical Information

- `svelte` latest package version: `5.53.6`
- `@mateothegreat/svelte5-router` latest package version: `2.16.19`
- `tokio-rs/axum` latest release tag: `axum-v0.8.8`
- `launchbadge/sqlx` latest tag: `v0.8.6`

### Project Context Reference

- No `project-context.md` file was discovered via `**/project-context.md`.
- Story context is derived from planning artifacts, implementation artifacts, source code, and recent git history.

### Story Completion Status

- Story 6.8 typing indicators and unread channel/guild activity behaviors are implemented across server and client surfaces.
- Senior code review fixes are applied and full client/server quality gates pass with story status set to `done`.

### Project Structure Notes

- Keep new unread/typing logic inside existing chat/channel/guild feature boundaries; avoid introducing cross-feature singletons that bypass store layering.
- Prefer extending existing websocket protocol + store ingestion patterns over one-off DOM/event hacks in components.
- Keep shortcut orchestration in shell/routing layer so keyboard behavior remains global and testable.

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Story 6.8: Typing Indicators and Channel Activity]
- [Source: _bmad-output/planning-artifacts/prd.md#User Experience & Navigation]
- [Source: _bmad-output/planning-artifacts/prd.md#Non-Functional Requirements]
- [Source: _bmad-output/planning-artifacts/architecture.md#API & Communication Patterns]
- [Source: _bmad-output/planning-artifacts/architecture.md#WebSocket Event Naming]
- [Source: _bmad-output/planning-artifacts/architecture.md#Structure Patterns]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#ChannelListItem]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#MessageInput]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Keyboard Shortcut Patterns]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Color blindness considerations]
- [Source: _bmad-output/implementation-artifacts/6-7-rich-embeds-and-markdown-rendering.md]
- [Source: _bmad-output/implementation-artifacts/6-6-file-upload-and-sharing.md]
- [Source: _bmad-output/implementation-artifacts/sprint-status.yaml]
- [Source: server/src/ws/protocol.rs]
- [Source: server/src/ws/gateway.rs]
- [Source: server/src/ws/registry.rs]
- [Source: server/src/services/message_service.rs]
- [Source: server/src/models/guild.rs]
- [Source: server/tests/server_binds_to_configured_port.rs]
- [Source: client/src/lib/ws/protocol.ts]
- [Source: client/src/lib/ws/client.ts]
- [Source: client/src/lib/features/members/presenceStore.svelte.ts]
- [Source: client/src/lib/features/chat/messageStore.svelte.ts]
- [Source: client/src/lib/features/chat/MessageArea.svelte]
- [Source: client/src/lib/features/channel/types.ts]
- [Source: client/src/lib/features/channel/ChannelList.svelte]
- [Source: client/src/lib/features/guild/types.ts]
- [Source: client/src/lib/features/guild/GuildRail.svelte]
- [Source: client/src/lib/features/shell/ShellRoute.svelte]
- [Source: client/src/lib/features/identity/navigationState.ts]
- [Source: client/src/lib/features/shell/ShellRoute.test.ts]
- [Source: client/src/lib/features/chat/MessageArea.test.ts]
- [Source: client/src/lib/features/chat/messageStore.test.ts]
- [Source: client/src/lib/features/channel/ChannelList.test.ts]
- [Source: client/src/lib/features/guild/GuildRail.test.ts]
- [Source: client/package.json]
- [Source: server/Cargo.toml]
- [Source: https://registry.npmjs.org/svelte/latest]
- [Source: https://registry.npmjs.org/@mateothegreat/svelte5-router/latest]
- [Source: https://api.github.com/repos/tokio-rs/axum/releases/latest]
- [Source: https://api.github.com/repos/launchbadge/sqlx/tags?per_page=1]

## Senior Developer Review (AI)

### Reviewer

Darko

### Date

2026-03-01

### Outcome

Approved after fixes

### Findings

1. **[HIGH] AC3 typing copy mismatch**  
   For 4+ typers, the indicator rendered "`A, B, and N others are typing...`" instead of required copy "`Several people are typing...`".  
   Evidence: `client/src/lib/features/chat/MessageArea.svelte`

2. **[MEDIUM] Global unread shortcut not truly global**  
   `Alt+Shift+Up/Down` was blocked when focus was inside editable elements, conflicting with UX guidance that global shortcuts work anywhere in-app.  
   Evidence: `client/src/lib/features/shell/ShellRoute.svelte`

### Fixes Applied

- Updated typing indicator grammar for 4+ users to return exact required copy: "`Several people are typing...`".
- Removed editable-target blocking for the unread navigation hotkey in shell routing.
- Added regression coverage:
  - `MessageArea.test.ts` now verifies the 4+ typers generic copy path.
  - `ShellRoute.test.ts` now verifies unread hotkey navigation when focus is in a textarea.
- Re-ran full quality gates:
  - `cd client && npm run lint && npm run check && npm run test && npm run build`
  - `cd server && cargo fmt --check && cargo clippy -- -D warnings && RUST_TEST_THREADS=1 cargo test`

## Dev Agent Record

### Agent Model Used

GPT-5.3-Codex (model ID: gpt-5.3-codex)

### Debug Log References

- `_bmad/core/tasks/workflow.xml` loaded and executed with `_bmad/bmm/workflows/4-implementation/dev-story/workflow.yaml`.
- Implemented server `channel_activity` websocket op/event fanout with permission-safe viewer filtering.
- Validation run completed successfully:
  - `cd client && npm run lint && npm run check && npm run test && npm run build`
  - `cd server && cargo fmt && cargo clippy -- -D warnings && RUST_TEST_THREADS=1 cargo test`

### Completion Notes List

- Added backend unread activity signaling (`channel_activity`) and permission-safe fanout to guild subscribers who can view the channel.
- Added client typing lifecycle support (throttled typing emit, per-channel typing state, 5-second expiry, and composer indicator rendering).
- Added client unread state derivation across channel and guild stores, including unread clear-on-view behavior.
- Added unread channel visuals in `ChannelList` and global unread navigation with `Alt+Shift+ArrowUp/ArrowDown` in shell routing.
- Added regression tests for typing lifecycle, unread visuals/state, and unread keyboard navigation, and passed full quality gates.

### File List

- server/src/ws/protocol.rs
- server/src/ws/registry.rs
- server/src/services/message_service.rs
- server/src/ws/gateway.rs
- server/src/handlers/messages.rs
- server/tests/server_binds_to_configured_port.rs
- client/src/lib/features/chat/messageStore.svelte.ts
- client/src/lib/features/chat/MessageArea.svelte
- client/src/lib/features/channel/types.ts
- client/src/lib/features/channel/channelStore.svelte.ts
- client/src/lib/features/guild/guildStore.svelte.ts
- client/src/lib/features/channel/ChannelList.svelte
- client/src/lib/features/shell/ShellRoute.svelte
- client/src/lib/features/chat/messageStore.test.ts
- client/src/lib/features/chat/MessageArea.test.ts
- client/src/lib/features/channel/ChannelList.test.ts
- client/src/lib/features/shell/ShellRoute.test.ts
- _bmad-output/implementation-artifacts/6-8-typing-indicators-and-channel-activity.md
- _bmad-output/implementation-artifacts/sprint-status.yaml

### Change Log

- 2026-03-01: Implemented Story 6.8 typing indicators, unread channel/guild activity indicators, unread navigation shortcut, and passed full client/server quality gates.
- 2026-03-01: Senior code review found AC3 copy mismatch and global shortcut scope gap; fixes applied with added regression tests and full quality-gate revalidation.
