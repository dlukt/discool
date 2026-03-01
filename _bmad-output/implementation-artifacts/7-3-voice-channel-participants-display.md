# Story 7.3: Voice Channel Participants Display

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a **user**,
I want to see who is in a voice channel before and after joining,
so that I know who I'll be talking with.

## Acceptance Criteria

1. **Given** users are in a voice channel  
   **When** another user views the ChannelList  
   **Then** the voice channel shows participant avatars inline (max 3 shown + "+N" overflow) and participant count.

2. **Given** a user is connected to voice  
   **When** they click the VoiceBar expand control  
   **Then** a full voice participant panel opens.

3. **Given** the participant panel or inline participant rows are visible  
   **When** participant data renders  
   **Then** each VoiceParticipant shows avatar, username, and mute/deafen icons when applicable.

4. **Given** a participant is speaking  
   **When** speaking state changes  
   **Then** an ice-blue speaking border glow appears while speaking and disappears when speaking stops.

5. **Given** users with motion sensitivity preferences  
   **When** speaking state transitions occur  
   **Then** `prefers-reduced-motion` is respected.

6. **Given** assistive technologies are active  
   **When** a voice channel row updates  
   **Then** screen readers announce voice occupancy (example: "3 users in voice channel General").

## Tasks / Subtasks

