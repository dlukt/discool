# Story 8.5: Moderation Log

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a **moderator**,
I want all moderation actions logged in an auditable mod log,
so that the moderation team has accountability and history.

## Acceptance Criteria

1. **Given** any moderation action is performed (`mute`, `kick`, `ban`, `voice_kick`, `message_delete`, `warn`)  
   **When** the action completes  
   **Then** a mod log entry is created with timestamp, moderator identity, action type, target user, and reason text.

2. **Given** mod log entries exist  
   **When** moderators view the log  
   **Then** it is append-only and entries cannot be edited or deleted.

3. **Given** a user has `VIEW_MOD_LOG` permission  
   **When** they open moderation tools for a guild  
   **Then** they can view the moderation log in a dedicated guild panel.

4. **Given** a mod log entry is rendered  
   **When** it appears in the UI  
   **Then** it shows timestamp, moderator name/avatar, action badge, target user, and reason.

5. **Given** moderators need to inspect specific actions  
   **When** they use log controls  
   **Then** entries can be sorted by date and filtered by action type.

6. **Given** a guild has a large moderation history  
   **When** moderators scroll the log  
   **Then** the view uses the same virtualized scroll-area approach as other large lists in the app.

## Tasks / Subtasks

- [x] Task 1: Extend moderation action schema for comprehensive log coverage (AC: 1, 2, 5)
  - [x] Add migration `server/migrations/0030_expand_moderation_actions_for_log_view.sql` to:
    - [x] expand `moderation_actions` action-type constraint to include future-safe values used by Epic 8 (`message_delete`, `warn`) in addition to existing (`mute`, `kick`, `ban`, `voice_kick`)
    - [x] add index(es) optimized for log reads: guild + action type + created timestamp ordering
    - [x] preserve append-only invariants and existing constraints (`actor != target`, duration/expiry pairing, active flag domain)
  - [x] Keep PostgreSQL/SQLite parity using the established rebuild-table migration pattern from `0027`/`0028`/`0029`.

- [x] Task 2: Implement moderation log read model with cursor pagination and filtering (AC: 1, 4, 5)
  - [x] Extend `server/src/models/moderation.rs` with list/query structs for moderation log rows joined with actor/target user profile fields needed by UI.
  - [x] Add DB query function(s) for:
    - [x] guild-scoped list
    - [x] optional action-type filter
    - [x] stable sort by timestamp + id
    - [x] cursor-based pagination for large datasets
  - [x] Ensure query path is strictly read-only; no update/delete mutations for log entries.

- [x] Task 3: Add service-layer moderation log endpoint logic and permission guardrails (AC: 1, 2, 3, 5)
  - [x] Add typed input/result models in `server/src/services/moderation_service.rs` (list input, log entry response, paged result).
  - [x] Require `VIEW_MOD_LOG` via `permissions::require_guild_permission(..., VIEW_MOD_LOG, "VIEW_MOD_LOG")` (owner access remains valid).
  - [x] Validate query inputs (`limit`, `cursor`, `order`, `action_type`) with explicit validation errors (no silent coercion beyond established clamping conventions).
  - [x] Reuse cursor patterns from message-history services for consistency.

- [x] Task 4: Expose moderation log HTTP API and wire routes (AC: 3, 5)
  - [x] Add `GET /api/v1/guilds/{guild_slug}/moderation/log` in `server/src/handlers/moderation.rs`.
  - [x] Register route in `server/src/handlers/mod.rs`.
  - [x] Keep API contract conventions:
    - [x] `snake_case` response fields
    - [x] response envelope `{ "data": [...], "cursor": ... }`
    - [x] explicit `ValidationError`/`Forbidden` behavior.

- [x] Task 5: Add client API support for moderation log retrieval (AC: 3, 5)
  - [x] Extend `client/src/lib/features/moderation/moderationApi.ts` with:
    - [x] moderation log entry types
    - [x] list input type (`limit`, `cursor`, `order`, `actionType`)
    - [x] `fetchModerationLog(...)` using `apiFetchCursorList`
  - [x] Add/extend tests in `client/src/lib/features/moderation/moderationApi.test.ts` for query mapping and response conversion.

