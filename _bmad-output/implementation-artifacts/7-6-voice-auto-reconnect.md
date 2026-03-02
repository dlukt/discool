# Story 7.6: Voice Auto-Reconnect

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a **user**,
I want to automatically reconnect to voice after a brief connection loss,
so that a network hiccup doesn't permanently drop me from the conversation.

## Acceptance Criteria

1. **Given** a user is in a voice channel and their network briefly drops  
   **When** the WebRTC connection is lost  
   **Then** the VoiceBar shows "Reconnecting..." with a pulsing connection indicator.

2. **Given** voice reconnect is in progress  
   **When** retries are attempted  
   **Then** the client automatically attempts to re-establish voice with exponential backoff.

3. **Given** reconnect succeeds within 5 seconds  
   **When** media/session is restored  
   **Then** the user is back in the same voice channel with no manual action required (NFR26).

4. **Given** reconnect attempts exceed 5 seconds  
   **When** the user is still disconnected  
   **Then** the VoiceBar shows "Connection lost" and retry attempts continue.

5. **Given** reconnect attempts continue for the terminal timeout window (30 seconds)  
   **When** reconnect still fails  
   **Then** the user is fully disconnected and the VoiceBar disappears.

6. **Given** voice reconnect is underway or has failed  
   **When** the user continues using chat  
   **Then** text chat remains functional and non-blocked (NFR28).

7. **Given** a user drops and reconnects to voice  
   **When** participant state changes occur  
   **Then** other users in the channel receive real-time `voice_state_update` transitions reflecting the reconnect lifecycle.

## Tasks / Subtasks

- [x] Task 1: Implement reconnect state-machine in voice store (AC: 1, 2, 3, 4, 5, 6)
  - [x] Extend `voiceStore.svelte.ts` with reconnect lifecycle state that preserves active voice context during transient disconnects.
  - [x] Add bounded reconnect window controls: fast-recovery threshold (5s) and terminal timeout (30s).
  - [x] Reuse and harden existing stale-context protections (`matchesActiveContext`, timer cleanup, context-bound callbacks) so old async events cannot corrupt active reconnect attempts.
  - [x] Keep retry scheduling exponential and deterministic (aligned with current `RETRY_INITIAL_MS` / `RETRY_MAX_MS` semantics unless intentionally revised).

- [x] Task 2: Add WebRTC reconnect hooks without breaking current signaling flow (AC: 2, 3, 4, 5)
  - [x] Extend `VoiceWebRtcClient` reconnect handling so `connectionState`/ICE transitions (`disconnected`/`failed`) can trigger reconnect progression instead of immediate idle teardown.
  - [x] Evaluate and implement safe ICE restart behavior (`restartIce()` + renegotiation path) where applicable to reduce full-session teardown churn.
  - [x] Ensure reconnect path remains compatible with existing `c_voice_join` / `voice_offer` / `c_voice_answer` / `c_voice_ice_candidate` flow.

- [x] Task 3: Implement required UX/status behavior for reconnecting voice (AC: 1, 4, 5, 6)
  - [x] Keep VoiceBar visible during reconnect attempts and expose explicit status copy transitions: `Reconnecting...` then `Connection lost`.
  - [x] Add pulsing/animated quality indicator state for reconnecting while respecting reduced-motion expectations.
  - [x] Ensure VoiceBar removal occurs only on terminal failure or explicit disconnect, not on the first transient drop.
  - [x] Preserve existing plain-language status/error principles in `MessageArea` and Voice UI surfaces.

- [x] Task 4: Preserve and verify real-time participant visibility semantics (AC: 7)
  - [x] Ensure reconnect lifecycle produces observable `voice_state_update` transitions for other participants (drop/removal and successful rejoin snapshots).
  - [x] Keep guild/channel permission checks and channel type validation unchanged for reconnect-triggered rejoin attempts.
  - [x] Avoid protocol churn unless required by AC coverage gaps; prefer existing `voice_state_update` payload contract unless a concrete gap is proven.

- [x] Task 5: Expand regression coverage for reconnect lifecycle (AC: all)
  - [x] Add `voiceStore.test.ts` coverage for transient disconnect → reconnect success (<5s), prolonged reconnect (>5s status copy), and terminal timeout disconnect (30s).
  - [x] Add/update Voice UI tests (`VoiceBar.test.ts`, `MessageArea.test.ts`) for reconnecting indicator + copy + VoiceBar persistence/removal transitions.
  - [x] Add/update server websocket integration tests for reconnect-related participant snapshot transitions in `server/tests/server_binds_to_configured_port.rs`.
  - [x] Run quality gates:
    - [x] `cd client && npm run lint && npm run check && npm run test && npm run build`
    - [x] `cd server && cargo fmt --check && cargo clippy -- -D warnings && cargo test`

