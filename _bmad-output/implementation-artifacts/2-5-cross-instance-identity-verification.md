# Story 2.5: Cross-Instance Identity Verification

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a **user**,
I want to use the same identity across multiple Discool instances,
So that I don't need to create a new account on every instance I join.

## Acceptance Criteria

1. **Given** a user has an identity created on Instance A
   **When** they click an invite link to a guild on Instance B
   **Then** the SPA detects their existing identity in browser storage.

2. **Given** a user has an identity created on Instance A
   **When** they click an invite link to a guild on Instance B
   **Then** the user is shown "Join [Guild Name] as [Username]?" with their avatar (one-click join).

3. **Given** a user has an identity created on Instance A
   **When** they click an invite link to a guild on Instance B
   **Then** Instance B verifies the identity cryptographically using the public key.

4. **Given** a user has an identity created on Instance A
   **When** they click an invite link to a guild on Instance B
   **Then** if the identity is new to Instance B, a user record is created automatically.

5. **Given** a user has an identity created on Instance A
   **When** they click an invite link to a guild on Instance B
   **Then** the user's display name and avatar are consistent across instances.

6. **Given** a user has an identity created on Instance A
   **When** they click an invite link to a guild on Instance B
   **Then** the entire cross-instance join completes in under 10 seconds (existing identity target).

## Tasks / Subtasks