- [x] Task 6: Build dedicated mod-log UI panel with virtualized rendering (AC: 3, 4, 5, 6)
  - [x] Add `client/src/lib/features/moderation/ModLogPanel.svelte` and `ModLogEntry.svelte` (or equivalent feature-local split).
  - [x] Mount panel in existing guild UX flow (member/moderation sidebar context) and gate access by `VIEW_MOD_LOG`.
  - [x] Render each entry with:
    - [x] timestamp
    - [x] moderator identity (name + avatar)
    - [x] target user
    - [x] reason
    - [x] action badge with required colors (mute=ice blue, kick=fire orange, ban=fire red; include readable defaults for additional actions).
  - [x] Provide action-type filter and date sort controls.
  - [x] Provide empty state copy: `"No moderation actions yet. That's a good thing."`
  - [x] Use existing virtualized-list patterns from `MessageArea.svelte` / `MemberList.svelte` for performance.

- [x] Task 7: Centralize moderation action logging to prevent drift in future stories (AC: 1, 2)
  - [x] Refactor duplicated moderation action insertion paths into shared helper(s) where practical, without changing current behavior.
  - [x] Confirm current actions (`mute`, `kick`, `ban`, `voice_kick`) all emit log records with required fields.
  - [x] Add explicit TODO hooks or typed extension points for upcoming `message_delete` (Story 8.6) and `warn` (report lifecycle stories) integration.

- [x] Task 8: Add regression coverage and run quality gates (AC: all)
  - [x] Server model/service/handler tests for:
    - [x] permission checks for `VIEW_MOD_LOG`
    - [x] pagination cursor behavior
    - [x] filtering and ordering behavior
    - [x] append-only enforcement (no mutation endpoints)
  - [x] Integration test additions in `server/tests/server_binds_to_configured_port.rs` validating end-to-end log retrieval after real moderation actions.
  - [x] Client tests for panel visibility gating, entry rendering, sort/filter behavior, and virtualized list behavior.
  - [x] Run quality gates:
    - [x] `cd client && npm run lint && npm run check && npm run test && npm run build`
    - [x] `cd server && cargo fmt --check && cargo clippy -- -D warnings && cargo test`

## Dev Notes

### Developer Context

- Existing moderation actions are already persisted in `moderation_actions` by Stories 8.1-8.4 (`mute`, `kick`, `ban`, `voice_kick`).
- `VIEW_MOD_LOG` permission bit already exists in server and client catalogs but is not yet enforced in moderation service/handlers.
- Current guild right sidebar is `MemberList` (desktop/tablet) via `ShellRoute`; this is the natural surface for a dedicated moderation-log panel.
- Existing moderation APIs and UI flow patterns live in:
  - `server/src/handlers/moderation.rs`
  - `server/src/services/moderation_service.rs`
  - `server/src/models/moderation.rs`
  - `client/src/lib/features/moderation/moderationApi.ts`
  - `client/src/lib/features/members/MemberList.svelte`

### Technical Requirements

- Authorization:
  - Require `VIEW_MOD_LOG` for mod-log reads.
  - Keep owner behavior consistent with existing permission system.
- Data model:
  - Log rows must include actor, target, action_type, reason, created timestamp.
  - Keep log append-only (no edit/delete semantics).
  - Preserve compatibility with existing action producers and upcoming moderation actions.
- API behavior:
  - Maintain `snake_case` wire format and `{ "data": ... }` response envelopes.
  - Support cursor pagination and action-type filtering for large logs.
  - Support date ordering controls required by AC (server and/or cursor-safe client strategy).
- UX:
  - Dedicated moderator-accessible panel inside guild context.
  - Action badges must be color-coded and text-labeled for accessibility.
  - Empty-state and error-state copy should remain clear and non-cheesy per UX spec.
- Performance:
  - Virtualized rendering required for high-entry logs.
  - Add indexes aligned with query shape to avoid full scans on large guilds.

### Architecture Compliance

1. Keep layer boundaries strict: handlers parse/serialize, services apply business rules, models own SQL.
2. Follow existing pagination conventions (`limit` + cursor) used by messages/DMs.
3. Keep Postgres/SQLite parity for all schema and query changes.
4. Preserve explicit error handling (`AppError`) and avoid hidden fallbacks.
5. Keep mod log immutable: no endpoint or helper should mutate historical entries.
6. Reuse existing moderation and permission helpers instead of introducing parallel auth logic.

### Library & Framework Requirements

