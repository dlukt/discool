# Story 6.9: Direct Messages

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a **user**,
I want to send direct messages to other users,
so that I can have private conversations outside of guild channels.

## Acceptance Criteria

1. **Given** a user clicks "Send DM" on another user's profile popover or member list entry  
   **When** the DM view opens  
   **Then** it shows the conversation history with that user.

2. **Given** a user has active direct conversations  
   **When** they navigate to Home  
   **Then** DMs appear under the Home button in the GuildRail as a DM list.

3. **Given** a DM conversation is open  
   **When** messages are rendered and composed  
   **Then** the view uses the same `MessageBubble` and `MessageInput` components as channel chat.

4. **Given** DM persistence is required  
   **When** schema migrations are applied  
   **Then** DMs are stored with a dedicated `dm_channels` table (introduced in this story).

5. **Given** two users share at least one guild  
   **When** one user opens or sends a DM to the other  
   **Then** DM messaging works across guild boundaries.

6. **Given** unread DM activity exists  
   **When** GuildRail renders the Home button  
   **Then** DM notifications appear as unread badges on Home.

7. **Given** a user opens quick switcher (`Ctrl+K`)  
   **When** search results render  
   **Then** DM conversations are included in results.

## Tasks / Subtasks

- [x] Task 1: Add DM persistence model and migrations (AC: 4, 5)
  - [x] Add migration `server/migrations/0023_create_dm_channels.sql` with canonical participant pairing (`user_low_id`, `user_high_id`), creation timestamps, and indexes for participant lookup.
  - [x] Add DM message persistence migration (recommended: `0024_create_dm_messages.sql`) keyed by `dm_channel_id` to avoid breaking existing guild-channel message invariants.
  - [x] Add corresponding Rust model modules (`server/src/models/dm_channel.rs`, `server/src/models/dm_message.rs`) and register in `server/src/models/mod.rs`.

- [x] Task 2: Implement server-side DM services and API endpoints (AC: 1, 2, 4, 5)
  - [x] Create `server/src/services/dm_service.rs` with open-or-create conversation, list conversations, and list/send DM messages.
  - [x] Enforce mutual-guild eligibility before DM creation/send using a dedicated membership helper (new shared-guild query in `guild_member` model).
  - [x] Add `server/src/handlers/dms.rs` and wire routes in `server/src/handlers/mod.rs` (for example: `POST /api/v1/dms`, `GET /api/v1/dms`, `GET /api/v1/dms/{dm_slug}/messages`).
  - [x] Preserve API envelope contract: all success responses wrapped in `{ "data": ... }` (with `cursor` where paginated).

- [x] Task 3: Extend websocket protocol for DM realtime delivery (AC: 1, 2, 6)
  - [x] Extend protocol typing in `server/src/ws/protocol.rs` and `client/src/lib/ws/protocol.ts` with explicit DM operations/events (for example: `c_dm_subscribe`, `c_dm_message_create`, `dm_message_create`, `dm_activity`).
  - [x] Update websocket gateway dispatch in `server/src/ws/gateway.rs` for DM ops and message fanout.
  - [x] Add DM-targeted subscription/fanout support in `server/src/ws/registry.rs` (or equivalent extension) without weakening existing guild/channel routing.
  - [x] Keep rate-limit behavior and structured websocket error format consistent with existing gateway behavior.

- [x] Task 4: Wire "Send DM" intent into navigable DM conversations (AC: 1, 2, 5)
  - [x] Consume the already-emitted `discool:open-dm-intent` event in shell orchestration (`client/src/lib/features/shell/ShellRoute.svelte`) to open/create a conversation and navigate to it.
  - [x] Add a dedicated DM route mode in `client/src/routes/routes.ts` (and route tests), keeping channel mode behavior unchanged.
  - [x] Ensure route persistence logic in `client/src/App.svelte` and `client/src/lib/features/identity/navigationState.ts` supports DM paths without corrupting guild-channel last-viewed state.

