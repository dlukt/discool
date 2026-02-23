# Story 2.2: Challenge-Response Authentication and Session Management

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a **user**,
I want to authenticate to an instance using my cryptographic identity,
So that the server knows I am who I claim to be without a password.

## Acceptance Criteria

1. **Given** a user has an identity stored in their browser (from Story 2.1)
   **When** they connect to a Discool instance
   **Then** the server issues a random challenge via `POST /api/v1/auth/challenge`
   **And** the client signs the challenge with the user's private key (Ed25519)
   **And** the server verifies the signature against the user's registered public key
   **And** upon successful verification, the server creates a session and returns a session token

2. **Given** a session has been created
   **Then** the session is stored server-side in the database (`sessions` table migration in this story)
   **And** the session token is a cryptographically random string (UUID v4)
   **And** the session record includes: `id`, `user_id`, `token`, `created_at`, `expires_at`, `last_active_at`

3. **Given** a valid session token exists
   **Then** the token is used for all subsequent REST API calls via `Authorization: Bearer <token>` header
   **And** the token will be used for WebSocket connection authentication (implemented in Epic 6)
   **And** unauthenticated requests to protected endpoints return 401 Unauthorized

4. **Given** an active session exists
   **Then** sessions have a configurable TTL (default: 7 days, configured in TOML as `auth.session_ttl_hours`)
   **And** the `last_active_at` timestamp is refreshed on authenticated API calls (not on every request -- debounced to at most once per minute)
   **And** expired sessions are rejected with 401 and the client must re-authenticate

5. **Given** the user wants to log out
   **When** they call `DELETE /api/v1/auth/logout`
   **Then** the session record is deleted from the database
   **And** the token is no longer valid for API calls

6. **Given** the entire authentication flow
   **Then** all authentication happens over TLS 1.3+ (NFR11) -- enforced at transport layer, not application layer
   **And** challenges expire after 5 minutes to prevent replay attacks
   **And** each challenge can only be used once (consumed on verification)

7. **Given** a user with an existing identity opens the SPA
   **When** the identity is found in IndexedDB
   **Then** challenge-response authentication happens automatically in the background
   **And** no login screen or manual action is required
   **And** the user sees a brief "Signing in..." state (skeleton loader, not a blocking modal)
   **And** if auto-auth fails (e.g., identity not registered on this instance), a clear message is shown

## Tasks / Subtasks

