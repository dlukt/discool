# Story 4.6: Join Guild via Invite Link

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a **new or existing user**,
I want to click an invite link and join a guild,
so that I can start participating in a community immediately.

## Acceptance Criteria

1. **Given** a user clicks a Discool invite link (e.g., `instance.com/invite/abc123`)  
   **When** the SPA loads  
   **Then** if the user has no identity: the onboarding screen shows guild icon + "Pick a username to join [Guild Name]"

2. **Given** a user has completed identity verification for the instance  
   **When** they continue invite onboarding (new identity or one-click existing identity)  
   **Then** guild membership is created (guild_members table migration in this story)

3. **Given** guild membership is created via invite join  
   **When** the join flow completes  
   **Then** the user lands in the guild's default channel with the text input focused and ready to type

4. **Given** the guild has a welcome screen configured  
   **When** a user joins via invite  
   **Then** the welcome screen is shown with rules/TOS and an "Accept & Continue" action before landing

5. **Given** join succeeds  
   **When** the app refreshes guild data  
   **Then** the joined guild appears in the user's GuildRail

6. **Given** invite join cannot be completed  
   **When** invite is invalid/expired or the instance is unavailable  
   **Then** invalid invite shows: "This invite link is invalid or has expired"  
   **And** unreachable instance shows: "This instance is currently unreachable. Your invite link will work when it's back online."

7. **Given** invite onboarding paths for both user types  
   **When** measured in realistic browser conditions  
   **Then** new identity creation completes in under 30 seconds  
   **And** existing identity join completes in under 10 seconds

8. **Given** an invite URL is shared in chat apps  
   **When** link unfurl metadata is fetched  
   **Then** the invite link renders OpenGraph metadata containing guild name and icon

## Tasks / Subtasks

- [x] Task 1: Add guild membership persistence and invite-consumption primitives (AC: 2, 5, 6)
  - [x] Add `server/migrations/0014_create_guild_members.sql` with `guild_members` (`guild_id`, `user_id`, `joined_at`, `joined_via_invite_code`) and uniqueness on (`guild_id`, `user_id`).
  - [x] Add `server/src/models/guild_member.rs` and wire `server/src/models/mod.rs` for membership insert/find/list helpers (SQLite + Postgres paths).
  - [x] Extend invite model/service operations to resolve invite by code and consume single-use invites atomically (no race-driven double joins).

- [x] Task 2: Expose invite resolution and invite-join APIs for onboarding (AC: 1, 2, 6, 8)
  - [x] Add public invite metadata endpoint (e.g., `GET /api/v1/invites/{invite_code}`) returning guild name/icon/default channel slug and welcome-screen metadata.
  - [x] Add authenticated invite-join endpoint (e.g., `POST /api/v1/invites/{invite_code}/join`) that creates membership idempotently and returns landing route details.
  - [x] Keep envelope contract (`{ "data": ... }`) and explicit `AppError` mapping with exact UX-facing error strings required by AC 6.

- [x] Task 3: Make guild/channel/category reads membership-aware while keeping owner-only mutations (AC: 3, 5)
  - [x] Update guild listing to include guilds where user is owner or member; preserve `is_owner` semantics for UI gating.
  - [x] Update channel/category list authorization to allow guild members to read channel structure.
  - [x] Keep create/update/delete/reorder/invite-management flows owner-gated until Epic 5 permission engine is in place.

- [x] Task 4: Implement invite deep-link onboarding in SPA routes/app shell (AC: 1, 2, 5, 6, 7)
  - [x] Add client invite route handling for `/invite/{code}` and persist invite context through identity creation/auth flows.
  - [x] Show onboarding copy for new users: guild icon + "Pick a username to join [Guild Name]" and for existing identities keep one-click join confirmation.
  - [x] After auth success, call invite-join API, refresh guild state, and navigate to `/{guildSlug}/{defaultChannelSlug}`.

- [x] Task 5: Implement optional welcome-screen gate and landing focus behavior (AC: 3, 4)
  - [x] Render welcome rules/TOS gate only when invite metadata indicates welcome content is configured; otherwise skip directly to landing.
  - [x] Ensure final landing path focuses the message input control for immediate typing.
  - [x] Keep behavior backward-compatible with current placeholder chat UI until Epic 6 message features fully land.

