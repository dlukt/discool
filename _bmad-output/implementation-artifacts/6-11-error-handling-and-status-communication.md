# Story 6.11: Error Handling and Status Communication

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a user,
I want clear, honest feedback when something goes wrong,
so that I understand what happened and what to do about it.

## Acceptance Criteria

1. Error messages use plain language (for example, "Connection lost. Reconnecting..." instead of technical protocol errors).
2. Permission errors show: "You don't have permission to do this."
3. Failed message sends show an error toast with a "Retry?" action.
4. Toasts render bottom-right, max 3 visible, stacked vertically.
5. Success toasts auto-dismiss after 4 seconds; error toasts persist until dismissed.
6. Toasts are pausable on hover/focus.
7. Async operations always show status text (for example, "Connecting...", "Uploading (43%)...", "Saving...").
8. No spinner appears without accompanying text.
9. Server `AppError` mapping returns user-friendly messages and never exposes internal details.
10. Error states are accessible and announced with `aria-live`.

## Tasks / Subtasks

- [x] Build a shared user-facing error mapping for REST and WebSocket failures (AC: 1, 2, 9)
  - [x] Audit server `AppError` usage and ensure user-facing messages are plain language, consistent, and safe.
  - [x] Add client-side error normalization utility that maps API/WS failures to UX-approved copy (including permission-denied copy).
  - [x] Ensure technical internals (stack traces, SQL details, protocol internals) never leak to users.
- [x] Implement a reusable toast system aligned with UX rules (AC: 3, 4, 5, 6, 10)
  - [x] Add a central toast store + viewport rendered bottom-right with vertical stack and max 3 visible.
  - [x] Support variants with behavior rules: success/info auto-dismiss at 4s, error persistent until dismissed.
  - [x] Add hover/focus pause behavior and keyboard/screen-reader-friendly semantics (`aria-live`).
  - [x] Support optional action CTA for error recovery (first use case: failed message send "Retry?").
- [x] Wire operation status communication into active chat flows (AC: 1, 3, 7, 8, 10)
  - [x] Keep connection lifecycle status text visible for WebSocket states (`connecting`, `reconnecting`, `disconnected`) with plain-language copy.
  - [x] Ensure attachment upload, message send, and edit/delete/save paths expose explicit status text (no spinner-only states).
  - [x] Route message send failures to toast + retry action instead of silent fail or raw error text.
- [x] Apply permission and operation-failure messaging consistently across Epic 6 surfaces (AC: 1, 2, 7)
  - [x] Standardize permission-denied messaging for chat actions.
  - [x] Ensure unreachable/connection-loss paths use actionable copy and retry/reconnect guidance.
  - [x] Reuse existing API envelope and error parsing contracts (`{ "error": { code, message, details } }`).
- [x] Add AC-focused tests and run quality gates (AC: 1-10)
  - [x] Client tests: toast stack behavior, pause-on-hover/focus, persistent error toast, retry action callback, and plain-language connection/status text rendering.
  - [x] Client tests: failed message send path and permission-denied path in chat UI.
  - [x] Server tests: `AppError` response mapping and sanitization behavior for internal errors.
  - [x] Run project quality commands:
    - [x] `cd client && npm run lint && npm run check && npm run test`
    - [x] `cd server && cargo fmt --check && cargo clippy -- -D warnings && cargo test`

## Dev Notes

### Developer Context

- Epic 6 already shipped WebSocket lifecycle handling, message timeline operations, typing indicators, DMs, and block-based filtering (Stories 6.1-6.10).
- Current UI already shows reconnecting banner and upload progress text in `MessageArea.svelte`; Story 6.11 should generalize consistent status communication, not replace existing good patterns.
- Current API layer (`client/src/lib/api.ts`) parses envelope errors and provides `ApiError`; this should be reused as the canonical REST error source.
- Current backend already uses `AppError` + `IntoResponse` in `server/src/error.rs`; this story hardens user-facing mapping consistency and client-facing copy quality.

### Technical Requirements & Guardrails

