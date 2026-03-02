# Story 7.5: Voice Channel Switching

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a **user**,
I want to switch between voice channels seamlessly,
so that I can move between conversations without manual disconnect/reconnect.

## Acceptance Criteria

1. **Given** a user is currently connected to a voice channel  
   **When** they click a different voice channel  
   **Then** the previous connection disconnects automatically.

2. **Given** a user switches from one voice channel to another  
   **When** the switch starts  
   **Then** a new connection to the target channel is established.

3. **Given** a user switches voice channels  
   **When** the transition happens  
   **Then** no manual disconnect step is required.

4. **Given** a user has switched channels  
   **When** the target channel connects  
   **Then** the VoiceBar shows the new channel name.

5. **Given** a user switches channels  
   **When** they leave one channel and join another  
   **Then** other users see the participant leave/join in real time.

6. **Given** a channel switch is initiated  
   **When** connection is established  
   **Then** join completes within the 2-second target (NFR2).

## Tasks / Subtasks

- [x] Task 1: Harden client channel-switch lifecycle in voice state/store (AC: 1, 2, 3, 4, 6)
  - [x] Ensure `voiceState.activateVoiceChannel()` performs deterministic switch sequencing: leave old context, reset retry/join timers, close old peer connection, and begin new join.
  - [x] Prevent stale retry/join callbacks from old channel context from mutating active channel state after a switch.
  - [x] Keep `voiceState.status` transitions coherent (`connecting`/`retrying`/`connected`/`failed`) during rapid channel changes.
  - [x] Keep switch path idempotent for repeated clicks on current active voice channel.

- [x] Task 2: Preserve user-facing continuity during switch (AC: 3, 4, 6)
  - [x] Keep VoiceBar behavior consistent during switch (channel label updates with active context; no manual disconnect UX step).
  - [x] Preserve existing connection messaging semantics (`Could not connect to voice. Retrying...`, terminal failure copy) while switching.
  - [x] Ensure participants panel state and active participant rendering stay internally consistent when channel context changes.

- [x] Task 3: Preserve voice participant volume and mute/deafen semantics across switching (AC: 2, 3, 4)
  - [x] Keep per-participant volume preference model and identity-scoped persistence intact across channel switches.
  - [x] Ensure deafen/mute control semantics remain unchanged after switch and reconnect path.
  - [x] Rebind participant audio outputs for the new channel snapshot without leaking old channel stream bindings.

- [x] Task 4: Verify server-side voice session transition and broadcast integrity (AC: 1, 2, 5)
  - [x] Confirm `c_voice_join` + `c_voice_leave` handling produces a correct leave/join sequence for switch scenarios.
  - [x] Ensure `voice_state_update` broadcasts reflect old-channel removal and new-channel addition for the switching participant.
  - [x] Keep permission checks (`VIEW_CHANNEL`) and voice-channel type validation intact for switch path.

- [x] Task 5: Add/expand tests for switch behavior and timing expectations (AC: all)
  - [x] Add `voiceStore.test.ts` coverage for active voice channel switching, stale event rejection, and control-state invariants.
  - [x] Add/update `MessageArea`/`ShellRoute` integration tests for route-driven voice switching behavior.
  - [x] Add server WebSocket tests for switch-related leave/join + `voice_state_update` event sequence.
  - [x] Assert NFR2 join timing instrumentation remains available and no regression to timeout defaults (`2_000ms` target baseline).
  - [x] Run quality gates:
    - [x] `cd client && npm run lint && npm run check && npm run test && npm run build`
    - [x] `cd server && cargo fmt --check && cargo clippy -- -D warnings && cargo test`

## Dev Notes

### Developer Context