- [x] Task 6: Serve OpenGraph metadata for invite URLs without introducing SSR (AC: 8)
  - [x] Add server-side invite HTML/meta response for `/invite/{code}` (guild title/description/icon) while keeping SPA boot behavior.
  - [x] Preserve existing static asset caching semantics and SPA fallback behavior for non-file routes.

- [x] Task 7: Extend tests for invite-join lifecycle and regressions (AC: all)
  - [x] Server integration tests in `server/tests/server_binds_to_configured_port.rs` for resolve/join success, single-use exhaustion, revoked/invalid invite errors, and membership-aware guild/channel reads.
  - [x] Client tests for invite deep-link onboarding, existing-identity one-click join, exact error copy, GuildRail update, and landing focus behavior.
  - [x] Add/extend static handler tests for invite OpenGraph response behavior.

- [x] Task 8: Run quality gates (AC: all)
  - [x] `cd client && npm run lint && npm run check && npm run test`
  - [x] `cd client && npm run build`
  - [x] `cd server && cargo fmt --check && cargo clippy -- -D warnings && cargo test`

## Dev Notes

### Developer Context

- Story 4.5 already shipped invite creation/list/revoke and emits canonical invite URLs (`/invite/{code}`), but there is no invite redemption flow yet.
- Current auth cross-instance path (`auth/challenge` + `auth/verify` with `cross_instance.enabled`) can provision a user identity on the instance, but does not create guild membership.
- Current guild/channel/category reads are effectively owner-scoped in services, which blocks post-join non-owner navigation unless membership-aware reads are added in this story.

### Technical Requirements

- Invite join must be idempotent for already-joined users (`guild_id` + `user_id` unique) and must not silently double-consume single-use invites.
- Preserve API field casing conventions (`snake_case` wire, `camelCase` client mapping) and current error envelope shape.
- Use explicit, deterministic errors:
  - invalid/expired invite → exact invalid copy from AC 6
  - network/instance unavailable → exact unreachable copy from AC 6
- Keep invite join latency paths tight (no redundant full-page reloads) to satisfy AC 7 timings.
- Welcome screen behavior must be optional and data-driven; if no configuration exists, flow must continue directly to landing.

### Architecture Compliance

1. Preserve server layering:
   - handlers = HTTP boundary + payload parsing
   - services = authorization + business rules + invite join orchestration
   - models = SQL access and atomic update helpers
2. Keep REST routes under `/api/v1` and preserve current `{ "data": ... }` response contract.
3. Maintain dual database compatibility (Postgres + SQLite) for all new membership and invite-join queries.
4. Do not introduce SSR/Node runtime; invite unfurl support must be Rust-served HTML/meta as documented architecture.

### Library & Framework Requirements

- Frontend stack remains Svelte 5 runes + `@mateothegreat/svelte5-router` for route handling.
- Backend stack remains Axum 0.8 + sqlx 0.8 with existing auth middleware and `AppError` taxonomy.
- Avoid introducing new state-management or routing dependencies; reuse current `App.svelte`, identity store, guild store, and route patterns.

### File Structure Requirements

Expected primary touch points:

- `server/migrations/0014_create_guild_members.sql` (new)
- `server/src/models/mod.rs`
- `server/src/models/guild_member.rs` (new)
- `server/src/models/guild.rs`
- `server/src/models/guild_invite.rs`
- `server/src/services/mod.rs`
- `server/src/services/guild_service.rs`
- `server/src/services/channel_service.rs`
- `server/src/services/category_service.rs`
- `server/src/services/guild_invite_service.rs` (or new invite-join service module)
- `server/src/handlers/mod.rs`
- `server/src/handlers/invites.rs` (extend) and/or dedicated invite-join handler module
- `server/src/static_files.rs` (invite OpenGraph handling)
- `server/tests/server_binds_to_configured_port.rs`
- `client/src/App.svelte`
- `client/src/routes/routes.ts`
- `client/src/lib/features/guild/guildApi.ts`
- `client/src/lib/features/guild/types.ts`
- `client/src/lib/features/guild/guildStore.svelte.ts`
- `client/src/lib/features/identity/LoginView.svelte`
- `client/src/lib/features/identity/CrossInstanceJoinPrompt.svelte`
- `client/src/lib/features/chat/MessageArea.svelte`
- `client/src/lib/features/guild/InviteModal.svelte` (invite URL/query compatibility adjustments as needed)
- Related client tests in guild/identity/app shell surfaces

