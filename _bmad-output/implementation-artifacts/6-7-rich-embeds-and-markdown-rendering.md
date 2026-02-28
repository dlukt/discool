# Story 6.7: Rich Embeds and Markdown Rendering

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a **user**,
I want messages to render markdown formatting and show link previews,
so that conversations are visually rich and informative.

## Acceptance Criteria

1. **Given** a user sends a message containing markdown or URLs  
   **When** the message is rendered  
   **Then** markdown is rendered: **bold**, *italic*, `inline code`, fenced code blocks (with syntax highlighting), ~~strikethrough~~, links, and blockquotes.

2. **Given** a rendered code block  
   **When** code is displayed  
   **Then** the block uses the monospace system font stack.

3. **Given** a message contains URLs  
   **When** the message is rendered  
   **Then** URLs are automatically converted to clickable links.

4. **Given** a message contains a shared URL  
   **When** the message is stored and rendered  
   **Then** a compact embed preview (title, description, thumbnail) is fetched server-side and displayed.

5. **Given** embed metadata is fetched  
   **When** equivalent URLs are shared again  
   **Then** metadata is reused from cache to avoid repeated remote fetches.

6. **Given** a user selects text in the message input  
   **When** they use the markdown toolbar or shortcuts  
   **Then** formatting controls are available for bold (Ctrl+B), italic (Ctrl+I), and code (Ctrl+E).

7. **Given** message content or embed metadata is rendered  
   **When** HTML is produced  
   **Then** output is sanitized to prevent XSS (NFR14).

## Tasks / Subtasks

- [x] Task 1: Add embed persistence and cache schema (AC: 4, 5)
  - [x] Add migration `server/migrations/0022_create_message_embeds.sql` for message embed records linked to `messages(id)` with cascade delete.
  - [x] Add migration table/indexes for URL-level embed cache keyed by normalized URL to avoid repeated remote fetches.
  - [x] Add model module(s) (`server/src/models/message_embed.rs`) and export via `server/src/models/mod.rs`.

- [x] Task 2: Implement server-side URL extraction, metadata fetch, and cache flow (AC: 4, 5, 7)
  - [x] Add embed service logic in `server/src/services/` (new module) and integrate through `message_service`.
  - [x] Extract URLs from normalized message text, fetch metadata server-side, and persist message-embed associations.
  - [x] Enforce SSRF-safe fetch rules (scheme allowlist, loopback/private-network denylist, timeout, payload-size cap).
  - [x] Sanitize embed fields before persistence/response (title, description, image URL fields).

- [x] Task 3: Extend message API/WS payloads for embeds (AC: 4, 5)
  - [x] Extend `MessageResponse` with embed payload (`snake_case`) while preserving existing fields.
  - [x] Ensure embed payload appears consistently for REST history and websocket `message_create`/`message_update`.
  - [x] Update client wire transforms (`camelCase` mapping) in `client/src/lib/features/chat/types.ts` and `messageStore.svelte.ts`.

- [x] Task 4: Implement markdown rendering in message UI (AC: 1, 2, 3, 7)
  - [x] Introduce markdown render utility for `MessageBubble` with support for required syntax (bold/italic/inline code/fenced code/strikethrough/links/blockquote).
  - [x] Add code-block syntax highlighting and enforce monospace stack styling for code blocks.
  - [x] Preserve virtualized rendering performance and avoid per-frame expensive re-parsing for unchanged messages.
  - [x] Sanitize rendered HTML before insertion into DOM.

- [x] Task 5: Render compact embed preview cards (AC: 4, 5, 7)
  - [x] Render embed cards in `MessageBubble.svelte` (title, description, thumbnail, domain/source).
  - [x] Handle missing fields gracefully (thumbnail/title/description optional) without layout breakage.
  - [x] Keep links safe (`rel="noopener noreferrer"`) and prevent unsafe protocols.

- [x] Task 6: Add input selection toolbar + keyboard shortcuts (AC: 6)
  - [x] Add selection-aware markdown toolbar in composer flow inside `MessageArea.svelte`.
  - [x] Implement Ctrl+B / Ctrl+I / Ctrl+E insertion wrappers around selected text in textarea.
  - [x] Preserve existing composer behaviors (Enter send, Shift+Enter newline, attachment flow, edit flow).

- [x] Task 7: Add regression coverage and run quality gates (AC: all)
  - [x] Server tests for URL extraction, embed fetch/cache behavior, sanitization, and payload shape compatibility.
  - [x] Client tests for markdown rendering, autolink behavior, embed cards, toolbar actions, and keyboard shortcuts.
  - [x] Validate no regressions in existing messaging, reactions, and attachments behavior.
  - [x] Run:
    - [x] `cd client && npm run lint && npm run check && npm run test && npm run build`
    - [x] `cd server && cargo fmt --check && cargo clippy -- -D warnings && RUST_TEST_THREADS=1 cargo test`

