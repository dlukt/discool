# Story 1.5: First-Run Admin Setup Screen

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As an **operator**,
I want a first-run setup screen in the SPA when I initialize a new instance,
So that I can create the admin identity and configure basic instance settings without a CLI.

## Acceptance Criteria

1. **Given** the instance has a fresh database with no admin identity
   **When** the operator opens the instance URL in a browser
   **Then** a first-run setup screen is displayed (not the normal guild view)

2. **Given** the first-run setup screen is displayed
   **When** the operator enters a username and optionally picks an avatar color
   **Then** the admin identity is created

3. **Given** the first-run setup screen is displayed
   **When** the operator sets an instance name and description
   **Then** the instance settings are saved

4. **Given** the first-run setup screen is displayed
   **When** the operator toggles P2P discovery opt-in/opt-out
   **Then** the discovery preference is persisted (default: opt-in)

5. **Given** the operator has completed all required fields and submits the form
   **When** the setup request is processed
   **Then** the admin identity is persisted, the instance is marked as initialized, and the SPA transitions to the normal app view

6. **Given** the instance has already been initialized
   **When** the operator (or any user) opens the instance URL
   **Then** the normal SPA is loaded (not the setup screen)

7. **Given** the instance has already been initialized
   **When** a `POST /api/v1/instance/setup` request is made
   **Then** it returns 409 Conflict

8. **Given** the setup form is submitted with missing required fields (username or instance name)
   **When** the request is processed
   **Then** it returns 422 with a descriptive validation error

9. **Given** the setup screen is displayed
   **Then** it follows the Dual Core design system (fire CTA button, single-column form, labels above inputs, dark zinc background)

## Tasks / Subtasks

