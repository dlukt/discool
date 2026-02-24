---
stepsCompleted: [step-01-validate-prerequisites, step-02-design-epics, step-03-create-stories, step-04-final-validation]
inputDocuments:
  - prd.md
  - architecture.md
  - ux-design-specification.md
---

# discool - Epic Breakdown

## Overview

This document provides the complete epic and story breakdown for discool, decomposing the requirements from the PRD, UX Design, and Architecture requirements into implementable stories.

## Requirements Inventory

### Functional Requirements

FR1: Users can create a portable cryptographic identity (keypair) client-side without requiring a server account
FR2: Users can set a display name and avatar for their identity
FR3: Users can authenticate across multiple Discool instances using a single identity
FR4: Users can optionally associate an email address with their identity for recovery
FR5: Users can recover their identity via email if browser storage is lost
FR6: Users can persist their identity in browser storage and resume sessions without re-authentication
FR7: The system can verify a user's identity cryptographically when they join an instance
FR8: Operators can deploy a Discool instance via Docker or single binary
FR9: Operators can configure instance settings (domain, TLS, defaults) through a configuration file
FR10: Operators can access an admin setup screen on first launch to initialize the instance
FR11: Operators can view instance resource usage and health status
FR12: Operators can export and back up instance data
FR13: Instances can discover other Discool instances via P2P network (or central directory fallback)
FR14: Operators can opt their instance out of P2P discovery (unlisted mode)
FR15: Users can create guilds on an instance
FR16: Guild owners can configure guild settings (name, icon, description)
FR17: Guild owners can create, rename, reorder, and delete text channels within a guild
FR18: Guild owners can create, rename, reorder, and delete voice channels within a guild
FR19: Guild owners can create channel categories to organize channels
FR20: Guild owners can generate invite links for their guild
FR21: Guild owners can generate single-use invite links
FR22: Guild owners can revoke invite links
FR23: Users can join a guild via an invite link
FR24: Guild owners can create, edit, and delete custom roles
FR25: Guild owners can assign permissions to roles (send messages, manage channels, kick members, ban members, manage roles, etc.)
FR26: Guild owners can set role hierarchy to determine permission precedence
FR27: Guild owners can assign roles to guild members
FR28: Guild owners can set channel-level permission overrides for specific roles
FR29: Guild owners can delegate role management to specific roles (e.g., moderators can assign roles)
FR30: Users can send text messages in text channels
FR31: Users can view persistent message history in text channels
FR32: Users can edit their own messages
FR33: Users can delete their own messages
FR34: Users can react to messages with emoji
FR35: Users can upload and share files in channels
FR36: Users can view rich embeds for shared links and images
FR37: Users can send direct messages to other users
FR38: Users can scroll through message history with messages loading progressively
FR39: Users can join and leave voice channels
FR40: Users can see who is currently in a voice channel before joining
FR41: Users can mute and unmute their own microphone
FR42: Users can deafen and undeafen their own audio
FR43: Users can adjust individual user volumes in a voice channel
FR44: The system can automatically reconnect users to voice after a brief connection loss
FR45: Moderators can mute a user in a guild (timed or permanent)
FR46: Moderators can kick a user from a guild
FR47: Moderators can ban a user from a guild (preventing rejoin with the same identity)
FR48: Moderators can kick a user from a voice channel
FR49: Moderators can view a user's message history within the guild
FR50: Moderators can delete any user's messages within the guild
FR51: The system logs all moderation actions in an auditable mod log with timestamps, actions, and moderator identity
FR52: Users can block other users (personal, client-side)
FR53: Users can report messages, files, or users to guild moderators
FR54: Moderators can view and act on a report queue (dismiss, warn, mute, kick, ban)
FR55: Reports are tracked with lifecycle status (pending, reviewed, actioned, dismissed)
FR56: Users can see activity indicators on channels and guilds (active conversations, voice participants, online members)
FR57: Users can view a member list for any guild they belong to
FR58: Users can see online/offline status of guild members
FR59: Users can navigate between multiple guilds they've joined
FR60: Users can access the platform via an invite link without installing an application
FR61: The system provides clear error messages when operations fail (instance unreachable, permission denied, etc.)
FR62: The system automatically reconnects WebSocket connections after brief disconnections
FR63: Users can export their personal data from an instance (GDPR support)
FR64: Users can delete their account from an instance (GDPR support)
FR65: The system sanitizes all user-generated content to prevent XSS and injection attacks
FR66: The system rate-limits API endpoints to prevent abuse

### NonFunctional Requirements

NFR1: WebSocket message delivery latency <100ms same-region, <300ms cross-continent
NFR2: Voice channel join time <2 seconds from click to connected
NFR3: Voice audio latency ≤150ms same-region (Mumble-tier)
NFR4: SPA initial load <3 seconds on 4G connection
NFR5: Time to interactive <2 seconds after initial load
NFR6: SPA bundle size <500KB gzipped (initial chunk)
NFR7: Client memory usage <200MB with 5 guilds active
NFR8: Message history scroll smooth 60fps with 10,000+ messages
NFR9: Server resource usage 50 concurrent users on 2 vCPU / 2GB RAM with headroom
NFR10: Server cold start time <5 seconds from binary launch to accepting connections
NFR11: Transport encryption — all connections use TLS 1.3+ (HTTP, WebSocket) and DTLS (WebRTC)
NFR12: Voice encryption — all voice streams use SRTP (standard WebRTC)
NFR13: Identity key protection — private keys encrypted at rest in browser storage; never transmitted
NFR14: Input sanitization — zero XSS vulnerabilities in UGC rendering
NFR15: Rate limiting — all API endpoints rate-limited; abuse attempts rejected within 1 second
NFR16: Permission enforcement — zero privilege escalation paths; every API call validates permissions server-side
NFR17: Dependency security — no known critical CVEs in production dependencies
NFR18: Security review cadence — automated security review cycle runs at minimum before each release
NFR19: Single-instance capacity — 50 concurrent users with <10% performance degradation vs. 1 user
NFR20: Message throughput — 100 messages/second sustained on reference hardware (2 vCPU/2GB)
NFR21: Voice channel capacity — 15 simultaneous voice users per channel without quality degradation
NFR22: Guild capacity — 50+ guilds per instance without performance impact
NFR23: Data growth — database handles 1 million messages without query degradation
NFR24: Instance uptime 99.5%+ for well-operated instances
NFR25: WebSocket auto-reconnect — automatic reconnection within 5 seconds of connection restoration
NFR26: Voice auto-reconnect — automatic voice reconnection within 5 seconds; no manual rejoin needed
NFR27: Data durability — zero message loss under normal operation; messages persisted before acknowledgment
NFR28: Graceful degradation — if voice fails, text continues working; partial failures don't cascade
NFR29: Backup integrity — exported backups can be fully restored to a new instance
NFR30: WCAG compliance — Level AA conformance per WCAG 2.1
NFR31: Keyboard navigation — all interactive elements reachable and operable via keyboard
NFR32: Screen reader support — all dynamic content announced via ARIA live regions; all controls labelled
NFR33: Color contrast — minimum 4.5:1 ratio for normal text, 3:1 for large text
NFR34: Motion sensitivity — reduced motion option disables all animations and transitions
NFR35: Deployment time — new instance operational within 30 minutes including configuration
NFR36: Update process — zero-downtime updates via container restart or binary swap
NFR37: Monitoring — instance exposes health check endpoint and basic metrics
NFR38: Logging — structured logging with configurable verbosity; no PII in logs by default

### Additional Requirements

**From Architecture:**

- Starter template: Composed foundation — `cargo init` + Axum 0.8.x for backend, `create vite` + `svelte-ts` + `shadcn-svelte init` for frontend
- Database: PostgreSQL 16.x primary, SQLite 3.45+ alternative, sqlx 0.8.6 with `Any` driver for runtime backend selection
- P2P protocol: libp2p 0.56.0 from day one — Kademlia DHT + Gossipsub for instance discovery and inter-instance pub/sub
- Authentication: Challenge-response (Ed25519 signature) → session token; WebSocket connection = session
- API pattern: REST (Axum handlers, JSON) for non-realtime; JSON WebSocket protocol (Discord-style envelope) for real-time
- Frontend router: @mateothegreat/svelte5-router 2.15.x for SPA navigation
- Caching: In-process moka (default), Redis 7.x (optional via config)
- State management: Custom WebSocket event store with Svelte 5 $state runes + TanStack Query (svelte-query) 6.0.x for REST
- Authorization: Computed permission bitflag sets cached per user-guild pair in moka/Redis; invalidated on role/override mutation
- Monitoring: tracing crate for structured logging + Prometheus /metrics endpoint
- WebRTC: webrtc-rs v0.17.x pinned (do NOT use v0.20 master — under active rewrite, not stable)
- File storage: Local filesystem for MVP (configurable path in TOML); S3-compatible backend post-MVP via FileStorageProvider trait
- Message search: PostgreSQL tsvector full-text search / SQLite FTS5
- Configuration: TOML config file (auto-detected at /etc/discool/config.toml or ./config.toml) + env var overrides for Docker
- CI/CD: GitHub Actions — build, test, lint, security scan; cross-compile Linux x86_64 + ARM64; Docker image to GHCR
- Project structure: Layered backend (handlers → services → models), feature-based frontend (features/ directories with co-located stores, API, types, tests)
- Implementation sequence: scaffold → P2P → identity → WebSocket gateway → guild/channel/role → SPA scaffold → text messaging → voice → moderation → files → deployment pipeline
- Health endpoints: /healthz (liveness), /readyz (readiness), optional Prometheus /metrics
- Error handling: AppError enum with IntoResponse; sanitized messages to clients; never expose internals
- Testing: Rust #[test] + cargo test, Vitest for frontend unit/component, Playwright for E2E
- WebSocket protocol: {"op": "...", "d": {...}, "s": 42, "t": 1708000000} — client ops prefixed c_

