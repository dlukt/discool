# Story 1.1: Initialize Project Scaffold and Serve SPA

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As an **operator**,
I want to build and run the Discool server binary that serves a basic Svelte 5 SPA,
So that I have a working foundation to build all features upon.

## Acceptance Criteria

1. **Given** the Rust workspace is initialized with Axum 0.8.x and the Svelte 5 project is initialized with Vite + shadcn-svelte
   **When** the operator builds the project (`cargo build --release` after `npm run build`)
   **Then** a single binary is produced that serves the compiled SPA assets via rust-embed

2. **Given** the server binary is running
   **When** navigating to `http://localhost:3000` in a browser
   **Then** the Svelte 5 SPA loads with the Dual Core theme (shadcn-svelte + fire/ice CSS custom properties)

3. **Given** the Vite dev server is running in development mode
   **When** the frontend makes requests to `/api/v1/*` or `/ws`
   **Then** Vite proxies those requests to the Axum backend (port 3000)

4. **Given** the SPA is being served (dev or production)
   **When** a user navigates to a non-file URL path (e.g., `/guild/channel`)
   **Then** the server falls back to serving `index.html` for client-side routing

5. **Given** the codebase is complete
   **When** running `cargo fmt`, `cargo clippy`, `biome check`, and `tsc -b`
   **Then** all pass cleanly with zero warnings and zero errors

## Tasks / Subtasks

