# Story 8.1: Mute User (Timed and Permanent)

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a **moderator**,
I want to mute a user in the guild for a specified duration,
so that I can temporarily restrict disruptive users from sending messages.

## Acceptance Criteria

1. **Given** a user has MUTE_MEMBERS permission  
   **When** they right-click a member -> Mute  
   **Then** a dialog appears with duration options: 1 hour, 24 hours, 1 week, custom, permanent.

2. **Given** the mute dialog is open  
   **When** the moderator enters mute details  
   **Then** reason is required and cannot be empty.

3. **Given** a moderator confirms mute  
   **When** the operation succeeds  
   **Then** the target user cannot send messages in any guild text channel.

4. **Given** a muted user opens the app  
   **When** mute is active  
   **Then** they see a clear muted indicator including expiration time (or permanent status).

5. **Given** a timed mute is active  
   **When** mute expires  
   **Then** send permissions are restored automatically without manual moderator action.

6. **Given** mute is created  
   **When** persistence is written  
   **Then** mute state is stored in the database in this story's migration.

7. **Given** mute succeeds  
   **When** moderator returns to the member list  
   **Then** toast confirms action (example: "User muted for 24 hours").

8. **Given** a moderator attempts to mute another member  
   **When** role hierarchy is evaluated  
   **Then** target must have lower highest role than the moderator.

9. **Given** mute action is executed  
   **When** audit data is recorded  
   **Then** action is persisted in a format consumable by Story 8.5 moderation log.

## Tasks / Subtasks

- [x] Task 1: Add moderation persistence for mute lifecycle (AC: 5, 6, 9)
  - [x] Add migration `server/migrations/0026_create_moderation_actions.sql` with a moderation actions table that supports mute-only now (`action_type='mute'`) and future Story 8.x actions.
  - [x] Include columns for `guild_id`, `actor_user_id`, `target_user_id`, `reason`, `duration_seconds` (nullable for permanent), `expires_at` (nullable for permanent), `is_active`, `created_at`, `updated_at`.
  - [x] Add indexes for active mute lookups by guild/target and expiration sweeps (`is_active`, `expires_at`).
  - [x] Add `server/src/models/moderation.rs` and export from `server/src/models/mod.rs` with PG/SQLite query parity.

- [x] Task 2: Implement server moderation mute workflow (AC: 2, 3, 5, 8, 9)
  - [x] Add `server/src/services/moderation_service.rs` and export in `server/src/services/mod.rs`.
  - [x] Implement `create_mute` service flow: validate input, require `MUTE_MEMBERS`, verify actor outranks target via `permissions::actor_outranks_target_member`, ensure actor/target are guild members, prevent self-mute, and persist mute action.
  - [x] Reuse existing RFC3339 time handling patterns (`chrono::Utc`, `DateTime::parse_from_rfc3339`) used in auth/recovery services.
  - [x] Add automatic expiration behavior by treating expired mutes as inactive during read/enforcement checks and updating persistence accordingly.
  - [x] Persist moderation reason and actor/target metadata needed by Story 8.5 mod log.

- [x] Task 3: Add moderation API endpoints and route wiring (AC: 1, 2, 6, 7)
  - [x] Add `server/src/handlers/moderation.rs` and register routes in `server/src/handlers/mod.rs`.
  - [x] Implement `POST /api/v1/guilds/{guild_slug}/moderation/mutes` for creating mute from moderator action.
  - [x] Implement `GET /api/v1/guilds/{guild_slug}/moderation/me/mute-status` for muted-user UX status (active/permanent/expiration/reason visibility rules).
  - [x] Keep response envelope contract `{ "data": ... }` and `snake_case` JSON fields.

- [x] Task 4: Enforce mute in real-time messaging paths (AC: 3, 4, 5)
  - [x] Update message send paths (`message_service::create_message`, `create_attachment_message`) to reject muted users with explicit user-facing error text.
  - [x] Ensure typing start operations are blocked for muted users to prevent "typing while muted" mismatch.
  - [x] Keep authorization boundaries unchanged: permission checks remain server-authoritative.

