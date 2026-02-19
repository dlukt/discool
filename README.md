# Discool

Discool is a self-hosted communication platform: **Rust (Axum) backend** + **Svelte 5 SPA** (embedded into the server binary via `rust-embed`).

## Quickstart (Docker / production)

1. (Optional) Create a config file:
   ```bash
   cp config.example.toml config.toml
   # edit config.toml (at minimum: database.url if you don't want to use env vars)
   ```

2. Start Discool + PostgreSQL (pull from GHCR; or add `--build` to build locally):
   ```bash
   docker compose pull
   docker compose up -d
   ```

3. Open: http://localhost:3000 and complete first-run setup.

### Configuration

- Config load order:
  1. `/etc/discool/config.toml`
  2. `./config.toml`
  3. `$DISCOOL_CONFIG`
  4. Environment variables (highest priority)

In `docker-compose.yml`, you can:
- set env var overrides under `services.discool.environment`
- mount a persistent config file:
  ```yaml
  # services:
  #   discool:
  #     volumes:
  #       - ./config.toml:/etc/discool/config.toml:ro
  ```

**Security note:** The provided Postgres credentials in `docker-compose.yml` are example defaults; change them before exposing the instance publicly.

### Updating

```bash
docker compose pull
docker compose up -d
```

## Development

### Local (recommended)

```bash
cd server && cargo run
cd client && npm run dev
```

### Docker (SQLite dev container)

```bash
docker compose -f docker-compose.dev.yml up -d --build
```

## Quality gates

```bash
cd client && npm run lint && npm run check
cd server && cargo fmt --check && cargo clippy -- -D warnings && cargo test
```
