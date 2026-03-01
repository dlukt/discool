# Story 7.2: Voice Bar and Basic Controls

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a **user**,
I want persistent voice controls when I'm in a voice channel,
so that I can manage my audio without navigating away from what I'm doing.

## Acceptance Criteria

1. **Given** a user has joined a voice channel  
   **When** the connection is established  
   **Then** a VoiceBar appears fixed at the bottom of the message area (above the message input)

2. **Given** the VoiceBar is visible  
   **When** it renders  
   **Then** it shows: voice channel name + guild name, connection quality indicator (green/yellow/red dot), mute toggle (mic icon), deafen toggle (headphone icon), disconnect button (phone icon, fire red)

3. **Given** a user clicks mute  
   **When** the action is applied  
   **Then** microphone transmission toggles on/off (mic icon crossed out when muted)

4. **Given** a user clicks deafen  
   **When** the action is applied  
   **Then** incoming audio toggles on/off (headphone icon crossed out when deafened; deafen also mutes)

5. **Given** a user clicks disconnect  
   **When** the action is applied  
   **Then** the user leaves the voice channel and the VoiceBar disappears

6. **Given** keyboard control shortcuts are enabled  
   **When** a user presses global shortcuts  
   **Then** `M` toggles mute, `D` toggles deafen, and `Ctrl+D` disconnects

7. **Given** the user is actively using text chat  
   **When** VoiceBar is visible  
   **Then** it remains compact and does not interfere with chat usage

8. **Given** assistive technologies are in use  
   **When** voice control state changes  
   **Then** all controls have accessible labels and changes are announced via `aria-live`

## Tasks / Subtasks

- [x] Task 1: Add a dedicated VoiceBar component and compact layout integration (AC: 1, 2, 7, 8)
  - [x] Create `client/src/lib/features/voice/VoiceBar.svelte` with channel/guild labels, connection quality dot, mute/deafen/disconnect controls, and `aria-label` coverage.
  - [x] Add visual state variants for active mute/deafen states and fire-red disconnect button.
  - [x] Mount the VoiceBar in `client/src/lib/features/chat/MessageArea.svelte` above the composer region, preserving current timeline/composer behavior.
  - [x] Visibility contract: render VoiceBar only when voice state is `connected` (AC1); hide in `idle` and after terminal `failed` disconnect cleanup; optional reconnect affordance must be explicit if `retrying` is shown.
  - [x] Ensure VoiceBar remains compact within current spacing/layout conventions.

- [x] Task 2: Extend client voice state for explicit control operations (AC: 2, 3, 4, 5, 6)
  - [x] Expand `client/src/lib/features/voice/types.ts` and `voiceStore.svelte.ts` with `isMuted`, `isDeafened`, and control intents/actions.
  - [x] Implement `toggleMute`, `toggleDeafen`, and `disconnect` actions with deterministic transitions and state reset on disconnect.
  - [x] Enforce rule: deafen implies mute in state and runtime behavior.
  - [x] Keep existing retry/connect status flow from Story 7.1 intact (do not regress join/offer/candidate handling).

- [x] Task 3: Implement mute/deafen/disconnect behavior at WebRTC/runtime level (AC: 3, 4, 5)
  - [x] Extend `client/src/lib/features/voice/webrtcClient.ts` to persist local audio track and toggle `MediaStreamTrack.enabled` for microphone mute/unmute.
  - [x] Implement deafen by muting tracked remote playback surfaces (`HTMLMediaElement.muted`) without dropping signaling session.
  - [x] Add explicit leave signaling (`c_voice_leave`) in `client/src/lib/ws/protocol.ts` and `server/src/ws/protocol.rs` with wire contract: payload `{ guild_slug, channel_slug }`.
  - [x] Define leave success response contract as `voice_connection_state` with `{ guild_slug, channel_slug, state: "disconnected" }`; repeated leave calls must be idempotent (success-shaped, no hard error).
  - [x] Add server handling in `server/src/ws/gateway.rs` plus targeted session cleanup in `server/src/webrtc/voice_channel.rs` to prevent stale voice sessions when disconnecting without closing WebSocket.

- [x] Task 4: Add global keyboard shortcuts for voice controls (AC: 6, 8)
  - [x] Wire `M`, `D`, and `Ctrl+D` in `client/src/lib/features/shell/ShellRoute.svelte` to voice store actions.
  - [x] Scope shortcuts to non-editable contexts (`input`, `textarea`, `contenteditable` excluded); preserve existing shell shortcuts (`Ctrl+K`, unread navigation).
  - [x] Call `event.preventDefault()` for handled voice shortcuts, including `Ctrl+D`, to avoid browser-default behavior conflicts.
  - [x] Surface state updates through existing `aria-live` messaging path in MessageArea/VoiceBar.