- [x] Task 1: Add explicit voice participant wire contracts across client/server (AC: 1, 2, 3, 4)
  - [x] Extend `server/src/ws/protocol.rs` with `voice_state_update` server op and include it in `STORY_6_1_SERVER_EVENTS` so clients can rely on hello capabilities.
  - [x] Add `VoiceStateUpdate` payload types (`guild_slug`, `channel_slug`, `participant_count`, `participants[]`) in the server voice signaling domain (`server/src/webrtc/signaling.rs` and/or `server/src/ws/gateway.rs`).
  - [x] Extend `client/src/lib/features/voice/types.ts` with `VoiceParticipantWire` and `VoiceStateUpdateWire`.
  - [x] Keep JSON field naming `snake_case` at the wire boundary and map to typed `camelCase` structures in store selectors.  
    [Source: _bmad-output/planning-artifacts/architecture.md#API Boundary (JSON)]  
    [Source: _bmad-output/planning-artifacts/architecture.md#WebSocket Event Naming]

- [x] Task 2: Implement server-side participant snapshot + guild broadcast path (AC: 1, 2, 3, 6)
  - [x] Refactor `server/src/webrtc/voice_channel.rs` session metadata to track user/guild/channel participant state (currently `_user_id` and related fields are not used).
  - [x] Add runtime helpers to list participants for a channel and produce deterministic participant snapshots.
  - [x] Enrich participant snapshot data with user profile fields (`username`, `display_name`, `avatar_color`) via existing guild/member model queries (`server/src/models/guild_member.rs`).
  - [x] On join/leave/disconnect (`handle_voice_join`, `handle_voice_leave`, connection teardown), broadcast `voice_state_update` to guild subscribers via `registry::broadcast_to_guild`.
  - [x] Ensure snapshot emission is idempotent and safe when sessions are already absent.  
    [Source: server/src/webrtc/voice_channel.rs]  
    [Source: server/src/ws/gateway.rs]  
    [Source: server/src/ws/registry.rs]

- [x] Task 3: Add participant mute/deafen/speaking state propagation (AC: 3, 4, 5)
  - [x] Add a client voice state update op (e.g., `c_voice_state_update`) in `client/src/lib/ws/protocol.ts` and `server/src/ws/protocol.rs`.
  - [x] Send local mute/deafen transitions from `client/src/lib/features/voice/voiceStore.svelte.ts` so remote participant icons are accurate.
  - [x] Implement local speaking-state detection in `client/src/lib/features/voice/webrtcClient.ts` (Web Audio analyser or WebRTC stats) and send updates only on state transitions to avoid event spam.
  - [x] Validate speaking updates server-side against active voice membership, then rebroadcast in `voice_state_update`.
  - [x] Keep speaking indicator visual as static state (no pulsing animation); transitions must be reduced/disabled under `prefers-reduced-motion`.  
    [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Motion]  
    [Source: https://developer.mozilla.org/en-US/docs/Web/API/AudioContext/createMediaStreamSource]  
    [Source: https://developer.mozilla.org/en-US/docs/Web/API/AnalyserNode/getByteTimeDomainData]  
    [Source: https://developer.mozilla.org/en-US/docs/Web/CSS/@media/prefers-reduced-motion]

- [x] Task 4: Extend voice store selectors for multi-channel participant views (AC: 1, 2, 3, 4, 6)
  - [x] Update `client/src/lib/features/voice/voiceStore.svelte.ts` to ingest `voice_state_update` envelopes and maintain participant snapshots keyed by `guildSlug:channelSlug`.
  - [x] Add selectors for:
    - [x] `participantCountForChannel(guildSlug, channelSlug)`
    - [x] `participantsForChannel(guildSlug, channelSlug)`
    - [x] `activeChannelParticipants()`
  - [x] Clear participant cache entries for channels when explicit disconnected state is received or when switching active context where applicable.
  - [x] Preserve Story 7.1/7.2 connection lifecycle behavior (`idle/connecting/retrying/connected/failed`) with no regressions.  
    [Source: _bmad-output/implementation-artifacts/7-1-webrtc-signaling-and-peer-connection.md]  
    [Source: _bmad-output/implementation-artifacts/7-2-voice-bar-and-basic-controls.md]

- [x] Task 5: Implement ChannelList voice occupancy UI (AC: 1, 6)
  - [x] Update both categorized and uncategorized channel row render paths in `client/src/lib/features/channel/ChannelList.svelte` to show:
    - [x] inline avatars (max 3),
    - [x] overflow chip (`+N`) when participants > 3,
    - [x] participant count text.
  - [x] Add a11y label updates that include occupancy phrasing (for example: `"Open channel General. 3 users in voice channel General"`), while preserving unread prefixes.
  - [x] Reuse existing avatar rendering conventions (initial + `avatarColor`) consistent with `MemberList.svelte` styling patterns.
  - [x] Keep text-channel appearance unchanged.  
    [Source: _bmad-output/planning-artifacts/ux-design-specification.md#ChannelListItem]  
    [Source: client/src/lib/features/channel/ChannelList.svelte]  
    [Source: client/src/lib/features/members/MemberList.svelte]

- [x] Task 6: Add VoiceBar expand affordance and participant panel UI (AC: 2, 3, 4, 5)
  - [x] Extend `client/src/lib/features/voice/VoiceBar.svelte` props with `onToggleParticipants` and `isParticipantsOpen`, and add the expand button required by UX.
  - [x] Add `client/src/lib/features/voice/VoicePanel.svelte` and `client/src/lib/features/voice/VoiceParticipant.svelte` aligned to architecture structure.
  - [x] Render panel from `client/src/lib/features/chat/MessageArea.svelte` when connected; panel should use `voiceState.activeChannelParticipants()` data.
  - [x] Include per-participant mute/deafen icons and speaking border-glow state class.
  - [x] Ensure panel controls and participant rows have keyboard/focus and ARIA labels matching existing VoiceBar accessibility expectations.  
    [Source: _bmad-output/planning-artifacts/architecture.md#Structure Patterns]  
    [Source: _bmad-output/planning-artifacts/ux-design-specification.md#VoiceBar]  
    [Source: _bmad-output/planning-artifacts/ux-design-specification.md#VoiceParticipant]

- [x] Task 7: Add AC-focused tests and run quality gates (AC: all)
  - [x] Client tests:
    - [x] `client/src/lib/features/channel/ChannelList.test.ts` (voice row avatars/count/overflow + ARIA copy)
    - [x] `client/src/lib/features/voice/VoiceBar.test.ts` (expand control)
    - [x] `client/src/lib/features/voice/voiceStore.test.ts` (`voice_state_update` ingest + selectors + disconnect cleanup)
    - [x] `client/src/lib/features/chat/MessageArea.test.ts` (panel open/close + wiring)
    - [x] New tests for `VoicePanel.svelte`/`VoiceParticipant.svelte`
  - [x] Server tests:
    - [x] `server/src/ws/protocol.rs` (new op parsing + server op string mapping)
    - [x] `server/src/ws/gateway.rs` (payload validation and join/leave/state-update broadcast behavior)
    - [x] `server/src/webrtc/voice_channel.rs` (participant snapshot/state transitions)
    - [x] `server/tests/server_binds_to_configured_port.rs` integration flow for `voice_state_update` events
  - [x] Run quality gates:
    - [x] `cd client && npm run lint && npm run check && npm run test && npm run build`
    - [x] `cd server && cargo fmt --check && cargo clippy -- -D warnings && cargo test`

## Dev Notes

### Developer Context

- Story 7.1 established voice signaling and peer connection lifecycle (`c_voice_join`, offer/answer/candidate, retry state machine).
- Story 7.2 added VoiceBar controls, mute/deafen/disconnect behavior, and keyboard shortcuts, but did not include participant roster display or participant state broadcast.
- Current codebase gap for this story:
  - `ChannelList.svelte` only shows voice icon (`🔊`) with no participant metadata.
  - `VoiceBar.svelte` has no expand button or participant panel.
  - `voiceStore.svelte.ts` tracks only active channel connection/control state, not roster snapshots.
  - Server voice flow sends signaling events and connection state but not participant list/state updates.  
    [Source: client/src/lib/features/channel/ChannelList.svelte]  
    [Source: client/src/lib/features/voice/VoiceBar.svelte]  
    [Source: client/src/lib/features/voice/voiceStore.svelte.ts]  
    [Source: server/src/ws/gateway.rs]  
    [Source: server/src/ws/protocol.rs]

### Technical Requirements

- Use the existing single shared WebSocket transport; do not add a second socket channel for voice roster state.
- Conform to WS naming conventions:
  - client ops prefixed with `c_`,
  - server events unprefixed `snake_case`.
- Treat `voice_state_update` as the single source of truth for participant roster rendering in channel list and expanded panel.
- Preserve existing voice connection behavior and join-time target (<2s) while adding participant updates.
- Keep graceful degradation: voice participant UI failures must not impact text messaging or shell routing.
- Avoid animation-heavy speaking visuals; use static border-glow class changes only.  
  [Source: _bmad-output/planning-artifacts/architecture.md#WebSocket Event Naming]  
  [Source: _bmad-output/planning-artifacts/prd.md#Voice Communication]  
  [Source: _bmad-output/planning-artifacts/prd.md#Non-Functional Requirements]  
  [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Motion]

### Architecture Compliance

1. Keep voice connection internals in `features/voice/*` and `server/src/webrtc/*`; avoid distributing participant logic into unrelated modules.
2. Keep server event distribution through `server/src/ws/registry.rs` (`broadcast_to_guild` for roster updates) to match existing pub/sub patterns.
3. Keep JSON contracts explicit and strongly typed on both sides; avoid untyped payload reads.
4. Keep accessibility announcements in existing polite/assertive live regions where possible.

### Library & Framework Requirements

- Frontend: Svelte 5 runes + existing testing stack (Vitest + Testing Library).
- Backend: Rust Axum + webrtc-rs `0.17.x` already pinned by project architecture.
- Speaking-state detection references:
  - Web Audio graph input via `AudioContext.createMediaStreamSource()`.
  - Time-domain sampling via `AnalyserNode.getByteTimeDomainData()`.
  - Reduced motion support via CSS media query.
  [Source: _bmad-output/planning-artifacts/architecture.md#Gap Analysis Results]  
  [Source: https://developer.mozilla.org/en-US/docs/Web/API/AudioContext/createMediaStreamSource]  
  [Source: https://developer.mozilla.org/en-US/docs/Web/API/AnalyserNode/getByteTimeDomainData]  
  [Source: https://developer.mozilla.org/en-US/docs/Web/CSS/@media/prefers-reduced-motion]

### File Structure Requirements

Expected primary touch points:

- Client
  - `client/src/lib/features/channel/ChannelList.svelte`
  - `client/src/lib/features/channel/ChannelList.test.ts`
  - `client/src/lib/features/chat/MessageArea.svelte`
  - `client/src/lib/features/chat/MessageArea.test.ts`
  - `client/src/lib/features/voice/VoiceBar.svelte`
  - `client/src/lib/features/voice/VoiceBar.test.ts`
  - `client/src/lib/features/voice/VoicePanel.svelte` (new)
  - `client/src/lib/features/voice/VoiceParticipant.svelte` (new)
  - `client/src/lib/features/voice/types.ts`
  - `client/src/lib/features/voice/voiceStore.svelte.ts`
  - `client/src/lib/features/voice/voiceStore.test.ts`
  - `client/src/lib/features/voice/webrtcClient.ts`
  - `client/src/lib/ws/protocol.ts`

- Server
  - `server/src/ws/protocol.rs`
  - `server/src/ws/gateway.rs`
  - `server/src/ws/registry.rs` (only if targeting semantics require helper extension)
  - `server/src/webrtc/voice_channel.rs`
  - `server/src/webrtc/signaling.rs`
  - `server/src/models/guild_member.rs` (reuse existing queries for participant profile fields)
  - `server/tests/server_binds_to_configured_port.rs`

### Testing Requirements

- Validate that ChannelList voice rows render max-three avatars and `+N` overflow correctly for 0, 1, 3, and 4+ participants.
- Validate screen-reader occupancy text exactly includes channel name and count.
- Validate VoiceBar expand button toggles panel without affecting existing mute/deafen/disconnect controls.
- Validate voice store receives `voice_state_update` and keeps panel/channel list in sync.
- Validate mute/deafen/speaking state propagation is reflected on remote clients via integration test event assertions.
- Validate disconnect/leave updates remove users from participant snapshots immediately.

### Previous Story Intelligence

- Story 7.1 and 7.2 hardening emphasized not regressing signaling reliability and existing keyboard ergonomics.
- Story 7.2 left the voice data model intentionally narrow (`isMuted`, `isDeafened`, active channel state only), which is the core extension point for this story.
- Existing user-facing retry/failure copy should remain unchanged unless a new participant-specific failure mode requires additional text.  
  [Source: _bmad-output/implementation-artifacts/7-1-webrtc-signaling-and-peer-connection.md]  
  [Source: _bmad-output/implementation-artifacts/7-2-voice-bar-and-basic-controls.md]

### Git Intelligence Summary

Recent commits relevant to this story:

- `3cb1816` feat: complete story 7-2 voice controls
- `9550eb0` chore: record story 7-1 review rerun
- `96c6188` fix: harden voice signaling state flow
- `0a71534` feat: implement quick switcher and webrtc signaling

Actionable implications:

- Keep voice work incremental in `features/voice/*` and `server/src/webrtc/*`.
- Preserve existing reconnect + status logic while extending state shape.
- Pair each protocol or UI behavior change with explicit test updates in the same story implementation.

### Latest Technical Information

- `RTCPeerConnection.getStats()` remains promise-based; deprecated callback form should not be used.
- `RTCInboundRtpStreamStats` includes `audioLevel`, which can inform speaking-state heuristics if a stats-based strategy is chosen.
- Web Audio media-stream source + analyser APIs are well-supported for local activity detection.
- `prefers-reduced-motion` continues to be the standard for motion sensitivity handling.  
  [Source: https://developer.mozilla.org/en-US/docs/Web/API/RTCPeerConnection/getStats]  
  [Source: https://developer.mozilla.org/en-US/docs/Web/API/RTCInboundRtpStreamStats]  
  [Source: https://developer.mozilla.org/en-US/docs/Web/API/AudioContext/createMediaStreamSource]  
  [Source: https://developer.mozilla.org/en-US/docs/Web/CSS/@media/prefers-reduced-motion]

### Project Context Reference

- No `project-context.md` file was discovered via `**/project-context.md`.
- Story context is derived from planning artifacts, implementation artifacts, current repository state, and targeted technical references.

### Story Completion Status

- Workflow analysis complete.
- Story document generated and ready for implementation planning handoff.
- Target sprint status for story is `ready-for-dev`.

### Project Structure Notes

- Architecture references include `VoicePanel.svelte` and `VoiceParticipant.svelte`, but these do not yet exist in the codebase; this story is the first planned introduction point.
- `ChannelList.svelte` contains duplicated channel row templates (categorized + uncategorized); ensure participant UI logic is applied consistently in both paths.
- `voiceStore.svelte.ts` is currently scoped to active voice connection only; participant roster must be represented without breaking existing connection-state selectors.

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Story 7.3: Voice Channel Participants Display]
- [Source: _bmad-output/planning-artifacts/prd.md#Voice Communication]
- [Source: _bmad-output/planning-artifacts/prd.md#User Experience & Navigation]
- [Source: _bmad-output/planning-artifacts/prd.md#Non-Functional Requirements]
- [Source: _bmad-output/planning-artifacts/architecture.md#WebSocket Event Naming]
- [Source: _bmad-output/planning-artifacts/architecture.md#Structure Patterns]
- [Source: _bmad-output/planning-artifacts/architecture.md#State Boundaries]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#ChannelListItem]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#VoiceBar]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#VoiceParticipant]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Motion]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Screen readers]
- [Source: _bmad-output/implementation-artifacts/7-1-webrtc-signaling-and-peer-connection.md]
- [Source: _bmad-output/implementation-artifacts/7-2-voice-bar-and-basic-controls.md]
- [Source: _bmad-output/implementation-artifacts/sprint-status.yaml]
- [Source: client/src/lib/features/channel/ChannelList.svelte]
- [Source: client/src/lib/features/channel/ChannelList.test.ts]
- [Source: client/src/lib/features/chat/MessageArea.svelte]
- [Source: client/src/lib/features/chat/MessageArea.test.ts]
- [Source: client/src/lib/features/voice/VoiceBar.svelte]
- [Source: client/src/lib/features/voice/VoiceBar.test.ts]
- [Source: client/src/lib/features/voice/voiceStore.svelte.ts]
- [Source: client/src/lib/features/voice/voiceStore.test.ts]
- [Source: client/src/lib/features/voice/webrtcClient.ts]
- [Source: client/src/lib/ws/protocol.ts]
- [Source: server/src/models/guild_member.rs]
- [Source: server/src/webrtc/signaling.rs]
- [Source: server/src/webrtc/voice_channel.rs]
- [Source: server/src/ws/gateway.rs]
- [Source: server/src/ws/protocol.rs]
- [Source: server/src/ws/registry.rs]
- [Source: server/tests/server_binds_to_configured_port.rs]
- [Source: https://developer.mozilla.org/en-US/docs/Web/API/RTCPeerConnection/getStats]
- [Source: https://developer.mozilla.org/en-US/docs/Web/API/RTCInboundRtpStreamStats]
- [Source: https://developer.mozilla.org/en-US/docs/Web/API/AudioContext/createMediaStreamSource]
- [Source: https://developer.mozilla.org/en-US/docs/Web/API/AnalyserNode/getByteTimeDomainData]
- [Source: https://developer.mozilla.org/en-US/docs/Web/CSS/@media/prefers-reduced-motion]

## Dev Agent Record

### Agent Model Used

GPT-5.3-Codex (model ID: gpt-5.3-codex)

### Debug Log References

- Marked Story 7.3 as in-progress in sprint tracker and loaded all implementation context.
- Added server/client websocket protocol contracts for `voice_state_update` and `c_voice_state_update`.
- Implemented voice runtime participant state tracking, deterministic snapshots, and guild snapshot broadcasts on join/leave/disconnect/state updates.
- Extended client voice store with `voice_state_update` ingest, participant selectors, local mute/deafen/speaking state publishing, and speaking-state detection hook-up.
- Implemented channel-list occupancy UI (inline avatars, overflow `+N`, participant count, occupancy a11y labels) in both categorized and uncategorized render paths.
- Added VoiceBar participants toggle plus new VoicePanel/VoiceParticipant components and MessageArea wiring.
- Added/updated server and client tests, then executed full quality gates for client and server successfully.

### Completion Notes List

- Implemented end-to-end voice participant state contracts and transport across websocket server/client boundaries.
- Added server-side participant snapshot lifecycle with profile enrichment and `voice_state_update` broadcast fanout.
- Added client-side participant snapshot state model and selectors (`participantCountForChannel`, `participantsForChannel`, `activeChannelParticipants`).
- Added local mute/deafen/speaking state propagation to server and connected speaking detection to state-transition-only updates.
- Delivered ChannelList occupancy UI and accessibility phrasing for voice rows while preserving text-channel behavior.
- Delivered expandable voice participant panel experience with participant status indicators and reduced-motion-safe speaking visuals.
- Expanded automated coverage for ChannelList, VoiceBar, VoicePanel, VoiceParticipant, voiceStore, MessageArea, protocol/gateway/runtime, and websocket integration behavior.

### File List

- _bmad-output/implementation-artifacts/7-3-voice-channel-participants-display.md
- _bmad-output/implementation-artifacts/sprint-status.yaml
- client/src/lib/features/channel/ChannelList.svelte
- client/src/lib/features/channel/ChannelList.test.ts
- client/src/lib/features/chat/MessageArea.svelte
- client/src/lib/features/chat/MessageArea.test.ts
- client/src/lib/features/voice/VoiceBar.svelte
- client/src/lib/features/voice/VoiceBar.test.ts
- client/src/lib/features/voice/VoicePanel.svelte
- client/src/lib/features/voice/VoicePanel.test.ts
- client/src/lib/features/voice/VoiceParticipant.svelte
- client/src/lib/features/voice/VoiceParticipant.test.ts
- client/src/lib/features/voice/types.ts
- client/src/lib/features/voice/voiceStore.svelte.ts
- client/src/lib/features/voice/voiceStore.test.ts
- client/src/lib/features/voice/webrtcClient.ts
- client/src/lib/ws/protocol.ts
- server/src/webrtc/signaling.rs
- server/src/webrtc/voice_channel.rs
- server/src/ws/gateway.rs
- server/src/ws/protocol.rs
- server/tests/server_binds_to_configured_port.rs

## Senior Developer Review (AI)

- Reviewer: Darko
- Date: 2026-03-01
- Outcome: Approve (after fix)
- Git vs Story File List Discrepancies: 0

### Findings

1. **MEDIUM** `server/src/webrtc/voice_channel.rs:226` accepted `is_speaking=true` while muted/deafened, allowing contradictory participant state from client payloads.
   - **Fix applied:** normalize speaking state server-side with `session.is_speaking = next_state.is_speaking && !session.is_muted`.
   - **Regression test updated:** `participant_state_updates_clear_speaking_when_muted`.
2. **HIGH** `server/tests/server_binds_to_configured_port.rs:4762` expected `is_speaking=true` after a muted/deafened state update, conflicting with normalized runtime behavior.
   - **Fix applied:** corrected expected value to `json!(false)` so websocket integration assertions match runtime normalization.
3. **MEDIUM** Voice occupancy updates exposed only static labels and did not provide live announcements for assistive tech users.
   - **Fix applied:** added `aria-live="polite"` occupancy announcements in `client/src/lib/features/voice/VoicePanel.svelte` and both voice row render paths in `client/src/lib/features/channel/ChannelList.svelte`.
   - **Regression tests updated:** `VoicePanel.test.ts` and `ChannelList.test.ts`.
4. **LOW** Voice participant speaking glow retained shadow under reduced-motion preferences.
   - **Fix applied:** added `motion-reduce:shadow-none` to speaking avatar state in `client/src/lib/features/voice/VoiceParticipant.svelte`.
5. **LOW** `sendVoiceStateUpdate` could emit stale updates if the provided context no longer matched the active channel.
   - **Fix applied:** guarded updates with connected-status and active-context checks in `client/src/lib/features/voice/voiceStore.svelte.ts`.

### Validation

- `cd server && cargo fmt --check`
- `cd server && cargo clippy -- -D warnings`
- `cd server && cargo test participant_state_updates_clear_speaking_when_muted`
- `cd server && cargo test websocket_voice_join_emits_offer_and_candidate_and_rejects_invalid_answer_sdp`
- `cd client && npm run test -- ChannelList.test.ts VoicePanel.test.ts VoiceParticipant.test.ts`
- `cd client && npm run test -- voiceStore.test.ts`
- `cd client && npm run lint && npm run check`

## Change Log

- 2026-03-01: Implemented Story 7.3 voice participant display across protocol, runtime, store, UI, accessibility, and automated tests; moved story status to `review`.
- 2026-03-01: Senior developer review identified and fixed muted/speaking state normalization in voice runtime; moved story status to `done`.
- 2026-03-01: YOLO rerun review fixed websocket speaking assertion mismatch, added live occupancy announcements for assistive tech, and hardened voice state update/reduced-motion behavior.