## Dev Notes

### Developer Context

- Story 7.5 completed channel-switch sequencing and stale-context guards in `voiceStore`; 7.6 should build on that same context safety model rather than introducing parallel state flows.
- Current reconnect behavior is join-attempt oriented (initial connect/retry) and does not yet satisfy AC-defined voice auto-reconnect lifecycle after an established call drops.
- `ShellRoute.svelte` currently activates voice based on route/channel selection, but reconnect requirements must be handled inside voice runtime/store logic because route state may stay unchanged during a transient network drop.  
  [Source: _bmad-output/implementation-artifacts/7-5-voice-channel-switching.md#Developer Context]  
  [Source: client/src/lib/features/voice/voiceStore.svelte.ts]  
  [Source: client/src/lib/features/shell/ShellRoute.svelte]

### Technical Requirements

- Preserve NFR26: reconnect to same channel automatically within 5 seconds when recovery is possible.
- Preserve NFR28: text messaging path must continue operating during voice reconnect/retry/failure states.
- Use explicit status messaging progression tied to lifecycle milestones (`Reconnecting...` -> `Connection lost` -> terminal disconnect).
- Keep async and timer behavior race-safe; reconnect callbacks must be gated to current active context.
- Keep existing voice copy semantics and error-surface patterns from Story 7.5 unless ACs require specific additions.

### Architecture Compliance

1. Keep `voiceStore.svelte.ts` as canonical voice client state boundary.
2. Preserve JSON wire `snake_case` at API/WS boundaries.
3. Preserve websocket operation naming contracts (`c_*` for client ops, unprefixed server ops).
4. Continue using `voice_state_update` as the participant snapshot contract.
5. Preserve graceful degradation and reliability principles documented in architecture/PRD.

### Library & Framework Requirements

- Frontend: Svelte 5 + TypeScript + Vitest.
- Backend: Rust + Axum websocket gateway + existing webrtc-rs runtime.
- WebRTC reconnect implementation should respect current browser/runtime semantics:
  - `RTCPeerConnection.connectionState` and `RTCPeerConnection.iceConnectionState` transition behavior for `disconnected` vs `failed`.
  - `RTCPeerConnection.restartIce()` triggers ICE restart on next `createOffer()` cycle.
  - `Window` `online`/`offline` events are allowed as network hints but should not be treated as sole reconnect truth signals.  
  [Source: https://developer.mozilla.org/en-US/docs/Web/API/RTCPeerConnection/connectionState]  
  [Source: https://developer.mozilla.org/en-US/docs/Web/API/RTCPeerConnection/iceConnectionState]  
  [Source: https://developer.mozilla.org/en-US/docs/Web/API/RTCPeerConnection/restartIce]  
  [Source: https://developer.mozilla.org/en-US/docs/Web/API/Window/online_event]  
  [Source: https://developer.mozilla.org/en-US/docs/Web/API/Window/offline_event]

### File Structure Requirements

Expected primary touch points:

- `client/src/lib/features/voice/voiceStore.svelte.ts`
- `client/src/lib/features/voice/webrtcClient.ts`
- `client/src/lib/features/voice/VoiceBar.svelte`
- `client/src/lib/features/voice/VoiceBar.test.ts`
- `client/src/lib/features/voice/voiceStore.test.ts`
- `client/src/lib/features/chat/MessageArea.svelte`
- `client/src/lib/features/chat/MessageArea.test.ts`
- `client/src/lib/features/shell/ShellRoute.svelte` (only if reconnect lifecycle wiring requires it)
- `server/src/ws/gateway.rs`
- `server/src/webrtc/voice_channel.rs`
- `server/tests/server_binds_to_configured_port.rs`
- `config.example.toml` and/or `server/src/config/settings.rs` (if reconnect knobs are adjusted)

### Testing Requirements

- Verify transient disconnect enters reconnect mode and auto-recovers without manual rejoin.
- Verify reconnect success within 5s restores same guild/channel voice context and keeps controls stable.
- Verify reconnect past 5s shows `Connection lost` while retries continue.
- Verify terminal timeout disconnect removes VoiceBar and resets active voice context cleanly.
- Verify text chat send/read UX remains available during reconnect lifecycle (NFR28).
- Verify other participants observe reconnect lifecycle state changes via `voice_state_update` transitions.
- Verify stale reconnect timers/events from prior contexts cannot overwrite active state.

### Previous Story Intelligence

- Story 7.5 introduced deterministic switch sequencing and stale-callback rejection patterns in `voiceStore`; reuse this approach for reconnect transitions.
- Story 7.5 kept user-facing copy stable (`Could not connect to voice. Retrying...`, `Voice connection failed. Check your network.`); 7.6 should extend rather than replace this language set.
- Story 7.5 server integration already validates leave/join snapshot rebroadcast behavior; 7.6 can build additional reconnect-specific variants on top of that proven pattern.  
  [Source: _bmad-output/implementation-artifacts/7-5-voice-channel-switching.md#Previous Story Intelligence]  
  [Source: _bmad-output/implementation-artifacts/7-5-voice-channel-switching.md#Senior Developer Review (AI)]  
  [Source: server/tests/server_binds_to_configured_port.rs]

### Git Intelligence Summary

Recent voice-work trend:

- `97da3f1` feat: complete story 7-5 voice channel switching
- `759b02f` feat: complete story 7-4 individual volume control
- `fc5f2bc` chore: prepare story 7-4 individual volume control
- `a40554c` feat: complete story 7-3 voice participants display
- `3cb1816` feat: complete story 7-2 voice controls

Actionable implications:

- Continue incremental voice evolution in established modules (`voiceStore`, `webrtcClient`, `gateway`, `voice_channel`).
- Preserve test-first regression guardrail pattern used in recent voice stories.
- Avoid introducing new WS protocol shapes unless AC coverage is impossible without them.

### Latest Technical Information

- `RTCPeerConnection.restartIce()` requests ICE restart for next offer and can help recover failed candidate paths while preserving media continuity.
- `iceConnectionState: disconnected` can be transient on unstable networks and may recover spontaneously; treat it differently from terminal `failed`.
- `connectionState` remains a high-level peer lifecycle signal; reconnect logic should map these states to UX copy and retry decisions.
- Browser `online`/`offline` events provide coarse network hints and can supplement, but not replace, transport-level reconnect state decisions.  
  [Source: https://developer.mozilla.org/en-US/docs/Web/API/RTCPeerConnection/restartIce]  
  [Source: https://developer.mozilla.org/en-US/docs/Web/API/RTCPeerConnection/iceConnectionState]  
  [Source: https://developer.mozilla.org/en-US/docs/Web/API/RTCPeerConnection/connectionState]  
  [Source: https://developer.mozilla.org/en-US/docs/Web/API/Window/online_event]  
  [Source: https://developer.mozilla.org/en-US/docs/Web/API/Window/offline_event]

### Project Context Reference

- No `project-context.md` file was discovered via `**/project-context.md`.
- Story context is derived from planning artifacts, implementation artifacts, source code, git history, and web standards references.

### Story Completion Status

- Workflow analysis complete for story `7-6-voice-auto-reconnect`.
- Story document generated and prepared for implementation handoff.
- Sprint status target for this story: `ready-for-dev`.
- Completion note: Ultimate context engine analysis completed - comprehensive developer guide created.

### Project Structure Notes

- Voice lifecycle and retry logic currently lives in `voiceStore.svelte.ts` with WebRTC mechanics in `webrtcClient.ts`; this separation should be preserved.
- Existing retry defaults are documented in `config.example.toml` and validated in `server/src/config/settings.rs` (`retry_initial_millis=400`, `retry_max_millis=1600`, `retry_max_attempts=2`).
- Server gateway already rebroadcasts `voice_state_update` on leave/join transitions and websocket cleanup; reconnect behavior should preserve these semantics while extending lifecycle coverage.

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Story 7.6: Voice Auto-Reconnect]
- [Source: _bmad-output/planning-artifacts/epics.md#Epic 7: Voice Communication]
- [Source: _bmad-output/planning-artifacts/prd.md#Reliability]
- [Source: _bmad-output/planning-artifacts/prd.md#Journey 6: Liam — "Something Went Wrong" (Edge Case)]
- [Source: _bmad-output/planning-artifacts/architecture.md#State Boundaries (Frontend)]
- [Source: _bmad-output/planning-artifacts/architecture.md#WebSocket Event Naming]
- [Source: _bmad-output/planning-artifacts/architecture.md#API Boundary (JSON)]
- [Source: _bmad-output/planning-artifacts/architecture.md#Non-Functional Requirements Coverage]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Journey 5: Voice Channel Lifecycle]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Journey 6: Identity Recovery]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Feedback Patterns]
- [Source: _bmad-output/implementation-artifacts/7-5-voice-channel-switching.md]
- [Source: _bmad-output/implementation-artifacts/sprint-status.yaml]
- [Source: client/src/lib/features/voice/voiceStore.svelte.ts]
- [Source: client/src/lib/features/voice/webrtcClient.ts]
- [Source: client/src/lib/features/voice/VoiceBar.svelte]
- [Source: client/src/lib/features/chat/MessageArea.svelte]
- [Source: client/src/lib/features/shell/ShellRoute.svelte]
- [Source: client/src/lib/ws/client.ts]
- [Source: server/src/ws/gateway.rs]
- [Source: server/src/webrtc/voice_channel.rs]
- [Source: server/src/ws/protocol.rs]
- [Source: server/tests/server_binds_to_configured_port.rs]
- [Source: config.example.toml]
- [Source: https://developer.mozilla.org/en-US/docs/Web/API/RTCPeerConnection/restartIce]
- [Source: https://developer.mozilla.org/en-US/docs/Web/API/RTCPeerConnection/iceConnectionState]
- [Source: https://developer.mozilla.org/en-US/docs/Web/API/RTCPeerConnection/connectionState]
- [Source: https://developer.mozilla.org/en-US/docs/Web/API/Window/online_event]
- [Source: https://developer.mozilla.org/en-US/docs/Web/API/Window/offline_event]

## Dev Agent Record

### Agent Model Used

GPT-5.3-Codex (model ID: gpt-5.3-codex)

### Debug Log References

- Loaded workflow engine and create-story workflow config/instructions.
- Resolved target story from user input `7-6` and sprint status key `7-6-voice-auto-reconnect`.
- Analyzed epics/PRD/architecture/UX artifacts and previous Story 7.5 implementation story.
- Reviewed current voice client/server code paths and reconnect-related tests.
- Gathered current WebRTC reconnect standards references for implementation guardrails.

### Completion Notes List

- Generated implementation-ready story context with AC-mapped tasks and test guidance.
- Implemented reconnect lifecycle state-machine with 5s fast-recovery and 30s terminal timeout windows while preserving active context safety checks.
- Added reconnect-aware WebRTC hooks (ICE state handling + safe `restartIce()` trigger) and kept signaling contract compatibility (`c_voice_join`/offer/answer/ICE).
- Updated VoiceBar/MessageArea UX so reconnecting voice remains visible with explicit `Reconnecting...` → `Connection lost` transitions and reduced-motion-safe pulse indicator.
- Expanded regression coverage across `voiceStore`, `VoiceBar`, `MessageArea`, and server websocket integration for leave/rejoin snapshot rebroadcast semantics.
- Story status set to `done` after AI code-review fixes.

### File List

- client/src/lib/features/chat/MessageArea.svelte
- client/src/lib/features/chat/MessageArea.test.ts
- client/src/lib/features/voice/VoiceBar.svelte
- client/src/lib/features/voice/VoiceBar.test.ts
- client/src/lib/features/voice/voiceStore.svelte.ts
- client/src/lib/features/voice/voiceStore.test.ts
- client/src/lib/features/voice/webrtcClient.ts
- server/tests/server_binds_to_configured_port.rs
- _bmad-output/implementation-artifacts/7-6-voice-auto-reconnect.md
- _bmad-output/implementation-artifacts/sprint-status.yaml

## Senior Developer Review (AI)

- Reviewer: Darko
- Date: 2026-03-02
- Outcome: Approve (after fixes)
- Git vs Story File List Discrepancies: 0

### Findings

1. **MEDIUM** `MessageArea` rendered `VoiceBar` for all `failed` states, which surfaced reconnect-specific `Connection lost` copy even for first-time join failures (`Voice connection failed. Check your network.`).
   - **Fix applied:** restricted failed-state `VoiceBar` visibility to reconnect failures by gating on `VOICE_CONNECTION_LOST_MESSAGE`.
   - **Validation:** updated `MessageArea.test.ts` with reconnect/non-reconnect failed-state assertions.

2. **MEDIUM** reconnect success reused stale `joinStartedAt`, producing misleading `joinLatencyMs` values that represented total session age instead of reconnect behavior.
   - **Fix applied:** reset `joinStartedAt` when entering reconnect lifecycle.
   - **Validation:** added reconnect-success assertion in `voiceStore.test.ts` ensuring `joinLatencyMs` stays `null`.

### Change Log

- 2026-03-02: Implemented Story 7.6 voice auto-reconnect lifecycle and validations; status advanced to `review`.
- 2026-03-02: YOLO code review found reconnect-state UX/latency regressions, applied targeted fixes, and moved story status to `done`.