**From UX Design:**

- Design system: shadcn-svelte with Dual Core (fire/ice) theme via CSS custom properties
- Color system: Ice blue (--primary) for navigation/selection, fire orange (--fire) for actions/CTAs, fire red (--destructive) for destructive actions, zinc for neutral
- Typography: Inter, system font stack; monospace stack for code blocks
- Layout: Four-panel desktop — guild rail 72px, channel list 240px, message area flex-1, member list 240px
- Responsive strategy: Desktop-first; tablet (768-1023px) collapses member list to toggle; mobile (<768px) single-panel drill-down with bottom navigation bar
- Block behavior: Complete erasure — no placeholders, no hints, no trace of blocked users
- Keyboard conventions: Standard OS conventions sacred (Home, End, Ctrl+A, text selection)
- Loading patterns: No spinner without text; skeleton placeholders; <200ms = no loading state; 200ms-2s = skeleton; >2s = skeleton + text
- Toast system: Bottom-right, max 3 visible, auto-dismiss 4s (info/success), 6s (warning), persistent (error); pausable on hover/focus
- Empty states: Friendly, action-suggesting, never error-like; guided next actions
- Context menus: Primary action discovery for messages, users, channels; right-click/long-press
- Quick switcher: Ctrl+K, fuzzy search across guilds/channels/DMs
- Voice bar: Persistent compact controls when in voice; keyboard shortcuts M (mute), D (deafen), Ctrl+D (disconnect)
- Form patterns: Validate on blur, single-column, labels above inputs, fire CTA bottom-right
- Accessibility: axe-core in CI, ARIA live regions for messages/voice state, focus management (trap in modals, return on close, input on channel nav), skip-to-content link, all touch targets min 48px, landmark regions
- Onboarding: Invite link → username picker → in guild; <30s new identity, <10s existing; keypair generation invisible
- Component strategy: shadcn-svelte for generic UI; custom components for domain-specific (GuildRail, ChannelListItem, MessageBubble, MessageInput, VoiceBar, VoiceParticipant, MemberListEntry, ModLogEntry, ReportQueueItem, InviteLinkCard)
- Button hierarchy: Fire (primary CTA), ice (navigation/selection), zinc/ghost (secondary), fire red (destructive); max ONE fire button per context
- Mobile voice: Bottom sheet with 64px mute/deafen/disconnect buttons, participant list, connection quality

### FR Coverage Map

| FR | Epic | Description |
|---|---|---|
| FR1 | Epic 2 | Client-side keypair creation |
| FR2 | Epic 2 | Display name and avatar |
| FR3 | Epic 2 | Cross-instance authentication |
| FR4 | Epic 2 | Optional email association |
| FR5 | Epic 2 | Email-based identity recovery |
| FR6 | Epic 2 | Browser storage persistence |
| FR7 | Epic 2 | Cryptographic identity verification |
| FR8 | Epic 1 | Docker/binary deployment |
| FR9 | Epic 1 | Configuration file |
| FR10 | Epic 1 | First-run admin setup |
| FR11 | Epic 1 | Resource usage and health |
| FR12 | Epic 1 | Data export and backup |
| FR13 | Epic 3 | P2P instance discovery |
| FR14 | Epic 3 | Discovery opt-out |
| FR15 | Epic 4 | Guild creation |
| FR16 | Epic 4 | Guild settings |
| FR17 | Epic 4 | Text channel CRUD |
| FR18 | Epic 4 | Voice channel CRUD |
| FR19 | Epic 4 | Channel categories |
| FR20 | Epic 4 | Invite link generation |
| FR21 | Epic 4 | Single-use invite links |
| FR22 | Epic 4 | Invite link revocation |
| FR23 | Epic 4 | Join via invite link |
| FR24 | Epic 5 | Role CRUD |
| FR25 | Epic 5 | Role permissions |
| FR26 | Epic 5 | Role hierarchy |
| FR27 | Epic 5 | Role assignment |
| FR28 | Epic 5 | Channel permission overrides |
| FR29 | Epic 5 | Role management delegation |
| FR30 | Epic 6 | Send text messages |
| FR31 | Epic 6 | View message history |
| FR32 | Epic 6 | Edit own messages |
| FR33 | Epic 6 | Delete own messages |
| FR34 | Epic 6 | Emoji reactions |
| FR35 | Epic 6 | File upload and sharing |
| FR36 | Epic 6 | Rich embeds |
| FR37 | Epic 6 | Direct messages |
| FR38 | Epic 6 | Progressive message history loading |
| FR39 | Epic 7 | Join/leave voice channels |
| FR40 | Epic 7 | See voice participants |
| FR41 | Epic 7 | Mute/unmute microphone |
| FR42 | Epic 7 | Deafen/undeafen audio |
| FR43 | Epic 7 | Individual volume control |
| FR44 | Epic 7 | Voice auto-reconnect |
| FR45 | Epic 8 | Mute user in guild |
| FR46 | Epic 8 | Kick user from guild |
| FR47 | Epic 8 | Ban user from guild |
| FR48 | Epic 8 | Kick user from voice |
| FR49 | Epic 8 | View user message history (mod) |
| FR50 | Epic 8 | Delete any message (mod) |
| FR51 | Epic 8 | Mod log |
| FR52 | Epic 6 | User blocking (client-side) |
| FR53 | Epic 8 | Report messages/files/users |
| FR54 | Epic 8 | Report queue |
| FR55 | Epic 8 | Report lifecycle tracking |
| FR56 | Epic 6 | Activity indicators |
| FR57 | Epic 5 | Member list |
| FR58 | Epic 5 | Online/offline status |
| FR59 | Epic 6 | Multi-guild navigation |
| FR60 | Epic 6 | No-install web access |
| FR61 | Epic 6 | Clear error messages |
| FR62 | Epic 6 | WebSocket auto-reconnect |
| FR63 | Epic 8 | Data export (GDPR) |
| FR64 | Epic 8 | Account deletion (GDPR) |
| FR65 | Epic 6 | Input sanitization |
| FR66 | Epic 6 | Rate limiting |

## Epic List

### Epic 1: Project Foundation & Instance Deployment
Operators (Tomas) can deploy a Discool instance via Docker or single binary, complete first-run admin setup, configure instance settings, view health/resource status, and export/back up data. The composed starter template (Rust Axum + Svelte 5 + shadcn-svelte) is initialized, database (PG/SQLite via sqlx) is set up, and the SPA serving pipeline is operational.
**FRs covered:** FR8, FR9, FR10, FR11, FR12
**NFRs addressed:** NFR9, NFR10, NFR17, NFR24, NFR35, NFR36, NFR37, NFR38

### Epic 2: Identity & Authentication
Users (Liam) can create a portable cryptographic identity by picking a username (keypair generated invisibly), authenticate to any instance via challenge-response, persist their session in browser storage, optionally associate an email for recovery, and recover their identity if browser storage is lost. The identity is DID/VC-compatible and works across all instances.
**FRs covered:** FR1, FR2, FR3, FR4, FR5, FR6, FR7
**NFRs addressed:** NFR11, NFR13

### Epic 3: P2P Discovery & Federation Foundation
Instances automatically discover each other via libp2p Kademlia DHT and Gossipsub. Operators can opt their instance out of discovery (unlisted mode). Cross-instance identity verification is operational. The P2P network has Sybil resistance and no single point of failure.
**FRs covered:** FR13, FR14
**NFRs addressed:** NFR24

### Epic 4: Guilds, Channels & Invites
Maya can create guilds, configure settings (name, icon, description), create/rename/reorder/delete text and voice channels, organize them into categories, and generate invite links (standard and single-use). Liam can click an invite link and join a guild, landing in the default channel. The SPA navigation shell (GuildRail, ChannelList, four-panel layout) is built.
**FRs covered:** FR15, FR16, FR17, FR18, FR19, FR20, FR21, FR22, FR23
**NFRs addressed:** NFR4, NFR5, NFR6, NFR22, NFR30, NFR31, NFR33, NFR34

### Epic 5: Roles, Permissions & Member Management
Maya can create roles with specific permissions, establish role hierarchy, assign roles to members, set channel-level permission overrides, and delegate role management. Members see a member list grouped by role with online/offline presence. The permission engine enforces RBAC on every server-side operation.
**FRs covered:** FR24, FR25, FR26, FR27, FR28, FR29, FR57, FR58
**NFRs addressed:** NFR16