- Use plain-language, user-actionable copy in all user-visible failures (no protocol codes or implementation jargon).
- Keep permission-denied language exact and consistent with AC: "You don't have permission to do this."
- Retry affordances must be explicit where operation recovery is possible (starting with failed message send).
- Preserve existing optimistic chat behavior and do not regress Story 6.9/6.10 message, DM, or block-erasure flows.
- Do not add spinner-only states; all async visuals must include explicit status text.

### Architecture Compliance Notes

- Preserve server contract: handlers return `Result<Json<T>, AppError>` and `AppError` maps to sanitized responses via `IntoResponse`.
- Preserve API envelope format (`{ "data": ... }` and `{ "error": ... }`) and existing `ApiError` parsing conventions.
- Respect WebSocket lifecycle model (`connecting` | `connected` | `reconnecting` | `disconnected`) and existing reconnect/backoff behavior.
- Keep state ownership clear:
  - Error/feedback presentation in dedicated UI/store layer.
  - Transport and protocol logic in `client/src/lib/ws/client.ts` and feature stores.
  - Server error categorization in `server/src/error.rs` and handler/service boundaries.

### Library & Framework Requirements

- Client baseline in repo:
  - `svelte ^5.45.2`
  - `@mateothegreat/svelte5-router ^2.16.19`
  - `vite ^7.3.1`
  - `vitest ^4.0.18`
- Server baseline in repo:
  - `axum 0.8`
  - `sqlx 0.8`
  - `tokio 1`
- Latest upstream check (awareness only, no upgrade required by this story):
  - Svelte latest: `5.53.6`
  - `@mateothegreat/svelte5-router` latest: `2.16.19`
  - Axum latest release: `0.8.8`
  - SQLx latest tag: `0.8.6`

### File Structure Requirements

- Expected client touch points:
  - `client/src/lib/features/chat/MessageArea.svelte`
  - `client/src/lib/features/chat/messageStore.svelte.ts`
  - `client/src/lib/ws/client.ts`
  - `client/src/lib/api.ts`
  - `client/src/App.svelte` (global feedback mount point if required)
  - New shared toast/feedback modules under `client/src/lib/` (store + UI component)
  - Targeted test files under existing feature/test structure
- Expected server touch points:
  - `server/src/error.rs`
  - Relevant handlers/services where user-facing error copy currently bypasses shared mapping
  - `server/tests/server_binds_to_configured_port.rs` (or targeted integration/unit tests)

### Testing Requirements Summary

- Validate toast placement and stacking constraints (bottom-right, max 3 visible, oldest dismissed first).
- Validate toast behavior timing and persistence rules (4s success/info, persistent error).
- Validate pause-on-hover/focus behavior and `aria-live` announcements.
- Validate failed send recovery path exposes "Retry?" action and executes retry handler.
- Validate permission-denied and reconnect messaging use plain language and do not leak technical internals.
- Validate server-side internal errors remain sanitized in client-facing responses.

### Previous Story Intelligence (6.10)

- Story 6.10 added broad chat/member/DM filtering surfaces and optional block sync; avoid introducing feedback logic that bypasses those stores/selectors.
- Recent follow-up commit (`e91d6ff`) extended reaction payload metadata for block filtering; keep new error/status changes compatible with existing message/reaction ingestion.
- Existing `MessageArea` and `messageStore` already contain upload/reconnect states and are the lowest-risk integration points for Story 6.11.

### Git Intelligence Summary

- Recent commit sequence confirms Epic 6 implementation cadence through 6.10 and a focused fix pass:
  - `e91d6ff` feat: include reaction actors for block filtering
  - `e6d031d` feat: finalize story 6-10 user blocking complete erasure
  - `0cc2c2c` feat: finalize story 6-9 direct messages
- This story should remain scoped to feedback/status quality and not absorb Story 6.12 quick-switcher functionality.

### Latest Technical Information

- Verified latest package/release endpoints for Svelte, svelte5-router, Axum, and SQLx.
- No breaking version pressure requires upgrade in this story; prioritize behavior correctness on current project baselines.
- Keep dependency changes out-of-scope unless required to satisfy acceptance criteria.

### Project Context Reference