- [x] Task 1: Initialize Rust backend project (AC: #1)
  - [x] 1.1 Run `cargo init server` to create the Rust project in `server/` directory
  - [x] 1.2 Configure `Cargo.toml` with required dependencies (axum 0.8, tokio, tower-http, rust-embed, serde, tracing)
  - [x] 1.3 Create `server/rust-toolchain.toml` to pin stable Rust version
  - [x] 1.4 Create minimal `main.rs` with Axum server bootstrap, graceful shutdown, and port binding to 3000
  - [x] 1.5 Create `server/src/error.rs` with `AppError` enum and `IntoResponse` implementation
  - [x] 1.6 Create `server/src/handlers/mod.rs` with route registration (router builder)
  - [x] 1.7 Create placeholder `GET /api/v1/ping` endpoint returning `{"data": {"status": "ok"}}`

- [x] Task 2: Initialize Svelte 5 frontend project (AC: #1, #2)
  - [x] 2.1 Run `npm create vite@latest client -- --template svelte-ts` in project root
  - [x] 2.2 Run `npx shadcn-svelte@latest init` inside `client/` directory
  - [x] 2.3 Configure Tailwind CSS v4 with the Dual Core fire/ice CSS custom properties in `app.css`
  - [x] 2.4 Set up `biome.json` for formatting and linting
  - [x] 2.5 Create `App.svelte` root component with minimal placeholder showing Discool branding and the Dual Core theme
  - [x] 2.6 Verify `tsc -b` passes with strict TypeScript configuration

- [x] Task 3: Implement SPA serving via rust-embed (AC: #1, #4)
  - [x] 3.1 Add `rust-embed` dependency with `#[derive(RustEmbed)]` pointing to `../client/dist/`
  - [x] 3.2 Create Axum handler that serves embedded static files (JS, CSS, images, etc.)
  - [x] 3.3 Implement fallback handler: any non-file, non-API path returns `index.html` (SPA routing support)
  - [x] 3.4 Set correct `Content-Type` headers based on file extension
  - [x] 3.5 Add `Cache-Control` headers for static assets (hashed filenames get long cache, `index.html` gets no-cache)

- [x] Task 4: Configure Vite dev proxy (AC: #3)
  - [x] 4.1 Configure `vite.config.ts` to proxy `/api/v1/*` requests to `http://localhost:3000`
  - [x] 4.2 Configure `vite.config.ts` to proxy `/ws` WebSocket connections to `http://localhost:3000`
  - [x] 4.3 Verify proxy works: frontend Vite dev server (port 5173) forwards API calls to Axum (port 3000)

- [x] Task 5: Set up Dual Core theme and verify visual output (AC: #2)
  - [x] 5.1 Define fire/ice CSS custom properties in `app.css` per the UX spec color system
  - [x] 5.2 Configure shadcn-svelte theme to use Dual Core tokens (ice blue primary, fire orange actions, zinc neutral, fire red destructive)
  - [x] 5.3 Create a minimal landing page component that demonstrates the theme (heading, ice button, fire button, zinc background)
  - [x] 5.4 Verify dark mode as default theme

- [x] Task 6: Code quality and linting (AC: #5)
  - [x] 6.1 Run `cargo fmt` and fix any formatting issues
  - [x] 6.2 Run `cargo clippy` and resolve all warnings
  - [x] 6.3 Run `biome check` on the frontend codebase and fix any issues
  - [x] 6.4 Run `tsc -b` and resolve all TypeScript errors
  - [x] 6.5 Verify `cargo build --release` succeeds after `npm run build`

## Dev Notes

### Architecture Compliance

**This is the foundational scaffold story. Every subsequent story builds on the patterns established here. Get it right.**

#### Backend Architecture (Rust)

- **Framework:** Axum 0.8.8 (latest stable). Use `axum = { version = "0.8", features = ["ws"] }` — the ws feature will be needed later but include now to avoid rebuild.
- **Runtime:** Tokio with `features = ["full"]`
- **HTTP utilities:** `tower-http = { version = "0.6", features = ["cors", "fs", "trace"] }` for CORS, static file serving, and request tracing.
- **Embedded assets:** `rust-embed = "8"` — embeds `client/dist/` at compile time in release mode. In debug mode, reads from filesystem (live reload friendly).
- **Serialization:** `serde = { version = "1", features = ["derive"] }` + `serde_json = "1"`
- **Logging:** `tracing = "0.1"` + `tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }` — structured logging from day one.

**Error handling pattern (establish now, use forever):**
```rust
pub enum AppError {
    NotFound,
    Internal(String),
    // Future variants: PermissionDenied, ValidationError, etc.
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, code, message) = match self {
            AppError::NotFound => (StatusCode::NOT_FOUND, "NOT_FOUND", "Resource not found"),
            AppError::Internal(msg) => {
                tracing::error!("Internal error: {}", msg);
                (StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL_ERROR", "An internal error occurred")
            }
        };
        let body = json!({"error": {"code": code, "message": message}});
        (status, Json(body)).into_response()
    }
}
```

**REST response format (establish now):**
- Success: `{"data": {...}}`
- Error: `{"error": {"code": "...", "message": "..."}}`
- NEVER bare JSON objects

**Project structure to create:**
```
server/
├── Cargo.toml
├── rust-toolchain.toml
├── src/
│   ├── main.rs           # Entry point, server bootstrap, graceful shutdown
│   ├── lib.rs            # Library root (re-exports)
│   ├── error.rs          # AppError enum
│   └── handlers/
│       └── mod.rs        # Router builder, health/ping endpoint
```

#### Frontend Architecture (Svelte 5)

- **Build tool:** Vite 7.x (latest major — requires Node.js 20.19+ or 22.12+). Architecture doc says 6.x but **Vite 7 is now the latest stable**. Use Vite 7.
- **Framework:** Svelte 5 with TypeScript strict mode, runes enabled by default
- **Design system:** shadcn-svelte (latest, Svelte 5 compatible). Initializes Bits UI, Tailwind CSS v4, and component infrastructure.
- **Linting:** Biome 2.x (has native Svelte file support as of v2.3). Replaces both ESLint and Prettier.
- **Tailwind CSS v4:** Uses CSS-first config via `@theme` directive (NOT the old `tailwind.config.js` approach). Import via `@import "tailwindcss"` in `app.css`.

**Project structure to create:**
```
client/
├── package.json
├── tsconfig.json
├── biome.json
├── vite.config.ts
├── svelte.config.js
├── index.html
├── src/
│   ├── main.ts           # Mount App.svelte
│   ├── App.svelte         # Root component
│   ├── app.css            # Tailwind imports, Dual Core theme CSS custom properties
│   └── lib/
│       └── components/
│           └── ui/        # shadcn-svelte components (auto-generated by init)
```

#### Dual Core Theme CSS (per UX Design Specification)

The CSS custom properties MUST match the UX spec exactly:

```css
:root {
  /* Navigation / Selection (Ice) */
  --primary: 210 100% 60%;
  --primary-foreground: 0 0% 100%;
  --accent: 210 80% 20%;
  --accent-foreground: 210 100% 70%;

  /* Actions / CTAs (Fire) */
  --fire: 20 90% 55%;
  --fire-foreground: 0 0% 100%;
  --destructive: 0 85% 55%;
  --destructive-foreground: 0 0% 100%;

  /* Neutral foundation (Zinc) */
  --background: 240 6% 7%;
  --foreground: 0 0% 90%;
  --muted: 240 4% 16%;
  --muted-foreground: 0 0% 55%;
  --border: 240 4% 20%;
  --ring: 210 100% 60%;

  /* Domain-specific */
  --channel-unread: 210 100% 60%;
  --channel-active: 210 80% 20%;
  --voice-connected: 142 70% 45%;
  --voice-speaking: 210 100% 60%;
}
```

#### SPA Serving Strategy

- **Development:** Vite dev server (port 5173) with HMR. Proxies `/api/v1/*` and `/ws` to Axum on port 3000.
- **Production:** `rust-embed` compiles `client/dist/` into the binary. Axum serves static files with a fallback handler that returns `index.html` for any path that doesn't match a file or API route.
- **Fallback logic:** Request path → check if it's `/api/v1/*` or `/ws` → route to handlers. Otherwise → check if `rust-embed` has a file matching the path → serve file. Otherwise → serve `index.html`.

#### Critical Version Notes (from web research, Feb 2026)

| Dependency | Architecture Doc | Actual Latest | Action |
|---|---|---|---|
| Vite | 6.x | **7.3.1** | Use Vite 7 — requires Node.js 20.19+ |
| Biome | (not versioned) | **2.4.2** | Use Biome 2 — has native Svelte support |
| Tailwind CSS | v4 | **4.1.18** | CSS-first config, NOT `tailwind.config.js` |
| Axum | 0.8.x | **0.8.8** | Consistent |
| rust-embed | 8 | **8.11.0** | Consistent |
| shadcn-svelte | latest | **1.1.0** | Svelte 5 + Tailwind v4 compatible |
| svelte5-router | 2.15.x | **2.16.19** | Close, use `^2.16` |
| TanStack svelte-query | 6.0.x | **6.0.18** | Consistent |
| tokio | 1 | **1.49.0** | Consistent |
| tower-http | 0.6 | **0.6.8** | Consistent |
| sqlx | 0.8.6 | **0.8.6** | Consistent (not needed this story) |

### Project Structure Notes

- This story creates the top-level `discool/` directory structure with `server/` and `client/` subdirectories
- The project root should also contain: `.gitignore`, `config.example.toml` (placeholder for Story 1.2)
- Do NOT create `Dockerfile`, `docker-compose.yml`, or `.github/` yet — those are Story 1.8
- Do NOT create database/migration files yet — those are Story 1.3
- Do NOT create the router (`svelte5-router`) setup yet — that is Story 4.1
- Do NOT install `@tanstack/svelte-query` yet — not needed until REST data fetching stories

### Testing Requirements

- **Backend:** Verify `cargo build --release` produces a working binary after frontend build
- **Backend:** Verify the binary serves the SPA at `http://localhost:3000`
- **Backend:** Verify SPA fallback routing (navigating to `/any/path` returns the SPA)
- **Backend:** Verify `/api/v1/ping` returns `{"data": {"status": "ok"}}`
- **Frontend:** Verify `npm run build` produces output in `client/dist/`
- **Frontend:** Verify `npm run dev` starts Vite dev server with working proxy
- **Linting:** `cargo fmt --check`, `cargo clippy -- -D warnings`, `biome check`, `tsc -b` — all must pass

### Anti-Patterns to Avoid

- Do NOT use `unwrap()` or `expect()` in handler code — use `?` with `AppError`
- Do NOT create a `tailwind.config.js` file — Tailwind CSS v4 uses CSS-first configuration
- Do NOT use SvelteKit — this is a pure SPA, no SSR, no server routes from the frontend
- Do NOT add database dependencies yet — that is Story 1.3
- Do NOT install unnecessary npm packages (keep initial bundle minimal)
- Do NOT use `any` type in TypeScript — use proper types from the start

### References

- [Source: _bmad-output/planning-artifacts/architecture.md#Starter Template Evaluation] — Composed Foundation approach
- [Source: _bmad-output/planning-artifacts/architecture.md#Complete Project Directory Structure] — Full project structure
- [Source: _bmad-output/planning-artifacts/architecture.md#Implementation Patterns & Consistency Rules] — Naming conventions, error handling
- [Source: _bmad-output/planning-artifacts/architecture.md#Development Workflow Integration] — Dev workflow, build, proxy config
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Design Direction Decision] — Dual Core color system
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Color System] — Fire/ice semantic color mapping
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Typography System] — Inter font, system font stack
- [Source: _bmad-output/planning-artifacts/epics.md#Story 1.1] — Acceptance criteria

## Dev Agent Record

### Agent Model Used

GitHub Copilot CLI 0.0.410

### Debug Log References

N/A

### Completion Notes List

1. Initialized Rust Axum backend (`server/`) with structured JSON `tracing`, graceful shutdown, and `GET /api/v1/ping`.
2. Initialized Svelte 5 + Vite SPA (`client/`) with shadcn-svelte + Tailwind CSS v4 and the Dual Core fire/ice theme tokens (dark by default).
3. Implemented static SPA serving via `rust-embed` with correct `Content-Type`, SPA `index.html` fallback, and cache headers for hashed assets.
4. Configured Vite dev proxy for `/api/v1/*` and `/ws` to backend port 3000.
5. Validated: `cargo fmt`, `cargo test`, `cargo clippy -- -D warnings`, `biome check`, `tsc -b`, `npm run build`, `cargo build --release`; smoke-tested `/`, `/guild/channel`, and `/api/v1/ping`.
6. Added baseline security headers (CSP, nosniff, frame-ancestors protection) via Axum middleware.
7. Standardized error responses to include a stable `details` field (`{"error": {..., "details": {}}}`).
8. Reduced coupling between server builds/tests and prebuilt `client/dist` via `server/build.rs` + a safe HTML fallback when dist output is missing.
9. Updated client README to Discool-specific instructions (no SvelteKit / no SSR) and tightened Vite WS proxy settings.
10. Initialized a git repository for change tracking (`git init`).

### File List

- .gitignore
- config.example.toml
- _bmad-output/implementation-artifacts/sprint-status.yaml
- _bmad-output/implementation-artifacts/1-1-initialize-project-scaffold-and-serve-spa.md
- server/.gitignore
- server/Cargo.toml
- server/Cargo.lock
- server/build.rs
- server/rust-toolchain.toml
- server/src/main.rs
- server/src/lib.rs
- server/src/error.rs
- server/src/handlers/mod.rs
- server/src/static_files.rs
- client/.gitignore
- client/.vscode/extensions.json
- client/README.md
- client/biome.json
- client/components.json
- client/index.html
- client/package.json
- client/package-lock.json
- client/public/vite.svg
- client/src/App.svelte
- client/src/app.css
- client/src/assets/svelte.svg
- client/src/lib/utils.ts
- client/src/main.ts
- client/svelte.config.js
- client/tsconfig.json
- client/tsconfig.app.json
- client/tsconfig.node.json
- client/vite.config.ts
 
### Senior Developer Review (AI)

- Review outcome: Changes requested → fixed (security headers, error envelope stability, build resilience, docs)
- Verification: `npm run lint`, `npm run check`, `npm run build`, `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test`, `cargo build --release`

### Change Log
 
- 2026-02-17: Completed Story 1.1 scaffold (backend + frontend), added embedded SPA serving + Vite proxy, and passed all quality gates.
- 2026-02-17: Applied senior review fixes (security headers, error envelope `details`, build.rs + dist fallback, README update, Vite WS proxy hardening, git init).