### Epic 6: Real-Time Text Communication
Users can send, edit, delete messages in text channels, react with emoji, upload files, view rich embeds, and send direct messages. Message history is persistent with virtual scrolling at 60fps. Activity indicators show where conversations are happening. Users can block others (complete erasure). The WebSocket gateway handles real-time events with auto-reconnect. Input sanitization and rate limiting protect against abuse.
**FRs covered:** FR30, FR31, FR32, FR33, FR34, FR35, FR36, FR37, FR38, FR52, FR56, FR59, FR60, FR61, FR62, FR65, FR66
**NFRs addressed:** NFR1, NFR4, NFR5, NFR6, NFR7, NFR8, NFR14, NFR15, NFR19, NFR20, NFR23, NFR25, NFR27

### Epic 7: Voice Communication
Users can drop into voice channels with one click, see who's in a channel before joining, control mute/deafen/volume, and auto-reconnect after connection loss. Voice quality targets Mumble-tier latency. The VoiceBar provides persistent compact controls. WebRTC handles peer connections with STUN/TURN for NAT traversal.
**FRs covered:** FR39, FR40, FR41, FR42, FR43, FR44
**NFRs addressed:** NFR2, NFR3, NFR12, NFR21, NFR26, NFR28

### Epic 8: Moderation, Reporting & Data Privacy
Rico can mute (timed/permanent), kick, and ban users, kick from voice, delete messages, and view user message history — all via context menus. All actions are logged in an append-only mod log. Users can report content, and moderators can review/act on a report queue with lifecycle tracking. Users can export their data and delete their accounts (GDPR compliance).
**FRs covered:** FR45, FR46, FR47, FR48, FR49, FR50, FR51, FR53, FR54, FR55, FR63, FR64
**NFRs addressed:** NFR18, NFR29

## Epic 1: Project Foundation & Instance Deployment

Operators (Tomas) can deploy a Discool instance via Docker or single binary, complete first-run admin setup, configure instance settings, view health/resource status, and export/back up data.

### Story 1.1: Initialize Project Scaffold and Serve SPA

As an **operator**,
I want to build and run the Discool server binary that serves a basic Svelte 5 SPA,
So that I have a working foundation to build all features upon.

**Acceptance Criteria:**

**Given** the Rust workspace is initialized with Axum 0.8.x and the Svelte 5 project is initialized with Vite + shadcn-svelte
**When** the operator builds the project (`cargo build --release` after `npm run build`)
**Then** a single binary is produced that serves the compiled SPA assets via rust-embed
**And** navigating to `http://localhost:3000` in a browser shows the Svelte 5 SPA with the Dual Core theme (shadcn-svelte + fire/ice CSS custom properties)
**And** the Vite dev server proxies `/api/v1/*` and `/ws` to the Axum backend during development
**And** non-file URL paths fall back to `index.html` for client-side routing
**And** `cargo fmt`, `cargo clippy`, `biome check`, and `tsc -b` all pass cleanly

### Story 1.2: Configuration System and Structured Logging

As an **operator**,
I want to configure my instance via a TOML config file with environment variable overrides,
So that I can customize settings for my deployment without modifying code.

**Acceptance Criteria:**

**Given** a `config.example.toml` exists with all documented options
**When** the operator places a config file at `./config.toml` or `/etc/discool/config.toml`
**Then** the server loads configuration from the file at startup
**And** environment variables override TOML values (e.g., `DISCOOL_PORT=8080`)
**And** missing required config values cause the server to exit with a clear error message
**And** the tracing crate provides structured JSON logging with configurable verbosity (debug/info/warn/error) via config
**And** no PII appears in logs by default
**And** the server logs its configuration summary (redacting secrets) on startup

### Story 1.3: Database Connection and Migration System

As an **operator**,
I want the server to connect to PostgreSQL or SQLite based on my configuration and run migrations automatically,
So that the database is always in the correct state when the instance starts.

**Acceptance Criteria:**

**Given** the operator has configured a database URL in the config (PostgreSQL connection string or SQLite file path)
**When** the server starts
**Then** it connects to the configured database using sqlx `Any` driver
**And** embedded migrations run automatically on startup, bringing the schema to the latest version
**And** if the database is unreachable, the server logs a clear error and exits with a non-zero status
**And** the `DatabaseBackend` trait abstracts PG/SQLite differences for backend-specific query variants
**And** connection pooling is configured with sensible defaults appropriate for the 2GB RAM target

### Story 1.4: Health Check and Metrics Endpoints

As an **operator**,
I want health check endpoints so I can monitor my instance and integrate with container orchestrators,
So that I know when my instance is healthy and ready to serve traffic.

**Acceptance Criteria:**

**Given** the server is running
**When** a request is made to `GET /healthz`
**Then** it returns 200 if the process is alive and not deadlocked
**And** `GET /readyz` returns 200 only when the database connection is established and migrations are complete
**And** `GET /readyz` returns 503 with a JSON body describing what's not ready if any check fails
**And** `GET /metrics` (optional, enabled via config) exposes Prometheus-format metrics
**And** server cold start time from binary launch to `/readyz` returning 200 is under 5 seconds (NFR10)

### Story 1.5: First-Run Admin Setup Screen

As an **operator**,
I want a first-run setup screen in the SPA when I initialize a new instance,
So that I can create the admin identity and configure basic instance settings without a CLI.

**Acceptance Criteria:**

**Given** the instance has a fresh database with no admin identity
**When** the operator opens the instance URL in a browser
**Then** a first-run setup screen is displayed (not the normal guild view)
**And** the operator can enter a username and optionally pick an avatar to create the admin identity
**And** the operator can set instance name and description
**And** the operator can toggle P2P discovery opt-in/opt-out (default: opt-in)
**And** after completing setup, the admin identity is persisted and the instance is marked as initialized
**And** subsequent visits load the normal SPA (not the setup screen)
**And** the setup screen follows the Dual Core design system (fire CTA button, single-column form, labels above inputs)

### Story 1.6: Instance Health Dashboard

As an **operator**,
I want to view resource usage and health status from within the SPA admin panel,
So that I can monitor my instance without SSH access.

**Acceptance Criteria:**

**Given** the operator is authenticated as the instance admin
**When** they navigate to the admin panel section in the SPA sidebar
**Then** they can see current CPU and memory usage of the server process
**And** they can see the number of active WebSocket connections
**And** they can see database size and connection pool status
**And** they can see instance uptime
**And** the health data refreshes periodically (every 30 seconds)
**And** the admin panel is a section within the SPA sidebar, not a separate interface

### Story 1.7: Data Export and Backup

As an **operator**,
I want to export and back up my instance data,
So that I can restore my instance or migrate to new hardware without data loss.

**Acceptance Criteria:**

**Given** the operator is authenticated as the instance admin
**When** they trigger a backup via the admin panel or CLI command
**Then** a complete database export is produced (SQL dump for PG, file copy for SQLite)
**And** the export includes all guilds, channels, messages, users, roles, and configuration
**And** the export can be downloaded via the admin panel or saved to a configured path
**And** a restored backup on a fresh instance produces a fully functional instance with all data intact (NFR29)
**And** backup progress is shown in the admin panel if the operation takes more than 2 seconds

### Story 1.8: Docker and Deployment Pipeline

As an **operator**,
I want to deploy Discool via Docker with a single `docker compose up` command,
So that I can run a production instance with minimal configuration effort.

**Acceptance Criteria:**

**Given** a multi-stage Dockerfile exists that builds both server and client, producing a minimal image
**When** the operator runs `docker compose up -d` with the provided `docker-compose.yml`
**Then** the Discool instance starts with PostgreSQL as the database
**And** the `docker-compose.yml` includes an example config with environment variable overrides
**And** the operator can mount a config volume for persistent configuration
**And** GitHub Actions CI builds, tests, lints, and produces Docker images pushed to GHCR
**And** cross-compilation targets Linux x86_64 and ARM64
**And** a new instance is operational within 30 minutes including configuration (NFR35)
**And** zero-downtime updates are achievable via container restart (NFR36)

## Epic 2: Identity & Authentication

Users (Liam) can create a portable cryptographic identity by picking a username (keypair generated invisibly), authenticate to any instance via challenge-response, persist their session in browser storage, optionally associate an email for recovery, and recover their identity if browser storage is lost.

### Story 2.1: Client-Side Keypair Generation and Identity Creation

As a **new user**,
I want to pick a username and have my identity created instantly,
So that I can join a guild without a complex registration process.

**Acceptance Criteria:**

**Given** a user opens the Discool SPA without an existing identity in browser storage
**When** they enter a username and optionally select an avatar
**Then** an Ed25519 keypair is generated client-side in the browser
**And** a DID document is created from the public key
**And** the private key is stored in IndexedDB, protected by same-origin policy
**And** the public key, username, and avatar are sent to the server to register the identity
**And** the user table and identity records are created in the database (migrations for this story)
**And** the entire process completes in under 3 seconds
**And** no cryptographic concepts are visible in the UI — the user sees only "Pick a username"

### Story 2.2: Challenge-Response Authentication and Session Management

As a **user**,
I want to authenticate to an instance using my cryptographic identity,
So that the server knows I am who I claim to be without a password.

**Acceptance Criteria:**

