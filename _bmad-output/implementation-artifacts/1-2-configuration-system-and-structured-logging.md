# Story 1.2: Configuration System and Structured Logging

Status: ready-for-dev

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As an **operator**,
I want to configure my instance via a TOML config file with environment variable overrides,
So that I can customize settings for my deployment without modifying code.

## Acceptance Criteria

1. **Given** a TOML file exists at `./config.toml` or `/etc/discool/config.toml`
   **When** the server starts
   **Then** it loads settings from that file

2. **Given** environment variables are set (e.g., `DISCOOL_SERVER__PORT=8080`)
   **When** the server starts
   **Then** env vars override TOML values

3. **Given** missing required config values
   **When** the server starts
   **Then** it exits with a clear error message naming the missing field(s)

4. **Given** the tracing crate provides structured JSON logging with configurable verbosity
   **When** the server logs an event
   **Then** the log includes timestamp, level, module, message, and any structured fields

5. **Given** a log event contains user input
   **When** the event is serialized
   **Then** no PII appears in logs by default

6. **Given** the server starts successfully
   **When** it has loaded the configuration
   **Then** the server logs its configuration summary (redacting secrets) on startup

## Tasks / Subtasks

- [ ] Task 1: Add config-rs dependency (AC: #1, #2)
  - [ ] 1.1 Add `config = { version = "0.15", features = ["toml"] }` to `server/Cargo.toml`
  - [ ] 1.2 Verify the dependency resolves and compiles

- [ ] Task 2: Create config module with typed structs (AC: #1, #3)
  - [ ] 2.1 Create `server/src/config/mod.rs` — module root, re-exports `Config` and `load()`
  - [ ] 2.2 Create `server/src/config/settings.rs` — typed config structs with `serde::Deserialize`
  - [ ] 2.3 Define `Config` struct with sections: `server` (ServerConfig), `log` (LogConfig)
  - [ ] 2.4 Define `ServerConfig` with `host: String` (default `"0.0.0.0"`) and `port: u16` (default `3000`)
  - [ ] 2.5 Define `LogConfig` with `level: String` (default `"info"`) and `format: LogFormat` enum (`json` | `pretty`, default `json`)
  - [ ] 2.6 Implement `Default` for all config structs

- [ ] Task 3: Implement config loading with file discovery + env overrides (AC: #1, #2)
  - [ ] 3.1 Implement `config::load()` function using config-rs `Config::builder()`
  - [ ] 3.2 Add config file sources: `./config.toml` (required: false), `/etc/discool/config.toml` (required: false)
  - [ ] 3.3 Support `DISCOOL_CONFIG` env var to specify a custom config file path (loaded last among files, highest file priority)
  - [ ] 3.4 Add environment variable source with prefix `DISCOOL` and `__` nested separator
  - [ ] 3.5 Call `.try_deserialize::<Config>()` to produce typed config

- [ ] Task 4: Implement config validation (AC: #3)
  - [ ] 4.1 Add a `Config::validate()` method that checks semantic constraints (port range, valid log level, etc.)
  - [ ] 4.2 On config load failure or validation failure in `main()`, print a clear error to stderr naming the missing/invalid field and exit with code 1
  - [ ] 4.3 Do NOT use tracing for config errors (tracing isn't initialized yet at that point) — use `eprintln!`

- [ ] Task 5: Wire config into main.rs (AC: #1, #4, #6)
  - [ ] 5.1 Update `main.rs` to call `config::load()` before anything else
  - [ ] 5.2 Pass loaded config to `init_tracing()` to set log level and format from config
  - [ ] 5.3 Use `config.server.host` and `config.server.port` for `TcpListener::bind()` instead of hardcoded `0.0.0.0:3000`
  - [ ] 5.4 Log the config summary after tracing is initialized (AC #6)
  - [ ] 5.5 Pass config as `Arc<Config>` shared state to the Axum router via `.with_state()`

- [ ] Task 6: Update tracing initialization (AC: #4, #5)
  - [ ] 6.1 Refactor `init_tracing()` to accept `&LogConfig` and configure level from `config.log.level`
  - [ ] 6.2 Support `RUST_LOG` env var as highest-priority override for log level (existing behavior preserved)
  - [ ] 6.3 Switch between JSON and pretty log format based on `config.log.format`
  - [ ] 6.4 Ensure JSON logs include: timestamp, level, module path (target), message, and structured fields

- [ ] Task 7: Implement config summary logging with secret redaction (AC: #6)
  - [ ] 7.1 Implement a `Config::log_summary()` method that logs each config section at `info` level
  - [ ] 7.2 Redact any fields that contain secrets (database URLs, API keys) — replace with `"[REDACTED]"`
  - [ ] 7.3 For this story, the only potentially sensitive field is the future `database.url` — establish the redaction pattern now

- [ ] Task 8: Update config.example.toml (AC: #1)
  - [ ] 8.1 Replace the placeholder `config.example.toml` with a fully documented example
  - [ ] 8.2 Include all config sections with their defaults, commented out, with explanatory comments

- [ ] Task 9: Update lib.rs and router for shared state (AC: #1)
  - [ ] 9.1 Add `pub mod config;` to `server/src/lib.rs`
  - [ ] 9.2 Update `handlers::router()` to accept and propagate `Arc<Config>` as Axum state (or make router generic over state for now)

- [ ] Task 10: Tests (AC: #1, #2, #3, #4)
  - [ ] 10.1 Unit test: config loads from a TOML string with all defaults
  - [ ] 10.2 Unit test: config validation rejects invalid port (0, or empty required fields when defined)
  - [ ] 10.3 Unit test: LogFormat deserialization from string ("json", "pretty")
  - [ ] 10.4 Unit test: config summary redacts sensitive fields
  - [ ] 10.5 Integration test: server starts with a minimal config.toml and binds to the configured port

- [ ] Task 11: Code quality (AC: all)
  - [ ] 11.1 Run `cargo fmt` and fix any formatting issues
  - [ ] 11.2 Run `cargo clippy -- -D warnings` and resolve all warnings
  - [ ] 11.3 Run `cargo test` and verify all tests pass
  - [ ] 11.4 Verify `cargo build --release` succeeds

## Dev Notes

### Architecture Compliance

**This story introduces the configuration foundation. Every future story that needs a new config field adds to the structs created here. The patterns established must be clean and extensible.**

#### Config Module Location (per architecture doc)

```
server/src/config/
├── mod.rs        # Module root: re-exports Config, load()
└── settings.rs   # All typed config structs, Default impls, validation
```

#### Dependency: config-rs (NOT figment, NOT raw toml)

Use the `config` crate (config-rs) v0.15.x:
- **Actively maintained** — multiple releases in 2025, owned by the `rust-cli` GitHub org (same as `clap`)
- Supports TOML files + env var overrides natively via `Config::builder()`
- `figment` (alternative) has not released since May 2024 — avoid for new code
- Raw `toml` crate has no env var override support — too low-level

```toml
# Add to server/Cargo.toml [dependencies]
config = { version = "0.15", features = ["toml"] }
```

#### Config Loading Pattern

```rust
use config::{Config as ConfigBuilder, Environment, File, FileFormat};

pub fn load() -> Result<Config, config::ConfigError> {
    let mut builder = ConfigBuilder::builder()
        // Lowest priority: system-wide config
        .add_source(File::new("/etc/discool/config", FileFormat::Toml).required(false))
        // Higher priority: local config (development)
        .add_source(File::new("config", FileFormat::Toml).required(false));

    // Highest file priority: custom path via DISCOOL_CONFIG env var
    if let Ok(path) = std::env::var("DISCOOL_CONFIG") {
        builder = builder.add_source(File::new(&path, FileFormat::Toml).required(true));
    }

    builder
        // Highest priority: env vars (DISCOOL_SERVER__PORT=8080 -> server.port)
        .add_source(
            Environment::with_prefix("DISCOOL")
                .prefix_separator("_")
                .separator("__"),
        )
        .build()?
        .try_deserialize()
}
```

**Priority order (highest to lowest):**
1. Environment variables (`DISCOOL_SERVER__PORT=8080`)
2. Custom config file (`DISCOOL_CONFIG=/path/to/config.toml`)
3. Local config file (`./config.toml`)
4. System config file (`/etc/discool/config.toml`)
5. Struct defaults (via `Default` impl)

#### Environment Variable Naming Convention

The `__` (double underscore) separates nested config sections:

| Env Var | Maps To | Example |
|---|---|---|
| `DISCOOL_SERVER__PORT` | `server.port` | `DISCOOL_SERVER__PORT=8080` |
| `DISCOOL_SERVER__HOST` | `server.host` | `DISCOOL_SERVER__HOST=127.0.0.1` |
| `DISCOOL_LOG__LEVEL` | `log.level` | `DISCOOL_LOG__LEVEL=debug` |
| `DISCOOL_LOG__FORMAT` | `log.format` | `DISCOOL_LOG__FORMAT=pretty` |

The double-underscore is the standard Rust config ecosystem convention (used by config-rs and figment). Single underscore is ambiguous for nested keys (e.g., `SERVER_PORT` could mean `server_port` or `server.port`).

**Note:** The epics document uses `DISCOOL_PORT=8080` as a simplified illustrative example. The actual implementation uses `DISCOOL_SERVER__PORT=8080` because the config is nested. Document this clearly in `config.example.toml`.

#### Typed Config Structs

```rust
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub server: ServerConfig,
    #[serde(default)]
    pub log: LogConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LogConfig {
    #[serde(default = "default_log_level")]
    pub level: String,
    #[serde(default)]
    pub format: LogFormat,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum LogFormat {
    Json,
    Pretty,
}

impl Default for LogFormat {
    fn default() -> Self {
        LogFormat::Json
    }
}

fn default_host() -> String { "0.0.0.0".to_string() }
fn default_port() -> u16 { 3000 }
fn default_log_level() -> String { "info".to_string() }
```

**Important:** All sections use `#[serde(default)]` so the server starts with zero config (all defaults). This means an empty `config.toml` or no config file at all is valid — the server just uses defaults. Required fields (like `database.url` in Story 1.3) will NOT have defaults and will fail clearly.

#### Tracing Initialization Pattern

```rust
use tracing_subscriber::{fmt, EnvFilter, prelude::*};

pub fn init_tracing(log_config: &LogConfig) {
    // RUST_LOG env var takes highest priority for level (debugging escape hatch)
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(&log_config.level));

    match log_config.format {
        LogFormat::Json => {
            tracing_subscriber::registry()
                .with(filter)
                .with(fmt::layer().json())
                .init();
        }
        LogFormat::Pretty => {
            tracing_subscriber::registry()
                .with(filter)
                .with(fmt::layer().pretty())
                .init();
        }
    }
}
```

**Key behaviors:**
- `RUST_LOG=debug` always overrides the config file level — this is the standard Rust debugging escape hatch
- JSON format includes: timestamp, level, target (module path), message, structured fields — all automatically provided by `tracing-subscriber`
- Pretty format is human-readable — useful for local development
- The default (JSON) is the right choice for production (machine-parseable, compatible with log aggregators)

#### Config Summary Logging (AC #6)

After tracing is initialized, log the effective config. Establish a pattern for redacting secrets:

```rust
impl Config {
    pub fn log_summary(&self) {
        tracing::info!(
            host = %self.server.host,
            port = self.server.port,
            log_level = %self.log.level,
            log_format = ?self.log.format,
            "Configuration loaded"
        );
    }
}
```

For future stories that add sensitive fields (e.g., `database.url`), the pattern is:
```rust
fn redact(value: &str) -> String {
    if value.len() <= 8 { return "[REDACTED]".to_string(); }
    format!("{}…[REDACTED]", &value[..8])
}
```

#### PII Prevention in Logs (AC #5)

Establish these patterns from day one:
- **Never** log raw user input (messages, usernames, emails) in production log events
- Use `tracing::instrument` with `skip` for functions that handle user data: `#[instrument(skip(user_input))]`
- Log identifiers (user IDs, guild IDs) but not content
- The `json` format doesn't automatically include PII — it only logs what you explicitly put in structured fields
- For this story, there's no user input handling yet, but establish the pattern in comments/documentation

#### main.rs Update Pattern

The updated `main()` flow:

```rust
#[tokio::main]
async fn main() {
    // 1. Load config (before tracing — use eprintln! for errors)
    let config = match discool_server::config::load() {
        Ok(cfg) => cfg,
        Err(err) => {
            eprintln!("ERROR: Failed to load configuration: {err}");
            std::process::exit(1);
        }
    };

    // 2. Validate config
    if let Err(err) = config.validate() {
        eprintln!("ERROR: Invalid configuration: {err}");
        std::process::exit(1);
    }

    // 3. Initialize tracing (now we can use tracing macros)
    init_tracing(&config.log);

    // 4. Log config summary
    config.log_summary();

    // 5. Build and start server
    let config = std::sync::Arc::new(config);
    let addr = format!("{}:{}", config.server.host, config.server.port);
    // ... bind listener, build router with state, serve
}
```

**Critical:** Config loading and validation happen BEFORE tracing init. Errors at this stage use `eprintln!` to stderr, not `tracing::error!`.

#### Axum Router State Update

The router needs to accept shared state for future handlers to access config:

```rust
use std::sync::Arc;
use axum::Router;

pub fn router(config: Arc<Config>) -> Router {
    let api = Router::new()
        .route("/ping", get(ping))
        .fallback(get(api_not_found));

    Router::new()
        .nest("/api/v1", api)
        .route("/ws", get(ws_not_found))
        .fallback(get(crate::static_files::handler))
        .layer(middleware::from_fn(security_headers))
        .with_state(config)
}
```

For now, no handler needs to access config. The state is passed to enable future stories to use `State(config): State<Arc<Config>>` in handler signatures without restructuring the router.

### Existing Code to Modify

These files from Story 1.1 must be updated:

| File | Change |
|---|---|
| `server/Cargo.toml` | Add `config = { version = "0.15", features = ["toml"] }` |
| `server/src/lib.rs` | Add `pub mod config;` |
| `server/src/main.rs` | Replace hardcoded port with config, refactor `init_tracing()`, add config load/validate/summary |
| `server/src/handlers/mod.rs` | Update `router()` to accept `Arc<Config>` state |
| `config.example.toml` | Replace placeholder with fully documented example |

### Files to Create

| File | Purpose |
|---|---|
| `server/src/config/mod.rs` | Module root, re-exports |
| `server/src/config/settings.rs` | Typed config structs, Default impls, load(), validate(), log_summary() |

### Testing Requirements

- **Unit:** Config deserializes from a TOML string with all defaults (empty string → valid Config with default values)
- **Unit:** Config with explicit values overrides defaults correctly
- **Unit:** `LogFormat` deserializes from `"json"` and `"pretty"` strings
- **Unit:** `LogFormat` deserialization fails on invalid string (e.g., `"invalid"`)
- **Unit:** `validate()` passes for default config
- **Unit:** `validate()` fails for port 0
- **Unit:** Config summary includes expected fields and redacts secrets
- **Integration:** Server binary starts with `./config.toml` containing `[server]\nport = 3001` and binds to port 3001
- **Linting:** `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test` — all must pass

### Anti-Patterns to Avoid

- Do NOT use `figment` crate — hasn't been released since May 2024, uncertain maintenance
- Do NOT use raw `toml` crate directly — no env var override support, too low-level
- Do NOT use `unwrap()` or `expect()` for config loading in `main()` — use match + `eprintln!` + `exit(1)` for clear error messages
- Do NOT use `tracing::error!` for config load failures — tracing isn't initialized yet at that point
- Do NOT add database, P2P, voice, or cache config sections — those belong to their respective stories
- Do NOT add a `--config` CLI flag (no `clap` dependency) — env var `DISCOOL_CONFIG` is sufficient
- Do NOT change the `config.example.toml` filename or move it — it stays at project root
- Do NOT log the full config struct via `Debug` — use explicit field logging to control what's exposed
- Do NOT make config mutable at runtime — load once at startup, wrap in `Arc`, share immutably

### References

- [Source: _bmad-output/planning-artifacts/architecture.md#Infrastructure & Deployment] — TOML config + env var overrides
- [Source: _bmad-output/planning-artifacts/architecture.md#Complete Project Directory Structure] — config/ module location
- [Source: _bmad-output/planning-artifacts/architecture.md#Implementation Patterns & Consistency Rules] — Error handling, logging patterns
- [Source: _bmad-output/planning-artifacts/prd.md#FR9] — Configuration file supports server, networking, storage settings
- [Source: _bmad-output/planning-artifacts/prd.md#NFR38] — Structured logging with configurable verbosity; no PII in logs
- [Source: _bmad-output/implementation-artifacts/1-1-initialize-project-scaffold-and-serve-spa.md] — Previous story, existing codebase
- [crates.io: config 0.15.19] — Actively maintained config crate (rust-cli org)
- [crates.io: toml 1.0.2] — TOML parser (used internally by config-rs)

## Dev Agent Record

### Agent Model Used

{{agent_model_name_version}}

### Debug Log References

### Completion Notes List

### File List