- [x] Task 5: Provide connection quality indicator mapping (AC: 2)
  - [x] Derive quality indicator from active peer connection state (`connected`, `connecting/retrying`, `disconnected/failed`) with explicit color mapping:
    - green: connected
    - yellow: connecting/retrying
    - red: disconnected/failed
  - [x] Ensure mapping remains stable across retry transitions and clears on disconnect.

- [x] Task 6: Add AC-focused test coverage and run quality gates (AC: all)
  - [x] Add/extend client tests:
    - `client/src/lib/features/voice/VoiceBar.test.ts` (new)
    - `client/src/lib/features/voice/voiceStore.test.ts`
    - `client/src/lib/features/voice/webrtcClient.test.ts`
    - `client/src/lib/features/shell/ShellRoute.test.ts`
    - `client/src/lib/features/chat/MessageArea.test.ts`
  - [x] Add/extend server tests in `server/src/ws/gateway.rs` and `server/src/webrtc/voice_channel.rs` for leave-operation validation and session cleanup.
  - [x] Run quality gates:
    - [x] `cd client && npm run lint && npm run check && npm run test && npm run build`
    - [x] `cd server && cargo fmt --check && cargo clippy -- -D warnings && cargo test`

## Dev Notes

### Developer Context

- Story 7.1 already established signaling, join retry behavior, and voice handshake plumbing:
  - `voiceStore.svelte.ts` manages `idle/connecting/connected/retrying/failed`
  - `webrtcClient.ts` handles offer/answer/candidate flow and local microphone acquisition
  - `ShellRoute.svelte` activates voice join when active channel type is `voice`
- Current UI surfaces voice status text in `MessageArea.svelte`, but has no persistent control bar yet.
- This story should build directly on Story 7.1 foundations, not reimplement join/signaling flows.

### Technical Requirements

- Keep single WebSocket transport for signaling; do not introduce additional sockets.
- Preserve existing envelope patterns and naming conventions (`snake_case`, `c_` prefix for client ops).
- Implement mute using `MediaStreamTrack.enabled` (recommended mute primitive for track media gating).
- Implement deafen using playback muting (`HTMLMediaElement.muted`) while keeping signaling alive.
- Manage remote playback elements explicitly in the voice runtime/client layer:
  - register on incoming track,
  - apply deafen state to all active remote elements,
  - cleanup on disconnect/leave/channel switch.
- Implement explicit disconnect cleanup path (client + server) so leaving voice does not require closing WS session.
- `c_voice_leave` protocol contract must be explicit and consistent across client/server:
  - request payload: `{ guild_slug, channel_slug }`
  - success event: `voice_connection_state` with `state: "disconnected"`
  - error path: existing `error` envelope (`code`, `message`, `details`)
  - repeated leave must be idempotent
- Maintain text-chat continuity: voice operations must never interrupt message timeline/composer operations.
- Preserve accessibility:
  - control labels (`aria-label`)
  - state announcements via `aria-live`
  - keyboard operability for all controls
- Keyboard scope rules:
  - ignore voice shortcuts while user is typing into editable controls,
  - for handled shortcuts (including `Ctrl+D`), call `preventDefault()`.

### Architecture Compliance

1. Keep layering consistent:
   - frontend control orchestration in `features/voice/*`
   - transport protocol changes in `client/src/lib/ws/protocol.ts` and `server/src/ws/protocol.rs`
   - server operation routing in `server/src/ws/gateway.rs`
   - session internals in `server/src/webrtc/voice_channel.rs`
2. Keep error payload contracts machine-readable (`code`, `message`, `details`) and user-safe.
3. Reuse existing chat shell/message area patterns for layout and status rendering.
4. Keep operations idempotent where possible (`disconnect` should be safe to call multiple times).

### Library & Framework Requirements

- Frontend:
  - Svelte `^5.45.2`
  - `@mateothegreat/svelte5-router ^2.16.19`
  - browser WebRTC (`RTCPeerConnection`, `MediaStreamTrack`, `HTMLMediaElement`)
- Backend:
  - Rust + Axum + Tokio
  - `webrtc-rs` `0.17.x` runtime already integrated by Story 7.1