**Given** a user has an identity stored in their browser
**When** they connect to a Discool instance
**Then** the server issues a random challenge
**And** the client signs the challenge with the user's private key
**And** the server verifies the signature against the user's registered public key
**And** upon successful verification, the server creates a session and returns a session token
**And** the session is stored server-side in the database (sessions table migration in this story)
**And** the session token is used for all subsequent REST API calls (Authorization header) and the WebSocket connection
**And** sessions have a configurable TTL with refresh on active connections
**And** all authentication happens over TLS 1.3+ (NFR11)

### Story 2.3: Identity Persistence and Auto-Login

As a **returning user**,
I want my identity to persist in the browser so I'm automatically logged in,
So that I don't have to re-authenticate every time I open Discool.

**Acceptance Criteria:**

**Given** a user has previously created an identity on this browser
**When** they navigate to the Discool instance URL or open a bookmark
**Then** the SPA detects the identity in IndexedDB/localStorage
**And** authentication happens automatically (challenge-response in the background)
**And** the user lands on their last active guild and channel
**And** unread indicators show what they missed
**And** no login screen or "session expired" message appears
**And** if the stored identity is corrupted or invalid, a clear recovery prompt is shown

### Story 2.4: Display Name and Avatar Management

As a **user**,
I want to update my display name and avatar,
So that I can control how others see me across all guilds.

**Acceptance Criteria:**

**Given** a user is authenticated
**When** they navigate to their user settings
**Then** they can change their display name
**And** they can upload or select a new avatar (with preview)
**And** changes are saved via REST API and reflected immediately in all guilds
**And** avatar images are validated (type, size limits) and stored via the file storage system
**And** the settings form follows UX patterns: single-column, labels above inputs, validate on blur, fire CTA

### Story 2.5: Cross-Instance Identity Verification

As a **user**,
I want to use the same identity across multiple Discool instances,
So that I don't need to create a new account on every instance I join.

**Acceptance Criteria:**

**Given** a user has an identity created on Instance A
**When** they click an invite link to a guild on Instance B
**Then** the SPA detects their existing identity in browser storage
**And** the user is shown "Join [Guild Name] as [Username]?" with their avatar (one-click join)
**And** Instance B verifies the identity cryptographically using the public key
**And** if the identity is new to Instance B, a user record is created automatically
**And** the user's display name and avatar are consistent across instances
**And** the entire cross-instance join completes in under 10 seconds (existing identity target)

### Story 2.6: Optional Email Association

As a **user**,
I want to optionally associate an email address with my identity,
So that I have a recovery option if I lose my browser storage.

**Acceptance Criteria:**

**Given** a user is authenticated
**When** they navigate to identity settings and enter an email address
**Then** a verification email is sent to the provided address
**And** the user clicks the verification link to confirm the association
**And** the server stores an encrypted copy of the user's private key, encrypted with a key derived from a server-side secret + email
**And** the email association is visible in user settings with a "Verified" status
**And** the email is optional — it is never required for normal operation
**And** a subtle, non-intrusive prompt suggests email association after the first session (not during onboarding)

### Story 2.7: Email-Based Identity Recovery

As a **user**,
I want to recover my identity via email if my browser storage is cleared,
So that I don't permanently lose access to my guilds and messages.

**Acceptance Criteria:**

**Given** a user opens Discool with no identity in browser storage
**When** they select "Recover existing identity" and enter their email
**Then** the server sends a recovery link to the registered email address
**And** clicking the link opens the SPA with a recovery token
**And** the encrypted private key is retrieved from the server and decrypted client-side
**And** the identity is restored in browser storage (IndexedDB)
**And** the user sees all their guilds, channels, and data as before
**And** if no identity is found for the email, a clear message is shown: "No identity found for this email"
**And** if the email isn't received, a "check spam or try again" message is shown
**And** the recovery screen uses plain language ("recover your identity") not cryptographic jargon

### Story 2.8: Vitest Setup and Client Test Baseline

As a **developer**,
I want a frontend unit/component test harness (Vitest + @testing-library/svelte),
So that we stop accruing client test debt and can safely evolve identity/auth UX.

**Acceptance Criteria:**

**Given** the client project dependencies are installed
**When** `cd client && npm run test` is executed
**Then** Vitest runs in CI-friendly mode (non-watch) and exits non-zero on failures
**And** the default test environment is `jsdom`
**And** `$lib` alias resolution works in tests
**And** `@testing-library/svelte` is configured for component tests

**Given** GitHub Actions CI runs on a pull request
**When** the client job executes
**Then** `npm run test` is executed in addition to lint/check/build

**Given** the identity/auth client modules from Stories 2.1–2.3 exist
**When** the test suite is run
**Then** there are deterministic unit tests covering core non-UI logic (crypto helpers, storage utilities, session persistence/restore)
**And** there are smoke-level component tests for the identity recovery prompts

## Epic 3: P2P Discovery & Federation Foundation

Instances automatically discover each other via libp2p Kademlia DHT and Gossipsub. Operators can opt their instance out of discovery (unlisted mode). Cross-instance identity verification is operational. The P2P network has Sybil resistance and no single point of failure.

### Story 3.1: libp2p Node Bootstrap and Instance Identity

As an **instance operator**,
I want my instance to have a unique cryptographic identity on the P2P network,
So that other instances can verify my instance is authentic.

**Acceptance Criteria:**

**Given** the server starts for the first time
**When** the P2P subsystem initializes
**Then** an Ed25519 keypair is generated for the instance (separate from user keypairs) and persisted in the database or config directory
**And** a libp2p 0.56.0 swarm is created with the instance keypair as the peer ID
**And** the swarm listens on a configurable port (default: separate from the HTTP port)
**And** on subsequent starts, the same instance keypair is loaded (identity is stable)
**And** the P2P node startup is logged with the instance's peer ID
**And** if P2P initialization fails, the server continues running (P2P is not a hard dependency for local operation)

### Story 3.2: Kademlia DHT Instance Discovery

As an **instance operator**,
I want my instance to discover other Discool instances on the network,
So that users can find and join guilds across different instances.

**Acceptance Criteria:**

**Given** the P2P node is running and configured with bootstrap peer addresses (from config)
**When** the instance starts
**Then** it connects to bootstrap peers and joins the Kademlia DHT
**And** the instance registers itself in the DHT with its public address and metadata (instance name, version)
**And** the instance discovers other registered instances within 60 seconds of coming online
**And** discovered instances are stored locally and periodically refreshed
**And** if all bootstrap peers are unreachable, the instance operates in standalone mode and retries with exponential backoff
**And** the P2P network status (discovered instances count, connection count) is exposed via the admin health dashboard (from Story 1.6)

### Story 3.3: Gossipsub Inter-Instance Communication

As a **system**,
I want instances to exchange events via Gossipsub pub/sub,
So that cross-instance operations (identity verification, guild discovery) can function.

**Acceptance Criteria:**

**Given** two or more instances have discovered each other via Kademlia
**When** an instance publishes an event to a Gossipsub topic
**Then** all subscribed instances receive the event
**And** topics are namespaced by protocol version (`/discool/gossip/1.0.0/...`)
**And** messages are signed by the sending instance's keypair and verified by receivers
**And** invalid or unsigned messages are rejected and logged
**And** message delivery is best-effort (eventual consistency, not guaranteed ordering)
**And** Gossipsub mesh parameters are tuned for small-to-medium network sizes (tens to hundreds of instances)

### Story 3.4: Sybil Resistance and Network Health

As an **instance operator**,
I want the P2P network to resist attacks from fake instances,
So that the discovery network remains trustworthy and functional.

**Acceptance Criteria:**

**Given** the P2P network is operational
**When** a new instance attempts to join the DHT
**Then** rate-based participation controls limit how quickly new peers can register
**And** peers that flood the network with excessive requests are temporarily throttled
**And** the DHT eviction policy favors long-lived, well-behaved peers over newly joined ones
**And** the operator can view P2P network health metrics (peer count, message rate, rejected peers) in the admin panel
**And** if the local instance detects degraded network conditions, it logs warnings

### Story 3.5: Private Instance and Discovery Opt-Out

As an **instance operator**,
I want to opt my instance out of P2P discovery,
So that my instance operates as an unlisted, private deployment.

**Acceptance Criteria:**

**Given** the operator sets `discovery.enabled = false` in the config (or toggled during first-run setup)
**When** the server starts
**Then** the P2P node does NOT register in the Kademlia DHT
**And** the instance does NOT appear in any other instance's discovered list
**And** the instance can still be accessed directly via its URL (invite links still work)
**And** users from other instances can still join guilds via direct invite links (identity verification works without DHT)
**And** the operator can re-enable discovery at any time by changing the config and restarting
**And** the P2P network status in the admin panel shows "Discovery: Disabled (Unlisted)"

## Epic 4: Guilds, Channels & Invites

Maya can create guilds, configure settings (name, icon, description), create/rename/reorder/delete text and voice channels, organize them into categories, and generate invite links (standard and single-use). Liam can click an invite link and join a guild, landing in the default channel. The SPA navigation shell (GuildRail, ChannelList, four-panel layout) is built.

### Story 4.1: SPA Navigation Shell and Routing

As a **user**,
I want a familiar navigation layout with guild icons, channel list, and message area,
So that I can navigate Discool intuitively without learning a new pattern.

**Acceptance Criteria:**