- [x] Task 5: Add Home DM list and unread badge surface (AC: 2, 6)
  - [x] Add DM list UI under Home (recommended new feature module: `client/src/lib/features/dm/` with `DMList.svelte`, `dmStore.svelte.ts`, `dmApi.ts`, `types.ts`).
  - [x] Update `GuildRail.svelte` Home button to render unread DM badge state independent of guild unread state.
  - [x] Clear DM unread state when conversation becomes active/read, while preserving existing channel unread logic.

- [x] Task 6: Reuse chat presentation components for DMs (AC: 1, 3)
  - [x] Refactor `MessageArea.svelte` and `messageStore.svelte.ts` to support conversation context (`channel` vs `dm`) while still rendering through existing `MessageBubble` + composer flow.
  - [x] Keep guild-channel features intact (history pagination, typing, unread, attachment flows) and avoid regressions by isolating DM-specific branching.
  - [x] Keep wire-to-domain transformation explicit (`snake_case` wire payloads mapped to typed client structures).

- [x] Task 7: Include DMs in quick-switcher search behavior (AC: 7)
  - [x] Add DM conversations into the quick-switcher data source/search provider used by `Ctrl+K`.
  - [x] If full quick-switcher UI is not yet present, add the minimal entry surface needed to satisfy AC7 now and structure it for Story 6.12 extension.
  - [x] Ensure DM results are grouped and navigable consistently with existing guild/channel navigation conventions.

- [x] Task 8: Add regression coverage and run quality gates (AC: all)
  - [x] Server integration tests in `server/tests/server_binds_to_configured_port.rs` for: DM creation authorization (mutual guild), DM message history retrieval, websocket DM fanout targeting, and unread DM activity signaling.
  - [x] Client tests for: DM intent event handling, Home DM list rendering, Home unread badge behavior, DM route persistence, MessageArea DM reuse, and quick-switcher DM result inclusion.
  - [x] Run:
    - [x] `cd client && npm run lint && npm run check && npm run test && npm run build`
    - [x] `cd server && cargo fmt --check && cargo clippy -- -D warnings && RUST_TEST_THREADS=1 cargo test`

## Dev Notes

### Developer Context

- `MemberList.svelte` already emits `discool:open-dm-intent` with `{ guildSlug, userId }`, but there is no consumer yet in shell routing/state.
- `ShellRoute.svelte` currently handles only `home | channel | settings | admin` and has no DM-specific route mode or intent listener.
- `GuildRail.svelte` Home control currently routes to `/` and has no DM badge/list model; unread badge logic only exists for guild entries.
- `MessageArea.svelte` and `messageStore.svelte.ts` are channel-only (`guild_slug` + `channel_slug`) and gate most behavior on `mode === 'channel'`.
- REST message APIs are currently guild-channel scoped (`/api/v1/guilds/{guild}/channels/{channel}/messages`); there are no DM endpoints.
- Server message schema (`messages`) is guild/channel-centric, and no `dm_channels` persistence exists yet.
- Websocket subscription model currently tracks a single guild/channel pair through `wsClient.setSubscription(...)` and registry channel keys.

### Technical Requirements

- Preserve backend layering: handlers -> services -> models; avoid embedding business logic in websocket registry/gateway glue.
- Enforce DM authorization using explicit mutual-guild checks; do not allow DM creation/sending without shared guild membership.
- Preserve existing sanitization and validation behavior for message content (control chars, max length, server-side HTML escaping).
- Keep websocket and REST error contracts explicit and unchanged in shape.
- Maintain transport efficiency and memory constraints for chat timeline behavior (NFR1/NFR7/NFR8) when adding DM context.
- Keep all wire payload fields `snake_case`; client-side mapped types remain `camelCase`.

### Architecture Compliance

1. Keep all new API routes under `/api/v1` with consistent envelope/HTTP semantics.
2. Extend protocol definitions centrally (`ws/protocol`) instead of ad-hoc op strings in components.
3. Reuse existing state boundaries: websocket ingress in stores, UI rendering in Svelte components, routing in shell layer.
4. Preserve existing guild/channel unread behavior while introducing separate DM unread state.
5. Keep permission and membership enforcement server-side; do not rely on client filtering for DM access control.

### Library & Framework Requirements