- [x] Task 1: Extend auth challenge flow for cross-instance onboarding context (AC: #1, #3, #4, #6)
  - [x] 1.1 Extend `server/src/identity/challenge.rs` `ChallengeRecord` to optionally carry cross-instance onboarding metadata (username/display/avatar context and mode flag).
  - [x] 1.2 Extend `server/src/handlers/auth.rs` request DTOs with optional cross-instance payloads while preserving existing request shape compatibility.
  - [x] 1.3 In `challenge`, preserve current behavior for local login (404 when identity missing), and add explicit cross-instance mode that can issue a challenge for first-seen DID keys.
  - [x] 1.4 Keep challenge replay/expiry guarantees unchanged (`check_challenge` + `validate_challenge` one-time consumption).

- [x] Task 2: Implement verified auto-provisioning on Instance B (AC: #3, #4, #5)
  - [x] 2.1 In `verify`, when cross-instance mode is active and DID does not exist locally, only create the user **after** successful signature verification.
  - [x] 2.2 Add/extend service helpers in `server/src/services/auth_service.rs` to support "fetch existing or create from verified DID" semantics.
  - [x] 2.3 Handle username collisions deterministically (same DID should reuse the same row; different DID with same username must not overwrite).
  - [x] 2.4 Preserve profile consistency rules from Story 2.4 (`display_name`, `avatar_color`, `avatar_url`) without mutating immutable identity fields (`did_key`, `public_key_multibase`).

- [x] Task 3: Build client join-confirmation flow for cross-instance auth (AC: #1, #2, #3, #6)
  - [x] 3.1 Extend `client/src/lib/features/identity/identityApi.ts` with typed cross-instance challenge/verify helpers (no `any`, keep snake_case wire mapping).
  - [x] 3.2 Add a new identity UI component (for example `CrossInstanceJoinPrompt.svelte`) in `client/src/lib/features/identity/` for "Join [Guild Name] as [Username]?" with avatar preview and one-click confirmation.
  - [x] 3.3 Wire `client/src/App.svelte` and `client/src/lib/features/identity/identityStore.svelte.ts` so existing stored identity auto-detection leads to the confirmation prompt before cross-instance auth.
  - [x] 3.4 Keep existing auth/session flows from Stories 2.2/2.3 intact (auth epoch race guard, unauthorized handler, localStorage session sync).

- [x] Task 4: Ensure cross-instance profile consistency without breaking local profile management (AC: #5)
  - [x] 4.1 Use Story 2.4 profile as source hints for outbound cross-instance onboarding payload (display name/avatar color).
  - [x] 4.2 On successful remote auth, trust and store remote canonical profile values in session state.
  - [x] 4.3 Do not regress current profile endpoints (`GET/PATCH /api/v1/users/me/profile`, `POST/GET /api/v1/users/me/avatar`).

- [x] Task 5: Re-frame forward dependency so Story 2.5 remains independently implementable (AC: #1-#6)
  - [x] 5.1 Keep identity verification/join-auth independent from Epic 4 invite infrastructure by supporting a direct-instance entry path (same auth semantics, no guild membership mutation required).
  - [x] 5.2 Treat invite-link guild-join wiring as integration with Epic 4 stories, while fully completing cross-instance identity verification now.
  - [x] 5.3 Preserve UX copy requirements for unreachable instance and invalid invite states when context is available.

- [x] Task 6: Add focused test coverage and verification checklist (AC: all)
  - [x] 6.1 Server tests in `server/src/handlers/auth.rs` (and service tests as needed):
    - Cross-instance challenge issuance for unknown DID in cross-instance mode.
    - Verify endpoint creates user only after valid signature/challenge.
    - Challenge replay remains rejected.
    - Username conflict behavior is deterministic and safe.
  - [x] 6.2 Client tests in `client/src/lib/features/identity/*.test.ts`:
    - Cross-instance prompt render + one-click path.
    - API wire transforms for cross-instance payloads.
    - Session/profile state remains consistent after remote auth.
  - [x] 6.3 Manual checks:
    - Existing identity from Instance A is recognized on Instance B.
    - Join confirmation shows expected username/avatar.
    - First-time-on-B path auto-provisions user and signs in.
    - End-to-end target stays under 10 seconds for existing identity.

## Dev Notes

### Architecture Compliance

1. **Auth and session model must stay centralized** in existing auth flow (`/api/v1/auth/challenge`, `/api/v1/auth/verify`, `AuthenticatedUser` middleware), not split into parallel ad-hoc auth logic.
2. **API contract consistency is required**: `{"data": ...}` and `{"error": ...}` envelopes, expected HTTP status semantics, and typed snake_case wire fields.
3. **Challenge lifecycle invariants are non-negotiable**: no bypass of challenge expiry/replay protections already enforced in `auth_service`.
4. **Dual DB compatibility remains mandatory** (PostgreSQL + SQLite query branches in handlers/services).

### Technical Requirements (Developer Guardrails)

- Preserve DID and signature validation quality from `server/src/handlers/auth.rs` (`validate_did_key_for_auth`, strict challenge/signature format checks).
- Preserve existing `identityState` safety behaviors: `authEpoch` race protection, cross-tab session sync, unauthorized auto-reauth.
- Any auto-provisioning path must be cryptographically gated (create user only after valid challenge + valid signature).
- Keep username validation semantics aligned with existing rules (`1-32`, alnum/`_`/`-`) and avoid silent coercion.
- Keep `display_name`/`avatar` handling consistent with Story 2.4 (`UserResponse` mapping and profile service normalization).

### API Contracts (Proposed Additive Shape)

`POST /api/v1/auth/challenge`
```json
{
  "did_key": "did:key:z6Mk...",
  "cross_instance": {
    "enabled": true,
    "username": "liam",
    "display_name": "Liam",
    "avatar_color": "#3B82F6"
  }
}
```

`POST /api/v1/auth/verify`
```json
{
  "did_key": "did:key:z6Mk...",
  "challenge": "<64-hex>",
  "signature": "<128-hex>",
  "cross_instance": {
    "enabled": true
  }
}
```

Response shape remains:
```json
{
  "data": {
    "token": "...",
    "expires_at": "...",
    "user": {
      "id": "...",
      "did_key": "...",
      "username": "...",
      "display_name": "...",
      "avatar_color": "#3B82F6",
      "avatar_url": "/api/v1/users/me/avatar",
      "created_at": "..."
    }
  }
}
```

### File Structure Requirements

Expected implementation touch points:

- Server
  - `server/src/identity/challenge.rs`
  - `server/src/handlers/auth.rs`
  - `server/src/services/auth_service.rs`
  - `server/src/models/user.rs` (only if response contract updates are needed)
  - `server/src/handlers/mod.rs` (only if route wiring changes)

- Client
  - `client/src/lib/features/identity/identityApi.ts`
  - `client/src/lib/features/identity/types.ts`
  - `client/src/lib/features/identity/identityStore.svelte.ts`
  - `client/src/App.svelte`
  - `client/src/lib/features/identity/CrossInstanceJoinPrompt.svelte` (new, if adopted)

- Tests
  - `server/src/handlers/auth.rs` (`#[cfg(test)]`)
  - `server/src/services/auth_service.rs` (`#[cfg(test)]`) as needed
  - `client/src/lib/features/identity/*.test.ts`

### Testing Requirements

Run existing quality gates after implementation:

- `cd server && cargo fmt --check && cargo clippy -- -D warnings && cargo test`
- `cd client && npm run lint && npm run check && npm run test`

### Anti-Patterns to Avoid

- Do **NOT** auto-create users on cross-instance path without successful signature verification.
- Do **NOT** break current local-login behavior (`NOT_FOUND` handling for non-cross-instance flow).
- Do **NOT** introduce broad catch-all error swallowing in identity/auth paths.
- Do **NOT** change `did_key` semantics or parse DID formats loosely.
- Do **NOT** fork auth/session state into duplicate stores or duplicate API clients.
- Do **NOT** tie completion of this story to Epic 4 route/membership data availability.

### Previous Story Intelligence

From Story 2.4 (`_bmad-output/implementation-artifacts/2-4-display-name-and-avatar-management.md`):

- Profile APIs (`/api/v1/users/me/profile`, `/api/v1/users/me/avatar`) and `UserResponse` mapping now define canonical display/ avatar behavior.
- `identityState.saveProfile()` already updates session state immediately after successful profile mutations.
- Server-side profile/avatar validation and storage cleanup behavior are already established; reuse these patterns instead of re-implementing profile logic in auth handlers.

### Git Intelligence

Recent commits show where identity/auth patterns are currently concentrated:

- `8d65d3c` feat(profile): Story 2.4 profile/avatar implementation (handlers/users, services/user_profile_service, client identity/profile APIs).
- `4d0f866` test: expanded client review-follow-up coverage.
- `ef8962a` test(client): vitest setup and identity test baseline.
- `fc14c80` feat(auth): session persistence and identity recovery prompts.
- `0a3b9db` feat(auth): challenge-response, middleware, and identity flow foundations.

Implication: keep Story 2.5 incremental over existing `auth.rs` + `identityStore.svelte.ts` patterns; avoid broad refactors.

### Latest Technical Information

1. **Axum 0.8.x baseline remains valid** for this repo (`Cargo.toml` uses `axum = "0.8"`); keep middleware/extractor-first patterns aligned with Axum docs.
2. **`@noble/ed25519` recommends async methods by default** (`keygenAsync`, `signAsync`, `verifyAsync`) and `Uint8Array` payloads; current client code already follows this pattern.
3. **Svelte 5 migration guidance confirms rune-first patterns** (`$state`, `$derived`, `$effect`) and direct DOM event attributes (`onclick`) as the modern path.
4. **Web Crypto API remains preferred for key material handling** (`CryptoKey`, non-extractable where possible); existing IndexedDB + WebCrypto design in `crypto.ts` should be preserved.

### Project Context Reference

- No `project-context.md` file was discovered via configured pattern `**/project-context.md`.
- Planning artifacts and existing implementation stories are the authoritative context sources.

### Scope Boundaries (Out of Scope for This Story)

- Guild membership mutation and invite lifecycle persistence from Epic 4 (invite tables, guild-join persistence) are not required to complete core cross-instance identity verification.
- Email association/recovery flows remain in Stories 2.6 and 2.7.
- Full P2P discovery and federation transport concerns stay in Epic 3.

### Story Completion Status

- Status updated to `review` after implementation, tests, and quality-gate validation.
- Story tasks/subtasks completed with additive cross-instance challenge/verify support.

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Epic 2] — Epic scope, FR1–FR7 mapping.
- [Source: _bmad-output/planning-artifacts/epics.md#Story 2.5] — Story statement + AC baseline.
- [Source: _bmad-output/planning-artifacts/implementation-readiness-report-2026-02-17.md#Epic 2] — Story 2.5 forward dependency and remediation guidance.
- [Source: _bmad-output/planning-artifacts/architecture.md#Cross-Cutting Concerns Identified] — identity verification and P2P constraints.
- [Source: _bmad-output/planning-artifacts/architecture.md#Authentication & Security] — challenge-response/session requirements.
- [Source: _bmad-output/planning-artifacts/prd.md#Functional Requirements] — FR3 and FR7 requirements.
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Flow: Returning user clicking an invite link (different guild)] — one-click join UX and <10s target.
- [Source: server/src/handlers/auth.rs] — existing challenge/verify behavior and validation constraints.
- [Source: client/src/lib/features/identity/identityStore.svelte.ts] — identity/session lifecycle and race-safety patterns.
- [Source: https://docs.rs/crate/axum/latest] — Axum 0.8 documentation baseline.
- [Source: https://github.com/paulmillr/noble-ed25519] — noble-ed25519 async API and security posture.
- [Source: https://svelte.dev/docs/svelte/v5-migration-guide] — Svelte 5 rune/event guidance.

## Dev Agent Record

### Agent Model Used

GPT-5.3-Codex (Copilot CLI 0.0.416)

### Debug Log References

- `cd server && cargo fmt --check && cargo clippy -- -D warnings && cargo test`
- `cd client && npm run lint && npm run check && npm run test`

### Completion Notes List

- Added optional cross-instance auth payloads and challenge metadata while preserving local 404 behavior for unknown identities.
- Implemented cryptographically gated cross-instance auto-provisioning with deterministic username conflict handling and DID-stable reuse.
- Added a one-click `CrossInstanceJoinPrompt` flow and wired identity store/app state for cross-instance confirmation and session persistence.
- Added/updated server and client tests for cross-instance challenge issuance, signature-gated provisioning, deterministic conflict behavior, API wire mapping, and prompt interaction.
- Validated full quality gates:
  - `cd server && cargo fmt --check && cargo clippy -- -D warnings && cargo test`
  - `cd client && npm run lint && npm run check && npm run test`

### File List

- `server/src/identity/challenge.rs`
- `server/src/services/auth_service.rs`
- `server/src/handlers/auth.rs`
- `client/src/lib/features/identity/types.ts`
- `client/src/lib/features/identity/identityApi.ts`
- `client/src/lib/features/identity/identityStore.svelte.ts`
- `client/src/App.svelte`
- `client/src/lib/features/identity/CrossInstanceJoinPrompt.svelte`
- `client/src/lib/features/identity/CrossInstanceJoinPrompt.test.ts`
- `client/src/lib/features/identity/identityApi.test.ts`
- `client/src/lib/features/identity/identityStore.test.ts`
- `_bmad-output/implementation-artifacts/2-5-cross-instance-identity-verification.md`
- `_bmad-output/implementation-artifacts/sprint-status.yaml`

### Change Log

- 2026-02-25: Implemented Story 2.5 cross-instance identity verification end-to-end across server/client, added tests, and moved story to `review`.
- 2026-02-25: Senior developer adversarial review completed in YOLO mode; no actionable HIGH/MEDIUM findings remained after verification and quality-gate rerun. Story moved to `done`.

## Senior Developer Review (AI)

### Reviewer

Darko (GPT-5.3-Codex)

### Outcome

✅ **Approved** — no unresolved HIGH or MEDIUM issues.

### Findings Summary

- Git vs story File List: no source-code discrepancies (expected `_bmad-output` artifacts excluded from code review scope).
- Acceptance Criteria audit: AC1–AC6 verified as implemented in server/client flow and tests.
- Security/quality audit: cryptographic gating, challenge replay protections, and typed API mappings are in place.
- Validation rerun passed:
  - `cd client && npm run lint && npm run check && npm run test`
  - `cd server && cargo fmt --check && cargo clippy -- -D warnings && cargo test`