### File Structure Requirements

Expected primary touch points:

- Client
  - `client/src/lib/features/voice/VoiceBar.svelte` (new)
  - `client/src/lib/features/voice/types.ts`
  - `client/src/lib/features/voice/voiceStore.svelte.ts`
  - `client/src/lib/features/voice/webrtcClient.ts`
  - `client/src/lib/features/chat/MessageArea.svelte`
  - `client/src/lib/features/shell/ShellRoute.svelte`
  - `client/src/lib/ws/protocol.ts`
- Server
  - `server/src/ws/protocol.rs`
  - `server/src/ws/gateway.rs`
  - `server/src/webrtc/voice_channel.rs`
  - `server/src/webrtc/signaling.rs` (only if payload/state shape changes are required)

### Testing Requirements

- Client behavior tests must cover:
  - VoiceBar rendering and compact placement
  - mute/deafen/disconnect button behavior and icon state
  - explicit visibility state contract (`connected` visible, `idle/failed` hidden unless explicitly designed otherwise)
  - keyboard shortcuts (`M`, `D`, `Ctrl+D`)
  - `aria-live` and control-label accessibility expectations
  - no regression to message composer/timeline behavior while voice controls are active
- Server behavior tests must cover:
  - new leave/disconnect op parsing
  - leave response contract (`voice_connection_state` with `state: "disconnected"`)
  - leave cleanup semantics (session removed without closing WS)
  - safe behavior on repeated leave/disconnect calls
- Maintain existing full quality gates for client and server before marking done.

### Previous Story Intelligence

- Story 7.1 review identified and fixed a high-impact issue where microphone transmission was not negotiated correctly; current implementation uses `sendrecv` with local microphone track capture. Do not regress this behavior.
- Story 7.1 also reinforced that voice trigger logic lives in `ShellRoute.svelte`, and voice state behavior should remain centralized in `voiceStore.svelte.ts`.
- Existing constants/messages:
  - `Could not connect to voice. Retrying...`
  - `Voice connection failed. Check your network.`
  should remain unchanged unless explicitly required.

### Git Intelligence Summary

Recent commits relevant to this story:

- `9550eb0` chore: record story 7-1 review rerun
- `96c6188` fix: harden voice signaling state flow
- `0a71534` feat: implement quick switcher and webrtc signaling

Actionable patterns:

- Keep changes incremental and scoped to feature modules.
- Add focused tests alongside fixes/features.
- Preserve existing keyboard/navigation interactions in shell-level handlers.

### Latest Technical Information

- MDN documents `MediaStreamTrack.enabled` as the correct mechanism for implementing mute/unmute on local media tracks.
- MDN `RTCPeerConnection.connectionState` defines canonical state values (`new`, `connecting`, `connected`, `disconnected`, `failed`, `closed`) suitable for UI quality mapping.
- MDN `HTMLMediaElement.muted` remains a direct boolean control for local playback mute/deafen behavior.
- docs.rs for `webrtc` confirms current API surface supports peer connection state handlers and ICE candidate signaling required by this story.

### Project Context Reference

- No `project-context.md` file was discovered via `**/project-context.md`.
- Story context is derived from planning artifacts, implementation artifacts, repository code, and targeted technical references.

### Story Completion Status

- Ultimate context engine analysis completed - comprehensive developer guide created.
- Story status set to `ready-for-dev`.

### Project Structure Notes

