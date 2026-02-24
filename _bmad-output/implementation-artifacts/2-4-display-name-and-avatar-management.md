# Story 2.4: Display Name and Avatar Management

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a **user**,
I want to update my display name and avatar,
So that I can control how others see me across all guilds.

## Acceptance Criteria

1. **Given** a user is authenticated
   **When** they navigate to their user settings
   **Then** they can change their display name.

2. **Given** a user is authenticated
   **When** they open avatar settings
   **Then** they can upload a new avatar image or keep using a generated/avatar-color option
   **And** they get an immediate preview before saving.

3. **Given** a user saves profile changes
   **When** the REST API request succeeds
   **Then** the new profile data is persisted server-side
   **And** the current SPA session state updates immediately (no full reload)
   **And** all future surfaces that resolve user profile data read the updated display name/avatar values.

4. **Given** a user uploads an avatar image
   **When** the backend processes it
   **Then** image type and size are validated
   **And** unsupported payloads are rejected with 422 `VALIDATION_ERROR`
   **And** accepted images are stored through a dedicated server-side avatar storage path/service (not base64 in DB/localStorage).

5. **Given** the settings UI
   **When** the form renders
   **Then** it follows UX guidance: single-column layout, labels above inputs, validate-on-blur behavior, and a clear fire-style primary CTA for save.

## Tasks / Subtasks