**Given** a user is authenticated
**When** the SPA loads
**Then** the four-panel desktop layout is rendered: GuildRail (72px) | ChannelList (240px) | MessageArea (flex-1) | MemberList (240px)
**And** svelte5-router handles client-side navigation with unique URLs per guild/channel (`instance.com/guild/channel`)
**And** browser back/forward buttons work (channel navigation pushes to browser history)
**And** refreshing the page returns the user to the same view
**And** on tablet (768-1023px), the member list is hidden by default with a toggle button
**And** on mobile (<768px), a single-panel drill-down with bottom navigation bar is shown
**And** the SPA initial load is under 3 seconds on 4G (NFR4) with bundle under 500KB gzipped (NFR6)
**And** skeleton loaders are shown during initial data fetch
**And** a skip-to-content link is the first focusable element for keyboard users

### Story 4.2: Guild Creation and Settings

As a **community builder**,
I want to create a guild with a name and icon,
So that I have a space for my community to gather.

**Acceptance Criteria:**

**Given** a user is authenticated
**When** they click the "+" button at the bottom of the GuildRail
**Then** a guild creation dialog appears with a name field (required) and optional icon upload
**And** submitting the form creates the guild on the server (guilds table migration in this story)
**And** the user becomes the guild owner automatically
**And** a default `#general` text channel is created automatically
**And** the user lands in the new guild's `#general` channel
**And** the guild icon appears in the GuildRail
**And** guild owners can later edit guild settings (name, icon, description) via a settings panel
**And** the dialog follows UX patterns: single-column, fire CTA "Create Guild", Enter key submits

### Story 4.3: Text and Voice Channel Management

As a **guild owner**,
I want to create, rename, reorder, and delete text and voice channels,
So that I can organize my guild's communication spaces.

**Acceptance Criteria:**

**Given** the user is the guild owner or has channel management permissions
**When** they click the "+" button next to a category header in the ChannelList
**Then** a channel creation dialog appears with name field and type selector (text/voice)
**And** the channel is created and appears in the ChannelList (channels table migration in this story)
**And** text channels show a `#` icon; voice channels show a speaker icon
**And** guild owners can rename channels via right-click context menu -> Edit Channel
**And** guild owners can reorder channels via drag-and-drop within and between categories
**And** guild owners can delete channels via right-click context menu -> Delete Channel (with confirmation dialog)
**And** deleting a channel with messages shows a warning: "This will permanently delete all messages in this channel"
**And** the ChannelList is scrollable if many channels exist

### Story 4.4: Channel Categories

As a **guild owner**,
I want to organize channels into collapsible categories,
So that my guild's channel list stays structured as it grows.

**Acceptance Criteria:**

**Given** the user is the guild owner or has channel management permissions
**When** they click "Create Category" in the channel list
**Then** a category is created with a name (uppercase, small text display)
**And** channels can be dragged into categories
**And** categories are collapsible/expandable via click on the chevron or category name
**And** collapsed categories hide their channels but preserve collapsed state per user
**And** the "+" button for creating channels appears next to each category header (visible on hover, always visible for guild owners)
**And** categories can be renamed and deleted (deleting a category moves its channels to uncategorized)
**And** keyboard: Enter toggles expand/collapse, arrow keys navigate children

### Story 4.5: Invite Link Generation and Management

As a **guild owner**,
I want to generate invite links so people can join my guild,
So that my community can grow through shared links.

**Acceptance Criteria:**

**Given** the user is the guild owner or has invite management permissions
**When** they click the "Invite People" button in the guild header or settings
**Then** an invite modal appears with options to generate a reusable or single-use invite link
**And** the generated link is displayed with a "Copy" button (copies to clipboard)
**And** copying shows a toast: "Invite link copied"
**And** the invite links table is created (migration in this story) with columns: guild_id, code, type (reusable/single_use), uses_remaining, created_by, created_at, revoked
**And** guild owners can view all active invite links with their type, creator, and creation date (InviteLinkCard component)
**And** guild owners can revoke any invite link (destructive action, fire red button)
**And** single-use links show remaining uses and become invalid after use
**And** invite link generation is at most 2 clicks (button -> copy)

### Story 4.6: Join Guild via Invite Link

As a **new or existing user**,
I want to click an invite link and join a guild,
So that I can start participating in a community immediately.

**Acceptance Criteria:**

**Given** a user clicks a Discool invite link (e.g., `instance.com/invite/abc123`)
**When** the SPA loads
**Then** if the user has no identity: the onboarding screen shows guild icon + "Pick a username to join [Guild Name]"
**And** after creating identity (or one-click join for existing identity), guild membership is created (guild_members table migration in this story)
**And** the user lands in the guild's default channel with the text input focused and ready to type
**And** if the guild has a welcome screen configured: it is shown with rules/TOS and "Accept & Continue" button before landing
**And** the guild appears in the user's GuildRail
**And** invalid invite links show: "This invite link is invalid or has expired"
**And** if the instance is unreachable: "This instance is currently unreachable. Your invite link will work when it's back online."
**And** new identity creation completes in under 30 seconds; existing identity join in under 10 seconds
**And** the invite link renders OpenGraph meta tags (guild name, icon) for link preview unfurling in chat apps

### Story 4.7: Guild Navigation and Activity Indicators

As a **user**,
I want to see which guilds have unread activity and switch between them instantly,
So that I can stay aware of conversations across all my communities.

**Acceptance Criteria:**

**Given** a user has joined multiple guilds
**When** they view the GuildRail
**Then** each guild shows its icon (48px circle) with the guild name in a tooltip on hover
**And** the active guild has an ice blue indicator bar on the left edge
**And** guilds with unread messages show a fire dot badge
**And** clicking a guild icon switches to that guild's channel list and last-viewed channel
**And** guild switching feels instant — no loading spinner; channel list swaps from cached data
**And** the user can reorder guilds via drag-and-drop in the GuildRail
**And** a Home button at the top of the GuildRail provides access to DMs (placeholder for Epic 6)
**And** keyboard: arrow keys navigate guild icons, Enter selects
**And** each guild icon is a button with `aria-label="Guild Name"`

## Epic 5: Roles, Permissions & Member Management

Maya can create roles with specific permissions, establish role hierarchy, assign roles to members, set channel-level permission overrides, and delegate role management. Members see a member list grouped by role with online/offline presence. The permission engine enforces RBAC on every server-side operation.

### Story 5.1: Role CRUD and Default Roles

As a **guild owner**,
I want to create, edit, and delete custom roles for my guild,
So that I can organize members and control access to features.

**Acceptance Criteria:**

**Given** the user is the guild owner
**When** they navigate to Guild Settings -> Roles
**Then** they see an `@everyone` default role that cannot be deleted (applies to all members)
**And** they can create a new role with a name and display color
**And** roles are listed in hierarchy order (owner role at top, @everyone at bottom)
**And** they can rename and change the color of any custom role
**And** they can delete a custom role (members with that role lose it; confirmation dialog required)
**And** the roles table is created (migration in this story) with columns: guild_id, name, color, position, permissions_bitflag, created_at
**And** a role_assignments table is created for the many-to-many relationship between users and roles

### Story 5.2: Permission Assignment and Bitflag Engine

As a **guild owner**,
I want to assign specific permissions to each role,
So that I can control what each group of members can do.

**Acceptance Criteria:**

**Given** the guild owner is editing a role in Guild Settings -> Roles
**When** they open the permissions panel for that role
**Then** they see a list of all available permissions as toggles: SEND_MESSAGES, MANAGE_CHANNELS, KICK_MEMBERS, BAN_MEMBERS, MANAGE_ROLES, MANAGE_GUILD, MANAGE_INVITES, MUTE_MEMBERS, VIEW_MOD_LOG, ATTACH_FILES, ADD_REACTIONS, MANAGE_MESSAGES
**And** each permission is a toggle switch (on/off)
**And** permissions are stored as a bitflag integer on the role record
**And** the owner role has all permissions implicitly and cannot be modified
**And** the @everyone role defaults to SEND_MESSAGES, ATTACH_FILES, ADD_REACTIONS enabled
**And** changes to permissions auto-save (toggle = immediate save with toast confirmation)
**And** the server validates that every API call checks the caller's computed permissions before executing (NFR16)

### Story 5.3: Role Hierarchy and Ordering

As a **guild owner**,
I want to set role hierarchy so higher roles take precedence,
So that permission conflicts are resolved predictably.

**Acceptance Criteria:**

**Given** the guild owner is viewing Guild Settings -> Roles
**When** they drag roles to reorder them
**Then** the role hierarchy is updated (higher position = higher authority)
**And** a user's effective permissions are the union of all their roles' permissions
**And** moderation actions (kick, ban, mute) can only target users whose highest role is lower than the actor's highest role
**And** the guild owner is always at the top of the hierarchy and cannot be reordered
**And** role position changes are persisted immediately and the computed permission cache is invalidated for all affected guild members

### Story 5.4: Role Assignment to Members

As a **guild owner**,
I want to assign and remove roles from guild members,
So that members get the appropriate permissions for their responsibilities.

**Acceptance Criteria:**

**Given** the user has MANAGE_ROLES permission or is the guild owner
**When** they right-click a member in the member list or open their profile popover
**Then** an "Assign Role" option is available showing all roles below the assigner's highest role
**And** they can toggle roles on/off for that member
**And** role changes take effect immediately (computed permission cache invalidated for that user)
**And** the member's username color in the member list and chat updates to reflect their highest role color
**And** role assignments are stored in the role_assignments table
**And** users cannot assign roles equal to or above their own highest role (enforced server-side)

