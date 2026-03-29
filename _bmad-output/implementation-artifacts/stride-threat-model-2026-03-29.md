# Discool STRIDE Threat Model and OWASP Security Sweep

Date: 2026-03-29

## Scope

- Svelte SPA client served from the Rust backend (`client/src/main.ts`)
- Axum HTTP + WebSocket server (`server/src/handlers/mod.rs`)
- SQLx-backed persistence, local avatar/attachment storage, libp2p, and WebRTC (`server/src`)

## System Summary

- The SPA is embedded into the Rust server and served alongside the API and `/ws` endpoint (`server/src/handlers/mod.rs:35-269`, `server/src/static_files.rs`).
- Authentication is DID + challenge/response based; session tokens are stored in the database and sent as bearer tokens (`server/src/services/auth_service.rs:23-77`, `server/src/middleware/auth.rs:17-54`).
- The client persists the session in `localStorage` under `discool-session` (`client/src/lib/features/identity/identityStore.svelte.ts:36-56`).
- The app stores message attachments and avatars on the local filesystem (`server/src/services/file_storage_service.rs:41-80`).
- Optional `/metrics` exposure is controlled by config, but the route itself is unauthenticated when enabled (`server/src/handlers/mod.rs:234-260`).

## Key Assets

1. Session tokens and authenticated WebSocket connections
2. User DID identities, encrypted recovery material, and recovery-email encryption secret
3. User content: messages, DMs, moderation reports, attachments, avatars
4. Admin-only health and backup capabilities
5. Database contents and backups
6. P2P identity key and live federation metadata
7. Operational metadata exposed via `/metrics`, `/healthz`, `/readyz`, and logs

## Trust Boundaries

1. **Browser ↔ HTTP API**: untrusted client input crosses into authenticated REST handlers.
2. **Browser ↔ WebSocket gateway**: persistent bidirectional channel with rate-limited client operations (`server/src/ws/gateway.rs:34-35`).
3. **Server ↔ Database**: sessions, user data, moderation data, and instance configuration persist here.
4. **Server ↔ Filesystem**: local storage for attachments, avatars, and P2P identity keys.
5. **Server ↔ Email provider**: recovery and verification workflows depend on SMTP configuration.
6. **Server ↔ Peer network**: libp2p traffic crosses an external trust boundary when federation is enabled.
7. **Operator ↔ Deployment config**: security posture depends on configuration such as TLS termination, metrics enablement, and email secret choice.

## Attack Surfaces

- Open auth endpoints: `/api/v1/auth/register`, `/challenge`, `/verify`
- Authenticated REST API and moderation/admin routes
- WebSocket endpoint `/ws`
- Static file serving and client-side markdown rendering
- Attachment/avatar multipart upload endpoints
- Optional `/metrics` endpoint
- Backup creation endpoint invoking database backup logic
- libp2p and WebRTC listeners when enabled

## STRIDE Analysis

| Category | Primary Threats | Notes / Evidence |
|---|---|---|
| **Spoofing** | Stolen bearer tokens let attackers impersonate users; admin gate is currently username-based rather than a dedicated admin credential model. | `client/src/lib/features/identity/identityStore.svelte.ts:36-56`, `server/src/handlers/admin.rs:50-86` |
| **Tampering** | Multipart metadata and other user-controlled payloads attempt to alter stored content or moderation state; filesystem writes rely on validated storage keys. | `server/src/services/file_storage_service.rs:46-100` |
| **Repudiation** | Weak auditability for admin access because admin authorization is a pre-auth stub and not strongly bound to a dedicated role/credential. | `server/src/handlers/admin.rs:78-86` |
| **Information Disclosure** | Bearer tokens can traverse plaintext links if TLS is not terminated externally; `/metrics` can expose internals when enabled; `localStorage` increases XSS blast radius. | `server/src/middleware/auth.rs:24-42`, `server/src/handlers/mod.rs:234-260`, `client/src/lib/features/identity/identityStore.svelte.ts:49-56` |
| **Denial of Service** | WebSocket floods, dependency-driven ReDoS, repeated auth attempts, and peer/network abuse can consume resources; some rate limits already exist. | `server/src/ws/gateway.rs:34-35`, `npm audit` output for `picomatch`, `server/src/services/auth_service.rs`, `server/src/p2p` |
| **Elevation of Privilege** | Admin-only routes and metrics exposure rely heavily on deployment/configuration discipline; token theft or weak admin checks can extend user privileges. | `server/src/handlers/admin.rs`, `server/src/handlers/mod.rs:237-260` |

## Potential Abuse Paths

1. Inject script via a future XSS bug or compromised dependency, steal `discool-session` from `localStorage`, then reuse the bearer token against REST and `/ws`.
2. Intercept unencrypted traffic if the app is deployed without TLS termination, capturing bearer tokens or recovery links in transit.
3. Query `/metrics` after an operator enables it, extracting operational internals for reconnaissance.
4. Abuse the admin stub by compromising the configured admin account and reaching backup/health endpoints with username-based authorization.
5. Exploit vulnerable dependency behavior in the frontend toolchain (current `picomatch` advisory) to degrade tooling or CI execution.
6. Leave the example `email.server_secret` unchanged, causing recovery-email encryption to share a public default secret across deployments.

