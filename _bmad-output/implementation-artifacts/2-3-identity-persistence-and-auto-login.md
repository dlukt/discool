# Story 2.3: Identity Persistence and Auto-Login

Status: in-progress

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a **returning user**,
I want my identity to persist in the browser so I'm automatically logged in,
So that I don't have to re-authenticate every time I open Discool.

## Acceptance Criteria

1. **Given** a user has previously created an identity on this browser
   **When** they navigate to the Discool instance URL or open a bookmark (even after closing the browser entirely)
   **Then** the SPA detects the identity in IndexedDB
   **And** authentication happens automatically via challenge-response in the background
   **And** no login screen, "session expired" message, or manual action is required

2. **Given** a user has a valid session token persisted in localStorage
   **When** they reopen the browser or open a new tab
   **Then** the stored session is validated (expiry checked client-side, then server-side on first API call)
   **And** if the token is still valid server-side, the user is immediately authenticated without a new challenge-response
   **And** if the token is expired or revoked server-side, a background challenge-response re-authentication happens transparently

3. **Given** a user was last active in a specific guild and channel
   **When** they return to Discool after being away
   **Then** the SPA navigates them to their last active location (stored in localStorage)
   **And** if the last active location is no longer accessible (deleted channel, kicked from guild), the user lands on the home view

4. **Given** a user has Discool open in multiple browser tabs
   **When** they log out in one tab
   **Then** the other tabs detect the session change via `StorageEvent` listener on localStorage
   **And** the other tabs clear their session and return to the login view
   **And** conversely, authenticating in one tab propagates the session to other tabs

5. **Given** a user's stored identity in IndexedDB is corrupted or invalid
   **When** the SPA attempts to load the identity on startup
   **Then** a clear recovery prompt is shown: "Your stored identity appears to be damaged"
   **And** the prompt offers: "Create a new identity" (clears corrupted data) or "Recover via email" (placeholder for Story 2.7)
   **And** no cryptographic jargon is shown in the UI

6. **Given** the user's stored identity is valid but the user record does not exist on this instance (e.g., cleared server DB)
   **When** the challenge-response authentication fails with "Identity not found"
   **Then** the user is shown: "Your identity is not registered on this instance"
   **And** the user is offered: "Register again" (re-registers with the same keypair and username) or "Try a different instance"
   **And** the existing keypair in IndexedDB is preserved (not cleared) -- only the server-side state is missing

7. **Given** a user wants to explicitly log out and prevent auto-login
   **When** they click "Log out" in user settings
   **Then** the session token is deleted from both the server and localStorage
   **And** the user is returned to the login view
   **And** auto-login does NOT happen on next visit (the identity remains in IndexedDB but the session is cleared)
   **And** on next visit, the user sees a brief "Signing in..." state as challenge-response re-authentication runs automatically

## Tasks / Subtasks

