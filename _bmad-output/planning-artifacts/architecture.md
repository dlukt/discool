---
stepsCompleted: [1, 2, 3, 4, 5, 6, 7, 8]
lastStep: 8
status: 'complete'
completedAt: '2026-02-17'
inputDocuments:
  - prd.md
  - product-brief-discool-2026-02-17.md
  - ux-design-specification.md
workflowType: 'architecture'
project_name: 'discool'
user_name: 'Darko'
date: '2026-02-17'
---

# Architecture Decision Document

_This document builds collaboratively through step-by-step discovery. Sections are appended as we work through each architectural decision together._

## Project Context Analysis

### Requirements Overview

**Functional Requirements:**
66 functional requirements across 8 domains. The architectural weight is concentrated in three areas: (1) P2P instance discovery and portable cryptographic identity form the distributed system foundation -- these are "get it right or rewrite" decisions; (2) real-time text and voice communication require dual-protocol handling (WebSocket + WebRTC) with reliability guarantees; (3) the RBAC permission system with hierarchical roles and channel-level overrides is the primary security boundary and must be enforced server-side on every operation.

**Non-Functional Requirements:**
38 NFRs organized into Performance (NFR1-10), Security (NFR11-18), Scalability (NFR19-23), Reliability (NFR24-29), Accessibility (NFR30-34), and Operational (NFR35-38). The most architecturally significant:
- NFR9: 50 concurrent users on 2 vCPU / 2GB RAM -- drives Rust's zero-cost abstractions and memory efficiency
- NFR1/NFR3: <100ms message latency, ≤150ms voice latency -- drives async I/O and WebRTC quality
- NFR11-16: TLS 1.3+, SRTP, client-side key protection, zero XSS, zero privilege escalation -- security is non-negotiable
- NFR20: 100 msg/s sustained throughput -- drives message pipeline and database write performance
- NFR23: 1M messages without query degradation -- drives database choice and indexing strategy

**UX-Driven Architectural Implications:**
- Virtual scrolling for 10k+ messages at 60fps -- requires efficient message pagination API and client-side virtualization
- Instant guild/channel switching -- requires client-side caching strategy and pre-fetching
- <3s SPA initial load on 4G, <500KB gzipped -- requires aggressive code splitting and lazy loading
- shadcn-svelte design system with Dual Core (fire/ice) theming -- CSS custom properties, Tailwind, Bits UI accessibility primitives
- Desktop-first responsive down to mobile single-panel -- same Svelte components, responsive CSS
- Block means complete erasure -- client-side content filtering, not server-side

**Scale & Complexity:**

- Primary domain: Full-stack distributed system (Rust + Svelte 5)
- Complexity level: High
- Estimated architectural components: ~12-15 major subsystems (identity, P2P discovery, WebSocket gateway, WebRTC signaling/TURN, message store, permission engine, guild/channel management, moderation, file storage, SPA frontend, build/deployment pipeline, monitoring/health)

### Technical Constraints & Dependencies

- **Language lock:** Rust backend, Svelte 5 frontend. No Node.js in production.
- **Deployment model:** Single binary or Docker. Must embed or serve SPA assets from the Rust process.
- **Hardware target:** $5 VPS (2 vCPU, 2GB RAM) as baseline deployment.
- **Development model:** Solo developer + LLM-assisted. Architecture must be navigable by AI agents and a single human.
- **No SSR:** SPA-only. Rust serves static HTML with OpenGraph meta tags for link unfurling.
- **Cryptographic standards:** Ed25519/X25519 with algorithm-agile design for future quantum-proof upgrade. DID/VC-compatible identity.
- **WebRTC dependency:** Requires STUN/TURN infrastructure for NAT traversal. Voice quality benchmarked against Mumble.
- **Database:** Not yet decided -- must handle 1M messages without degradation, support efficient pagination, and run within 2GB RAM constraint.
- **No defederation:** Architectural principle, not a feature flag. No instance-level blocking at the protocol level.

### Cross-Cutting Concerns Identified

