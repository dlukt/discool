# Discool client (Svelte 5 SPA)

Discool’s frontend is a **pure SPA** built with **Svelte 5 + Vite + shadcn-svelte + Tailwind CSS v4**.
It is served by the Rust backend in `../server` (no SvelteKit / no SSR).

## Development

Terminal 1 (backend):
```bash
cd server
cargo run
```

Terminal 2 (frontend):
```bash
cd client
npm run dev
```

Vite proxies `/api/v1/*` and `/ws` to `http://localhost:3000` (see `vite.config.ts`).

## Build (production)

```bash
cd client
npm run build

cd ../server
cargo build --release
./target/release/discool-server
```

Open: http://localhost:3000

## Quality gates

```bash
cd client && npm run lint && npm run check
cd server && cargo fmt --check && cargo clippy -- -D warnings && cargo test
```
