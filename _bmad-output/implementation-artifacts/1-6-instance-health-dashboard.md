# Story 1.6: Instance Health Dashboard

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As an **operator**,
I want to view resource usage and health status from within the SPA admin panel,
So that I can monitor my instance without SSH access.

## Acceptance Criteria

1. **Given** the operator is authenticated as the instance admin
   **When** they navigate to the admin panel section in the SPA sidebar
   **Then** they can see current CPU and memory usage of the server process

2. **Given** the admin panel is displayed
   **When** the health data loads
   **Then** they can see the number of active WebSocket connections

3. **Given** the admin panel is displayed
   **When** the health data loads
   **Then** they can see database size and connection pool status

4. **Given** the admin panel is displayed
   **When** the health data loads
   **Then** they can see instance uptime

5. **Given** the admin panel is displayed
   **When** 30 seconds have elapsed since the last data fetch
   **Then** the health data refreshes automatically

6. **Given** the admin panel
   **Then** the admin panel is a section within the SPA sidebar, not a separate interface

## Tasks / Subtasks

- [x] Task 1: Add server-side health stats endpoint (AC: #1, #2, #3, #4)
  - [x] 1.1 Add `Forbidden` variant to `AppError` in `server/src/error.rs` — maps to 403 with `{"error": {"code": "FORBIDDEN", "message": "...", "details": {}}}` JSON
  - [x] 1.2 Create `GET /api/v1/admin/health` handler in `server/src/handlers/admin.rs`
  - [x] 1.3 Guard endpoint: return 403 (`AppError::Forbidden`) if instance is not initialized (pre-auth guard — full auth guard added in Epic 2). Reuse the `is_initialized()` check from `instance.rs` (extract to a shared function or call inline)
  - [x] 1.4 Collect CPU usage of the server process via `/proc/self/stat` parsing (Linux) with fallback to 0.0 on unsupported platforms
  - [x] 1.5 Collect memory usage (RSS) of the server process via `/proc/self/status` parsing (Linux) with fallback
  - [x] 1.6 Collect DB pool stats: active connections, idle connections, max connections (already available via `pool.size()` / `pool.num_idle()`)
  - [x] 1.7 Collect DB size: `pg_database_size(current_database())` for PostgreSQL, file size for SQLite (detect via connection URL prefix)
  - [x] 1.8 Collect uptime from `state.start_time.elapsed()`
  - [x] 1.9 Collect active WebSocket connections count: return 0 for now (placeholder — WebSocket gateway is Story 6.1; the field exists so the client doesn't need to change when the gateway is implemented)
  - [x] 1.10 Return `{"data": { "cpu_usage_percent": f64, "memory_rss_bytes": u64, "uptime_seconds": u64, "db_size_bytes": u64, "db_pool_active": u32, "db_pool_idle": u32, "db_pool_max": u32, "websocket_connections": u32 }}`

- [x] Task 2: Wire admin routes into router (AC: #1)
  - [x] 2.1 Add `mod admin;` declaration in `server/src/handlers/mod.rs`
  - [x] 2.2 Add `.route("/admin/health", get(admin::get_health))` to the `/api/v1` router
  - [x] 2.3 Ensure the route is within the `tracked` router so metrics track it when enabled

- [x] Task 3: Client-side admin API functions (AC: #1, #5)
  - [x] 3.1 Add `getAdminHealth(): Promise<AdminHealth>` function to `client/src/lib/api.ts`
  - [x] 3.2 Define TypeScript types: `AdminHealth` with camelCase fields matching the API response

- [x] Task 4: AdminPanel component (AC: #1, #2, #3, #4, #5, #6)
  - [x] 4.1 Create `client/src/lib/components/AdminPanel.svelte`
  - [x] 4.2 Display CPU usage as percentage with a simple bar or text indicator
  - [x] 4.3 Display memory usage in human-readable format (e.g., "42 MB / RSS")
  - [x] 4.4 Display active WebSocket connections count (will show 0 until Epic 6)
  - [x] 4.5 Display database size in human-readable format (e.g., "1.2 MB")
  - [x] 4.6 Display DB pool status: active/idle/max connections
  - [x] 4.7 Display uptime in human-readable format (e.g., "2d 5h 30m")
  - [x] 4.8 Auto-refresh every 30 seconds using `setInterval` with cleanup on destroy
  - [x] 4.9 Show loading skeleton on initial fetch (per UX loading patterns: <200ms = no state, 200ms-2s = skeleton)
  - [x] 4.10 Show error state with retry button if the fetch fails
  - [x] 4.11 Follow Dual Core design system: dark zinc background, ice blue for metric labels, fire for any action buttons
  - [x] 4.12 Use shadcn-svelte Card or similar component for metric grouping

- [x] Task 5: Integrate admin panel into App.svelte (AC: #6)
  - [x] 5.1 Add an "Admin" navigation link/button in the SPA sidebar area (visible only when instance is initialized — the operator is the only user pre-auth)
  - [x] 5.2 Conditionally render `<AdminPanel>` when the admin view is selected
  - [x] 5.3 Default view remains the normal app scaffold; admin panel is toggled via sidebar
  - [x] 5.4 The admin panel replaces the main content area (not a modal or overlay)

- [x] Task 6: Server tests (AC: #1, #2, #3, #4)
  - [x] 6.1 Unit test: `GET /api/v1/admin/health` returns 200 with correct JSON shape on initialized instance
  - [x] 6.2 Unit test: `GET /api/v1/admin/health` returns 403 on uninitialized instance
  - [x] 6.3 Unit test: response contains `uptime_seconds` > 0
  - [x] 6.4 Unit test: response contains `db_pool_idle` >= 0, `db_pool_max` > 0
  - [x] 6.5 Unit test: response contains `websocket_connections` = 0 (placeholder)
  - [x] 6.6 Unit test: response contains `cpu_usage_percent` >= 0.0
  - [x] 6.7 Unit test: response contains `memory_rss_bytes` >= 0
  - [x] 6.8 Unit test: response contains `db_size_bytes` >= 0
  - [x] 6.9 Integration test: `GET /api/v1/admin/health` after instance setup returns 200
  - [x] 6.10 Integration test: `GET /api/v1/admin/health` before instance setup returns 403

- [x] Task 7: Code quality (AC: all)
  - [x] 7.1 Run `cargo fmt` and fix any formatting issues
  - [x] 7.2 Run `cargo clippy -- -D warnings` and resolve all warnings
  - [x] 7.3 Run `cargo test` and verify all tests pass
  - [x] 7.4 Run `cargo build --release` and verify it succeeds
  - [x] 7.5 Run `npx biome check .` in client directory and fix issues
  - [x] 7.6 Run `npx svelte-check --tsconfig ./tsconfig.app.json` in client directory and fix issues

## Dev Notes

### Architecture Compliance

**This story adds an admin health dashboard endpoint and a corresponding SPA component. It builds on the existing /healthz, /readyz, and /metrics infrastructure from Story 1.4, and extends the SPA from Story 1.5 with a new admin panel view.**

#### Server Module Location (per architecture doc)

```
server/src/handlers/
├── mod.rs        # Route registration, router builder
├── health.rs     # /healthz, /readyz, /metrics (existing)
├── instance.rs   # /api/v1/instance, /api/v1/instance/setup (existing)
├── admin.rs      # /api/v1/admin/health (NEW)
└── ...
```

**Why a separate `admin.rs` instead of adding to `instance.rs`:** The instance module handles instance identity and setup (one-time operations). The admin module will grow to include backup/export (Story 1.7) and potentially other operator-only endpoints. Separating concerns keeps each module focused.

#### Client Component Location (per architecture doc)

```
client/src/
├── App.svelte                  # Root: conditional setup vs. normal app (MODIFY)
├── lib/
│   ├── api.ts                  # API client wrapper (MODIFY — add getAdminHealth)
│   ├── components/
│   │   ├── SetupPage.svelte    # First-run setup form (existing)
│   │   └── AdminPanel.svelte   # Health dashboard panel (NEW)
│   └── utils.ts                # Existing utils
└── main.ts
```

Note: The architecture doc specifies a `features/` directory structure. However, the admin panel is an operator-only section (not a user-facing feature like guild/chat/voice). It doesn't justify a full feature directory with stores, API hooks, and types. A single component + API function is appropriate. When Story 1.7 (backup/export) arrives, both admin components can optionally be grouped into `features/admin/` if needed.

#### API Design (per architecture conventions)

| Endpoint | Method | Purpose | Response |
|---|---|---|---|
| `/api/v1/admin/health` | GET | Instance health metrics | `{"data": { ... metrics ... }}` |

All responses follow the `{"data": ...}` / `{"error": {"code": "...", "message": "...", "details": {}}}` format.

**HTTP status codes used:**
- 200: Success (health metrics returned)
- 403: Forbidden (instance not initialized — no admin exists)
- 500: Internal error

#### Admin Access Control (Pre-Auth)

**Critical context:** There is NO authentication system yet (Epic 2). The admin health endpoint cannot use session tokens or identity verification.

**Guard strategy:** Check that the instance is initialized. If it is, the endpoint is accessible. This is acceptable because:
1. Before initialization, there's no admin — the endpoint should not be accessible.
2. After initialization, the only person with access to the instance URL is the operator (the instance hasn't been shared yet — invite links are Epic 4).
3. When Epic 2 introduces authentication, the admin guard will be upgraded to verify the caller's identity against the admin user record.

The endpoint returns `AppError::Forbidden` (403) if the instance is not initialized. A `TODO` comment in the code should note that this guard needs to be upgraded to proper auth in Epic 2.

#### Health Metrics Collection

**CPU usage:** Read `/proc/self/stat` on Linux to get process CPU time. Calculate percentage based on the delta between two readings (or use a single-point snapshot relative to system uptime). On non-Linux platforms, return 0.0. Do NOT add the `sysinfo` crate — it's a heavy dependency (~2MB compiled) for a simple `/proc` read. Direct `/proc` parsing keeps the binary small (NFR9: 2 vCPU/2GB target).

**Memory usage:** Read `/proc/self/status` on Linux for `VmRSS` (resident set size in kB). Convert to bytes. On non-Linux, return 0.

**Database size:**
- SQLite: Parse the database URL from config. If it contains a file path (not `:memory:`), use `std::fs::metadata(path).len()`. For `:memory:`, return 0.
- PostgreSQL: Execute `SELECT pg_database_size(current_database())` — returns size in bytes.
- Detection: Check if the configured database URL starts with `sqlite:` or `postgres:`.

**DB pool stats:** Already available via `pool.size()` (total) and `pool.num_idle()` (idle). Active = size - idle. Max comes from config.

**Uptime:** `state.start_time.elapsed().as_secs()` — already tracked in AppState.

**WebSocket connections:** Return 0 for now. When the WebSocket gateway (Story 6.1) is implemented, it will maintain a connection counter in AppState (or a shared AtomicU32). The response field is included now so the client component doesn't need to change.

#### Admin Health Response Format

```json
{
  "data": {
    "cpu_usage_percent": 2.5,
    "memory_rss_bytes": 44040192,
    "uptime_seconds": 86400,
    "db_size_bytes": 1048576,
    "db_pool_active": 2,
    "db_pool_idle": 3,
    "db_pool_max": 5,
    "websocket_connections": 0
  }
}
```

All numeric fields. The client converts `snake_case` to `camelCase` via the existing `api.ts` transform layer (established in Story 1.5).

#### Admin Panel Design (per UX spec)

The UX spec defines (Journey 3):
- Admin panel is a section within the SPA sidebar, not a separate interface
- Health/status information is **glanceable, not a separate dashboard**
- Instance health: CPU, RAM, connections
- P2P network status: discovered instances (Story 3.2 — not this story)
- Instance settings (already accessible via setup — future edit capability)
- Backup/export (Story 1.7)

**Layout:**
- A simple sidebar tab/button labeled "Admin" with a settings/gear icon
- When selected, the main content area shows health metrics in a clean card-based layout
- Each metric is a labeled value: label in `text-sm` muted, value in `text-lg` or `text-xl`
- Cards grouped: "Server" (CPU, Memory, Uptime), "Database" (Size, Pool), "Connections" (WebSocket)
- Auto-refresh indicator: subtle "Last updated: X seconds ago" text
- Dark zinc background, ice blue for labels, zinc-400 for secondary text
- No fire orange buttons in this view (no primary actions — this is read-only)

**Loading pattern (per UX spec):**
- <200ms response: no loading state shown
- 200ms-2s: skeleton placeholders matching the card layout
- >2s: skeleton + "Loading health data..." text

**Error state:**
- If the health fetch fails: show error card with "Could not load health data" + "Retry" button (fire orange CTA)

### Existing Code to Modify

| File | Change |
|---|---|
| `server/src/error.rs` | Add `Forbidden` variant to `AppError` — maps to 403 |
| `server/src/handlers/mod.rs` | Add `mod admin;`, wire `/api/v1/admin/health` route |
| `client/src/lib/api.ts` | Add `getAdminHealth()` function and `AdminHealth` type |
| `client/src/App.svelte` | Add admin panel navigation and conditional rendering |

### Files to Create

| File | Purpose |
|---|---|
| `server/src/handlers/admin.rs` | `get_health()` handler with process metrics collection |
| `client/src/lib/components/AdminPanel.svelte` | Health dashboard panel component |

### Project Structure Notes

- `/api/v1/admin/health` is nested under the existing `/api/v1` router — it is tracked by Prometheus metrics when enabled
- The `admin` module follows the established handler pattern: one file per domain concern
- No new Cargo dependencies needed — process metrics are read from `/proc` directly. The `sysinfo` crate is intentionally avoided (too heavy for the 2GB RAM target)
- No new npm dependencies needed — the client uses existing `api.ts` wrapper and Svelte 5 reactivity
- The DB size query for PostgreSQL uses a raw SQL function call; for SQLite it uses filesystem metadata — this is a backend-specific query, consistent with the `DatabaseBackend` abstraction pattern from architecture doc

### Testing Requirements

**Server unit tests (in `server/src/handlers/admin.rs` `#[cfg(test)]` module):**
- `GET /api/v1/admin/health` returns 200 with correct JSON shape on initialized instance
- `GET /api/v1/admin/health` returns 403 on uninitialized instance
- Response contains all expected fields with valid values
- `uptime_seconds` > 0
- `db_pool_max` > 0
- `websocket_connections` = 0 (placeholder value)
- `cpu_usage_percent` >= 0.0
- `memory_rss_bytes` >= 0

For unit testing, reuse the `test_state()` pattern from `health.rs` — create an `AppState` with a real `sqlite::memory:` pool (with migrations run). For the initialized guard, insert the `initialized_at` row into `instance_settings` (same as the setup handler does).

**Server integration tests (in `server/tests/server_binds_to_configured_port.rs`):**
- `GET /api/v1/admin/health` before setup returns 403
- `POST /api/v1/instance/setup` then `GET /api/v1/admin/health` returns 200 with valid health data

For integration tests, extend the existing test helpers. The setup flow is already tested — just add the admin health check after setup.

**Client tests:**
- Client component tests deferred (same rationale as Story 1.5 — Vitest + @testing-library/svelte not yet set up)

**Linting:** `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test`, `npx biome check .`, `npx svelte-check` — all must pass.

### Anti-Patterns to Avoid

- Do NOT use `unwrap()` or `expect()` in handler code — use `?` with `AppError`
- Do NOT return bare JSON objects — always wrap in `{"data": ...}` or `{"error": ...}`
- Do NOT add the `sysinfo` crate — it's too heavy for this use case. Read `/proc` directly
- Do NOT use `any` type in TypeScript — define proper `AdminHealth` types
- Do NOT call `fetch()` directly in Svelte components — use the `api.ts` wrapper
- Do NOT block the handler on CPU-intensive metrics collection — `/proc` reads are fast and non-blocking
- Do NOT expose internal error details (file paths, SQL errors) in the API response
- Do NOT implement a separate admin authentication system — use the initialized guard for now, note the upgrade path for Epic 2
- Do NOT use `console.log` in client code
- Do NOT make the admin panel a separate page/route — it's a section within the SPA sidebar per UX spec
- Do NOT add polling faster than 30 seconds — this is an operator dashboard, not a real-time monitor
- Do NOT add charts or graphs — the UX spec says "glanceable" health info, simple labeled values are correct

### Previous Story Intelligence

**From Story 1.5 (First-Run Admin Setup Screen):**
- `AppState { config: Arc<Config>, pool: AnyPool, start_time: Instant }` in `lib.rs`
- `instance_settings` table exists with key-value pairs including `initialized_at` — use this to check if instance is initialized (same `is_initialized()` pattern from instance.rs)
- Handler pattern: `pub async fn handler_name(State(state): State<AppState>) -> impl IntoResponse`
- JSON response pattern: `(StatusCode::OK, Json(json!({...})))` tuple
- Router pattern: `Router::new().route(path, method(handler))`, nested via `.nest("/api/v1", api)`
- Error pattern: `AppError` enum → `IntoResponse` with JSON `{"error": {"code", "message", "details"}}`
- `AppError::Forbidden` variant may need to be added if not already present (check error.rs)
- Metrics layer wraps the `tracked` router — new API routes in the `/api/v1` nest are automatically tracked
- Integration test helpers: `spawn_server()`, `wait_for_bind()`, `http_status()`, `http_response()`, `response_body()`, `write_server_config()`, `http_post()`
- Client API: `apiFetch<T>()` wrapper handles `{"data": T}` / `{"error": {...}}` response format with snake→camel conversion
- Client App.svelte: fetches `GET /api/v1/instance` on mount, shows setup or normal view conditionally

**From Story 1.4 (Health Check and Metrics Endpoints):**
- `health.rs` already collects pool stats: `pool.num_idle()`, `pool.size()` — reuse same approach
- `update_custom_metrics()` shows how to read pool stats and `start_time.elapsed()`
- `register_custom_metrics()` pattern for Prometheus gauges — admin health is a separate concern (REST API, not Prometheus) but uses the same underlying data
- Test helpers: `test_state()` creates AppState with sqlite::memory: + migrations

**Review follow-ups from previous stories:**
- No `expect("validated")` — use explicit `Option` handling
- DB URL redaction is wired — maintain pattern in any new DB-touching code
- Health endpoints are outside metrics tracking — new `/api/v1/admin/*` routes should be inside (they are, since they're nested under `/api/v1`)

### Git Intelligence

**Recent commits:**
- `13be36e` — feat: add first-run admin setup screen with instance configuration (Story 1.5)
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
- Client: Svelte 5, Vite, Tailwind CSS 4, Biome linter, api.ts wrapper with snake→camel conversion

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Story 1.6] — Full acceptance criteria and story statement
- [Source: _bmad-output/planning-artifacts/epics.md#FR11] — "Operators can view instance resource usage and health status"
- [Source: _bmad-output/planning-artifacts/epics.md#NFR37] — "Instance exposes health check endpoint and basic metrics"
- [Source: _bmad-output/planning-artifacts/architecture.md#Health endpoints] — /healthz, /readyz, optional Prometheus /metrics
- [Source: _bmad-output/planning-artifacts/architecture.md#Format Patterns] — REST API response format: `{"data": ...}` / `{"error": ...}`
- [Source: _bmad-output/planning-artifacts/architecture.md#Process Patterns] — Error handling: `Result<Json<T>, AppError>`, `IntoResponse` mapping
- [Source: _bmad-output/planning-artifacts/architecture.md#Enforcement Guidelines] — Anti-patterns to avoid in handlers and frontend
- [Source: _bmad-output/planning-artifacts/architecture.md#Complete Project Directory Structure] — Handler file structure, client feature structure
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Journey 3] — Admin panel within SPA sidebar, health info glanceable
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Loading Patterns] — Skeleton placeholders, no spinner without text
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Design Direction Decision] — Dual Core: ice for labels, fire for CTAs, zinc foundation
- [Source: _bmad-output/implementation-artifacts/1-5-first-run-admin-setup-screen.md] — Handler patterns, router wiring, test infrastructure, AppState shape, client API wrapper, is_initialized() pattern
- [Source: _bmad-output/implementation-artifacts/1-4-health-check-and-metrics-endpoints.md] — Pool stats collection, Prometheus metrics, health.rs test_state() helper

## Dev Agent Record

### Agent Model Used

GitHub Copilot CLI 0.0.411

### Debug Log References

- `cd server && cargo fmt`
- `cd server && cargo clippy -- -D warnings`
- `cd server && cargo test -q`
- `cd server && cargo build --release -q`
- `cd client && npm ci`
- `cd client && npm run lint`
- `cd client && npm run check`

### Completion Notes List

- Added `AppError::Forbidden` (403) and implemented `GET /api/v1/admin/health` (initialized-only guard) returning process + DB health metrics.
- Wired the admin route under `/api/v1` inside the tracked router for metrics compatibility.
- Added `getAdminHealth()` + `AdminHealth` types and built an SPA AdminPanel with 30s auto-refresh, skeleton loading, and error retry.
- Integrated Admin view into the SPA sidebar and added unit + integration tests for the endpoint.
- Polish: CPU usage now prefers delta-based sampling between calls (fallback to lifetime average on first sample) and UI shows "Last updated: Xs ago".
- Review fixes: added an inline Retry action when refresh fails, added warning logs for DB size lookup failures, and tightened admin health JSON type assertions in tests.

### File List

- `_bmad-output/implementation-artifacts/sprint-status.yaml` (updated story status)
- `_bmad-output/implementation-artifacts/1-6-instance-health-dashboard.md` (updated tasks/status/record)
- `server/src/error.rs`
- `server/src/handlers/admin.rs` (new)
- `server/src/handlers/mod.rs`
- `server/src/handlers/instance.rs`
- `server/tests/server_binds_to_configured_port.rs`
- `client/src/lib/api.ts`
- `client/src/lib/components/AdminPanel.svelte` (new)
- `client/src/App.svelte`

## Change Log

- 2026-02-18: Story created from epics, architecture, and UX design specifications.
- 2026-02-18: Implemented admin health endpoint + SPA admin panel and added unit/integration tests; marked story ready for review.
- 2026-02-18: Polish: improved CPU usage freshness and aligned UI "Last updated" display to UX spec.
- 2026-02-18: Senior dev code review: fixed refresh retry UX, added DB size error logging, and tightened tests; marked story done.

## Senior Developer Review (AI)

_Reviewer: Darko on 2026-02-18_

### Findings

**MEDIUM**
- `client/src/lib/components/AdminPanel.svelte`: When a background refresh failed after initial load, the UI showed an error but offered no retry action (users had to wait for the next 30s tick). **Fixed** by adding a Retry button in the inline error banner.
- `server/src/handlers/admin.rs`: DB size collection swallowed errors and returned `0` with no logging, making failures hard to diagnose. **Fixed** by emitting `tracing::warn!` logs on postgres query / sqlite metadata failures (still returns `0` to keep the endpoint resilient).

**LOW**
- `server/src/handlers/admin.rs` tests asserted integer fields (`memory_rss_bytes`, `db_size_bytes`) as floats. **Fixed** by asserting they parse as `u64`.

### Outcome

✅ Approved (issues fixed)
