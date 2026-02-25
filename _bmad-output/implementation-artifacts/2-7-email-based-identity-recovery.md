# Story 2.7: Email-Based Identity Recovery

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a **user**,
I want to recover my identity via email if my browser storage is cleared,
So that I don't permanently lose access to my guilds and messages.

## Acceptance Criteria

1. **Given** a user opens Discool with no identity in browser storage  
   **When** they select "Recover existing identity" and enter their email  
   **Then** the server sends a recovery link to the registered email address.

2. **Given** a user receives the recovery email  
   **When** they click the link  
   **Then** the SPA opens with a recovery token.

3. **Given** the SPA has a recovery token  
   **When** the recovery endpoint validates the token  
   **Then** the encrypted private key payload for that identity is returned for client-side restore.

4. **Given** the encrypted payload is returned  
   **When** the client completes restore  
   **Then** the identity is restored in browser storage (IndexedDB).

5. **Given** identity restore succeeds  
   **When** the user continues  
   **Then** they can authenticate and see their guilds, channels, and prior data as before.

6. **Given** no recovery identity exists for the submitted email  
   **When** the user submits the recovery form  
   **Then** a clear message is shown: **"No identity found for this email."**

7. **Given** the recovery email is delayed or blocked  
   **When** the user asks for help  
   **Then** a clear message is shown: **"Didn't receive the email? Check spam, or try again."**

8. **Given** recovery is presented in the UI  
   **When** content is rendered  
   **Then** plain language is used ("recover your identity"), not cryptographic jargon.

9. **Given** recovery endpoints are public  
   **When** requests are processed  
   **Then** token handling is one-time, short-lived, hashed at rest, and rate-limited.

10. **Given** existing optional recovery-email association from Story 2.6  
    **When** Story 2.7 is implemented  
    **Then** Story 2.6 association/verification behavior remains backward-compatible.

## Tasks / Subtasks