### Story 5.5: Channel-Level Permission Overrides

As a **guild owner**,
I want to set permission overrides for specific roles on specific channels,
So that I can create private channels or restrict actions per channel.

**Acceptance Criteria:**

**Given** the user has MANAGE_CHANNELS permission or is the guild owner
**When** they open channel settings for a specific channel
**Then** they see a "Permission Overrides" section listing roles
**And** for each role, they can set per-permission overrides: Allow / Deny / Inherit
**And** the channel_permission_overrides table is created (migration in this story) with columns: channel_id, role_id, allow_bitflag, deny_bitflag
**And** effective channel permissions = (role permissions | channel allow) & ~channel deny
**And** a channel with @everyone denied on VIEW means only explicitly allowed roles can see it (private channel)
**And** override changes invalidate the permission cache for affected user-guild pairs
**And** the override UI uses a three-state toggle per permission (allow/deny/inherit) with clear visual distinction

### Story 5.6: Role Management Delegation

As a **guild owner**,
I want to allow moderators to assign roles,
So that I don't have to handle every role change myself.

**Acceptance Criteria:**

**Given** a role has the MANAGE_ROLES permission enabled
**When** a user with that role attempts to assign or remove roles from another member
**Then** they can only assign roles that are lower in the hierarchy than their own highest role
**And** they cannot modify the permissions of any role (only the guild owner can do that)
**And** they cannot delete roles (only the guild owner can)
**And** all role assignment actions by delegated managers are visible in the mod log (when moderation is implemented in Epic 8)
**And** the server enforces these constraints on every role assignment API call

### Story 5.7: Member List with Presence

As a **user**,
I want to see who's in my guild and whether they're online,
So that I know who's available and the guild feels alive.

**Acceptance Criteria:**

**Given** a user is viewing a guild
**When** the member list panel is visible (right sidebar, 240px)
**Then** members are grouped by their highest role (role name as section header, colored)
**And** within each role group, online members appear first, then offline
**And** each member entry shows: avatar (32px) + status dot (green=online, yellow=idle, gray=offline) + username (colored by highest role)
**And** clicking a member opens a profile popover with: avatar, username, roles, "Send DM" button, and moderation actions (if the viewer has permissions)
**And** the member list updates in real-time as users come online/offline
**And** presence is tracked via WebSocket connection state (connected = online, disconnect after timeout = offline)
**And** the member list uses virtual scrolling for guilds with many members
**And** screen readers announce member count and status for each role group

## Epic 6: Real-Time Text Communication

Users can send, edit, delete messages in text channels, react with emoji, upload files, view rich embeds, and send direct messages. Message history is persistent with virtual scrolling at 60fps. Activity indicators show where conversations are happening. Users can block others (complete erasure). The WebSocket gateway handles real-time events with auto-reconnect. Input sanitization and rate limiting protect against abuse.

### Story 6.1: WebSocket Gateway and Connection Management

As a **user**,
I want a persistent real-time connection to the server,
So that I receive messages and events instantly without polling.

**Acceptance Criteria:**

**Given** a user is authenticated
**When** the SPA establishes a WebSocket connection to `/ws`
**Then** the server authenticates the connection using the session token
**And** the WebSocket protocol uses JSON envelopes: `{"op": "...", "d": {...}, "s": 42, "t": ...}`
**And** server events include: `message_create`, `message_update`, `message_delete`, `typing_start`, `presence_update`, `guild_update`, `channel_update`
**And** client operations are prefixed with `c_`: `c_message_create`, `c_typing_start`, etc.
**And** the server tracks connected clients per guild/channel for targeted event broadcasting
**And** message delivery latency is under 100ms same-region (NFR1)
**And** if the connection drops, the client auto-reconnects within 5 seconds with exponential backoff (NFR25)
**And** during reconnection, the UI shows "Reconnecting..." in a status bar, not a blocking overlay
**And** rate limiting is enforced per-connection: excessive messages are rejected with an error event (NFR15)

### Story 6.2: Send and Display Text Messages

As a **user**,
I want to send and read text messages in a channel,
So that I can communicate with others in my guild.

**Acceptance Criteria:**

**Given** a user is in a text channel
**When** they type a message in the MessageInput and press Enter (or click the fire Send button)
**Then** the message appears immediately in the message area (optimistic UI)
**And** the message is sent via WebSocket (`c_message_create`) and persisted to the database (messages table migration in this story)
**And** all other users in the channel receive the message in real-time via `message_create` event
**And** messages render as MessageBubble: author avatar (32px) + author name (role-colored) + timestamp + message content
**And** consecutive messages from the same author collapse into compact mode (no repeated avatar/name)
**And** system messages (join/leave) render centered and muted
**And** the message input is auto-focused when navigating to a channel
**And** Shift+Enter creates a new line; Enter sends
**And** empty channels show: "This is the beginning of #channel-name. Say something!"
**And** all message content is sanitized server-side before storage to prevent XSS (NFR14)
**And** messages are persisted before acknowledgment to ensure zero loss (NFR27)

### Story 6.3: Message History and Virtual Scrolling

As a **user**,
I want to scroll through message history smoothly,
So that I can read past conversations without lag or jank.

**Acceptance Criteria:**

**Given** a channel has hundreds or thousands of messages
**When** the user scrolls up in the message area
**Then** older messages load progressively via REST API pagination (cursor-based, not offset-based)
**And** skeleton placeholders appear while messages load (matching message shape)
**And** scrolling is smooth at 60fps with 10,000+ messages in the channel (NFR8)
**And** only visible messages are rendered in the DOM (virtual scrolling)
**And** scrolling down past the newest loaded message shows new messages arriving in real-time
**And** a "Jump to present" button appears when the user has scrolled far from the newest messages
**And** the scroll position is preserved when switching away from and back to a channel
**And** client memory usage stays under 200MB with 5 guilds active (NFR7)

### Story 6.4: Edit and Delete Own Messages

As a **user**,
I want to edit and delete my own messages,
So that I can correct mistakes or remove messages I no longer want visible.

**Acceptance Criteria:**

**Given** a user has sent a message
**When** they press Up arrow (with empty input) or right-click -> Edit
**Then** the message enters inline edit mode with the original text in the input
**And** pressing Enter saves the edit; Escape cancels
**And** edited messages show an "(edited)" label next to the timestamp
**And** the edit is sent via WebSocket and broadcast to all channel members as `message_update`
**And** when a user right-clicks their message -> Delete (or presses Del)
**Then** a confirmation dialog appears
**And** after confirmation, the message is removed from view for all users via `message_delete`
**And** the hover action bar (edit, delete, react, reply) appears on mouseover, top-right of the message

### Story 6.5: Emoji Reactions

As a **user**,
I want to react to messages with emoji,
So that I can respond quickly without sending a full message.

**Acceptance Criteria:**

**Given** a user hovers over a message
**When** they click the react button (+) in the hover action bar or right-click -> React
**Then** an emoji picker popover appears
**And** selecting an emoji adds it as a reaction to the message
**And** reactions appear below the message as emoji badges with count
**And** clicking an existing reaction emoji toggles the user's reaction on/off
**And** multiple users reacting with the same emoji increments the count
**And** reactions are broadcast to all channel members in real-time
**And** the reactions table is created (migration in this story) with columns: message_id, user_id, emoji, created_at
**And** users with ADD_REACTIONS permission denied cannot add reactions

### Story 6.6: File Upload and Sharing

As a **user**,
I want to upload and share files in channels,
So that I can share images, documents, and other files with my guild.

**Acceptance Criteria:**

**Given** a user is in a text channel
**When** they click the file upload button (paperclip icon) or drag-and-drop a file onto the message area
**Then** a file preview chip appears above the message input
**And** they can add a text message alongside the file (optional)
**And** clicking Send uploads the file and creates the message
**And** a progress bar shows upload percentage in the message input area
**And** uploaded images render inline in the message at a reasonable size (clickable to fullscreen)
**And** non-image files render as download links with file name, size, and type icon
**And** files are stored on the local filesystem in the configured path (FileStorageProvider trait for future S3)
**And** file types and sizes are validated server-side (configurable max size in TOML config)
**And** the drag-drop overlay appears when dragging a file over the message area
**And** users with ATTACH_FILES permission denied cannot upload files

### Story 6.7: Rich Embeds and Markdown Rendering

As a **user**,
I want messages to render markdown formatting and show link previews,
So that conversations are visually rich and informative.

**Acceptance Criteria:**

**Given** a user sends a message containing markdown or URLs
**When** the message is rendered
**Then** markdown is rendered: **bold**, *italic*, `inline code`, ```code blocks``` (with syntax highlighting), ~~strikethrough~~, [links](url), > blockquotes
**And** code blocks use the monospace system font stack
**And** URLs are automatically converted to clickable links
**And** shared URLs generate a compact embed preview (title, description, thumbnail) fetched server-side
**And** embed metadata is cached to avoid repeated fetches
**And** markdown formatting toolbar appears on text selection in the input: bold (Ctrl+B), italic (Ctrl+I), code (Ctrl+E)
**And** all rendered HTML is sanitized to prevent XSS (NFR14)