- [x] Task 1: Migrate session storage from sessionStorage to localStorage (AC: #1, #2, #4)
  - [x] 1.1 Update `client/src/lib/features/identity/identityStore.svelte.ts`:
    - Change all `sessionStorage` reads/writes to `localStorage` with key `discool-session`
    - Store session as JSON: `{ token: string, expiresAt: string, user: RegisteredUser }`
    - On `restoreSession()`: read from localStorage, validate `expiresAt` client-side (reject if expired), then use token for API calls (server validates on first authenticated request via middleware)
    - On `authenticate()` success: write session to localStorage (replacing any existing)
    - On `logout()`: remove `discool-session` from localStorage and call API logout
  - [x] 1.2 Remove all `sessionStorage` references (clean migration, no dual-storage)

- [x] Task 2: Add cross-tab session synchronization (AC: #4)
  - [x] 2.1 In `identityStore.svelte.ts`, add a `StorageEvent` listener during `initialize()`:
    - Listen for `window.addEventListener('storage', handler)` on the `discool-session` key
    - When key is removed (logout in another tab): clear local `session` state, set `session = null`, navigate to login view
    - When key is set (login in another tab): parse the new session value, update `session` state, call `setSessionToken()` to update the API client
  - [x] 2.2 Clean up the storage event listener on module teardown (if applicable -- Svelte 5 runes modules are singletons, so this may just stay for the app lifetime)

- [ ] Task 3: Add last-active-location persistence (AC: #3)
  - [x] 3.1 Create `client/src/lib/features/identity/navigationState.ts`:
    - `saveLastLocation(path: string): void` -- writes to localStorage key `discool-last-location`
    - `getLastLocation(): string | null` -- reads from localStorage
    - `clearLastLocation(): void` -- removes the key
  - [x] 3.2 In `App.svelte`, after successful authentication:
    - Read last location from `getLastLocation()`
    - If a last location exists: attempt navigation (when router is available in Epic 4, this will use `router.push()`; for now, store the path for future use)
    - If no last location or location is invalid: show the default home view
  - [ ] 3.3 In `App.svelte`, when the current view changes (placeholder -- will be wired to router in Epic 4):
    - Call `saveLastLocation(currentPath)` whenever the user navigates to a guild/channel
    - Do NOT save admin panel or settings paths as "last location"
  - [x] 3.4 On `logout()`: call `clearLastLocation()` (fresh start on next login)

- [x] Task 4: Improve corrupted identity detection and recovery UX (AC: #5)
  - [x] 4.1 Update `crypto.ts` `loadStoredIdentity()`:
    - Add validation: check that `publicKey` is a Uint8Array of length 32, `didKey` starts with `did:key:z6Mk`, `username` is non-empty
    - If any validation fails: return a distinct result (e.g., `{ status: 'corrupted' }` vs `{ status: 'found', identity }` vs `{ status: 'none' }`) instead of just `null`
    - Catch IndexedDB errors (database deleted, object store missing) and return `'corrupted'` status
  - [x] 4.2 Update `identityStore.svelte.ts` `initialize()`:
    - Handle the new `'corrupted'` status from `loadStoredIdentity()`
    - Set a new state field: `identityCorrupted: boolean`
  - [x] 4.3 Create `client/src/lib/features/identity/RecoveryPrompt.svelte`:
    - Shown when `identityState.identityCorrupted` is true
    - Header: "Something went wrong with your stored data"
    - Body: "We couldn't load your saved identity. This can happen if browser data was partially cleared."
    - Actions: "Start fresh" button (fire CTA, calls `identityState.clear()` then redirects to LoginView) and "Recover via email" button (ice/secondary, disabled with tooltip "Coming soon" -- placeholder for Story 2.7)
    - No mention of "keypair", "IndexedDB", "cryptographic", "DID", or any technical terms
    - Follow Dual Core design: single-column centered layout, fire CTA button
  - [x] 4.4 Update `App.svelte` state machine to show `RecoveryPrompt` when `identityCorrupted` is true

- [x] Task 5: Handle "identity not found on instance" scenario (AC: #6)
  - [x] 5.1 Update `identityStore.svelte.ts` `authenticate()`:
    - When challenge request returns 404 ("Identity not found on this instance"): set a new state field `identityNotRegistered: boolean = true`
    - Do NOT clear the identity from IndexedDB (the keypair is still valid)
  - [x] 5.2 Create `client/src/lib/features/identity/ReRegisterPrompt.svelte`:
    - Shown when `identityState.identityNotRegistered` is true
    - Header: "Welcome back!"
    - Body: "Your identity isn't registered on this instance yet. Would you like to register?"
    - Show the stored username and avatar color (read from identityState.identity)
    - "Register as [username]" button (fire CTA): calls the existing `POST /api/v1/auth/register` endpoint with the stored identity's DID key, username, and avatar color, then calls `authenticate()` to get a session
    - "Use a different name" button (secondary): navigates to LoginView but preserves the existing keypair (only changes username on re-registration)
    - Handle conflict errors (username taken on this instance): show inline error, suggest changing username
  - [x] 5.3 Update `App.svelte` state machine to show `ReRegisterPrompt` when `identityNotRegistered` is true

- [x] Task 6: Refine auto-login flow and eliminate flash states (AC: #1, #7)
  - [x] 6.1 Update `App.svelte` loading sequence:
    - Current: `loading → error | !initialized → SetupPage | identity + authenticating → spinner | identity + authError | identity + session → MainLayout`
    - New: add `identityCorrupted → RecoveryPrompt` and `identityNotRegistered → ReRegisterPrompt` states
    - Ensure the "Signing in..." skeleton appears for no more than the time needed for challenge-response (~100-500ms on fast connections)
    - If session is restored from localStorage without re-auth: transition directly to main layout with no intermediate loading state
  - [x] 6.2 Ensure that after explicit logout (AC #7):
    - `discool-session` is removed from localStorage
    - On next visit: identity is loaded from IndexedDB → no stored session → `authenticate()` runs → "Signing in..." skeleton → session created → main layout
    - The user is NOT asked to "pick a username" (that's only for new identities)

- [ ] Task 7: Client tests (AC: all)
  - [ ] 7.1 Update `identityStore` tests (if Vitest configured):
    - Test: session persists in localStorage after authenticate()
    - Test: session restored from localStorage on initialize()
    - Test: expired session in localStorage triggers re-authentication
    - Test: StorageEvent triggers session sync across tabs
    - Test: corrupted identity returns 'corrupted' status
    - Test: logout clears localStorage session
  - [ ] 7.2 Component tests for RecoveryPrompt.svelte and ReRegisterPrompt.svelte (optional if Vitest configured)

- [ ] Task 8: Verify lint + existing tests pass (all ACs)
  - [x] 8.1 `cd server && cargo fmt --check && cargo clippy -- -D warnings && cargo test`
  - [x] 8.2 `cd client && npx biome check . && npx svelte-check --tsconfig ./tsconfig.app.json`
  - [ ] 8.3 Manually verify: create identity → close browser → reopen → auto-login works without login screen

### Review Follow-ups (AI)

- [x] [AI-Review][MEDIUM] Recovery prompt copy now matches AC #5 and "Start fresh" handles clear failures without unhandled rejections. [client/src/lib/features/identity/RecoveryPrompt.svelte:1-79, client/src/App.svelte:210-212]
- [x] [AI-Review][MEDIUM] Added a logout CTA in the authenticated sidebar and made server logout best-effort (avoid unhandled promise when the server is unreachable). [client/src/App.svelte:156-164, client/src/lib/features/identity/identityStore.svelte.ts:220-236]
- [x] [AI-Review][LOW] LoginView submit button copy now matches reregister mode. [client/src/lib/features/identity/LoginView.svelte:273-287]
- [x] [AI-Review][MEDIUM] Sprint status synced back to `in-progress` since AC gaps remain. [_bmad-output/implementation-artifacts/sprint-status.yaml]
- [ ] [AI-Review][HIGH] AC #3 is not met yet: last-active-location is never written because `currentPath` is only `/` or `/admin` and is filtered out; no navigation attempt exists. Blocked until router + guild/channel routes exist (Epic 4). [client/src/App.svelte:30-76]
- [x] [AI-Review][MEDIUM] ReRegisterPrompt now includes the "Try a different instance" option from AC #6. [client/src/lib/features/identity/ReRegisterPrompt.svelte:129-142]
- [ ] [AI-Review][MEDIUM] Git working tree includes changes outside this story's File List (planning + unrelated story docs); split/clean before committing. [git status]
- [ ] [AI-Review][MEDIUM] Automated client tests for persistence/sync are still missing (Task 7); revisit after Story 2.8 (Vitest baseline) lands.

## Dev Notes

### Architecture Compliance

This story primarily modifies the **client-side identity persistence layer** established in Stories 2.1 and 2.2. No new server endpoints or migrations are needed -- the existing challenge-response and session management infrastructure is sufficient.

1. **Session persistence migration** -- Moving from `sessionStorage` to `localStorage` is a deliberate upgrade from Story 2.2's design. Story 2.2 explicitly deferred persistent sessions: "Do NOT add 'remember me' / persistent login -- that is Story 2.3." The `localStorage` approach is appropriate because:
   - The session token is a server-side UUID (not a JWT) -- server remains authoritative
   - If the token expires server-side, the client's 401 handler triggers transparent re-authentication
   - The challenge-response re-auth is fast (~100-500ms) so even expired tokens don't create UX friction
   - `localStorage` is shared across tabs (unlike `sessionStorage`), enabling cross-tab sync

2. **Cross-tab synchronization** -- Uses the browser's native `StorageEvent` API, which fires when localStorage changes in another tab. This is a lightweight, zero-dependency approach that matches the architecture's "minimal complexity" principle. No BroadcastChannel or SharedWorker needed.

3. **Last-active location** -- Stored in `localStorage` as a simple path string. The architecture specifies `@mateothegreat/svelte5-router 2.15.x` for routing, but the router is not yet installed (routing is implicit in App.svelte's state machine). This story creates the persistence mechanism; Epic 4 (Story 4.1) will wire it to the actual router. For now, it stores/restores the concept of "where was I" without actual navigation.

4. **RecoveryPrompt and ReRegisterPrompt** -- New Svelte components in `features/identity/`. Follow the same pattern as `LoginView.svelte`: Svelte 5 runes, callback props, no legacy patterns. The architecture places all identity UI in `features/identity/`.

5. **No server changes** -- All ACs can be satisfied with client-side changes only. The server's session validation, challenge-response, and registration endpoints are already complete from Stories 2.1 and 2.2. The 404 response from challenge endpoint already distinguishes "identity not found" from other errors.

### Authentication Flow Update

```
Browser Opens                    Server
  │                                │
  ├─ Check localStorage ──►        │
  │  "discool-session" exists?     │
  │                                │
  ├─ [YES: token found]           │
  │  Parse JSON, check expiresAt   │
  │  If expired client-side:       │
  │    → Challenge-response flow   │
  │  If valid client-side:         │
  │    → Set token in api.ts       │
  │    → Continue to main layout   │
  │    → Server validates on       │
  │      first API call (401 if    │
  │      expired server-side →     │
  │      auto re-auth)             │
  │                                │
  ├─ [NO: no token]               │
  │  Load identity from IndexedDB  │
  │  If identity exists:           │
  │    → Challenge-response flow ──►
  │    → Store session in          │
  │      localStorage              │
  │  If no identity:               │
  │    → Show LoginView            │
  │  If identity corrupted:        │
  │    → Show RecoveryPrompt       │
  │                                │
  ├─ [Challenge-response fails     │
  │   with 404]                    │
  │  Identity not on instance:     │
  │    → Show ReRegisterPrompt     │
  │                                │
```

### Cross-Tab Session Sync Design

```
Tab A                          Tab B (listening for StorageEvent)
  │                              │
  ├─ authenticate() succeeds     │
  │  localStorage.setItem(       │
  │    'discool-session', JSON)  │
  │                              ├── StorageEvent fires
  │                              │   event.key === 'discool-session'
  │                              │   event.newValue !== null
  │                              │   → Parse session
  │                              │   → Update identityState.session
  │                              │   → Call setSessionToken()
  │                              │
  ├─ logout()                    │
  │  localStorage.removeItem(    │
  │    'discool-session')        │
  │                              ├── StorageEvent fires
  │                              │   event.key === 'discool-session'
  │                              │   event.newValue === null
  │                              │   → Clear session
  │                              │   → Show login/auth state
```

Note: `StorageEvent` only fires in OTHER tabs, not the one that made the change. This is by design and avoids feedback loops.

### localStorage Keys

| Key | Type | Purpose |
|---|---|---|
| `discool-session` | JSON string | `{ token, expiresAt, user }` -- persistent session |
| `discool-last-location` | string | Last active URL path (e.g., `/guild/abc/channel/def`) |

IndexedDB remains unchanged from Stories 2.1/2.2:
- Database: `discool-identity` (version 1)
- Object store: `keys` -- encrypted secret key, public key, DID, username, avatar color, registeredAt

### Security Considerations

**localStorage vs sessionStorage trade-off:**
- `sessionStorage`: cleared on tab close (more secure against physical access to an open browser)
- `localStorage`: persists across sessions (required for "auto-login on return" -- this story's purpose)
- The security model remains sound because:
  - Tokens are server-validated UUIDs with configurable TTL (default 7 days)
  - Server can revoke tokens at any time (DB delete)
  - The 401 handler in `api.ts` already handles expired/revoked tokens gracefully
  - The private key in IndexedDB is encrypted with a non-extractable AES-GCM key
  - XSS protection: localStorage is same-origin; the private key wrapping key is non-extractable
  - If physical security is a concern, users can explicitly log out (AC #7)

**Token rotation (NOT in scope):**
- Token rotation (issuing a new token on each use) is NOT implemented in this story
- Rationale: The session TTL is configurable and the 401 handler provides transparent re-auth
- If needed in the future, it can be added to the middleware refresh logic without client changes

### Svelte 5 Patterns (Critical)

Continue using Svelte 5 runes as established in Stories 2.1 and 2.2:
- State: `let identityCorrupted = $state(false)`
- Derived: `let canAutoLogin = $derived(identity !== null && !identityCorrupted)`
- Props: `let { onstartfresh, onrecover }: Props = $props()`
- Events: callback props (`onstartfresh`, `onreregister`) -- NOT `createEventDispatcher`
- No legacy patterns (`writable`, `$:`, `on:click` -- use `onclick`)

### Existing Code Reuse

- **`identityState.authenticate()`** from Story 2.2: Already implements challenge-response. This story wraps it with persistent storage.
- **`identityState.restoreSession()`** from Story 2.2: Already validates session expiry. Change storage backend from sessionStorage to localStorage.
- **`setSessionToken()` / `setUnauthorizedHandler()`** from `api.ts` (Story 2.2): Continue using. No changes needed.
- **`loadStoredIdentity()` / `clearStoredIdentity()`** from `crypto.ts` (Story 2.1): Extend with validation for corrupted detection.
- **`identityApi.register()`** from Story 2.1: Reuse in ReRegisterPrompt for re-registration with existing keypair.
- **`identityApi.requestChallenge()` / `verifyChallenge()`** from Story 2.2: Used by `authenticate()`, no changes.
- **`cn()` utility** from `$lib/utils.ts`: For conditional CSS in new components.
- **Avatar color picker pattern** from `LoginView.svelte`: Display stored avatar color in ReRegisterPrompt (read-only display, not a picker).
- **`apiFetch<T>()`** from `$lib/api.ts`: All API calls go through this. No changes needed.
- **`ApiError`** from `$lib/api.ts`: Handle 404/409 errors in ReRegisterPrompt.

### Files to Create

| File | Purpose |
|---|---|
| `client/src/lib/features/identity/RecoveryPrompt.svelte` | UI for corrupted identity recovery |
| `client/src/lib/features/identity/ReRegisterPrompt.svelte` | UI for re-registering on an instance |
| `client/src/lib/features/identity/navigationState.ts` | Last-active location persistence |

### Files to Modify

| File | Change |
|---|---|
| `client/src/lib/features/identity/identityStore.svelte.ts` | Migrate sessionStorage → localStorage, add StorageEvent listener, add `identityCorrupted` and `identityNotRegistered` state fields |
| `client/src/lib/features/identity/crypto.ts` | Enhance `loadStoredIdentity()` with validation and `'corrupted'` status return |
| `client/src/App.svelte` | Add RecoveryPrompt and ReRegisterPrompt to state machine, wire last-active location |

### Project Structure Notes

- All new files are in `client/src/lib/features/identity/` -- matches the architecture's feature-based structure
- No new server files or migrations -- this story is client-only
- `navigationState.ts` is a simple utility module (not a Svelte rune store) because it only interacts with localStorage -- no reactive state needed
- No new directories are created
- Component naming follows PascalCase convention: `RecoveryPrompt.svelte`, `ReRegisterPrompt.svelte`

### Testing Requirements

**Client tests** (if Vitest configured):
- `identityStore` -- session persists in localStorage, restored on init, expired triggers re-auth, StorageEvent sync
- `crypto.ts` -- corrupted identity returns `'corrupted'` status, valid identity returns `'found'`, missing returns `'none'`
- `navigationState.ts` -- save/get/clear round-trip on localStorage
- `RecoveryPrompt.svelte` -- "Start fresh" calls clear, "Recover via email" is disabled
- `ReRegisterPrompt.svelte` -- "Register" calls register API, handles username conflict

**Manual testing checklist:**
1. Create identity → close browser → reopen → lands in main layout without login screen
2. Create identity → open second tab → second tab auto-authenticates
3. Log out in tab A → tab B returns to login view
4. Clear IndexedDB manually → reopen → RecoveryPrompt shown
5. Clear server DB but keep browser data → reopen → ReRegisterPrompt shown
6. Token expires server-side → user continues using app → 401 triggers transparent re-auth

**Linting:** `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test`, `npx biome check .`, `npx svelte-check --tsconfig ./tsconfig.app.json`

### Anti-Patterns to Avoid

- Do NOT store the private key in localStorage -- it stays in IndexedDB with AES-GCM encryption (as established in Story 2.1)
- Do NOT implement token rotation or refresh tokens -- the challenge-response re-auth is fast enough and the 401 handler already covers expired tokens
- Do NOT use `BroadcastChannel` or `SharedWorker` for cross-tab sync -- `StorageEvent` on localStorage is simpler and universally supported
- Do NOT add server-side session listing or management -- that is a future story
- Do NOT implement "remember me" as a separate toggle -- persistence is the default behavior in this story; explicit logout is the opt-out
- Do NOT navigate to a last-active location using direct DOM manipulation -- store the path and let the router (Epic 4) handle navigation
- Do NOT use Svelte 4 patterns in new components -- Svelte 5 runes only
- Do NOT clear IndexedDB on auth failure -- the keypair may still be valid even if the server record is missing (AC #6)
- Do NOT show cryptographic terms ("keypair", "Ed25519", "DID", "IndexedDB") in any user-facing text
- Do NOT add unread tracking or unread indicators in this story -- that requires WebSocket message infrastructure (Epic 6). The AC mentions "unread indicators" but this depends on messaging; defer to Story 6.8 (Typing Indicators and Channel Activity)
- Do NOT use `window.location.reload()` or hard navigation for state transitions -- use Svelte's reactive state to re-render

### Previous Story Intelligence

**From Story 2.2 (Challenge-Response Authentication):**
- `sessionStorage` was chosen deliberately as a security-first default. Story 2.2's anti-patterns explicitly said: "Do NOT add 'remember me' / persistent login -- that is Story 2.3."
- The 401 unauthorized handler in `api.ts` already clears the session and calls `authenticate()`. This means transitioning to localStorage is safe -- if the stored token is invalid, the re-auth flow already works.
- `restoreSession()` validates `expiresAt` by parsing as a Date and comparing to now. This logic stays the same; only the storage backend changes.
- The `identityState.initialize()` flow: load identity → restore session → if no session, authenticate. This stays the same.
- Senior dev review added: client reacts to runtime 401s (expired/revoked sessions), clears session + re-authenticates. This is critical for localStorage persistence -- tokens may expire between browser sessions.
- App.svelte uses `$state()` for all local state (fixed in review). Continue this pattern.
- `onclick` (not `on:click`) is used for event handlers in Svelte 5 (fixed in review). Continue this pattern.

**From Story 2.1 (Client-Side Keypair Generation):**
- `loadStoredIdentity()` returns `null` if no identity is stored OR if `registeredAt` is not set. Need to add more granular error detection.
- `clearStoredIdentity()` deletes the IndexedDB record and database. Reuse for "Start fresh" in RecoveryPrompt.
- LoginView maps server error messages to user-friendly copy -- apply same pattern in ReRegisterPrompt for 409 conflicts.
- `encryptAndStoreKey()` stores the identity record in IndexedDB. The stored record includes: wrappingKey, encryptedSecretKey, iv, publicKey, didKey, username, avatarColor, registeredAt.

### Git Intelligence

**Recent commit patterns:**
```
0a3b9db feat(auth): add challenge-response auth, session model & management, middleware, and auth services; update client identity flows
0753378 fix: map internal errors in LoginView, fix base58 edge-case, add DID multibase test
```

- Story 2.2 was the most recent feature commit. This story directly continues that work.
- The codebase is clean (no uncommitted changes on main branch).
- The existing auth infrastructure (challenge-response, sessions, middleware) is fully tested and reviewed.

Expected commit for this story: `feat(auth): add persistent sessions, cross-tab sync, and identity recovery prompts`

### Latest Technical Information

**localStorage API (Web standard):**
- `localStorage.setItem(key, JSON.stringify(value))` -- persists across browser sessions
- `localStorage.getItem(key)` -- returns string or null
- `localStorage.removeItem(key)` -- removes key
- Storage limit: ~5-10MB per origin (more than sufficient for session JSON)
- Same-origin policy applies (secure)

**StorageEvent API (Web standard):**
- `window.addEventListener('storage', (event: StorageEvent) => { ... })` -- fires when localStorage changes in another tab/window of the same origin
- `event.key` -- the key that changed
- `event.newValue` -- the new value (null if removed)
- `event.oldValue` -- the previous value
- Does NOT fire in the tab that made the change (prevents feedback loops)
- Supported in all modern browsers

**Svelte 5 runes module pattern:**
- `identityStore.svelte.ts` exports a reactive state object using `$state`
- Module-level effects with `$effect` run when the module is first imported
- For cleanup: `$effect` returns a cleanup function that runs on re-execution, but module-level effects persist for the app lifetime
- `StorageEvent` listener should be added inside an `$effect` in the store initialization, or simply in `initialize()` since it's called once on mount

### Scope Boundaries (What This Story Does NOT Include)

- **Unread indicators**: Mentioned in the AC from epics but depends on WebSocket messaging (Epic 6, Story 6.8). This story defers unread tracking entirely.
- **Last active guild/channel navigation**: The mechanism (localStorage) is built, but actual navigation to a guild/channel requires the router + guild data (Epic 4). This story stores the path; Epic 4 reads it.
- **Email-based recovery**: The RecoveryPrompt has a disabled "Recover via email" button as a placeholder. Full implementation is Story 2.7.
- **Multi-device session sync**: This story handles multi-TAB (same browser). Cross-device sessions require server-side session listing, which is a future enhancement.
- **Token rotation**: Not needed given the transparent re-auth flow.
- **User preferences/settings page**: Profile/settings UI is Story 2.4. This story only handles the persistence plumbing.

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Story 2.3] -- Story statement and acceptance criteria
- [Source: _bmad-output/planning-artifacts/epics.md#Epic 2] -- Epic objectives (FR1-7, NFR11, NFR13)
- [Source: _bmad-output/planning-artifacts/epics.md#FR6] -- "Users can persist their identity in browser storage and resume sessions"
- [Source: _bmad-output/planning-artifacts/architecture.md#Authentication & Security] -- Session management: server-side session store, short TTL with refresh
- [Source: _bmad-output/planning-artifacts/architecture.md#Frontend Architecture] -- Svelte 5 runes, feature-based structure
- [Source: _bmad-output/planning-artifacts/architecture.md#State Boundaries] -- Auth/session in authStore.svelte.ts, identity flow
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Loading Patterns] -- <200ms no loading state, 200ms-2s skeleton
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Onboarding Flow] -- <10s existing identity join
- [Source: _bmad-output/implementation-artifacts/2-1-client-side-keypair-generation-and-identity-creation.md] -- IndexedDB storage, loadStoredIdentity(), clearStoredIdentity(), LoginView patterns
- [Source: _bmad-output/implementation-artifacts/2-2-challenge-response-authentication-and-session-management.md] -- sessionStorage usage, 401 handler, restoreSession(), authenticate(), anti-patterns deferring to Story 2.3

## Dev Agent Record

### Agent Model Used

Copilot CLI 0.0.414

### Debug Log References

- `cd client && npm run lint && npm run check && npm run build`
- `cd server && cargo fmt --check && cargo clippy -- -D warnings && cargo test`

### Completion Notes List

- Migrated client session persistence from `sessionStorage` to `localStorage` (`discool-session`).
- Added cross-tab session sync via `StorageEvent` on `discool-session`.
- Added last-active-location persistence helpers (`discool-last-location`) and wired to `App.svelte` (router hook for Epic 4).
- Added corrupted identity detection in `loadStoredIdentity()` with explicit `{ status: ... }` return and RecoveryPrompt UX.
- Added "identity not registered on this instance" handling (404 from challenge) with ReRegisterPrompt + re-register flow that preserves the keypair.
- Note: No Vitest client test runner is configured in this repo; Task 7 is currently blocked/pending (adding a test runner would require new deps).
- Manual verification (Task 8.3) still pending.

### File List

- `_bmad-output/implementation-artifacts/sprint-status.yaml`
- `client/src/App.svelte`
- `client/src/lib/features/identity/LoginView.svelte`
- `client/src/lib/features/identity/ReRegisterPrompt.svelte`
- `client/src/lib/features/identity/RecoveryPrompt.svelte`
- `client/src/lib/features/identity/crypto.ts`
- `client/src/lib/features/identity/identityStore.svelte.ts`
- `client/src/lib/features/identity/navigationState.ts`

## Senior Developer Review (AI)

**Outcome:** Changes Requested (after quick fixes)

### Findings Fixed

- **MEDIUM:** Recovery prompt copy now matches AC #5 and the "Start fresh" action handles clear failures without unhandled promise rejections. [client/src/lib/features/identity/RecoveryPrompt.svelte:1-79, client/src/App.svelte:210-212]
- **MEDIUM:** Added a logout CTA in the authenticated sidebar and made server logout best-effort (avoid unhandled promise when the server is unreachable). [client/src/App.svelte:156-164, client/src/lib/features/identity/identityStore.svelte.ts:220-236]
- **LOW:** LoginView submit button copy now matches reregister mode. [client/src/lib/features/identity/LoginView.svelte:273-287]
- **MEDIUM:** Sprint status synced back to `in-progress` since AC gaps remain. [_bmad-output/implementation-artifacts/sprint-status.yaml]
- **MEDIUM:** ReRegisterPrompt now includes a "Try a different instance" option (AC #6). [client/src/lib/features/identity/ReRegisterPrompt.svelte:129-142]
- **MEDIUM:** Clear persisted session token on 401 before re-authentication to avoid stale token loops. [client/src/lib/features/identity/identityStore.svelte.ts:323-330]
- **HIGH:** Prevent cross-tab logout/session changes from being undone by an in-flight `authenticate()` (auth epoch guard). [client/src/lib/features/identity/identityStore.svelte.ts]
- **MEDIUM:** If re-register hits "Identity already registered on this instance", fall back to signing in instead of showing a misleading network error. [client/src/lib/features/identity/ReRegisterPrompt.svelte]
- **LOW:** Hardened `restoreSession()` to validate `avatarColor` as a hex color to avoid unsafe/garbage values from localStorage. [client/src/lib/features/identity/identityStore.svelte.ts]
- **MEDIUM:** StorageEvent "session updated" path now re-authenticates if `restoreSession()` fails, preventing a stuck unauthenticated state after cross-tab updates; `restoreSession()` also guards localStorage access errors to avoid unhandled rejections. [client/src/lib/features/identity/identityStore.svelte.ts]
- **MEDIUM:** Hardened localStorage writes/removals for both session persistence and last-location helpers so startup/logout won't crash if browser storage is blocked or throws. [client/src/lib/features/identity/identityStore.svelte.ts, client/src/lib/features/identity/navigationState.ts]
- **MEDIUM:** `loadStoredIdentity()` now validates the stored wrapping key/IV/encrypted secret key shape so corrupted identities are detected earlier (AC #5). [client/src/lib/features/identity/crypto.ts]
- **LOW:** LoginView reregister init no longer relies on an unsafe `avatarColor` cast; it now guards palette values. [client/src/lib/features/identity/LoginView.svelte]
- **MEDIUM:** Last-location persistence now uses `window.location.pathname` for `currentPath` (when available), avoiding hard-coded `/`/`/admin` paths and preparing for router wiring (AC #3). [client/src/App.svelte:30-76]
- **MEDIUM:** `restoreSession()` now clears any in-memory session/token if `discool-session` is missing, and treats a missing `avatarColor` as `null` to avoid breaking older session payloads. [client/src/lib/features/identity/identityStore.svelte.ts:282-299, 350-372]
- **MEDIUM:** When identity storage is detected as corrupted, `discool-last-location` is cleared as well (avoid restoring stale navigation state after a "start fresh"). [client/src/lib/features/identity/identityStore.svelte.ts:90-100]
- **MEDIUM:** 401 unauthorized handler now bumps `authEpoch` and resets `authenticating` before re-auth, preventing in-flight auth from reinstating a stale/invalid session. [client/src/lib/features/identity/identityStore.svelte.ts:388-397]

### Findings Remaining

- **HIGH:** AC #3 is still not met: there is no automatic resume navigation to the last active location (router + guild/channel routes are not in place yet). Last-location is stored/read, but the app does not yet drive real URL navigation. Blocked until Epic 4. [client/src/App.svelte:30-76]
- **MEDIUM:** Automated client tests for persistence/sync are still missing (Task 7); revisit after Story 2.8 (Vitest baseline) lands.

### Git vs Story Discrepancies

Uncommitted changes exist outside this story's File List (should be split/clean before commit):

- `_bmad-output/implementation-artifacts/1-5-first-run-admin-setup-screen.md`
- `_bmad-output/implementation-artifacts/1-6-instance-health-dashboard.md`
- `_bmad-output/implementation-artifacts/1-7-data-export-and-backup.md`
- `_bmad-output/planning-artifacts/epics.md`
- `_bmad-output/implementation-artifacts/2-8-vitest-setup-and-client-test-baseline.md`

### Verification

- `cd client && npm run lint && npm run check`
- `cd client && npm run build`

## Change Log

- 2026-02-23: Implemented persistent sessions (localStorage), cross-tab sync, last-location persistence, and recovery/re-register prompts.
- 2026-02-23: Senior dev review: fixed RecoveryPrompt UX copy + safe clear handling, added Logout CTA, and updated reregister button copy; remaining AC gaps -> status stays in-progress.
- 2026-02-23: Senior dev review: fixed auth race around cross-tab logout/session changes, improved re-register conflict handling, and hardened session restore validation.
- 2026-02-23: Senior dev review: fixed cross-tab session sync edge-case (invalid/expired session update now triggers re-auth) and hardened session restore against localStorage access errors.
- 2026-02-24: Senior dev review: hardened localStorage writes/removals for session + last-location helpers.
- 2026-02-24: Senior dev review: strengthened corrupted identity validation (wrapping key/IV/secret blob), removed unsafe avatar color cast, and aligned last-location tracking with `window.location.pathname`.
- 2026-02-24: Senior dev review: improved session restore tolerance (missing localStorage entry / missing avatarColor), cleared last-location on corrupted identity, and invalidated auth epoch on 401 before re-auth.
