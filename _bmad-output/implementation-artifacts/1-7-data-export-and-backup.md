# Story 1.7: Data Export and Backup

Status: ready-for-dev

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As an **operator**,
I want to export and back up my instance data,
So that I can restore my instance or migrate to new hardware without data loss.

## Acceptance Criteria

1. **Given** the operator is authenticated as the instance admin
   **When** they trigger a backup via the admin panel or CLI command
   **Then** a complete database export is produced (SQL dump for PG, file copy for SQLite)

2. **Given** a backup is triggered
   **When** the database is SQLite
   **Then** a consistent snapshot is produced via `VACUUM INTO` (handles WAL mode correctly)
   **And** when the database is PostgreSQL, a `pg_dump --format=plain` SQL dump is produced

3. **Given** a backup is produced
   **Then** the export includes all tables: `schema_metadata`, `instance_settings`, `admin_users`, and any future domain tables (guilds, channels, messages, users, roles, configuration)

4. **Given** the admin panel is displayed
   **When** the operator clicks the "Download Backup" button
   **Then** the backup file is downloaded via the browser
   **And** if `backup.output_dir` is configured in TOML, the backup is also saved to that directory

5. **Given** a backup file from a SQLite instance
   **When** the operator copies it as the database file on a fresh instance and starts the server
   **Then** the instance is fully functional with all data intact (NFR29)
   **And** given a SQL dump from a PostgreSQL instance, restoring via `psql -f backup.sql` produces a fully functional instance

6. **Given** the admin panel backup operation takes more than 2 seconds
   **Then** a "Creating backup..." status is shown in the admin panel during the operation

## Tasks / Subtasks

