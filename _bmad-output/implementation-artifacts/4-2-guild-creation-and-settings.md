# Story 4.2: Guild Creation and Settings

Status: review

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a **community builder**,
I want to create a guild with a name and icon,
so that I have a space for my community to gather.

## Acceptance Criteria

1. **Given** a user is authenticated  
   **When** they click the "+" button at the bottom of the GuildRail  
   **Then** a guild creation dialog appears with a name field (required) and optional icon upload

2. **Given** a valid guild creation form submission  
   **When** the request is sent  
   **Then** the guild is created on the server (guilds table migration in this story)

3. **Given** the guild is created  
   **When** persistence succeeds  
   **Then** the creator becomes the guild owner automatically

4. **Given** the guild is newly created  
   **When** initialization completes  
   **Then** a default `#general` text channel is created automatically

5. **Given** creation succeeds  
   **When** the client updates navigation state  
   **Then** the user lands in the new guild's `#general` channel

6. **Given** the user returns to the shell after creation  
   **When** the GuildRail renders  
   **Then** the new guild icon appears in the guild list

7. **Given** the authenticated user is the guild owner  
   **When** they open guild settings  
   **Then** they can edit guild name, icon, and description

8. **Given** the guild creation dialog is open  
   **When** the user interacts with the form  
   **Then** it follows UX patterns: single-column layout, fire CTA "Create Guild", and Enter key submit

## Tasks / Subtasks

- [x] Task 1: Add backend guild persistence and ownership foundation (AC: 2, 3, 4)
  - [x] Add a new SQL migration for `guilds` with owner linkage, display fields, icon metadata, and default channel slug.
  - [x] Keep schema naming/index conventions aligned with architecture (`snake_case`, plural tables, `idx_*` indexes).
  - [x] Persist automatic owner assignment on create and initialize default channel slug as `general`.

- [x] Task 2: Implement authenticated guild REST handlers and service layer (AC: 2, 3, 7)
  - [x] Add `handlers/guilds.rs`, `services/guild_service.rs`, and `models/guild.rs` wired through module `mod.rs` files.
  - [x] Expose at minimum create/list/update guild endpoints under `/api/v1/guilds` using `AuthenticatedUser`.
  - [x] Preserve API envelope contract (`{ "data": ... }`) and error envelope consistency with `AppError`.

- [x] Task 3: Add client guild API contracts and state wiring (AC: 2, 5, 6, 7)
  - [x] Add guild request/response wire types and `snake_case` ↔ `camelCase` mapping in the client API layer.
  - [x] Implement client functions for create/list/update guild and optional guild icon upload.
  - [x] Add a guild state holder in `features/guild` and replace hardcoded guild data in `GuildRail`.

- [x] Task 4: Implement GuildRail create flow UI (AC: 1, 8)
  - [x] Add a "+" trigger at the bottom of GuildRail that opens a creation dialog.
  - [x] Build a single-column dialog form with required name, optional icon, validation-on-blur, and fire primary CTA.
  - [x] Ensure Enter submits and errors render inline beneath invalid fields.

- [x] Task 5: Implement guild settings edit surface for owners (AC: 7)
  - [x] Add a guild settings panel/modal path from authenticated shell views.
  - [x] Support editing name, description, and icon with owner-only affordances.
  - [x] Reuse existing image validation/storage patterns used by avatar uploads where practical.

- [x] Task 6: Complete post-create navigation and shell rendering integration (AC: 4, 5, 6)
  - [x] After successful creation, navigate to `/{guild_slug}/general` and keep route persistence behavior intact.
  - [x] Ensure `ChannelList` resolves a default `general` channel path for newly created guilds before channel CRUD story work.
  - [x] Render guild icon/initial fallback in GuildRail consistently across desktop/tablet/mobile shell modes.

- [x] Task 7: Cover behavior with server + client tests and quality gates (AC: all)
  - [x] Add server tests for guild creation, owner assignment, and guild settings update authorization.
  - [x] Add client tests for create-dialog UX behavior, submit flow, and GuildRail state updates.
  - [x] Run quality gates: `cd client && npm run lint && npm run check && npm run test` and `cd server && cargo fmt --check && cargo clippy -- -D warnings && cargo test`.

