# Story 2.1: Client-Side Keypair Generation and Identity Creation

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a **new user**,
I want to pick a username and have my identity created instantly,
So that I can join a guild without a complex registration process.

## Acceptance Criteria

1. **Given** a user opens the Discool SPA without an existing identity in browser storage
   **When** they enter a username and optionally select an avatar color
   **Then** an Ed25519 keypair is generated client-side in the browser
   **And** a DID document is created from the public key (`did:key:z6Mk...` format)

2. **Given** the keypair has been generated
   **Then** the private key is encrypted with an AES-256-GCM wrapping key and stored in IndexedDB
   **And** the wrapping key is a non-extractable `CryptoKey` object also stored in IndexedDB (same-origin protection + XSS-resistant)
   **And** raw private key bytes are never persisted unencrypted (NFR13)

3. **Given** the keypair is stored locally
   **When** the user clicks "Create Identity"
   **Then** the `did_key`, `username`, and optional `avatar_color` are sent to the server via `POST /api/v1/auth/register`
   **And** the server validates the DID format, extracts the Ed25519 public key, and verifies it is a valid curve point
   **And** a `users` table record is created with `id` (UUID), `did_key`, `public_key_multibase`, `username`, `avatar_color`, `created_at`, `updated_at`

4. **Given** the server has created the user record
   **Then** the client receives the created user data
   **And** the identity store is updated to reflect the registered state
   **And** the App.svelte state machine transitions from "no identity" to "identity exists"

5. **Given** the username is already taken on this instance
   **When** the registration request is submitted
   **Then** the server returns 409 Conflict with a clear message
   **And** the client displays "Username already taken" inline on the form (no page reload)

6. **Given** the DID is already registered on this instance (same keypair re-registration)
   **When** the registration request is submitted
   **Then** the server returns 409 Conflict with "Identity already registered on this instance"

7. **Given** the entire flow from username entry to server confirmation
   **Then** the process completes in under 3 seconds (including keypair generation, encryption, and network round-trip)
   **And** no cryptographic concepts are visible in the UI — the user sees only "Pick a username"

## Tasks / Subtasks