### Testing Requirements

- Server:
  - Verify invite metadata resolve endpoint for active, revoked, consumed, and unknown invites.
  - Verify invite join creates membership and is idempotent on repeated join by same user.
  - Verify single-use invite transitions to invalid after successful consumption.
  - Verify guild list includes joined (non-owned) guilds and that channel/category list reads are member-accessible while mutations remain forbidden for non-owners.
  - Verify invite URL OpenGraph HTML/meta output includes guild name/icon for valid invites.
- Client:
  - Verify `/invite/{code}` deep-link flow for new identity and existing identity paths.
  - Verify exact error copy for invalid/expired invite and unreachable instance.
  - Verify successful join updates GuildRail and navigates to default channel.
  - Verify landing behavior focuses message input (or equivalent immediate-input control).

### Previous Story Intelligence

- Story 4.5 established invite persistence (`guild_invites`), owner-gated invite APIs under `/api/v1/guilds/{guild_slug}/invites`, and canonical `invite_url` payload values (`/invite/{code}`).
- `InviteModal.svelte` already appends `guild_name` query data and enforces copy feedback text `"Invite link copied"`; Story 4.6 should reuse that context rather than inventing a second invite URL format.
- Current shell/channel surfaces and tests already follow a vertical-slice pattern (handler → service → model and feature API → store → component); this story should extend that pattern for invite join and membership reads.

### Git Intelligence Summary

- `e735b72 feat: implement story 4-5 invite management` provides the immediate base for invite code lifecycle and UI entry points.
- `599b44e feat: finalize channel and category management` confirms channel/category infrastructure is available but currently owner-centric in service authorization.
- Recent workflow/status commits (`e502990`, `d123729`) indicate sprint/story tracking is maintained alongside implementation artifacts and should stay consistent.

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
   - SQLx: latest stable remains `0.8.x` line
3. No dependency upgrade is required for Story 4.6; implement against existing pinned versions.

### Project Context Reference

- No `project-context.md` was discovered via `**/project-context.md`.
- Story context is derived from planning artifacts, prior Epic 4 implementation artifacts, and current runtime client/server code.

### Story Completion Status

- Ultimate context analysis completed — comprehensive developer implementation guide created.
- Story status set to `ready-for-dev`.

### Project Structure Notes