## OWASP-Style Security Sweep

### 1. High — Missing in-process TLS enforcement / bearer-token transport risk

- **Threat model link**: Information disclosure, spoofing
- **Evidence**: Authentication relies on bearer tokens in the `Authorization` header (`server/src/middleware/auth.rs:24-42`), while server config exposes host/port only and does not implement TLS termination (`server/src/config/settings.rs`, `server/src/main.rs:84-118`).
- **Impact**: A deployment mistake can expose session tokens and recovery links over plaintext HTTP or WS.
- **Mitigations**:
  - Require TLS termination in front of the app and document it as mandatory production guidance.
  - Optionally reject non-loopback startup in production unless a proxy/TLS setting is explicitly acknowledged.
  - Consider secure cookies for browser sessions if the architecture evolves beyond bearer tokens.

### 2. High — Dependency security finding in frontend toolchain (`picomatch`)

- **Threat model link**: Denial of service / supply-chain risk
- **Evidence**: `npm audit --audit-level=high` reports `GHSA-3v7f-55p6-f55p` and `GHSA-c2c7-rcm5-vvqj` against `picomatch 4.0.0 - 4.0.3`.
- **Impact**: Maliciously crafted glob patterns can trigger incorrect matching or ReDoS in affected execution paths.
- **Mitigations**:
  - Update the affected dependency chain with `npm audit fix` once compatibility is confirmed.
  - Keep lockfile-based CI auditing enabled.

### 3. Medium — Session token persisted in `localStorage`

- **Threat model link**: Spoofing, information disclosure, elevation of privilege
- **Evidence**: `discool-session` is written to and read from `localStorage` (`client/src/lib/features/identity/identityStore.svelte.ts:36-56`, `93-130`, `149-152`).
- **Impact**: Any XSS or malicious extension can exfiltrate a live bearer token.
- **Mitigations**:
  - Prefer `HttpOnly`, `Secure`, `SameSite` cookies if compatible with the auth model.
  - Keep the CSP strict and continue sanitizing rendered markdown (`server/src/handlers/mod.rs:31-33`, `client/src/lib/features/chat/messageMarkdown.ts:41-60`).
  - Shorten session lifetime or add device/session binding for defense in depth.

### 4. Medium — `/metrics` becomes public when enabled

- **Threat model link**: Information disclosure
- **Evidence**: The router exposes `/metrics` without auth when metrics are enabled (`server/src/handlers/mod.rs:234-260`).
- **Impact**: Metrics can leak internal topology, traffic patterns, and capacity data to any reachable client.
- **Mitigations**:
  - Protect `/metrics` behind network policy, reverse-proxy auth, or an internal-only bind.
  - Consider adding an app-level auth or shared secret guard if public deployment is common.

### 5. Medium — Admin authorization is still a stub

- **Threat model link**: Spoofing, repudiation, elevation of privilege
- **Evidence**: Admin access is granted by matching the authenticated username against the first row in `admin_users`, and the code itself marks this as a TODO (`server/src/handlers/admin.rs:50-86`, `175-186`).
- **Impact**: The model is brittle, difficult to audit, and weaker than explicit RBAC or separate admin credentials.
- **Mitigations**:
  - Replace the username check with a first-class admin role/permission model.
  - Add stronger audit logging for admin actions and consider step-up authentication for backups.

### 6. Low — Example recovery-email server secret can be left unchanged

- **Threat model link**: Information disclosure / configuration weakness
- **Evidence**: The default config uses `change-me-in-production` for `email.server_secret` (`server/src/config/settings.rs:964-966`, `config.example.toml:232-234`).
- **Impact**: If an operator deploys with the example secret, recovery-email encryption strength collapses to a known shared secret.
- **Mitigations**:
  - Fail startup or warn loudly when the example secret is used.
  - Generate and document a unique per-instance secret.
- **Status**: Addressed in this change by adding a startup warning and clarifying the config example.

## Existing Positive Controls

- Strict CSP and defensive security headers (`server/src/handlers/mod.rs:31-33`, `280-299`)
- One-time auth challenge consumption (`server/src/services/auth_service.rs:66-76`)
- Filesystem storage-key validation (`server/src/services/file_storage_service.rs:83-100`)
- Markdown sanitization and external URL safety checks (`client/src/lib/features/chat/messageMarkdown.ts:41-60`, `71-153`)
- WebSocket rate limiting (`server/src/ws/gateway.rs:34-35`)

## Residual Risk Summary

The strongest remaining risks are deployment/configuration-dependent: transport security, public metrics exposure, and the current admin authorization model. The least-priority actionable issue was the example `email.server_secret`; this review addresses it with a warning so insecure deployments are easier to detect before recovery email is enabled in production.