1. **Identity verification** -- Every API request must verify the caller's cryptographic identity. Cross-instance identity verification adds complexity for federated operations (joining remote guilds, DMs across instances).
2. **Permission enforcement** -- Every write and many reads must check the RBAC permission chain: user → roles → role hierarchy → channel overrides. Must be fast (cached) and correct (no privilege escalation).
3. **Real-time event distribution** -- Messages, presence, typing indicators, voice state changes, moderation actions all flow through a pub/sub system to connected WebSocket clients. Must scale to 50 concurrent users per instance with minimal latency.
4. **P2P protocol** -- Instance discovery, identity exchange, and cross-instance operations (remote guild joins, federated DMs) depend on a DHT/gossip layer with Sybil resistance. Fallback to central directory for MVP.
5. **Rate limiting** -- All API endpoints, WebSocket messages, and voice connections must be rate-limited. Different limits for different operation types.
6. **Input sanitization** -- All user-generated content (messages, guild names, channel names, file uploads, profile fields) must be sanitized against XSS, injection, and payload attacks before storage and before rendering.
7. **Audit trail** -- All moderation actions logged with actor, target, action, reason, timestamp. Append-only, queryable by moderators.
8. **Error handling & resilience** -- Auto-reconnect for WebSocket and WebRTC. Graceful degradation (voice failure doesn't break text). Honest error messages to users. Offline message queuing on reconnect.

## Starter Template Evaluation

### Primary Technology Domain

Full-stack distributed system: **Rust backend (Axum)** + **Svelte 5 SPA frontend (Vite)**, compiled and deployed independently. The Rust binary serves the pre-built SPA bundle as embedded static assets.

### Starter Options Considered

**Backend (Rust):**

| Option | Status | Verdict |
|---|---|---|
| **`cargo init` + Axum from scratch** | Axum 0.8.8 (Jan 2026), actively maintained by Tokio team | **Selected** -- Axum is the community standard for async Rust web servers. Built-in WebSocket support, Tower middleware ecosystem, no macros. No starter template needed; Cargo + crate dependencies provide a clean foundation. |
| Actix Web | v4.12.1 (Nov 2025), mature | Rejected -- 10-15% faster in raw benchmarks but more complex API, actor model adds cognitive overhead, macro-heavy. Axum's simplicity wins for a solo + LLM-assisted project. |
| Loco (Rails-like Rust) | New, convention-over-configuration | Rejected -- designed for CRUD apps, not real-time communication platforms. Adds opinions that conflict with Discool's architecture. |
| Rust10x web-app template | Production blueprint | Considered for project structure patterns (onion architecture) but not adopted as a starter. Too opinionated for a P2P communication platform. |

**Frontend (Svelte 5):**

| Option | Status | Verdict |
|---|---|---|
| **`create vite@latest` with `svelte-ts` template** + shadcn-svelte CLI init | Vite 6.x, Svelte 5.x, shadcn-svelte latest | **Selected** -- minimal Vite + Svelte 5 + TypeScript template. No SvelteKit overhead (no SSR, no server routes -- the Rust backend handles all server concerns). shadcn-svelte CLI adds Bits UI, Tailwind CSS, and component infrastructure. |
| SvelteKit | Official Svelte framework | Rejected -- SvelteKit brings its own server, routing, SSR, and build system. Discool's PRD explicitly requires no Node.js in production. SvelteKit's server-side concerns conflict with the Rust backend architecture. |
| svelte5-shadcn-next-template | Community template | Rejected -- minimal community template with uncertain maintenance. `create vite` + shadcn-svelte CLI is more reliable and better documented. |
| sveltekit-stackter | Production SvelteKit template | Rejected -- SvelteKit-based, same reasons as above. |

### Selected Approach: Composed Foundation

**Rationale:** No single starter template exists for "Rust Axum + Svelte 5 SPA serving embedded assets with WebSocket + WebRTC." The correct approach is two independent, minimal scaffolds composed together:

1. A Rust workspace with Axum as the HTTP/WebSocket server
2. A Svelte 5 SPA built with Vite, initialized with shadcn-svelte

This gives maximum control over both codebases with no framework opinions to fight against.

**Initialization Commands:**

```bash
# Backend: Rust workspace
cargo init discool-server
# Then add dependencies in Cargo.toml:
# axum = { version = "0.8", features = ["ws"] }
# tokio = { version = "1", features = ["full"] }
# tower = "0.5"
# tower-http = { version = "0.6", features = ["cors", "fs", "trace"] }
# serde = { version = "1", features = ["derive"] }
# rust-embed = "8"

# Frontend: Svelte 5 SPA
npm create vite@latest discool-client -- --template svelte-ts
cd discool-client
npx shadcn-svelte@latest init
```

### Architectural Decisions Provided by Starters

**Language & Runtime:**
- Backend: Rust (stable toolchain), async runtime via Tokio
- Frontend: TypeScript (strict mode), Svelte 5 with runes

**Styling Solution:**
- Tailwind CSS v4 (via shadcn-svelte init)
- CSS custom properties for theming (Dual Core fire/ice palette)
- Bits UI primitives for accessible component foundations

**Build Tooling:**
- Backend: `cargo build --release` produces single binary. `rust-embed` embeds SPA assets at compile time.
- Frontend: Vite dev server for development. `vite build` produces static assets for embedding.

**Testing Framework:**
- Backend: Rust's built-in `#[test]` + `cargo test`. Integration tests with `axum::test` utilities.
- Frontend: Vitest (Vite-native) for unit/component tests. Playwright for E2E.

**Code Organization:**
- Backend: Modular Rust project structure (handlers, models, services, middleware, db layers)
- Frontend: Svelte 5 component-based architecture. shadcn-svelte components in `$lib/components/ui/`, custom domain components alongside.

**Development Experience:**
- Backend: `cargo watch` for auto-recompile on changes
- Frontend: Vite HMR for instant hot-reload during development
- Both run independently during development (frontend proxies API calls to backend)

**SPA Serving Strategy:**
- Development: Vite dev server (port 5173) proxies API/WS calls to Axum (port 3000)
- Production: `rust-embed` embeds compiled SPA assets into the Rust binary. Axum serves them with a fallback handler that routes non-file paths to `index.html` for client-side routing.

## Core Architectural Decisions

### Decision Priority Analysis

**Critical Decisions (Block Implementation):**
- Database: PostgreSQL primary, SQLite alternative, sqlx 0.8.x with `Any` driver
- P2P: libp2p 0.56.0 from day one -- Kademlia DHT + Gossipsub
- Authentication: Challenge-response → session token over WebSocket
- API: REST for non-realtime, JSON WebSocket protocol for real-time
- Router: svelte5-router for SPA navigation

**Important Decisions (Shape Architecture):**
- Caching: In-process (moka) default, Redis optional
- Data modeling: Normalized relational
- Data layer: Custom WebSocket store + TanStack Query for REST
- Authorization: Computed permission sets cached per user-guild pair
- Monitoring: tracing + Prometheus metrics

**Deferred Decisions (Post-MVP):**
- Horizontal clustering (Phase 2 -- may require PostgreSQL-only mode)
- E2E encryption key exchange protocol (Phase 2)
- Bot/plugin API design (Phase 2)
- Native mobile app architecture (Phase 3)

### Data Architecture

| Decision | Choice | Version | Rationale |
|---|---|---|---|
| **Primary database** | PostgreSQL | 16.x | Industry standard. Scales to clustering (Phase 2). LISTEN/NOTIFY for real-time. Full-text search. JSON support. |
| **Alternative database** | SQLite | 3.45+ | Zero-config embedded option for simple deployments. Single-file DB. Ideal for small instances. |
| **Database driver** | sqlx | 0.8.6 | Async, compile-time checked queries, supports both PG and SQLite via `Any` driver. Runtime backend selection via config. |
| **Migration tool** | sqlx-cli | 0.8.6 | SQL migrations compatible with both backends. Embedded migration runner for production. |
| **Data modeling** | Normalized relational | -- | Clean FK relationships, JOINs for permission resolution. Well-understood, sqlx handles it well. |
| **Caching (default)** | moka (in-process) | latest | High-performance concurrent cache. TTL expiration, size-based eviction. Zero network overhead. |
| **Caching (optional)** | Redis | 7.x | Operator-selected via config. Shared cache for future clustering. `fred` crate for async Rust client. |

**Database abstraction pattern:** A `DatabaseBackend` trait abstracts PG and SQLite differences. sqlx `Any` driver handles most cases; backend-specific query variants where SQL dialects diverge (e.g., RETURNING clause, UPSERT syntax).

### Authentication & Security

| Decision | Choice | Rationale |
|---|---|---|
| **Identity standard** | Ed25519/X25519, DID/VC-compatible | PRD requirement. Algorithm-agile for future quantum-proof upgrade. |
| **API authentication** | Challenge-response → session token | Client signs server challenge with private key. Server issues short-lived session token. WebSocket connection = session. REST endpoints use same token. |
| **Session management** | Server-side session store (DB-backed) | Token maps to session record. Enables revocation. Short TTL with refresh on active connections. |
| **Authorization** | Computed permission sets, cached per user-guild pair | RBAC chain (user → roles → hierarchy → channel overrides) computed on login and role change. Cached in moka/Redis. Invalidated on role or override mutation. |
| **Key encryption at rest** | Optional passphrase (default: none) | IndexedDB + same-origin policy by default (frictionless for Liam). Optional PBKDF2-derived AES encryption via user settings (for Aisha). |
| **Transport security** | TLS 1.3+ (HTTP/WS), DTLS/SRTP (WebRTC) | PRD requirement. Enforced at reverse proxy or built-in rustls. |

### API & Communication Patterns

| Decision | Choice | Rationale |
|---|---|---|
| **REST API** | Axum handlers, JSON request/response | Non-realtime operations: guild settings, role management, file uploads, moderation actions. Simple, well-understood. |
| **WebSocket protocol** | JSON messages, Discord-style envelope | `{"op": "...", "d": {...}, "s": 42, "t": 1708000000}` -- op (operation), d (data), s (sequence for replay), t (timestamp). Proven at scale. |
| **Error format** | `{"error": {"code": "...", "message": "...", "details": {}}}` | Machine-readable code + human-readable message. Consistent across REST and WebSocket. |
| **P2P protocol** | libp2p 0.56.0 | Kademlia DHT for instance discovery, Gossipsub for inter-instance pub/sub. Built-in NAT traversal. P2P from day one -- core architectural principle. |
| **WebRTC** | webrtc-rs | Async Rust WebRTC implementation. Axum handles signaling over WebSocket. STUN/TURN for NAT traversal. |
| **Rate limiting** | Tower middleware | Per-endpoint, per-user rate limits. Different tiers for different operation types. tower-governor or custom Tower layer. |

### Frontend Architecture

| Decision | Choice | Version | Rationale |
|---|---|---|---|
| **Framework** | Svelte 5 (pure SPA, no SvelteKit) | 5.x | PRD requirement. No Node.js in production. Vite build, static assets. |
| **Router** | @mateothegreat/svelte5-router | 2.15.x | Svelte 5 native. Nested routes, async loading, route hooks/guards, named params, history API. Most feature-rich Svelte 5 SPA router available. |
| **State (UI)** | Svelte 5 runes ($state, $derived, $effect) | -- | Built-in reactive state. No external state library needed. |
| **State (server/REST)** | TanStack Query (svelte-query) | 6.0.x | REST endpoint caching, deduplication, background refresh. Svelte 5 runes-native. |
| **State (real-time)** | Custom WebSocket event store | -- | Svelte 5 $state runes ingesting WebSocket events. Messages, presence, typing indicators, voice state. |
| **Design system** | shadcn-svelte (Bits UI + Tailwind CSS v4) | latest | Accessible primitives, composable, full source ownership. Dual Core fire/ice theme via CSS custom properties. |
| **Testing** | Vitest (unit/component) + Playwright (E2E) | latest | Vite-native test runner. Playwright for cross-browser E2E. |

### Infrastructure & Deployment

| Decision | Choice | Rationale |
|---|---|---|
| **Deployment** | Single binary (rust-embed) or Docker image | PRD requirement. Binary embeds SPA assets. Docker for operators who prefer containers. |
| **Configuration** | TOML config file + env var overrides | Rust ecosystem standard. Auto-detected at `/etc/discool/config.toml` or `./config.toml`. Env vars for Docker. |
| **TLS** | Reverse proxy (nginx/Caddy) recommended, built-in rustls optional | Operators choose. Reverse proxy is standard practice; built-in for simple setups. |
| **CI/CD** | GitHub Actions | Build, test, lint, security scan. Cross-compile Linux x86_64 + ARM64. Docker image to GHCR. |
| **Logging** | tracing crate | Tokio ecosystem standard. Structured logging, configurable verbosity. |
| **Health endpoints** | `/healthz` (liveness) + `/readyz` (readiness) + optional Prometheus `/metrics` | K8s-standard probe endpoints. `/healthz`: process alive, not deadlocked. `/readyz`: DB connected, P2P bootstrapped, ready to serve. Prometheus for operator dashboards. |
| **Kubernetes** | Kustomize manifests in `contrib/k8s/` | Deployment, Service, Ingress, ConfigMap, liveness/readiness probes mapped to `/healthz` and `/readyz`. Community-contributed, not core deployment path. |

### Decision Impact Analysis

**Implementation Sequence:**
1. Rust project scaffold + Axum + sqlx + database schema (foundation)
2. libp2p integration -- instance identity, Kademlia bootstrap, Gossipsub (P2P core)
3. Cryptographic identity system -- keypair generation, DID, challenge-response auth
4. WebSocket gateway -- connection management, session binding, event dispatch
5. Guild/channel/role data model + permission engine
6. Svelte 5 SPA scaffold + router + shadcn-svelte + WebSocket client
7. Text messaging (WebSocket events + message store)
8. Voice (WebRTC signaling + webrtc-rs)
9. Moderation, reporting, mod log
10. File uploads, embeds
11. Deployment pipeline (binary build, Docker, contrib/k8s)

**Cross-Component Dependencies:**
- libp2p ↔ identity system (instance keypairs used for P2P authentication)
- Permission engine ↔ every API handler and WebSocket event (cross-cutting)
- sqlx `Any` driver ↔ all database queries (must test against both PG and SQLite)
- WebSocket gateway ↔ frontend store (shared message protocol contract)
- Caching layer ↔ permission engine + guild data (invalidation on mutations)

## Implementation Patterns & Consistency Rules

### Pattern Categories Defined

**Critical Conflict Points Identified:** 28 areas where AI agents could make different choices, organized into 5 categories. These rules are non-negotiable -- all agents must follow them.

### Naming Patterns

**Database Naming Conventions:**
- Tables: `snake_case`, plural (`guilds`, `channels`, `users`, `guild_members`, `mod_log_entries`)
- Columns: `snake_case` (`guild_id`, `created_at`, `display_name`)
- Foreign keys: `{referenced_table_singular}_id` (`guild_id`, `user_id`, `channel_id`)
- Indexes: `idx_{table}_{columns}` (`idx_users_email`, `idx_messages_channel_id_created_at`)
- Constraints: `{table}_{type}_{columns}` (`users_uq_email`, `guild_members_pk_guild_user`)
- Enums (PG): `snake_case` (`channel_type`, `permission_level`)

**API Naming Conventions:**
- Base path: `/api/v1`
- Resources: `snake_case`, plural (`/api/v1/guilds`, `/api/v1/guilds/:guild_id/channels`)
- Route params: `snake_case` (`:guild_id`, `:channel_id`, `:message_id`)
- Query params: `snake_case` (`?before_id=`, `?limit=`, `?sort_by=`)
- No trailing slashes
- Nested resources max 2 levels deep (`/guilds/:guild_id/channels/:channel_id/messages`)

**Rust Code Naming:**
- Structs/Enums: `PascalCase` (`Guild`, `ChannelMessage`, `PermissionSet`)
- Functions/methods: `snake_case` (`get_guild_by_id`, `verify_identity`)
- Files/modules: `snake_case` (`guild_handler.rs`, `permission_engine.rs`)
- Constants: `SCREAMING_SNAKE_CASE` (`MAX_MESSAGE_LENGTH`, `DEFAULT_RATE_LIMIT`)
- Traits: `PascalCase`, descriptive (`DatabaseBackend`, `CacheProvider`, `EventEmitter`)

**Frontend Code Naming:**
- Components: `PascalCase` files (`GuildRail.svelte`, `MessageBubble.svelte`)
- TypeScript variables/functions: `camelCase` (`guildId`, `getUserData`, `handleMessageSend`)
- TypeScript types/interfaces: `PascalCase` (`Guild`, `ChannelMessage`, `UserPresence`)
- Stores/state files: `camelCase` (`guildStore.ts`, `presenceState.svelte.ts`)
- Utility files: `camelCase` (`formatDate.ts`, `permissionUtils.ts`)
- Constants: `SCREAMING_SNAKE_CASE` (`MAX_MESSAGE_LENGTH`, `WS_RECONNECT_DELAY`)
- CSS: Tailwind utilities only. No custom class naming convention.

**API Boundary (JSON):**
- All JSON fields: `snake_case` (`guild_id`, `display_name`, `created_at`)
- Frontend maps `snake_case` JSON to `camelCase` TypeScript via a thin transform layer in the API client
- Booleans: `true`/`false` (never `1`/`0`)
- Nulls: explicit `null` for absent optional fields, omit field entirely only if never set
- Dates: ISO 8601 strings (`2026-02-17T14:30:00Z`). UTC always.
- IDs: string representation of UUIDs (`"550e8400-e29b-41d4-a716-446655440000"`)

**WebSocket Event Naming:**
- Operations: `snake_case` (`message_create`, `presence_update`, `voice_state_update`, `guild_member_add`)
- Envelope: `{"op": "message_create", "d": {...}, "s": 42, "t": 1708000000}`
- Client-to-server ops prefixed: `c_` (`c_message_send`, `c_typing_start`)
- Server-to-client ops: no prefix (`message_create`, `typing_start`)

### Structure Patterns

**Project Organization:**
```
discool/
├── server/                    # Rust backend (Cargo workspace root)
│   ├── src/
│   │   ├── main.rs
│   │   ├── config/            # TOML config parsing, env overrides
│   │   ├── handlers/          # Axum route handlers (REST + WS)
│   │   ├── models/            # Database models, sqlx types
│   │   ├── services/          # Business logic layer
│   │   ├── middleware/        # Tower middleware (auth, rate limit, logging)
│   │   ├── p2p/               # libp2p integration (discovery, gossip)
│   │   ├── identity/          # Cryptographic identity, DID, verification
│   │   ├── permissions/       # RBAC engine, permission computation
│   │   ├── ws/                # WebSocket gateway, connection manager
│   │   ├── webrtc/            # WebRTC signaling, TURN
│   │   ├── cache/             # Cache abstraction (moka/Redis)
│   │   └── error.rs           # AppError enum, IntoResponse impl
│   ├── migrations/            # sqlx SQL migrations
│   ├── tests/                 # Integration tests
│   └── Cargo.toml
├── client/                    # Svelte 5 frontend
│   ├── src/
│   │   ├── lib/
│   │   │   ├── components/
│   │   │   │   └── ui/        # shadcn-svelte components
│   │   │   ├── features/      # Feature modules
│   │   │   │   ├── guild/     # Components, stores, types for guilds
│   │   │   │   ├── chat/      # Message display, input, virtual scroll
│   │   │   │   ├── voice/     # Voice UI, WebRTC client
│   │   │   │   ├── identity/  # Login, keypair management, recovery
│   │   │   │   └── moderation/# Mod tools, reports, log viewer
│   │   │   ├── stores/        # Global stores (WebSocket, presence, auth)
│   │   │   ├── api/           # REST API client, TanStack Query hooks
│   │   │   ├── ws/            # WebSocket client, event dispatcher
│   │   │   ├── utils/         # Shared utilities
│   │   │   └── types/         # Shared TypeScript types
│   │   ├── routes/            # Route definitions (svelte5-router)
│   │   ├── app.css            # Global styles, Tailwind imports
│   │   ├── App.svelte         # Root component
│   │   └── main.ts            # Entry point
│   ├── tests/
│   │   └── e2e/               # Playwright E2E tests
│   ├── biome.json             # Biome config (formatter + linter)
│   ├── vite.config.ts
│   └── package.json
├── contrib/
│   └── k8s/                   # Kustomize manifests
├── config.example.toml        # Example configuration
└── Dockerfile
```

**Test Organization:**
- Rust unit tests: inline `#[cfg(test)] mod tests` in source files
- Rust integration tests: `server/tests/` directory, one file per domain (`test_guilds.rs`, `test_messages.rs`, `test_permissions.rs`)
- Svelte component tests: co-located (`GuildRail.test.ts` next to `GuildRail.svelte`)
- E2E tests: `client/tests/e2e/` with Playwright

**Feature Module Pattern (Frontend):**
Each feature directory contains:
```
features/guild/
├── GuildRail.svelte          # Components
├── GuildRail.test.ts         # Co-located test
├── GuildSettings.svelte
├── guildStore.svelte.ts      # Feature-specific reactive state
├── guildApi.ts               # TanStack Query hooks for guild REST endpoints
└── types.ts                  # Feature-specific types
```

### Format Patterns

**REST API Response Format:**

Success:
```json
{"data": {"guild_id": "...", "name": "..."}}
```

Success (list):
```json
{"data": [{"guild_id": "...", "name": "..."}], "cursor": "..."}
```

Error:
```json
{"error": {"code": "PERMISSION_DENIED", "message": "You don't have permission to send messages in this channel.", "details": {}}}
```

- All responses wrapped in `{"data": ...}` or `{"error": ...}` -- never bare objects
- Lists use cursor-based pagination (`cursor` field), never offset-based
- HTTP status codes: 200 (success), 201 (created), 204 (no content), 400 (bad request), 401 (unauthorized), 403 (forbidden), 404 (not found), 409 (conflict), 422 (validation error), 429 (rate limited), 500 (internal error)

**WebSocket Event Format:**

Server → Client:
```json
{"op": "message_create", "d": {"channel_id": "...", "content": "...", "author": {...}}, "s": 42, "t": 1708000000}
```

Client → Server:
```json
{"op": "c_message_send", "d": {"channel_id": "...", "content": "..."}}
```

### Communication Patterns

**State Management Pattern (Frontend):**

```
WebSocket events → ws/eventDispatcher.ts → feature stores ($state runes)
                                          → UI components (reactive)
REST responses  → api/client.ts → TanStack Query cache → UI components
```

- Real-time data (messages, presence, typing): WebSocket event store. Svelte 5 `$state` runes. Direct mutation (Svelte runes are mutable by design).
- Server state (guild settings, role configs, user profiles): TanStack Query. Cached, deduped, background-refreshed.
- UI state (modal open, sidebar collapsed, selected channel): Local `$state` in components or feature stores.
- Never mix the three -- a component reads from one source of truth per data type.

**Event Versioning:**
- WebSocket protocol versioned via initial HELLO handshake (`{"op": "hello", "d": {"protocol_version": 1}}`)
- Client declares supported protocol version. Server negotiates or rejects.
- P2P protocol versioned separately via libp2p protocol IDs (`/discool/discovery/1.0.0`, `/discool/gossip/1.0.0`)

### Process Patterns

**Error Handling:**

Rust backend:
- All handlers return `Result<Json<T>, AppError>`
- `AppError` is an enum with variants per error category (`NotFound`, `PermissionDenied`, `ValidationError`, `Internal`, etc.)
- `AppError` implements `IntoResponse` -- maps to correct HTTP status + JSON error format
- Internal errors log full context via `tracing::error!`, return sanitized message to client
- Never expose stack traces, internal paths, or SQL errors to clients

Svelte frontend:
- REST errors: TanStack Query `onError` callbacks. Surface via toast notifications.
- WebSocket errors: event store error handling. Connection errors trigger reconnect. Operation errors surface via toast.
- Toasts follow UX spec: non-intrusive, pausable on hover, auto-dismiss after 5s, `aria-live="polite"`
- Form validation errors: inline, below the field, red text. Validate on blur + on submit.

**Loading State Pattern:**
- Each TanStack Query hook provides `isLoading`, `isError`, `data` states
- WebSocket connection state: `connecting` | `connected` | `reconnecting` | `disconnected`
- Skeleton loaders for initial page load (guild list, channel list, message area)
- Optimistic updates for message sending (show immediately, reconcile on server ack)
- No global loading spinner -- each component handles its own loading state

**Retry/Reconnect Pattern:**
- WebSocket: exponential backoff (1s, 2s, 4s, 8s, max 30s) with jitter. Resume from last sequence number `s`.
- WebRTC: attempt ICE restart first, then full renegotiation, then notify user
- REST: TanStack Query default retry (3 attempts with exponential backoff). No retry on 4xx errors.

### Enforcement Guidelines

**All AI Agents MUST:**
1. Follow naming conventions exactly -- no deviations without explicit approval
2. Place files in the correct directory per the project structure
3. Use `Result<Json<T>, AppError>` for all Rust handlers -- never unwrap in handler code
4. Wrap all REST responses in `{"data": ...}` or `{"error": ...}`
5. Use cursor-based pagination for all list endpoints
6. Co-locate frontend tests with components
7. Use `snake_case` for all JSON fields crossing the API boundary
8. Prefix client-to-server WebSocket ops with `c_`
9. Run `cargo fmt` and `cargo clippy` before considering Rust code complete
10. Run `tsc -b` and `biome check` before considering frontend code complete

**Anti-Patterns (Never Do This):**
- `unwrap()` or `expect()` in handler code (use `?` operator with `AppError`)
- Bare JSON objects in REST responses (always wrap in `data`/`error`)
- `offset`-based pagination (always cursor-based)
- Frontend components directly calling `fetch()` (use TanStack Query or the WebSocket store)
- Storing derived state that can be computed from existing state
- `any` type in TypeScript (use proper types or `unknown` with type guards)
- Console.log in production code (use structured logging / dev-only debug)

## Project Structure & Boundaries

### Complete Project Directory Structure

```
discool/
├── .github/
│   └── workflows/
│       ├── ci.yml                        # Build, test, lint, clippy (both server + client)
│       ├── release.yml                   # Cross-compile binaries, Docker image, GHCR push
│       └── security.yml                  # Dependency audit, cargo-audit, npm audit
│
├── server/                               # Rust backend
│   ├── Cargo.toml                        # Workspace root, dependencies
│   ├── Cargo.lock
│   ├── rust-toolchain.toml               # Pin Rust stable version
│   ├── src/
│   │   ├── main.rs                       # Entry point: config load, server bootstrap, graceful shutdown
│   │   ├── lib.rs                        # Library root, re-exports
│   │   ├── config/
│   │   │   ├── mod.rs                    # Config module root
│   │   │   └── settings.rs              # TOML + env var parsing, typed config structs
│   │   ├── handlers/
│   │   │   ├── mod.rs                    # Route registration, router builder
│   │   │   ├── health.rs                # /healthz, /readyz, /metrics
│   │   │   ├── auth.rs                  # /api/v1/auth/* -- challenge, session, identity
│   │   │   ├── guilds.rs               # /api/v1/guilds/* -- CRUD, settings, invites
│   │   │   ├── channels.rs             # /api/v1/guilds/:guild_id/channels/*
│   │   │   ├── messages.rs             # /api/v1/.../channels/:channel_id/messages
│   │   │   ├── roles.rs                # /api/v1/guilds/:guild_id/roles/*
│   │   │   ├── users.rs                # /api/v1/users/* -- profile, settings, blocks
│   │   │   ├── moderation.rs           # /api/v1/.../moderation/* -- bans, mutes, reports
│   │   │   ├── files.rs                # /api/v1/files/* -- upload, download, metadata
│   │   │   └── ws.rs                   # WebSocket upgrade handler, dispatches to ws/
│   │   ├── models/
│   │   │   ├── mod.rs
│   │   │   ├── user.rs                  # User, UserProfile, UserSettings
│   │   │   ├── guild.rs                # Guild, GuildSettings, GuildInvite
│   │   │   ├── channel.rs              # Channel, ChannelType enum
│   │   │   ├── message.rs              # Message, Attachment, Embed
│   │   │   ├── role.rs                 # Role, RolePermissions
│   │   │   ├── permission.rs           # PermissionSet, PermissionOverride
│   │   │   ├── session.rs              # Session, SessionToken
│   │   │   └── moderation.rs           # ModLogEntry, Ban, Mute, Report
│   │   ├── services/
│   │   │   ├── mod.rs
│   │   │   ├── auth_service.rs          # Challenge-response, session management
│   │   │   ├── guild_service.rs        # Guild lifecycle, membership
│   │   │   ├── channel_service.rs      # Channel CRUD, ordering
│   │   │   ├── message_service.rs      # Message creation, editing, deletion, pagination
│   │   │   ├── role_service.rs         # Role CRUD, hierarchy management
│   │   │   ├── moderation_service.rs   # Ban/mute/kick logic, mod log
│   │   │   ├── file_service.rs         # Upload processing, storage, metadata
│   │   │   └── notification_service.rs # @mention detection, unread tracking
│   │   ├── permissions/
│   │   │   ├── mod.rs
│   │   │   ├── engine.rs               # Permission computation: user → roles → hierarchy → overrides
│   │   │   ├── bitflags.rs             # Permission bitflag definitions
│   │   │   └── cache.rs               # Cached permission set per user-guild pair
│   │   ├── ws/
│   │   │   ├── mod.rs
│   │   │   ├── gateway.rs              # WebSocket connection lifecycle, heartbeat
│   │   │   ├── connection.rs           # Per-connection state, subscriptions
│   │   │   ├── dispatcher.rs           # Route incoming ops to handlers
│   │   │   ├── events.rs              # Server → client event definitions
│   │   │   ├── ops.rs                 # Client → server op definitions
│   │   │   └── presence.rs            # Online/offline/idle tracking, typing indicators
│   │   ├── webrtc/
│   │   │   ├── mod.rs
│   │   │   ├── signaling.rs            # SDP offer/answer/ICE candidate relay
│   │   │   ├── voice_channel.rs       # Voice channel state, participant tracking
│   │   │   └── turn.rs               # TURN server configuration / relay
│   │   ├── p2p/
│   │   │   ├── mod.rs
│   │   │   ├── node.rs                 # libp2p swarm setup, behaviour composition
│   │   │   ├── discovery.rs           # Kademlia DHT bootstrap, instance registration
│   │   │   ├── gossip.rs             # Gossipsub topics, inter-instance event relay
│   │   │   ├── identity.rs           # Instance keypair, peer ID management
│   │   │   └── protocol.rs           # Custom protocol definitions (/discool/*)
│   │   ├── identity/
│   │   │   ├── mod.rs
│   │   │   ├── did.rs                  # DID document creation, verification
│   │   │   ├── keypair.rs            # Ed25519/X25519 key operations
│   │   │   ├── challenge.rs          # Challenge generation, signature verification
│   │   │   └── recovery.rs           # Email-based key recovery flow
│   │   ├── middleware/
│   │   │   ├── mod.rs
│   │   │   ├── auth.rs                # Session token extraction, identity verification
│   │   │   ├── rate_limit.rs         # Per-endpoint, per-user rate limiting
│   │   │   ├── cors.rs              # CORS configuration
│   │   │   └── request_id.rs        # Unique request ID for tracing
│   │   ├── cache/
│   │   │   ├── mod.rs
│   │   │   ├── provider.rs           # CacheProvider trait (moka/Redis abstraction)
│   │   │   ├── moka_backend.rs      # In-process moka cache implementation
│   │   │   └── redis_backend.rs     # Redis cache implementation
│   │   ├── db/
│   │   │   ├── mod.rs
│   │   │   ├── pool.rs              # Connection pool setup (PG or SQLite via Any)
│   │   │   └── migrate.rs          # Embedded migration runner
│   │   └── error.rs                  # AppError enum, IntoResponse, error mapping
│   ├── migrations/
│   │   ├── 001_create_users.sql
│   │   ├── 002_create_guilds.sql
│   │   ├── 003_create_channels.sql
│   │   ├── 004_create_messages.sql
│   │   ├── 005_create_roles.sql
│   │   ├── 006_create_permissions.sql
│   │   ├── 007_create_guild_members.sql
│   │   ├── 008_create_sessions.sql
│   │   ├── 009_create_mod_log.sql
│   │   └── 010_create_files.sql
│   └── tests/
│       ├── common/
│       │   └── mod.rs               # Test helpers, DB setup, fixtures
│       ├── test_auth.rs
│       ├── test_guilds.rs
│       ├── test_channels.rs
│       ├── test_messages.rs
│       ├── test_permissions.rs
│       ├── test_moderation.rs
│       └── test_p2p.rs
│
├── client/                              # Svelte 5 frontend
│   ├── package.json
│   ├── tsconfig.json
│   ├── biome.json                       # Biome formatter + linter config
│   ├── vite.config.ts
│   ├── svelte.config.js
│   ├── index.html                       # SPA entry HTML
│   ├── src/
│   │   ├── main.ts                      # Mount App.svelte, init router
│   │   ├── App.svelte                   # Root: router outlet, global providers
│   │   ├── app.css                      # Tailwind imports, CSS custom properties (fire/ice theme)
│   │   ├── lib/
│   │   │   ├── components/
│   │   │   │   └── ui/                  # shadcn-svelte components (Button, Dialog, etc.)
│   │   │   ├── features/
│   │   │   │   ├── identity/
│   │   │   │   │   ├── LoginView.svelte          # Username entry, keypair generation
│   │   │   │   │   ├── KeyManager.svelte         # Export, import, passphrase settings
│   │   │   │   │   ├── identityStore.svelte.ts   # Auth state, keypair, session token
│   │   │   │   │   ├── identityApi.ts            # REST: challenge, session endpoints
│   │   │   │   │   ├── crypto.ts                 # Ed25519 sign/verify, key encryption
│   │   │   │   │   └── types.ts
│   │   │   │   ├── guild/
│   │   │   │   │   ├── GuildRail.svelte          # Left sidebar guild icons
│   │   │   │   │   ├── GuildRail.test.ts
│   │   │   │   │   ├── GuildHome.svelte          # Guild landing with activity feed
│   │   │   │   │   ├── GuildSettings.svelte      # Settings modal/page
│   │   │   │   │   ├── GuildCreate.svelte        # Create guild flow
│   │   │   │   │   ├── InviteModal.svelte        # Generate/share invite links
│   │   │   │   │   ├── guildStore.svelte.ts      # Guild list, active guild, membership
│   │   │   │   │   ├── guildApi.ts               # REST: guild CRUD, invites
│   │   │   │   │   └── types.ts
│   │   │   │   ├── channel/
│   │   │   │   │   ├── ChannelList.svelte        # Channel sidebar within guild
│   │   │   │   │   ├── ChannelList.test.ts
│   │   │   │   │   ├── ChannelHeader.svelte      # Channel name, topic, actions
│   │   │   │   │   ├── ChannelCreate.svelte      # Create channel modal
│   │   │   │   │   ├── channelStore.svelte.ts    # Channels per guild, active channel
│   │   │   │   │   ├── channelApi.ts
│   │   │   │   │   └── types.ts
│   │   │   │   ├── chat/
│   │   │   │   │   ├── MessageArea.svelte        # Virtual scrolling message list
│   │   │   │   │   ├── MessageArea.test.ts
│   │   │   │   │   ├── MessageBubble.svelte      # Single message display
│   │   │   │   │   ├── MessageInput.svelte       # Compose area with formatting
│   │   │   │   │   ├── TypingIndicator.svelte
│   │   │   │   │   ├── messageStore.svelte.ts    # Messages per channel, pagination
│   │   │   │   │   ├── messageApi.ts             # REST: message history, search
│   │   │   │   │   └── types.ts
│   │   │   │   ├── voice/
│   │   │   │   │   ├── VoiceBar.svelte           # Compact voice status bar
│   │   │   │   │   ├── VoicePanel.svelte         # Full voice controls (mute, deafen, disconnect)
│   │   │   │   │   ├── VoiceParticipant.svelte   # Single participant with volume
│   │   │   │   │   ├── voiceStore.svelte.ts      # Voice connection state, participants
│   │   │   │   │   ├── webrtcClient.ts           # WebRTC peer connection management
│   │   │   │   │   └── types.ts
│   │   │   │   ├── members/
│   │   │   │   │   ├── MemberList.svelte         # Right sidebar member list
│   │   │   │   │   ├── MemberCard.svelte         # User popover card
│   │   │   │   │   ├── RoleManager.svelte        # Role assignment UI
│   │   │   │   │   ├── memberStore.svelte.ts
│   │   │   │   │   └── types.ts
│   │   │   │   ├── moderation/
│   │   │   │   │   ├── ModPanel.svelte           # Moderator tools panel
│   │   │   │   │   ├── ReportModal.svelte        # Report user/message flow
│   │   │   │   │   ├── ModLog.svelte             # Audit log viewer
│   │   │   │   │   ├── moderationApi.ts
│   │   │   │   │   └── types.ts
│   │   │   │   └── settings/
│   │   │   │       ├── UserSettings.svelte       # User preferences, appearance, notifications
│   │   │   │       ├── AppearanceSettings.svelte # Theme toggle, font size, reduced motion
│   │   │   │       └── NotificationSettings.svelte
│   │   │   ├── stores/
│   │   │   │   ├── authStore.svelte.ts           # Global auth state, session token
│   │   │   │   ├── presenceStore.svelte.ts       # Online/offline/idle for all visible users
│   │   │   │   ├── notificationStore.svelte.ts   # Unread counts, @mentions
│   │   │   │   └── toastStore.svelte.ts          # Toast notification queue
│   │   │   ├── api/
│   │   │   │   ├── client.ts                     # Base HTTP client, auth headers, snake→camel transform
│   │   │   │   ├── queryClient.ts                # TanStack Query client config
│   │   │   │   └── types.ts                      # API response wrappers (ApiResponse, ApiError)
│   │   │   ├── ws/
│   │   │   │   ├── client.ts                     # WebSocket connection, reconnect, heartbeat
│   │   │   │   ├── eventDispatcher.ts            # Route incoming ops to feature stores
│   │   │   │   └── types.ts                      # WsMessage envelope, op types
│   │   │   ├── utils/
│   │   │   │   ├── formatDate.ts
│   │   │   │   ├── formatFileSize.ts
│   │   │   │   ├── permissions.ts                # Client-side permission checks for UI gating
│   │   │   │   └── accessibility.ts              # a11y helpers, focus management, skip links
│   │   │   └── types/
│   │   │       ├── index.ts                      # Re-exports
│   │   │       ├── guild.ts                      # Shared guild types
│   │   │       ├── user.ts                       # Shared user types
│   │   │       ├── message.ts                    # Shared message types
│   │   │       └── permissions.ts                # Permission bitflags (mirrors server)
│   │   └── routes/
│   │       └── routes.ts                         # svelte5-router route definitions
│   ├── static/
│   │   └── favicon.svg
│   └── tests/
│       └── e2e/
│           ├── playwright.config.ts
│           ├── auth.spec.ts
│           ├── messaging.spec.ts
│           ├── guild.spec.ts
│           └── voice.spec.ts
│
├── contrib/
│   └── k8s/
│       ├── kustomization.yaml
│       ├── base/
│       │   ├── deployment.yaml           # Discool deployment, resource limits
│       │   ├── service.yaml              # ClusterIP service
│       │   ├── ingress.yaml              # Ingress with TLS
│       │   ├── configmap.yaml            # config.toml as ConfigMap
│       │   └── hpa.yaml                  # Horizontal pod autoscaler (optional)
│       └── overlays/
│           ├── dev/
│           │   └── kustomization.yaml
│           └── production/
│               └── kustomization.yaml
│
├── config.example.toml                   # Example config with all options documented
├── Dockerfile                            # Multi-stage: build server + client, produce minimal image
├── docker-compose.yml                    # Dev: Discool + PostgreSQL + (optional) Redis
├── .gitignore
└── LICENSE
```

### Architectural Boundaries

**API Boundaries:**

```
Client (Browser)
  │
  ├── REST (/api/v1/*) ──────────► handlers/ ──► services/ ──► db/ (sqlx)
  │                                   │                          │
  ├── WebSocket (/ws) ───────────► ws/gateway ──► services/ ──► db/
  │                                   │
  └── WebRTC (peer-to-peer) ◄───► webrtc/signaling (via WS)
                                      │
                                  Instance ◄──► p2p/ ──► Other Instances (libp2p)
```

- **handlers/** is the only layer that touches HTTP/Axum types. Services receive plain Rust types.
- **services/** contains all business logic. Never accesses the database directly -- calls repository functions on models.
- **models/** define sqlx query functions and structs. Only layer that produces SQL.
- **ws/** handles WebSocket frame parsing and dispatches to services. Never contains business logic.
- **p2p/** communicates with other instances. Never accesses the local database directly -- calls services.

**Data Flow:**

```
Client action → REST/WS → handler → middleware (auth, rate limit)
  → service (business logic + permission check via permissions/)
  → model (database query via sqlx)
  → service (format response)
  → handler (serialize to JSON)
  → Client

Incoming P2P event → p2p/gossip → service → model → ws/dispatcher → connected clients
```

**State Boundaries (Frontend):**

| Data type | Source | Store | Invalidation |
|---|---|---|---|
| Messages | WebSocket `message_create` + REST history | `messageStore.svelte.ts` | Append on WS event, paginate via REST |
| Presence | WebSocket `presence_update` | `presenceStore.svelte.ts` | Replace on each event |
| Guild list | REST `/api/v1/guilds` | TanStack Query + `guildStore.svelte.ts` | Refetch on guild join/leave WS event |
| Channel list | REST `/api/v1/guilds/:id/channels` | TanStack Query + `channelStore.svelte.ts` | Refetch on channel CRUD WS event |
| Guild settings | REST `/api/v1/guilds/:id` | TanStack Query | Invalidate on `guild_update` WS event |
| Role config | REST `/api/v1/guilds/:id/roles` | TanStack Query | Invalidate on `role_update` WS event |
| Voice state | WebSocket `voice_state_update` | `voiceStore.svelte.ts` | Replace on each event |
| Auth/session | Identity flow | `authStore.svelte.ts` | On login/logout/token refresh |
| UI state | User actions | Local `$state` per component | Component lifecycle |

### Requirements to Structure Mapping

**FR Category: Identity & Authentication (FR1-7)**
- Server: `identity/`, `handlers/auth.rs`, `services/auth_service.rs`, `models/user.rs`, `models/session.rs`
- Client: `features/identity/`, `stores/authStore.svelte.ts`

**FR Category: Instance Management (FR8-14)**
- Server: `p2p/`, `config/`, `handlers/health.rs`
- Client: N/A (operator-facing, not user-facing)

**FR Category: Guild Management (FR15-23)**
- Server: `handlers/guilds.rs`, `services/guild_service.rs`, `models/guild.rs`
- Client: `features/guild/`

**FR Category: Roles & Permissions (FR24-29)**
- Server: `permissions/`, `handlers/roles.rs`, `services/role_service.rs`, `models/role.rs`, `models/permission.rs`
- Client: `features/members/RoleManager.svelte`, `utils/permissions.ts`, `types/permissions.ts`

**FR Category: Text Communication (FR30-38)**
- Server: `handlers/messages.rs`, `services/message_service.rs`, `models/message.rs`, `ws/events.rs`, `ws/ops.rs`
- Client: `features/chat/`, `ws/`

**FR Category: Voice Communication (FR39-44)**
- Server: `webrtc/`, `ws/` (signaling events)
- Client: `features/voice/`

**FR Category: Moderation & Safety (FR45-55)**
- Server: `handlers/moderation.rs`, `services/moderation_service.rs`, `models/moderation.rs`
- Client: `features/moderation/`

**FR Category: User Experience & Navigation (FR56-62)**
- Client: `routes/routes.ts`, `App.svelte`, `features/settings/`, `stores/notificationStore.svelte.ts`

**FR Category: Data & Privacy (FR63-66)**
- Server: cross-cutting across all services (data export, deletion, audit)

**Cross-Cutting Concerns Mapping:**
- Permission checks: `permissions/` called from every service method
- Rate limiting: `middleware/rate_limit.rs` applied to all handlers
- Input sanitization: validated in handlers before reaching services
- Audit logging: `tracing` instrumentation in services, mod log in `services/moderation_service.rs`
- Caching: `cache/` used by `permissions/cache.rs`, `services/guild_service.rs`, `services/channel_service.rs`

### Development Workflow Integration

**Development:**
```bash
# Terminal 1: Rust backend (auto-recompile)
cd server && cargo watch -x run

# Terminal 2: Svelte frontend (Vite HMR)
cd client && npm run dev

# Terminal 3: PostgreSQL (if not using SQLite)
docker compose up postgres
```

Vite proxies `/api/v1/*` and `/ws` to `localhost:3000` (Axum) during development.

**Build:**
```bash
# Frontend build
cd client && npm run build     # outputs to client/dist/

# Backend build (embeds client/dist/ via rust-embed)
cd server && cargo build --release
```

**Deployment:**
```bash
# Option A: Single binary
./discool-server --config /etc/discool/config.toml

# Option B: Docker
docker run -v /path/to/config.toml:/etc/discool/config.toml ghcr.io/darko/discool:latest

# Option C: Kubernetes
kubectl apply -k contrib/k8s/overlays/production/
```

## Architecture Validation Results

### Coherence Validation

**Decision Compatibility:** All technology choices validated as compatible. Axum 0.8, sqlx 0.8, libp2p 0.56 are all Tokio-native with no runtime conflicts. TanStack Query v6 and svelte5-router are both Svelte 5 runes-native. One version pin required: **webrtc-rs must use v0.17.x** (Tokio-based, production-ready) -- the v0.20 master branch is under active rewrite and not yet stable.

**Pattern Consistency:** Naming conventions are internally consistent across both language worlds. Rust follows Rust idioms, TypeScript follows JS idioms, API boundary uses `snake_case` with a thin client-side transform. No contradictions found.

**Structure Alignment:** Project structure maps cleanly to all architectural decisions. Layer boundaries are clear and enforced: handlers → services → models, with no shortcut paths.

### Requirements Coverage Validation

**Functional Requirements Coverage:** 66/66 FRs covered.

| FR Category | FRs | Architecture Location | Status |
|---|---|---|---|
| Identity & Authentication (FR1-7) | 7 | `identity/`, `handlers/auth.rs`, `features/identity/` | Covered |
| Instance Management (FR8-14) | 7 | `p2p/`, `config/`, `handlers/health.rs` | Covered |
| Guild Management (FR15-23) | 9 | `handlers/guilds.rs`, `services/guild_service.rs`, `features/guild/` | Covered |
| Roles & Permissions (FR24-29) | 6 | `permissions/`, `handlers/roles.rs`, `features/members/` | Covered |
| Text Communication (FR30-38) | 9 | `handlers/messages.rs`, `ws/`, `features/chat/` | Covered |
| Voice Communication (FR39-44) | 6 | `webrtc/`, `ws/`, `features/voice/` | Covered |
| Moderation & Safety (FR45-55) | 11 | `handlers/moderation.rs`, `services/moderation_service.rs`, `features/moderation/` | Covered |
| User Experience (FR56-62) | 7 | `routes/`, `features/settings/`, `stores/` | Covered |
| Data & Privacy (FR63-66) | 4 | Cross-cutting in services | Covered |

**Non-Functional Requirements Coverage:** 38/38 NFRs addressed.
- Performance (NFR1-10): Tokio async, WebSocket direct dispatch, in-process cache, cursor pagination
- Security (NFR11-18): TLS 1.3+, SRTP, Ed25519, input sanitization, rate limiting, AppError (no leak)
- Scalability (NFR19-23): Connection pooling, cache abstraction, indexed queries, PG for scale path
- Reliability (NFR24-29): Exponential backoff reconnect, graceful degradation, sequence-based resume
- Accessibility (NFR30-34): Bits UI primitives, semantic HTML, WCAG 2.1 AA, focus management
- Operational (NFR35-38): `/healthz`, `/readyz`, tracing, Prometheus, TOML config, Docker, Kustomize

### Implementation Readiness Validation

**Decision Completeness:** All critical and important decisions documented with verified versions. No ambiguous technology choices remain.

**Structure Completeness:** Every FR category maps to specific server modules and client feature directories. Cross-cutting concerns have explicit locations. Integration points (handler → service → model, WS → dispatcher → store) are fully specified.

**Pattern Completeness:** 28 conflict points addressed across naming, structure, format, communication, and process patterns. Enforcement rules and anti-patterns documented.

### Gap Analysis Results

**Critical Gaps:** None.

**Important Gaps Resolved:**

| Gap | Resolution |
|---|---|
| webrtc-rs version | Pinned to v0.17.x (Tokio, production). Do not use master/v0.20. |
| File storage backend | Local filesystem for MVP (configurable path in TOML). S3-compatible backend as post-MVP option via `FileStorageProvider` trait (same pattern as cache/database abstraction). |
| Message search | PostgreSQL: `tsvector` full-text search. SQLite: FTS5 extension. Backend-specific implementations behind a shared search interface. |

**Nice-to-Have (deferred):**
- OpenAPI spec generation (`utoipa` crate)
- Database seeding for development
- Contribution guidelines for `contrib/`

### Architecture Completeness Checklist

**Requirements Analysis**
- [x] Project context thoroughly analyzed (66 FRs, 38 NFRs)
- [x] Scale and complexity assessed (High -- distributed real-time system)
- [x] Technical constraints identified (Rust + Svelte 5, no Node.js, 2GB RAM target)
- [x] Cross-cutting concerns mapped (8 concerns across all components)

**Architectural Decisions**
- [x] Critical decisions documented with versions (PG/SQLite, Axum 0.8, libp2p 0.56, sqlx 0.8)
- [x] Technology stack fully specified (all crates and npm packages identified)
- [x] Integration patterns defined (REST, WebSocket, P2P, WebRTC)
- [x] Performance considerations addressed (async I/O, caching, cursor pagination)

**Implementation Patterns**
- [x] Naming conventions established (DB, API, Rust, TS, JSON boundary, WS events)
- [x] Structure patterns defined (feature-based frontend, layered backend)
- [x] Communication patterns specified (WS envelope, state management flow, event versioning)
- [x] Process patterns documented (error handling, loading states, retry/reconnect)

**Project Structure**
- [x] Complete directory structure defined (~120 files/directories)
- [x] Component boundaries established (handlers → services → models)
- [x] Integration points mapped (REST, WS, P2P, WebRTC data flows)
- [x] Requirements to structure mapping complete (all 9 FR categories)

### Architecture Readiness Assessment

**Overall Status:** READY FOR IMPLEMENTATION

**Confidence Level:** High

**Key Strengths:**
- Clean separation between two codebases (Rust/Svelte) with a well-defined JSON API contract
- P2P is a first-class citizen, not an afterthought -- libp2p integrated from day one
- Consistent abstraction pattern (database, cache, file storage) enables operator choice without code duplication
- Layer boundaries prevent spaghetti -- handlers never touch SQL, services never touch HTTP types, p2p never touches the DB
- Permission engine is centralized and cached, preventing the common mistake of scattered auth checks

**Areas for Future Enhancement:**
- Horizontal clustering (Phase 2 -- PG-only mode, shared cache via Redis)
- E2E encryption key exchange (Phase 2)
- Bot/plugin API (Phase 2)
- S3 file storage backend (post-MVP)
- OpenAPI documentation generation
- Native mobile considerations (Phase 3)

### Implementation Handoff

**AI Agent Guidelines:**
- Follow all architectural decisions exactly as documented
- Use implementation patterns consistently across all components
- Respect project structure and boundaries
- Refer to this document for all architectural questions
- When in doubt about a pattern, check the Enforcement Guidelines section

**First Implementation Priority:**
1. Initialize Rust project (`cargo init`) + Svelte project (`create vite` + `shadcn-svelte init`)
2. Set up Axum skeleton with `/healthz`, `/readyz`, and static SPA serving
3. Set up sqlx with PostgreSQL connection and first migration (`001_create_users.sql`)
4. libp2p node bootstrap with Kademlia DHT
