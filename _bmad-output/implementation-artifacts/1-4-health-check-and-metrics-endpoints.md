# Story 1.4: Health Check and Metrics Endpoints

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As an **operator**,
I want health check endpoints so I can monitor my instance and integrate with container orchestrators,
So that I know when my instance is healthy and ready to serve traffic.

## Acceptance Criteria

1. **Given** the server is running
   **When** a request is made to `GET /healthz`
   **Then** it returns 200 if the process is alive and not deadlocked

2. **Given** the server is running and the database is connected with migrations applied
   **When** a request is made to `GET /readyz`
   **Then** it returns 200 with a JSON body confirming all checks passed

3. **Given** the server is running but the database is unreachable or migrations have not been applied
   **When** a request is made to `GET /readyz`
   **Then** it returns 503 with a JSON body describing what is not ready

4. **Given** metrics are enabled via configuration (`metrics.enabled = true`)
   **When** a request is made to `GET /metrics`
   **Then** it returns Prometheus-format text metrics including HTTP request counters, durations, and DB pool gauges

5. **Given** metrics are disabled via configuration (default)
   **When** a request is made to `GET /metrics`
   **Then** it returns 404

6. **Given** the server binary is launched with a valid config
   **When** startup completes
   **Then** `/readyz` returns 200 within 5 seconds of binary launch (NFR10)

## Tasks / Subtasks