- Backend: Rust + Axum + SQLx + Tokio (existing stack only).
- Frontend: Svelte 5 + existing feature/store pattern + existing API utilities.
- UI primitives: use existing design tokens/components and existing virtualized list approach from `MessageArea` / `MemberList`.
- No new global state framework or alternate data-fetching library.

### File Structure Requirements

Expected primary touch points:

- `server/migrations/0030_expand_moderation_actions_for_log_view.sql` (new)
- `server/src/models/moderation.rs`
- `server/src/services/moderation_service.rs`
- `server/src/handlers/moderation.rs`
- `server/src/handlers/mod.rs`
- `server/tests/server_binds_to_configured_port.rs`
- `client/src/lib/features/moderation/moderationApi.ts`
- `client/src/lib/features/moderation/moderationApi.test.ts`
- `client/src/lib/features/moderation/ModLogPanel.svelte` (new)
- `client/src/lib/features/moderation/ModLogEntry.svelte` (new)
- `client/src/lib/features/members/MemberList.svelte`
- `client/src/lib/features/members/MemberList.test.ts`
- `client/src/lib/features/shell/ShellRoute.svelte` (if panel toggle/surface is added there)
- `client/src/lib/features/shell/ShellRoute.test.ts` (if ShellRoute wiring changes)

### Testing Requirements

- Server:
  - Only `VIEW_MOD_LOG` users can fetch log entries.
  - List endpoint returns required fields and stable ordering.
  - Cursor pagination works across page boundaries without duplicates/skips.
  - Action-type filters return only requested categories.
  - Existing moderation action creation flows still append rows correctly.
- Client:
  - Panel visibility and access are permission-gated.
  - Entry rendering includes all required fields and badge labels/colors.
  - Sort/filter controls affect rendered list correctly.
  - Virtualization renders efficiently for long logs.
  - Empty-state copy matches UX spec.

### Previous Story Intelligence

- Story 8.4 introduced `voice_kick` as a first-class moderation action persisted in `moderation_actions`; mod-log views must include it from day one.
- Stories 8.2 and 8.3 hardened transactional moderation mutation patterns in `models/moderation.rs`; keep this consistency when refactoring log insertion helpers.
- Stories 8.1-8.4 standardized required reason text for moderation actions, so mod-log entries can rely on non-empty reasons.
- Existing moderation UX patterns (confirmation dialog + explicit toasts + keyboard accessibility) are already established and should be preserved.

### Git Intelligence Summary

- Recent commits show moderation development cadence and surfaces to extend:
  - `90a3b7f` feat: complete story 8-4 voice kick moderation flow
  - `15e3cc8` feat: complete story 8.3 ban moderation flow
  - `ee59039` fix: harden kick transaction review
  - `a272645` feat: implement story 8.2 kick moderation flow
  - `8966e28` feat: complete story 8-1 mute moderation
- Practical implication: build 8.5 as an extension of established moderation architecture, not a greenfield subsystem.

### Latest Technical Information