- [ ] Task 1: Add optional backup configuration (AC: #4)
  - [ ] 1.1 Add `BackupConfig` struct to `server/src/config/settings.rs` with `output_dir: Option<String>` field
  - [ ] 1.2 Add `backup: Option<BackupConfig>` field to `Config` struct with `#[serde(default)]`
  - [ ] 1.3 In `Config::validate()`, if `backup.output_dir` is set, verify the directory exists or can be created (log a warning, do not fail startup)

- [ ] Task 2: Backend backup endpoint (AC: #1, #2, #3, #4, #5)
  - [ ] 2.1 Add `create_backup` async handler in `server/src/handlers/admin.rs`
  - [ ] 2.2 Guard with pre-auth check: return 403 (`AppError::Forbidden`) if instance is not initialized (same pattern as `get_health`)
  - [ ] 2.3 Detect database backend via `DatabaseBackend::from_url()` on the configured database URL
  - [ ] 2.4 Generate a temp file path: `std::env::temp_dir().join(format!("discool-backup-{timestamp}.{ext}"))` where ext is `db` (SQLite) or `sql` (PostgreSQL)
  - [ ] 2.5 SQLite backup: execute `VACUUM INTO '{temp_path}'` via `sqlx::query` — produces a consistent snapshot including WAL data
  - [ ] 2.6 PostgreSQL backup: spawn `tokio::process::Command::new("pg_dump")` with `--dbname <url> --format=plain --file <temp_path>` — return `AppError::Internal` if `pg_dump` is not found or exits non-zero
  - [ ] 2.7 Read the temp file into a `Vec<u8>` response body (acceptable for MVP database sizes)
  - [ ] 2.8 Set response headers: `Content-Type: application/octet-stream` and `Content-Disposition: attachment; filename="discool-backup-{timestamp}.{ext}"`
  - [ ] 2.9 If `config.backup.output_dir` is set: also copy the backup file to that directory (best-effort — log warning on failure, do not fail the response)
  - [ ] 2.10 Clean up the temp file after response body is read (use `defer`-style cleanup or explicit `tokio::fs::remove_file`)
  - [ ] 2.11 Generate filename with UTC timestamp: `discool-backup-2026-02-19-143022.db` or `.sql`

- [ ] Task 3: Wire backup route into router (AC: #1)
  - [ ] 3.1 Add `.route("/admin/backup", post(admin::create_backup))` to the `/api/v1` router in `server/src/handlers/mod.rs`
  - [ ] 3.2 Ensure the route is within the `tracked` router so metrics track it when enabled

- [ ] Task 4: Client-side backup API function (AC: #1, #4)
  - [ ] 4.1 Add `downloadBackup()` async function to `client/src/lib/api.ts` that POSTs to `/api/v1/admin/backup`, receives the binary response, and triggers a browser file download via Blob URL + programmatic `<a>` click
  - [ ] 4.2 Handle error responses (403, 500) by parsing the JSON error body and throwing `ApiError`

- [ ] Task 5: AdminPanel backup section (AC: #1, #4, #6)
  - [ ] 5.1 Add a "Backup" card section in `client/src/lib/components/AdminPanel.svelte` below the existing health metrics
  - [ ] 5.2 Add a "Download Backup" button styled as a fire CTA (primary action)
  - [ ] 5.3 On click: disable the button, show "Creating backup..." status text
  - [ ] 5.4 On success: trigger browser download, show brief "Backup complete" text, re-enable button
  - [ ] 5.5 On error: show error message with "Retry" button (fire CTA), same pattern as health error state
  - [ ] 5.6 Follow Dual Core design system: dark zinc background, ice blue for labels, fire for the CTA button

- [ ] Task 6: Server tests (AC: #1, #2, #3, #5)
  - [ ] 6.1 Unit test: `POST /api/v1/admin/backup` returns 403 when instance is not initialized
  - [ ] 6.2 Unit test: `POST /api/v1/admin/backup` returns 200 with `Content-Type: application/octet-stream` on initialized instance
  - [ ] 6.3 Unit test: response has `Content-Disposition` header with `attachment` and a `.db` filename
  - [ ] 6.4 Unit test: response body starts with SQLite magic bytes (`SQLite format 3\0`) — 16 bytes
  - [ ] 6.5 Unit test: backup of initialized instance contains `instance_settings` and `admin_users` data (open the backup bytes as a new SQLite connection, query tables)
  - [ ] 6.6 Integration test: `POST /api/v1/admin/backup` before setup returns 403
  - [ ] 6.7 Integration test: `POST /api/v1/admin/backup` after setup returns 200 with downloadable backup
  - [ ] 6.8 Integration test: backup response body starts with SQLite magic bytes

- [ ] Task 7: Code quality (AC: all)
  - [ ] 7.1 Run `cargo fmt` and fix any formatting issues
  - [ ] 7.2 Run `cargo clippy -- -D warnings` and resolve all warnings
  - [ ] 7.3 Run `cargo test` and verify all tests pass
  - [ ] 7.4 Run `cargo build --release` and verify it succeeds
  - [ ] 7.5 Run `npx biome check .` in client directory and fix issues
  - [ ] 7.6 Run `npx svelte-check --tsconfig ./tsconfig.app.json` in client directory and fix issues

## Dev Notes

### Architecture Compliance

**This story adds a database backup/export endpoint and a corresponding admin panel UI section. It extends the existing `admin.rs` handler module and `AdminPanel.svelte` component from Story 1.6. The backup approach uses native database tools: `VACUUM INTO` for SQLite and `pg_dump` for PostgreSQL.**

#### Server Module Location (per architecture doc)

```
server/src/handlers/
├── mod.rs        # Route registration (MODIFY — add backup route)
├── health.rs     # /healthz, /readyz, /metrics (existing)
├── instance.rs   # /api/v1/instance, /api/v1/instance/setup (existing)
├── admin.rs      # /api/v1/admin/health (existing), /api/v1/admin/backup (NEW)
└── ...
```

**Why add to `admin.rs` instead of a new file:** Story 1.6 dev notes explicitly stated: "The admin module will grow to include backup/export (Story 1.7) and potentially other operator-only endpoints." Keeping backup in `admin.rs` follows this plan and avoids unnecessary file proliferation.

#### Client Component Location (per architecture doc)

```
client/src/
├── App.svelte                  # Root: already wired to show AdminPanel (NO CHANGE)
├── lib/
│   ├── api.ts                  # API client wrapper (MODIFY — add downloadBackup)
│   ├── components/
│   │   ├── SetupPage.svelte    # First-run setup form (existing)
│   │   └── AdminPanel.svelte   # Health + Backup panel (MODIFY — add backup section)
│   └── utils.ts
└── main.ts
```

#### API Design (per architecture conventions)

| Endpoint | Method | Purpose | Response |
|---|---|---|---|
| `/api/v1/admin/backup` | POST | Create and download database backup | Binary file (application/octet-stream) with Content-Disposition header |

**Note:** This endpoint returns a binary file download, NOT the standard `{"data": ...}` JSON envelope. This is intentional — the response IS the backup file. Error responses still use the standard `{"error": {...}}` JSON format.

**HTTP status codes used:**
- 200: Success (backup file returned)
- 403: Forbidden (instance not initialized)
- 500: Internal error (backup generation failed, pg_dump not found, etc.)

#### Admin Access Control (Pre-Auth)

Same guard as Story 1.6: check `is_initialized()`. No authentication system yet (Epic 2). The same TODO comment applies.

#### Backup Strategy by Backend

**SQLite (`VACUUM INTO`):**
- `VACUUM INTO '/path/to/temp.db'` produces a complete, consistent, standalone database file
- Handles WAL (Write-Ahead Logging) mode correctly — the backup includes uncommitted WAL entries
- Works on both file-based and `:memory:` databases
- Atomic from the perspective of concurrent connections
- No additional dependencies needed
- Restore: copy the backup file as the new database file path, start the server

**PostgreSQL (`pg_dump`):**
- `pg_dump --dbname=<url> --format=plain --file=<path>` produces a SQL dump
- Requires `pg_dump` binary to be installed on the server (standard for PostgreSQL deployments)
- The database URL is passed via `--dbname` flag to `tokio::process::Command` (no shell interpretation — safe from injection)
- If `pg_dump` is not found: return `AppError::Internal("pg_dump not found. Install PostgreSQL client tools to enable backups.")`
- If `pg_dump` exits non-zero: return `AppError::Internal` with the stderr output (redact the URL)
- Restore: `createdb discool_new && psql -d discool_new -f backup.sql`, update config, restart server

**File naming convention:**
- SQLite: `discool-backup-{YYYY-MM-DD-HHmmss}.db`
- PostgreSQL: `discool-backup-{YYYY-MM-DD-HHmmss}.sql`
- UTC timestamp to avoid timezone confusion

#### Optional Save-to-Directory

If `[backup]` section exists in config with `output_dir`:
```toml
[backup]
output_dir = "/var/backups/discool"
```

The backup handler will:
1. Always produce the backup file (temp dir)
2. If `output_dir` is configured: copy to `{output_dir}/discool-backup-{timestamp}.{ext}` (best-effort, log warning on failure)
3. Return the file as HTTP download

This allows operators to set up automated backups via cron (`curl -X POST http://localhost:3000/api/v1/admin/backup -o /dev/null`) while also accumulating copies in the configured directory.

#### CLI Backup (No Separate Binary)

The AC mentions "CLI command." Since no separate CLI binary exists, CLI backup is performed via HTTP:
```bash
# Download backup to a file
curl -X POST http://localhost:3000/api/v1/admin/backup -o discool-backup.db

# Or with wget
wget --post-data='' http://localhost:3000/api/v1/admin/backup -O discool-backup.db
```

This is documented, not implemented as a separate feature.

### Existing Code to Modify

| File | Change |
|---|---|
| `server/src/config/settings.rs` | Add `BackupConfig` struct and `backup` field to `Config` |
| `server/src/handlers/admin.rs` | Add `create_backup()` handler with SQLite/PG backup logic |
| `server/src/handlers/mod.rs` | Add `/admin/backup` route |
| `client/src/lib/api.ts` | Add `downloadBackup()` function |
| `client/src/lib/components/AdminPanel.svelte` | Add backup section with download button |

### Files to Create

None. All changes are additions to existing files.

### Project Structure Notes

- The `/api/v1/admin/backup` route is nested under the existing `/api/v1` router — it is tracked by Prometheus metrics when enabled
- No new Cargo dependencies needed — `VACUUM INTO` is a SQLite SQL statement via sqlx; `pg_dump` is spawned via `tokio::process::Command` (already available in tokio)
- No new npm dependencies needed
- The `BackupConfig` follows the established pattern of optional config sections (`metrics: Option<MetricsConfig>`)
- Temp files use `std::env::temp_dir()` for platform-portable temporary storage

### Testing Requirements

**Server unit tests (in `server/src/handlers/admin.rs` `#[cfg(test)]` module):**
- `POST /api/v1/admin/backup` returns 403 on uninitialized instance (same pattern as health tests)
- `POST /api/v1/admin/backup` returns 200 on initialized instance
- Response has `Content-Type: application/octet-stream`
- Response has `Content-Disposition: attachment; filename="discool-backup-..."` header
- Response body starts with SQLite magic bytes: `53 51 4c 69 74 65 20 66 6f 72 6d 61 74 20 33 00` ("SQLite format 3\0")
- Backup contains actual data: open the returned bytes as a new SQLite connection (using `sqlx::sqlite::SqliteConnectOptions::from_str("sqlite::memory:")` and `ATTACH DATABASE`), verify `instance_settings` table has data

For unit testing, reuse the `test_state()` pattern from the health tests — create an `AppState` with a real `sqlite::memory:` pool (with migrations run) and insert `initialized_at` to simulate initialization.

**Note on testing backup content:** Since `VACUUM INTO` writes to a file path, the unit test needs to:
1. Call the handler
2. Receive the response bytes
3. Write them to a temp file
4. Open that temp file as a SQLite database
5. Verify tables and data exist
6. Clean up the temp file

**Server integration tests (in `server/tests/server_binds_to_configured_port.rs`):**
- `POST /api/v1/admin/backup` before setup returns 403
- `POST /api/v1/admin/backup` after setup returns 200
- Response body is non-empty and starts with SQLite magic bytes

For integration tests, extend the existing test helpers. Use the `http_post` helper (with empty body) and check the response.

**Client tests:**
- Client component tests deferred (same rationale as Story 1.5 and 1.6 — Vitest + @testing-library/svelte not yet set up)

**Linting:** `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test`, `npx biome check .`, `npx svelte-check` — all must pass.

### Anti-Patterns to Avoid

- Do NOT use `unwrap()` or `expect()` in handler code — use `?` with `AppError`
- Do NOT add the `sysinfo` crate or any heavy dependencies for backup
- Do NOT shell-out via `sh -c "pg_dump ..."` — use `Command::new("pg_dump")` with explicit args to avoid injection
- Do NOT expose the database URL in error messages — redact it
- Do NOT read the database URL from the request — always use the configured URL from `state.config`
- Do NOT allow arbitrary file paths from the client — the output directory is config-only
- Do NOT use `any` type in TypeScript — define proper types
- Do NOT call `fetch()` directly in Svelte components — use the `api.ts` wrapper
- Do NOT use `console.log` in client code
- Do NOT add charts or progress bars for the backup — a simple text status ("Creating backup...") is appropriate per UX spec
- Do NOT block the handler indefinitely — set a timeout on the pg_dump subprocess (30 seconds)
- Do NOT leave temp files on disk if the handler errors — ensure cleanup runs on all code paths
- Do NOT use string interpolation for the VACUUM INTO path in a way that allows injection — generate the path internally, never from user input

### Previous Story Intelligence

**From Story 1.6 (Instance Health Dashboard):**
- `admin.rs` already exists with `get_health()` handler, `AdminHealth` struct, CPU/memory/DB collection functions
- `test_state()` helper creates `AppState` with `sqlite::memory:` pool + migrations — reuse directly
- `json_value()` test helper extracts JSON from Response — reuse for error case tests
- Pre-auth guard pattern: `if !super::instance::is_initialized(&state.pool).await? { return Err(AppError::Forbidden(...)) }`
- Admin panel is already integrated into App.svelte sidebar — no changes needed to App.svelte
- `AdminPanel.svelte` has auto-refresh, skeleton loading, error retry patterns — extend with backup section
- `getAdminHealth()` in api.ts shows the pattern for admin API functions
- The admin health tests (unit + integration) show exactly how to test 403/200 for admin endpoints

**From Story 1.5 (First-Run Admin Setup Screen):**
- `is_initialized()` in `instance.rs` — used by the pre-auth guard
- `instance_settings` table with `initialized_at` key — insert this in tests to simulate initialization
- `admin_users` table — backup should contain this data
- JSON response pattern: `(StatusCode::OK, Json(json!({...})))` for normal responses

**From Story 1.4 (Health Check and Metrics Endpoints):**
- Router pattern: routes in the tracked router are automatically metered by Prometheus
- `http_post()` integration test helper — will need to support receiving binary responses

**Review follow-ups from previous stories:**
- No `expect("validated")` — use explicit `Option` handling
- DB URL redaction is wired — maintain pattern for pg_dump error messages
- Admin routes are inside metrics tracking — new `/api/v1/admin/backup` follows suit

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
- Integration test pattern: process-spawning with `TestServer`, `wait_for_bind()`, raw HTTP helpers
- Client: Svelte 5, Vite, Tailwind CSS 4, Biome linter, api.ts wrapper

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Story 1.7] — Full acceptance criteria and story statement
- [Source: _bmad-output/planning-artifacts/epics.md#FR12] — "Operators can export and back up instance data"
- [Source: _bmad-output/planning-artifacts/epics.md#NFR29] — "Backup integrity — exported backups can be fully restored to a new instance"
- [Source: _bmad-output/planning-artifacts/architecture.md#Database] — PostgreSQL 16.x primary, SQLite 3.45+ alternative, sqlx 0.8.6 Any driver
- [Source: _bmad-output/planning-artifacts/architecture.md#Configuration] — TOML config file with env var overrides
- [Source: _bmad-output/planning-artifacts/architecture.md#Format Patterns] — REST API response format
- [Source: _bmad-output/planning-artifacts/architecture.md#Enforcement Guidelines] — Anti-patterns to avoid
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Journey 3] — Admin panel: "Backup/export" listed as admin panel feature
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Loading Patterns] — <200ms no state, 200ms-2s skeleton, >2s skeleton + text
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Button Hierarchy] — Fire CTA for primary actions, destructive styling avoided for non-destructive operations
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Feedback Patterns] — Toast notifications bottom-right, success auto-dismiss 4s
- [Source: _bmad-output/implementation-artifacts/1-6-instance-health-dashboard.md] — admin.rs handler patterns, AdminPanel.svelte structure, test_state() helper, pre-auth guard, API wrapper patterns

## Dev Agent Record

### Agent Model Used

### Debug Log References

### Completion Notes List

### File List

## Change Log

- 2026-02-19: Story created from epics, architecture, UX design, and previous story intelligence.