- Story 7.4 completed local participant volume controls, WebRTC output binding, and identity-scoped volume persistence; 7.5 must preserve that behavior through channel transitions.
- Current routing integration drives voice activation in `ShellRoute.svelte` by selecting a voice channel route, then calling `voiceState.activateVoiceChannel(activeGuild, activeChannel)`.
- Current voice state source-of-truth is `voiceStore.svelte.ts` and voice presence snapshots are keyed by `guild_slug:channel_slug`.
- Server currently exposes `c_voice_join`, `c_voice_leave`, `c_voice_state_update` flow with broadcast updates via `voice_state_update`.  
  [Source: _bmad-output/implementation-artifacts/7-4-individual-volume-control.md#Previous Story Intelligence]  
  [Source: client/src/lib/features/shell/ShellRoute.svelte]  
  [Source: client/src/lib/features/voice/voiceStore.svelte.ts]  
  [Source: server/src/ws/gateway.rs]

### Technical Requirements

- Keep switching on the existing WebSocket signaling contract (`c_voice_join` / `c_voice_leave` / `voice_state_update`); do not introduce new transport channels for this story.
- Keep channel switch join path aligned with NFR2 (`<2s` click-to-connected target).
- Preserve graceful degradation (voice failures must not break text path).
- Preserve existing user-visible retry/failure copy and avoid silent failures.
- Avoid race conditions from old-context async callbacks after a switch; apply strict active-context checks before mutating state.  
  [Source: _bmad-output/planning-artifacts/epics.md#Story 7.5: Voice Channel Switching]  
  [Source: _bmad-output/planning-artifacts/prd.md#Performance]  
  [Source: _bmad-output/planning-artifacts/prd.md#Reliability]

### Architecture Compliance

1. Keep voice runtime state in `voiceStore.svelte.ts` as the canonical client state boundary.
2. Keep wire payload fields `snake_case` at WS/API boundaries; continue mapping into typed TS structures.
3. Preserve WebSocket op naming conventions (`c_` for client operations, no prefix for server operations).
4. Keep voice state broadcast consumption via `voice_state_update` snapshots and channel-scoped derivations.
5. Preserve reliability principles: reconnect/backoff, partial-failure isolation, and explicit status propagation.  
   [Source: _bmad-output/planning-artifacts/architecture.md#API Boundary (JSON)]  
   [Source: _bmad-output/planning-artifacts/architecture.md#WebSocket Event Naming]  
   [Source: _bmad-output/planning-artifacts/architecture.md#State Boundaries (Frontend)]  
   [Source: _bmad-output/planning-artifacts/architecture.md#Non-Functional Requirements Coverage]

### Library & Framework Requirements

- Frontend remains Svelte 5 + TypeScript + Vitest.
- Backend remains Rust + Axum + existing WS gateway + webrtc-rs integration.
- WebRTC/session lifecycle handling should respect current browser API behavior:
  - `RTCPeerConnection.close()` tears down peer connection.
  - `MediaStreamTrack.stop()` ends local track usage.
  - `RTCPeerConnection.connectionState` (`connecting`, `connected`, `disconnected`, `failed`, `closed`) should continue to drive client state transitions.  
  [Source: _bmad-output/planning-artifacts/architecture.md#Frontend Architecture]  
  [Source: _bmad-output/planning-artifacts/architecture.md#API & Communication Patterns]  
  [Source: https://developer.mozilla.org/en-US/docs/Web/API/RTCPeerConnection/close]  
  [Source: https://developer.mozilla.org/en-US/docs/Web/API/MediaStreamTrack/stop]  
  [Source: https://developer.mozilla.org/en-US/docs/Web/API/RTCPeerConnection/connectionState]

### File Structure Requirements

Expected primary touch points:

- `client/src/lib/features/voice/voiceStore.svelte.ts`
- `client/src/lib/features/voice/webrtcClient.ts`
- `client/src/lib/features/shell/ShellRoute.svelte`
- `client/src/lib/features/chat/MessageArea.svelte`
- `client/src/lib/features/channel/ChannelList.svelte`
- `client/src/lib/features/voice/VoiceBar.svelte`
- `client/src/lib/features/voice/voiceStore.test.ts`
- `client/src/lib/features/voice/VoiceBar.test.ts`
- `server/src/ws/gateway.rs`
- `server/src/webrtc/voice_channel.rs`
- `server/src/ws/protocol.rs` (only if protocol/schema changes are strictly required)

### Testing Requirements

- Validate switching from Voice Channel A → Voice Channel B sends leave/join flow without manual disconnect interaction.
- Validate VoiceBar channel label tracks the active connected voice channel after switch.
- Validate participant lists/occupancy updates for both channels via `voice_state_update`.
- Validate switch under transient failure/retry conditions still follows existing copy and state semantics.
- Validate no regressions in mute/deafen/deafen-implies-mute behavior after switch.
- Validate participant volume preferences remain scoped and applied correctly after switch/rejoin.
- Validate stale async events for old channel do not override new active-channel state.
- Validate text chat remains available during voice switch failure path (NFR28).

### Previous Story Intelligence

- Story 7.4 introduced participant-volume persistence and participant-audio binding; switch flow must not leak stale stream bindings or lose preference application.
- Story 7.4 hardening included explicit persistence snapshot behavior and regression tests; keep that pattern when introducing new async switch paths.
- Story 7.3/7.4 accessibility expectations remain in force: voice state changes must remain screen-reader compatible and reduced-motion behavior must not regress.  
  [Source: _bmad-output/implementation-artifacts/7-4-individual-volume-control.md#Senior Developer Review (AI)]  
  [Source: _bmad-output/implementation-artifacts/7-4-individual-volume-control.md#Previous Story Intelligence]

### Git Intelligence Summary

Recent commit trend in voice work:

- `759b02f` feat: complete story 7-4 individual volume control
- `fc5f2bc` chore: prepare story 7-4 individual volume control
- `a40554c` feat: complete story 7-3 voice participants display
- `3cb1816` feat: complete story 7-2 voice controls
- `9550eb0` chore: record story 7-1 review rerun

Actionable implications:

- Keep voice changes incremental and centered in existing voice feature modules.
- Preserve existing test-first guardrail pattern for voice state and signaling behavior.
- Avoid protocol churn unless absolutely required by AC gaps.

### Latest Technical Information

- `RTCPeerConnection.close()` has no parameters and closes the current peer connection/session; safe teardown remains a first-class switch operation.
- `MediaStreamTrack.stop()` immediately sets track `readyState` to `ended`; local track stop should remain part of cleanup.
- `RTCPeerConnection.connectionState` exposes lifecycle states used by current voice store transition logic (`connected`, `disconnected`, `failed`, `closed`).  
  [Source: https://developer.mozilla.org/en-US/docs/Web/API/RTCPeerConnection/close]  
  [Source: https://developer.mozilla.org/en-US/docs/Web/API/MediaStreamTrack/stop]  
  [Source: https://developer.mozilla.org/en-US/docs/Web/API/RTCPeerConnection/connectionState]

### Project Context Reference

- No `project-context.md` file was discovered via `**/project-context.md`.
- Story context is derived from planning artifacts, implementation artifacts, repository code, and current Web API references.

### Story Completion Status

- Workflow analysis complete.
- Story document generated and ready for implementation handoff.
- Sprint status target for this story: `ready-for-dev`.
- Completion note: Ultimate context engine analysis completed - comprehensive developer guide created.

### Project Structure Notes

- Route-driven voice activation currently occurs in `ShellRoute.svelte`; switching behavior is currently achieved by selecting a different voice channel route.
- `voiceStore.activateVoiceChannel()` already handles same-channel dedupe and old-context leave/cleanup; story focus is on seamlessness, race-hardening, and comprehensive verification across UI + WS + runtime.
- Server voice runtime keys sessions by `connection_id:guild_slug:channel_slug`; switch scenarios must maintain expected leave/join visibility to other participants.

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Epic 7: Voice Communication]
- [Source: _bmad-output/planning-artifacts/epics.md#Story 7.5: Voice Channel Switching]
- [Source: _bmad-output/planning-artifacts/prd.md#Voice Communication]
- [Source: _bmad-output/planning-artifacts/prd.md#Performance]
- [Source: _bmad-output/planning-artifacts/prd.md#Reliability]
- [Source: _bmad-output/planning-artifacts/architecture.md#API Boundary (JSON)]
- [Source: _bmad-output/planning-artifacts/architecture.md#WebSocket Event Naming]
- [Source: _bmad-output/planning-artifacts/architecture.md#State Boundaries (Frontend)]
- [Source: _bmad-output/planning-artifacts/architecture.md#Non-Functional Requirements Coverage]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Journey 5: Voice Channel Lifecycle]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#VoiceBar]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#VoiceParticipant]
- [Source: _bmad-output/implementation-artifacts/7-4-individual-volume-control.md]
- [Source: _bmad-output/implementation-artifacts/sprint-status.yaml]
- [Source: client/src/lib/features/voice/voiceStore.svelte.ts]
- [Source: client/src/lib/features/voice/webrtcClient.ts]
- [Source: client/src/lib/features/voice/VoiceBar.svelte]
- [Source: client/src/lib/features/shell/ShellRoute.svelte]
- [Source: client/src/lib/features/chat/MessageArea.svelte]
- [Source: client/src/lib/features/channel/ChannelList.svelte]
- [Source: server/src/ws/gateway.rs]
- [Source: server/src/webrtc/voice_channel.rs]
- [Source: server/src/ws/protocol.rs]
- [Source: https://developer.mozilla.org/en-US/docs/Web/API/RTCPeerConnection/close]
- [Source: https://developer.mozilla.org/en-US/docs/Web/API/MediaStreamTrack/stop]
- [Source: https://developer.mozilla.org/en-US/docs/Web/API/RTCPeerConnection/connectionState]

## Dev Agent Record

### Agent Model Used

GPT-5.3-Codex (model ID: gpt-5.3-codex)

### Debug Log References

- Loaded workflow engine and create-story workflow config/instructions.
- Resolved story from user input and sprint status: `7-5-voice-channel-switching`.
- Analyzed epics/PRD/architecture/UX artifacts plus previous Story 7.4 implementation artifact.
- Hardened `voiceStore` join-failure handling so stale async callbacks and stale error envelopes from old contexts cannot mutate active channel retry state.
- Added channel-switch regression coverage in `voiceStore.test.ts` for leave/join ordering, idempotent activation, and stale-context rejection.
- Added route/channel continuity coverage in `ShellRoute.test.ts` and `MessageArea.test.ts`.
- Updated server voice switching behavior by replacing prior same-guild sessions on `c_voice_join` and rebroadcasting old-channel `voice_state_update` snapshots.
- Added server switch-semantics tests in `voice_channel.rs` and `ws/gateway.rs`.
- Ran YOLO code review, identified missing end-to-end WebSocket switch-sequence coverage, and added integration regression coverage in `server/tests/server_binds_to_configured_port.rs`.
- Ran full quality gates for client and server and confirmed pass.

### Completion Notes List

- Implemented race-safe channel switching in the client voice store by binding join-failure handling to the active voice context.
- Preserved switch UX semantics (automatic leave/join, unchanged retry/failure copy, idempotent repeated activation behavior) with explicit tests.
- Added integration coverage ensuring voice UI channel context updates correctly during route/channel changes.
- Hardened server switch handling so new joins in the same guild clear prior session state and trigger old/new channel participant broadcasts.
- Added server WebSocket integration coverage for voice switch leave/join snapshot rebroadcast behavior.
- Completed all required lint/type/test/build gates for client and server.
- Story status set to `done`.

### File List

- _bmad-output/implementation-artifacts/7-5-voice-channel-switching.md
- _bmad-output/implementation-artifacts/sprint-status.yaml
- client/src/lib/features/chat/MessageArea.test.ts
- client/src/lib/features/shell/ShellRoute.test.ts
- client/src/lib/features/voice/voiceStore.svelte.ts
- client/src/lib/features/voice/voiceStore.test.ts
- server/tests/server_binds_to_configured_port.rs
- server/src/webrtc/voice_channel.rs
- server/src/ws/gateway.rs

## Senior Developer Review (AI)

- Reviewer: Darko
- Date: 2026-03-02
- Outcome: Approve (after fix)
- Git vs Story File List Discrepancies: 0

### Findings

1. **MEDIUM** Task 5 claimed server WebSocket switch-sequence coverage, but no integration test validated `c_voice_join` switch behavior and old/new `voice_state_update` rebroadcast semantics in `server/tests`.
   - **Fix applied:** added `websocket_voice_switch_rebroadcasts_leave_and_join_snapshots` to `server/tests/server_binds_to_configured_port.rs`.
   - **Validation:** `cd server && cargo fmt --check && cargo test --test server_binds_to_configured_port websocket_voice_switch_rebroadcasts_leave_and_join_snapshots`.

### Change Log

- 2026-03-02: Completed Story 7.5 voice channel switching implementation, added client/server switch regression coverage, and passed full client/server quality gates.
- 2026-03-02: YOLO code review found missing WebSocket switch integration coverage, added server integration regression test, and moved story status to `done`.
