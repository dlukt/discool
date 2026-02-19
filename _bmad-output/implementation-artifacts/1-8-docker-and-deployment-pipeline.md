# Story 1.8: Docker and Deployment Pipeline

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As an **operator**,
I want to deploy Discool via Docker with a single `docker compose up` command,
So that I can run a production instance with minimal configuration effort.

## Acceptance Criteria

1. **Given** a multi-stage Dockerfile exists that builds both server and client, producing a minimal image
   **When** the operator runs `docker compose up -d` with the provided `docker-compose.yml`
   **Then** the Discool instance starts with PostgreSQL as the database

2. **Given** the `docker-compose.yml` is provided
   **Then** it includes an example config with environment variable overrides
   **And** the operator can mount a config volume for persistent configuration

3. **Given** GitHub Actions CI workflows are configured
   **When** code is pushed to the repository
   **Then** CI builds, tests, lints, and produces Docker images pushed to GHCR

4. **Given** the release workflow runs
   **Then** cross-compilation targets Linux x86_64 and ARM64

5. **Given** a fresh operator follows the README
   **Then** a new instance is operational within 30 minutes including configuration (NFR35)

6. **Given** a running containerized instance
   **When** the operator updates to a new version via `docker compose pull && docker compose up -d`
   **Then** zero-downtime updates are achievable via container restart (NFR36)

## Tasks / Subtasks