- `project-context.md` was not found in configured artifact locations during create-story discovery.

### Story Completion Status

- Story file created: `_bmad-output/implementation-artifacts/6-11-error-handling-and-status-communication.md`
- Sprint status target for this story: `ready-for-dev`
- Completion note: Ultimate context engine analysis completed - comprehensive developer guide created.

### Project Structure Notes

- Planned touches align with existing project boundaries (`client/src/lib/features/chat`, `client/src/lib/ws`, shared `client/src/lib` utilities/stores, and `server/src/error.rs`).
- No structural conflicts detected; this story is additive and should reuse existing primitives rather than introducing parallel transport/error stacks.

### References

- Story definition and ACs: [_bmad-output/planning-artifacts/epics.md (Epic 6, Story 6.11)]
- UX feedback standards: [_bmad-output/planning-artifacts/ux-design-specification.md (Feedback Patterns; Status messages; accessibility notes)]
- Error/resilience architecture: [_bmad-output/planning-artifacts/architecture.md (Error Handling, Loading State Pattern, Retry/Reconnect Pattern)]
- Product requirements: [_bmad-output/planning-artifacts/prd.md (FR61 clear error messages, FR62 auto-reconnect)]
- Previous story context: [_bmad-output/implementation-artifacts/6-10-user-blocking-complete-erasure.md]
- Current server error mapping: [`server/src/error.rs`]
- Current client transport/API surfaces: [`client/src/lib/ws/client.ts`, `client/src/lib/api.ts`, `client/src/lib/features/chat/MessageArea.svelte`, `client/src/lib/features/chat/messageStore.svelte.ts`]
- Latest dependency/release checks:
  - https://registry.npmjs.org/svelte/latest
  - https://registry.npmjs.org/@mateothegreat/svelte5-router/latest
  - https://api.github.com/repos/tokio-rs/axum/releases/latest
  - https://api.github.com/repos/launchbadge/sqlx/tags?per_page=1

## Dev Agent Record

### Agent Model Used

GPT-5.3-Codex (model ID: gpt-5.3-codex)

### Debug Log References

- Story creation workflow execution: create-story (epic 6, story 11).

### Completion Notes List

- Added a shared user-facing error mapping module for API and WebSocket failures, including standardized permission copy and lifecycle status text helpers.
- Implemented a reusable toast system (store + viewport) with bottom-right stack, max 3 visible, auto-dismiss behavior (success/info 4s), persistent error toasts, hover/focus pause, and accessible live announcements.
- Updated `MessageArea` to surface plain-language connection status, explicit operation status text for send/save/delete/upload flows, and retryable error toasts for failed sends.
- Added focused tests for toast behavior, chat retry/status UX, and client error mapping.
- Added server `AppError` unit tests validating internal-error sanitization and response envelope consistency.
- Executed quality gates successfully:
  - `cd client && npm run lint && npm run check && npm run test`
  - `cd server && cargo fmt --check && cargo clippy -- -D warnings && cargo test`

### File List

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
- `server/src/error.rs`

### Change Log

- 2026-03-01: Implemented Story 6.11 error/status UX enhancements, reusable toasts, client/server test coverage, and completed full client/server quality gates.
- 2026-03-01: Senior Developer Review (AI) found two test coverage gaps; both were fixed and client quality gates were re-run successfully.

### Senior Developer Review (AI)

- Reviewer: Darko (AI)
- Date: 2026-03-01
- Outcome: Approve

#### Findings

1. [HIGH] Toast AC test coverage was incomplete: focus pause/resume behavior was implemented but not tested (`client/src/lib/components/ToastViewport.test.ts`).
2. [HIGH] Chat permission-denied path coverage was incomplete: no test asserted standardized permission copy when attachment sends are blocked (`client/src/lib/features/chat/MessageArea.test.ts`).

#### Fixes Applied

- Added focus pause/resume toast test in `client/src/lib/components/ToastViewport.test.ts`.
- Added permission-denied attachment send test in `client/src/lib/features/chat/MessageArea.test.ts`.
- Re-ran `cd client && npm run lint && npm run check && npm run test` (pass).