- Axum continues to emphasize extractor-based handlers and Tower middleware composition, which matches current `handlers/moderation.rs` conventions.  
  [Source: https://docs.rs/axum/latest/axum/]
- SQLx async APIs require runtime features; keep existing runtime/TLS setup and DB parity practices in moderation query implementations.  
  [Source: https://docs.rs/sqlx/latest/sqlx/]
- The `webrtc` crate family currently publishes `0.17.1` modules; voice moderation events remain aligned with current runtime behavior and should keep compatibility with existing voice-kick flow.  
  [Source: https://docs.rs/webrtc/latest/webrtc/]
- Svelte compile-time component model remains the expected frontend implementation path; mod-log UI should follow existing component/store patterns.  
  [Source: https://svelte.dev/docs/svelte/overview]

### Project Context Reference

- No `project-context.md` discovered via `**/project-context.md`.
- Story context derived from planning artifacts, existing Epic 8 implementation files, and current runtime code.

### Story Completion Status

- Story implementation completed at `_bmad-output/implementation-artifacts/8-5-moderation-log.md`.
- Story status set to `review`.
- Sprint status target for this story: `review`.
- Completion note: Moderation log backend, API, UI panel, and regression coverage were implemented and validated with full quality gates.

### Project Structure Notes

- Keep mod-log backend logic in existing moderation modules (handler/service/model) rather than adding a parallel subsystem.
- Prefer sidebar-integrated dedicated panel so moderator workflows stay in guild context.
- Reuse existing virtualized list patterns (`MessageArea`, `MemberList`) to minimize performance regressions and duplicate logic.
- Ensure no coupling of read-only mod-log views to mutation side effects.

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Epic 8: Moderation, Reporting & Data Privacy]
- [Source: _bmad-output/planning-artifacts/epics.md#Story 8.5: Moderation Log]
- [Source: _bmad-output/planning-artifacts/prd.md#Moderation & Safety]
- [Source: _bmad-output/planning-artifacts/prd.md#FR51]
- [Source: _bmad-output/planning-artifacts/architecture.md#Cross-Cutting Concerns Identified]
- [Source: _bmad-output/planning-artifacts/architecture.md#FR Category: Moderation & Safety]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Journey 4: Moderation Workflow]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#ModLogEntry]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Empty state scenarios]
- [Source: _bmad-output/implementation-artifacts/sprint-status.yaml]
- [Source: _bmad-output/implementation-artifacts/8-4-kick-user-from-voice-channel.md]
- [Source: server/src/permissions/mod.rs]
- [Source: server/src/models/moderation.rs]
- [Source: server/src/services/moderation_service.rs]
- [Source: server/src/handlers/moderation.rs]
- [Source: server/src/handlers/mod.rs]
- [Source: client/src/lib/features/moderation/moderationApi.ts]
- [Source: client/src/lib/features/members/MemberList.svelte]
- [Source: client/src/lib/features/shell/ShellRoute.svelte]
- [Source: client/src/lib/api.ts]

## Dev Agent Record

### Agent Model Used

GPT-5.3-Codex (model ID: gpt-5.3-codex)

### Debug Log References

- Story key resolved from sprint status: `8-5-moderation-log`.
- Full quality gates executed successfully:
  - `cd client && npm run lint && npm run check && npm run test && npm run build`
  - `cd server && cargo fmt --check && cargo clippy -- -D warnings && cargo test`
- Moderation log endpoint and UI behavior validated through service/handler/integration and client tests.

### Completion Notes List

- Added migration `0030_expand_moderation_actions_for_log_view.sql` with expanded moderation action types and log-read indexes.
- Implemented moderation log read path in model/service/handler layers with cursor pagination, filter/order controls, and `VIEW_MOD_LOG` authorization.
- Added client moderation log API types + mapping and a dedicated virtualized `ModLogPanel` integrated in `MemberList` with permission-gated tab access.
- Added and updated backend/client tests covering envelope shape, permission gating, filtering/sorting, pagination, and panel rendering behavior.

### File List

- server/migrations/0030_expand_moderation_actions_for_log_view.sql
- server/src/models/moderation.rs
- server/src/services/moderation_service.rs
- server/src/handlers/moderation.rs
- server/src/handlers/mod.rs
- server/tests/server_binds_to_configured_port.rs
- client/src/lib/features/moderation/moderationApi.ts
- client/src/lib/features/moderation/moderationApi.test.ts
- client/src/lib/features/moderation/ModLogPanel.svelte
- client/src/lib/features/moderation/ModLogEntry.svelte
- client/src/lib/features/moderation/ModLogPanel.test.ts
- client/src/lib/features/members/MemberList.svelte
- client/src/lib/features/members/MemberList.test.ts
- _bmad-output/implementation-artifacts/sprint-status.yaml
- _bmad-output/implementation-artifacts/8-5-moderation-log.md

## Senior Developer Review (AI)

### Reviewer

Darko

### Outcome

Approved — no HIGH or MEDIUM findings.

### Notes

- Story claims and changed source files are aligned for the 8.5 scope.
- Acceptance Criteria coverage verified for audit logging, append-only behavior, `VIEW_MOD_LOG` gating, sort/filter controls, and virtualized rendering.
- Quality gates re-run successfully:
  - `cd client && npm run lint && npm run check && npm run test && npm run build`
  - `cd server && cargo fmt --check && cargo clippy -- -D warnings && cargo test`

## Change Log

- 2026-03-02: Implemented Story 8.5 moderation log backend/frontend functionality, added regression coverage, and moved story to review.
- 2026-03-02: Senior code review (YOLO) completed with no actionable findings; story approved and moved to done.