- [x] Task 1: Extend user profile persistence for display name + avatar metadata (AC: #1, #3, #4)
  - [x] 1.1 Add a new migration (next sequential number) to support profile editing without breaking existing auth/session flows.
    - Add `display_name` to `users` (nullable, non-unique; fallback to `username` when absent).
    - Add avatar storage metadata needed for retrieval (for example: storage key/path + MIME + size + updated timestamp), either in `users` or a dedicated avatar table keyed by `user_id`.
  - [x] 1.2 Update `server/src/models/user.rs` and response mapping so profile responses can include display name and avatar representation while preserving existing fields used by auth/session.
  - [x] 1.3 Keep migration SQL compatible with both PostgreSQL and SQLite.

- [x] Task 2: Implement backend profile/avatar service logic with strict validation (AC: #1, #2, #3, #4)
  - [x] 2.1 Add a profile service module (for example `server/src/services/user_profile_service.rs`) for:
    - Fetch current authenticated user profile.
    - Update display name with server-side validation.
    - Save avatar metadata after validated upload.
  - [x] 2.2 Add avatar validation rules:
    - Allowed MIME set (PNG/JPEG/WEBP).
    - Configurable max size (default 2 MB).
    - Prefer content-sniffing/magic-byte guard in addition to declared content type.
  - [x] 2.3 Add safe storage behavior:
    - Generate server-owned file names (UUID/random), never trust user file names.
    - Store files under a dedicated avatar directory managed by config.
    - Replace old avatar file references safely when user updates avatar.

- [x] Task 3: Add authenticated REST endpoints for profile management (AC: #1, #2, #3, #4)
  - [x] 3.1 Create `server/src/handlers/users.rs` and register it in `server/src/handlers/mod.rs`.
  - [x] 3.2 Implement endpoints behind `AuthenticatedUser`:
    - `GET /api/v1/users/me/profile`
    - `PATCH /api/v1/users/me/profile` (display name + avatar selection mode/metadata)
    - `POST /api/v1/users/me/avatar` (multipart upload path)
  - [x] 3.3 Keep API contracts consistent:
    - JSON envelope `{ "data": ... }` / `{ "error": ... }`
    - `snake_case` wire fields
    - `AppError`-based 401/422/500 behavior
  - [x] 3.4 If multipart handling is introduced, enable required Axum feature(s) and keep request size limits explicit.

- [x] Task 4: Add/extend client API contract for profile settings (AC: #1, #2, #3, #4)
  - [x] 4.1 Add a dedicated client API module (or extend identity API cleanly) for profile read/update and avatar upload.
  - [x] 4.2 Add typed wire transforms in `client/src/lib/features/identity/types.ts` (snake_case <-> camelCase mapping aligned with existing API helpers).
  - [x] 4.3 Ensure session-aware auth header flow remains via `$lib/api.ts` token handling (no duplicated auth plumbing).

- [x] Task 5: Build profile settings UI with preview and validation-on-blur (AC: #1, #2, #5)
  - [x] 5.1 Create a settings component in `client/src/lib/features/identity/` (for example `ProfileSettingsView.svelte`) using Svelte 5 runes patterns.
  - [x] 5.2 Implement fields and interaction model:
    - Display name input (label above, blur validation).
    - Avatar mode section (reuse existing avatar color option pattern from `LoginView.svelte`).
    - Image file picker with immediate preview and clear/remove action.
  - [x] 5.3 Keep UX/style aligned with current app conventions:
    - Single-column form
    - Inline validation messages
    - Fire primary save CTA
    - Non-blocking feedback (toast/inline status)

- [x] Task 6: Wire settings into current app state and ensure immediate reflection (AC: #1, #3, #5)
  - [x] 6.1 Add a settings entry point in `client/src/App.svelte` (current scaffold has no dedicated settings surface yet).
  - [x] 6.2 On successful save, update reactive state in `identityState.session.user` and any rendered identity views immediately.
  - [x] 6.3 Preserve existing auth flows from Stories 2.2/2.3 (`identityCorrupted`, `identityNotRegistered`, cross-tab safety, unauthorized handler behavior).

- [x] Task 7: Add focused test coverage and manual verification checklist (AC: all)
  - [x] 7.1 Server tests:
    - Profile endpoint auth enforcement.
    - Display name validation cases (empty/length/characters as defined).
    - Avatar upload accept/reject matrix (MIME/type/size).
    - Persistence and retrieval of updated profile data.
  - [x] 7.2 Client tests (Vitest + Testing Library):
    - Settings form validation-on-blur.
    - Avatar preview lifecycle (select -> preview -> replace/remove).
    - Successful save updates UI state without reload.
    - API error handling surfaces clear messages.
  - [x] 7.3 Manual checks:
    - Authenticated user updates display name -> reflected in UI immediately.
    - Avatar upload (valid image) works end-to-end.
    - Invalid image type/oversized file returns actionable error.
    - Reopen app/session still shows updated profile data.

## Dev Notes

### Architecture Compliance

This story introduces the first explicit `/api/v1/users/*` profile surface in the real codebase (currently only `auth`, `instance`, and `admin` handlers are wired). Implementation should align with architecture requirements while staying minimally invasive to existing auth/session behavior:

1. **Keep auth model intact**: Challenge-response and session token handling from Stories 2.2/2.3 remain authoritative. Profile endpoints must use `AuthenticatedUser` extractor (`server/src/middleware/auth.rs`) instead of introducing parallel auth checks.
2. **Respect API envelope and error contract**: Continue `{"data": ...}` success and `{"error": ...}` failure shapes with `AppError` mapping.
3. **Use planned route surface**: Architecture already reserves `handlers/users.rs` and `/api/v1/users/*`; this story should realize that path instead of overloading `/auth/*`.
4. **Do not conflate identity handle and display name**: `username` remains identity/registration handle; introduce mutable `display_name` for user-facing profile presentation.

### Technical Requirements (Developer Guardrails)

- Preserve existing login/register/challenge/verify/logout behavior and test coverage.
- Avoid breaking `identityState` persistence mechanics from Story 2.3 (localStorage session restore, auth epoch race guard, storage event synchronization).
- Ensure profile update APIs are idempotent where practical.
- Keep response fields explicit and typed; no `any`/untyped payload assumptions in client code.
- Keep DB timestamp usage RFC 3339 strings from Rust (`Utc::now().to_rfc3339()`), matching current patterns.

### API Contracts (Proposed)

`GET /api/v1/users/me/profile`
```json
{
  "data": {
    "id": "uuid",
    "did_key": "did:key:z6Mk...",
    "username": "liam",
    "display_name": "Liam",
    "avatar_color": "#3B82F6",
    "avatar_url": "/api/v1/users/me/avatar"
  }
}
```

`PATCH /api/v1/users/me/profile`
```json
{
  "display_name": "Liam from Guild",
  "avatar_color": "#3B82F6"
}
```

`POST /api/v1/users/me/avatar` (multipart)
- single `avatar` part
- validated MIME + size
- returns updated profile payload in `{ "data": ... }`

`422 VALIDATION_ERROR` examples:
- invalid display name
- unsupported image type
- file exceeds configured size

### Library / Framework Requirements

- **Backend**: Axum 0.8 existing stack; enable multipart extraction only if required by endpoint design.
- **Frontend**: Svelte 5 runes (`$state`, `$derived`, callback props, `onclick` attributes).
- **Validation**: Keep server-side validation as source of truth even when client-side checks exist.
- **No new production runtime frameworks** unless strictly necessary.

### File Structure Requirements

Create/modify only architecture-aligned locations:

- Server:
  - `server/src/handlers/users.rs` (new)
  - `server/src/handlers/mod.rs` (register routes)
  - `server/src/services/user_profile_service.rs` (new)
  - `server/src/services/mod.rs` (export module)
  - `server/src/models/user.rs` (+ optional new profile/avatar model file if needed)
  - `server/migrations/` (new migration file)
  - `server/src/config/settings.rs` + `config.example.toml` if adding avatar storage config

- Client:
  - `client/src/lib/features/identity/` for settings UI + API helpers
  - `client/src/App.svelte` for navigation/state wiring
  - `client/src/lib/features/identity/identityStore.svelte.ts` for immediate reactive updates

### Testing Requirements

Run existing quality gates after implementation:

- `cd server && cargo fmt --check && cargo clippy -- -D warnings && cargo test`
- `cd client && npm run lint && npm run check`

Add/extend tests in existing test style:
- Rust handler/service tests in `server/src/**` + integration tests in `server/tests/`
- Vitest tests in `client/src/lib/features/identity/*.test.ts`

### Anti-Patterns to Avoid

- Do **NOT** mutate `did_key` or alter cryptographic identity during profile updates.
- Do **NOT** overload `username` as mutable display label; keep separate `display_name`.
- Do **NOT** store avatar binaries/base64 blobs in localStorage or session payloads.
- Do **NOT** trust only client-declared MIME type; validate payload robustly server-side.
- Do **NOT** add broad catch-all error swallowing; surface explicit validation/processing failures.
- Do **NOT** bypass existing API envelope contract with bare JSON responses.

### Previous Story Intelligence

From Story 2.3 (`_bmad-output/implementation-artifacts/2-3-identity-persistence-and-auto-login.md`):

- Session persistence moved to `localStorage` (`discool-session`) and is synchronized cross-tab via `StorageEvent`.
- Auth race handling uses `authEpoch`; preserve this behavior when updating session user profile data.
- `identityStore` already hardens malformed stored data; profile updates should keep stored payload shape valid.
- One known remaining gap in 2.3 is router-based resume navigation (Epic 4 dependency). Do not entangle this story with router completion.

### Git Intelligence

Recent commits show identity/auth work is active and test-heavy:

- `4d0f866` test: add client review follow-up coverage
- `ef8962a` test(client): add identity feature tests, vitest config, and test setup
- `fc14c80` feat(auth): persist identity sessions
- `0a3b9db` feat(auth): challenge-response/session/auth middleware and client identity flow updates
- `0753378` fix: LoginView internal error mapping + DID/base58 edge-case test

Implication: keep this story incremental and aligned with existing identity module boundaries; avoid broad refactors.

### Latest Technical Information

1. **Axum multipart handling**: Use Axum multipart extractors with explicit body-size limits for upload endpoints and enforce limits at both app and proxy layers.  
   Source: https://docs.rs/axum/latest/axum/extract/struct.Multipart.html

2. **Image upload hardening**: Validate allowed image types (PNG/JPEG/WEBP), file size, and prefer content verification beyond declared MIME where possible.  
   Source: https://developer.mozilla.org/en-US/docs/Web/Media/Guides/Formats/Image_types

3. **Preview UX**: Use immediate client-side preview (`URL.createObjectURL`) with cleanup (`URL.revokeObjectURL`), validation feedback, and keyboard-accessible controls.  
   Source: https://www.okupter.com/blog/sveltekit-file-upload

4. **Accessibility expectations**: Labeled controls, inline error messaging, and screen-reader-friendly status messaging are required for file inputs and settings forms.  
   Source: https://uploadcare.com/blog/file-uploader-ux-best-practices/

### Project Context Reference

- No `project-context.md` file was discovered via configured pattern `**/project-context.md` in this repository.
- Use planning artifacts + existing implementation stories as the primary context source.

### Scope Boundaries (Out of Scope for This Story)

- Cross-instance profile propagation protocol details (Story 2.5 handles cross-instance identity verification).
- Email association/recovery flow (Stories 2.6 and 2.7).
- General channel/file attachment system for chat (Epic 6 file-sharing stories).

### Story Completion Status

- Status set to `ready-for-dev`.
- Ultimate context engine analysis completed - comprehensive developer guide created.

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Story 2.4] — Story statement and acceptance criteria
- [Source: _bmad-output/planning-artifacts/epics.md#Epic 2] — Epic objectives and related story sequencing
- [Source: _bmad-output/planning-artifacts/prd.md#Identity & Authentication] — FR2 display name/avatar requirement
- [Source: _bmad-output/planning-artifacts/prd.md#Non-Functional Requirements - Security] — validation and sanitization expectations
- [Source: _bmad-output/planning-artifacts/architecture.md#API Boundary (JSON)] — snake_case + envelope contracts
- [Source: _bmad-output/planning-artifacts/architecture.md#Authentication & Security] — session/auth constraints
- [Source: _bmad-output/planning-artifacts/architecture.md#Structure Patterns] — intended `handlers/users.rs` and feature locations
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Experience Mechanics] — avatar optionality + flow simplicity
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Component Inventory] — Avatar/input/form component guidance
- [Source: _bmad-output/implementation-artifacts/2-3-identity-persistence-and-auto-login.md] — existing identity/session patterns and guardrails

## Dev Agent Record

### Agent Model Used

GitHub Copilot CLI 0.0.414

### Debug Log References

- `cd client && npm run lint && npm run check && npm run test`
- `cd server && cargo fmt --check && cargo clippy -- -D warnings && cargo test`
- Added migration, handlers, services, client settings UI, and profile/avatar API tests.

### Completion Notes List

- Implemented profile persistence updates with `display_name` and avatar storage metadata plus config-driven avatar storage limits/paths.
- Added authenticated `/api/v1/users/me/profile` (GET/PATCH) and `/api/v1/users/me/avatar` (POST/GET) endpoints with strict validation and safe server-owned file storage.
- Extended client identity API/types/store for profile updates and immediate `identityState.session.user` refresh without reload.
- Added `ProfileSettingsView` UI with single-column layout, blur validation, avatar mode controls, immediate image preview, and fire primary save CTA.
- Added focused server/client tests for auth enforcement, display name/avatar validation matrix, persistence, preview behavior, and error handling.

### Change Log

- 2026-02-24: Implemented Story 2.4 end-to-end (server profile/avatar APIs + storage, client settings UI/state wiring, and expanded automated tests).
- 2026-02-24: Senior code review fixes applied for avatar mode switching and profile/session consistency on mixed profile+avatar saves.

### File List

- `_bmad-output/implementation-artifacts/2-4-display-name-and-avatar-management.md`
- `_bmad-output/implementation-artifacts/sprint-status.yaml`
- `client/src/App.svelte`
- `client/src/lib/features/identity/ProfileSettingsView.svelte`
- `client/src/lib/features/identity/ProfileSettingsView.test.ts`
- `client/src/lib/features/identity/identityApi.ts`
- `client/src/lib/features/identity/identityApi.test.ts`
- `client/src/lib/features/identity/identityStore.svelte.ts`
- `client/src/lib/features/identity/identityStore.test.ts`
- `client/src/lib/features/identity/types.ts`
- `config.example.toml`
- `server/Cargo.toml`
- `server/Cargo.lock`
- `server/migrations/0005_user_profile_avatar_metadata.sql`
- `server/src/config/mod.rs`
- `server/src/config/settings.rs`
- `server/src/handlers/auth.rs`
- `server/src/handlers/mod.rs`
- `server/src/handlers/users.rs`
- `server/src/models/user.rs`
- `server/src/services/auth_service.rs`
- `server/src/services/mod.rs`
- `server/src/services/user_profile_service.rs`
- `server/tests/server_binds_to_configured_port.rs`

### Senior Developer Review (AI)

- Outcome: **Changes Requested → Fixed** (high/medium findings resolved in this pass).
- High: Switching from uploaded image avatar back to color mode did not clear server avatar metadata, so `avatar_url` stayed active.
  - Fix: `update_profile` now clears avatar storage fields and old file when `avatar_color` is explicitly updated; integration coverage added for `upload -> switch-to-color -> avatar 404`.
- Medium: `identityState.saveProfile` deferred session update until after avatar upload; if upload failed after profile PATCH success, UI/session could stay stale.
  - Fix: session is now updated immediately after each successful profile mutation step (PATCH and upload).
- Medium: server-side invalid `display_name` rejection path lacked integration coverage.
  - Fix: added `users_profile_patch_rejects_invalid_display_name` integration test to assert 422 `VALIDATION_ERROR`.
- Verification rerun: `cd client && npm run lint && npm run check && npm run test && npm run build` and `cd server && cargo fmt --check && cargo clippy -- -D warnings && cargo test`.
