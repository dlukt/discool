# Story 3.1: libp2p Node Bootstrap and Instance Identity

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As an **instance operator**,
I want my instance to have a unique cryptographic identity on the P2P network,
So that other instances can verify my instance is authentic.

## Acceptance Criteria

1. **Given** the server starts for the first time  
   **When** the P2P subsystem initializes  
   **Then** an Ed25519 keypair is generated for the instance (separate from user keypairs) and persisted in the database or config directory.

2. **Given** the server starts  
   **When** the P2P subsystem initializes  
   **Then** a libp2p 0.56.0 swarm is created with the instance keypair as the peer ID.

3. **Given** the swarm is created  
   **When** it starts listening  
   **Then** it listens on a configurable port (default: separate from HTTP port).

4. **Given** the server restarts  
   **When** the P2P subsystem initializes  
   **Then** the same instance keypair is loaded and peer identity remains stable.

5. **Given** P2P startup completes  
   **When** logs are emitted  
   **Then** startup logs include the instance peer ID.

6. **Given** P2P initialization fails  
   **When** server startup continues  
   **Then** HTTP/API service still starts and runs in local-only mode (P2P not a hard dependency).

## Tasks / Subtasks

- [x] Task 1: Add P2P runtime configuration and dependency baseline (AC: #2, #3, #6)
  - [x] 1.1 Add `libp2p = "0.56"` to `server/Cargo.toml` with required features for identity + swarm bootstrap + tokio runtime.
  - [x] 1.2 Add a new `[p2p]` section in `server/src/config/settings.rs` and `config.example.toml` with:
    - `enabled` (default: `true`)
    - `listen_host` (default: `0.0.0.0`)
    - `listen_port` (default: `4001`, separate from `server.port`)
    - `identity_key_path` (default under `./data/p2p/`)
  - [x] 1.3 Extend config validation to reject invalid p2p port values and ensure configured key path parent directory can be created.

- [x] Task 2: Implement persistent instance P2P identity (AC: #1, #4, #5)
  - [x] 2.1 Create `server/src/p2p/identity.rs` with load-or-create keypair logic:
    - load existing Ed25519 keypair from `identity_key_path` if present,
    - generate keypair if missing,
    - persist private key with restrictive permissions (0600 where supported).
  - [x] 2.2 Serialize/deserialize identity using libp2p identity APIs (protobuf encoding), never custom ad-hoc key formats.
  - [x] 2.3 Derive and return stable `PeerId` from the loaded/generated keypair.
  - [x] 2.4 Ensure private key bytes are never logged and errors redact sensitive file contents.

- [x] Task 3: Add libp2p node bootstrap module (AC: #2, #3, #5)
  - [x] 3.1 Create `server/src/p2p/mod.rs` and `server/src/p2p/node.rs` for P2P bootstrap scaffolding.
  - [x] 3.2 Build a startup path that constructs a libp2p swarm from the persisted keypair.
  - [x] 3.3 Bind swarm listen address from config (`/ip4/{listen_host}/tcp/{listen_port}` or equivalent multiaddr).
  - [x] 3.4 Emit structured startup logs containing `peer_id` and resolved listen addresses.

- [x] Task 4: Wire startup lifecycle with graceful degradation (AC: #5, #6)
  - [x] 4.1 Initialize P2P runtime during server startup in `server/src/main.rs` after config + DB migrations and before serving requests.
  - [x] 4.2 If P2P bootstrap succeeds, keep swarm task alive on Tokio runtime.
  - [x] 4.3 If P2P bootstrap fails, log warning-level diagnostics and continue starting HTTP server.
  - [x] 4.4 Ensure shutdown path cleanly drops/stops P2P task without blocking normal process termination.

- [x] Task 5: Integrate minimal state plumbing for future stories (AC: #2, #6)
  - [x] 5.1 Add `pub mod p2p;` in `server/src/lib.rs`.
  - [x] 5.2 Add lightweight app-state surface for P2P metadata needed by later stories (peer_id/listen_addrs exposure) without prematurely implementing discovery logic.
  - [x] 5.3 Keep this story scoped to identity + bootstrap only; defer Kademlia/Gossipsub behavior to Stories 3.2 and 3.3.

- [x] Task 6: Add tests and verification coverage (AC: all)
  - [x] 6.1 Unit tests in `server/src/p2p/identity.rs`:
    - generates new key when file absent,
    - re-loads same key and preserves PeerId across restarts,
    - rejects malformed key files with explicit error.
  - [x] 6.2 Config tests in `server/src/config/settings.rs` for p2p defaults and invalid values.
  - [x] 6.3 Integration coverage in `server/tests/` proving server still binds and serves `/healthz` when P2P startup is forced to fail.
  - [x] 6.4 Verify existing quality gates:
    - `cd server && cargo fmt --check && cargo clippy -- -D warnings && cargo test`

## Dev Notes

### Architecture Compliance

1. Keep P2P as a dedicated domain module under `server/src/p2p/` (do not place swarm logic in handlers or db modules).  
2. Keep startup orchestration in `main.rs`; do not let P2P bootstrap bypass existing config validation and migration ordering.  
3. Preserve existing response envelope/API contracts; this story should not alter REST payload conventions.  
4. P2P failure must be non-fatal for local instance operation, matching PRD and story scope.

### Technical Requirements (Developer Guardrails)

- Instance P2P identity must be separate from user identity/auth keys.
- PeerId must remain stable across process restarts when key file is intact.
- Key persistence must be secure-by-default (restricted filesystem permissions, no plaintext key logging).
- Do not silently regenerate identity when key file is malformed/corrupted; surface explicit warning and keep server in local-only mode.
- Startup log lines must include peer ID and listen addresses for operator troubleshooting.
- Keep story scope strict: no DHT registration, bootstrap peers, or gossip mesh logic in 3.1.

### Library & Framework Requirements

- Use `rust-libp2p` v0.56.x with Tokio runtime support.
- Use libp2p identity encode/decode APIs for key persistence (`to_protobuf_encoding` / `from_protobuf_encoding`).
- Keep existing Rust stack conventions: `Result<_, AppError>` patterns where surfaced, `tracing` structured logs, `sqlx` compatibility untouched.

### File Structure Requirements

Expected implementation touch points:

- `server/Cargo.toml`
- `server/src/lib.rs`
- `server/src/main.rs`
- `server/src/config/settings.rs`
- `server/src/config/mod.rs` (if exports change)
- `server/src/p2p/mod.rs` (new)
- `server/src/p2p/identity.rs` (new)
- `server/src/p2p/node.rs` (new)
- `server/tests/*.rs` (new or updated integration coverage)
- `config.example.toml`

### Testing Requirements

- `cd server && cargo fmt --check`
- `cd server && cargo clippy -- -D warnings`
- `cd server && cargo test`
- Add at least one integration scenario where P2P bootstrap failure still allows `/healthz` and `/readyz` to function.

### Previous Story Intelligence

- This is Story **3.1**, so there is no earlier story in Epic 3 to mine for implementation learnings.
- Reuse established patterns from completed Epic 1/2 work:
  - config loading/validation through `server/src/config/settings.rs`,
  - structured startup logging in `server/src/main.rs`,
  - SQLite/Postgres compatibility discipline in server code and tests.

### Latest Technical Information

1. rust-libp2p **v0.56.0** is the active stable line and is Tokio-first (async-std support removed), matching this repo's Tokio runtime stack.  
2. Recommended key persistence path is libp2p identity protobuf encoding with secure file handling so PeerId remains stable across restarts.  
3. Kademlia address propagation typically needs Identify/manual address wiring; keep this in mind for Story 3.2, but do not implement it in 3.1.

### Project Context Reference

- No `project-context.md` file discovered via workflow pattern `**/project-context.md`.
- Planning artifacts (`epics.md`, `architecture.md`, `prd.md`, `ux-design-specification.md`) are the authoritative context for this story.

### Story Completion Status

- Ultimate context engine analysis completed - comprehensive developer guide created.
- Story marked `ready-for-dev` for implementation handoff.

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Story 3.1: libp2p Node Bootstrap and Instance Identity]
- [Source: _bmad-output/planning-artifacts/epics.md#Epic 3: P2P Discovery & Federation Foundation]
- [Source: _bmad-output/planning-artifacts/prd.md#Functional Requirements]
- [Source: _bmad-output/planning-artifacts/prd.md#P2P & Distributed System Constraints]
- [Source: _bmad-output/planning-artifacts/architecture.md#Core Architectural Decisions]
- [Source: _bmad-output/planning-artifacts/architecture.md#Project Structure & Boundaries]
- [Source: _bmad-output/planning-artifacts/architecture.md#Implementation Patterns & Consistency Rules]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Core User Experience]
- [Source: server/src/main.rs]
- [Source: server/src/config/settings.rs]
- [Source: server/src/lib.rs]
- [Source: server/src/handlers/instance.rs]
- [Source: server/migrations/0002_instance_and_admin.sql]
- [Source: https://github.com/libp2p/rust-libp2p/releases]
- [Source: https://libp2p.io/releases/2025-06-28-rust-libp2p/]
- [Source: https://libp2p.github.io/rust-libp2p/libp2p_identity/index.html]
- [Source: https://docs.rs/libp2p/latest/libp2p/]

## Dev Agent Record

### Agent Model Used

GPT-5.3-Codex (Copilot CLI 0.0.418)

### Debug Log References

- `cd server && cargo fmt --check && cargo clippy -- -D warnings && cargo test` (pass)
- `cd server && cargo test -q` after P2P module wiring (pass)

### Completion Notes List

- Added `libp2p` 0.56 baseline with Tokio/TCP/Noise/Yamux features and introduced validated `[p2p]` runtime config defaults.
- Implemented persistent instance identity in `server/src/p2p/identity.rs` using libp2p protobuf encoding/decoding with restrictive key file permissions.
- Added P2P bootstrap/runtime scaffolding in `server/src/p2p/node.rs` and wired startup lifecycle in `main.rs` with graceful local-only fallback on bootstrap errors.
- Added lightweight shared app-state metadata surface (`peer_id`, `listen_addrs`) for future Epic 3 stories without adding discovery behaviors.
- Added config, identity, and integration coverage including forced P2P startup failure while `/healthz` and `/readyz` remain available.

### File List

- `config.example.toml`
- `server/Cargo.toml`
- `server/Cargo.lock`
- `server/src/config/mod.rs`
- `server/src/config/settings.rs`
- `server/src/lib.rs`
- `server/src/main.rs`
- `server/src/p2p/mod.rs`
- `server/src/p2p/identity.rs`
- `server/src/p2p/node.rs`
- `server/src/middleware/auth.rs`
- `server/src/handlers/auth.rs`
- `server/src/handlers/users.rs`
- `server/src/handlers/instance.rs`
- `server/src/handlers/admin.rs`
- `server/src/handlers/health.rs`
- `server/tests/server_binds_to_configured_port.rs`
- `_bmad-output/implementation-artifacts/3-1-libp2p-node-bootstrap-and-instance-identity.md`
- `_bmad-output/implementation-artifacts/sprint-status.yaml`

### Senior Developer Review (AI)

- Reviewer: Darko
- Date: 2026-02-25
- Outcome: Changes requested and fixed in this pass

Findings addressed:
1. `server/src/config/settings.rs`: P2P validation executed even when `p2p.enabled = false`, which could block startup for local-only mode due to irrelevant P2P config values.
2. `server/src/config/settings.rs`: No guard prevented `p2p.listen_port` from matching `server.port`, which can make HTTP bind fail despite successful P2P bootstrap.
3. `Dev Agent Record -> File List`: `server/Cargo.lock` was changed in git but missing from the documented file list.

Fixes applied:
- Gated P2P validation and key-path directory checks behind `p2p.enabled`.
- Added validation rejecting `p2p.listen_port == server.port`.
- Added config tests for port conflict and disabled-P2P validation behavior.
- Updated File List to include `server/Cargo.lock`.

## Change Log

- 2026-02-25: Implemented Story 3.1 P2P identity/bootstrap foundation, added tests, and moved story to review.
- 2026-02-25: Senior code review completed; fixed P2P validation edge cases and synchronized file list documentation.
