# Story 7.1: WebRTC Signaling and Peer Connection

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a **user**,
I want to establish a voice connection when I click a voice channel,
so that I can talk with others without any setup.

## Acceptance Criteria

1. **Given** a user clicks a voice channel in the ChannelList  
   **When** the voice connection is initiated  
   **Then** the client sends a `c_voice_join` event via the existing WebSocket

2. **Given** a voice join request is accepted  
   **When** signaling starts  
   **Then** the server returns an SDP offer and ICE candidates for the SFU

3. **Given** signaling exchange is in progress  
   **When** the client processes server signaling data  
   **Then** the client completes the WebRTC handshake (offer/answer/ICE)

4. **Given** voice signaling is implemented on the backend  
   **When** the service crate is configured  
   **Then** the connection uses `webrtc-rs v0.17.x` on the server side

5. **Given** users are behind NAT  
   **When** WebRTC connection setup runs  
   **Then** STUN servers are used for NAT traversal by default (configurable in TOML)

6. **Given** direct connectivity fails  
   **When** ICE negotiation requires relay  
   **Then** TURN server relay is available as fallback (configurable in TOML)

7. **Given** media starts flowing  
   **When** transport security is evaluated  
   **Then** all voice streams are encrypted with SRTP (NFR12)

8. **Given** a user joins voice  
   **When** time is measured from click to connected  
   **Then** voice channel join completes in under 2 seconds (NFR2)

9. **Given** the connection fails on first attempt  
   **When** the UI reports state  
   **Then** it shows `"Could not connect to voice. Retrying..."` with automatic retry

10. **Given** retry also fails  
    **When** retry policy is exhausted  
    **Then** the UI shows `"Voice connection failed. Check your network."`

11. **Given** voice connection setup fails or retries  
    **When** the user keeps using text chat  
    **Then** voice connection failure does not affect text chat functionality (NFR28)

## Tasks / Subtasks

- [x] Task 1: Add voice/WebRTC configuration and runtime wiring (AC: 4, 5, 6, 8)
  - [x] Extend `server/src/config/settings.rs` with a dedicated voice/WebRTC config block (STUN URLs, optional TURN URL + credentials, join timing/retry knobs) and validation rules.
  - [x] Export new config types through `server/src/config/mod.rs`, include redacted summary logging in config startup logs, and wire state into `AppState` (`server/src/lib.rs` / `server/src/main.rs`).
  - [x] Update `config.example.toml` with all supported keys and matching `DISCOOL_*` environment variable mapping.

- [x] Task 2: Extend the WebSocket protocol contract for voice signaling (AC: 1, 2, 3)
  - [x] Add client operations in `server/src/ws/protocol.rs` and `client/src/lib/ws/protocol.ts` for `c_voice_join`, `c_voice_answer`, and `c_voice_ice_candidate`.
  - [x] Add server operation names for `voice_offer`, `voice_ice_candidate`, and `voice_connection_state` using existing `{ op, d, s, t }` envelope rules.
  - [x] Add strict payload parsing and validation errors for malformed voice signaling payloads, preserving the existing machine-readable error format.

- [x] Task 3: Implement backend signaling flow with webrtc-rs 0.17.x (AC: 2, 3, 4, 7)
  - [x] Add `server/src/webrtc/` module boundary (`mod.rs`, `signaling.rs`, `voice_channel.rs`, `turn.rs`) and register it from `server/src/lib.rs`.
  - [x] In gateway handling (`server/src/ws/gateway.rs`), validate `c_voice_join` against guild membership, `VIEW_CHANNEL`, and `channel_type == "voice"` before starting signaling.
  - [x] Create peer connection offer path on server, emit `voice_offer`, trickle `voice_ice_candidate`, and apply client `c_voice_answer` + `c_voice_ice_candidate` responses.
  - [x] Ensure connection setup uses DTLS/SRTP defaults from `webrtc-rs` and fails fast with explicit, user-safe errors when setup cannot proceed.

- [x] Task 4: Build client voice signaling and peer-connection state module (AC: 1, 3, 7, 8)
  - [x] Add `client/src/lib/features/voice/types.ts`, `webrtcClient.ts`, and `voiceStore.svelte.ts` following existing feature-store patterns.
  - [x] Implement client flow: send `c_voice_join`, consume `voice_offer`, set remote description, create/send answer, exchange ICE candidates, and monitor connection state.
  - [x] Capture connection state transitions (`connecting`, `connected`, `retrying`, `failed`) with explicit timestamps for join-latency measurement.