- Story 4.6 is the bridge between invite generation (Story 4.5) and broader member/permission systems (Epic 5+); keep scope focused on invite redemption and member read-access baseline.
- Welcome-screen display is required when configured, but configuration UX has a known planning gap; implement display support with safe optional defaults and avoid blocking join on missing config.

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Epic 4: Guilds, Channels & Invites]
- [Source: _bmad-output/planning-artifacts/epics.md#Story 4.6: Join Guild via Invite Link]
- [Source: _bmad-output/planning-artifacts/prd.md#Success Criteria]
- [Source: _bmad-output/planning-artifacts/prd.md#Web Application Specific Requirements]
- [Source: _bmad-output/planning-artifacts/architecture.md#Technical Constraints & Dependencies]
- [Source: _bmad-output/planning-artifacts/architecture.md#Implementation Patterns & Consistency Rules]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Experience Mechanics]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Journey 1: New User Onboarding (Liam)]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#InviteLinkCard]
- [Source: _bmad-output/planning-artifacts/implementation-readiness-report-2026-02-17.md#Epic 4: Guilds, Channels & Invites]
- [Source: _bmad-output/implementation-artifacts/4-5-invite-link-generation-and-management.md]
- [Source: _bmad-output/implementation-artifacts/sprint-status.yaml]
- [Source: client/src/App.svelte]
- [Source: client/src/routes/routes.ts]
- [Source: client/src/lib/features/guild/InviteModal.svelte]
- [Source: client/src/lib/features/guild/guildApi.ts]
- [Source: client/src/lib/features/guild/guildStore.svelte.ts]
- [Source: client/src/lib/features/identity/identityStore.svelte.ts]
- [Source: client/src/lib/features/identity/CrossInstanceJoinPrompt.svelte]
- [Source: client/src/lib/features/identity/LoginView.svelte]
- [Source: server/src/handlers/mod.rs]
- [Source: server/src/handlers/auth.rs]
- [Source: server/src/handlers/invites.rs]
- [Source: server/src/services/auth_service.rs]
- [Source: server/src/services/guild_service.rs]
- [Source: server/src/services/channel_service.rs]
- [Source: server/src/services/category_service.rs]
- [Source: server/src/services/guild_invite_service.rs]
- [Source: server/src/models/guild.rs]
- [Source: server/src/models/guild_invite.rs]
- [Source: server/src/static_files.rs]
- [Source: server/migrations/0010_create_guilds.sql]
- [Source: server/migrations/0011_create_channels.sql]
- [Source: server/migrations/0013_create_guild_invites.sql]
- [Source: server/tests/server_binds_to_configured_port.rs]
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
- Story source selection parsed from user input: `4-6` → `4-6-join-guild-via-invite-link`
- Validation commands:
  - `cd client && npm run lint && npm run check && npm run test && npm run build`
  - `cd server && cargo fmt --check && cargo clippy -- -D warnings && cargo test`

### Completion Notes List

- Added membership persistence (`guild_members`) and idempotent invite join consumption for reusable/single-use invites.
- Added public invite resolve and authenticated join APIs under `/api/v1/invites/{invite_code}` and `/api/v1/invites/{invite_code}/join`.
- Expanded read authorization so joined members can list guilds/channels/categories, while owner-only mutations remain unchanged.
- Implemented invite OpenGraph fallback HTML for `/invite/{code}` while preserving SPA boot.
- Added invite deep-link onboarding and post-auth auto-join in `App.svelte`, including optional welcome gate rendering and error copy mapping.
- Added message composer focus-on-landing behavior and updated client/server tests for invite join lifecycle.
- Quality gates passed: client lint/check/test/build and server fmt/clippy/test.

### File List

- _bmad-output/implementation-artifacts/4-6-join-guild-via-invite-link.md
- client/src/App.svelte
- client/src/lib/features/chat/MessageArea.svelte
- client/src/lib/features/guild/guildApi.ts
- client/src/lib/features/guild/guildApi.test.ts
- client/src/lib/features/guild/types.ts
- client/src/lib/features/identity/CrossInstanceJoinPrompt.svelte
- client/src/lib/features/identity/LoginView.svelte
- client/src/lib/features/identity/LoginView.test.ts
- server/migrations/0014_create_guild_members.sql
- server/src/handlers/invites.rs
- server/src/handlers/mod.rs
- server/src/models/guild.rs
- server/src/models/guild_invite.rs
- server/src/models/guild_member.rs
- server/src/models/mod.rs
- server/src/services/category_service.rs
- server/src/services/channel_service.rs
- server/src/services/guild_invite_service.rs
- server/src/services/guild_service.rs
- server/src/static_files.rs
- server/tests/server_binds_to_configured_port.rs

## Senior Developer Review (AI)

### Reviewer

Darko (GPT-5.3-Codex) on 2026-02-28

### Outcome

Approve - no actionable HIGH/MEDIUM findings.

### Notes

- Acceptance Criteria 1-8 were cross-checked against the current implementation and tests.
- Git changes for source files align with the story File List.
- Quality gates passed:
  - `cd client && npm run lint && npm run check && npm run test && npm run build`
  - `cd server && cargo fmt --check && cargo clippy -- -D warnings && cargo test`

## Change Log

- 2026-02-28: Senior AI review completed (YOLO); no actionable findings; story marked `done`.