- [x] Task 1: Create database migration for `users` table (AC: #3)
  - [x] 1.1 Create `server/migrations/0003_create_users.sql` with `users` table: `id TEXT PRIMARY KEY`, `did_key TEXT NOT NULL UNIQUE`, `public_key_multibase TEXT NOT NULL`, `username TEXT NOT NULL UNIQUE`, `avatar_color TEXT`, `created_at TEXT NOT NULL`, `updated_at TEXT NOT NULL`
  - [x] 1.2 Add indexes: `idx_users_did_key` on `did_key`, `idx_users_username` on `username`
  - [x] 1.3 Verify migration runs on both SQLite and PostgreSQL (`cargo test`)

- [x] Task 2: Add server Cargo dependencies (AC: #3)
  - [x] 2.1 Add `ed25519-dalek = { version = "2", features = ["alloc"] }` to `Cargo.toml`
  - [x] 2.2 Add `bs58 = "0.5"` to `Cargo.toml`

- [x] Task 3: Create server identity module — DID validation + public key extraction (AC: #3)
  - [x] 3.1 Create `server/src/identity/mod.rs` exporting `did` and `keypair` submodules
  - [x] 3.2 Create `server/src/identity/did.rs`:
    - `parse_did_key(did: &str) -> Result<[u8; 32], DidError>` — validates `did:key:z6Mk...` prefix, decodes base58btc (after `z` multibase prefix), checks multicodec varint prefix `[0xed, 0x01]`, returns raw 32-byte public key
    - `DidError` enum: `InvalidPrefix`, `InvalidMultibase`, `InvalidMulticodec`, `InvalidKeyLength`
    - Unit tests: valid DID, wrong prefix, truncated key, invalid base58, wrong multicodec
  - [x] 3.3 Create `server/src/identity/keypair.rs`:
    - `validate_ed25519_public_key(bytes: &[u8; 32]) -> Result<(), KeyError>` — uses `ed25519_dalek::VerifyingKey::from_bytes()` to confirm the bytes are a valid Ed25519 point
    - `KeyError` enum: `InvalidPoint`
    - Unit tests: valid key, invalid all-zeros key, random invalid bytes
  - [x] 3.4 Register `identity` module in `server/src/lib.rs`

- [x] Task 4: Create server user model (AC: #3)
  - [x] 4.1 Create `server/src/models/mod.rs` exporting `user` submodule
  - [x] 4.2 Create `server/src/models/user.rs`:
    - `User` struct: `id: String`, `did_key: String`, `public_key_multibase: String`, `username: String`, `avatar_color: Option<String>`, `created_at: String`, `updated_at: String`
    - Derive `sqlx::FromRow`, `serde::Serialize`
    - `UserResponse` struct (for API response): same fields minus `updated_at` (or include it, match team preference)
  - [x] 4.3 Register `models` module in `server/src/lib.rs`

- [x] Task 5: Create server registration handler (AC: #3, #5, #6)
  - [x] 5.1 Create `server/src/handlers/auth.rs`:
    - `POST /api/v1/auth/register` handler
    - Wire type: `RegisterRequest { did_key: String, username: String, avatar_color: Option<String> }`
    - Validation: `username` required (1-32 chars, alphanumeric + underscore + hyphen, trimmed), `avatar_color` if present must be `#RRGGBB` hex, `did_key` must start with `did:key:z6Mk`
    - Call `identity::did::parse_did_key()` to extract public key bytes
    - Call `identity::keypair::validate_ed25519_public_key()` to validate the key
    - Derive `public_key_multibase` from the DID (the part after `did:key:`)
    - Insert into `users` table with UUID v4 `id`, RFC 3339 timestamps
    - Use `INSERT ... ON CONFLICT DO NOTHING` pattern — check rows affected to detect duplicate DID vs duplicate username (two separate queries if needed for precise error messages, or use a transaction with separate existence checks)
    - Return 201 with `{ "data": { id, did_key, username, avatar_color, created_at } }`
    - Return 409 for duplicate DID or duplicate username (with distinguishable error messages)
    - Return 422 for validation errors
  - [x] 5.2 Register route in `server/src/handlers/mod.rs`: `POST /api/v1/auth/register => auth::register`
  - [x] 5.3 Unit tests: valid registration, duplicate DID, duplicate username, invalid DID format, invalid username chars, empty username, username too long, invalid avatar color

- [x] Task 6: Install client npm dependency (AC: #1)
  - [x] 6.1 Run `npm install @noble/ed25519` in `client/` directory
  - [x] 6.2 Verify the package is added to `package.json` dependencies (not devDependencies)

- [x] Task 7: Create client crypto module — keypair generation, DID creation, key encryption (AC: #1, #2)
  - [x] 7.1 Create `client/src/lib/features/identity/crypto.ts`:
    - `generateIdentity(): Promise<{ secretKey: Uint8Array, publicKey: Uint8Array, didKey: string }>` — calls `ed.keygenAsync()`, constructs DID:key string
    - `didKeyFromPublicKey(publicKey: Uint8Array): string` — multicodec prefix `[0xed, 0x01]` + publicKey → base58btc encode → `'z'` prefix → `'did:key:'` prefix
    - `base58btcEncode(bytes: Uint8Array): string` — Bitcoin alphabet base58 encoder (inline, ~20 lines)
    - `encryptAndStoreKey(secretKey: Uint8Array, publicKey: Uint8Array, didKey: string, username: string): Promise<void>` — generates AES-256-GCM wrapping key (non-extractable CryptoKey), encrypts secretKey, stores `{ wrappingKey, encryptedSecretKey, iv, publicKey, didKey, username }` in IndexedDB
    - `loadStoredIdentity(): Promise<StoredIdentity | null>` — reads from IndexedDB, returns identity metadata (publicKey, didKey, username) if present, or null
    - `decryptSecretKey(): Promise<Uint8Array>` — loads wrapping key from IndexedDB, decrypts secretKey (for future signing use in Story 2.2)
    - `clearStoredIdentity(): Promise<void>` — removes all identity data from IndexedDB
    - Internal: `openIdentityDb(): Promise<IDBDatabase>` — opens/creates `discool-identity` database with `keys` object store (version 1)
    - Type: `StoredIdentity { publicKey: Uint8Array, didKey: string, username: string, avatarColor: string | null, registeredAt: string }`
  - [x] 7.2 All IndexedDB operations use raw API wrapped in Promise helpers (no external IndexedDB library)
  - [x] 7.3 Zero the `secretKey` Uint8Array after encryption (fill with 0s) to minimize memory exposure window

- [x] Task 8: Create client identity store (AC: #4)
  - [x] 8.1 Create `client/src/lib/features/identity/identityStore.svelte.ts`:
    - Uses Svelte 5 `$state` runes (NOT legacy `writable()` stores)
    - State: `identity: StoredIdentity | null`, `loading: boolean`, `error: string | null`
    - `initialize()` — calls `loadStoredIdentity()`, sets state
    - `register(didKey: string, username: string, avatarColor: string | null): Promise<void>` — calls `identityApi.register()`, updates state on success
    - `clear(): Promise<void>` — calls `clearStoredIdentity()`, resets state
    - Exported as reactive state object: `export const identityState = $state({ ... })`

- [x] Task 9: Create client identity API (AC: #3, #4)
  - [x] 9.1 Create `client/src/lib/features/identity/identityApi.ts`:
    - Follow existing `api.ts` patterns: wire types (snake_case) + public types (camelCase) + mapper functions
    - `RegisterRequestWire { did_key: string, username: string, avatar_color?: string }`
    - `RegisteredUserWire { id: string, did_key: string, username: string, avatar_color?: string, created_at: string }`
    - `RegisteredUser { id: string, didKey: string, username: string, avatarColor: string | null, createdAt: string }` (public type)
    - `register(didKey: string, username: string, avatarColor?: string): Promise<RegisteredUser>` — calls `apiFetch<RegisteredUserWire>('/api/v1/auth/register', { method: 'POST', body: ... })`, maps response
    - Uses existing `apiFetch` from `$lib/api.ts` and `ApiError` for error handling
  - [x] 9.2 Create `client/src/lib/features/identity/types.ts` — shared type definitions used by crypto, store, and API modules

- [x] Task 10: Create LoginView component — "Pick a username" UI (AC: #1, #7)
  - [x] 10.1 Create `client/src/lib/features/identity/LoginView.svelte`:
    - Header: "Pick a username" (large, centered, no mention of crypto/identity/keypair)
    - Username input: required, auto-focus, validate on blur (1-32 chars, `^[a-zA-Z0-9_-]+$`), inline error message
    - Avatar color picker: reuse the exact pattern from `SetupPage.svelte` (8 preset hex colors, radio group, keyboard navigation with ArrowLeft/Right/Up/Down/Home/End)
    - "Create Identity" button (primary/fire accent): disabled until username valid, shows loading spinner during creation
    - Behind the scenes on submit: `generateIdentity()` → `encryptAndStoreKey()` → `identityApi.register()` → update identity store
    - Error display: server errors shown inline below the form (username taken, DID already registered, network error)
    - Follow existing form patterns from SetupPage: single-column layout, labels above inputs, validate on blur, fire CTA
    - Use Svelte 5 runes (`$state`, `$derived`, `$effect`) — NOT legacy `createEventDispatcher` or `on:event` syntax
    - Use `{@render children()}` or callback props for parent communication — NOT `dispatch('complete', ...)`
  - [x] 10.2 No mention of "keypair", "cryptography", "Ed25519", "DID", or "identity" in any user-facing text. The UI says "Pick a username" and "Create" / "Join".

- [x] Task 11: Integrate identity check into App.svelte (AC: #4)
  - [x] 11.1 Update `App.svelte` state machine:
    - On mount: existing `getInstanceStatus()` call stays
    - After instance status loaded (initialized): call `identityState.initialize()` to check IndexedDB for stored identity
    - New state: `initialized && !identity` → show `<LoginView oncomplete={handleIdentityCreated} />`
    - Existing state: `initialized && identity` → show main layout (home + admin sidebar)
    - Loading states: show spinner while checking instance status AND identity
  - [x] 11.2 The SetupPage flow remains unchanged — it runs first (instance must be initialized before identity creation)
  - [x] 11.3 After identity creation, transition to main layout immediately (no page reload)

- [x] Task 12: Server integration tests (AC: #3, #5, #6, #7)
  - [x] 12.1 Add to `server/tests/server_binds_to_configured_port.rs` (or create `server/tests/test_auth.rs` if separation preferred):
    - Test: POST register with valid data → 201, response has correct fields
    - Test: POST register with same DID → 409 "Identity already registered"
    - Test: POST register with same username different DID → 409 "Username already taken"
    - Test: POST register with invalid DID format → 422
    - Test: POST register with empty username → 422
    - Test: POST register before instance setup → should still work (registration is independent of admin setup)
    - All tests use SQLite in-memory (existing pattern)

- [x] Task 13: Verify lint + existing tests pass (all ACs)
  - [x] 13.1 `cargo fmt --check && cargo clippy -- -D warnings && cargo test`
  - [x] 13.2 `cd client && npx biome check . && npx svelte-check --tsconfig ./tsconfig.app.json`

## Dev Notes

### Architecture Compliance

This story implements the first layer of the identity system as specified in the architecture doc. It establishes:

1. **Server `identity/` module** — with `did.rs` (DID:key parsing) and `keypair.rs` (Ed25519 validation). Matches the architecture structure at `server/src/identity/`. The `challenge.rs` and `recovery.rs` files are NOT created in this story (Story 2.2 and 2.7 respectively).

2. **Server `models/` module** — with `user.rs`. This is a new module layer. The architecture specifies `models/user.rs` for `User, UserProfile, UserSettings`. This story only creates the base `User` struct.

3. **Server `handlers/auth.rs`** — Registration endpoint only. The architecture specifies this file handles `/api/v1/auth/*`. Challenge-response is Story 2.2.

4. **Client `features/identity/`** — Matches the architecture's `lib/features/identity/` directory with `crypto.ts`, `identityStore.svelte.ts`, `identityApi.ts`, `LoginView.svelte`, and `types.ts`.

5. **No services layer yet** — The architecture specifies `services/auth_service.rs`, but the existing codebase has no services layer (handlers query DB directly). Introducing services is deferred until Story 2.2 when the auth logic becomes complex enough to warrant extraction.

### Ed25519 Library Choices

**Client — `@noble/ed25519` (v3):**
- Web Crypto Ed25519 is NOT universally supported as of Feb 2026. Chrome/Chromium only shipped it enabled-by-default in ~v137 (May 2025). Safari 17+ and Firefox 129+ support it, but Samsung Internet and older Chromium do not.
- `@noble/ed25519` is 3.7KB gzipped, zero production dependencies, audited by cure53, RFC 8032 compliant, SUF-CMA secure.
- Uses `crypto.subtle.digest('SHA-512', ...)` internally for async methods — no `@noble/hashes` dependency needed.
- API: `ed.keygenAsync()` returns `{ secretKey: Uint8Array(32), publicKey: Uint8Array(32) }`.

**Server — `ed25519-dalek` (v2):**
- Standard Rust Ed25519 crate for public key validation.
- `VerifyingKey::from_bytes(&[u8; 32])` returns `Result` — rejects invalid points.
- The `alloc` feature is needed for this operation.

### DID:key Format

The DID:key method for Ed25519 produces DIDs starting with `did:key:z6Mk`:
```
did:key:z6MkhaXgBZDvotDkL5257faiztiGiC2QtKLGpbnnEGta2doK
        ^
        z = multibase prefix for base58btc
         6Mk... = base58btc(0xed01 + 32_byte_pubkey)
                   ^^^^^
                   multicodec varint for ed25519-pub
```

Construction: `[0xed, 0x01]` (2 bytes) + raw 32-byte public key = 34 bytes → base58btc encode → prepend `z` → prepend `did:key:`.

### Private Key Protection (NFR13)

Private key bytes from `@noble/ed25519` are raw `Uint8Array(32)`. These MUST be encrypted before storage:

1. Generate AES-256-GCM wrapping key: `crypto.subtle.generateKey({ name: 'AES-GCM', length: 256 }, false, ['encrypt', 'decrypt'])` — `extractable: false` makes the key opaque to JavaScript (XSS-resistant)
2. Encrypt: `crypto.subtle.encrypt({ name: 'AES-GCM', iv }, wrappingKey, secretKeyBytes)` with random 12-byte IV
3. Store in IndexedDB: `{ wrappingKey: CryptoKey, encryptedSecretKey: ArrayBuffer, iv: Uint8Array, publicKey: Uint8Array, didKey: string, username: string }`
4. Zero the plaintext `secretKey` immediately after encryption: `secretKey.fill(0)`

The wrapping key is non-extractable — its raw bytes cannot be read by JavaScript. IndexedDB uses the structured clone algorithm to serialize `CryptoKey` objects without exposing key material.

### Database Schema

Migration `0003_create_users.sql`:
```sql
CREATE TABLE IF NOT EXISTS users (
    id TEXT PRIMARY KEY,
    did_key TEXT NOT NULL UNIQUE,
    public_key_multibase TEXT NOT NULL,
    username TEXT NOT NULL UNIQUE,
    avatar_color TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_users_did_key ON users(did_key);
CREATE INDEX IF NOT EXISTS idx_users_username ON users(username);
```

- `id`: UUID v4 as TEXT (matches `admin_users.id` pattern)
- `did_key`: Full DID string (`did:key:z6Mk...`), UNIQUE — one identity per instance
- `public_key_multibase`: Multibase-encoded public key (the part after `did:key:`), stored for convenient verification without re-parsing the DID
- `username`: UNIQUE per instance — prevents display confusion
- `avatar_color`: Nullable `#RRGGBB` hex (same pattern as `admin_users.avatar_color`)
- Timestamps: RFC 3339 TEXT format (matches existing `admin_users.created_at` pattern)

The `admin_users` table (from Epic 1) remains separate. Admin auth will be linked to the identity system in a later story.

### API Contract

```
POST /api/v1/auth/register
Content-Type: application/json

Request body:
{
  "did_key": "did:key:z6MkhaXgBZDvotDkL5257faiztiGiC2QtKLGpbnnEGta2doK",
  "username": "liam",
  "avatar_color": "#3B82F6"  // optional
}

Success (201):
{
  "data": {
    "id": "550e8400-e29b-41d4-a716-446655440000",
    "did_key": "did:key:z6MkhaXgBZDvotDkL5257faiztiGiC2QtKLGpbnnEGta2doK",
    "username": "liam",
    "avatar_color": "#3B82F6",
    "created_at": "2026-02-19T12:00:00Z"
  }
}

Conflict (409 — duplicate DID):
{ "error": { "code": "CONFLICT", "message": "Identity already registered on this instance" } }

Conflict (409 — duplicate username):
{ "error": { "code": "CONFLICT", "message": "Username already taken" } }

Validation error (422):
{ "error": { "code": "VALIDATION_ERROR", "message": "Invalid DID format: must start with did:key:z6Mk" } }
```

### Username Validation Rules

- Required, 1-32 characters
- Allowed characters: `[a-zA-Z0-9_-]` (alphanumeric, underscore, hyphen)
- Trimmed (no leading/trailing whitespace)
- Case-sensitive (for now; case-insensitive uniqueness can be added later if needed)
- Validated both client-side (blur) and server-side (authoritative)

### App.svelte State Machine Update

Current flow:
```
loading → error | !initialized → SetupPage | initialized → MainLayout
```

Updated flow:
```
loading → error | !initialized → SetupPage | initialized + identityLoading → spinner | initialized + !identity → LoginView | initialized + identity → MainLayout
```

The identity check happens AFTER the instance status check. If the instance isn't initialized, the SetupPage runs first. Only after initialization does the identity check occur.

### Svelte 5 Patterns (Critical)

The existing `SetupPage.svelte` uses legacy Svelte 4 patterns (`createEventDispatcher`, `on:complete`). New components in this story MUST use Svelte 5 runes:
- State: `let count = $state(0)` — NOT `let count = writable(0)`
- Derived: `let doubled = $derived(count * 2)` — NOT `$: doubled = count * 2`
- Effects: `$effect(() => { ... })` — NOT `$: { ... }`
- Props: `let { value, onchange }: Props = $props()` — NOT `export let value`
- Events: callback props (`oncomplete`, `onerror`) — NOT `createEventDispatcher`
- Children: `{@render children?.()}` — NOT `<slot />`

The `LoginView` component communicates with `App.svelte` via a callback prop (`oncomplete`), not event dispatching.

### Existing Code Reuse

- **`apiFetch<T>()`** from `$lib/api.ts`: Use for the register API call. Do NOT create a separate fetch wrapper.
- **`ApiError`** from `$lib/api.ts`: Handle 409/422 errors using the existing error class.
- **Avatar color picker pattern** from `SetupPage.svelte`: Reuse the 8-color preset radio group with keyboard navigation. Extract shared logic if significant duplication, but do NOT over-engineer — copying the pattern is acceptable for 2 components.
- **`cn()` utility** from `$lib/utils.ts`: Use for conditional class merging.
- **JSON envelope pattern**: All API responses use `{ "data": ... }` for success and `{ "error": { "code", "message", "details" } }` for errors. Follow this exactly.
- **Timestamp format**: RFC 3339 string (`chrono::Utc::now().to_rfc3339()`). Matches existing `initialized_at` pattern in instance setup handler.

### Files to Create

| File | Purpose |
|---|---|
| `server/migrations/0003_create_users.sql` | Users table migration |
| `server/src/identity/mod.rs` | Identity module root |
| `server/src/identity/did.rs` | DID:key parsing and validation |
| `server/src/identity/keypair.rs` | Ed25519 public key validation |
| `server/src/models/mod.rs` | Models module root |
| `server/src/models/user.rs` | User struct + sqlx::FromRow |
| `server/src/handlers/auth.rs` | Registration handler |
| `client/src/lib/features/identity/crypto.ts` | Keypair generation, DID, encryption, IndexedDB |
| `client/src/lib/features/identity/identityStore.svelte.ts` | Reactive identity state |
| `client/src/lib/features/identity/identityApi.ts` | Registration API call |
| `client/src/lib/features/identity/types.ts` | Shared types |
| `client/src/lib/features/identity/LoginView.svelte` | "Pick a username" UI |

### Files to Modify

| File | Change |
|---|---|
| `server/Cargo.toml` | Add `ed25519-dalek` and `bs58` dependencies |
| `server/src/lib.rs` | Register `identity` and `models` modules |
| `server/src/handlers/mod.rs` | Add `auth` module and `POST /api/v1/auth/register` route |
| `client/package.json` | Add `@noble/ed25519` dependency |
| `client/src/App.svelte` | Add identity state check, show LoginView when no identity |

### Project Structure Notes

- All new server files follow the architecture doc's directory structure: `identity/`, `models/`, `handlers/auth.rs`
- All new client files follow the architecture doc's feature-based structure: `lib/features/identity/`
- The `server/src/identity/` module is at the same level as `db/`, `config/`, `handlers/` — consistent with the architecture's layered approach
- The `server/src/models/` module is new — no existing models directory exists. Create `models/mod.rs` as the module root.
- Client feature directory `lib/features/` is new — no existing features directory exists. Create it as the parent for `identity/`.
- Migration naming: `0003_` prefix continues the 4-digit numbering from `0001_` and `0002_` (NOT the 3-digit `001_` from the architecture doc — follow actual codebase convention)
- File naming: `identityStore.svelte.ts` (camelCase, Svelte rune file convention) matches architecture spec

### Testing Requirements

**Server unit tests** (in each module):
- `identity::did` — parse valid DID, reject malformed DIDs (wrong prefix, bad base58, wrong multicodec, short key, all-zeros key)
- `identity::keypair` — accept valid Ed25519 public key, reject invalid curve point
- `handlers::auth` — registration validation (username rules, avatar_color format, DID format)

**Server integration tests** (in `server/tests/`):
- Register user → 201 with correct response body
- Register duplicate DID → 409
- Register duplicate username → 409
- Register with invalid DID → 422
- Register with empty/invalid username → 422
- Register before instance setup → should work (user registration is independent of instance initialization)
- Verify existing tests still pass (health, instance setup, admin health, backup)

**Client tests** (if Vitest is configured — optional for this story):
- Covered by `client/src/lib/features/identity/crypto.test.ts` and `client/src/lib/features/identity/LoginView.test.ts`.

**Linting:** `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test`, `npx biome check .`, `npx svelte-check --tsconfig ./tsconfig.app.json`

### Anti-Patterns to Avoid

- Do NOT use Web Crypto `crypto.subtle.generateKey({ name: "Ed25519" })` as the primary path — browser support is fragmented. Use `@noble/ed25519` as the universal solution.
- Do NOT store raw private key bytes unencrypted in IndexedDB, localStorage, or sessionStorage.
- Do NOT expose "Ed25519", "keypair", "DID", "cryptographic identity", "signing key", or similar terms in any user-facing UI text.
- Do NOT use Svelte 4 patterns (`createEventDispatcher`, `export let`, `$:`, `writable()`) in new components. Use Svelte 5 runes.
- Do NOT create a separate `services/` layer for Story 2.1 — the handler can interact with the DB directly (consistent with existing handlers). Introduce services in Story 2.2 when logic complexity warrants it.
- Do NOT add authentication/session management to the register endpoint — that is Story 2.2.
- Do NOT add email association — that is Story 2.6.
- Do NOT modify the `admin_users` table or the instance setup flow — those remain as-is.
- Do NOT use `CURRENT_TIMESTAMP` in SQL inserts — use Rust-generated RFC 3339 timestamps (lesson from Story 1.8: PostgreSQL type mismatch).
- Do NOT use `$1`/`$2` PostgreSQL-specific placeholders in raw SQL — use the sqlx `query!` macro or `query()` with `?` placeholders and let the `Any` driver translate (match existing pattern in `instance.rs`).
- Do NOT add `@noble/hashes` as a dependency — `@noble/ed25519` async API uses `crypto.subtle` internally and doesn't need it.
- Do NOT use `npm install --save-dev` for `@noble/ed25519` — it is a runtime production dependency, not a dev dependency. Use `npm install` (adds to `dependencies`).

### Previous Story Intelligence

**From Story 1.8 (Docker and Deployment Pipeline):**
- SQL inserts must use bound RFC 3339 timestamp parameters — NOT `CURRENT_TIMESTAMP` (caused PostgreSQL type mismatch)
- sqlx `Any` driver uses `?` placeholders in `query()` — the driver translates to `$1`/`$2` for PostgreSQL
- Integration tests spawn a real server with SQLite in-memory (`sqlite::memory:`) — follow the same pattern for auth tests
- Commit format: `feat: description` for new features
- Full lint suite must pass: `cargo fmt`, `cargo clippy -- -D warnings`, `cargo test`, `biome check`, `svelte-check`

**From Story 1.5 (First-Run Admin Setup):**
- `SetupPage.svelte` uses 8 preset avatar colors with keyboard-navigable radio group — reuse this exact pattern
- Form validation pattern: validate on blur, show inline errors, disable submit until valid
- `instance.rs` handler uses `ON CONFLICT DO NOTHING` + rows-affected check for race-safe idempotency — use the same approach in the register handler
- Avatar color validation: `#RRGGBB` hex regex — reuse the same validation pattern

**From Story 1.1 (Project Scaffold):**
- `apiFetch<T>()` is the typed API client — all REST calls go through it
- `ApiError` class has `code`, `message`, `details` fields
- Wire types (snake_case) + public types (camelCase) + mapper functions is the API pattern
- Vite dev proxy: `/api/v1/*` → `http://localhost:3000` (already configured, no changes needed)

### Git Intelligence

**Recent commit patterns:**
```
107b620 feat: add Docker and deployment pipeline with multi-db support
a17bf38 feat: add admin backup/export endpoint with SQLite and PostgreSQL support
6842ec1 feat: add admin health dashboard with sidebar navigation
13be36e feat: add first-run admin setup screen with instance configuration
df8d403 feat: add health check and Prometheus metrics endpoints
```

Expected commit for this story: `feat: add client-side keypair generation and identity registration`

### Latest Technical Information

**`@noble/ed25519` v3 (npm):**
- Async API: `keygenAsync()`, `signAsync()`, `verifyAsync()` — use these (no extra deps needed)
- `secretKey` is 32 bytes (seed), `publicKey` is 32 bytes
- Performance: keygen ~88us, sign ~169us, verify ~780us (Apple M4 benchmark)
- RFC 8032 compliant, FIPS 186-5 compatible, cure53 audited

**`ed25519-dalek` v2 (Rust crate):**
- `VerifyingKey::from_bytes(&[u8; 32])` → `Result<VerifyingKey, SignatureError>`
- Rejects keys that are not valid Ed25519 curve points (including all-zeros)
- Feature `alloc` needed for byte deserialization

**`bs58` v0.5 (Rust crate):**
- `bs58::decode(input).into_vec()` → `Result<Vec<u8>, Error>`
- Supports Bitcoin base58 alphabet (same as multibase base58btc)

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Story 2.1] — Story statement and acceptance criteria
- [Source: _bmad-output/planning-artifacts/epics.md#Epic 2] — Epic objectives (FR1-7, NFR11, NFR13)
- [Source: _bmad-output/planning-artifacts/epics.md#FR1] — "Users can create a portable cryptographic identity (keypair) client-side"
- [Source: _bmad-output/planning-artifacts/epics.md#FR2] — "Users can set a display name and avatar"
- [Source: _bmad-output/planning-artifacts/epics.md#NFR13] — "Private keys encrypted at rest; never transmitted"
- [Source: _bmad-output/planning-artifacts/architecture.md#Authentication & Security] — Ed25519/X25519, DID/VC-compatible, challenge-response → session token
- [Source: _bmad-output/planning-artifacts/architecture.md#Project Structure] — `identity/`, `handlers/auth.rs`, `models/user.rs`, `features/identity/`
- [Source: _bmad-output/planning-artifacts/architecture.md#Naming Conventions] — snake_case tables, camelCase TS, `idx_` index prefix
- [Source: _bmad-output/planning-artifacts/architecture.md#API Design] — REST JSON envelope, HTTP status codes
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Onboarding Flow] — "Pick a username" flow, <30s new identity, <10s existing
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Design Principles] — "Invisible cryptographic identity", "No crypto concepts in UI"
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#UX Patterns] — Form patterns: single-column, labels above, validate on blur, fire CTA
- [Source: _bmad-output/planning-artifacts/prd.md#FR1-FR7] — Identity & Authentication functional requirements
- [Source: _bmad-output/planning-artifacts/prd.md#Security Constraints] — Transport encryption, key protection, input sanitization
- [Source: _bmad-output/implementation-artifacts/1-8-docker-and-deployment-pipeline.md] — Previous story patterns, SQL timestamp lessons, test patterns
- [Source: W3C CCG did:key Method v0.7] — DID:key specification, multicodec `0xed` prefix, base58btc multibase

## Dev Agent Record

### Agent Model Used

GitHub Copilot CLI 0.0.414

### Debug Log References

- `cd server && cargo test -q`
- `cd server && cargo fmt`
- `cd server && cargo fmt --check`
- `cd server && cargo clippy -- -D warnings`
- `cd client && npm install @noble/ed25519`
- `cd client && npx biome check .`
- `cd client && npx svelte-check --tsconfig ./tsconfig.app.json`

### Completion Notes List

- Added a `users` table migration (`0003_create_users.sql`) with unique constraints and indexes for `did_key` and `username`.
- Added server-side DID:key parsing + Ed25519 public key validation in `server/src/identity/`.
- Implemented `POST /api/v1/auth/register` with validation (username, avatar color, DID), conflict handling (duplicate DID vs username), and unit tests.
- Added server integration tests for registration (201/409/422 cases) using SQLite in-memory.
- Added client identity feature modules (keygen + DID:key, AES-GCM key wrapping + IndexedDB persistence, identity API + store).
- Added `LoginView` (“Pick a username”) and updated `App.svelte` to show it when instance is initialized but no identity is registered.
- Post-implementation polish: added a `did_key` length cap (DoS guard), hid crypto-ish validation errors from the UI, and reduced integration test port flakiness.

### File List

- `_bmad-output/implementation-artifacts/sprint-status.yaml` (status updated)
- `_bmad-output/implementation-artifacts/2-1-client-side-keypair-generation-and-identity-creation.md` (tasks/status/record updated)
- `server/migrations/0003_create_users.sql` (new)
- `server/Cargo.toml`
- `server/Cargo.lock`
- `server/src/lib.rs`
- `server/src/handlers/mod.rs`
- `server/src/handlers/auth.rs` (new)
- `server/src/identity/mod.rs` (new)
- `server/src/identity/did.rs` (new)
- `server/src/identity/keypair.rs` (new)
- `server/src/models/mod.rs` (new)
- `server/src/models/user.rs` (new)
- `server/tests/server_binds_to_configured_port.rs`
- `client/package.json`
- `client/package-lock.json`
- `client/src/App.svelte`
- `client/src/lib/features/identity/crypto.ts` (new)
- `client/src/lib/features/identity/crypto.test.ts`
- `client/src/lib/features/identity/identityApi.ts` (new)
- `client/src/lib/features/identity/identityStore.svelte.ts` (new)
- `client/src/lib/features/identity/LoginView.svelte` (new)
- `client/src/lib/features/identity/LoginView.test.ts`
- `client/src/lib/features/identity/types.ts` (new)

## Senior Developer Review (AI)

**Outcome:** Approved (after fixes)

### Findings Fixed
 
- **MEDIUM:** UI could surface server conflict message "Identity already registered on this instance" (violates "no identity/crypto terms in UI" requirement). Fixed by mapping to user-friendly copy in `LoginView.svelte`.
- **MEDIUM:** "Username already taken" was shown only in a generic form banner. Fixed to show inline under the Username field (matches AC #5 intent).
- **LOW:** Server existence-check helpers used `Option<i64>` for `SELECT 1` scalar; switched to `Option<i32>` to match the SQL literal type and avoid backend-specific decode surprises.
- **LOW:** SQLite auth queries used Postgres-style `$1` placeholders; switched to SQLite `?1` placeholders for clarity (no behavior change).
- **MEDIUM:** LoginView could surface raw internal error strings (e.g. IndexedDB failures or "No stored identity..."), leaking forbidden terms in the UI. Fixed by mapping non-API errors to user-friendly copy in `LoginView.svelte`.
- **LOW:** `base58btcEncode()` returned incorrect output for empty/all-zero byte arrays (extra leading `1`). Fixed with explicit early returns in `crypto.ts`.
- **LOW:** Added a unit test for DID strings missing the multibase `z` prefix in `server/src/identity/did.rs`.

### Verification

- `cd client && npm run build && npm run lint && npm run check`
- `cd server && cargo fmt --check && cargo clippy -- -D warnings && cargo test -q`

## Change Log

- 2026-02-19: Story created from epics, architecture, UX design spec, PRD, and previous story intelligence (Story 1.8). Includes comprehensive technical research on Ed25519 browser support, @noble/ed25519 API, DID:key specification, and IndexedDB key storage patterns.
- 2026-02-22: Implemented client-side keypair generation + encrypted local storage, server registration endpoint + validation, wired SPA login view + identity store, added unit/integration tests, and ran full lint/test suite; marked story ready for review.
- 2026-02-22: Post-implementation polish: guard against pathological `did_key` inputs, improve user-facing error messages, and stabilize integration test port selection; re-ran full checks.
- 2026-02-22: Senior dev review: fixed LoginView conflict copy + inline username-taken error, hardened server existence checks, re-ran full checks; marked story done.
- 2026-02-22: Review follow-up: normalized SQLite placeholder syntax in auth queries; re-ran server checks.
- 2026-02-22: Review follow-up: avoid surfacing internal error strings in LoginView, fix base58btc edge case, and add missing DID multibase test; re-ran full checks.