### Story 6.8: Typing Indicators and Channel Activity

As a **user**,
I want to see who's typing and which channels have activity,
So that conversations feel alive and I know where to look.

**Acceptance Criteria:**

**Given** a user is typing in a channel
**When** they start typing
**Then** a `c_typing_start` event is sent via WebSocket
**And** other users in the channel see "[Username] is typing..." above the message input
**And** if multiple users are typing: "[User1] and [User2] are typing..." (max 3 names, then "Several people are typing...")
**And** typing indicators disappear after 5 seconds of inactivity
**And** channels with unread messages show a bold channel name + ice blue dot in the ChannelList
**And** the unread indicator clears when the user views the channel
**And** Alt+Shift+Up/Down navigates to the previous/next unread channel

### Story 6.9: Direct Messages

As a **user**,
I want to send direct messages to other users,
So that I can have private conversations outside of guild channels.

**Acceptance Criteria:**

**Given** a user clicks "Send DM" on another user's profile popover or member list entry
**When** the DM view opens
**Then** it shows the conversation history with that user
**And** DMs appear under the Home button in the GuildRail (DM list)
**And** DMs use the same MessageBubble and MessageInput components as channels
**And** DMs are stored in the database with a dedicated dm_channels table (migration in this story)
**And** DMs work across guilds (if two users share any guild, they can DM)
**And** DM notifications appear as unread badges on the Home button in the GuildRail
**And** the quick switcher (Ctrl+K) includes DM conversations in search results

### Story 6.10: User Blocking (Complete Erasure)

As a **user**,
I want to block another user so they are completely invisible to me,
So that I can remove toxic individuals from my experience entirely.

**Acceptance Criteria:**

**Given** a user right-clicks another user -> Block (or via profile popover)
**When** they confirm the block
**Then** the blocked user's messages are removed from view — no placeholders, no "hidden message" hints, no trace
**And** the blocked user's reactions are removed from view
**And** the blocked user does not appear in the member list for the blocking user
**And** the blocked user's presence (online/offline) is hidden
**And** the blocked user's typing indicators are hidden
**And** blocks are stored client-side (IndexedDB) and optionally synced to the server for cross-device consistency
**And** the user can manage their block list in user settings (view blocked users, unblock)
**And** unblocking restores visibility of the user's future messages (past messages during block remain hidden)

### Story 6.11: Error Handling and Status Communication

As a **user**,
I want clear, honest feedback when something goes wrong,
So that I understand what happened and what to do about it.

**Acceptance Criteria:**

**Given** an error occurs during any operation
**When** the error is displayed to the user
**Then** error messages use plain language: "Connection lost. Reconnecting..." not "WebSocket error 1006"
**And** permission errors show: "You don't have permission to do this"
**And** failed message sends show an error toast with a "Retry?" action
**And** the toast system renders bottom-right, max 3 visible, stacked vertically
**And** success toasts auto-dismiss after 4 seconds; error toasts persist until dismissed
**And** toasts are pausable on hover/focus
**And** async operations always show status text: "Connecting...", "Uploading (43%)...", "Saving..."
**And** no spinner appears without accompanying text
**And** the server's AppError enum maps to user-friendly messages; internal details are never exposed to clients
**And** all error states are accessible (announced via `aria-live`)

### Story 6.12: Quick Switcher

As a **user**,
I want a keyboard shortcut to quickly search and navigate to any guild, channel, or DM,
So that I can move around the app efficiently.

**Acceptance Criteria:**

**Given** a user presses Ctrl+K (or Cmd+K on macOS)
**When** the quick switcher overlay appears
**Then** a search input is focused with a list of recently active items (before typing)
**And** typing performs fuzzy search across guild names, channel names, and DM usernames
**And** results are grouped by type: Channels, DMs, Guilds
**And** arrow keys navigate results, Enter selects and navigates, Escape closes
**And** the switcher renders as a centered modal overlay with focus trapped
**And** selecting a result navigates to that guild/channel/DM instantly

## Epic 7: Voice Communication

Users can drop into voice channels with one click, see who's in a channel before joining, control mute/deafen/volume, and auto-reconnect after connection loss. Voice quality targets Mumble-tier latency. The VoiceBar provides persistent compact controls. WebRTC handles peer connections with STUN/TURN for NAT traversal.

### Story 7.1: WebRTC Signaling and Peer Connection

As a **user**,
I want to establish a voice connection when I click a voice channel,
So that I can talk with others without any setup.

**Acceptance Criteria:**

**Given** a user clicks a voice channel in the ChannelList
**When** the voice connection is initiated
**Then** the client sends a `c_voice_join` event via the existing WebSocket
**And** the server returns an SDP offer and ICE candidates for the SFU
**And** the client completes the WebRTC handshake (offer/answer/ICE)
**And** the connection uses webrtc-rs v0.17.x on the server side
**And** STUN servers are used for NAT traversal by default (configurable in TOML)
**And** TURN server relay is available as fallback (configurable in TOML)
**And** all voice streams are encrypted with SRTP (NFR12)
**And** voice channel join completes in under 2 seconds from click to connected (NFR2)
**And** if the connection fails, the UI shows "Could not connect to voice. Retrying..." with automatic retry
**And** if retry fails, the UI shows "Voice connection failed. Check your network."
**And** voice connection failure does not affect text chat functionality (NFR28)

### Story 7.2: Voice Bar and Basic Controls

As a **user**,
I want persistent voice controls when I'm in a voice channel,
So that I can manage my audio without navigating away from what I'm doing.

**Acceptance Criteria:**

**Given** a user has joined a voice channel
**When** the connection is established
**Then** a VoiceBar appears fixed at the bottom of the message area (above the message input)
**And** the VoiceBar shows: voice channel name + guild name, connection quality indicator (green/yellow/red dot), mute toggle (mic icon), deafen toggle (headphone icon), disconnect button (phone icon, fire red)
**And** clicking mute toggles the microphone on/off (mic icon crossed out when muted)
**And** clicking deafen toggles all incoming audio on/off (headphone icon crossed out when deafened; deafen also mutes)
**And** clicking disconnect leaves the voice channel and the VoiceBar disappears
**And** keyboard shortcuts work globally: M = toggle mute, D = toggle deafen, Ctrl+D = disconnect
**And** the VoiceBar is compact and does not interfere with chat usage
**And** all VoiceBar controls have accessible labels and state changes are announced via `aria-live`

### Story 7.3: Voice Channel Participants Display

As a **user**,
I want to see who is in a voice channel before and after joining,
So that I know who I'll be talking with.

**Acceptance Criteria:**

**Given** users are in a voice channel
**When** another user views the ChannelList
**Then** the voice channel shows participant avatars inline (max 3 shown + "+N" overflow)
**And** the voice channel shows the participant count
**And** clicking the expand button on the VoiceBar opens a full voice participant panel
**And** each VoiceParticipant shows: avatar (with ice blue border glow when speaking), username, mute/deafen icons if applicable
**And** the speaking indicator is a static border glow (not an animation) that appears when the user is speaking and disappears when they stop
**And** `prefers-reduced-motion` is respected for any speaking indicator transitions
**And** screen readers announce: "3 users in voice channel General"

### Story 7.4: Individual Volume Control

As a **user**,
I want to adjust the volume of individual users in a voice channel,
So that I can balance audio levels to my preference.

**Acceptance Criteria:**

**Given** a user is in a voice channel with other participants
**When** they click on a participant in the voice panel
**Then** a volume slider appears (horizontal, accessible via keyboard)
**And** adjusting the slider changes that user's audio volume locally (client-side only)
**And** volume settings persist per user across sessions (stored in IndexedDB)
**And** the slider range is 0% (muted) to 200% (amplified)
**And** default volume is 100%
**And** moderators see an additional "Kick from voice" option on each participant (placeholder, implemented in Epic 8)

### Story 7.5: Voice Channel Switching

As a **user**,
I want to switch between voice channels seamlessly,
So that I can move between conversations without manual disconnect/reconnect.

**Acceptance Criteria:**

**Given** a user is currently connected to a voice channel
**When** they click a different voice channel
**Then** the previous connection disconnects automatically
**And** a new connection to the target channel is established
**And** the transition is seamless — no manual disconnect step required
**And** the VoiceBar updates to show the new channel name
**And** other users see the participant leave and join in real-time
**And** the switch completes within the 2-second join target (NFR2)

### Story 7.6: Voice Auto-Reconnect

As a **user**,
I want to automatically reconnect to voice after a brief connection loss,
So that a network hiccup doesn't permanently drop me from the conversation.

**Acceptance Criteria:**

**Given** a user is in a voice channel and their network briefly drops
**When** the WebRTC connection is lost
**Then** the VoiceBar shows "Reconnecting..." with a pulsing connection indicator
**And** the client automatically attempts to re-establish the voice connection with exponential backoff
**And** if reconnection succeeds within 5 seconds, the user is back in the same channel with no manual action needed (NFR26)
**And** if reconnection takes longer, the VoiceBar shows "Connection lost" and continues retrying
**And** if reconnection ultimately fails (e.g., 30 seconds of attempts), the user is disconnected and the VoiceBar disappears
**And** during reconnection, text chat continues working normally (NFR28)
**And** other users in the channel see the reconnecting user's status update in real-time