- [x] Task 5: Integrate voice join trigger and user feedback in shell/chat surfaces (AC: 1, 9, 10, 11)
  - [x] Trigger voice-join initiation when a voice channel becomes active from ChannelList navigation, without altering text-channel behavior.
  - [x] Surface required copy exactly: `"Could not connect to voice. Retrying..."` and `"Voice connection failed. Check your network."` using existing status/toast accessibility patterns (`aria-live`).
  - [x] Keep retries automatic and bounded for this story; do not add VoiceBar controls yet (covered by Story 7.2).

- [x] Task 6: Enforce graceful degradation and isolation from chat transport (AC: 11)
  - [x] Ensure voice failure paths do not modify or interrupt existing message timeline, DM, or WebSocket subscription flows in `messageStore`/`wsClient`.
  - [x] Ensure cleanup of failed/abandoned peer connections does not close the shared WebSocket session.

- [x] Task 7: Add AC-focused tests and run quality gates (AC: all)
  - [x] Server unit tests for protocol op parsing, signaling payload validation, channel-type gating, and voice state transitions.
  - [x] Server integration tests for authenticated `c_voice_join` flow and expected `voice_offer` / `voice_ice_candidate` events.
  - [x] Client tests for signaling handshake flow, retry-to-failure messaging, and no-regression behavior for text chat while voice fails.
  - [x] Run quality gates:
    - [x] `cd client && npm run lint && npm run check && npm run test && npm run build`
    - [x] `cd server && cargo fmt --check && cargo clippy -- -D warnings && cargo test`

## Dev Notes

### Developer Context

- Epic 7 starts from a mature real-time baseline: Story 6.1 already implemented authenticated `/ws`, sequence envelopes, lifecycle states, and reconnect handling; voice signaling must extend this path instead of creating a second transport.
- Channel primitives already support `channel_type: "voice"` in both client and server (`channelStore`, `channel_service`, `models/channel`), so Story 7.1 should reuse existing channel lookup and permission pathways.
- There is currently no server `webrtc` module and no voice signaling operations in WS protocol definitions; this story introduces that foundation for Stories 7.2–7.7.
- Current error/status UX from Story 6.11 already provides plain-language status and toast patterns; reuse those patterns for voice retry/failure copy.

### Technical Requirements

- Keep `/ws` as the signaling transport and preserve existing envelope format (`op`, `d`, `s`, `t`) with `snake_case` fields.
- Enforce `c_` prefix for all client-to-server voice operations.
- Validate join requests server-side:
  - authenticated session,
  - guild/channel exist and are visible to caller,
  - target channel type is `voice`.
- Use `webrtc-rs` `0.17.x` on backend and browser `RTCPeerConnection` on client.
- STUN is default-on and configurable; TURN is optional fallback and configurable (TOML + env overrides).
- Treat SDP/ICE payloads as signaling data transport, not business-domain payloads.
- Preserve NFR12 by relying on WebRTC DTLS/SRTP defaults; do not introduce plaintext media paths.
- Instrument click-to-connected latency to verify `< 2s` target (NFR2).
- Preserve NFR28: failures in voice setup must not cascade to text messaging flows.

### Architecture Compliance

1. Keep layering consistent with current project boundaries:
   - `handlers/ws` handles upgrade boundary,
   - `ws/gateway` routes validated protocol operations,
   - `webrtc/*` owns signaling/session mechanics.
2. Keep API and WS error payloads sanitized and machine-readable (`code`, `message`, `details`).
3. Preserve transport naming patterns from architecture (`snake_case`, `c_` prefix).
4. Keep feature-module organization on client (`features/voice/*`) and avoid scattering voice state into unrelated stores.
5. Maintain existing shared WS client as single socket authority; do not create a parallel long-lived signaling socket.

### Library & Framework Requirements

- Backend baseline (already in repo): Axum `0.8`, Tokio `1`, sqlx `0.8`.
- New backend dependency expected in this story: `webrtc` crate on `0.17.x`.
- Frontend baseline (already in repo): Svelte `^5.45.2`, `@mateothegreat/svelte5-router ^2.16.19`, Vitest `^4.0.18`.
- No additional frontend dependency is required for this story.

### File Structure Requirements

Expected primary touch points:

- Server
  - `server/Cargo.toml`
  - `server/src/config/settings.rs`
  - `server/src/config/mod.rs`
  - `server/src/lib.rs`
  - `server/src/main.rs`
  - `server/src/ws/protocol.rs`
  - `server/src/ws/gateway.rs`
  - `server/src/webrtc/mod.rs` (new)
  - `server/src/webrtc/signaling.rs` (new)
  - `server/src/webrtc/voice_channel.rs` (new)
  - `server/src/webrtc/turn.rs` (new)
  - `server/tests/server_binds_to_configured_port.rs`
  - `config.example.toml`
