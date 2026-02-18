# Story 1.3: Database Connection and Migration System

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As an **operator**,
I want the server to connect to PostgreSQL or SQLite based on my configuration and run migrations automatically,
So that the database is always in the correct state when the instance starts.

## Acceptance Criteria

1. **Given** the operator has configured a database URL in the config (PostgreSQL connection string or SQLite file path)
   **When** the server starts
   **Then** it connects to the configured database using sqlx `Any` driver

2. **Given** a database connection is established
   **When** the server starts
   **Then** embedded migrations run automatically on startup, bringing the schema to the latest version

3. **Given** the database is unreachable or the URL is invalid
   **When** the server starts
   **Then** it logs a clear error and exits with a non-zero status code

4. **Given** the `DatabaseBackend` abstraction exists
   **When** backend-specific SQL is needed
   **Then** it abstracts PG/SQLite differences for query variants

5. **Given** the server is running on 2GB RAM hardware
   **When** the connection pool is created
   **Then** pooling is configured with sensible defaults (max 5 connections, aggressive idle timeout)

## Tasks / Subtasks

- [x] Task 1: Add sqlx dependency to Cargo.toml (AC: #1)
  - [x] 1.1 Add `sqlx = { version = "0.8", features = ["runtime-tokio", "tls-rustls-ring-webpki", "any", "postgres", "sqlite", "migrate", "uuid", "chrono", "json"] }` to `server/Cargo.toml`
  - [x] 1.2 Add `uuid = { version = "1", features = ["v4", "serde"] }` and `chrono = { version = "0.4", features = ["serde"] }` for type support
  - [x] 1.3 Verify the dependency resolves and compiles

- [x] Task 2: Add database config section (AC: #1, #3, #5)
  - [x] 2.1 Add `DatabaseConfig` struct to `server/src/config/settings.rs` with `url: String` (required, no default) and `max_connections: u32` (default 5)
  - [x] 2.2 Add `#[serde(default)]` on `database` field in `Config` — but `url` has NO default so validation catches it
  - [x] 2.3 Add validation: `database.url` must not be empty, must start with `postgres://`, `postgresql://`, `sqlite://`, or `sqlite:`
  - [x] 2.4 Wire `redact_secret()` into `Config::log_summary()` for `database.url`
  - [x] 2.5 Update `config.example.toml` with `[database]` section and documented options

- [x] Task 3: Create db module — pool setup (AC: #1, #3, #5)
  - [x] 3.1 Create `server/src/db/mod.rs` — module root, re-exports `init_pool()` and `DatabaseBackend`
  - [x] 3.2 Create `server/src/db/pool.rs` — pool initialization with `AnyPool`
  - [x] 3.3 Call `sqlx::any::install_default_drivers()` as the FIRST line in `init_pool()`
  - [x] 3.4 Configure `AnyPoolOptions`: max_connections from config (default 5), min_connections 0, acquire_timeout 15s, idle_timeout 60s, max_lifetime 30min
  - [x] 3.5 Return `AnyPool` on success; return a descriptive error on failure (connection refused, invalid URL, auth failure)

- [x] Task 4: Create db module — migration runner (AC: #2)
  - [x] 4.1 Create `server/src/db/migrate.rs` — embedded migration runner
  - [x] 4.2 Use `sqlx::migrate!()` macro to embed migrations at compile time (defaults to `migrations/`)
  - [x] 4.3 Expose `run_migrations(pool: &AnyPool)` function
  - [x] 4.4 On migration failure, return error with clear context (which migration failed, why)

- [x] Task 5: Create DatabaseBackend abstraction (AC: #4)
  - [x] 5.1 Create `server/src/db/backend.rs` — backend enum + helpers
  - [x] 5.2 Define `DatabaseBackend` enum with `Postgres` and `Sqlite` variants
  - [x] 5.3 Add `from_url(url: &str)` and `detect(url: &str)` for backend detection from config URLs
  - [x] 5.4 Add utility methods for backend-specific SQL patterns (e.g., `returning_clause()`, `upsert_syntax()`, `now_function()`)

- [x] Task 6: Create initial migration (AC: #2)
  - [x] 6.1 Create `server/migrations/` directory
  - [x] 6.2 Create `server/migrations/0001_initial_schema.sql` — a minimal migration that creates a `schema_metadata` metadata table with instance info (verifies the migration system works)
  - [x] 6.3 Ensure the migration SQL is compatible with BOTH PostgreSQL and SQLite

- [x] Task 7: Wire database into main.rs startup (AC: #1, #2, #3)
  - [x] 7.1 After config load + tracing init, call `db::init_pool(&config.database)`
  - [x] 7.2 On pool init failure, log error with `tracing::error!` and `exit(1)`
  - [x] 7.3 Call `db::run_migrations(&pool)` after pool creation
  - [x] 7.4 On migration failure, log error and `exit(1)`
  - [x] 7.5 Log success: "Database connected" with backend type and pool stats

- [x] Task 8: Wire pool into Axum shared state (AC: #1)
  - [x] 8.1 Create `AppState` struct holding `Arc<Config>` + `AnyPool` (or keep separate state extractors)
  - [x] 8.2 Update `handlers::router()` to accept the pool alongside config
  - [x] 8.3 Handlers can now extract pool via `State` for future stories

- [x] Task 9: Add build.rs for migration recompilation (AC: #2)
  - [x] 9.1 Create `server/build.rs` with `cargo:rerun-if-changed=migrations`
  - [x] 9.2 This ensures `cargo build` recompiles when migration files change

- [x] Task 10: Tests (AC: #1, #2, #3, #4, #5)
  - [x] 10.1 Unit test: `DatabaseBackend::detect()` returns correct variant for postgres:// and sqlite:// URLs
  - [x] 10.2 Unit test: `DatabaseConfig` validation rejects empty URL, invalid scheme
  - [x] 10.3 Unit test: `DatabaseConfig` validation accepts valid postgres:// and sqlite:// URLs
  - [x] 10.4 Integration test: pool connects to a SQLite in-memory database (`sqlite::memory:`)
  - [x] 10.5 Integration test: migrations run successfully on SQLite in-memory
  - [x] 10.6 Integration test: server starts with `database.url = "sqlite::memory:"` in config and `/healthz` returns 200
  - [x] 10.7 Unit test: config summary redacts database.url

- [x] Task 11: Code quality (AC: all)
  - [x] 11.1 Run `cargo fmt` and fix any formatting issues
  - [x] 11.2 Run `cargo clippy -- -D warnings` and resolve all warnings
  - [x] 11.3 Run `cargo test` and verify all tests pass
  - [x] 11.4 Verify `cargo build --release` succeeds

### Review Follow-ups (AI)

- [x] [AI-Review][MEDIUM] Avoid panic-based exits by removing `expect()` from the startup DB config access path. [server/src/main.rs]
- [x] [AI-Review][MEDIUM] Validate `database.max_connections` is >= 1 (prevents invalid pool config). [server/src/config/settings.rs]
- [x] [AI-Review][LOW] Log a warning when overriding `max_connections` to 1 for `sqlite::memory:`. [server/src/db/pool.rs]

## Dev Notes

### Architecture Compliance

**This story introduces the database foundation. Every future story that needs database access uses the pool and migration system created here. Patterns must be clean and extensible.**

#### Database Module Location (per architecture doc)

```
server/src/db/
├── mod.rs        # Module root: re-exports init_pool(), run_migrations(), DatabaseBackend
├── pool.rs       # AnyPool initialization, connection options
├── migrate.rs    # Embedded migration runner via sqlx::migrate!()
└── backend.rs    # DatabaseBackend enum, backend-specific SQL helpers
```

#### Dependency: sqlx 0.8.6

```toml
# Add to server/Cargo.toml [dependencies]
sqlx = { version = "0.8", features = [
    "runtime-tokio",
    "tls-rustls-ring-webpki",
    "any",
    "postgres",
    "sqlite",
    "migrate",
    "uuid",
    "chrono",
    "json",
] }
uuid = { version = "1", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }
```

**Why these features:**
- `runtime-tokio` — Axum uses Tokio; sqlx must match the async runtime
- `tls-rustls-ring-webpki` — Pure Rust TLS for PostgreSQL connections; no system OpenSSL dependency. SQLite ignores TLS.
- `any` — Enables `AnyPool`/`AnyConnection` for runtime driver switching between PG and SQLite
- `postgres` + `sqlite` — Both backend drivers compiled in; `install_default_drivers()` registers them
- `migrate` — `sqlx::migrate!()` macro to embed and run SQL migrations
- `uuid`, `chrono`, `json` — Type support for future stories (UUID PKs, timestamps, JSONB fields)

#### CRITICAL: install_default_drivers()

**Breaking change from sqlx 0.7 → 0.8:** The `Any` driver no longer auto-registers backends. You MUST call `sqlx::any::install_default_drivers()` before creating any `AnyPool` or `AnyConnection`. Failure to do so causes a **runtime panic**.

```rust
pub async fn init_pool(config: &DatabaseConfig) -> Result<AnyPool, sqlx::Error> {
    // CRITICAL: Must be called before any AnyPool/AnyConnection creation.
    // This registers all compiled-in drivers (postgres, sqlite) with the Any runtime.
    // Omitting this call causes a runtime panic in sqlx 0.8.x.
    sqlx::any::install_default_drivers();

    AnyPoolOptions::new()
        .max_connections(config.max_connections)
        .min_connections(0)
        .acquire_timeout(std::time::Duration::from_secs(15))
        .idle_timeout(Some(std::time::Duration::from_secs(60)))
        .max_lifetime(Some(std::time::Duration::from_secs(1800)))
        .connect(&config.url)
        .await
}
```

#### Database Config Section

Add to the existing `Config` struct in `server/src/config/settings.rs`:

```rust
#[derive(Debug, Clone, Deserialize)]
pub struct DatabaseConfig {
    pub url: String, // Required — no default. Validation catches missing.
    #[serde(default = "default_max_connections")]
    pub max_connections: u32,
}

fn default_max_connections() -> u32 { 5 }
```

**Important:** `DatabaseConfig` does NOT implement `Default` because `url` is required. The `Config` struct should use `Option<DatabaseConfig>` or make the `database` section required via validation:

```rust
#[derive(Debug, Clone, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub server: ServerConfig,
    #[serde(default)]
    pub log: LogConfig,
    pub database: Option<DatabaseConfig>, // None if section missing from TOML
}
```

Then in `validate()`:
```rust
if self.database.is_none() {
    return Err(ConfigValidationError::new(
        "database.url",
        "required — set database.url in config or DISCOOL_DATABASE__URL env var",
    ));
}
let db = self.database.as_ref().unwrap();
if db.url.trim().is_empty() {
    return Err(ConfigValidationError::new("database.url", "must not be empty"));
}
if !db.url.starts_with("postgres://")
    && !db.url.starts_with("postgresql://")
    && !db.url.starts_with("sqlite://")
    && !db.url.starts_with("sqlite:")
{
    return Err(ConfigValidationError::new(
        "database.url",
        "must start with postgres://, postgresql://, sqlite://, or sqlite:",
    ));
}
```

**SQLite URL formats** accepted by sqlx:
- `sqlite::memory:` — in-memory database (for testing)
- `sqlite://path/to/db.sqlite` — file-based database
- `sqlite:./data/discool.db?mode=rwc` — relative path with create mode

**PostgreSQL URL formats:**
- `postgres://user:pass@localhost:5432/discool`
- `postgresql://user:pass@host/dbname?sslmode=require`

#### Connection Pool Sizing for 2GB RAM Target

| Parameter | Value | Rationale |
|---|---|---|
| `max_connections` | 5 (configurable) | Each PG connection uses ~5-10MB RSS. 5 connections = ~50MB max. Leaves headroom for the app, P2P, WebSocket connections within 2GB. |
| `min_connections` | 0 | Don't hold connections when idle. The server may be freshly started with no traffic. |
| `acquire_timeout` | 15s | Fail fast if pool is exhausted under load. Default 30s is too long for a real-time chat app. |
| `idle_timeout` | 60s | Release idle connections quickly to free memory. |
| `max_lifetime` | 30min | Retire long-lived connections to avoid stale state. |
| `test_before_acquire` | true (default) | Keep enabled — prevents handing out dead connections after network blips. Adds ~1ms latency per acquire. |

For SQLite: If using file-based SQLite, `max_connections` > 1 is fine with WAL mode. For in-memory SQLite (`sqlite::memory:`), max_connections MUST be 1 to avoid separate in-memory databases per connection.

#### Migration System

Migrations live in `server/migrations/` and follow sqlx naming: `{version}_{description}.sql`.

```rust
use sqlx::AnyPool;

pub async fn run_migrations(pool: &AnyPool) -> Result<(), sqlx::migrate::MigrateError> {
    sqlx::migrate!("migrations").run(pool).await
}
```

The `migrate!()` macro path is relative to `Cargo.toml` (i.e., `server/migrations/` since Cargo.toml is in `server/`).

**Initial migration** (`server/migrations/0001_initial_schema.sql`):
```sql
-- Initial schema migration: verifies the migration system works.
-- Actual domain tables (users, guilds, channels, etc.) are created by their respective stories.
CREATE TABLE IF NOT EXISTS schema_metadata (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

INSERT INTO schema_metadata (key, value) VALUES ('initialized_at', CURRENT_TIMESTAMP);
```

This migration is intentionally minimal — it proves the migration system works end-to-end on both PG and SQLite. Domain tables are created by their respective stories (users in 2.1, guilds in 4.2, etc.).

**Recompilation on migration changes** — add `server/build.rs`:
```rust
fn main() {
    println!("cargo:rerun-if-changed=migrations");
}
```

#### DatabaseBackend Enum

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DatabaseBackend {
    Postgres,
    Sqlite,
}

impl DatabaseBackend {
    /// Detect the backend from the database URL.
    pub fn from_url(url: &str) -> Result<Self, String> {
        if url.starts_with("postgres://") || url.starts_with("postgresql://") {
            Ok(Self::Postgres)
        } else if url.starts_with("sqlite://") || url.starts_with("sqlite:") {
            Ok(Self::Sqlite)
        } else {
            Err(format!("unsupported database URL scheme: {url}"))
        }
    }

    /// SQL fragment for getting the current timestamp.
    pub fn now_function(&self) -> &'static str {
        match self {
            Self::Postgres => "NOW()",
            Self::Sqlite => "datetime('now')",
        }
    }

    /// Whether the backend supports RETURNING clauses.
    pub fn supports_returning(&self) -> bool {
        match self {
            Self::Postgres => true,
            Self::Sqlite => true, // SQLite 3.35+ supports RETURNING
        }
    }
}
```

This enum is used by future stories when writing queries that differ between backends. For this story, just define the enum and the detection logic. Utility methods will be added as needed by future stories.

#### AppState Pattern

The current codebase passes `Arc<Config>` as Axum state. With the pool added, create a shared `AppState`:

```rust
use sqlx::AnyPool;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub pool: AnyPool,
}
```

Update `handlers::router()` to accept `AppState` instead of `Arc<Config>`. Handlers extract via `State(state): State<AppState>`.

**Alternative:** Use Axum's `FromRef` trait to keep extracting `Arc<Config>` and `AnyPool` separately. Either approach works; the `AppState` struct is simpler and more explicit.

#### main.rs Update Pattern

The updated `main()` flow:

```rust
#[tokio::main]
async fn main() {
    // 1. Load config (before tracing)
    let config = match discool_server::config::load() { ... };
    if let Err(err) = config.validate() { ... }

    // 2. Initialize tracing
    init_tracing(&config.log);
    config.log_summary();

    // 3. Connect to database (NEW)
    let db_config = config.database.as_ref().expect("validated");
    let pool = match discool_server::db::init_pool(db_config).await {
        Ok(pool) => pool,
        Err(err) => {
            tracing::error!(%err, "Failed to connect to database");
            std::process::exit(1);
        }
    };

    // 4. Run migrations (NEW)
    if let Err(err) = discool_server::db::run_migrations(&pool).await {
        tracing::error!(%err, "Failed to run database migrations");
        std::process::exit(1);
    }
    tracing::info!(
        backend = %discool_server::db::DatabaseBackend::from_url(&db_config.url)
            .map(|b| format!("{b:?}")).unwrap_or_else(|e| e),
        "Database connected and migrations complete"
    );

    // 5. Build app state and start server
    let state = discool_server::AppState {
        config: std::sync::Arc::new(config),
        pool,
    };
    let app = handlers::router(state.clone());
    // ... bind listener, serve (same as before)
}
```

**Critical ordering:** Database init happens AFTER tracing is initialized so errors can use `tracing::error!`. Config validation (which checks `database.url` is present) happens BEFORE tracing, using `eprintln!`.

#### Config Summary Update — Wire redact_secret()

The `log_summary()` method established in Story 1.2 has `redact_secret()` ready but not wired. This story wires it:

```rust
pub fn log_summary(&self) {
    let db_url = self.database.as_ref()
        .map(|db| redact_secret(&db.url))
        .unwrap_or("[not configured]");

    tracing::info!(
        host = %self.server.host,
        port = self.server.port,
        log_level = %self.log.level,
        log_format = %self.log.format,
        database_url = %db_url,
        "Configuration loaded"
    );
}
```

The `redact_secret()` function from Story 1.2 currently returns `"[REDACTED]"` for all input. This is fine — the point is that database URLs (which contain passwords) are never logged in full.

### Existing Code to Modify

| File | Change |
|---|---|
| `server/Cargo.toml` | Add sqlx, uuid, chrono dependencies |
| `server/src/config/settings.rs` | Add `DatabaseConfig` struct, validation for database.url, wire `redact_secret()` into `log_summary()` |
| `server/src/lib.rs` | Add `pub mod db;`, export `AppState` |
| `server/src/main.rs` | Add database init + migration run between tracing init and server start |
| `server/src/handlers/mod.rs` | Update `router()` to accept `AppState` instead of `Arc<Config>` |
| `config.example.toml` | Add `[database]` section |

### Files to Create

| File | Purpose |
|---|---|
| `server/src/db/mod.rs` | Module root, re-exports |
| `server/src/db/pool.rs` | `init_pool()` — AnyPool creation with config-driven options |
| `server/src/db/migrate.rs` | `run_migrations()` — embedded migration runner |
| `server/src/db/backend.rs` | `DatabaseBackend` enum, backend detection, SQL helpers |
| `server/migrations/0001_initial_schema.sql` | Initial migration — schema_metadata table |
| `server/build.rs` | `cargo:rerun-if-changed=migrations` for recompilation on new migrations |

### Project Structure Notes

- `server/src/db/` is the correct location per architecture doc
- `server/migrations/` is the correct migration directory per sqlx convention and architecture doc
- `AppState` struct should be defined in `server/src/lib.rs` (exported for use in main.rs and handlers)
- The `models/` directory is NOT created in this story — models are added by their respective domain stories

### Testing Requirements

- **Unit:** `DatabaseBackend::from_url()` detects postgres and sqlite correctly
- **Unit:** `DatabaseBackend::from_url()` rejects unknown schemes
- **Unit:** `DatabaseConfig` validation rejects empty URL
- **Unit:** `DatabaseConfig` validation rejects invalid URL scheme (e.g., `mysql://...`)
- **Unit:** `DatabaseConfig` validation accepts `postgres://...` and `sqlite::memory:`
- **Unit:** Config summary redacts database URL (never logs password)
- **Integration:** Pool connects to `sqlite::memory:` successfully
- **Integration:** Migrations run on `sqlite::memory:` without error
- **Integration:** Server starts with SQLite config, `/healthz` returns 200
- **Linting:** `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test` — all must pass

Note: Integration tests use SQLite in-memory (`sqlite::memory:`) to avoid requiring a running PostgreSQL instance. PostgreSQL testing is deferred to CI with a real PG container.

### Anti-Patterns to Avoid

- Do NOT use `sqlx::query!()` or `sqlx::query_as!()` compile-time macros with `AnyPool` — they require a concrete database type at compile time. Use `sqlx::query()` and `sqlx::query_as::<_, T>()` dynamic API instead.
- Do NOT forget `sqlx::any::install_default_drivers()` — omitting this causes a runtime panic in sqlx 0.8.x
- Do NOT use `unwrap()` or `expect()` for pool creation or migration in main — use match + tracing::error + exit(1)
- Do NOT hardcode pool sizes — make `max_connections` configurable via `[database]` config
- Do NOT set `min_connections > 0` — wastes memory holding idle connections on a 2GB server
- Do NOT create domain tables (users, guilds, etc.) in this story — each story creates its own tables
- Do NOT use offset-based pagination in any future queries — always cursor-based (architecture rule)
- Do NOT add `diesel` or `sea-orm` — sqlx is the chosen ORM-free query layer
- Do NOT use `PgPool` or `SqlitePool` directly — always use `AnyPool` for runtime switching
- Do NOT skip the `build.rs` recompilation trigger — without it, new migrations may not be picked up by `cargo build`

### Previous Story Intelligence

**From Story 1.2 (Configuration System):**
- Config loads via `config-rs` v0.15 with TOML + env var overrides
- Env var pattern: `DISCOOL_DATABASE__URL` maps to `database.url`
- `Config::validate()` pattern established — extend it for database section
- `redact_secret()` exists but is `#[allow(dead_code)]` — wire it now for `database.url`
- `Arc<Config>` passed as Axum state — transition to `AppState` struct
- Integration test pattern established in `server/tests/server_binds_to_configured_port.rs` — follow the same process-spawning pattern for database integration tests
- Error pattern: pre-tracing errors use `eprintln!` + `exit(1)`, post-tracing errors use `tracing::error!` + `exit(1)`

**Review follow-up from Story 1.2 (LOW priority, address now):**
- `redact_secret()` is present but not wired into `Config::log_summary()` — wire it for `database.url` in this story

### Git Intelligence

**Recent commits:**
- `e204ccb` — Add configuration system with structured logging support (Story 1.2)
- `a828f94` — Initial commit: BMAD framework, Svelte client, and Rust server (Story 1.1)

**Patterns established:**
- Rust module pattern: `mod.rs` + separate files per concern
- Config pattern: typed structs with `serde::Deserialize`, validation method, log summary
- Error pattern: `AppError` enum in `error.rs` with `IntoResponse`
- State pattern: `Arc<Config>` as Axum state (to be upgraded to `AppState`)
- Test pattern: inline `#[cfg(test)]` for unit tests, `server/tests/` for integration tests

### References

- [Source: _bmad-output/planning-artifacts/architecture.md#Data Architecture] — sqlx 0.8.6, Any driver, PG/SQLite, DatabaseBackend trait
- [Source: _bmad-output/planning-artifacts/architecture.md#Complete Project Directory Structure] — db/ module location, migrations/ directory
- [Source: _bmad-output/planning-artifacts/architecture.md#Implementation Patterns & Consistency Rules] — Error handling, naming conventions
- [Source: _bmad-output/planning-artifacts/architecture.md#Infrastructure & Deployment] — TOML config, env overrides, 2GB RAM target
- [Source: _bmad-output/planning-artifacts/epics.md#Story 1.3] — Acceptance criteria, story statement
- [Source: _bmad-output/planning-artifacts/prd.md#FR8-FR9] — Instance deployment, configuration
- [Source: _bmad-output/planning-artifacts/prd.md#NFR9] — 50 concurrent users on 2 vCPU / 2GB RAM
- [Source: _bmad-output/planning-artifacts/prd.md#NFR23] — 1M messages without query degradation
- [Source: _bmad-output/implementation-artifacts/1-2-configuration-system-and-structured-logging.md] — Previous story patterns, config module, redact_secret()
- [crates.io: sqlx 0.8.6] — Latest stable; MUST call install_default_drivers() for Any driver
- [sqlx CHANGELOG] — 0.8.x breaking change: explicit driver registration required
- [sqlx::migrate! docs] — Path relative to Cargo.toml, _sqlx_migrations tracking table

## Dev Agent Record

### Agent Model Used

GitHub Copilot CLI

### Debug Log References

- `cd server && cargo test -q` (baseline) ✅
- `cd server && cargo fmt` ✅
- `cd server && cargo clippy -q -- -D warnings` ✅
- `cd server && cargo test -q` ✅
- `cd server && cargo build --release -q` ✅

### Completion Notes List

- Added `sqlx` Any driver support (Postgres/SQLite) and type deps (`uuid`, `chrono`).
- Added `[database]` config (url required, max_connections default 5), validation, and redacted config summary logging.
- Implemented `server/src/db/` with `init_pool()` (sensible pool defaults; `sqlite::memory:` forces `max_connections=1`) and embedded migrations runner.
- Added initial migration (`server/migrations/0001_initial_schema.sql`) to validate migration plumbing on both backends.
- Wired DB init + migrations into server startup and introduced `AppState { config, pool }` for Axum.
- Added `/healthz` endpoint plus unit/integration tests covering backend detection, config validation, pool/migrations, and server startup.
- Senior dev review: removed panic-based DB config access; added validation for `database.max_connections >= 1`.

### File List

- config.example.toml
- server/Cargo.toml
- server/Cargo.lock
- server/build.rs
- server/migrations/0001_initial_schema.sql
- server/src/config/mod.rs
- server/src/config/settings.rs
- server/src/db/backend.rs
- server/src/db/migrate.rs
- server/src/db/mod.rs
- server/src/db/pool.rs
- server/src/handlers/mod.rs
- server/src/lib.rs
- server/src/main.rs
- server/tests/database_sqlite_in_memory.rs
- server/tests/server_binds_to_configured_port.rs
- _bmad-output/implementation-artifacts/sprint-status.yaml
- _bmad-output/implementation-artifacts/1-3-database-connection-and-migration-system.md

## Change Log

- 2026-02-18: Implemented sqlx AnyPool database init + embedded migrations; added AppState with pool, config validation + redacted logging, and unit/integration tests. Story marked ready for review.
- 2026-02-18: Senior dev review fixes applied (panic-free startup, `database.max_connections` validation); story marked done.
- 2026-02-18: Code review follow-ups: sanitize DB connection error logs, warn/keep `sqlite::memory:` pool stable, and align story Task 6.2 migration table name.

## Senior Developer Review (AI)

Reviewer: Darko
Date: 2026-02-18

### Findings

- **[HIGH]** DB connection failures logged raw sqlx errors, which may include the full database URL (credentials leak risk). (`server/src/main.rs`)
- **[MEDIUM]** `sqlite::memory:` pool behavior was surprising: forced `max_connections = 1` silently and could drop the only connection due to idle timeout, resetting the in-memory DB. (`server/src/db/pool.rs`)
- **[MEDIUM]** Story task 6.2 referenced `_schema_version` but the migration/test use `schema_metadata` (doc mismatch). (`_bmad-output/implementation-artifacts/1-3-database-connection-and-migration-system.md`)
- **[MEDIUM]** `main.rs` used `expect("validated")` for `database` access; violates "no unwrap/expect" guideline and can panic-exit. (`server/src/main.rs`)
- **[MEDIUM]** Config validation allowed `database.max_connections = 0`, which can produce invalid pool configuration at runtime. (`server/src/config/settings.rs`)
- **[LOW]** Config validation only checks `database.url` scheme prefixes; malformed-but-prefixed URLs will fail later during connection init instead of at validate-time. (`server/src/config/settings.rs`)

### Fixes Applied

- Sanitized DB connection error logging by redacting the configured DB URL if it appears in the error string. (`server/src/main.rs`)
- Added warning + in-memory-safe pooling settings for `sqlite::memory:` (force `max_connections=1`, keep connection alive). (`server/src/db/pool.rs`)
- Updated Task 6.2 to match the actual migration table name (`schema_metadata`). (`_bmad-output/implementation-artifacts/1-3-database-connection-and-migration-system.md`)
- Removed `expect()` and replaced with explicit handling + non-zero exit. (`server/src/main.rs`)
- Added `database.max_connections >= 1` validation + unit test. (`server/src/config/settings.rs`)

### Verification

- `cd server && cargo test -q` ✅