- Current project baselines:
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
  - `server/migrations/0023_create_dm_channels.sql` (new)
  - `server/migrations/0024_create_dm_messages.sql` (new, if separate DM message store)
  - `server/src/models/dm_channel.rs` (new)
  - `server/src/models/dm_message.rs` (new)
  - `server/src/models/guild_member.rs` (shared-guild query helper)
  - `server/src/models/mod.rs`
  - `server/src/services/dm_service.rs` (new)
  - `server/src/handlers/dms.rs` (new)
  - `server/src/handlers/mod.rs`
  - `server/src/ws/protocol.rs`
  - `server/src/ws/gateway.rs`
  - `server/src/ws/registry.rs`
  - `server/tests/server_binds_to_configured_port.rs`

- Client
  - `client/src/routes/routes.ts`
  - `client/src/routes/routes.test.ts`
  - `client/src/App.svelte`
  - `client/src/lib/features/identity/navigationState.ts`
  - `client/src/lib/features/shell/ShellRoute.svelte`
  - `client/src/lib/features/shell/ShellRoute.test.ts`
  - `client/src/lib/features/guild/GuildRail.svelte`
  - `client/src/lib/features/guild/GuildRail.test.ts`
  - `client/src/lib/features/members/MemberList.svelte`
  - `client/src/lib/features/members/MemberList.test.ts`
  - `client/src/lib/features/chat/MessageArea.svelte`
  - `client/src/lib/features/chat/messageStore.svelte.ts`
  - `client/src/lib/ws/protocol.ts`
  - `client/src/lib/ws/client.ts`
  - `client/src/lib/features/dm/DMList.svelte` (new)
  - `client/src/lib/features/dm/dmStore.svelte.ts` (new)
  - `client/src/lib/features/dm/dmApi.ts` (new)
  - `client/src/lib/features/dm/types.ts` (new)

### Testing Requirements

- Server integration coverage must verify:
  - DM creation is blocked unless users share at least one guild.
  - DM history API enforces participant-only access.
  - Websocket DM event fanout reaches only conversation participants.
  - Home unread DM activity signal is emitted and cleared correctly.
- Client coverage must verify:
  - `discool:open-dm-intent` opens/navigates to DM conversation.
  - Home DM list rendering and Home unread badge transitions.
  - DM message timeline/composer reuse existing message components.
  - DM routes are persisted/restored safely without breaking guild-channel persistence.
  - Quick-switcher DM inclusion for `Ctrl+K`.
- Preserve and re-run full quality gates before status promotion.

### Previous Story Intelligence

- Story 6.8 reinforced strict websocket contract discipline (`c_` client ops, `snake_case` wire payloads, typed parse/transform layers).
- Story 6.8 required permission-safe fanout logic for activity signals; DM event fanout must apply similarly strict recipient scoping.
- Story 6.8 review fixes highlighted UX strictness (exact copy and truly global shortcuts); DM UX text and keyboard behavior should be validated explicitly.
- Story 6.8 completion used full quality gates, including `RUST_TEST_THREADS=1` for stable server test execution in shared environments.

### Git Intelligence Summary

Recent commit sequence:

- `aef45a9` feat: finalize story 6-8 typing indicators and channel activity
- `46525a7` feat: finalize story 6-7 rich embeds and markdown rendering
- `b73f308` feat: finalize story 6-6 file upload and sharing
- `46550ff` feat: finalize story 6-5 emoji reactions
- `bfe2ad1` feat: finalize story 6-4 edit/delete own messages

Recurring implementation pattern:

- Server and client changes land together per story, with protocol/type updates mirrored on both sides.
- `server/tests/server_binds_to_configured_port.rs` is the primary integration safety net for websocket/message behavior.
- Chat feature work typically touches `MessageArea`, `messageStore`, protocol typing, and corresponding test files in one cohesive slice.

### Latest Technical Information

- `svelte` latest package version: `5.53.6`
- `@mateothegreat/svelte5-router` latest package version: `2.16.19`
- `tokio-rs/axum` latest release tag: `axum-v0.8.8`
- `launchbadge/sqlx` latest tag: `v0.8.6`