- Voice channel activation currently happens in `ShellRoute.svelte` by inspecting active channel type.
- `MessageArea.svelte` already contains connection/voice status surfaces and is the correct integration point for the VoiceBar placement requirement.
- Server `VoiceRuntime` currently clears sessions only on socket close; this story should add explicit disconnect/leave cleanup to avoid stale session buildup during channel leave/switch patterns.

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Story 7.2: Voice Bar and Basic Controls]
- [Source: _bmad-output/planning-artifacts/epics.md#Epic 7: Voice Communication]
- [Source: _bmad-output/planning-artifacts/prd.md#Voice Communication (FR39-FR44)]
- [Source: _bmad-output/planning-artifacts/prd.md#NFR2: Voice channel join time]
- [Source: _bmad-output/planning-artifacts/prd.md#NFR26: Voice auto-reconnect]
- [Source: _bmad-output/planning-artifacts/prd.md#NFR28: Graceful degradation]
- [Source: _bmad-output/planning-artifacts/architecture.md#API & Communication Patterns]
- [Source: _bmad-output/planning-artifacts/architecture.md#Frontend Architecture]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Journey 5: Voice Channel Lifecycle]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#VoiceBar]
- [Source: _bmad-output/implementation-artifacts/7-1-webrtc-signaling-and-peer-connection.md]
- [Source: _bmad-output/implementation-artifacts/sprint-status.yaml]
- [Source: client/src/lib/features/voice/voiceStore.svelte.ts]
- [Source: client/src/lib/features/voice/webrtcClient.ts]
- [Source: client/src/lib/features/chat/MessageArea.svelte]
- [Source: client/src/lib/features/shell/ShellRoute.svelte]
- [Source: client/src/lib/ws/protocol.ts]
- [Source: server/src/ws/protocol.rs]
- [Source: server/src/ws/gateway.rs]
- [Source: server/src/webrtc/voice_channel.rs]
- [Source: https://developer.mozilla.org/en-US/docs/Web/API/MediaStreamTrack/enabled]
- [Source: https://developer.mozilla.org/en-US/docs/Web/API/RTCPeerConnection/connectionState]
- [Source: https://developer.mozilla.org/en-US/docs/Web/API/HTMLMediaElement/muted]
- [Source: https://docs.rs/webrtc/latest/webrtc/peer_connection/struct.RTCPeerConnection.html]

## Dev Agent Record

### Agent Model Used

GPT-5.3-Codex (model ID: gpt-5.3-codex)

### Debug Log References

- Workflow engine loaded and create-story workflow executed for target `7-2`.
- Implemented VoiceBar UI integration and control actions across MessageArea/voice store/runtime/protocol layers.
- Added explicit `c_voice_leave` handling in client/server with idempotent voice session cleanup path.
- Executed full client and server quality gates with passing results.

### Completion Notes List

- Added compact VoiceBar with connection quality indicator, accessible labels, aria-live control announcements, and fire-red disconnect control.
- Extended voice state management with `isMuted`/`isDeafened` plus deterministic `toggleMute`/`toggleDeafen`/`disconnect` actions while preserving Story 7.1 retry/connect flow.
- Implemented runtime mute/deafen behavior (`MediaStreamTrack.enabled` + remote playback muting) and explicit leave signaling (`c_voice_leave`) with success-shaped `voice_connection_state: disconnected`.
- Added keyboard shortcuts (`M`, `D`, `Ctrl+D`) scoped to non-editable targets with `preventDefault()` for handled shortcuts.
- Added/updated AC-focused client/server tests and passed required quality gates.

### File List

- _bmad-output/implementation-artifacts/7-2-voice-bar-and-basic-controls.md
- _bmad-output/implementation-artifacts/sprint-status.yaml
- client/src/lib/features/chat/MessageArea.svelte
- client/src/lib/features/chat/MessageArea.test.ts
- client/src/lib/features/shell/ShellRoute.svelte
- client/src/lib/features/shell/ShellRoute.test.ts
- client/src/lib/features/voice/VoiceBar.svelte
- client/src/lib/features/voice/VoiceBar.test.ts
- client/src/lib/features/voice/types.ts
- client/src/lib/features/voice/voiceStore.svelte.ts
- client/src/lib/features/voice/voiceStore.test.ts
- client/src/lib/features/voice/webrtcClient.ts
- client/src/lib/features/voice/webrtcClient.test.ts
- client/src/lib/ws/protocol.ts
- server/src/webrtc/voice_channel.rs
- server/src/ws/gateway.rs
- server/src/ws/protocol.rs

## Change Log

- 2026-03-01: Implemented Story 7.2 VoiceBar controls, runtime mute/deafen/disconnect behavior, explicit leave signaling, keyboard shortcuts, and AC-focused test coverage.
- 2026-03-01: Senior Developer Review (AI) completed in YOLO mode; no HIGH/MEDIUM findings, no code fixes required, quality gates passed, and story moved to `done`.

### Senior Developer Review (AI)

- Reviewer: Darko (GPT-5.3-Codex)
- Date: 2026-03-01
- Outcome: **Approve**
- Findings:
  - No actionable HIGH or MEDIUM issues found.
  - Git changes and story File List match exactly (0 discrepancies).
  - Acceptance Criteria and completed tasks were validated against implementation and automated tests.
- Validation: `cd client && npm run lint && npm run check && npm run test && npm run build && cd ../server && cargo fmt --check && cargo clippy -- -D warnings && cargo test`