- [x] Task 1: Add unauthenticated recovery-initiation API flow (AC: #1, #6, #7, #9)
  - [x] 1.1 Add a public endpoint under auth surface (recommended: `POST /api/v1/auth/recovery-email/start`) in `server/src/handlers/auth.rs` and route wiring in `server/src/handlers/mod.rs`.
  - [x] 1.2 Accept only email input; normalize and validate format via existing validation error patterns.
  - [x] 1.3 Resolve only **verified** associations from `user_recovery_email` and trigger recovery email delivery.
  - [x] 1.4 Return API envelopes (`{"data": ...}` / `{"error": ...}`) and UX-mappable error codes/messages for:
    - no identity found,
    - too many attempts,
    - email send failure.

- [x] Task 2: Add recovery token lifecycle and redemption API (AC: #2, #3, #9, #10)
  - [x] 2.1 Implement dedicated recovery-token persistence to avoid cross-flow token confusion with Story 2.6 verification tokens (new migration preferred, e.g. `server/migrations/0008_identity_recovery_tokens.sql`).
  - [x] 2.2 Store token hash only (never plaintext), enforce TTL and single-use semantics, and record requester IP for rate limiting.
  - [x] 2.3 Add token redemption endpoint (recommended: `GET /api/v1/auth/recovery-email/recover?token=...`) returning payload needed for client restore in `{"data": ...}` envelope.
  - [x] 2.4 Keep Story 2.6 endpoint `GET /api/v1/auth/recovery-email/verify` behavior intact for email-association verification.

- [x] Task 3: Implement recovery email delivery path (AC: #1, #2, #7, #9)
  - [x] 3.1 Add recovery-specific email sender function in `server/src/services/email_service.rs` (or equivalent) with recovery link and plain-language copy.
  - [x] 3.2 Build links from configured base URL only; do not derive callback origin from request headers.
  - [x] 3.3 Keep SMTP failure explicit and user-visible via current AppError envelope conventions.

- [x] Task 4: Implement client recovery UX for no-identity entry point (AC: #1, #6, #7, #8)
  - [x] 4.1 Add recovery API methods/types in `client/src/lib/features/identity/identityApi.ts` and `types.ts` for start + redeem flow.
  - [x] 4.2 Add a dedicated recovery form/view in `client/src/lib/features/identity/` (or extend existing identity flow) to collect email and show states/messages.
  - [x] 4.3 Wire no-identity state in `client/src/App.svelte` so users can choose **Create new identity** or **Recover existing identity**.
  - [x] 4.4 Keep copy plain-language per UX spec; include exact fallback messages from AC #6/#7.

- [x] Task 5: Restore identity locally and re-authenticate (AC: #3, #4, #5, #8)
  - [x] 5.1 Add identity restore helper(s) in `client/src/lib/features/identity/crypto.ts` and `identityStore.svelte.ts` to persist recovered identity into IndexedDB using existing storage primitives.
  - [x] 5.2 Reuse existing challenge-response auth flow after restore (do not add a parallel auth system).
  - [x] 5.3 Replace Story 2.6 placeholder in `App.svelte` (`handleRecoveryPromptRecover`) with real flow initiation.
  - [x] 5.4 Ensure sensitive key material is short-lived in memory and cleared where practical.

- [x] Task 6: Add focused test coverage and manual verification (AC: all)
  - [x] 6.1 Server tests (`server/src/handlers/auth.rs`, `server/src/services/recovery_email_service.rs` and/or new tests):
    - start-recovery success for verified association,
    - no-identity-found behavior,
    - token expiry and replay rejection,
    - IP/user rate-limit enforcement,
    - redeem returns expected payload shape.
  - [x] 6.2 Client tests (`client/src/lib/features/identity/*.test.ts`):
    - recovery option visible in no-identity flow,
    - start flow renders success/error states correctly,
    - token redemption restores identity state and proceeds to auth.
  - [x] 6.3 Manual checks:
    - cleared browser storage → recover via email works end-to-end,
    - invalid/expired token path shows clear recovery error,
    - existing Story 2.6 association verification still works.

## Dev Notes

### Architecture Compliance

1. Preserve backend layering and boundaries: handlers → services → models (`server/src/handlers`, `server/src/services`, `server/src/models`).  
2. Keep REST envelopes and AppError handling consistent (`{"data": ...}` / `{"error": ...}`, no bare objects).  
3. Maintain PostgreSQL + SQLite compatibility for all queries/migrations (`sqlx` Any-driver constraints).  
4. Keep email recovery optional and outside onboarding gates (progressive commitment UX principle).

### Technical Requirements (Developer Guardrails)

- Reuse existing recovery-email assets from Story 2.6 (`user_recovery_email`, `email_verification_tokens`, `recovery_email_service`) instead of parallel duplicate storage.
- Keep token security strict: high entropy, hash-at-rest, short TTL, one-time use, explicit replay protection.
- Keep rate limiting on both initiation and redemption paths (per-IP and/or per-user as applicable).
- Never log plaintext token values, verification/recovery links with tokens, or private key payloads.
- Keep server/client error copy actionable and plain-language.

### Library & Framework Requirements

- Backend continues on Rust + Axum + sqlx + lettre + aes-gcm stack already used in Story 2.6.
- Svelte 5 rune patterns remain the standard in client components (`$state`, `onclick`, prop callbacks).
- Keep `snake_case` across API wire contracts and typed camelCase mapping in client adapters.

### File Structure Requirements

Expected implementation touch points:

- Server
  - `server/src/handlers/mod.rs` (route wiring)
  - `server/src/handlers/auth.rs` (public recovery start/redeem endpoints)
  - `server/src/services/recovery_email_service.rs` (token lifecycle + payload retrieval)
  - `server/src/services/email_service.rs` (recovery email sender/copy)
  - `server/src/models/recovery_email.rs` (token/response structs if expanded)
  - `server/migrations/0008_identity_recovery_tokens.sql` (or equivalent schema update)

- Client
  - `client/src/App.svelte`
  - `client/src/lib/features/identity/identityApi.ts`
  - `client/src/lib/features/identity/types.ts`
  - `client/src/lib/features/identity/identityStore.svelte.ts`
  - `client/src/lib/features/identity/crypto.ts`
  - `client/src/lib/features/identity/RecoveryPrompt.svelte`
  - `client/src/lib/features/identity/*recovery*.svelte` (new recovery entry component if added)

- Config/Docs
  - `config.example.toml` (if new recovery URL config is introduced)

### Testing Requirements

Run existing quality gates after implementation:

- `cd server && cargo fmt --check && cargo clippy -- -D warnings && cargo test`
- `cd client && npm run lint && npm run check && npm run test`

### Previous Story Intelligence

- Story 2.6 already implemented optional email association endpoints and persistence (`GET/POST /api/v1/users/me/recovery-email`, `GET /api/v1/auth/recovery-email/verify`) and marks 2.6 as done.
- `App.svelte` still contains an explicit Story 2.7 placeholder error in `handleRecoveryPromptRecover()` and currently does not provide a no-identity recovery entry path.
- `RecoveryPrompt.svelte` already has a "Recover via email" CTA hook; wire it to real flow rather than introducing a separate disconnected prompt system.
- Preserve Story 2.6 hardening: initiation history/rate-limit protections and verification-attempt tracking.

### Git Intelligence Summary

Recent commits are identity-focused and already touched most recovery-related files:

- `f0eebd0` feat: complete story 2.6 optional email association
- `6af96cc` feat: complete story 2.5 cross-instance identity verification
- `8d65d3c` feat(profile): implement story 2.4 profile and avatar management
- `4d0f866` test: add client review follow-up coverage
- `ef8962a` test(client): add identity feature tests, vitest config, and test setup

Implication: implement Story 2.7 incrementally in existing identity/auth surfaces, avoiding broad refactors.

### Latest Technical Information

1. OWASP account-recovery guidance aligns with current design direction: generic-safe recovery flow, hashed one-time tokens, strict TTL, and rate limiting.
2. Lettre current stable series remains `0.11.x`; repository already uses `lettre = "0.11"` with async rustls feature set.
3. Svelte 5 migration guidance favors direct DOM event attributes (`onclick`) over legacy `on:click` for new rune-based components.

### Project Context Reference

- No `project-context.md` file discovered via configured pattern `**/project-context.md`.
- Planning artifacts and completed Epic 2 stories are the authoritative context inputs for this story.

### Story Completion Status

- Ultimate context engine analysis completed - comprehensive developer guide created.
- Story marked `ready-for-dev` for implementation handoff.

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Story 2.7: Email-Based Identity Recovery]
- [Source: _bmad-output/planning-artifacts/prd.md#Security Constraints]
- [Source: _bmad-output/planning-artifacts/prd.md#Functional Requirements]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Journey 6: Identity Recovery]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Flow Optimization Principles]
- [Source: _bmad-output/planning-artifacts/architecture.md#Data Architecture]
- [Source: _bmad-output/planning-artifacts/architecture.md#Authentication & Security]
- [Source: _bmad-output/planning-artifacts/architecture.md#Format Patterns]
- [Source: _bmad-output/planning-artifacts/architecture.md#Enforcement Guidelines]
- [Source: _bmad-output/implementation-artifacts/2-6-optional-email-association.md]
- [Source: server/src/handlers/mod.rs]
- [Source: server/src/handlers/auth.rs]
- [Source: server/src/handlers/users.rs]
- [Source: server/src/services/recovery_email_service.rs]
- [Source: server/src/services/email_service.rs]
- [Source: server/src/models/recovery_email.rs]
- [Source: server/migrations/0006_recovery_email_association.sql]
- [Source: server/migrations/0007_email_verification_attempts.sql]
- [Source: client/src/App.svelte]
- [Source: client/src/lib/features/identity/RecoveryPrompt.svelte]
- [Source: client/src/lib/features/identity/identityStore.svelte.ts]
- [Source: client/src/lib/features/identity/identityApi.ts]
- [Source: client/src/lib/features/identity/crypto.ts]
- [Source: https://cheatsheetseries.owasp.org/cheatsheets/Forgot_Password_Cheat_Sheet.html]
- [Source: https://docs.rs/lettre/latest/lettre/]
- [Source: https://svelte.dev/docs/svelte/v5-migration-guide]

## Dev Agent Record

### Agent Model Used

GPT-5.3-Codex (Copilot CLI 0.0.416)

### Debug Log References

- `cd server && cargo fmt --check && cargo clippy -- -D warnings && cargo test`
- `cd client && npm run lint && npm run check && npm run test`

### Completion Notes List

- Added unauthenticated recovery endpoints `POST /api/v1/auth/recovery-email/start` and `GET /api/v1/auth/recovery-email/recover` with strict envelope/error patterns and Story 2.6 verify endpoint compatibility preserved.
- Added dedicated recovery lifecycle persistence (`0008_identity_recovery_tokens.sql`) with hashed one-time tokens, TTL, replay protection, and explicit start/redeem rate-limit tracking tables.
- Extended recovery service logic to resolve only verified recovery-email associations, send recovery-start responses with plain-language help copy, and redeem recovery tokens into restore-ready payloads.
- Added recovery-specific email delivery path with configured `email.recovery_url_base` links and plain-language recovery content.
- Added client recovery flow API/types/store methods plus new `IdentityRecoveryView` UX for no-identity and corrupted-identity entry points, including exact no-identity/help copy requirements.
- Implemented local identity restore helper in `crypto.ts` and reused existing challenge-response authentication after token redemption.
- Replaced Story 2.7 placeholder `handleRecoveryPromptRecover` flow in `App.svelte` with real recovery-flow initiation and UI routing.
- Added/updated focused server/client tests for success, no-identity messaging, token expiry/replay, rate limits, payload mapping, restore flow wiring, and recovery CTA behavior.
- Senior code review fixes applied: recovery token URL state now re-syncs in `App.svelte` after token clear, failed recovery-email sends now mark tokens as `send-failed` and are excluded from per-user start quota, and regression tests were added for both paths.

### File List

- `server/migrations/0008_identity_recovery_tokens.sql`
- `server/src/models/recovery_email.rs`
- `server/src/services/recovery_email_service.rs`
- `server/src/services/email_service.rs`
- `server/src/handlers/auth.rs`
- `server/src/handlers/mod.rs`
- `server/src/config/settings.rs`
- `config.example.toml`
- `client/src/lib/features/identity/types.ts`
- `client/src/lib/features/identity/identityApi.ts`
- `client/src/lib/features/identity/identityStore.svelte.ts`
- `client/src/lib/features/identity/crypto.ts`
- `client/src/lib/features/identity/IdentityRecoveryView.svelte`
- `client/src/App.svelte`
- `client/src/lib/features/identity/LoginView.svelte`
- `client/src/lib/features/identity/RecoveryPrompt.svelte`
- `client/src/lib/features/identity/identityApi.test.ts`
- `client/src/lib/features/identity/identityStore.test.ts`
- `client/src/lib/features/identity/crypto.test.ts`
- `client/src/lib/features/identity/IdentityRecoveryView.test.ts`
- `client/src/lib/features/identity/LoginView.test.ts`
- `client/src/lib/features/identity/RecoveryPrompt.test.ts`
- `_bmad-output/implementation-artifacts/2-7-email-based-identity-recovery.md`
- `_bmad-output/implementation-artifacts/sprint-status.yaml`

### Senior Developer Review (AI)

- **Date:** 2026-02-25
- **Outcome:** Changes Requested → Fixed
- **Findings addressed:**
  - **HIGH:** `App.svelte` cached `recovery_token` and could keep users stuck in recovery UI after the URL token was cleared.
  - **MEDIUM:** `start_identity_recovery` persisted recovery tokens before email send; send failures consumed per-user recovery quota.
  - **LOW:** No explicit regression checks existed for token-clear synchronization and send-failure quota behavior.
- **Fix summary:** Added token-clear callback wiring between `IdentityRecoveryView` and `App.svelte`, added `send-failed` token cleanup + quota exclusion in recovery service/handler flow, and added focused client/server tests.
- **Additional review fix (2026-02-25):** URL token query values in recovery/verification email links are now percent-encoded in `email_service::build_token_link`, with regression coverage for reserved characters.

### Change Log

- 2026-02-25: Implemented Story 2.7 email-based identity recovery end-to-end (public start/redeem APIs, dedicated recovery token lifecycle, recovery email delivery, client recovery UX + local restore + re-auth, and focused server/client test coverage). Status moved to `review`.
- 2026-02-25: Completed adversarial code review follow-ups; fixed recovery-token UI state sync and send-failure quota consumption, added regression tests, and moved status to `done`.
- 2026-02-25: Follow-up review hardening: percent-encoded email-link token query values and added server regression test for encoded tokens.