- [x] Task 5: Replace member-list placeholder with real mute interaction (AC: 1, 2, 7, 8)
  - [x] Add moderation API client in `client/src/lib/features/moderation/moderationApi.ts`.
  - [x] Update `client/src/lib/features/members/MemberList.svelte` to replace "Mute member (coming soon)" with a real mute dialog.
  - [x] Dialog must include preset durations (1h/24h/1w), custom duration input, permanent option, and required reason field.
  - [x] Show success toast via `toastState.show({ variant: 'success', message: ... })` with selected duration wording.
  - [x] Keep keyboard/context-menu behavior compatible with current Shift+F10 / ContextMenu support.

- [x] Task 6: Surface muted-user state in composer UX (AC: 3, 4, 5)
  - [x] Add mute status store/module (e.g., `client/src/lib/features/moderation/muteStatusStore.svelte.ts`) keyed by guild.
  - [x] Update `client/src/lib/features/chat/MessageArea.svelte` to show a clear muted banner and disable send/attach actions while mute is active.
  - [x] Show expiration timestamp for timed mute and explicit permanent label for permanent mute.
  - [x] Ensure status refresh handles automatic unmute (time expiry) without manual page reload.

- [x] Task 7: Add regression coverage for moderation mute flow (AC: all)
  - [x] Server unit tests in `server/src/services/moderation_service.rs` for permission guardrails, role hierarchy, timed/permanent mute creation, and expiry handling.
  - [x] Server handler tests for validation and API response envelope shape.
  - [x] Client component tests in `MemberList.test.ts` for mute dialog validation and action dispatch.
  - [x] Client tests in `MessageArea.test.ts` for muted-state composer disablement and user-visible mute status.
  - [x] Run quality gates:
    - [x] `cd client && npm run lint && npm run check && npm run test && npm run build`
    - [x] `cd server && cargo fmt --check && cargo clippy -- -D warnings && cargo test`

## Dev Notes

### Developer Context

- Epic 8 moderation actions are designed as right-click context menu actions with required reason capture and append-only audit trail.
- `MemberList.svelte` already exposes moderation permissions and context-menu entry points, but currently renders mute/kick/ban as "coming soon".
- Permission bitflags already include `MUTE_MEMBERS`, `KICK_MEMBERS`, `BAN_MEMBERS`, `MANAGE_MESSAGES`, and `VIEW_MOD_LOG` on both server and client catalogs.
- Messaging writes currently flow through WebSocket ops into `message_service`; ownership-only delete rules are enforced server-side, so mute enforcement must also live server-side.
- No moderation persistence/model/handler modules exist yet in runtime code; this story establishes the first moderation backend slice.

### Technical Requirements

- Use `permissions::require_guild_permission(..., MUTE_MEMBERS, "MUTE_MEMBERS")` for authorization.
- Enforce hierarchy with `permissions::actor_outranks_target_member` and deny equal/higher role targets.
- Mute reason is required (trimmed non-empty); reject oversized inputs consistently with existing validation patterns.
- Duration handling:
  - Presets: `1h`, `24h`, `1w`
  - Custom: validated positive duration
  - Permanent: `expires_at = null`, `duration_seconds = null`
- Automatic unmute must be deterministic:
  - On read/enforcement, if `expires_at <= now`, mark action inactive and treat user as unmuted.
  - Do not require background worker to satisfy AC5.
- Muted users cannot send text messages or attachment-backed messages in guild channels.
- Muted users must see explicit mute state and expiration in channel composer area.
- Preserve API boundary conventions: `snake_case` wire payloads, `{ "data": ... }` envelopes, and explicit errors (no silent fallbacks).

### Architecture Compliance

1. Respect layer boundaries: handlers parse/serialize only; services own moderation logic; models own SQL.
2. Reuse existing role/permission engine and avoid duplicating authority logic in handlers.
3. Keep PG/SQLite parity in models and migrations (same behavior, different placeholder syntax only).
4. Maintain current WebSocket architecture and only extend protocol if absolutely necessary for mute UX.
5. Keep moderation persistence forward-compatible with Story 8.5 (mod log) without backtracking schema design.
6. Preserve security stance: server-authoritative enforcement, no client-only mute enforcement.

### Library & Framework Requirements

- Backend remains Rust + Axum 0.8 + Tokio + SQLx 0.8 from existing `server/Cargo.toml`.
- Frontend remains Svelte 5 + TypeScript + `@mateothegreat/svelte5-router` + existing toast store.
- Do not add new scheduler libraries for timed mute expiry; use existing `chrono` + runtime time handling patterns.
- Keep context-menu accessibility aligned with ARIA menu interaction guidance (keyboard operability and semantic roles).