## Dev Notes

### Developer Context

- Story 4.1 already shipped router-backed shell navigation (`/:guild/:channel`) and currently uses hardcoded guild/channel placeholder data in `GuildRail.svelte` and `ChannelList.svelte`.
- The backend currently exposes auth/users/admin/instance endpoints only; guild handlers/services/models are not yet implemented in runtime code.
- Keep this story scoped to guild creation + settings; full channel CRUD belongs to Story 4.3.

### Technical Requirements

- Require authentication for all guild mutation endpoints using `middleware::auth::AuthenticatedUser`.
- Keep DB compatibility across SQLite + PostgreSQL by following existing query branching patterns in services/models.
- Keep wire format `snake_case` and API envelope `{ "data": ... }`, with client mapping to `camelCase` in API helpers.
- Guild create flow must support optional icon upload while keeping creation successful without an icon.
- Enforce safe image handling for guild icons (MIME validation, size limits, and controlled storage keys), reusing avatar-service patterns where possible.

### Architecture Compliance

1. Add guild domain code to architecture-aligned paths:
   - Server: `handlers/guilds.rs`, `services/guild_service.rs`, `models/guild.rs`
   - Client: `features/guild/` plus API helpers/types
2. Keep REST resource naming under `/api/v1/guilds` and use `snake_case` route params and payload fields.
3. Preserve existing separation of concerns:
   - handlers = HTTP boundary
   - services = business logic
   - models = DB access
4. Do not introduce SvelteKit/server-side frontend concerns; keep SPA + Rust Axum split intact.

### Library & Framework Requirements

- Backend remains on Axum 0.8 + sqlx 0.8 with dual Postgres/SQLite support.
- Frontend remains on Svelte 5 runes + Vite + `@mateothegreat/svelte5-router`.
- Keep existing Tailwind v4 + CSS token design language (`fire` for primary actions, `ice` for navigation/selection).

### File Structure Requirements

Expected primary touch points:

- `server/migrations/0010_create_guilds.sql` (new)
- `server/src/handlers/mod.rs`
- `server/src/handlers/guilds.rs` (new)
- `server/src/services/mod.rs`
- `server/src/services/guild_service.rs` (new)
- `server/src/models/mod.rs`
- `server/src/models/guild.rs` (new)
- `server/tests/server_binds_to_configured_port.rs` and/or new guild-focused integration test file
- `client/src/lib/api.ts` and/or `client/src/lib/features/guild/guildApi.ts` (new)
- `client/src/lib/features/guild/GuildRail.svelte`
- `client/src/lib/features/channel/ChannelList.svelte`
- `client/src/lib/features/chat/MessageArea.svelte` (settings entry point if needed)
- `client/src/lib/features/guild/GuildCreate.svelte` (new)
- `client/src/lib/features/guild/GuildSettings.svelte` (new)
- `client/src/lib/features/**/**/*.test.ts`

### Testing Requirements

- Server:
  - Validate authenticated guild creation success and unauthenticated rejection.
  - Validate owner assignment and guild settings update authorization (owner vs non-owner).
  - Validate default `general` channel initialization behavior exposed by API response/state.
- Client:
  - Validate "+" opens create dialog, required-name validation, Enter submit behavior, and fire CTA labeling.
  - Validate successful create navigates to `/{guild}/general` and updates GuildRail.
  - Validate settings edits refresh rendered guild name/icon/description.

### Previous Story Intelligence

- Story 4.1 established shell route persistence and restored-location behavior (`saveLastLocation` / `getLastLocation`) that must remain intact.
- Story 4.1 delivered responsive shell behavior across desktop/tablet/mobile; guild creation/settings entry points must work in each shell mode.
- Story 4.1 test patterns (`ShellRoute.test.ts`, route tests, client quality gates) should be followed for new client behavior coverage.

### Latest Technical Information

1. Server runtime already pins `axum = 0.8` with multipart enabled and `sqlx = 0.8` with both `postgres` and `sqlite` features enabled.
2. Client runtime already pins `@mateothegreat/svelte5-router = 2.16.19` and Svelte 5 (`^5.45.2`), so new guild flows should integrate with existing router APIs.
3. Existing avatar upload pipeline already enforces MIME sniffing and upload-size limits; guild icon upload should mirror those safeguards.