## Dev Notes

### Developer Context

- Current message payloads already include attachments and reactions but not embed objects.
- Message content normalization in `message_service` currently escapes HTML before storage; markdown support should build on this without introducing unsafe render paths.
- `MessageBubble.svelte` currently renders message text as plain text (`<p>`), so markdown parsing/rendering is net-new.
- `MessageArea.svelte` owns composer behavior and already handles attachment UI; markdown selection toolbar and shortcuts should integrate there.
- Message history and realtime fanout already flow through `messageStore.svelte.ts` and websocket `message_create` events.

### Technical Requirements

- Keep backend layering (`handlers -> services -> models`) and avoid business logic in handlers.
- Keep JSON API/WS contracts explicit and backwards-compatible (`snake_case` on wire, `camelCase` in client transforms).
- Do not store rendered HTML; store normalized source message text and derive safe render output at display time.
- Embed fetch path must be bounded and safe (timeouts, response size limits, protocol filtering, network target restrictions).
- Preserve existing permission checks for viewing/sending messages; embed enrichment must not bypass authorization.
- Preserve virtual timeline performance requirements (60fps target with large message history).

### Architecture Compliance

1. Continue using `handlers/messages.rs` + `services/message_service.rs` + `models/*` separation.
2. Preserve websocket envelope and operation naming conventions already used by `ws/protocol.rs`.
3. Keep history/live payload schema aligned so REST and websocket surfaces serialize embeds identically.
4. Follow existing sanitization/security posture (escaped/sanitized user content, safe error responses, no internal leak).
5. Keep frontend state boundaries (`messageStore` for chat timeline state, Svelte components for rendering/UI only).

### Library & Framework Requirements

- Current project baseline:
  - `svelte`: `^5.45.2`
  - `axum`: `0.8`
  - `sqlx`: `0.8`
- Candidate markdown/embed libraries for this story (evaluate and pin deliberately if added):
  - `marked` latest: `17.0.3`
  - `dompurify` latest: `3.3.1`
  - `highlight.js` latest: `11.11.1`
  - `reqwest` latest: `0.13.2`
  - `scraper` latest: `0.25.0`
  - `ammonia` latest: `4.1.2`
- Prefer reusing existing sanitization patterns where possible; only add new dependencies when they materially reduce risk/complexity.

### File Structure Requirements

Expected primary touch points:

- Server
  - `server/migrations/0022_create_message_embeds.sql` (new)
  - `server/src/models/message_embed.rs` (new)
  - `server/src/models/mod.rs`
  - `server/src/services/embed_service.rs` (new)
  - `server/src/services/mod.rs`
  - `server/src/services/message_service.rs`
  - `server/src/handlers/messages.rs`
  - `server/tests/server_binds_to_configured_port.rs`
  - `server/Cargo.toml` (if new crates are introduced)

- Client
  - `client/src/lib/features/chat/types.ts`
  - `client/src/lib/features/chat/messageStore.svelte.ts`
  - `client/src/lib/features/chat/MessageBubble.svelte`
  - `client/src/lib/features/chat/MessageArea.svelte`
  - `client/src/lib/features/chat/MessageBubble.test.ts`
  - `client/src/lib/features/chat/MessageArea.test.ts`
  - `client/src/lib/features/chat/messageStore.test.ts`
  - `client/package.json` (if new packages are introduced)

### Testing Requirements

- Server integration coverage should verify:
  - message create/history includes embeds for URL messages,
  - server-side embed fetching behavior and cache reuse,
  - sanitization behavior for embed metadata and message output.
- Server unit coverage should verify:
  - URL extraction normalization and deduplication rules,
  - fetch guardrails (timeouts, private IP rejection, invalid URLs),
  - embed serialization shape is stable.
- Client coverage should verify:
  - markdown rendering matrix for required syntax,
  - clickable autolinks and embed card rendering states,
  - toolbar/shortcut formatting behavior on selected text,
  - compatibility with existing attachments and reactions rendering.

### Previous Story Intelligence

- Story 6.6 established attachment hydration and payload expansion patterns in `message_service` + chat store; embed payloads should follow the same consistency model.
- Story 6.6 reinforced persist-before-broadcast and handler/service separation; embed generation should retain this boundary.
- Story 6.5 reaction updates already demonstrated message payload enrichment and timeline reconciliation patterns.
- Story 6.3 established cursor pagination and virtualization constraints; markdown rendering must not degrade timeline behavior.

