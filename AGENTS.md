# Repository Guidelines

## Project Structure & Module Organization
- `client/`: Svelte 5 SPA. Entry point is `client/src/main.ts`; shared utilities live in `client/src/lib`; UI components use `client/src/lib/components`.
- `server/`: Rust Axum backend and static file host. HTTP handlers are in `server/src/handlers`, configuration in `server/src/config`, DB code in `server/src/db`, and SQL migrations in `server/migrations`.
- `server/tests/`: integration tests that boot the server and verify HTTP/runtime behavior.
- `config.example.toml`: full config reference and env var mapping (`DISCOOL_*`).
- `_bmad/` and `_bmad-output/`: planning/workflow artifacts; keep product runtime code in `client/` and `server/`.

## Build, Test, and Development Commands
- `cd server && cargo run`: run backend locally (default `0.0.0.0:3000`).
- `cd client && npm run dev`: run frontend dev server; proxies `/api/v1` and `/ws` to backend.
- `cd client && npm run build`: build SPA assets into `client/dist`.
- `cd server && cargo build --release`: build production server (requires `client/dist/index.html`, enforced by `server/build.rs`).
- Quality gate commands:
  - `cd client && npm run lint && npm run check`
  - `cd server && cargo fmt --check && cargo clippy -- -D warnings && cargo test`

## Coding Style & Naming Conventions
- Frontend formatting/linting is handled by Biome (`client/biome.json`): 2-space indentation, single quotes, semicolons as needed.
- Svelte components use `PascalCase` filenames (example: `SetupPage.svelte`); TS variables/functions use `camelCase`.
- Rust follows `rustfmt` defaults; modules/functions in `snake_case`, structs/enums in `PascalCase`.
- Keep API contracts explicit in typed helpers (`client/src/lib/api.ts`) and preserve `{ "data": ... }` response envelopes.

## Testing Guidelines
- Rust tests are the primary automated suite:
  - Integration tests: `server/tests/*.rs`
  - Unit tests: `#[cfg(test)]` modules in `server/src/**`
- Test names should describe behavior (for example, `healthz_returns_200`).
- For frontend changes, run `npm run lint` and `npm run check`, then include manual verification notes in the PR.

## Commit & Pull Request Guidelines
- Prefer Conventional Commit prefixes seen in history (`feat:`, `chore:`), with short imperative summaries.
- Keep commits scoped (client, server, or docs) and include related migration/config updates in the same PR when applicable.
- PRs should include:
  - What changed and why
  - Commands/tests run
  - Screenshots or short recordings for UI updates
  - Linked issue/task and any breaking config/database notes