- [x] Task 1: Database migration for instance settings and admin user (AC: #1, #5, #6)
  - [x] 1.1 Create `server/migrations/0002_instance_and_admin.sql`
  - [x] 1.2 Add `instance_settings` table: `key TEXT PRIMARY KEY`, `value TEXT NOT NULL`
  - [x] 1.3 Add `admin_users` table: `id TEXT PRIMARY KEY`, `username TEXT NOT NULL UNIQUE`, `avatar_color TEXT`, `created_at TEXT NOT NULL DEFAULT (CURRENT_TIMESTAMP)`
  - [x] 1.4 Verify migration runs cleanly on SQLite and PostgreSQL via `cargo test`

- [x] Task 2: Add AppError variants for setup validation (AC: #7, #8)
  - [x] 2.1 Add `Conflict(String)` variant to `AppError` in `server/src/error.rs` — maps to 409
  - [x] 2.2 Add `ValidationError(String)` variant to `AppError` — maps to 422
  - [x] 2.3 Both variants produce `{"error": {"code": "...", "message": "...", "details": {}}}` JSON per architecture spec

- [x] Task 3: Instance handler module with query functions and API handlers (AC: #1, #2, #3, #4, #5, #6, #7, #8)
  - [x] 3.1 Create `server/src/handlers/instance.rs`
  - [x] 3.2 Define `SetupRequest` struct (serde Deserialize): `admin_username: Option<String>` (validated required), `avatar_color: Option<String>`, `instance_name: Option<String>` (validated required), `instance_description: Option<String>`, `discovery_enabled: Option<bool>` (default true)
  - [x] 3.3 Define `InstanceStatus` response struct (serde Serialize): `initialized: bool`, `name: Option<String>`, `description: Option<String>`, `discovery_enabled: Option<bool>`, `admin: Option<AdminInfo>`
  - [x] 3.4 Define `AdminInfo` struct (serde Serialize): `username: String`, `avatar_color: Option<String>`
  - [x] 3.5 Implement `is_initialized()` async fn: query `SELECT value FROM instance_settings WHERE key = 'initialized_at' LIMIT 1` against the pool; returns `bool`
  - [x] 3.6 Implement `get_instance` handler (`GET /api/v1/instance`): calls `is_initialized()`, if true also fetches instance name/description/discovery/admin from DB; wraps in `{"data": ...}`
  - [x] 3.7 Implement `setup_instance` handler (`POST /api/v1/instance/setup`): validates not-yet-initialized (409), validates required fields (422), inserts `admin_users` row, inserts `instance_settings` rows (`instance_name`, `instance_description`, `discovery_enabled`, `initialized_at`), returns `{"data": ...}` with full InstanceStatus
  - [x] 3.8 Use `uuid::Uuid::new_v4().to_string()` for admin user ID
  - [x] 3.9 Use `tracing::info!` to log successful setup (with username, no sensitive data)

- [x] Task 4: Wire instance routes into router (AC: #1, #6)
  - [x] 4.1 Add `mod instance;` declaration in `server/src/handlers/mod.rs`
  - [x] 4.2 Add `.route("/instance", get(instance::get_instance))` to the `/api/v1` router
  - [x] 4.3 Add `.route("/instance/setup", post(instance::setup_instance))` to the `/api/v1` router
  - [x] 4.4 Import `axum::routing::post` in `mod.rs`
  - [x] 4.5 Verify new routes are within the `tracked` router so metrics track them when enabled

- [x] Task 5: Client-side API client module (AC: #1, #5)
  - [x] 5.1 Create `client/src/lib/api.ts` with a typed `apiFetch<T>()` wrapper that handles `{"data": T}` / `{"error": {...}}` response format
  - [x] 5.2 Define `ApiError` class with `code`, `message`, `details` fields
  - [x] 5.3 Implement `getInstanceStatus(): Promise<InstanceStatus>` function
  - [x] 5.4 Implement `submitSetup(data: SetupRequest): Promise<InstanceStatus>` function
  - [x] 5.5 Define TypeScript types: `InstanceStatus`, `AdminInfo`, `SetupRequest`

- [x] Task 6: Setup page component (AC: #2, #3, #4, #5, #8, #9)
  - [x] 6.1 Create `client/src/lib/components/SetupPage.svelte`
  - [x] 6.2 Single-column form layout: labels above inputs, max-width container, centered on page
  - [x] 6.3 Header: Discool logo/name + "Set up your instance" heading
  - [x] 6.4 Admin username input (required, text, placeholder "Pick a username")
  - [x] 6.5 Avatar color picker: 8 predefined color swatches, one selected by default, shows initials preview
  - [x] 6.6 Instance name input (required, text, placeholder "My Instance")
  - [x] 6.7 Instance description textarea (optional, placeholder "A short description of your instance")
  - [x] 6.8 P2P discovery toggle (checkbox/switch, default: checked/on, label "Allow this instance to be discovered by others")
  - [x] 6.9 Fire CTA button "Complete Setup" at the bottom
  - [x] 6.10 Client-side validation on blur: username required (min 1 char), instance name required (min 1 char)
  - [x] 6.11 Inline error messages below fields in destructive color
  - [x] 6.12 Loading state on submit: button disabled, shows "Setting up..." text
  - [x] 6.13 Server error display: toast or inline error message above form
  - [x] 6.14 On success: dispatch event or callback to parent to transition to normal app view

- [x] Task 7: App initialization and conditional rendering (AC: #1, #6)
  - [x] 7.1 Update `client/src/App.svelte` to fetch `GET /api/v1/instance` on mount
  - [x] 7.2 Show loading state (skeleton or minimal spinner with "Loading..." text) while checking
  - [x] 7.3 If `initialized: false` → render `<SetupPage>` component
  - [x] 7.4 If `initialized: true` → render normal app scaffold (existing content)
  - [x] 7.5 If fetch fails (network error) → show error state: "Could not connect to the server. Is it running?"
  - [x] 7.6 After successful setup, re-fetch instance status and transition to normal view

- [x] Task 8: Server tests (AC: #1, #2, #5, #6, #7, #8)
  - [x] 8.1 Unit test: `is_initialized()` returns false on fresh DB (after migrations)
  - [x] 8.2 Unit test: `GET /api/v1/instance` returns `{"data": {"initialized": false, ...}}` on fresh DB
  - [x] 8.3 Unit test: `POST /api/v1/instance/setup` with valid data returns 200 with initialized=true
  - [x] 8.4 Unit test: `GET /api/v1/instance` returns initialized=true with admin info after setup
  - [x] 8.5 Unit test: `POST /api/v1/instance/setup` returns 409 if already initialized
  - [x] 8.6 Unit test: `POST /api/v1/instance/setup` returns 422 for empty username
  - [x] 8.7 Unit test: `POST /api/v1/instance/setup` returns 422 for empty instance name
  - [x] 8.8 Unit test: `POST /api/v1/instance/setup` defaults `discovery_enabled` to true when not provided
  - [x] 8.9 Unit test: avatar_color is optional and stored correctly when provided
  - [x] 8.10 Integration test: fresh server `GET /api/v1/instance` returns `{"data": {"initialized": false}}`
  - [x] 8.11 Integration test: `POST /api/v1/instance/setup` then `GET /api/v1/instance` returns initialized=true with correct data
  - [x] 8.12 Integration test: double `POST /api/v1/instance/setup` returns 409 on second call
  - [x] 8.13 Update `write_server_config()` test helper if needed (no changes expected — existing config is sufficient)

- [x] Task 9: Code quality (AC: all)
  - [x] 9.1 Run `cargo fmt` and fix any formatting issues
  - [x] 9.2 Run `cargo clippy -- -D warnings` and resolve all warnings
  - [x] 9.3 Run `cargo test` and verify all tests pass
  - [x] 9.4 Run `cargo build --release` and verify it succeeds
  - [x] 9.5 Run `npx biome check .` in client directory and fix issues
  - [x] 9.6 Run `npx svelte-check --tsconfig ./tsconfig.app.json` in client directory and fix issues

## Dev Notes

### Architecture Compliance

**This story introduces the first full-stack feature: a first-run setup screen. It spans both the Rust backend (new API endpoints, new migration) and the Svelte frontend (new component, API client, conditional app rendering). This is the first story that touches the client-side code beyond the initial scaffold.**

#### Server Module Location (per architecture doc)

```
server/src/handlers/
├── mod.rs        # Route registration, router builder
├── health.rs     # /healthz, /readyz, /metrics
├── instance.rs   # /api/v1/instance, /api/v1/instance/setup (NEW)
├── auth.rs       # Future: /api/v1/auth/*
├── guilds.rs     # Future: /api/v1/guilds/*
└── ...
```

#### Client Component Location (per architecture doc)

```
client/src/
├── App.svelte                  # Root: conditional setup vs. normal app
├── lib/
│   ├── api.ts                  # API client wrapper (NEW)
│   ├── components/
│   │   └── SetupPage.svelte    # First-run setup form (NEW)
│   └── utils.ts                # Existing utils
└── main.ts
```

Note: The architecture doc specifies a `features/` directory structure with co-located stores and API hooks. For this story, the setup page is a one-time-use component that doesn't justify a full feature directory. The API client (`api.ts`) is placed in `lib/` as a shared utility. Future stories that introduce TanStack Query and feature-specific stores will establish the full feature directory pattern.

#### API Design (per architecture conventions)

| Endpoint | Method | Purpose | Response |
|---|---|---|---|
| `/api/v1/instance` | GET | Check initialization status | `{"data": {"initialized": bool, ...}}` |
| `/api/v1/instance/setup` | POST | Perform first-run setup | `{"data": {"initialized": true, ...}}` |

All responses follow the `{"data": ...}` / `{"error": {"code": "...", "message": "...", "details": {}}}` format.

**HTTP status codes used:**
- 200: Success (GET instance status, POST setup success)
- 409: Conflict (setup already completed)
- 422: Validation error (missing required fields)
- 500: Internal error

#### Database Schema

```sql
-- 0002_instance_and_admin.sql

CREATE TABLE IF NOT EXISTS instance_settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS admin_users (
    id TEXT PRIMARY KEY,
    username TEXT NOT NULL UNIQUE,
    avatar_color TEXT,
    created_at TEXT NOT NULL DEFAULT (CURRENT_TIMESTAMP)
);
```

The `instance_settings` table stores key-value pairs:
- `initialized_at` — timestamp, presence indicates setup is complete
- `instance_name` — operator-chosen instance name
- `instance_description` — optional description
- `discovery_enabled` — "true" or "false" string

The `admin_users` table stores the admin identity. For now this is a simple username + avatar color. Cryptographic identity (Ed25519 keypair, challenge-response auth) is introduced in Epic 2 (Story 2.1+). The `admin_users` table will be extended or migrated at that point.

#### SetupRequest JSON Format

```json
{
  "admin_username": "tomas",
  "avatar_color": "#3399ff",
  "instance_name": "My Instance",
  "instance_description": "A cool place to hang out",
  "discovery_enabled": true
}
```

- `admin_username`: required, non-empty, trimmed
- `avatar_color`: optional, hex color string (e.g., "#3399ff"), defaults to a predefined color if not provided
- `instance_name`: required, non-empty, trimmed
- `instance_description`: optional, can be empty string or null
- `discovery_enabled`: optional, defaults to `true` if not provided

#### InstanceStatus Response Format

Fresh instance:
```json
{"data": {"initialized": false}}
```

After setup:
```json
{
  "data": {
    "initialized": true,
    "name": "My Instance",
    "description": "A cool place to hang out",
    "discovery_enabled": true,
    "admin": {
      "username": "tomas",
      "avatar_color": "#3399ff"
    }
  }
}
```

#### Setup Screen Design (per UX spec)

The setup screen follows the Dual Core design system and the UX specification's Journey 3 (Instance Deployment & First Run):

- **Layout:** Single-column form, centered, max-width ~480px, dark zinc background
- **Header:** "Discool" name/logo + "Set up your instance" subheading
- **Sections:**
  1. Admin identity (username + avatar color picker)
  2. Instance settings (name + description)
  3. Discovery toggle
  4. Submit button
- **Button:** Fire orange CTA "Complete Setup" — the ONE primary action
- **Validation:** On blur, inline errors below fields in destructive red
- **Typography:** Labels above inputs (per UX form patterns), `text-sm` for labels, `text-base` for inputs
- **Spacing:** `space-4` (16px) between form groups, `space-2` (8px) between label and input

**Avatar color picker:** 8 predefined colors displayed as clickable swatches. Selecting a color shows an initials-based preview (first letter of username in a colored circle). This is a lightweight alternative to file upload — full avatar upload infrastructure will be added in a later story.

Predefined avatar colors:
```
#3b82f6 (blue), #ef4444 (red), #22c55e (green), #f59e0b (amber),
#8b5cf6 (purple), #ec4899 (pink), #06b6d4 (cyan), #f97316 (orange)
```

### Existing Code to Modify

| File | Change |
|---|---|
| `server/src/error.rs` | Add `Conflict` and `ValidationError` variants to `AppError` |
| `server/src/handlers/mod.rs` | Add `mod instance;`, wire new routes, import `post` |
| `client/src/App.svelte` | Replace scaffold content with initialization check + conditional rendering |

### Files to Create

| File | Purpose |
|---|---|
| `server/migrations/0002_instance_and_admin.sql` | Schema for instance settings and admin users |
| `server/src/handlers/instance.rs` | `get_instance()` and `setup_instance()` handlers with query logic |
| `client/src/lib/api.ts` | Typed fetch wrapper for API calls |
| `client/src/lib/components/SetupPage.svelte` | First-run setup form component |

### Project Structure Notes

- `/api/v1/instance` and `/api/v1/instance/setup` are nested under the existing `/api/v1` router — they are tracked by Prometheus metrics when enabled
- The `instance` module follows the established handler pattern: one file per domain concern
- No new Cargo dependencies needed — `uuid`, `serde`, `serde_json`, `sqlx`, `axum` are already in `Cargo.toml`
- No new npm dependencies needed — the client uses native `fetch` wrapped in a typed helper. TanStack Query will be introduced when the main app data fetching layer is built (likely Epic 2 or 3)
- The `admin_users` table uses `TEXT` for the `id` column (UUID as string) for sqlx `Any` backend compatibility across SQLite and PostgreSQL

### Testing Requirements

**Server unit tests (in `server/src/handlers/instance.rs` `#[cfg(test)]` module):**
- `is_initialized()` returns false on fresh DB
- `GET /api/v1/instance` returns correct JSON shape for uninitialized state
- `POST /api/v1/instance/setup` with valid data succeeds and persists all fields
- `GET /api/v1/instance` returns correct JSON shape after setup with admin info
- `POST /api/v1/instance/setup` returns 409 on second call
- `POST /api/v1/instance/setup` returns 422 for empty/missing username
- `POST /api/v1/instance/setup` returns 422 for empty/missing instance name
- `discovery_enabled` defaults to true when omitted from request
- `avatar_color` is optional and correctly stored

For unit testing, create a test helper that builds an `AppState` with a real `sqlite::memory:` pool (with migrations run), then calls the handler functions directly via `axum::test::TestClient` or by invoking the handler with `State(state)` and extracting the response.

**Server integration tests (in `server/tests/server_binds_to_configured_port.rs`):**
- Fresh server returns `{"data": {"initialized": false}}` from `GET /api/v1/instance`
- `POST /api/v1/instance/setup` with valid JSON body returns 200, then `GET /api/v1/instance` returns initialized=true
- Second `POST /api/v1/instance/setup` returns 409

For integration tests, extend the existing `http_response()` helper to support POST requests with JSON body. Add a `http_post()` helper function.

**Client tests:**
- Svelte component tests are deferred — the testing infrastructure (vitest + @testing-library/svelte) is not yet set up. This is appropriate for the first client-side story. Testing infrastructure should be introduced in a dedicated story or when the client has enough components to justify the setup cost.

**Linting:** `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test`, `npx biome check .`, `npx svelte-check` — all must pass.

### Anti-Patterns to Avoid

- Do NOT use `unwrap()` or `expect()` in handler code — use `?` with `AppError`
- Do NOT return bare JSON objects — always wrap in `{"data": ...}` or `{"error": ...}`
- Do NOT call `fetch()` directly in Svelte components — use the `api.ts` wrapper
- Do NOT use `any` type in TypeScript — define proper types
- Do NOT add authentication/authorization checks to the setup endpoint — it's only available before initialization, which is its own guard
- Do NOT log the avatar color or instance description at info level — only log the username and instance name
- Do NOT use `console.log` in client code — use structured logging or dev-only debug
- Do NOT make the setup form multi-step/wizard — it's a single form per UX spec (the setup screen is one screen, not a wizard)
- Do NOT add a router library for this story — conditional rendering in App.svelte is sufficient. The router will be introduced when there are actual routes to navigate between
- Do NOT implement file-based avatar upload — use color-based initials avatar. File upload infrastructure comes in a later story
- Do NOT introduce TanStack Query for this story — the one-time setup flow doesn't benefit from caching/deduplication. TanStack Query will be introduced when the main app data layer is built

### Previous Story Intelligence

**From Story 1.4 (Health Check and Metrics Endpoints):**
- `AppState { config: Arc<Config>, pool: AnyPool, start_time: Instant }` in `lib.rs`
- `schema_metadata` table exists — readyz queries it. Instance setup adds `instance_settings` as a sibling table with the same key-value pattern
- Handler pattern: `pub async fn handler_name(State(state): State<AppState>) -> impl IntoResponse`
- JSON response pattern: `(StatusCode::OK, Json(json!({...})))` tuple
- Router pattern: `Router::new().route(path, method(handler))`, nested via `.nest("/api/v1", api)`
- Error pattern: `AppError` enum → `IntoResponse` with JSON `{"error": {"code", "message", "details"}}`
- Metrics layer wraps the `tracked` router — new API routes in the `/api/v1` nest are automatically tracked
- Integration test helpers: `spawn_server()`, `wait_for_bind()`, `http_status()`, `http_response()`, `response_body()`, `write_server_config()`
- Config helper `write_server_config()` generates TOML with `[server]`, `[log]`, `[database]`, optional `[metrics]` sections — no changes needed for this story (setup uses defaults)

**From Story 1.3 (Database Connection and Migration System):**
- Migrations embedded via `sqlx::migrate!("./migrations")` in `db/migrate.rs`
- Any new `.sql` file in `server/migrations/` is automatically picked up
- Pool config: max 5 connections, SQLite in-memory forced to 1 connection
- DB error logging: redact DB URL from error messages

**Review follow-ups from previous stories:**
- No `expect("validated")` — use explicit `Option` handling
- DB URL redaction is wired — maintain pattern in any new DB-touching code
- Health endpoints are outside metrics tracking — new API routes should be inside (they are, since they're nested under `/api/v1`)

### Git Intelligence

**Recent commits:**
- `df8d403` — feat: add health check and Prometheus metrics endpoints (Story 1.4)
- `3a676ee` — chore: commit from Copilot CLI (Story 1.3)
- `e204ccb` — Add configuration system with structured logging support (Story 1.2)
- `a828f94` — Initial commit: BMAD framework, Svelte client, and Rust server (Story 1.1)

**Patterns established:**
- Commit messages use conventional commits: `feat:`, `chore:`, `fix:`
- Rust module pattern: `mod.rs` + separate files per concern
- Config pattern: typed structs with `serde::Deserialize`, `Option<T>` for optional sections
- Error pattern: `AppError` enum with `IntoResponse`, structured JSON
- State pattern: `AppState` struct with `config: Arc<Config>`, `pool: AnyPool`, `start_time: Instant`
- Test pattern: inline `#[cfg(test)]` for unit tests, `server/tests/` for integration tests
- Integration test pattern: process-spawning with `TestServer`, `wait_for_bind()`, raw HTTP via `TcpStream`
- Router pattern: `handlers::router(state)` builds the full Axum router with layers
- Client: Svelte 5, Vite, Tailwind CSS 4, Biome linter, no runtime dependencies beyond dev tooling

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Story 1.5] — Full acceptance criteria and story statement
- [Source: _bmad-output/planning-artifacts/epics.md#FR10] — "Operators can access an admin setup screen on first launch to initialize the instance"
- [Source: _bmad-output/planning-artifacts/architecture.md#Format Patterns] — REST API response format: `{"data": ...}` / `{"error": ...}`
- [Source: _bmad-output/planning-artifacts/architecture.md#Process Patterns] — Error handling: `Result<Json<T>, AppError>`, `IntoResponse` mapping
- [Source: _bmad-output/planning-artifacts/architecture.md#Enforcement Guidelines] — Anti-patterns to avoid in handlers and frontend
- [Source: _bmad-output/planning-artifacts/architecture.md#Complete Project Directory Structure] — Handler file structure, client feature structure
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Journey 3] — Instance Deployment & First Run flow (Tomas)
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Form Patterns] — Single-column, labels above inputs, validate on blur, fire CTA
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Design Direction Decision] — Dual Core: ice for navigation, fire for CTAs, zinc foundation
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Button Hierarchy] — Fire orange for primary action, max ONE per context
- [Source: _bmad-output/implementation-artifacts/1-4-health-check-and-metrics-endpoints.md] — Handler patterns, router wiring, test infrastructure, AppState shape

## Dev Agent Record

### Agent Model Used

GitHub Copilot CLI

### Debug Log References

- `cd server && cargo test`
- `cd server && cargo fmt`
- `cd server && cargo clippy -- -D warnings`
- `cd server && cargo build --release`
- `cd client && npm ci`
- `cd client && npm run lint`
- `cd client && npm run check`

### Completion Notes List

- Added migration `0002_instance_and_admin.sql` with `instance_settings` + `admin_users`.
- Added `AppError::Conflict` (409) + `AppError::ValidationError` (422) with standard `{"error": ...}` JSON.
- Implemented `GET /api/v1/instance` and `POST /api/v1/instance/setup` with DB persistence + success log.
- Made setup inserts DB-agnostic (SQLite/Postgres) via `sqlx::QueryBuilder` and added a race-safe `initialized_at` insert guard.
- Added Svelte setup screen + typed API client; App now renders setup vs normal view based on instance status.
- Added unit + integration tests for instance status and setup flows.

### File List

- `_bmad-output/implementation-artifacts/sprint-status.yaml` (updated story status)
- `_bmad-output/implementation-artifacts/1-5-first-run-admin-setup-screen.md` (updated tasks/status/record)
- `AGENTS.md` (repo-level guidelines for agents)
- `server/migrations/0002_instance_and_admin.sql` (new)
- `server/src/error.rs` (updated AppError variants)
- `server/src/handlers/mod.rs` (wired instance routes)
- `server/src/handlers/instance.rs` (new handlers + tests)
- `server/tests/server_binds_to_configured_port.rs` (added instance integration tests + POST helper)
- `client/src/lib/api.ts` (new typed API client)
- `client/src/lib/components/SetupPage.svelte` (new setup screen)
- `client/src/App.svelte` (conditional setup vs normal view)

## Senior Developer Review (AI)

### Summary

✅ **ACs validated end-to-end** (fresh DB shows setup screen; setup persists admin + instance settings; initialized instances skip setup; 409 + 422 behavior verified).

### Findings (and fixes applied)

#### 🔴 HIGH
- **`POST /api/v1/instance/setup` returned Axum default JSON rejection for missing required fields**, which broke the documented `{"error": ...}` contract and produced non-human-friendly messages in the SPA.
  - **Fix:** `server/src/handlers/instance.rs` now deserializes `admin_username` / `instance_name` as `Option<String>` and validates them explicitly, so missing/empty fields reliably return `AppError::ValidationError("... is required")` in the standard `{"error": ...}` shape.
  - **Note:** The initialized-state check runs before request validation so initialized instances always return **409** (AC #7).
  - **Proof:** Added integration tests for missing fields in `server/tests/server_binds_to_configured_port.rs`.

#### 🟡 MEDIUM
- **App init error state treated all failures as "server not running"**, hiding real API errors (e.g. structured `AppError` responses).
  - **Fix:** `client/src/App.svelte` now surfaces `ApiError.message` when available.
- **Setup form submit could be blocked by native browser validation (`required`)**, preventing our custom inline validation from running consistently.
  - **Fix:** `client/src/lib/components/SetupPage.svelte` now uses `novalidate` so submission always goes through our Svelte validation path.
- **`avatar_color` accepted arbitrary strings server-side**, which could lead to invalid persisted state (and potential CSS injection if later rendered into styles).
  - **Fix:** `server/src/handlers/instance.rs` now validates `avatar_color` as `#RRGGBB` and returns 422 on invalid values; added a unit test.
- **Client API types used `snake_case` field names**, diverging from the architecture rule of `snake_case` JSON + `camelCase` TypeScript, increasing cognitive load and making it easier to accidentally mix naming conventions.
  - **Fix:** `client/src/lib/api.ts` now converts between `snake_case` wire format and `camelCase` client types in `getInstanceStatus()` and `submitSetup()`.
- **`apiFetch()` merged `init.headers` via object spread**, which drops headers when a `Headers` instance (or tuple array) is passed, breaking future auth/header use.
  - **Fix:** `client/src/lib/api.ts` now merges via `new Headers(init.headers)` and sets JSON `content-type` only when needed.

#### 🟢 LOW
- **Avatar color picker used `role="radiogroup"` but options lacked radio semantics**, weakening accessibility.
  - **Fix:** `client/src/lib/components/SetupPage.svelte` now uses `role="radio"` + `aria-checked`.
- **Avatar color picker `aria-label` announced raw hex values**, which is noisy and hard to parse in screen readers.
  - **Fix:** `client/src/lib/components/SetupPage.svelte` now uses named colors for `aria-label`s.
- **Avatar color picker had no keyboard navigation**, making it awkward to use without a mouse.
  - **Fix:** `client/src/lib/components/SetupPage.svelte` now uses roving `tabindex` + arrow key navigation for color selection.
- **Retry button on the server error state used an "ice" style**, which is inconsistent with the "fire CTA" rule for primary actions.
  - **Fix:** `client/src/App.svelte` now renders the Retry button with fire styling.

### Verification

- `cd server && cargo fmt --check && cargo clippy -- -D warnings && cargo test` ✅
- `cd client && npm ci && npm run lint && npm run check` ✅

## Change Log

- 2026-02-18: Story created from epics, architecture, and UX design specifications.
- 2026-02-18: Implemented first-run setup backend + frontend and added tests; marked story ready for review.
- 2026-02-18: Improved setup DB writes to be DB-agnostic (AnyPool) and race-safe for concurrent setup calls.
- 2026-02-18: Senior dev review: fixed setup JSON validation handling + added tests; improved client error display + avatar picker a11y; marked story done.
- 2026-02-18: Senior dev review follow-up: hardened setup validation (no brittle JSON rejection string matching), validated `avatar_color`, and ensured setup form uses custom validation; re-ran server/client checks.
- 2026-02-18: Senior dev review follow-up: aligned API client types to `camelCase` + added wire conversion, improved avatar color picker keyboard accessibility, and updated error-state CTA styling; re-ran server/client checks.
- 2026-02-18: Code review follow-up: fixed `apiFetch()` header merging and improved avatar picker screen-reader labels; re-ran client checks.