### Project Context Reference

- No `project-context.md` file was discovered via `**/project-context.md`.
- Story context is derived from planning artifacts, implementation artifacts, source code, and recent git history.

### Story Completion Status

- Story file created: `_bmad-output/implementation-artifacts/6-9-direct-messages.md`
- Sprint status target: `6-9-direct-messages -> ready-for-dev`
- Completion note: Ultimate context engine analysis completed - comprehensive developer guide created.

### Project Structure Notes

- Keep DM implementation isolated to explicit DM service/store modules while reusing existing chat presentation components.
- Avoid introducing implicit "fake guild" shortcuts that blur guild-channel and DM authorization boundaries.
- Preserve route and persistence behavior for existing guild/channel flows while adding explicit DM routing semantics.

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Story 6.9: Direct Messages]
- [Source: _bmad-output/planning-artifacts/epics.md#Story 6.12: Quick Switcher]
- [Source: _bmad-output/planning-artifacts/prd.md#Text Communication]
- [Source: _bmad-output/planning-artifacts/prd.md#User Experience & Navigation]
- [Source: _bmad-output/planning-artifacts/prd.md#Non-Functional Requirements]
- [Source: _bmad-output/planning-artifacts/architecture.md#API & Communication Patterns]
- [Source: _bmad-output/planning-artifacts/architecture.md#Implementation Patterns & Consistency Rules]
- [Source: _bmad-output/planning-artifacts/architecture.md#Architectural Boundaries]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#GuildRail]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#User context menu]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Quick switcher (`Ctrl+K`)]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Navigation Patterns]
- [Source: _bmad-output/implementation-artifacts/6-8-typing-indicators-and-channel-activity.md]
- [Source: _bmad-output/implementation-artifacts/sprint-status.yaml]
- [Source: client/src/lib/features/members/MemberList.svelte]
- [Source: client/src/lib/features/shell/ShellRoute.svelte]
- [Source: client/src/lib/features/guild/GuildRail.svelte]
- [Source: client/src/lib/features/chat/MessageArea.svelte]
- [Source: client/src/lib/features/chat/messageStore.svelte.ts]
- [Source: client/src/lib/ws/client.ts]
- [Source: client/src/lib/ws/protocol.ts]
- [Source: client/src/routes/routes.ts]
- [Source: client/src/App.svelte]
- [Source: server/src/handlers/mod.rs]
- [Source: server/src/handlers/messages.rs]
- [Source: server/src/services/message_service.rs]
- [Source: server/src/ws/gateway.rs]
- [Source: server/src/ws/registry.rs]
- [Source: server/src/ws/protocol.rs]
- [Source: server/src/models/message.rs]
- [Source: server/migrations/0018_create_messages.sql]
- [Source: server/migrations/0019_add_messages_cursor_index.sql]
- [Source: server/src/models/guild_member.rs]
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

Approved (follow-up pass found no outstanding HIGH/MEDIUM issues)

### Findings

1. **[HIGH] DM send path missed shared-guild eligibility re-check (AC5)**  
   `create_dm_message` validated DM participation but did not re-check `users_share_guild`, so users who no longer shared any guild could still send DMs.  
   Evidence: `server/src/services/dm_service.rs:258-315` (before fix)

2. **[MEDIUM] Concurrent DM open conflict path could return internal error**  
   After `insert_dm_channel` conflict, the immediate follow-up lookup could race a concurrent transaction commit and return `None`, causing `"Failed to resolve DM channel after insert conflict"`.  
   Evidence: `server/src/services/dm_service.rs:121-147` (before fix)

### Fixes Applied

- Added a shared-guild check to `create_dm_message` before persisting DM messages.
- Added bounded retry-based resolution for DM channel lookup after insert conflicts.
- Added regression test `dm_message_send_requires_shared_guild_membership`.
- Re-ran quality gates:
  - `cd client && npm run lint && npm run check && npm run test && npm run build`
  - `cd server && cargo fmt --check && cargo clippy -- -D warnings && RUST_TEST_THREADS=1 cargo test`
- Follow-up YOLO review pass:
  - Re-ran `cd client && npm run lint && npm run check && npm run test && npm run build && cd ../server && cargo fmt --check && cargo clippy -- -D warnings && RUST_TEST_THREADS=1 cargo test`
  - Verified story File List matches actual changed files (including untracked files)
  - Confirmed no outstanding HIGH/MEDIUM findings

## Dev Agent Record

### Agent Model Used

GitHub Copilot CLI (GPT-5 class model)

### Debug Log References

- `cd client && npm run check --silent`
- `cd client && npm run lint --silent && npm run check --silent && npm run test --silent && npm run build --silent`
- `cd server && cargo test --test server_binds_to_configured_port dm_`
- `cd server && cargo fmt`
- `cd server && cargo clippy -- -D warnings && RUST_TEST_THREADS=1 cargo test`

### Completion Notes List

- Implemented DM persistence with canonical participant pairing (`dm_channels`) and dedicated `dm_messages` storage plus indexes.
- Added DM backend service + REST endpoints (`POST /api/v1/dms`, `GET /api/v1/dms`, `GET /api/v1/dms/{dm_slug}/messages`) with participant and shared-guild authorization.
- Extended websocket protocol, gateway, and registry with DM subscribe/create/activity operations and participant-scoped fanout.
- Added client DM route mode, DM state/API/list UI, shell DM intent handling, DM quick-switcher inclusion, and chat store/message area DM reuse.
- Added and updated client/server tests for DM intent, home DM badge/list, DM chat behavior, DM fanout, DM history authorization, and DM activity signaling.
- Ran full client and server quality gates successfully.

### File List

- `server/migrations/0023_create_dm_channels.sql`
- `server/migrations/0024_create_dm_messages.sql`
- `server/src/models/dm_channel.rs`
- `server/src/models/dm_message.rs`
- `server/src/models/guild_member.rs`
- `server/src/models/mod.rs`
- `server/src/services/dm_service.rs`
- `server/src/services/mod.rs`
- `server/src/handlers/dms.rs`
- `server/src/handlers/mod.rs`
- `server/src/ws/protocol.rs`
- `server/src/ws/registry.rs`
- `server/src/ws/gateway.rs`
- `server/tests/server_binds_to_configured_port.rs`
- `client/src/lib/features/dm/types.ts`
- `client/src/lib/features/dm/dmApi.ts`
- `client/src/lib/features/dm/dmStore.svelte.ts`
- `client/src/lib/features/dm/DMList.svelte`
- `client/src/lib/ws/protocol.ts`
- `client/src/lib/ws/client.ts`
- `client/src/lib/ws/client.test.ts`
- `client/src/routes/routes.ts`
- `client/src/routes/routes.test.ts`
- `client/src/App.svelte`
- `client/src/lib/features/shell/ShellRoute.svelte`
- `client/src/lib/features/shell/ShellRoute.test.ts`
- `client/src/lib/features/shell/__mocks__/GuildRailMock.svelte`
- `client/src/lib/features/shell/__mocks__/ChannelListMock.svelte`
- `client/src/lib/features/shell/__mocks__/MessageAreaMock.svelte`
- `client/src/lib/features/shell/__mocks__/MemberListMock.svelte`
- `client/src/lib/features/guild/GuildRail.svelte`
- `client/src/lib/features/guild/GuildRail.test.ts`
- `client/src/lib/features/chat/types.ts`
- `client/src/lib/features/chat/messageStore.svelte.ts`
- `client/src/lib/features/chat/messageStore.test.ts`
- `client/src/lib/features/chat/MessageArea.svelte`
- `client/src/lib/features/chat/MessageArea.test.ts`
- `_bmad-output/implementation-artifacts/6-9-direct-messages.md`
- `_bmad-output/implementation-artifacts/sprint-status.yaml`

### Change Log

- 2026-03-01: Senior review identified a DM send authorization gap and DM conflict-resolution race; fixes and regression test added, story status moved to `in-progress` per findings policy.
- 2026-03-01: Follow-up YOLO code review found no remaining HIGH/MEDIUM findings; story status moved to `done`.