### File Structure Requirements

Expected primary touch points:

- `server/migrations/0026_create_moderation_actions.sql` (new)
- `server/src/models/mod.rs` (export new module)
- `server/src/models/moderation.rs` (new)
- `server/src/services/mod.rs` (export new module)
- `server/src/services/moderation_service.rs` (new)
- `server/src/handlers/mod.rs` (route registration)
- `server/src/handlers/moderation.rs` (new)
- `server/src/services/message_service.rs` (mute enforcement in send paths)
- `server/src/ws/gateway.rs` (typing/send guard integration if required)
- `client/src/lib/features/moderation/moderationApi.ts` (new)
- `client/src/lib/features/moderation/muteStatusStore.svelte.ts` (new)
- `client/src/lib/features/members/MemberList.svelte`
- `client/src/lib/features/members/MemberList.test.ts`
- `client/src/lib/features/chat/MessageArea.svelte`
- `client/src/lib/features/chat/MessageArea.test.ts`

### Testing Requirements

- Server:
  - Validate permission and role hierarchy guardrails for mute actions.
  - Validate timed mute expiry behavior and permanent mute behavior.
  - Validate mute status endpoint output shape and authorization.
  - Validate muted users are blocked from message create and attachment create paths.
- Client:
  - Validate mute dialog enforces required reason and valid duration.
  - Validate successful mute flow dispatches API call and success toast text.
  - Validate muted composer state disables send controls and shows clear expiry/permanent messaging.
  - Validate unmute-after-expiry refresh returns composer to normal behavior.

### Latest Technical Information