- [x] Task 1: Create database migration for `sessions` table (AC: #2)
  - [x] 1.1 Create `server/migrations/0004_create_sessions.sql` with `sessions` table: `id TEXT PRIMARY KEY`, `user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE`, `token TEXT NOT NULL UNIQUE`, `created_at TEXT NOT NULL`, `expires_at TEXT NOT NULL`, `last_active_at TEXT NOT NULL`
  - [x] 1.2 Add indexes: `idx_sessions_token` on `token`, `idx_sessions_user_id` on `user_id`, `idx_sessions_expires_at` on `expires_at`
  - [x] 1.3 Verify migration runs on both SQLite and PostgreSQL (`cargo test`)

- [x] Task 2: Add `Unauthorized` variant to `AppError` (AC: #3)
  - [x] 2.1 Add `Unauthorized(String)` variant to `AppError` enum in `server/src/error.rs`
  - [x] 2.2 Map `Unauthorized` to HTTP 401 with `{ "error": { "code": "UNAUTHORIZED", "message": "...", "details": {} } }`

- [x] Task 3: Add server dependencies (AC: #1, #2)
  - [x] 3.1 Add `rand = "0.9"` to `server/Cargo.toml` (for secure challenge nonce generation)
  - [x] 3.2 Add `dashmap = "6"` to `server/Cargo.toml` (for in-memory challenge nonce store)
  - [x] 3.3 Verify `ed25519-dalek` v2 with `alloc` feature is already present (it is -- Story 2.1 added it)

- [x] Task 4: Create challenge module -- nonce generation and signature verification (AC: #1, #6)
  - [x] 4.1 Create `server/src/identity/challenge.rs`:
    - `ChallengeRecord { challenge: String, did_key: String, created_at: Instant }` -- stored in memory
    - `generate_challenge() -> String` -- returns 32 random bytes hex-encoded (64 chars) using `rand::Rng::random::<[u8; 32]>()`
    - `verify_signature(public_key_bytes: &[u8; 32], challenge: &str, signature_bytes: &[u8; 64]) -> Result<(), VerifyError>` -- constructs `ed25519_dalek::VerifyingKey` + `Signature`, calls `.verify(challenge.as_bytes(), &signature)`
    - `VerifyError` enum: `InvalidPublicKey`, `InvalidSignature`
    - Unit tests: valid signature (use `SigningKey::from_bytes` to create a known keypair, sign, verify), invalid signature (wrong challenge), invalid public key
  - [x] 4.2 Register `challenge` module in `server/src/identity/mod.rs`

- [x] Task 5: Add challenge store and session config to `AppState` (AC: #1, #4, #6)
  - [x] 5.1 Update `AppState` in `server/src/lib.rs`:
    - Add `pub challenges: Arc<DashMap<String, ChallengeRecord>>` -- keyed by DID, one active challenge per DID
    - Import `DashMap` from `dashmap` crate
  - [x] 5.2 Add `auth` section to `Config` struct in `server/src/config/settings.rs`:
    - `session_ttl_hours: u64` (default: 168 = 7 days)
    - `challenge_ttl_seconds: u64` (default: 300 = 5 minutes)
  - [x] 5.3 Update `AppState` construction in `server/src/main.rs` to initialize `challenges: Arc::new(DashMap::new())`
  - [x] 5.4 Add `[auth]` section to `config.example.toml` with commented-out defaults

- [x] Task 6: Create session model (AC: #2)
  - [x] 6.1 Create `server/src/models/session.rs`:
    - `Session` struct: `id: String`, `user_id: String`, `token: String`, `created_at: String`, `expires_at: String`, `last_active_at: String` -- derive `sqlx::FromRow`
    - `SessionResponse` struct (for API): `token: String`, `expires_at: String`, `user: UserResponse`
  - [x] 6.2 Register `session` module in `server/src/models/mod.rs`

- [x] Task 7: Create auth service -- session CRUD and challenge lifecycle (AC: #1, #2, #4, #5, #6)
  - [x] 7.1 Create `server/src/services/mod.rs` exporting `auth_service` submodule
  - [x] 7.2 Create `server/src/services/auth_service.rs`:
    - `create_challenge(challenges: &DashMap<String, ChallengeRecord>, did_key: &str) -> String` -- generates challenge, stores in DashMap keyed by DID (replaces any existing challenge for this DID), returns challenge string
    - `validate_challenge(challenges: &DashMap<String, ChallengeRecord>, did_key: &str, challenge: &str, ttl_seconds: u64) -> Result<(), AuthError>` -- checks DID has a pending challenge, challenge matches, not expired; removes challenge from DashMap on success (one-time use)
    - `create_session(pool: &DbPool, user_id: &str, ttl_hours: u64) -> Result<Session, AppError>` -- generates UUID token, inserts into DB with `created_at`, `expires_at` (`created_at + ttl_hours`), `last_active_at` = `created_at`
    - `validate_session(pool: &DbPool, token: &str) -> Result<(Session, User), AppError>` -- looks up session by token, checks `expires_at > now`, returns session + user (via JOIN or separate query)
    - `refresh_session(pool: &DbPool, session_id: &str) -> Result<(), AppError>` -- updates `last_active_at` to now
    - `delete_session(pool: &DbPool, token: &str) -> Result<(), AppError>` -- deletes session by token
    - `fetch_user_by_did(pool: &DbPool, did_key: &str) -> Result<User, AppError>` -- looks up user by `did_key`, returns `NotFound` if not registered
    - `AuthError` enum: `ChallengeNotFound`, `ChallengeMismatch`, `ChallengeExpired`
  - [x] 7.3 Register `services` module in `server/src/lib.rs`
  - [x] 7.4 Unit tests for challenge lifecycle: create, validate, expire, one-time use, wrong DID

- [x] Task 8: Extend auth handler -- challenge and verify endpoints (AC: #1, #3, #5, #6)
  - [x] 8.1 Add to `server/src/handlers/auth.rs`:
    - `POST /api/v1/auth/challenge` handler:
      - Wire type: `ChallengeRequest { did_key: String }`
      - Validates DID format (same checks as register)
      - Verifies user exists on this instance (calls `fetch_user_by_did`)
      - Generates challenge via `create_challenge()`
      - Returns 200 with `{ "data": { "challenge": "<hex_string>", "expires_in": 300 } }`
    - `POST /api/v1/auth/verify` handler:
      - Wire type: `VerifyRequest { did_key: String, challenge: String, signature: String }` -- signature is hex-encoded 64 bytes
      - Validates DID, looks up user, extracts public key from DID
      - Validates challenge via `validate_challenge()` (checks match + expiry + consumes nonce)
      - Hex-decodes signature, verifies via `challenge::verify_signature()`
      - On success: creates session via `create_session()`, returns 200 with `{ "data": { "token": "...", "expires_at": "...", "user": { ... } } }`
      - On failure: returns 401 Unauthorized with clear error message
    - `DELETE /api/v1/auth/logout` handler:
      - Requires auth middleware (Bearer token)
      - Deletes session via `delete_session()`
      - Returns 204 No Content
  - [x] 8.2 Register new routes in `server/src/handlers/mod.rs`:
    - `POST /api/v1/auth/challenge => auth::challenge`
    - `POST /api/v1/auth/verify => auth::verify`
    - `DELETE /api/v1/auth/logout => auth::logout` (requires auth)
  - [x] 8.3 Unit tests: successful challenge-verify flow, expired challenge, wrong signature, unregistered DID, replay (reuse challenge), logout

- [x] Task 9: Create auth middleware extractor (AC: #3, #4)
  - [x] 9.1 Create `server/src/middleware/mod.rs` exporting `auth` submodule
  - [x] 9.2 Create `server/src/middleware/auth.rs`:
    - `AuthenticatedUser` struct: `pub user_id: String`, `pub session_id: String`, `pub username: String`, `pub did_key: String`
    - Implement `FromRequestParts<AppState>` for `AuthenticatedUser`:
      - Extracts `Authorization: Bearer <token>` header
      - Calls `validate_session()` to verify token
      - Refreshes `last_active_at` (debounced: only if last refresh was >60 seconds ago)
      - Returns `AuthenticatedUser` on success, 401 on failure
    - No separate middleware layer needed -- Axum extractor pattern is idiomatic
  - [x] 9.3 Register `middleware` module in `server/src/lib.rs`
  - [x] 9.4 Wire `AuthenticatedUser` into the existing `GET /api/v1/admin/health` endpoint as a demonstration of protected routes (currently unprotected)
  - [x] 9.5 Unit tests: valid token extracts user, expired token returns 401, missing header returns 401, malformed header returns 401

- [x] Task 10: Add `signChallenge` to client crypto module (AC: #1, #7)
  - [x] 10.1 Add to `client/src/lib/features/identity/crypto.ts`:
    - `signChallenge(challengeHex: string): Promise<string>` -- decrypts secret key via `decryptSecretKey()`, converts hex challenge to `Uint8Array`, calls `ed.signAsync(challengeBytes, secretKey)`, zeros secretKey, returns hex-encoded 64-byte signature
    - Internal helper: `hexToBytes(hex: string): Uint8Array` and `bytesToHex(bytes: Uint8Array): string`
    - Export `signChallenge` from the module
  - [x] 10.2 The `decryptSecretKey()` function already exists and returns the raw 32-byte secret key -- use it directly

- [x] Task 11: Add challenge and verify API functions to client (AC: #1, #7)
  - [x] 11.1 Add to `client/src/lib/features/identity/identityApi.ts`:
    - Wire types (internal):
      - `ChallengeRequestWire { did_key: string }`
      - `ChallengeResponseWire { challenge: string, expires_in: number }`
      - `VerifyRequestWire { did_key: string, challenge: string, signature: string }`
      - `VerifyResponseWire { token: string, expires_at: string, user: RegisteredUserWire }`
    - Public types:
      - `AuthSession { token: string, expiresAt: string, user: RegisteredUser }`
    - `requestChallenge(didKey: string): Promise<{ challenge: string; expiresIn: number }>` -- calls `POST /api/v1/auth/challenge`
    - `verifyChallenge(didKey: string, challenge: string, signature: string): Promise<AuthSession>` -- calls `POST /api/v1/auth/verify`, maps response
    - `logout(token: string): Promise<void>` -- calls `DELETE /api/v1/auth/logout` with `Authorization: Bearer <token>` header
  - [x] 11.2 Add `AuthSession` type to `client/src/lib/features/identity/types.ts`

- [x] Task 12: Extend identity store with authentication state (AC: #7)
  - [x] 12.1 Update `client/src/lib/features/identity/identityStore.svelte.ts`:
    - Add state fields: `session: AuthSession | null`, `authenticating: boolean`, `authError: string | null`
    - Add `authenticate(): Promise<void>` action:
      1. Set `authenticating = true`, `authError = null`
      2. Call `requestChallenge(identity.didKey)` to get challenge
      3. Call `signChallenge(challenge)` to sign with private key
      4. Call `verifyChallenge(identity.didKey, challenge, signature)` to get session token
      5. Set `session = result`, `authenticating = false`
      6. Store session token in `sessionStorage` (not `localStorage` -- cleared on tab close for security)
    - Add `logout(): Promise<void>` action: calls `identityApi.logout()`, clears `session`, clears `sessionStorage`
    - Add `restoreSession(): Promise<boolean>` action: checks `sessionStorage` for existing token, validates against server via a lightweight endpoint (or just proceeds and lets the next API call fail)
    - Update `initialize()` to call `restoreSession()` after loading identity, then `authenticate()` if no valid session exists
  - [x] 12.2 The `register()` action should call `authenticate()` after successful registration to immediately get a session token

- [x] Task 13: Inject auth header into API client (AC: #3)
  - [x] 13.1 Update `client/src/lib/api.ts`:
    - Add module-level `let sessionToken: string | null = null`
    - Add `setSessionToken(token: string | null)` export to set/clear the token
    - In `apiFetch()`: if `sessionToken` is set and no `Authorization` header exists in `init`, inject `Authorization: Bearer <token>` automatically
    - This ensures all future API calls (guild creation, channel management, etc.) are authenticated without per-call changes
  - [x] 13.2 Wire `identityStore` to call `setSessionToken()` when session is set/cleared

- [x] Task 14: Update App.svelte state machine (AC: #7)
  - [x] 14.1 Update `client/src/App.svelte`:
    - Current flow: `loading | error | !initialized | !identity | identity → main`
    - New flow: `loading | error | !initialized | !identity | identity + authenticating | identity + authError | identity + session → main`
    - When `identityState.identity` exists but `identityState.session` is null and `identityState.authenticating` is true: show skeleton/loading state with "Signing in..." text
    - When `identityState.authError` is not null: show error message with "Try again" button
    - When `identityState.session` is set: show main layout (existing behavior)
    - Auto-authentication should happen transparently -- no manual "Sign In" button needed
  - [x] 14.2 No UI changes to `LoginView.svelte` -- it remains the registration form. After registration, `authenticate()` is called automatically.

- [x] Task 15: Server integration tests (AC: #1, #2, #3, #4, #5, #6)
  - [x] 15.1 Add to `server/tests/` (create `test_auth_session.rs` or extend existing test file):
    - Test: POST challenge for registered DID → 200 with challenge string
    - Test: POST challenge for unregistered DID → 404
    - Test: POST verify with valid signature → 200 with token + user
    - Test: POST verify with invalid signature → 401
    - Test: POST verify with expired challenge → 401
    - Test: POST verify reusing consumed challenge → 401 (replay prevention)
    - Test: Authenticated request with valid token → succeeds
    - Test: Authenticated request with expired session → 401
    - Test: DELETE logout with valid token → 204, subsequent requests → 401
    - Test: Request without Authorization header to protected endpoint → 401
  - [x] 15.2 Use the existing `test_state()` + `did_for_signing_key()` helpers from auth.rs tests; create signing helpers using `ed25519_dalek::SigningKey`

- [x] Task 16: Verify lint + existing tests pass (all ACs)
  - [x] 16.1 `cargo fmt --check && cargo clippy -- -D warnings && cargo test`
  - [x] 16.2 `cd client && npx biome check . && npx svelte-check --tsconfig ./tsconfig.app.json`

## Dev Notes

### Architecture Compliance

This story creates the **second layer of the identity system** and introduces the **services layer** and **auth middleware**, both mandated by the architecture doc but deferred from Story 2.1.

1. **Server `identity/challenge.rs`** -- New file implementing challenge generation and Ed25519 signature verification. Fills the gap in the `identity/` module (Story 2.1 created `did.rs` and `keypair.rs`; `recovery.rs` is Story 2.7).

2. **Server `services/` module** -- NEW module layer. The architecture specifies `services/auth_service.rs` for "Challenge-response, session management." Story 2.1 deferred service creation because handler-to-DB was sufficient. This story introduces it because auth logic is now complex enough: challenge lifecycle, session CRUD, token validation, session refresh debouncing.

3. **Server `middleware/auth.rs`** -- NEW module layer. The architecture specifies `middleware/auth.rs` for "Session token extraction, identity verification." Uses Axum `FromRequestParts` extractor pattern (not a Tower middleware layer) -- more idiomatic for Axum 0.8 and matches how state is accessed in existing handlers.

4. **Server `models/session.rs`** -- New file for session data model. Architecture specifies `models/session.rs` for `Session, SessionToken`.

5. **Client `identityStore.svelte.ts`** -- Extended with authentication flow. Architecture places auth state in both `features/identity/identityStore.svelte.ts` (identity + session) and `stores/authStore.svelte.ts` (global auth). For Story 2.2, we keep everything in `identityStore` since there's no separate global store yet. If the store grows too complex in later stories, extract a separate `authStore`.

6. **Client `api.ts`** -- Extended with session token injection. Architecture specifies `api/client.ts` handles "auth headers, snake-to-camel transform." The token injection point is here, not in each API call.

### Authentication Flow (Detailed)

```
Browser                          Server
  │                                │
  ├─ POST /api/v1/auth/challenge ──►
  │  { "did_key": "did:key:z6Mk..."}│
  │                                │  Verify user exists by DID
  │                                │  Generate 32-byte random challenge
  │                                │  Store in DashMap keyed by DID
  │◄── { "data": { "challenge":   │
  │      "a1b2c3...", "expires_in":│
  │      300 } }                   │
  │                                │
  │  [Client: decrypt secret key   │
  │   from IndexedDB, sign         │
  │   challenge, zero secret key]  │
  │                                │
  ├─ POST /api/v1/auth/verify ────►
  │  { "did_key": "did:key:z6Mk..",│
  │    "challenge": "a1b2c3...",   │
  │    "signature": "d4e5f6..." }  │
  │                                │  Validate challenge (exists, matches, not expired)
  │                                │  Remove challenge from DashMap (one-time use)
  │                                │  Extract public key from DID
  │                                │  Verify Ed25519 signature
  │                                │  Create session in DB
  │◄── { "data": { "token":       │
  │      "uuid-v4", "expires_at":  │
  │      "2026-03-02T...",         │
  │      "user": { ... } } }      │
  │                                │
  │  [Client: store token in       │
  │   sessionStorage, set in       │
  │   api.ts for all future calls] │
  │                                │
  ├─ GET /api/v1/admin/health ────►   (with Authorization: Bearer <token>)
  │                                │  Auth middleware validates token
  │◄── { "data": { ... } }        │
```

### Challenge Nonce Store Design

**Why in-memory `DashMap` instead of DB:**
- Challenges are ephemeral (5-minute TTL) -- no persistence needed across restarts
- DashMap is lock-free concurrent access, ideal for Tokio async context
- One challenge per DID key (new challenge replaces old) prevents nonce accumulation
- If the server restarts, pending challenges expire naturally -- client retries
- For future multi-instance deployments, challenges would move to Redis (same pattern as the architecture's cache abstraction)

**Challenge keying:**
- Keyed by DID (not by challenge string) to ensure one active challenge per identity
- This prevents a client from requesting many challenges and brute-forcing
- When a new challenge is requested for the same DID, the old one is replaced

**Cleanup strategy:**
- Expired challenges are cleaned up lazily (checked at validation time)
- A periodic cleanup task is NOT needed for MVP (DashMap memory is bounded by registered user count)

### Session Token Design

**Token format:** UUID v4 (36 chars with hyphens). Sufficient for session identification:
- 122 bits of randomness (UUID v4 spec)
- Not a JWT -- no client-side decode needed, server is authoritative
- Simple string comparison for validation

**TTL and refresh:**
- Default: 7 days (`auth.session_ttl_hours = 168`)
- `last_active_at` updated on authenticated requests, debounced to once per minute
- Expiry is absolute (`created_at + ttl`), not sliding window -- prevents sessions from lasting forever
- To implement sliding window in the future: update `expires_at` on refresh instead of just `last_active_at`

**Storage (client-side):**
- `sessionStorage` (not `localStorage`) -- automatically cleared when the tab is closed
- This matches the security model: closing the browser requires re-authentication
- The challenge-response is fast (sub-second) so re-auth on new tab is not a UX problem

### Database Schema

Migration `0004_create_sessions.sql`:
```sql
CREATE TABLE IF NOT EXISTS sessions (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    token TEXT NOT NULL UNIQUE,
    created_at TEXT NOT NULL,
    expires_at TEXT NOT NULL,
    last_active_at TEXT NOT NULL,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_sessions_token ON sessions(token);
CREATE INDEX IF NOT EXISTS idx_sessions_user_id ON sessions(user_id);
CREATE INDEX IF NOT EXISTS idx_sessions_expires_at ON sessions(expires_at);
```

- `id`: UUID v4 as TEXT (matches `users.id` pattern)
- `user_id`: FK to `users.id`, CASCADE delete (if user is deleted, all their sessions go away)
- `token`: UUID v4 as TEXT, UNIQUE index for O(1) lookup
- `expires_at`: RFC 3339 TEXT -- compared as string in SQL (ISO 8601 strings sort lexicographically)
- `last_active_at`: RFC 3339 TEXT -- for session refresh tracking
- SQLite note: `FOREIGN KEY` requires `PRAGMA foreign_keys = ON` per connection. The existing SQLite pool setup should already handle this; verify in `db/pool.rs`. If not, add `?mode=rwc&_foreign_keys=on` to the SQLite URL or run the pragma after connection.

### API Contracts

```
POST /api/v1/auth/challenge
Content-Type: application/json

Request body:
{
  "did_key": "did:key:z6MkhaXgBZDvotDkL5257faiztiGiC2QtKLGpbnnEGta2doK"
}

Success (200):
{
  "data": {
    "challenge": "a1b2c3d4e5f6...64hex_chars...",
    "expires_in": 300
  }
}

Not Found (404 -- DID not registered):
{ "error": { "code": "NOT_FOUND", "message": "Identity not found on this instance" } }

Validation Error (422):
{ "error": { "code": "VALIDATION_ERROR", "message": "Invalid DID format" } }
```

```
POST /api/v1/auth/verify
Content-Type: application/json

Request body:
{
  "did_key": "did:key:z6MkhaXgBZDvotDkL5257faiztiGiC2QtKLGpbnnEGta2doK",
  "challenge": "a1b2c3d4e5f6...64hex_chars...",
  "signature": "e7f8a9b0c1d2...128hex_chars..."
}

Success (200):
{
  "data": {
    "token": "550e8400-e29b-41d4-a716-446655440000",
    "expires_at": "2026-03-02T12:00:00Z",
    "user": {
      "id": "123e4567-e89b-12d3-a456-426614174000",
      "did_key": "did:key:z6MkhaXgBZDvotDkL5257faiztiGiC2QtKLGpbnnEGta2doK",
      "username": "liam",
      "avatar_color": "#3B82F6",
      "created_at": "2026-02-19T12:00:00Z"
    }
  }
}

Unauthorized (401 -- bad signature):
{ "error": { "code": "UNAUTHORIZED", "message": "Invalid signature" } }

Unauthorized (401 -- expired/missing challenge):
{ "error": { "code": "UNAUTHORIZED", "message": "Challenge expired or not found" } }
```

```
DELETE /api/v1/auth/logout
Authorization: Bearer <token>

Success (204): No body

Unauthorized (401 -- invalid token):
{ "error": { "code": "UNAUTHORIZED", "message": "Invalid or expired session" } }
```

### Ed25519 Signature Specifics

**Client signing (`@noble/ed25519` v3):**
```typescript
import * as ed from '@noble/ed25519';
// secretKey is 32 bytes (seed), signAsync returns 64-byte signature
const signature = await ed.signAsync(challengeBytes, secretKey);
// signature is Uint8Array(64)
```

`ed.signAsync()` internally hashes the message with SHA-512 (via `crypto.subtle.digest`). The challenge bytes should be the raw hex challenge string encoded as UTF-8 bytes (i.e., sign the hex string directly, not the decoded binary). This avoids encoding ambiguity between client and server.

**Server verification (`ed25519-dalek` v2):**
```rust
use ed25519_dalek::{Signature, VerifyingKey, Verifier};
let vk = VerifyingKey::from_bytes(&public_key_bytes)?;
let sig = Signature::from_bytes(&signature_bytes);
vk.verify(challenge_str.as_bytes(), &sig)?;
```

**Critical: both sides must sign/verify the same bytes.** The convention:
- Challenge is a hex string (e.g., `"a1b2c3..."`)
- Client signs `challengeHex.as_bytes()` (UTF-8 encoding of the hex string)
- Server verifies against `challenge_str.as_bytes()` (same UTF-8 bytes)
- This means the client passes `new TextEncoder().encode(challengeHex)` to `ed.signAsync()`

### App.svelte State Machine Update

Current flow (from Story 2.1):
```
loading → error | !initialized → SetupPage | initialized + identityLoading → spinner | initialized + !identity → LoginView | initialized + identity → MainLayout
```

Updated flow:
```
loading → error | !initialized → SetupPage | initialized + identityLoading → spinner
| initialized + !identity → LoginView
| initialized + identity + authenticating → "Signing in..." skeleton
| initialized + identity + authError → error with retry
| initialized + identity + session → MainLayout
```

The auto-authentication is triggered inside `identityState.initialize()`:
1. Load stored identity from IndexedDB
2. If identity exists: check sessionStorage for existing token
3. If token exists: set session (optimistic -- will fail on next API call if expired)
4. If no token: call `authenticate()` automatically (challenge-response)

### Svelte 5 Patterns (Critical)

Continue using Svelte 5 runes as established in Story 2.1:
- State: `let session = $state<AuthSession | null>(null)`
- Derived: `let isAuthenticated = $derived(session !== null)`
- Props: `let { onsuccess }: Props = $props()`
- No legacy patterns (`writable`, `createEventDispatcher`, `$:`)

### Existing Code Reuse

- **`identity::did::parse_did_key()`** from Story 2.1: Use to extract public key bytes from DID in the verify handler
- **`identity::keypair::validate_ed25519_public_key()`** from Story 2.1: Optionally use for key validation before signature verification (not strictly needed since `VerifyingKey::from_bytes` already validates)
- **`apiFetch<T>()`** from `$lib/api.ts`: Use for all new API calls
- **`ApiError`** from `$lib/api.ts`: Handle 401/404 errors
- **`UserResponse`** struct from `models/user.rs`: Reuse in session response
- **`did_for_signing_key()` test helper** from `handlers/auth.rs` tests: Reuse in session integration tests
- **`test_state()`** helper from auth.rs tests: Reuse (may need to be extracted to a shared test utility)
- **DbPool dual-match pattern**: Follow the exact same `match pool { Postgres(p) => ..., Sqlite(p) => ... }` pattern for all new DB queries
- **RFC 3339 timestamps**: `Utc::now().to_rfc3339()` for all timestamps (matches existing pattern)
- **`cn()` utility** from `$lib/utils.ts`: For any conditional CSS in updated components

### Files to Create

| File | Purpose |
|---|---|
| `server/migrations/0004_create_sessions.sql` | Sessions table migration |
| `server/src/identity/challenge.rs` | Challenge generation + Ed25519 signature verification |
| `server/src/models/session.rs` | Session struct + sqlx::FromRow |
| `server/src/services/mod.rs` | Services module root |
| `server/src/services/auth_service.rs` | Challenge lifecycle, session CRUD, user lookup |
| `server/src/middleware/mod.rs` | Middleware module root |
| `server/src/middleware/auth.rs` | AuthenticatedUser extractor (FromRequestParts) |

### Files to Modify

| File | Change |
|---|---|
| `server/Cargo.toml` | Add `rand`, `dashmap` dependencies |
| `server/src/lib.rs` | Add `services`, `middleware` modules; extend `AppState` with `challenges` |
| `server/src/main.rs` | Initialize `challenges` DashMap in AppState construction |
| `server/src/error.rs` | Add `Unauthorized` variant |
| `server/src/identity/mod.rs` | Add `challenge` module |
| `server/src/models/mod.rs` | Add `session` module |
| `server/src/config/settings.rs` | Add `auth` config section |
| `server/src/handlers/auth.rs` | Add `challenge`, `verify`, `logout` handlers |
| `server/src/handlers/mod.rs` | Register new auth routes, add auth middleware to protected routes |
| `config.example.toml` | Add `[auth]` section with defaults |
| `client/src/lib/features/identity/crypto.ts` | Add `signChallenge()`, `hexToBytes()`, `bytesToHex()` |
| `client/src/lib/features/identity/identityApi.ts` | Add `requestChallenge()`, `verifyChallenge()`, `logout()` |
| `client/src/lib/features/identity/identityStore.svelte.ts` | Add `session`, `authenticating`, `authError` state; `authenticate()`, `logout()`, `restoreSession()` actions |
| `client/src/lib/features/identity/types.ts` | Add `AuthSession` type |
| `client/src/lib/api.ts` | Add `setSessionToken()` and auto-inject Authorization header |
| `client/src/App.svelte` | Add authenticating/authError states to the state machine |

### Project Structure Notes

- `server/src/services/` is a NEW top-level module -- matches architecture's layered pattern (handlers -> services -> models). Story 2.1 explicitly deferred this; Story 2.2 introduces it.
- `server/src/middleware/` is a NEW top-level module -- matches architecture spec at `server/src/middleware/auth.rs`.
- Migration numbering continues with `0004_` (four-digit prefix matching existing `0001_`, `0002_`, `0003_`).
- All new Rust files follow existing naming: `snake_case.rs`, modules via `mod.rs`.
- No new client directories needed -- all files go into existing `features/identity/` or are modifications to existing files.

### Testing Requirements

**Server unit tests** (in each module):
- `identity::challenge` -- generate challenge (is 64 hex chars), verify valid signature, reject invalid signature, reject wrong challenge text
- `services::auth_service` -- create challenge + validate flow, challenge expiry, challenge one-time use, session creation, session validation, session expiry
- `handlers::auth` -- challenge endpoint (valid DID, unknown DID), verify endpoint (valid flow, bad signature, expired challenge), logout

**Server integration tests** (in `server/tests/`):
- Full flow: register → challenge → sign → verify → authenticated request → logout
- Expired session returns 401
- Missing Authorization header returns 401
- Malformed Bearer token returns 401
- Challenge replay (reuse same challenge) returns 401
- Multiple sessions for same user (allowed -- different tabs/devices)
- Logout invalidates only the specific token, not other sessions

**Client tests** (if Vitest is configured -- optional for this story):
- `crypto.ts` -- `signChallenge()` produces valid hex signature, `hexToBytes/bytesToHex` round-trip
- `identityApi.ts` -- wire type mapping correctness (mock apiFetch)

**Linting:** `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test`, `npx biome check .`, `npx svelte-check --tsconfig ./tsconfig.app.json`

### Anti-Patterns to Avoid

- Do NOT use JWTs for session tokens -- the architecture specifies server-side session store (DB-backed). JWTs cannot be revoked without a blocklist, which defeats the purpose.
- Do NOT store session tokens in `localStorage` -- use `sessionStorage` (cleared on tab close) for better security. Persistent sessions (cross-tab) are Story 2.3.
- Do NOT sign the raw binary challenge bytes on the client -- sign the hex string's UTF-8 bytes so both sides agree on what was signed without encoding confusion.
- Do NOT create a Tower middleware layer for auth -- use Axum's `FromRequestParts` extractor. It's more composable and doesn't require wrapping the entire router.
- Do NOT add rate limiting to auth endpoints in this story -- rate limiting middleware is a cross-cutting concern specified for a later epic. The challenge mechanism itself provides replay protection.
- Do NOT add WebSocket authentication in this story -- that is Story 6.1 (WebSocket Gateway). The session token will be used there, but the WS connection logic is not part of this story.
- Do NOT add "remember me" / persistent login -- that is Story 2.3 (Identity Persistence and Auto-Login).
- Do NOT add email-based recovery -- that is Story 2.7.
- Do NOT use `unwrap()` or `expect()` in handler or service code -- use `?` with `AppError`. `unwrap()` is acceptable only in test code.
- Do NOT use `$1`/`$2` placeholders in SQLite queries -- use `?1`/`?2` (match existing pattern in `auth.rs`).
- Do NOT use `CURRENT_TIMESTAMP` in SQL inserts -- use Rust-generated RFC 3339 timestamps (lesson from Story 1.8).
- Do NOT modify the registration handler's behavior -- it stays unauthenticated (new users must register before they can authenticate).
- Do NOT add a `services/` call to the registration handler -- leave it calling DB directly (matches existing pattern; services layer is for new complex logic).

### Previous Story Intelligence

**From Story 2.1 (Client-Side Keypair Generation):**
- `decryptSecretKey()` in `crypto.ts` returns raw `Uint8Array(32)` secret key from IndexedDB -- use this directly for signing
- `loadStoredIdentity()` returns `StoredIdentity | null` with `publicKey: Uint8Array`, `didKey: string` -- use `didKey` for challenge request
- `identityState.initialize()` loads identity from IndexedDB on mount -- extend this to also restore/create session
- Registration handler uses `ON CONFLICT DO NOTHING` + existence checks -- follow same conflict resolution pattern
- Test helper `did_for_signing_key([1u8; 32])` creates a DID from a known secret -- reuse for signing in auth tests
- The `@noble/ed25519` import pattern: `import * as ed from '@noble/ed25519'` -- consistent with `crypto.ts`
- AES-GCM wrapping key is non-extractable -- `decryptSecretKey()` handles all the unwrapping internally
- `secret_key.fill(0)` after use -- zero the key in `signChallenge()` too

**From Story 1.8 (Docker and Deployment Pipeline):**
- SQL inserts must use bound RFC 3339 timestamp parameters -- NOT `CURRENT_TIMESTAMP`
- Integration tests spawn a real server with SQLite in-memory -- same pattern for session tests
- `DbPool` enum with `Postgres`/`Sqlite` arms -- all new queries must dual-path

**From Story 1.5 (First-Run Admin Setup):**
- `instance.rs` uses `ON CONFLICT DO NOTHING` + rows-affected check -- applicable pattern if session creation has unique constraints

**From Story 1.1 (Project Scaffold):**
- `apiFetch<T>()` uses `new Headers(init.headers)` -- custom headers can be passed in `init.headers` without changing `apiFetch` itself
- Vite dev proxy handles `/api/v1/*` routing to Axum -- no proxy changes needed

### Git Intelligence

**Recent commit patterns:**
```
0753378 fix: map internal errors in LoginView, fix base58 edge-case, add DID multibase test
23636a0 Exclude index.html from file path 404 check
b9ca101 Add audit.toml to ignore MySQL-only RSA advisory
8703d6f refactor: simplify nested match in db_size_bytes handler
ff12c54 feat: switch from AnyPool to typed DbPool for Postgres/SQLite
107b620 feat: add Docker and deployment pipeline with multi-db support
```

- `ff12c54`: Important -- `DbPool` is now a typed enum (`Postgres(PgPool)` / `Sqlite(SqlitePool)`), NOT `AnyPool`. All queries use `match pool { ... }`. Follow this pattern exactly.
- `0753378`: Latest Story 2.1 fix -- internal error strings are mapped to user-friendly messages in the UI. Apply same pattern for auth errors.

Expected commit for this story: `feat: add challenge-response authentication and session management`

### Latest Technical Information

**`ed25519-dalek` v2 -- Signature verification:**
- `VerifyingKey::from_bytes(&[u8; 32])` -- creates verifier from raw public key bytes
- `Signature::from_bytes(&[u8; 64])` -- creates signature from raw bytes (NOT `from_slice`)
- `vk.verify(message: &[u8], &sig)` -- returns `Result<(), SignatureError>`
- Import: `use ed25519_dalek::{Signature, VerifyingKey, Verifier};` -- the `Verifier` trait must be imported for `.verify()` method

**`@noble/ed25519` v3 -- Signing:**
- `ed.signAsync(message: Uint8Array, secretKey: Uint8Array)` -- returns `Promise<Uint8Array>` (64 bytes)
- `secretKey` is the 32-byte seed (same bytes stored in IndexedDB)
- The async variant uses `crypto.subtle.digest('SHA-512', ...)` internally -- no extra dependencies needed
- Import: `import * as ed from '@noble/ed25519'`

**`rand` crate v0.9 (Rust):**
- `use rand::Rng; rand::rng().random::<[u8; 32]>()` -- generates 32 cryptographically secure random bytes
- Default RNG is `ThreadRng` backed by the OS CSPRNG

**`dashmap` crate v6:**
- `DashMap::new()` -- creates concurrent hash map
- `.insert(key, value)` -- inserts (replaces if key exists)
- `.get(&key)` -- returns `Option<Ref<K, V>>` (read guard)
- `.remove(&key)` -- removes and returns `Option<(K, V)>`
- Lock-free concurrent reads, sharded write locks -- safe for Tokio async

### SQLite Foreign Keys Note

SQLite requires `PRAGMA foreign_keys = ON` per connection for FK constraints to be enforced. Check if the existing pool setup handles this. If not, add it to `db/pool.rs` in the SQLite pool connection options:

```rust
// In SqlitePoolOptions or connection string
"sqlite:./data.db?mode=rwc" // add &_foreign_keys=on if using sqlx connection string
```

Or set the pragma via `sqlx::sqlite::SqliteConnectOptions::pragma("foreign_keys", "ON")`.

This is important because `sessions.user_id REFERENCES users(id) ON DELETE CASCADE` won't work without it.

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Story 2.2] -- Story statement and acceptance criteria
- [Source: _bmad-output/planning-artifacts/epics.md#Epic 2] -- Epic objectives (FR1-7, NFR11, NFR13)
- [Source: _bmad-output/planning-artifacts/epics.md#FR7] -- "The system can verify a user's identity cryptographically"
- [Source: _bmad-output/planning-artifacts/epics.md#NFR11] -- "All connections use TLS 1.3+"
- [Source: _bmad-output/planning-artifacts/architecture.md#Authentication & Security] -- Challenge-response -> session token, server-side session store
- [Source: _bmad-output/planning-artifacts/architecture.md#API & Communication Patterns] -- REST JSON envelope, error format
- [Source: _bmad-output/planning-artifacts/architecture.md#Project Structure] -- `identity/challenge.rs`, `services/auth_service.rs`, `models/session.rs`, `middleware/auth.rs`
- [Source: _bmad-output/planning-artifacts/architecture.md#Naming Conventions] -- snake_case tables/columns, PascalCase Rust structs
- [Source: _bmad-output/planning-artifacts/architecture.md#Enforcement Guidelines] -- Result<Json<T>, AppError>, cursor pagination, no unwrap in handlers
- [Source: _bmad-output/planning-artifacts/architecture.md#Anti-Patterns] -- no bare JSON, no offset pagination, no `any` types
- [Source: _bmad-output/implementation-artifacts/2-1-client-side-keypair-generation-and-identity-creation.md] -- Previous story patterns, crypto module API, test helpers, DID format, key storage
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Loading Patterns] -- <200ms no loading state, 200ms-2s skeleton, >2s skeleton + text

## Dev Agent Record

### Agent Model Used

GitHub Copilot CLI 0.0.414

### Debug Log References

- `cd client && npm ci && npm run lint && npm run check`
- `cd server && cargo fmt --check && cargo clippy -- -D warnings && cargo test`

### Completion Notes List

- Implemented challenge-response authentication (`/api/v1/auth/challenge`, `/api/v1/auth/verify`) using Ed25519 signatures over a server-issued nonce.
- Added DB-backed sessions with UUIDv4 tokens, configurable TTL (`auth.session_ttl_hours`), and debounced `last_active_at` refresh via the `AuthenticatedUser` extractor.
- Implemented logout (`DELETE /api/v1/auth/logout`) to delete the server-side session record and invalidate the token.
- Updated client identity flow to auto-authenticate in the background (shows "Signing in..." skeleton) and inject `Authorization: Bearer <token>` for subsequent API calls.
- Added unit + integration test coverage for the full lifecycle (challenge/verify/replay/expiry/logout/protected routes).

### File List

- `_bmad-output/implementation-artifacts/sprint-status.yaml`
- `_bmad-output/implementation-artifacts/2-2-challenge-response-authentication-and-session-management.md`
- `config.example.toml`
- `server/Cargo.toml`
- `server/Cargo.lock`
- `server/migrations/0004_create_sessions.sql`
- `server/src/config/mod.rs`
- `server/src/config/settings.rs`
- `server/src/db/pool.rs`
- `server/src/error.rs`
- `server/src/handlers/admin.rs`
- `server/src/handlers/auth.rs`
- `server/src/handlers/health.rs`
- `server/src/handlers/instance.rs`
- `server/src/handlers/mod.rs`
- `server/src/identity/challenge.rs`
- `server/src/identity/mod.rs`
- `server/src/lib.rs`
- `server/src/main.rs`
- `server/src/middleware/auth.rs`
- `server/src/middleware/mod.rs`
- `server/src/models/mod.rs`
- `server/src/models/session.rs`
- `server/src/services/auth_service.rs`
- `server/src/services/mod.rs`
- `server/tests/server_binds_to_configured_port.rs`
- `client/src/App.svelte`
- `client/src/lib/api.ts`
- `client/src/lib/features/identity/crypto.ts`
- `client/src/lib/features/identity/identityApi.ts`
- `client/src/lib/features/identity/identityStore.svelte.ts`
- `client/src/lib/features/identity/types.ts`
- `client/package-lock.json`

## Senior Developer Review (AI)

**Outcome:** Approved (after fixes)

### Findings Fixed
 
- **HIGH:** `POST /api/v1/admin/backup` was callable without any authentication (full DB exfiltration risk). Fixed by requiring a valid session (Bearer token) and updating client + tests accordingly.
- **HIGH:** Admin endpoints (`/api/v1/admin/*`) were accessible to any authenticated user. Fixed by requiring the authenticated username to match the instance admin in `admin_users` (and hiding the Admin UI for non-admin sessions).
- **MEDIUM:** Client session restore did not validate `expiresAt`, so expired sessions could be shown as logged-in until the first 401. Fixed by validating timestamp + expiry and re-authing when expired.
- **LOW:** Successful client session restore did not clear stale `authError`, which could incorrectly show the auth error view. Fixed by clearing `authError` when a session restore succeeds.
- **MEDIUM:** Session expiry enforcement compared RFC 3339 timestamps as strings, which is fragile (format/precision differences can break ordering). Fixed by parsing `expires_at` as RFC 3339 and comparing as `DateTime`.
- **MEDIUM:** `POST /api/v1/auth/verify` accepted an unbounded `challenge` string. Fixed by validating challenge format (64 hex chars) before processing.
- **MEDIUM:** `POST /api/v1/auth/verify` performed signature verification before confirming a pending challenge existed (unnecessary crypto work / easier DoS). Fixed by checking challenge existence/expiry/match before verifying the signature (without consuming).
- **MEDIUM:** `POST /api/v1/auth/verify` consumed the stored challenge before validating the signature payload, so malformed or invalid signatures could burn a challenge and force a retry. Fixed by validating signature format + cryptographic verification before consuming the challenge.
- **LOW:** `DELETE /api/v1/auth/logout` re-parsed the bearer token even though `AuthenticatedUser` already validated it. Fixed by deleting sessions by `session_id` instead of re-reading headers.
- **LOW:** Invalid `sessions.last_active_at` timestamps would trigger a 500 during auth extraction. Fixed by logging a warning and forcing a refresh of `last_active_at`.
- **MEDIUM:** Invalid `sessions.expires_at` timestamps would trigger a 500 during auth extraction. Fixed by logging a warning and treating the session as expired (401).
- **LOW:** Expired/invalid sessions were not cleaned up, allowing `sessions` table growth over time. Fixed by deleting expired/invalid sessions during validation (best-effort) while still returning 401.
- **LOW:** Authorization header parsing was scheme-case-sensitive (`Bearer` only). Fixed by accepting case-insensitive `bearer` scheme and adding coverage.
- **MEDIUM:** `npm audit` flagged vulnerabilities in `svelte` and transitive `devalue`. Fixed via `npm audit fix` (updated `client/package-lock.json`); audit now clean.
- **LOW:** Minor review hardening: client backup download now uses the same auth-header injection behavior as other API calls, and integration test helpers were cleaned up after the auth change.
- **LOW:** Added integration test coverage to ensure non-admin sessions receive 403 on `/api/v1/admin/health` (guards against regressions in admin authorization).
- **MEDIUM:** `App.svelte` used non-reactive local state in runes mode (triggering svelte-check warnings and risking stale UI). Fixed by converting local state to `$state(...)` and switching DOM event handlers from `on:click` to `onclick`.
- **MEDIUM:** Client did not react to runtime 401s (expired/revoked sessions), leaving `identityState.session` set while API calls fail. Fixed by adding an `apiFetch` unauthorized hook and wiring it to clear the session + re-authenticate.
- **LOW:** Client `signChallenge()` now enforces the expected 64-hex-char challenge length for clearer failures; minor server extractor cleanup (avoid `parts` shadowing).

### Verification

- `cd client && npm ci && npm run build && npm run lint && npm run check`
- `cd client && npm audit fix && npm run build && npm run lint && npm run check`
- `cd server && cargo fmt --check && cargo clippy -- -D warnings && cargo test`

## Change Log

- 2026-02-23: Implemented Story 2.2 (challenge-response auth + session management) and added tests; status -> review.
- 2026-02-23: Post-implementation hardening: challenge mismatch no longer consumes the stored challenge; stricter client session restore validation.
- 2026-02-23: Senior dev review: secured admin backup endpoint behind session auth, validated restored session expiry, and updated client + tests; status -> done.
- 2026-02-23: Senior dev review hardening: validate auth challenge format, parse session expiry as RFC 3339 DateTime, and update client dependencies via `npm audit fix`.
- 2026-02-23: Senior dev review hardening: validate signature before consuming challenges, delete sessions by session_id on logout, and harden last_active_at refresh behavior.
- 2026-02-23: Senior dev review hardening: treat invalid `sessions.expires_at` as expired (401) and accept case-insensitive `Bearer` scheme; added tests.
- 2026-02-23: Senior dev review hardening: pre-check pending challenge before signature verification, delete expired/invalid sessions during validation, and clear stale `authError` on successful session restore.
- 2026-02-23: Senior dev review follow-up: add non-admin admin access regression test and fix a misleading test name.
- 2026-02-23: Senior dev review follow-up: fix App.svelte runes state/event handling (remove svelte-check warnings).
- 2026-02-23: Senior dev review follow-up: auto-clear client session + re-authenticate on 401; tighten client challenge validation.