- Client
  - `client/src/lib/ws/protocol.ts`
  - `client/src/lib/ws/client.ts`
  - `client/src/lib/features/voice/types.ts` (new)
  - `client/src/lib/features/voice/webrtcClient.ts` (new)
  - `client/src/lib/features/voice/voiceStore.svelte.ts` (new)
  - `client/src/lib/features/shell/ShellRoute.svelte`
  - `client/src/lib/features/channel/ChannelList.svelte` (if trigger wiring is implemented here)
  - `client/src/lib/features/chat/MessageArea.svelte` (status text integration)

### Testing Requirements

- Server unit tests:
  - client op parsing for new voice ops,
  - payload validation for join/answer/candidate operations,
  - channel type + permission gating,
  - signaling state transitions and cleanup paths.
- Server integration tests:
  - authenticated `c_voice_join` initiates signaling and emits `voice_offer`,
  - candidate relay events are emitted in expected envelope shape,
  - invalid join/channel scenarios return explicit structured errors.
- Client tests:
  - joining a voice channel triggers `c_voice_join`,
  - offer/answer/candidate flow updates state to connected,
  - retry copy + terminal failure copy match AC text exactly,
  - text chat send/edit/delete behavior remains unaffected when voice setup fails.

### Latest Technical Information

- `docs.rs` for `webrtc` latest shows the current line at `0.17.1` and exposes `stun`, `turn`, `srtp`, and peer-connection APIs consistent with this story's server requirements.
- MDN signaling guidance emphasizes transporting SDP/ICE through signaling as opaque payloads and preserving negotiation order (`setRemoteDescription` before applying remote ICE candidates).
- MDN `icecandidate` and `restartIce()` guidance supports trickle-ICE handling and controlled retry strategy when ICE enters failed states.

### Project Context Reference

- No `project-context.md` file was discovered via `**/project-context.md`.
- Context for this story is derived from planning artifacts, implementation artifacts, current repository state, and targeted technical research.

### Story Completion Status

- Ultimate context engine analysis completed - comprehensive developer guide created.
- Story status set to `ready-for-dev`.

### Project Structure Notes