- Axum current docs emphasize extractor-based handlers and Tower middleware composition; moderation routes should follow current handler/state patterns.  
  [Source: https://docs.rs/axum/latest/axum/]
- SQLx runtime/TLS behavior remains feature-driven; current project setup (`runtime-tokio`, rustls) already matches required async DB access for moderation persistence.  
  [Source: https://docs.rs/sqlx/latest/sqlx/]
- Tokio timing guidance indicates `sleep_until`/`interval` are suitable for coarse scheduling, but expiration checks can also be enforced lazily at read/write boundaries for simpler reliability.  
  [Source: https://docs.rs/tokio/latest/tokio/time/fn.sleep_until.html]
- ARIA menu guidance confirms context-menu keyboard patterns (ContextMenu key, Shift+F10, arrow navigation), which should be preserved while adding mute actions.  
  [Source: https://www.w3.org/WAI/ARIA/apg/patterns/menu/]

### Project Context Reference

- No `project-context.md` was discovered via `**/project-context.md`.
- Context for this story is derived from planning artifacts, architecture, UX spec, sprint status, and current runtime code.

### Story Completion Status

- Workflow analysis and implementation are complete for story `8-1-mute-user-timed-and-permanent`.
- Story implementation is complete and ready for review.
- Sprint status target for this story: `review`.
- Completion note: Story delivered with backend enforcement, client UX, and regression coverage.

### Project Structure Notes

- Existing member moderation UI entry point is in `features/members/MemberList.svelte`; this story should replace placeholder actions instead of adding parallel moderation UI paths.
- Existing channel composer behavior and send pipeline are in `features/chat/MessageArea.svelte` and `messageStore.svelte.ts`; muted-state UX should integrate there.
- Server currently lacks moderation modules (`handlers/moderation.rs`, `services/moderation_service.rs`, `models/moderation.rs`) even though architecture reserves those boundaries.

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Epic 8: Moderation, Reporting & Data Privacy]
- [Source: _bmad-output/planning-artifacts/epics.md#Story 8.1: Mute User (Timed and Permanent)]
- [Source: _bmad-output/planning-artifacts/prd.md#Moderation & Safety]
- [Source: _bmad-output/planning-artifacts/prd.md#Domain-Specific Requirements]
- [Source: _bmad-output/planning-artifacts/architecture.md#Cross-Cutting Concerns Identified]
- [Source: _bmad-output/planning-artifacts/architecture.md#Requirements to Structure Mapping]
- [Source: _bmad-output/planning-artifacts/architecture.md#Naming Patterns]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Context Menu Patterns]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Journey 4 (moderation flow)]
- [Source: _bmad-output/implementation-artifacts/sprint-status.yaml]
- [Source: server/src/permissions/mod.rs]
- [Source: server/src/handlers/mod.rs]
- [Source: server/src/services/message_service.rs]
- [Source: server/src/services/role_service.rs]
- [Source: client/src/lib/features/members/MemberList.svelte]
- [Source: client/src/lib/features/chat/MessageArea.svelte]
- [Source: client/src/lib/features/guild/permissions.ts]
- [Source: client/src/lib/feedback/toastStore.svelte.ts]
- [Source: client/src/lib/feedback/userFacingError.ts]
- [Source: https://docs.rs/axum/latest/axum/]
- [Source: https://docs.rs/sqlx/latest/sqlx/]
- [Source: https://docs.rs/tokio/latest/tokio/time/fn.sleep_until.html]
- [Source: https://www.w3.org/WAI/ARIA/apg/patterns/menu/]

## Dev Agent Record

### Agent Model Used

GPT-5.3-Codex (model ID: gpt-5.3-codex)

### Debug Log References

- Story context generated from Epic 8, architecture constraints, UX context-menu patterns, and current codebase structure.
- No prior story intelligence section included because this is story `8.1`.

### Completion Notes List

- Added moderation persistence (`moderation_actions`) with model/service APIs for create, read-active, and deactivate flows.
- Added moderation handlers/routes for mute creation and self mute-status lookup using `{ "data": ... }` envelopes.
- Wired mute enforcement into message create/attachment create paths and typing-start websocket handling.
- Added member-list mute dialog (preset/custom/permanent with required reason), moderation API client, and success toast feedback.
- Added muted-composer UX with guild mute status store, banner messaging, disabled send/attach, and timed-status refresh.
- Added/updated regression coverage for moderation service, moderation handler envelope/validation, message enforcement, and client mute UX.
- Passed quality gates:
  - `cd client && npm run lint && npm run check && npm run test && npm run build`
  - `cd server && cargo fmt --check && cargo clippy -- -D warnings && cargo test`

### File List

- _bmad-output/implementation-artifacts/8-1-mute-user-timed-and-permanent.md
- client/src/lib/features/chat/MessageArea.svelte
- client/src/lib/features/chat/MessageArea.test.ts
- client/src/lib/features/members/MemberList.svelte
- client/src/lib/features/members/MemberList.test.ts
- client/src/lib/features/moderation/moderationApi.ts
- client/src/lib/features/moderation/muteStatusStore.svelte.ts
- _bmad-output/implementation-artifacts/sprint-status.yaml
- server/migrations/0026_create_moderation_actions.sql
- server/src/handlers/mod.rs
- server/src/handlers/moderation.rs
- server/src/models/mod.rs
- server/src/models/moderation.rs
- server/src/services/message_service.rs
- server/src/services/mod.rs
- server/src/services/moderation_service.rs
- server/src/ws/gateway.rs

## Senior Developer Review (AI)

### Outcome

Approved after fixes.

### Findings and Resolutions

- **[MEDIUM][Fixed]** Mute duration options in `MemberList.svelte` included an extra `10 minutes` preset that was not part of AC1 (`1 hour`, `24 hours`, `1 week`, `custom`, `permanent`).  
  **Fix:** Removed the `10m` preset from `MuteDurationPreset`, removed the `10 minutes` UI option, and removed the corresponding duration conversion branch.

- **[MEDIUM][Fixed]** Mute status refresh scheduling in `MessageArea.svelte` could exceed JavaScript timer limits for far-future expirations, causing timeout overflow warnings and immediate refresh loops.  
  **Fix:** Capped refresh delay to `2_147_483_647ms` when scheduling mute-expiry refresh.

### Verification

- `cd client && npm run lint && npm run check && npm run test -- MemberList.test.ts`
- `cd client && npm run lint && npm run check && npm run test -- MessageArea.test.ts`

## Change Log

- 2026-03-02: Senior AI code review found and fixed mute refresh timer overflow risk in `MessageArea.svelte` by capping timeout delay to JS max timeout.
- 2026-03-02: Senior AI code review completed in YOLO mode; found and fixed one AC1-alignment issue (removed unsupported 10-minute mute preset), then re-ran frontend lint/check/tests.
- 2026-03-02: Story status updated to `done` and sprint status synced for `8-1-mute-user-timed-and-permanent`.