- [x] Task 1: Create health handler module (AC: #1, #2, #3)
  - [x] 1.1 Create `server/src/handlers/health.rs` with `healthz` and `readyz` handlers
  - [x] 1.2 `healthz` returns `StatusCode::OK` (process alive = can respond to request)
  - [x] 1.3 `readyz` extracts `State<AppState>`, queries `SELECT value FROM schema_metadata WHERE key = 'initialized_at' LIMIT 1` against the pool
  - [x] 1.4 `readyz` returns 200 with `{"status": "ready", "checks": {"database": "connected", "migrations": "applied"}}` on success
  - [x] 1.5 `readyz` returns 503 with `{"status": "not_ready", "checks": {"database": "unavailable"}}` if query fails (connection error)
  - [x] 1.6 `readyz` returns 503 with `{"status": "not_ready", "checks": {"database": "connected", "migrations": "pending"}}` if query returns no rows

- [x] Task 2: Add metrics configuration (AC: #4, #5)
  - [x] 2.1 Add `MetricsConfig` struct to `server/src/config/settings.rs` with `enabled: bool` (default `false`)
  - [x] 2.2 Add `pub metrics: Option<MetricsConfig>` field to `Config` struct with `#[serde(default)]`
  - [x] 2.3 Export `MetricsConfig` from `server/src/config/mod.rs`
  - [x] 2.4 Add helper method `Config::metrics_enabled(&self) -> bool` that returns `self.metrics.as_ref().map_or(false, |m| m.enabled)`
  - [x] 2.5 Update `config.example.toml` with `[metrics]` section

- [x] Task 3: Add axum-prometheus dependency (AC: #4)
  - [x] 3.1 Add `axum-prometheus = "0.10"` to `server/Cargo.toml`
  - [x] 3.2 Verify it compiles with axum 0.8 and edition 2024

- [x] Task 4: Wire health and metrics routes into router (AC: #1, #2, #3, #4, #5)
  - [x] 4.1 Remove inline `healthz` handler from `server/src/handlers/mod.rs`
  - [x] 4.2 Add `mod health;` declaration in `server/src/handlers/mod.rs`
  - [x] 4.3 Update router to use `health::healthz` and `health::readyz`
  - [x] 4.4 Add `.route("/readyz", get(health::readyz))` to the root router (alongside `/healthz`)
  - [x] 4.5 Conditionally add metrics: if `state.config.metrics_enabled()`, call `PrometheusMetricLayer::pair()`, add the layer and expose `/metrics` route via `metric_handle.render()`
  - [x] 4.6 Health endpoints (`/healthz`, `/readyz`) must NOT be behind the metrics layer (they must work even if metrics are disabled)

- [x] Task 5: Register custom metrics (AC: #4)
  - [x] 5.1 Register `discool_info` gauge (value 1, label `version` from Cargo.toml)
  - [x] 5.2 Register `discool_db_pool_connections` gauge (labels: `state` = `active`|`idle`|`max`) updated on each `/metrics` scrape or via background task
  - [x] 5.3 Register `discool_uptime_seconds` gauge updated on scrape

- [x] Task 6: Tests (AC: #1, #2, #3, #4, #5, #6)
  - [x] 6.1 Unit test: `healthz` returns 200
  - [x] 6.2 Unit test: `readyz` returns 200 with correct JSON when pool is healthy (use in-memory SQLite pool in test)
  - [x] 6.3 Unit test: `readyz` returns 503 with descriptive JSON when pool query fails
  - [x] 6.4 Unit test: `MetricsConfig` defaults to `enabled = false`
  - [x] 6.5 Integration test: `/healthz` returns 200 (extends existing test)
  - [x] 6.6 Integration test: `/readyz` returns 200 after startup with SQLite in-memory
  - [x] 6.7 Integration test: `/readyz` response body is valid JSON with `status` and `checks` fields
  - [x] 6.8 Integration test: `/metrics` returns 200 with `text/plain` content type when `metrics.enabled = true`
  - [x] 6.9 Integration test: `/metrics` returns 404 when metrics not configured
  - [x] 6.10 Integration test: cold start time — measure time from process spawn to `/readyz` returning 200, assert < 5 seconds (NFR10)
  - [x] 6.11 Update `write_server_config()` helper to accept optional metrics enabled flag

- [x] Task 7: Code quality (AC: all)
  - [x] 7.1 Run `cargo fmt` and fix any formatting issues
  - [x] 7.2 Run `cargo clippy -- -D warnings` and resolve all warnings
  - [x] 7.3 Run `cargo test` and verify all tests pass
  - [x] 7.4 Verify `cargo build --release` succeeds

## Dev Notes

### Architecture Compliance

**This story introduces the health/readiness/metrics endpoints. These are the foundation for operator monitoring and container orchestrator integration (K8s liveness/readiness probes).**

#### Health Module Location (per architecture doc)

```
server/src/handlers/
├── mod.rs        # Route registration, router builder
├── health.rs     # /healthz, /readyz, /metrics (NEW)
├── auth.rs       # Future: /api/v1/auth/*
├── guilds.rs     # Future: /api/v1/guilds/*
└── ...
```

#### Endpoint Design (per architecture)

| Endpoint | Purpose | Response | Status |
|---|---|---|---|
| `GET /healthz` | Liveness probe | `200 OK` (empty body or minimal) | Process alive |
| `GET /readyz` | Readiness probe | `200 OK` with JSON checks | DB connected + migrations applied |
| `GET /readyz` | Readiness probe (failing) | `503 Service Unavailable` with JSON | Describes what's not ready |
| `GET /metrics` | Prometheus scrape | `200 OK` with text/plain Prometheus format | Config-gated |

**K8s mapping** (from architecture): `contrib/k8s/` manifests will map liveness to `/healthz` and readiness to `/readyz`.

#### /healthz Design

The current `/healthz` handler already returns `StatusCode::OK`. This is correct for a liveness probe — if the process can respond to HTTP, it is alive and not deadlocked. No DB check needed for liveness.

Move the handler from `handlers/mod.rs` to `handlers/health.rs` per architecture file structure.

#### /readyz Design

The readiness probe verifies the server can actually serve traffic. It must confirm:
1. **Database connectivity** — the pool can acquire a connection and execute a query
2. **Migrations applied** — the `schema_metadata` table exists and has data

A single query handles both checks:

```rust
use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use serde_json::json;

pub async fn readyz(State(state): State<crate::AppState>) -> impl IntoResponse {
    match sqlx::query_scalar::<_, String>(
        "SELECT value FROM schema_metadata WHERE key = 'initialized_at' LIMIT 1",
    )
    .fetch_optional(&state.pool)
    .await
    {
        Ok(Some(_)) => (
            StatusCode::OK,
            Json(json!({
                "status": "ready",
                "checks": {
                    "database": "connected",
                    "migrations": "applied"
                }
            })),
        ),
        Ok(None) => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({
                "status": "not_ready",
                "checks": {
                    "database": "connected",
                    "migrations": "pending"
                }
            })),
        ),
        Err(_) => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({
                "status": "not_ready",
                "checks": {
                    "database": "unavailable"
                }
            })),
        ),
    }
}
```

**Why this works:** If the query executes and returns a row, both the connection and migrations are confirmed. If it connects but the table doesn't exist (no rows), migrations are pending. If it can't connect at all, the database is unavailable.

**Do NOT log errors on every failed readyz check** — this endpoint may be polled frequently by orchestrators. Failed checks are expected during startup/shutdown. Use `tracing::debug!` at most.

#### Metrics Design

**Dependency:** `axum-prometheus = "0.10"` — compatible with axum 0.8, MSRV 1.75.

`axum-prometheus` provides:
- `axum_http_requests_total` counter (labels: method, endpoint, status)
- `axum_http_requests_duration_seconds` histogram
- `axum_http_requests_pending` gauge

It uses the `metrics` + `metrics-exporter-prometheus` ecosystem under the hood.

**Conditional wiring pattern:**

```rust
use axum_prometheus::PrometheusMetricLayer;

pub fn router(state: AppState) -> Router {
    let api = Router::new()
        .route("/ping", get(ping))
        .fallback(get(api_not_found));

    // Health endpoints are always available, outside metrics layer
    let mut app = Router::new()
        .nest("/api/v1", api)
        .route("/healthz", get(health::healthz))
        .route("/readyz", get(health::readyz))
        .route("/ws", get(ws_not_found));

    // Conditionally add metrics
    if state.config.metrics_enabled() {
        let (prometheus_layer, metric_handle) = PrometheusMetricLayer::pair();
        app = app
            .route("/metrics", get(move || async move { metric_handle.render() }))
            .layer(prometheus_layer);
    }

    app.fallback(get(crate::static_files::handler))
        .layer(middleware::from_fn(security_headers))
        .with_state(state)
}
```

**Important:** `/healthz` and `/readyz` must be registered BEFORE the metrics layer is added so they work regardless of metrics config. The metrics layer wraps routes registered before it.

Actually — Axum layers wrap ALL routes in the router they're applied to, regardless of order. To exclude health endpoints from metrics tracking, use a nested approach:

```rust
pub fn router(state: AppState) -> Router {
    let api = Router::new()
        .route("/ping", get(ping))
        .fallback(get(api_not_found));

    // Routes that should be tracked by metrics
    let mut tracked = Router::new()
        .nest("/api/v1", api)
        .route("/ws", get(ws_not_found))
        .fallback(get(crate::static_files::handler));

    if state.config.metrics_enabled() {
        let (prometheus_layer, metric_handle) = PrometheusMetricLayer::pair();
        tracked = tracked.layer(prometheus_layer);
        // Merge the /metrics route into the outer router
        let metrics_route = Router::new()
            .route("/metrics", get(move || async move { metric_handle.render() }));
        tracked = tracked.merge(metrics_route);
    }

    // Health endpoints outside metrics tracking
    Router::new()
        .route("/healthz", get(health::healthz))
        .route("/readyz", get(health::readyz))
        .merge(tracked)
        .layer(middleware::from_fn(security_headers))
        .with_state(state)
}
```

The developer should verify the exact layering behavior with a test. The key constraint: health endpoints must return correct responses regardless of metrics configuration.

#### Custom Metrics

Beyond the automatic HTTP metrics, register:

```rust
// In health.rs or a dedicated metrics module
use metrics::{gauge, describe_gauge};

pub fn register_custom_metrics() {
    describe_gauge!("discool_info", "Discool server information");
    describe_gauge!("discool_uptime_seconds", "Server uptime in seconds");
    describe_gauge!("discool_db_pool_connections", "Database pool connection counts");

    // Set version info gauge
    gauge!("discool_info", "version" => env!("CARGO_PKG_VERSION")).set(1.0);
}

pub fn update_pool_metrics(pool: &sqlx::AnyPool) {
    gauge!("discool_db_pool_connections", "state" => "max").set(pool.size() as f64);
    gauge!("discool_db_pool_connections", "state" => "idle").set(pool.num_idle() as f64);
}
```

Call `register_custom_metrics()` during startup if metrics are enabled. Call `update_pool_metrics()` either in the `/metrics` handler or on a background interval.

#### MetricsConfig

```rust
#[derive(Debug, Clone, Deserialize)]
pub struct MetricsConfig {
    #[serde(default)]
    pub enabled: bool,
}

impl Default for MetricsConfig {
    fn default() -> Self {
        Self { enabled: false }
    }
}
```

Add to `Config`:
```rust
#[serde(default)]
pub metrics: Option<MetricsConfig>,
```

Helper:
```rust
impl Config {
    pub fn metrics_enabled(&self) -> bool {
        self.metrics.as_ref().map_or(false, |m| m.enabled)
    }
}
```

TOML:
```toml
[metrics]
enabled = false  # Set to true to expose /metrics endpoint with Prometheus format
```

Env var: `DISCOOL_METRICS__ENABLED=true`

### Existing Code to Modify

| File | Change |
|---|---|
| `server/Cargo.toml` | Add `axum-prometheus = "0.10"` dependency |
| `server/src/config/settings.rs` | Add `MetricsConfig` struct, `metrics_enabled()` helper |
| `server/src/config/mod.rs` | Export `MetricsConfig` |
| `server/src/handlers/mod.rs` | Remove inline `healthz`, add `mod health`, wire `/readyz` route, conditional metrics layer |
| `config.example.toml` | Add `[metrics]` section |
| `server/tests/server_binds_to_configured_port.rs` | Add readyz and metrics integration tests, update config helper |

### Files to Create

| File | Purpose |
|---|---|
| `server/src/handlers/health.rs` | `healthz()`, `readyz()` handlers, custom metrics registration |

### Project Structure Notes

- `server/src/handlers/health.rs` is the correct location per architecture doc (`handlers/health.rs # /healthz, /readyz, /metrics`)
- Health endpoints are root-level routes (`/healthz`, `/readyz`, `/metrics`), NOT under `/api/v1`
- No new database tables needed — readyz queries the existing `schema_metadata` table from Story 1.3's migration

### Testing Requirements

- **Unit:** `healthz` returns `StatusCode::OK`
- **Unit:** `readyz` returns 200 with correct JSON when DB is available and migrations applied
- **Unit:** `readyz` returns 503 with descriptive JSON when DB is unavailable
- **Unit:** `MetricsConfig` defaults disabled; deserializes correctly from TOML
- **Unit:** `Config::metrics_enabled()` returns correct boolean
- **Integration:** `/healthz` returns 200 (verify existing behavior preserved)
- **Integration:** `/readyz` returns 200 with valid JSON after server starts with `sqlite::memory:`
- **Integration:** `/metrics` returns 200 with Prometheus text when `metrics.enabled = true`
- **Integration:** `/metrics` returns 404 when metrics not configured
- **Integration:** Cold start < 5s — time from `spawn_server()` to `/readyz` returning 200
- **Linting:** `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test` — all must pass

For unit testing `readyz`, create a test helper that builds an `AppState` with a real `sqlite::memory:` pool (with migrations run). The pool is lightweight and tests run fast.

For testing readyz failure (503), the cleanest approach is to close the pool before calling the handler:
```rust
#[tokio::test]
async fn readyz_returns_503_when_pool_closed() {
    let state = test_app_state().await;
    state.pool.close().await;
    let response = readyz(State(state)).await.into_response();
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
}
```

### Anti-Patterns to Avoid

- Do NOT add database writes or mutations in health check handlers — read-only queries only
- Do NOT log at `warn` or `error` level on readyz failures — these are expected during startup/shutdown; use `debug` at most
- Do NOT make `/healthz` check the database — liveness probes must be lightweight and not depend on external services
- Do NOT add auth or rate limiting to health endpoints — orchestrators need unrestricted access
- Do NOT use `unwrap()` or `expect()` in health handlers — handle all errors gracefully
- Do NOT put health endpoints behind `/api/v1` prefix — they are root-level operational endpoints
- Do NOT add `axum-prometheus` as a default feature or always-on — it must be config-gated
- Do NOT track `/healthz` or `/readyz` in Prometheus HTTP metrics — they are polled frequently by orchestrators and would skew request metrics
- Do NOT use `prometheus` crate directly alongside `axum-prometheus` — they use different registries (`axum-prometheus` uses the `metrics` ecosystem). Use `metrics::gauge!()` / `metrics::counter!()` macros for custom metrics.

### Previous Story Intelligence

**From Story 1.3 (Database Connection and Migration System):**
- `AppState { config: Arc<Config>, pool: AnyPool }` established in `lib.rs`
- `schema_metadata` table created by `0001_initial_schema.sql` — readyz can query this
- Pool configuration: max 5 connections, `sqlite::memory:` forced to 1 connection
- DB error logging pattern: redact DB URL from error messages
- Integration test pattern: `TestServer` struct with `spawn_server()`, `wait_for_bind()`, `http_status()` helpers in `server/tests/server_binds_to_configured_port.rs`
- `write_server_config()` generates TOML with `[server]`, `[log]`, `[database]` sections — extend for `[metrics]`
- The existing `healthz_returns_200` integration test verifies `/healthz` end-to-end
- Error pattern: `AppError` enum in `error.rs` with structured JSON responses `{"error": {"code": "...", "message": "...", "details": {}}}`
- Config pattern: typed structs with `serde::Deserialize`, `Option<T>` for optional sections, `#[serde(default)]` for defaults

**Review follow-ups from Story 1.3 that are relevant:**
- The `expect("validated")` pattern was removed — do not reintroduce; use explicit `Option` handling
- DB URL redaction is wired — maintain this pattern in any new DB-touching code

### Git Intelligence

**Recent commits:**
- `3a676ee` — chore: commit from Copilot CLI (Story 1.3 implementation)
- `e204ccb` — Add configuration system with structured logging support (Story 1.2)
- `a828f94` — Initial commit: BMAD framework, Svelte client, and Rust server (Story 1.1)

**Patterns established:**
- Rust module pattern: `mod.rs` + separate files per concern
- Config pattern: typed structs with `serde::Deserialize`, `Option<T>` for optional sections
- Error pattern: `AppError` enum with `IntoResponse`, structured JSON
- State pattern: `AppState` struct with `config: Arc<Config>` and `pool: AnyPool`
- Test pattern: inline `#[cfg(test)]` for unit tests, `server/tests/` for integration tests
- Integration test pattern: process-spawning with `TestServer`, `wait_for_bind()`, raw HTTP via `TcpStream`
- Router pattern: `handlers::router(state)` builds the full Axum router with layers

### References

- [Source: _bmad-output/planning-artifacts/architecture.md#Infrastructure & Deployment] — `/healthz` (liveness) + `/readyz` (readiness) + optional Prometheus `/metrics`
- [Source: _bmad-output/planning-artifacts/architecture.md#Complete Project Directory Structure] — `handlers/health.rs` location
- [Source: _bmad-output/planning-artifacts/architecture.md#Technical Decision Matrix] — K8s probes mapped to `/healthz` and `/readyz`
- [Source: _bmad-output/planning-artifacts/epics.md#Story 1.4] — Acceptance criteria, story statement
- [Source: _bmad-output/planning-artifacts/epics.md#NFR10] — Server cold start time < 5 seconds
- [Source: _bmad-output/planning-artifacts/epics.md#NFR37] — Instance exposes health check endpoint and basic metrics
- [Source: _bmad-output/planning-artifacts/prd.md#NFR9] — 50 concurrent users on 2 vCPU / 2GB RAM
- [Source: _bmad-output/implementation-artifacts/1-3-database-connection-and-migration-system.md] — AppState, pool config, schema_metadata table, integration test patterns
- [GitHub: Ptrskay3/axum-prometheus] — v0.10.0 supports axum 0.8; provides HTTP request metrics via `metrics` ecosystem
- [crates.io: axum-prometheus 0.10] — Middleware for Prometheus HTTP metrics; MSRV 1.75

## Dev Agent Record

### Agent Model Used

GitHub Copilot CLI 0.0.411

### Debug Log References

- `cd server && cargo test -q`
- `cd server && cargo fmt`
- `cd server && cargo clippy -- -D warnings`
- `cd server && cargo build --release -q`

### Completion Notes List

- Implemented `GET /healthz` (liveness) and `GET /readyz` (readiness) with DB + migration checks.
- Added optional Prometheus `GET /metrics` endpoint gated by `metrics.enabled`, with custom gauges (`discool_*`).
- Added unit + integration tests covering `/healthz`, `/readyz`, `/metrics`, and cold start NFR10 (< 5s).

### File List

- `server/src/handlers/health.rs` (new)
- `server/src/handlers/mod.rs`
- `server/src/config/settings.rs`
- `server/src/config/mod.rs`
- `server/tests/server_binds_to_configured_port.rs`
- `server/Cargo.toml`
- `server/Cargo.lock`
- `config.example.toml`
- `_bmad-output/implementation-artifacts/1-4-health-check-and-metrics-endpoints.md`
- `_bmad-output/implementation-artifacts/sprint-status.yaml`

## Senior Developer Review (AI)

**Outcome:** Changes applied and verified; story status set to `done`.

### Findings (fixed)

- **MEDIUM:** `/metrics` scrapes were included in HTTP request metrics (skews dashboards). Moved `/metrics` route outside `PrometheusMetricLayer`.
- **MEDIUM:** Cold start NFR10 test used `< 5s` which is stricter than "within 5 seconds" and can be borderline flaky; changed to `<= 5s` with clearer message.
- **LOW:** `config.example.toml` was missing the `DISCOOL_METRICS__ENABLED` example env var; added.

### Verification

- `cd server && cargo fmt`
- `cd server && cargo clippy -- -D warnings`
- `cd server && cargo test -q`

## Change Log

- 2026-02-18: Implement health/readiness endpoints and optional Prometheus metrics (+ tests, config, deps).
- 2026-02-18: Senior review fixes: exclude `/metrics` from request metrics; clarify NFR10 timing assertion; document metrics env var.