- Existing real-time infrastructure (`server/src/ws/*`, `client/src/lib/ws/*`) is already production-critical; Story 7.1 should extend it with voice signaling rather than bypassing it.
- Voice participant UI and controls are intentionally deferred to later stories (7.2+), so this story should focus on signaling + connection foundation.
- Current permission catalog includes voice-relevant moderation bits (`MUTE_MEMBERS`) but not explicit connect/speak bits; avoid broad permission-model expansion unless required by implementation constraints.

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Epic 7: Voice Communication]
- [Source: _bmad-output/planning-artifacts/epics.md#Story 7.1: WebRTC Signaling and Peer Connection]
- [Source: _bmad-output/planning-artifacts/prd.md#Voice Communication (FR39-FR44)]
- [Source: _bmad-output/planning-artifacts/prd.md#NFR2: Voice channel join time]
- [Source: _bmad-output/planning-artifacts/prd.md#NFR12: Voice encryption]
- [Source: _bmad-output/planning-artifacts/prd.md#NFR28: Graceful degradation]
- [Source: _bmad-output/planning-artifacts/architecture.md#API & Communication Patterns]
- [Source: _bmad-output/planning-artifacts/architecture.md#Retry/Reconnect Pattern]
- [Source: _bmad-output/planning-artifacts/architecture.md#Requirements to Structure Mapping (Voice Communication FR39-44)]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Journey 5: Voice Channel Lifecycle]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#VoiceBar]
- [Source: _bmad-output/implementation-artifacts/6-1-websocket-gateway-and-connection-management.md]
- [Source: _bmad-output/implementation-artifacts/6-11-error-handling-and-status-communication.md]
- [Source: _bmad-output/implementation-artifacts/sprint-status.yaml]
- [Source: server/src/ws/protocol.rs]
- [Source: server/src/ws/gateway.rs]
- [Source: server/src/ws/registry.rs]
- [Source: server/src/handlers/ws.rs]
- [Source: server/src/config/settings.rs]
- [Source: server/src/services/channel_service.rs]
- [Source: client/src/lib/ws/protocol.ts]
- [Source: client/src/lib/ws/client.ts]
- [Source: client/src/lib/features/channel/ChannelList.svelte]
- [Source: client/src/lib/features/chat/messageStore.svelte.ts]
- [Source: https://docs.rs/webrtc/latest/webrtc/]
- [Source: https://docs.rs/webrtc/latest/webrtc/peer_connection/struct.RTCPeerConnection.html]
- [Source: https://developer.mozilla.org/en-US/docs/Web/API/WebRTC_API/Signaling_and_video_calling]
- [Source: https://developer.mozilla.org/en-US/docs/Web/API/RTCPeerConnection/icecandidate_event]
- [Source: https://developer.mozilla.org/en-US/docs/Web/API/RTCPeerConnection/restartIce]

## Dev Agent Record

### Agent Model Used

GPT-5.3-Codex (model ID: gpt-5.3-codex)

### Debug Log References

- Workflow engine loaded: `_bmad/core/tasks/workflow.xml`
- Workflow config loaded: `_bmad/bmm/workflows/4-implementation/dev-story/workflow.yaml`
- Story source parsed from user input: `7-1` -> `7-1-webrtc-signaling-and-peer-connection`
- Quality gates executed:
  - `cd server && cargo fmt --check && cargo clippy -- -D warnings && cargo test`
  - `cd client && npm run lint && npm run check && npm run test && npm run build`

### Completion Notes List

- Implemented voice configuration surface, runtime wiring, and `webrtc-rs` module boundary for signaling.
- Added WS protocol ops and gateway handlers for `c_voice_join` / `c_voice_answer` / `c_voice_ice_candidate` and server signaling events.
- Implemented client voice signaling state/store, retry handling, and required user-facing failure/retry copy with `aria-live`.
- Added server and client coverage for signaling flow, validation/error paths, and voice-status UI behavior.
- Fixed follow-up test compile errors from `AppState` expansion by wiring `voice_runtime` in server test helpers.
- Senior review fixes: WebRTC client now captures local microphone audio and negotiates `sendrecv`; added focused unit coverage in `webrtcClient.test.ts` and clarified trigger wiring in `ShellRoute.svelte`.

### File List

- _bmad-output/implementation-artifacts/7-1-webrtc-signaling-and-peer-connection.md
- _bmad-output/implementation-artifacts/sprint-status.yaml
- config.example.toml
- server/Cargo.toml
- server/Cargo.lock
- server/src/config/mod.rs
- server/src/config/settings.rs
- server/src/error.rs
- server/src/lib.rs
- server/src/main.rs
- server/src/handlers/admin.rs
- server/src/handlers/auth.rs
- server/src/handlers/health.rs
- server/src/handlers/instance.rs
- server/src/handlers/users.rs
- server/src/handlers/ws.rs
- server/src/middleware/auth.rs
- server/src/ws/gateway.rs
- server/src/ws/protocol.rs
- server/src/webrtc/mod.rs
- server/src/webrtc/signaling.rs
- server/src/webrtc/turn.rs
- server/src/webrtc/voice_channel.rs
- server/tests/server_binds_to_configured_port.rs
- client/src/App.svelte
- client/src/lib/components/ToastViewport.svelte
- client/src/lib/components/ToastViewport.test.ts
- client/src/lib/features/chat/MessageArea.svelte
- client/src/lib/features/chat/MessageArea.test.ts
- client/src/lib/features/shell/ShellRoute.svelte
- client/src/lib/features/shell/ShellRoute.test.ts
- client/src/lib/features/voice/types.ts
- client/src/lib/features/voice/voiceStore.svelte.ts
- client/src/lib/features/voice/voiceStore.test.ts
- client/src/lib/features/voice/webrtcClient.ts
- client/src/lib/features/voice/webrtcClient.test.ts
- client/src/lib/feedback/toastStore.svelte.ts
- client/src/lib/feedback/userFacingError.ts
- client/src/lib/feedback/userFacingError.test.ts
- client/src/lib/ws/client.test.ts
- client/src/lib/ws/protocol.ts

## Change Log

- 2026-03-01: Completed Story 7.1 implementation for WebRTC signaling and peer-connection foundation; added server/client signaling flows, voice status UX, test coverage, and passed client/server quality gates.
- 2026-03-01: Senior Developer Review (AI) found one HIGH signaling issue and one MEDIUM documentation gap; both were fixed, tests were added, and story status was moved to `done`.

### Senior Developer Review (AI)

- Reviewer: Darko (AI)
- Date: 2026-03-01
- Outcome: Approve after fixes

#### Findings

1. [HIGH] `webrtcClient` negotiated `recvonly` audio and did not capture a local microphone track, preventing users from transmitting voice.
2. [MEDIUM] Story File List omitted multiple modified source files, reducing review traceability.
3. [LOW] Story implementation notes were ambiguous about voice-join trigger location (implemented in `ShellRoute.svelte`, not `ChannelList.svelte`).

#### Fixes Applied

- Updated `client/src/lib/features/voice/webrtcClient.ts` to request microphone access (`getUserMedia({ audio: true })`) and negotiate `sendrecv` with a local audio track.
- Added `client/src/lib/features/voice/webrtcClient.test.ts` covering sendrecv negotiation and microphone-unavailable failure behavior.
- Updated story Completion Notes and File List to align with actual implementation and clarify trigger wiring.
- Re-ran quality gates:
  - `cd client && npm run lint && npm run check && npm run test && npm run build`
  - `cd server && cargo fmt --check && cargo clippy -- -D warnings && cargo test`
