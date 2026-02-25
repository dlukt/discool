# Story 2.6: Optional Email Association

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a **user**,
I want to optionally associate an email address with my identity,
So that I have a recovery option if I lose my browser storage.

## Acceptance Criteria

1. **Given** a user is authenticated  
   **When** they navigate to identity settings and enter an email address  
   **Then** a verification email is sent to the provided address.

2. **Given** a user is authenticated  
   **When** they receive the verification email and click the verification link  
   **Then** the association is marked as verified for their identity.

3. **Given** a user verifies their email association  
   **When** verification completes  
   **Then** the server stores an encrypted copy of the user's private key using a key derived from a server-side secret and normalized email.

4. **Given** a user has associated an email  
   **When** they open identity settings  
   **Then** email association state is visible with a clear **Verified** status.

5. **Given** a user does not want to associate an email  
   **When** they use Discool normally  
   **Then** no feature is blocked and email remains optional.

6. **Given** a user has completed their first successful session  
   **When** they return to normal app flow  
   **Then** a subtle, non-intrusive prompt suggests enabling email recovery (not during onboarding).

## Tasks / Subtasks

- [x] Task 1: Add server-side persistence for recovery email association and verification lifecycle (AC: #2, #3, #4, #5)
  - [x] 1.1 Add a new migration (e.g., `server/migrations/0006_recovery_email_association.sql`) for:
    - `user_recovery_email` (one row per user, normalized email, verified timestamp, encrypted private key blob, metadata, created/updated timestamps)
    - `email_verification_tokens` (hashed token, user_id, target email, expires_at, used_at, created_at)
  - [x] 1.2 Ensure SQL and indexes are compatible for both PostgreSQL and SQLite as in existing migrations.
  - [x] 1.3 Add/extend Rust model structs for these records under `server/src/models/`.
  - [x] 1.4 Keep writes idempotent for repeated initiation requests from the same authenticated user.

- [x] Task 2: Implement authenticated API flow to start email association (AC: #1, #4, #5)
  - [x] 2.1 Add endpoint(s) under existing authenticated user surface (recommended: `POST /api/v1/users/me/recovery-email` and `GET /api/v1/users/me/recovery-email`) in `server/src/handlers/users.rs` and route wiring in `server/src/handlers/mod.rs`.
  - [x] 2.2 Validate email format, normalize case, and reject malformed input with existing `VALIDATION_ERROR` envelope patterns.
  - [x] 2.3 Generate high-entropy verification token, store only token hash server-side, and persist strict expiration + one-time-use fields.
  - [x] 2.4 Return non-sensitive status shape via `{ "data": ... }` envelope (never return raw tokens).

- [x] Task 3: Implement verification and secure key escrow write path (AC: #2, #3)
  - [x] 3.1 Add verification endpoint for clicked links (recommended path: `GET /api/v1/auth/recovery-email/verify` with token query param).
  - [x] 3.2 Mark token as used atomically and reject replay/expired tokens with explicit errors.
  - [x] 3.3 Reuse existing client key access flow (`decryptSecretKey` in `client/src/lib/features/identity/crypto.ts`) to provide private key material only when user explicitly enables recovery.
  - [x] 3.4 On server, derive encryption key from server secret + normalized email, encrypt private key payload, and store encrypted blob plus algorithm/version metadata for Story 2.7 compatibility.
  - [x] 3.5 Ensure sensitive values (token plaintext, private key bytes, derived keys) are not logged and are zeroized/short-lived in memory where practical.

- [x] Task 4: Add email delivery integration and configuration plumbing (AC: #1, #2)
  - [x] 4.1 Extend config structs in `server/src/config/settings.rs` and exports in `server/src/config/mod.rs` with an email section (SMTP host/port/auth/sender/from name, verification URL base, token TTL, rate limits).
  - [x] 4.2 Document new config in `config.example.toml` and environment mapping (`DISCOOL_EMAIL__...`).
  - [x] 4.3 Implement mail delivery service (recommended crate: `lettre` async SMTP, Tokio + rustls feature set) under `server/src/services/`.
  - [x] 4.4 Keep delivery failures explicit: association-start endpoint should report a clear failure when email cannot be sent (no silent success fallback).

- [x] Task 5: Implement settings UI + subtle recovery prompt in client (AC: #4, #5, #6)
  - [x] 5.1 Extend identity API/types in `client/src/lib/features/identity/identityApi.ts` and `types.ts` for recovery-email initiate/status operations with snake_case wire mapping.
  - [x] 5.2 Extend `client/src/lib/features/identity/ProfileSettingsView.svelte` with a recovery email section showing current status (`unverified`/`verified`) and actions (`Send verification`, `Resend`, optional `Remove`).
  - [x] 5.3 Activate recovery action path in `client/src/lib/features/identity/RecoveryPrompt.svelte` (currently disabled) by wiring explicit handlers.
  - [x] 5.4 Add post-first-session suggestion outside onboarding in `client/src/App.svelte` or a dedicated settings nudge component; do not place prompt inside `LoginView.svelte`.
  - [x] 5.5 Preserve current create/register/auth flows from Stories 2.1–2.5; email association must remain optional.

- [x] Task 6: Add focused test coverage for security, API contracts, and UX behavior (AC: all)
  - [x] 6.1 Server tests in `server/src/handlers/users.rs` and/or new service tests:
    - authenticated start-association success path
    - malformed email returns 422
    - token expiry/replay rejection
    - verify endpoint marks association verified exactly once
    - encrypted key payload persisted only after successful verification flow
  - [x] 6.2 Client tests in `client/src/lib/features/identity/*.test.ts`:
    - Profile settings email section renders and updates status
    - Recovery prompt CTA wired and not permanently disabled
    - post-session prompt appears only after session exists and not during onboarding
  - [x] 6.3 Manual checks:
    - verification email received and link consumed once
    - settings display verified state correctly
    - app works fully when no email is associated

## Dev Notes

### Architecture Compliance

1. Keep existing auth/session model centralized (challenge/verify/session in `auth` handler + `auth_service`) and avoid introducing a parallel auth path for email association.
2. Preserve API response envelopes (`{ "data": ... }` / `{ "error": ... }`) and current `AppError` semantics.
3. Maintain dual-database compatibility (PostgreSQL and SQLite branches) for new tables and queries.
4. Keep onboarding friction low: this story is settings-driven and optional; no onboarding gate changes.

### Technical Requirements (Developer Guardrails)

- Reuse existing identity storage and signing architecture from `crypto.ts` and `identityStore.svelte.ts`; do not duplicate key-storage mechanisms.
- Verification tokens must be high-entropy, short-lived, one-time, and stored hashed server-side.
- Apply per-user + IP rate limits to email initiation and verification attempts.
- Keep user-visible errors plain-language and actionable, consistent with Story 6.11 UX principles.
- Never store plaintext private keys or plaintext verification tokens in persistent storage.

### API Contracts (Proposed Additive Shape)

`POST /api/v1/users/me/recovery-email`
```json
{
  "email": "liam@example.com",
  "encrypted_private_key": "<base64-or-hex-payload>",
  "encryption_context": {
    "algorithm": "aes-256-gcm",
    "version": 1
  }
}
```

`GET /api/v1/users/me/recovery-email`
```json
{
  "data": {
    "email_masked": "l***@example.com",
    "verified": true,
    "verified_at": "2026-02-25T13:00:00Z"
  }
}
```

`GET /api/v1/auth/recovery-email/verify?token=<token>`
```json
{
  "data": {
    "verified": true
  }
}
```

### File Structure Requirements

Expected implementation touch points:

- Server
  - `server/src/handlers/mod.rs` (route wiring)
  - `server/src/handlers/users.rs` (authenticated recovery email endpoints)
  - `server/src/handlers/auth.rs` (verification endpoint if kept under auth surface)
  - `server/src/services/` (new recovery email + mail delivery service modules)
  - `server/src/config/settings.rs`
  - `server/src/config/mod.rs`
  - `server/src/models/` (new model types for recovery email + tokens)
  - `server/migrations/0006_recovery_email_association.sql` (new)
  - `server/Cargo.toml` (SMTP dependency additions if needed)

- Client
  - `client/src/lib/features/identity/identityApi.ts`
  - `client/src/lib/features/identity/types.ts`
  - `client/src/lib/features/identity/identityStore.svelte.ts` (state/status wiring)
  - `client/src/lib/features/identity/ProfileSettingsView.svelte`
  - `client/src/lib/features/identity/RecoveryPrompt.svelte`
  - `client/src/App.svelte` (post-session suggestion placement)

- Config/docs
  - `config.example.toml`

### Testing Requirements

Run existing quality gates after implementation:

- `cd server && cargo fmt --check && cargo clippy -- -D warnings && cargo test`
- `cd client && npm run lint && npm run check && npm run test`

### Anti-Patterns to Avoid

- Do **NOT** make email association mandatory for login or onboarding.
- Do **NOT** store verification tokens in plaintext.
- Do **NOT** allow token reuse after successful verification.
- Do **NOT** log raw email verification links/tokens/private key payloads.
- Do **NOT** add catch-all success fallbacks when SMTP fails.
- Do **NOT** break existing challenge-response auth flows from Stories 2.2/2.5.

### Previous Story Intelligence

From Story 2.5 and current identity implementation:

- `identityState` already models edge states (`identityCorrupted`, `identityNotRegistered`, `crossInstanceJoinError`) and should be extended rather than replaced.
- `RecoveryPrompt.svelte` already contains a disabled "Recover via email" action, giving a clear UI entry point for this story.
- `crypto.ts` already exposes `decryptSecretKey()` and strict identity validation helpers; reuse these primitives for recovery-enablement flows.
- Server-side challenge replay/expiry protections are already enforced in `auth_service`; do not weaken or bypass them.

### Git Intelligence

Recent commits show identity/auth/profile work concentrated in existing files:

- `6af96cc` feat: complete story 2.5 cross-instance identity verification
- `8d65d3c` feat(profile): implement story 2.4 profile and avatar management
- `4d0f866` test: add client review follow-up coverage
- `ef8962a` test(client): add identity feature tests, vitest config, and test setup
- `fc14c80` feat(auth): persist identity sessions

Implication: implement Story 2.6 incrementally in current identity/auth surfaces; avoid broad refactors.

### Latest Technical Information

1. `lettre` remains the recommended Rust SMTP library for Tokio-based backends; prefer async transport configuration aligned with current crate docs.
2. OWASP guidance for verification/recovery flows: high-entropy tokens, hashed token storage, strict expiration, one-time-use, and rate limiting.
3. Keep Svelte 5 rune-first patterns (`$state`, DOM attributes like `onclick`) consistent with current client conventions.
4. Maintain existing API boundary conventions (`snake_case` over the wire, typed camelCase mapping in client adapters).

### Project Context Reference

- No `project-context.md` file was discovered via configured pattern `**/project-context.md`.
- Planning artifacts and existing implementation stories are the authoritative context sources.

### Scope Boundaries (Out of Scope for This Story)

- Full email-based identity recovery execution flow (token redemption that restores local identity) is Story 2.7.
- Changes to invite-link membership flows are out of scope.
- P2P/federation networking behavior (Epic 3) is out of scope.

### Story Completion Status

- Story context generated with implementation guardrails and marked `ready-for-dev`.
- Sprint tracking should reflect `2-6-optional-email-association: ready-for-dev`.

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Story 2.6: Optional Email Association]
- [Source: _bmad-output/planning-artifacts/prd.md#Functional Requirements]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Journey 6: Identity Recovery]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Effortless Interactions]
- [Source: _bmad-output/planning-artifacts/architecture.md#Authentication & Security]
- [Source: server/src/handlers/auth.rs]
- [Source: server/src/services/auth_service.rs]
- [Source: client/src/lib/features/identity/crypto.ts]
- [Source: client/src/lib/features/identity/RecoveryPrompt.svelte]
- [Source: https://cheatsheetseries.owasp.org/cheatsheets/Authentication_Cheat_Sheet.html]
- [Source: https://cheatsheetseries.owasp.org/cheatsheets/Forgot_Password_Cheat_Sheet.html]
- [Source: https://docs.rs/lettre/latest/lettre/]

## Dev Agent Record

### Agent Model Used

GPT-5.3-Codex (Copilot CLI 0.0.416)

### Debug Log References

- `cd server && cargo fmt --check && cargo clippy -- -D warnings && cargo test`
- `cd client && npm run lint && npm run check && npm run test`

### Completion Notes List

- Added `0006_recovery_email_association` migration plus new recovery-email models/services for hashed one-time verification tokens and verified escrow persistence.
- Implemented `POST/GET /api/v1/users/me/recovery-email` and `GET /api/v1/auth/recovery-email/verify` with strict validation, replay/expiry handling, and `{"data": ...}` envelopes.
- Added server-side encryption-at-rest for recovery payloads using a key derived from `email.server_secret` + normalized email, plus metadata for future recovery flow compatibility.
- Added configurable SMTP delivery (`lettre`) with explicit failure propagation and documented full `DISCOOL_EMAIL__...` configuration surface.
- Extended client identity API/types/store and Profile Settings UI for optional recovery email initiation/status; wired `decryptSecretKey()` only on explicit recovery enable action.
- Enabled `RecoveryPrompt` recovery CTA handler and added a subtle post-session “set up recovery email” nudge on the authenticated home view (outside onboarding).
- Added/updated server and client automated tests for success/validation/replay/expiry behaviors and new UX/API mappings.
- Follow-up review hardening: preserved initiation-rate history (no retry bypass), added explicit verification-attempt tracking/rate-limiting, and fixed client recovery-loading cleanup on decrypt failures.

### File List

- `server/migrations/0006_recovery_email_association.sql`
- `server/migrations/0007_email_verification_attempts.sql`
- `server/src/models/recovery_email.rs`
- `server/src/models/mod.rs`
- `server/src/services/recovery_email_service.rs`
- `server/src/services/email_service.rs`
- `server/src/services/mod.rs`
- `server/src/config/settings.rs`
- `server/src/config/mod.rs`
- `server/src/handlers/mod.rs`
- `server/src/handlers/users.rs`
- `server/src/handlers/auth.rs`
- `server/Cargo.toml`
- `server/Cargo.lock`
- `client/src/lib/features/identity/types.ts`
- `client/src/lib/features/identity/identityApi.ts`
- `client/src/lib/features/identity/identityStore.svelte.ts`
- `client/src/lib/features/identity/ProfileSettingsView.svelte`
- `client/src/lib/features/identity/RecoveryPrompt.svelte`
- `client/src/App.svelte`
- `client/src/lib/features/identity/identityApi.test.ts`
- `client/src/lib/features/identity/identityStore.test.ts`
- `client/src/lib/features/identity/ProfileSettingsView.test.ts`
- `client/src/lib/features/identity/RecoveryPrompt.test.ts`
- `config.example.toml`
- `_bmad-output/implementation-artifacts/2-6-optional-email-association.md`
- `_bmad-output/implementation-artifacts/sprint-status.yaml`

### Change Log

- 2026-02-25: Implemented Story 2.6 optional email association end-to-end (server persistence/API/verification, SMTP integration, client settings + prompt updates, and automated tests). Status moved to `review`.
- 2026-02-25: Completed adversarial code review follow-ups (rate-limit bypass fixes + decrypt-failure loading fix), re-ran full quality gates, and moved status to `done`.

### Senior Developer Review (AI)

- **Outcome:** Approved after fixes.
- **Findings fixed:**
  1. Repeated start-association retries could bypass hourly limits because pending token rows were deleted and removed from count history.
  2. Verification rate limiting counted only successful consumptions, allowing unlimited invalid-token attempts per IP.
  3. Client recovery-email flow could leave `recoveryEmailLoading` stuck if `decryptSecretKey()` failed before entering the API-call `try/finally`.
- **Verification run:**
  - `cd server && cargo fmt --check && cargo clippy -- -D warnings && cargo test`
  - `cd client && npm run lint && npm run check && npm run test`