### Story 7.7: Mobile Voice Controls

As a **mobile user**,
I want touch-friendly voice controls,
So that I can manage voice on my phone without precision tapping.

**Acceptance Criteria:**

**Given** a user is in a voice channel on a mobile device (<768px)
**When** they view the voice controls
**Then** a compact voice bar appears at the top of the message area (channel name + mute icon)
**And** swiping up on the voice bar opens a full bottom sheet
**And** the bottom sheet shows: large mute button (64px, center), large deafen button (64px), disconnect button (fire red, 64px), participant list with volume controls, connection quality indicator
**And** all touch targets are at minimum 48px with 8px spacing between adjacent targets
**And** the bottom sheet allows voice control while browsing channels (stays open during navigation)
**And** the bottom sheet dismisses by swiping down or tapping outside

## Epic 8: Moderation, Reporting & Data Privacy

Rico can mute (timed/permanent), kick, and ban users, kick from voice, delete messages, and view user message history — all via context menus. All actions are logged in an append-only mod log. Users can report content, and moderators can review/act on a report queue with lifecycle tracking. Users can export their data and delete their accounts (GDPR compliance).

### Story 8.1: Mute User (Timed and Permanent)

As a **moderator**,
I want to mute a user in the guild for a specified duration,
So that I can temporarily restrict disruptive users from sending messages.

**Acceptance Criteria:**

**Given** a user has MUTE_MEMBERS permission
**When** they right-click a member -> Mute
**Then** a dialog appears with duration options: 1 hour, 24 hours, 1 week, custom, permanent
**And** the moderator must enter a reason (required text field)
**And** confirming mutes the user — they cannot send messages in any text channel of the guild
**And** the muted user sees a clear indication that they are muted and when the mute expires
**And** timed mutes expire automatically; the user regains send permissions without manual intervention
**And** the mute is stored in the database (moderation_actions table migration in this story)
**And** a toast confirms: "User muted for 24 hours"
**And** the action can only target users whose highest role is lower than the moderator's highest role
**And** the mute action is recorded in the mod log (see Story 8.5)

### Story 8.2: Kick User from Guild

As a **moderator**,
I want to kick a user from the guild,
So that I can remove them while allowing them to rejoin via a new invite.

**Acceptance Criteria:**

**Given** a user has KICK_MEMBERS permission
**When** they right-click a member -> Kick from guild
**Then** a confirmation dialog appears with the user's name and a required reason field
**And** confirming removes the user from the guild
**And** the kicked user's guild disappears from their GuildRail
**And** the kicked user can rejoin via a new invite link
**And** the action can only target users whose highest role is lower than the moderator's
**And** a toast confirms: "User kicked from guild"
**And** the kick is recorded in the mod log

### Story 8.3: Ban User from Guild

As a **moderator**,
I want to ban a user from the guild,
So that I can permanently remove them and prevent re-entry with the same identity.

**Acceptance Criteria:**

**Given** a user has BAN_MEMBERS permission
**When** they right-click a member -> Ban from guild
**Then** a confirmation dialog appears with the user's name, a required reason field, and an option to delete their recent messages (last 1hr / 24hr / 7 days / none)
**And** confirming removes the user from the guild and adds their identity to the guild ban list (guild_bans table migration in this story)
**And** the banned user cannot rejoin with the same cryptographic identity
**And** the ban action button uses fire red (destructive action styling)
**And** the action can only target users whose highest role is lower than the moderator's
**And** guild owners can view and manage the ban list (unban users) in Guild Settings
**And** the ban is recorded in the mod log

### Story 8.4: Kick User from Voice Channel

As a **moderator**,
I want to kick a user from a voice channel without kicking them from the guild,
So that I can address voice-specific issues without broader consequences.

**Acceptance Criteria:**

**Given** a user has MUTE_MEMBERS permission and a target user is in a voice channel
**When** the moderator clicks the target user in the voice participant panel -> Kick from voice
**Then** the target user is disconnected from the voice channel
**And** the target user's text channel access is unaffected
**And** the target user can rejoin the voice channel (this is not a voice ban)
**And** the moderator must provide a reason
**And** a toast confirms: "User kicked from voice"
**And** the kick is recorded in the mod log

### Story 8.5: Moderation Log

As a **moderator**,
I want all moderation actions logged in an auditable mod log,
So that the moderation team has accountability and history.

**Acceptance Criteria:**

**Given** any moderation action is performed (mute, kick, ban, voice kick, message delete, warn)
**When** the action completes
**Then** a mod log entry is created with: timestamp, moderator identity, action type, target user, reason text
**And** the mod log is append-only — entries cannot be edited or deleted
**And** moderators with VIEW_MOD_LOG permission can view the mod log in a dedicated panel within the guild
**And** each ModLogEntry renders: timestamp, moderator name + avatar, action badge (color-coded: mute = ice blue, kick = fire orange, ban = fire red), target user, reason
**And** the mod log is sortable by date and filterable by action type
**And** the mod log uses the same Scroll Area component with virtual scrolling for large logs

### Story 8.6: Message Deletion by Moderators

As a **moderator**,
I want to delete any user's messages,
So that I can remove inappropriate content from channels.

**Acceptance Criteria:**

**Given** a user has MANAGE_MESSAGES permission
**When** they right-click a message -> Delete message
**Then** the message is removed from view for all users
**And** the moderator must provide a reason (logged in mod log)
**And** the deleted message is soft-deleted in the database (not permanently erased, for audit trail)
**And** blocked users' messages that were deleted are still invisible to the blocking user (erasure takes precedence)
**And** a toast confirms: "Message deleted"

### Story 8.7: View User Message History (Moderator)

As a **moderator**,
I want to view a user's message history within the guild,
So that I can assess patterns of behavior before taking action.

**Acceptance Criteria:**

**Given** a user has MANAGE_MESSAGES or KICK_MEMBERS permission
**When** they right-click a member -> View message history
**Then** a panel opens showing that user's messages across all guild channels, sorted by recency
**And** messages are paginated with virtual scrolling
**And** each message shows the channel it was posted in, timestamp, and content
**And** the moderator can click on any message to navigate to it in context (jump to message in its channel)
**And** the panel can be filtered by channel or date range

### Story 8.8: User Content Reporting

As a **user**,
I want to report messages, files, or users to guild moderators,
So that I can flag harmful content for review.

**Acceptance Criteria:**

**Given** a user sees problematic content
**When** they right-click a message -> Report (or right-click a user -> Report)
**Then** a report dialog appears with a required reason field and optional category (spam, harassment, rule violation, other)
**And** submitting creates a report (reports table migration in this story) with: reporter_id, target (message/user), reason, category, status=pending, created_at
**And** a toast confirms: "Report submitted. A moderator will review it."
**And** users cannot report their own messages
**And** duplicate reports from the same user for the same target are prevented

### Story 8.9: Report Queue and Lifecycle

As a **moderator**,
I want to review and act on user reports,
So that I can address community issues efficiently.

**Acceptance Criteria:**

**Given** a user has VIEW_MOD_LOG permission
**When** they open the report queue (accessible via mod tools in the guild sidebar)
**Then** they see a list of reports as ReportQueueItem components, sorted by newest first
**And** each report shows: status badge (pending/reviewed/actioned/dismissed), content preview, reporter identity, reason, timestamp
**And** pending reports are highlighted; reviewed/actioned/dismissed are neutral/muted
**And** the moderator can take actions directly from the report: dismiss, warn (send DM), mute, kick, ban
**And** taking an action updates the report status (pending -> actioned) and creates a corresponding mod log entry
**And** dismissing a report changes status to dismissed with an optional reason
**And** the report queue shows "No pending reports" when empty
**And** report status transitions are tracked: pending -> reviewed -> actioned/dismissed

### Story 8.10: Personal Data Export (GDPR)

As a **user**,
I want to export all my personal data from an instance,
So that I have a copy of my information as required by data protection regulations.

**Acceptance Criteria:**

**Given** a user is authenticated
**When** they navigate to User Settings -> Privacy -> Export My Data
**Then** they can request a data export
**And** the export is generated as a JSON file containing: profile data (username, avatar, email if associated), guild memberships, messages sent, DMs, reactions, files uploaded, block list
**And** the export is available for download within the SPA
**And** if the export takes more than 2 seconds, progress is shown
**And** a toast confirms: "Your data export is ready for download"
**And** the export does not include other users' data or messages

### Story 8.11: Account Deletion (GDPR)

As a **user**,
I want to permanently delete my account from an instance,
So that my personal data is removed as required by data protection regulations.

**Acceptance Criteria:**

**Given** a user is authenticated
**When** they navigate to User Settings -> Privacy -> Delete My Account
**Then** a confirmation dialog warns: "This will permanently delete your identity and all associated data from this instance. This cannot be undone."
**And** the user must type their username to confirm (preventing accidental deletion)
**And** confirming deletes: user record, all messages (or replaces author with "Deleted User"), DMs, reactions, guild memberships, uploaded files, session data
**And** the deletion cascades properly across all related tables
**And** the user is logged out and their identity is removed from the instance
**And** the user's portable identity in their browser remains intact (they can still use it on other instances)
**And** the delete button uses fire red destructive styling and is not the default action