### Project Context Reference

- No `project-context.md` was discovered via `**/project-context.md`.
- Story context is derived from epics, PRD, architecture, UX design artifacts, and current client/server source.

### Story Completion Status

- Ultimate context engine analysis completed — comprehensive developer guide created.
- Story implementation completed and moved to `review`.

### Project Structure Notes

- This story intentionally introduces the first guild-domain runtime path while preserving current shell/routing foundations from Story 4.1.
- Channel CRUD and invite lifecycle details remain in subsequent Epic 4 stories; avoid prematurely implementing Story 4.3+ scope.

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Epic 4: Guilds, Channels & Invites]
- [Source: _bmad-output/planning-artifacts/epics.md#Story 4.2: Guild Creation and Settings]
- [Source: _bmad-output/planning-artifacts/prd.md#Guild Management]
- [Source: _bmad-output/planning-artifacts/prd.md#Non-Functional Requirements]
- [Source: _bmad-output/planning-artifacts/architecture.md#Implementation Patterns & Consistency Rules]
- [Source: _bmad-output/planning-artifacts/architecture.md#Requirements to Structure Mapping]
- [Source: _bmad-output/planning-artifacts/architecture.md#API & Communication Patterns]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Journey 2: Guild Setup (Maya)]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Form Patterns]
- [Source: client/src/lib/features/guild/GuildRail.svelte]
- [Source: client/src/lib/features/channel/ChannelList.svelte]
- [Source: client/src/lib/features/shell/ShellRoute.svelte]
- [Source: client/src/routes/routes.ts]
- [Source: client/src/lib/api.ts]
- [Source: server/src/handlers/mod.rs]
- [Source: server/src/middleware/auth.rs]
- [Source: server/src/services/user_profile_service.rs]
- [Source: server/Cargo.toml]

## Dev Agent Record

### Agent Model Used

GPT-5.3-Codex (model ID: gpt-5.3-codex)

### Debug Log References

- `cd client && npm run lint && npm run check && npm run test`
- `cd server && cargo fmt --check && cargo clippy -- -D warnings && cargo test`

### Completion Notes List

- Added guild persistence runtime (`guilds` migration + model/service/handlers/routes) with owner assignment and default `general` channel slug.
- Added owner-authorized guild settings updates (name/description/icon) and safe guild icon upload handling with MIME sniffing and size limits.
- Wired client guild API + state store into shell guild rail/channel list, including create flow and post-create navigation to `/{guild}/general`.
- Added guild settings entry point in shell header and modal integration for owner edits.
- Added client tests for GuildRail create UX/submit navigation and GuildSettings owner editing behavior.
- Verified full client and server quality gates pass.

### File List

- _bmad-output/implementation-artifacts/4-2-guild-creation-and-settings.md
- _bmad-output/implementation-artifacts/sprint-status.yaml
- client/src/App.svelte
- client/src/lib/features/channel/ChannelList.svelte
- client/src/lib/features/guild/GuildRail.svelte
- client/src/lib/features/guild/GuildSettings.svelte
- client/src/lib/features/guild/GuildRail.test.ts
- client/src/lib/features/guild/GuildSettings.test.ts
- client/src/lib/features/guild/guildApi.ts
- client/src/lib/features/guild/guildStore.svelte.ts
- client/src/lib/features/guild/types.ts
- client/src/lib/features/shell/ShellRoute.svelte
- server/migrations/0010_create_guilds.sql
- server/src/handlers/guilds.rs
- server/src/handlers/mod.rs
- server/src/models/guild.rs
- server/src/models/mod.rs
- server/src/services/guild_service.rs
- server/src/services/mod.rs
- server/tests/server_binds_to_configured_port.rs

## Change Log

- 2026-02-28: Story created and marked ready-for-dev with implementation context for guild creation and settings.
- 2026-02-28: Implemented guild creation/settings backend + frontend flows, added guild client tests, validated full client/server quality gates, and moved story to review.