### Git Intelligence Summary

Recent commit conventions and sequence for Epic 6:

- `b73f308` feat: finalize story 6-6 file upload and sharing
- `46550ff` feat: finalize story 6-5 emoji reactions
- `bfe2ad1` feat: finalize story 6-4 edit/delete own messages
- `8741b54` feat: finalize story 6-3 message history
- `3bff024` feat: finalize story 6-2 messaging and review

### Latest Technical Information

- NPM registry latest:
  - `marked`: `17.0.3`
  - `dompurify`: `3.3.1`
  - `highlight.js`: `11.11.1`
- docs.rs latest:
  - `reqwest`: `0.13.2`
  - `scraper`: `0.25.0`
  - `ammonia`: `4.1.2`

### Project Context Reference

- No `project-context.md` file was discovered via `**/project-context.md`.
- Story context is derived from planning artifacts, implementation artifacts, current codebase patterns, and recent git history.

### Story Completion Status

- Story 6.7 rich embeds and markdown rendering is implemented across server, client, and test layers.
- Story status transitioned to `review` after passing the full client and server quality gates.

### Project Structure Notes

- Keep message evolution centered in `message_service`; avoid splitting embed logic into ad-hoc handler/database paths.
- Keep chat feature boundaries stable: API/wire transforms in `types.ts` + `messageStore.svelte.ts`, rendering in `MessageBubble.svelte`, composer interactions in `MessageArea.svelte`.
- Preserve existing operational conventions for error envelopes and websocket operation names.

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Story 6.7: Rich Embeds and Markdown Rendering]
- [Source: _bmad-output/planning-artifacts/epics.md#Epic 6: Real-Time Text Communication]
- [Source: _bmad-output/planning-artifacts/prd.md#Text Communication]
- [Source: _bmad-output/planning-artifacts/prd.md#Security]
- [Source: _bmad-output/planning-artifacts/prd.md#Domain-Specific Requirements]
- [Source: _bmad-output/planning-artifacts/architecture.md#API & Communication Patterns]
- [Source: _bmad-output/planning-artifacts/architecture.md#Implementation Patterns & Consistency Rules]
- [Source: _bmad-output/planning-artifacts/architecture.md#State Boundaries (Frontend)]
- [Source: _bmad-output/planning-artifacts/architecture.md#Requirements to Structure Mapping]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#MessageBubble]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#MessageInput]
- [Source: _bmad-output/implementation-artifacts/6-6-file-upload-and-sharing.md]
- [Source: _bmad-output/implementation-artifacts/sprint-status.yaml]
- [Source: server/src/services/message_service.rs]
- [Source: server/src/handlers/messages.rs]
- [Source: server/src/ws/protocol.rs]
- [Source: client/src/lib/features/chat/MessageBubble.svelte]
- [Source: client/src/lib/features/chat/MessageArea.svelte]
- [Source: client/src/lib/features/chat/messageStore.svelte.ts]
- [Source: client/src/lib/features/chat/types.ts]
- [Source: client/package.json]
- [Source: server/Cargo.toml]
- [Source: https://registry.npmjs.org/marked/latest]
- [Source: https://registry.npmjs.org/dompurify/latest]
- [Source: https://registry.npmjs.org/highlight.js/latest]
- [Source: https://docs.rs/crate/reqwest/latest]
- [Source: https://docs.rs/crate/scraper/latest]
- [Source: https://docs.rs/crate/ammonia/latest]

## Dev Agent Record

### Agent Model Used

GPT-5.3-Codex (model ID: gpt-5.3-codex)

### Debug Log References

- `_bmad/core/tasks/workflow.xml` loaded and executed with `_bmad/bmm/workflows/4-implementation/dev-story/workflow.yaml`.
- Story `6-7-rich-embeds-and-markdown-rendering` moved from `ready-for-dev` to `in-progress`, then to `review` after completion.
- Validation run completed successfully:
  - `cd client && npm run lint && npm run check && npm run test && npm run build`
  - `cd server && cargo fmt --check && cargo clippy -- -D warnings && RUST_TEST_THREADS=1 cargo test`

### Completion Notes List

- Added embed persistence/cache schema and model/service support for URL metadata extraction, SSRF-safe fetch, sanitization, and cache reuse.
- Extended message REST/WS payloads and client wire mappings to include embeds consistently for live and history paths.
- Implemented markdown rendering with safe sanitization, syntax highlighting, compact embed cards, and composer formatting toolbar/shortcuts.
- Added/updated server and client tests and verified all quality gates pass.

### File List

- server/migrations/0022_create_message_embeds.sql
- server/src/models/message_embed.rs
- server/src/models/message.rs
- server/src/models/mod.rs
- server/src/services/embed_service.rs
- server/src/services/mod.rs
- server/src/services/message_service.rs
- server/tests/server_binds_to_configured_port.rs
- server/Cargo.toml
- server/Cargo.lock
- client/src/lib/features/chat/messageMarkdown.ts
- client/src/lib/features/chat/types.ts
- client/src/lib/features/chat/messageStore.svelte.ts
- client/src/lib/features/chat/MessageBubble.svelte
- client/src/lib/features/chat/MessageArea.svelte
- client/src/lib/features/chat/MessageBubble.test.ts
- client/src/lib/features/chat/MessageArea.test.ts
- client/src/lib/features/chat/messageStore.test.ts
- client/package.json
- client/package-lock.json
- _bmad-output/implementation-artifacts/6-7-rich-embeds-and-markdown-rendering.md
- _bmad-output/implementation-artifacts/sprint-status.yaml

### Change Log

- 2026-02-28: Implemented Story 6.7 rich embeds and markdown rendering end-to-end and passed full client/server quality gates.
- 2026-02-28: Senior Developer Review (AI) completed in YOLO mode; fixed an SSRF safeguard gap for IPv4-mapped IPv6 targets in `server/src/services/embed_service.rs`, re-ran full quality gates, and moved story to `done`.
- 2026-02-28: Follow-up YOLO code review fixed DNS-rebinding-resistant embed fetch resolution and removed embed metadata double-escaping, then re-ran quality gates.
- 2026-02-28: YOLO code review identified/fixed concurrency consistency and resilience gaps in message/embed sync paths, tightened client-side local/private URL blocking, expanded server IP denylist safeguards, and re-ran full quality gates.
- 2026-02-28: Final YOLO code review found no new actionable code defects and synced Dev Agent File List with git by adding `server/src/models/message.rs`.

### Senior Developer Review (AI)

- Reviewer: Darko (GPT-5.3-Codex)
- Date: 2026-02-28
- Outcome: **Approve**
- Findings:
  - Fixed 1 HIGH issue in `server/src/services/embed_service.rs`: SSRF guard logic could classify IPv4-mapped IPv6 addresses (for example `::ffff:127.0.0.1`) as public and allow internal fetch targets.
  - Added regression assertions to ensure mapped loopback/private IPv4 addresses are rejected while mapped public IPv4 addresses remain allowed.
  - Fixed 1 HIGH issue in `server/src/services/embed_service.rs`: embed fetch DNS checks were vulnerable to rebinding between validation and request resolution; fetches now pin host resolution to prevalidated public addresses.
  - Fixed 1 MEDIUM issue in `server/src/services/embed_service.rs`: embed title/description fields were HTML-escaped before storage, causing double-escaped UI output; sanitized plain text is now stored.
  - Hardened markdown link rendering in `client/src/lib/features/chat/messageMarkdown.ts` by escaping link label text before interpolation.
  - No remaining actionable HIGH or MEDIUM issues after fix and re-review.
  - Acceptance Criteria and completed task claims were cross-checked against implementation and test coverage.
- Validation: `cd client && npm run lint && npm run check && npm run test && npm run build && cd ../server && cargo fmt --check && cargo clippy -- -D warnings && RUST_TEST_THREADS=1 cargo test`
  - Follow-up review/fix cycle:
    - Fixed 2 HIGH issues in `server/src/services/message_service.rs` + `server/src/models/message.rs`:
      - Message update now uses optimistic concurrency (`updated_at` match) to prevent concurrent edit races from producing mismatched message content vs embed set.
      - Embed sync failures are now handled best-effort with warning logs so message create/update/attachment writes do not return false-failure responses after successful persistence.
    - Fixed 2 MEDIUM issues:
      - `client/src/lib/features/chat/messageMarkdown.ts` now blocks local/private URL targets (localhost, private/reserved IPv4, local IPv6 scopes) for markdown links and embed links/thumbnails.
      - `server/src/services/embed_service.rs` now rejects additional non-public IPv4 ranges (CGNAT/shared, benchmarking, this-network, reserved) in SSRF safeguards.
    - Added regression coverage in `client/src/lib/features/chat/MessageBubble.test.ts` and `server/src/services/embed_service.rs` tests.
    - Re-review result: no remaining actionable HIGH or MEDIUM findings.
    - Final YOLO re-review result: no actionable HIGH or MEDIUM code findings; one MEDIUM documentation discrepancy was fixed by adding `server/src/models/message.rs` to the File List.