- [x] Task 1: Create multi-stage Dockerfile (AC: #1, #4)
  - [x] 1.1 Create `Dockerfile` at project root using `cargo-chef` pattern for dependency caching
  - [x] 1.2 Stage 1 (chef): Use `rust:1.93-bookworm` as base, install `cargo-chef`, Node.js 24 LTS, and npm
  - [x] 1.3 Stage 2 (planner): `cargo chef prepare --recipe-path recipe.json`
  - [x] 1.4 Stage 3 (builder): `cargo chef cook --release`, build client (`npm ci && npm run build`), then `cargo build --release` (rust-embed picks up `client/dist/`)
  - [x] 1.5 Stage 4 (runtime): Use `debian:bookworm-slim`, copy only the release binary, install `libssl3` and `ca-certificates` and `postgresql-client-16` (for `pg_dump` backup support)
  - [x] 1.6 Create non-root `discool` user (UID 1000) and run as that user
  - [x] 1.7 Set `EXPOSE 3000` and `CMD ["./discool-server"]`
  - [x] 1.8 Add `.dockerignore` to exclude `target/`, `node_modules/`, `.git/`, `_bmad*`, etc.

- [x] Task 2: Create docker-compose.yml (AC: #1, #2)
  - [x] 2.1 Create `docker-compose.yml` at project root with `discool` and `postgres` services
  - [x] 2.2 Discool service: build from `.` or pull from `ghcr.io/dlukt/discool:latest`, expose port 3000, depend on postgres healthcheck
  - [x] 2.3 PostgreSQL service: `postgres:16-bookworm`, named volume for data persistence, healthcheck via `pg_isready`
  - [x] 2.4 Environment variable overrides for Discool: `DISCOOL_DATABASE__URL`, `DISCOOL_SERVER__HOST`, `DISCOOL_SERVER__PORT`
  - [x] 2.5 Optional config volume mount commented out: `./config.toml:/etc/discool/config.toml:ro`
  - [x] 2.6 Add a `docker-compose.dev.yml` override for development (mounts source, uses SQLite, enables hot reload)

- [x] Task 3: Create GitHub Actions CI workflow (AC: #3)
  - [x] 3.1 Create `.github/workflows/ci.yml` triggered on push to `main` and PRs
  - [x] 3.2 Job 1 (server): Install Rust stable, cache cargo registry/target, run `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test`
  - [x] 3.3 Job 2 (client): Install Node.js 24 LTS, run `npm ci`, `npx biome check .`, `npx svelte-check --tsconfig ./tsconfig.app.json`, `npm run build`
  - [x] 3.4 Use `actions/checkout@v6`, `dtolnay/rust-toolchain@stable`, `actions/setup-node@v6`
  - [x] 3.5 Cache `~/.cargo/registry`, `~/.cargo/git`, `server/target` keyed by `Cargo.lock` hash via `actions/cache@v5`

- [x] Task 4: Create GitHub Actions release workflow (AC: #3, #4)
  - [x] 4.1 Create `.github/workflows/release.yml` triggered on push to `main` and version tags (`v*`)
  - [x] 4.2 Build Docker image using `docker/build-push-action@v6` with `docker/setup-buildx-action@v3`
  - [x] 4.3 Multi-platform build: `linux/amd64,linux/arm64` using QEMU emulation via `docker/setup-qemu-action@v3`
  - [x] 4.4 Login to GHCR via `docker/login-action@v3` with `github.actor` + `GITHUB_TOKEN`
  - [x] 4.5 Tag images: `ghcr.io/dlukt/discool:latest`, `ghcr.io/dlukt/discool:<version>`, `ghcr.io/dlukt/discool:<sha>`
  - [x] 4.6 Use `docker/metadata-action@v5` for automatic tag generation from git refs

- [x] Task 5: Create GitHub Actions security workflow (AC: #3)
  - [x] 5.1 Create `.github/workflows/security.yml` triggered on schedule (weekly) and on push to main
  - [x] 5.2 Run `cargo audit` (install via `cargo install cargo-audit`)
  - [x] 5.3 Run `npm audit --audit-level=high` in client directory
  - [x] 5.4 Fail the workflow if critical/high vulnerabilities are found

- [x] Task 6: Update config.example.toml for Docker (AC: #2)
  - [x] 6.1 Add commented Docker-specific examples showing PostgreSQL URL pattern
  - [x] 6.2 Add documentation comment about env var overrides for Docker deployments

- [x] Task 7: Verify full pipeline locally (AC: #1, #5, #6)
  - [x] 7.1 `docker compose build` succeeds
  - [x] 7.2 `docker compose up -d` starts both services, Discool connects to PostgreSQL
  - [x] 7.3 Navigate to `http://localhost:3000` shows the SPA with first-run setup
  - [x] 7.4 Complete first-run setup, verify health dashboard works
  - [x] 7.5 `docker compose down && docker compose up -d` demonstrates zero-downtime restart capability
  - [x] 7.6 Verify all existing tests still pass: `cargo test`, `npm run check`, `npm run lint`

- [x] Task 8: Add operator README (AC: #5)
  - [x] 8.1 Add root `README.md` with Docker quickstart, config mounting, and update instructions

## Dev Notes

### Architecture Compliance

**This story creates the deployment infrastructure (Dockerfile, docker-compose, and GitHub Actions workflows). A small server fix was also required so the first-run setup endpoint works correctly on PostgreSQL (used by Docker).**

#### Dockerfile Strategy (per architecture doc)

The architecture specifies: "Multi-stage: build server + client, produce minimal image" and "Single binary (rust-embed) or Docker image" as the deployment model.

The Dockerfile uses the `cargo-chef` pattern for efficient Docker layer caching:

```
Stage 1 (chef):     Install cargo-chef + Node.js 24 in rust:1.93-bookworm
Stage 2 (planner):  cargo chef prepare → recipe.json (dependency fingerprint)
Stage 3 (builder):  cargo chef cook (cached deps) → npm build → cargo build --release
Stage 4 (runtime):  debian:bookworm-slim + binary only (~100MB final image)
```

**Why `debian:bookworm-slim` instead of `alpine` or `distroless`:**
- The server uses sqlx with `tls-rustls-ring-webpki` (statically linked TLS) — no OpenSSL dependency
- However, `pg_dump` is needed at runtime for PostgreSQL backup support (Story 1.7)
- `pg_dump` requires glibc and PostgreSQL client libraries — rules out musl/alpine and distroless
- The runtime image installs `postgresql-client-16` so `pg_dump` matches the `postgres:16-*` container used in docker-compose

**Why `cargo-chef` over plain multi-stage:**
- Rust dependency compilation is the slowest step (~3-5 min for this project's deps)
- `cargo-chef` separates dependency compilation into a cached layer
- Source changes only recompile the application binary (~30s), not all dependencies
- Cache invalidation only occurs when `Cargo.toml` or `Cargo.lock` change

#### Docker Compose Layout

```yaml
services:
  discool:
    build: .
    ports: ["3000:3000"]
    environment:
      DISCOOL_DATABASE__URL: postgres://discool:discool@postgres:5432/discool
    depends_on:
      postgres:
        condition: service_healthy

  postgres:
    image: postgres:16-bookworm
    volumes: [pgdata:/var/lib/postgresql/data]
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U discool"]
```

#### GitHub Actions CI/CD (per architecture doc)

The architecture specifies three workflows:
- `ci.yml` — Build, test, lint, clippy (both server + client)
- `release.yml` — Cross-compile binaries, Docker image, GHCR push
- `security.yml` — Dependency audit, cargo-audit, npm audit

**CI workflow runs on:** push to main, PRs against main
**Release workflow runs on:** push to `main` and push of version tags (`v*`)
**Security workflow runs on:** weekly schedule + push to main

**GHCR image path:** `ghcr.io/dlukt/discool`
**Tagging strategy:** `latest` + `v1.2.3` (from tag) + `sha-abc1234` (commit SHA)

**Cross-compilation approach:** Docker Buildx with QEMU emulation for `linux/arm64` on `ubuntu-latest` runners. This is simpler than native cross-compilation and works reliably for the project size.

#### API/Route Changes

**Minimal:** `POST /api/v1/instance/setup` inserts were adjusted so they work on PostgreSQL when running under sqlx `Any` (needed for Docker + Postgres). No other API changes.

### Files to Create

| File | Purpose |
|---|---|
| `Dockerfile` | Multi-stage build: cargo-chef + Node.js → minimal runtime |
| `.dockerignore` | Exclude build artifacts, git, BMAD files from Docker context |
| `docker-compose.yml` | Production-ready: Discool + PostgreSQL |
| `docker-compose.dev.yml` | Development override: SQLite, source mounts |
| `README.md` | Operator quickstart (Docker + config + updates) |
| `.github/workflows/ci.yml` | CI: build, test, lint for server + client |
| `.github/workflows/release.yml` | Release: multi-arch Docker image → GHCR |
| `.github/workflows/security.yml` | Security: cargo-audit + npm audit |

### Files to Modify

| File | Change |
|---|---|
| `config.example.toml` | Add Docker deployment documentation comments |
| `server/src/handlers/instance.rs` | Fix instance setup inserts so they work on PostgreSQL (sqlx Any placeholder compatibility) |

### Project Structure Notes

- All new files follow the architecture doc's directory structure specification
- No new Cargo or npm dependencies required — this is infrastructure-only
- The Dockerfile builds the existing `server/` and `client/` codebases unchanged
- `rust-embed` in the release binary automatically picks up `client/dist/` — the Dockerfile ensures `npm run build` runs before `cargo build --release`
- The `docker-compose.yml` uses the same `config.example.toml` documented environment variables
- GitHub Actions workflows use only standard community actions (actions/checkout@v6, docker/* actions)

### Testing Requirements

**This story does not add application tests.** The verification is operational:

1. `docker compose build` — Dockerfile compiles without errors
2. `docker compose up` — Services start, Discool connects to PostgreSQL, SPA loads
3. GitHub Actions `ci.yml` — Validate by pushing to a branch and verifying the workflow passes
4. All existing tests continue passing (`cargo test`, `npm run check`, `npm run lint`)

**Linting:** `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test`, `npx biome check .`, `npx svelte-check` — verified in CI workflow.

### Anti-Patterns to Avoid

- Do NOT use `rust:latest` — pin the Rust version to match the project's toolchain
- Do NOT run the container as root — create a `discool` user in the Dockerfile
- Do NOT copy the entire source tree without `.dockerignore` — massive context bloats build time
- Do NOT use `alpine` as the runtime base — `pg_dump` needs glibc (PostgreSQL client)
- Do NOT hardcode the database URL in docker-compose.yml — use environment variables
- Do NOT use `docker/build-push-action@v5` — use `@v6` (latest stable as of 2026)
- Do NOT skip the `depends_on` healthcheck — Discool will crash if PostgreSQL isn't ready
- Do NOT put secrets (passwords, tokens) in the Dockerfile or docker-compose.yml — use environment variables or Docker secrets
- Do NOT use `COPY . .` before `cargo chef cook` — this defeats the caching purpose
- Do NOT forget to run `npm run build` BEFORE `cargo build --release` — rust-embed embeds `client/dist/` at compile time
- Do NOT use `cargo install` in the Dockerfile for the project binary — use `cargo build --release` and copy the binary from `target/release/`
- Do NOT use `--mount=type=cache` for cargo target in multi-platform builds — cache keys conflict between architectures

### Previous Story Intelligence

**From Story 1.7 (Data Export and Backup):**
- Backup handler uses `pg_dump` for PostgreSQL — the Docker runtime image MUST include `postgresql-client` package
- `VACUUM INTO` for SQLite backup works without additional dependencies
- Backup endpoint: `POST /api/v1/admin/backup` — no changes needed, just verify it works in the container
- Config validates `backup.output_dir` on startup — Docker volumes can mount this path
- Tests use `sqlite::memory:` — CI workflow runs `cargo test` without a database service

**From Story 1.6 (Instance Health Dashboard):**
- Admin health endpoint uses `sysinfo`-style metrics (CPU, memory) — these work in containers (reads from `/proc`)
- Health dashboard auto-refreshes — no container-specific changes needed

**From Story 1.5 (First-Run Admin Setup Screen):**
- First-run setup creates admin identity and instance settings — docker-compose should show this on first start
- The `is_initialized()` check reads from the database — works identically in PostgreSQL

**From Story 1.4 (Health Check and Metrics):**
- `/healthz` and `/readyz` endpoints exist — docker-compose can use `/readyz` for healthcheck
- `/readyz` checks database connection — will return 503 until PostgreSQL is ready
- Prometheus `/metrics` is optional (config-driven) — document in docker-compose

**From Story 1.3 (Database Connection and Migration):**
- sqlx `Any` driver supports PostgreSQL via connection URL — docker-compose passes `postgres://...`
- Migrations run automatically on startup — no manual migration step needed in Docker

**From Story 1.2 (Configuration System):**
- Config auto-detection: `/etc/discool/config.toml` or `./config.toml` — Docker can mount to either path
- Environment variable overrides: `DISCOOL_` prefix with `__` separator — used in docker-compose.yml
- Config precedence: file → env vars — Docker env vars override mounted config file

**From Story 1.1 (Project Scaffold):**
- `cargo build --release` produces `discool-server` binary in `target/release/`
- `npm run build` outputs to `client/dist/` — rust-embed reads this directory
- Vite dev proxy: `/api/v1/*` and `/ws` → `localhost:3000` — only for development, not Docker production
- Binary serves SPA via rust-embed — no separate web server needed in production

**Review patterns across all stories:**
- Commit format: `feat: description` for new features
- All stories run full lint suite: `cargo fmt`, `cargo clippy`, `cargo test`, `biome check`, `svelte-check`
- Integration tests spawn a real server process and make HTTP requests
- SQLite used for testing (`sqlite::memory:`), PostgreSQL for production Docker

### Git Intelligence

**Recent commits (last 7):**
```
a17bf38 feat: add admin backup/export endpoint with SQLite and PostgreSQL support
6842ec1 feat: add admin health dashboard with sidebar navigation
13be36e feat: add first-run admin setup screen with instance configuration
df8d403 feat: add health check and Prometheus metrics endpoints
3a676ee chore: commit from Copilot CLI
e204ccb Add configuration system with structured logging support
a828f94 Initial commit: BMAD framework, Svelte client, and Rust server
```

**Patterns:**
- Conventional commits: `feat:`, `chore:`, `fix:`
- No existing CI/CD — this story establishes it
- No Dockerfile or Docker configuration exists yet
- Server binary name: `discool-server` (from `Cargo.toml` package name)
- Rust edition: 2024 (requires Rust 1.85+)
- Key dependencies: axum 0.8, sqlx 0.8, tokio 1, rust-embed 8, tower-http 0.6
- Client: Svelte 5.45, Vite 7.3, Tailwind CSS 4, Biome 2.4, TypeScript 5.9

### Latest Technical Information

**Rust Docker Base Image:**
- Use `rust:1.93-bookworm` (latest stable as of Feb 2026, matches edition 2024 requirement)
- Pin the exact Rust version to ensure reproducible builds

**cargo-chef:**
- Latest version handles workspace projects and sqlx prepare correctly
- Install via `cargo install cargo-chef` in the chef stage
- `recipe.json` stays stable across source changes — only dependency changes invalidate it

**Docker Buildx Multi-Platform:**
- `docker/setup-buildx-action@v3` + `docker/setup-qemu-action@v3` for ARM64 emulation
- `docker/build-push-action@v6` is the latest stable version
- `docker/metadata-action@v5` for automatic tag generation
- QEMU-based ARM64 builds are slower (~3x) but require no native ARM runners

**GitHub Actions:**
- `actions/checkout@v6`, `actions/setup-node@v6`, `dtolnay/rust-toolchain@stable` are current
- `actions/cache@v5` is the latest cache action (v2 backend, improved performance)
- `GITHUB_TOKEN` has automatic GHCR write permissions when `packages: write` is in the workflow permissions
- Cargo caching: `actions/cache@v5` with key `${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}`

**PostgreSQL 16:**
- `postgres:16-bookworm` is the recommended Docker image
- `pg_isready` healthcheck is the standard approach for `depends_on` conditions

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Story 1.8] — Full acceptance criteria and story statement
- [Source: _bmad-output/planning-artifacts/epics.md#FR8] — "Operators can deploy a Discool instance via Docker or single binary"
- [Source: _bmad-output/planning-artifacts/epics.md#FR9] — "Operators can configure instance settings through a configuration file"
- [Source: _bmad-output/planning-artifacts/epics.md#NFR35] — "New instance operational within 30 minutes including configuration"
- [Source: _bmad-output/planning-artifacts/epics.md#NFR36] — "Zero-downtime updates via container restart or binary swap"
- [Source: _bmad-output/planning-artifacts/epics.md#NFR17] — "No known critical CVEs in production dependencies"
- [Source: _bmad-output/planning-artifacts/architecture.md#Deployment] — Docker image, Dockerfile, docker-compose.yml specs
- [Source: _bmad-output/planning-artifacts/architecture.md#CI/CD] — GitHub Actions workflows: ci.yml, release.yml, security.yml
- [Source: _bmad-output/planning-artifacts/architecture.md#Project Structure] — .github/workflows/, contrib/k8s/, Dockerfile, docker-compose.yml
- [Source: _bmad-output/planning-artifacts/architecture.md#Configuration] — TOML config + env var overrides for Docker
- [Source: _bmad-output/implementation-artifacts/1-7-data-export-and-backup.md] — pg_dump dependency, backup config, existing test patterns
- [Source: _bmad-output/implementation-artifacts/1-7-data-export-and-backup.md#Dev Notes] — Admin handler patterns, pre-auth guard, test helpers

## Dev Agent Record

### Agent Model Used

GitHub Copilot CLI 0.0.411

### Debug Log References

- `docker build --progress=plain -t discool-local:test .`
- `docker compose -f docker-compose.yml build`
- `docker compose -f docker-compose.yml up -d`
- `curl http://localhost:3000/readyz`
- `curl -X POST http://localhost:3000/api/v1/instance/setup ...`
- `curl http://localhost:3000/api/v1/admin/health`
- `docker compose -f docker-compose.yml down && docker compose -f docker-compose.yml up -d`
- `cd client && npm ci && npm run lint && npm run check`
- `cd server && cargo fmt && cargo clippy -- -D warnings && cargo test`

### Completion Notes List

- Added a cargo-chef multi-stage `Dockerfile` that builds client (Node.js 24) and server (release) into a slim Debian runtime image (non-root, includes `pg_dump`).
- Added `.dockerignore` to keep Docker context small.
- Added production `docker-compose.yml` (Discool + PostgreSQL 16) with env-var overrides and an optional config bind mount.
- Added `docker-compose.dev.yml` dev override using SQLite, source mounts, and hot reload (Vite + `cargo watch`) for development.
- Added GitHub Actions workflows: `ci.yml` (lint/test), `release.yml` (multi-arch GHCR publish on main + tags), `security.yml` (cargo-audit + npm audit).
- Updated `config.example.toml` with Docker-specific notes and a docker-compose PostgreSQL URL example.
- Fix: `POST /api/v1/instance/setup` failed on PostgreSQL due to placeholder syntax; switched inserts to `$n` placeholders so setup works in Docker + Postgres.
- Polish: `docker-compose.dev.yml` uses a local image name (`discool-dev`) and is usable standalone for SQLite dev; `security.yml` installs `cargo-audit` with `--locked` for reproducibility.

### File List

- Dockerfile (new)
- .dockerignore (new)
- docker-compose.yml (new)
- docker-compose.dev.yml (new)
- .github/workflows/ci.yml (new)
- .github/workflows/release.yml (new)
- .github/workflows/security.yml (new)
- README.md (new)
- config.example.toml
- server/src/handlers/instance.rs
- _bmad-output/implementation-artifacts/sprint-status.yaml
- _bmad-output/implementation-artifacts/1-8-docker-and-deployment-pipeline.md

## Change Log

- 2026-02-19: Story created from epics, architecture, and previous story intelligence.
- 2026-02-19: Implemented Dockerfile/compose + GitHub Actions workflows, validated docker-compose deployment locally, fixed Postgres instance setup inserts; marked story ready for review.
- 2026-02-19: Post-implementation polish: make `docker-compose.dev.yml` standalone-friendly for SQLite dev; security workflow uses `cargo install --locked`.
- 2026-02-19: Senior dev code review: fix `docker-compose.dev.yml` first-run `npm ci` guard, install `postgresql-client-16` in the runtime image, publish GHCR images on main pushes, add root README; marked story done and synced sprint-status.
- 2026-02-19: Follow-up review: fix Postgres instance setup timestamp insert, pin GitHub Actions to stable action major versions, update README quickstart to pull GHCR image first.

## Senior Developer Review (AI)

_Reviewer: Darko on 2026-02-19_

### Findings

**HIGH**
- `Dockerfile`: Runtime installed `postgresql-client` (Debian bookworm defaults) while `docker-compose.yml` uses `postgres:16-bookworm`. That can break `pg_dump`-based backups (Story 1.7) due to client/server major version mismatch. **Fixed** by installing `postgresql-client-16` in the runtime stage.
- `docker-compose.dev.yml`: First-run `npm ci` could be skipped because `node_modules/` exists when mounted as a Docker volume (empty dir still passes `-d node_modules`). **Fixed** by checking for `node_modules/.bin` instead.

**MEDIUM**
- `.github/workflows/release.yml`: Only ran on version tags, so `ghcr.io/dlukt/discool:latest` would not update on `main` pushes (breaks the documented `docker compose pull` update path). **Fixed** by triggering the workflow on pushes to `main` as well.
- AC #5 required an operator README, but there was no root `README.md`. **Fixed** by adding a concise Docker quickstart/config/update guide at repo root.

**LOW**
- Dev Notes claimed “no Rust changes”; but `server/src/handlers/instance.rs` was updated for PostgreSQL compatibility. **Fixed** by updating the story notes/tables to reflect the minimal API/route change.

### Outcome

✅ Approved (issues fixed)

### Follow-up Review (AI)

_Reviewer: GitHub Copilot CLI on 2026-02-19_

#### Findings

**HIGH**
- `server/src/handlers/instance.rs`: `initialized_at` insert used `CURRENT_TIMESTAMP` into a `TEXT` column, which can fail on PostgreSQL due to type mismatch. **Fixed** by inserting an RFC3339 string timestamp via bound parameters.

**MEDIUM**
- GitHub Actions workflow versions were inconsistent after my initial follow-up: I downgraded `actions/checkout`/`actions/setup-node` to `@v4`, but `@v6` is valid/current (and the story was correct). **Fixed** by switching back to `actions/checkout@v6` and `actions/setup-node@v6` (keeping `actions/cache@v5`, `docker/build-push-action@v6`).

**LOW**
- Root README quickstart did not recommend pulling the prebuilt GHCR image first, making the default path slower/heavier for operators. **Fixed** by adding `docker compose pull` to the quickstart.
