# Story 7.4: Individual Volume Control

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a **user**,
I want to adjust the volume of individual users in a voice channel,
so that I can balance audio levels to my preference.

## Acceptance Criteria

1. **Given** a user is in a voice channel with other participants  
   **When** they click on a participant in the voice panel  
   **Then** a volume slider appears (horizontal, accessible via keyboard).

2. **Given** a participant volume slider is available  
   **When** the user adjusts the slider  
   **Then** that participant's audio volume changes locally for that user only (client-side, no server-side persistence).

3. **Given** a user has customized participant volumes  
   **When** they reconnect or reload  
   **Then** per-participant volume settings are restored from IndexedDB.

4. **Given** the volume control is rendered  
   **When** the user interacts with it  
   **Then** the supported range is 0% (muted) to 200% (amplified).

5. **Given** no saved preference exists for a participant  
   **When** volume control initializes  
   **Then** the default value is 100%.

6. **Given** the viewer has moderator voice permissions  
   **When** they inspect a participant row  
   **Then** a visible "Kick from voice" placeholder action is shown (non-functional in this story; functional behavior ships in Epic 8).

## Tasks / Subtasks

- [x] Task 1: Add participant volume preference state model in voice store (AC: 2, 3, 4, 5)
  - [x] Extend `client/src/lib/features/voice/types.ts` with strongly typed participant volume preference contracts (no `any`).
  - [x] Extend `client/src/lib/features/voice/voiceStore.svelte.ts` with per-participant volume selectors/actions keyed by stable user identity.
  - [x] Normalize and clamp values to `0..200` percent and derive audio scalar values in one place to prevent drift.
  - [x] Keep existing voice lifecycle behavior (`idle/connecting/retrying/connected/failed`) unchanged.

- [x] Task 2: Persist volume preferences in IndexedDB (AC: 3, 4, 5)
  - [x] Add voice volume storage module under `client/src/lib/features/voice/` using IndexedDB transaction helpers aligned with existing identity/block patterns.
  - [x] Scope persisted values to current viewer identity + participant user ID so settings do not leak between identities.
  - [x] Handle database open/read/write failures explicitly with existing user-facing error patterns (no silent failure path).
  - [x] Load persisted volume preferences during voice store initialization and apply defaults when missing.

- [x] Task 3: Wire per-participant playback gain in WebRTC client (AC: 2, 4, 5)
  - [x] Refactor `client/src/lib/features/voice/webrtcClient.ts` remote playback tracking from a plain set to a participant-addressable structure.
  - [x] Support `0..100%` via `HTMLMediaElement.volume` and `101..200%` via Web Audio `GainNode` amplification path.
  - [x] Preserve existing deafen behavior as a global local playback mute across all participant outputs.
  - [x] Ensure cleanup closes audio resources (`AudioContext`, nodes, elements) on disconnect/channel switch.
  - [x] If deterministic track-to-user mapping is not available in current signaling flow, add minimal metadata to existing voice payloads to establish that mapping without introducing a new transport.

- [x] Task 4: Add VoiceParticipant volume slider UI and keyboard semantics (AC: 1, 2, 4, 5)
  - [x] Update `client/src/lib/features/voice/VoiceParticipant.svelte` to include a horizontal slider and current percent label.
  - [x] Keep existing speaking/muted/deafened visual indicators and reduced-motion behavior intact.
  - [x] Add ARIA labeling/value text so screen readers announce participant name and current volume.
  - [x] Ensure keyboard interaction supports arrow keys and Home/End with predictable increments.

- [x] Task 5: Add moderator "Kick from voice" placeholder affordance (AC: 6)
  - [x] Update `VoiceParticipant`/`VoicePanel` props to accept moderator capability context from caller.
  - [x] Show a clearly labeled placeholder action only when viewer has `MUTE_MEMBERS` permission.
  - [x] Keep action non-functional in this story (visual placeholder only), with copy indicating Epic 8 ownership.

- [x] Task 6: Integrate volume controls in voice panel/message area wiring (AC: 1, 2, 6)
  - [x] Update `client/src/lib/features/voice/VoicePanel.svelte` to pass slider callbacks and moderator state into participant rows.
  - [x] Update `client/src/lib/features/chat/MessageArea.svelte` wiring to provide permission context from existing guild/member permission data.
  - [x] Ensure panel open/close behavior and existing VoiceBar controls remain unchanged.

- [x] Task 7: Add AC-focused tests and run quality gates (AC: all)
  - [x] Add/expand `VoiceParticipant.test.ts` for slider rendering, keyboard behavior, ARIA labeling, and default value.
  - [x] Add/expand `VoicePanel.test.ts` for participant volume propagation and moderator placeholder visibility.
  - [x] Add/expand `voiceStore.test.ts` for persistence load/save, clamping, identity scoping, and default fallback behavior.
  - [x] Add targeted tests for `webrtcClient.ts` volume application paths (0-100 direct volume, >100 gain node) and cleanup.
  - [x] Update `MessageArea.test.ts` for moderator-context wiring into voice panel.
  - [x] Run quality gates:
    - [x] `cd client && npm run lint && npm run check && npm run test && npm run build`
    - [x] `cd server && cargo fmt --check && cargo clippy -- -D warnings && cargo test` (run fully if any server/wire changes are made)

## Dev Notes

### Developer Context

- Story 7.3 established participant roster rendering, speaking indicators, and `voice_state_update` synchronization across channel list and voice panel.
- Current voice participant UI (`VoiceParticipant.svelte`) is read-only and has no interactive per-user volume controls.
- Current WebRTC client stores remote audio elements in a `Set<HTMLAudioElement>` and does not expose participant-addressable output controls.
- Existing voice state model in `voiceStore.svelte.ts` tracks connection and participant metadata, but not persisted per-participant local volume preferences.
- Existing moderator permission keys already include `MUTE_MEMBERS`, which can gate the placeholder affordance for "Kick from voice".  
  [Source: _bmad-output/implementation-artifacts/7-3-voice-channel-participants-display.md#Previous Story Intelligence]  
  [Source: client/src/lib/features/voice/VoiceParticipant.svelte]  
  [Source: client/src/lib/features/voice/webrtcClient.ts]  
  [Source: client/src/lib/features/voice/voiceStore.svelte.ts]  
  [Source: client/src/lib/features/guild/permissions.ts]

### Technical Requirements

- Implement volume control as **local-only** behavior; do not send volume preference changes over WebSocket or persist server-side.
- Enforce slider/value bounds at all state boundaries: minimum `0`, maximum `200`, default `100`.
- Preserve existing retry/failure user messaging and connection state behavior from Stories 7.1–7.3.
- Keep JSON boundary and WS naming conventions unchanged unless additional participant media metadata is strictly required.
- Avoid introducing separate transport/state channels; reuse existing voice feature boundaries and stores.  
  [Source: _bmad-output/planning-artifacts/epics.md#Story 7.4: Individual Volume Control]  
  [Source: _bmad-output/planning-artifacts/prd.md#Voice Communication]  
  [Source: _bmad-output/planning-artifacts/architecture.md#WebSocket Event Naming]  
  [Source: _bmad-output/implementation-artifacts/7-3-voice-channel-participants-display.md#Technical Requirements]

### Architecture Compliance

1. Keep voice feature implementation within `client/src/lib/features/voice/*`; avoid leaking volume logic into unrelated domains.
2. Keep voice runtime state source-of-truth in `voiceStore.svelte.ts` for panel/channel consumers.
3. Keep wire contracts in `snake_case` at the boundary and map to typed `camelCase` structures in client state.
4. Preserve accessibility behavior already established in Story 7.3 (`aria-live`, reduced-motion-safe speaking indicators).
5. If server payload extension is required for participant/audio mapping, keep it additive and backward compatible.  
   [Source: _bmad-output/planning-artifacts/architecture.md#Structure Patterns]  
   [Source: _bmad-output/planning-artifacts/architecture.md#State Boundaries]  
   [Source: _bmad-output/planning-artifacts/architecture.md#API Boundary (JSON)]  
   [Source: _bmad-output/implementation-artifacts/7-3-voice-channel-participants-display.md#Senior Developer Review (AI)]

### Library & Framework Requirements

- Frontend stack remains Svelte 5 + TypeScript + Vitest (no new UI framework required).
- For audio gain behavior:
  - `HTMLMediaElement.volume` accepts `0..1` only.
  - `GainNode`/`createGain()` is appropriate for amplification above 100%; constructor docs indicate nominal gain range `(-∞,+∞)`.
- IndexedDB is asynchronous and same-origin scoped; use explicit transaction completion/error handling.
- Keep WebRTC path aligned with current `VoiceWebRtcClient` implementation and avoid broad rewrites unrelated to ACs.  
  [Source: https://developer.mozilla.org/en-US/docs/Web/API/HTMLMediaElement/volume]  
  [Source: https://developer.mozilla.org/en-US/docs/Web/API/BaseAudioContext/createGain]  
  [Source: https://developer.mozilla.org/en-US/docs/Web/API/GainNode/GainNode]  
  [Source: https://developer.mozilla.org/en-US/docs/Web/API/IndexedDB_API]  
  [Source: client/src/lib/features/voice/webrtcClient.ts]

### File Structure Requirements

Expected primary touch points:

- `client/src/lib/features/voice/types.ts`
- `client/src/lib/features/voice/voiceStore.svelte.ts`
- `client/src/lib/features/voice/webrtcClient.ts`
- `client/src/lib/features/voice/VoiceParticipant.svelte`
- `client/src/lib/features/voice/VoiceParticipant.test.ts`
- `client/src/lib/features/voice/VoicePanel.svelte`
- `client/src/lib/features/voice/VoicePanel.test.ts`
- `client/src/lib/features/chat/MessageArea.svelte`
- `client/src/lib/features/chat/MessageArea.test.ts`
- `client/src/lib/features/guild/permissions.ts` (read/gate integration only; no permission catalog drift)
- Optional (only if mapping metadata is needed):
  - `server/src/webrtc/signaling.rs`
  - `server/src/ws/gateway.rs`
  - `server/src/ws/protocol.rs`
  - `client/src/lib/ws/protocol.ts`

### Testing Requirements

- Verify slider appears per participant row only in voice panel context and is keyboard operable.
- Verify value mapping behavior:
  - 0% mutes participant output,
  - 100% is neutral/default,
  - >100% uses amplification path without affecting other participants.
- Verify volume changes are local-only and do not emit new WebSocket operations.
- Verify IndexedDB persistence restores values after reconnect/reload for the same viewer identity.
- Verify moderator placeholder action visibility is permission-gated (`MUTE_MEMBERS`) and non-functional.
- Verify no regressions in existing voice controls (mute/deafen/disconnect/participants toggle) and accessibility announcements.

### Previous Story Intelligence

- Preserve the 7.3 fix that prevents contradictory speaking state (`is_speaking` must not persist while muted/deafened).
- Preserve 7.3 accessibility improvements (`aria-live` occupancy announcements in voice panel/channel list).
- Preserve reduced-motion safeguards added to speaking visuals.
- Keep `sendVoiceStateUpdate` context guards intact; volume controls should not introduce stale-context signaling paths.  
  [Source: _bmad-output/implementation-artifacts/7-3-voice-channel-participants-display.md#Senior Developer Review (AI)]  
  [Source: _bmad-output/implementation-artifacts/7-3-voice-channel-participants-display.md#Previous Story Intelligence]

### Git Intelligence Summary

Recent commits relevant to this story:

- `a40554c` feat: complete story 7-3 voice participants display
- `3cb1816` feat: complete story 7-2 voice controls
- `9550eb0` chore: record story 7-1 review rerun
- `96c6188` fix: harden voice signaling state flow
- `2e85be1` chore: finalize 6-12 review

Actionable implications:

- Keep voice work incremental and concentrated in existing voice feature modules.
- Preserve reliability and UX copy conventions established in prior voice stories.
- Pair every store/UI/media pipeline change with direct tests in the same story implementation.

### Latest Technical Information

- `HTMLMediaElement.volume` supports only `0..1`, so amplified playback needs a gain node path.
- `AudioContext.createGain()` and `GainNode.gain` are the standard Web Audio APIs for runtime gain control.
- `GainNode` constructor options document gain's nominal range as `(-∞,+∞)`, supporting amplification above unity.
- IndexedDB remains asynchronous and same-origin constrained, requiring explicit transaction lifecycle handling.  
  [Source: https://developer.mozilla.org/en-US/docs/Web/API/HTMLMediaElement/volume]  
  [Source: https://developer.mozilla.org/en-US/docs/Web/API/BaseAudioContext/createGain]  
  [Source: https://developer.mozilla.org/en-US/docs/Web/API/GainNode/GainNode]  
  [Source: https://developer.mozilla.org/en-US/docs/Web/API/IndexedDB_API]

### Project Context Reference

- No `project-context.md` file was discovered via `**/project-context.md`.
- Story context is derived from planning artifacts, existing implementation artifacts, current repository code, and current technical references.

### Story Completion Status

- Workflow analysis complete.
- Story document generated and ready for implementation handoff.
- Sprint status target for this story: `ready-for-dev`.
- Completion note: Ultimate context analysis completed - comprehensive developer guide created.

### Project Structure Notes

- `VoiceParticipant.svelte` is currently a non-interactive row component; this story introduces first interactive control inside participant rows.
- `webrtcClient.ts` remote output currently uses a `Set<HTMLAudioElement>` with no participant keying, which is a primary integration risk for per-user volume.
- IndexedDB helper patterns already exist in identity feature modules (`blockStore.svelte.ts`, `crypto.ts`); prefer reuse/extraction over duplicating ad-hoc storage code.
- No dedicated slider UI component currently exists under `client/src/lib/components/ui/`; native range input is the lowest-risk path unless a shared slider is introduced deliberately.

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Story 7.4: Individual Volume Control]
- [Source: _bmad-output/planning-artifacts/prd.md#Voice Communication]
- [Source: _bmad-output/planning-artifacts/prd.md#Non-Functional Requirements]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Journey 5: Voice Channel Lifecycle]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#VoiceParticipant]
- [Source: _bmad-output/planning-artifacts/architecture.md#WebSocket Event Naming]
- [Source: _bmad-output/planning-artifacts/architecture.md#API Boundary (JSON)]
- [Source: _bmad-output/planning-artifacts/architecture.md#State Boundaries]
- [Source: _bmad-output/implementation-artifacts/7-3-voice-channel-participants-display.md]
- [Source: _bmad-output/implementation-artifacts/sprint-status.yaml]
- [Source: client/src/lib/features/voice/VoiceParticipant.svelte]
- [Source: client/src/lib/features/voice/VoicePanel.svelte]
- [Source: client/src/lib/features/voice/voiceStore.svelte.ts]
- [Source: client/src/lib/features/voice/webrtcClient.ts]
- [Source: client/src/lib/features/chat/MessageArea.svelte]
- [Source: client/src/lib/features/guild/permissions.ts]
- [Source: client/src/lib/features/identity/blockStore.svelte.ts]
- [Source: client/src/lib/features/identity/crypto.ts]
- [Source: server/src/ws/gateway.rs]
- [Source: server/src/webrtc/signaling.rs]
- [Source: server/src/ws/protocol.rs]
- [Source: client/src/lib/ws/protocol.ts]
- [Source: https://developer.mozilla.org/en-US/docs/Web/API/HTMLMediaElement/volume]
- [Source: https://developer.mozilla.org/en-US/docs/Web/API/BaseAudioContext/createGain]
- [Source: https://developer.mozilla.org/en-US/docs/Web/API/GainNode/GainNode]
- [Source: https://developer.mozilla.org/en-US/docs/Web/API/IndexedDB_API]

## Dev Agent Record

### Agent Model Used

GPT-5.3-Codex (model ID: gpt-5.3-codex)

### Debug Log References

- Loaded workflow engine and create-story instructions.
- Resolved target story from user input and sprint status (`7-4-individual-volume-control`).
- Analyzed epics/architecture/PRD/UX artifacts, previous story intelligence, current voice implementation, and latest technical references.
- Implemented per-participant volume domain contracts, shared clamp/scalar helpers, and voice store selectors/actions for participant-local volume state.
- Added IndexedDB-backed participant volume persistence scoped by viewer identity with explicit toast-based error surfacing on load/save failures.
- Refactored WebRTC remote playback management to participant-addressable outputs with 0-100 media-element volume and 101-200 Web Audio gain amplification plus cleanup.
- Added interactive VoiceParticipant slider controls, ARIA/value semantics, and moderator-only "Kick from voice" placeholder wiring via VoicePanel and MessageArea permission context.
- Expanded voice and chat tests (VoiceParticipant, VoicePanel, voiceStore, webrtcClient, MessageArea) and re-ran client quality gates successfully.

### Completion Notes List

- Completed Story 7.4 implementation across voice state, persistence, playback pipeline, UI, and moderation-context wiring.
- Volume preferences now clamp to 0..200 with a shared scalar conversion source-of-truth and are restored per viewer identity from IndexedDB.
- Participant playback now supports amplification above 100% through GainNode while preserving deafen semantics and resource cleanup on disconnect.
- Voice participant rows now expose click-to-expand slider controls with ARIA/value text and keyboard-operable range behavior.
- Moderator context now gates a non-functional "Kick from voice (Epic 8 placeholder)" affordance in participant controls.
- Client quality gates passed: `npm run lint`, `npm run check`, `npm run test`, and `npm run build`.

### File List

- _bmad-output/implementation-artifacts/7-4-individual-volume-control.md
- _bmad-output/implementation-artifacts/sprint-status.yaml
- client/src/lib/features/chat/MessageArea.svelte
- client/src/lib/features/chat/MessageArea.test.ts
- client/src/lib/features/voice/participantVolume.ts
- client/src/lib/features/voice/participantVolumeStore.svelte.ts
- client/src/lib/features/voice/types.ts
- client/src/lib/features/voice/VoicePanel.svelte
- client/src/lib/features/voice/VoicePanel.test.ts
- client/src/lib/features/voice/VoiceParticipant.svelte
- client/src/lib/features/voice/VoiceParticipant.test.ts
- client/src/lib/features/voice/voiceStore.svelte.ts
- client/src/lib/features/voice/voiceStore.test.ts
- client/src/lib/features/voice/webrtcClient.ts
- client/src/lib/features/voice/webrtcClient.test.ts

## Senior Developer Review (AI)

- Reviewer: Darko
- Date: 2026-03-02
- Outcome: Approve (after fix)
- Git vs Story File List Discrepancies: 0

### Findings

1. **MEDIUM** `client/src/lib/features/voice/voiceStore.svelte.ts` queued participant-volume saves using live store state, which could persist the wrong preference map after owner-context reset while saves were queued.
   - **Fix applied:** persistence queue now captures a snapshot payload at queue time and writes that snapshot (`queueParticipantVolumePersistence(ownerUserId, persistedVolumes)`).
   - **Regression test added:** `queues persistence snapshots without leaking across owner resets` in `client/src/lib/features/voice/voiceStore.test.ts`.

### Validation

- `cd client && npm run lint && npm run check`
- `cd client && npm run test`
- `cd client && npm run build`
- `cd server && cargo fmt --check && cargo clippy -- -D warnings && cargo test`

## Change Log

- 2026-03-01: Created Story 7.4 with comprehensive implementation context and marked status as `ready-for-dev`.
- 2026-03-02: Implemented individual participant volume controls, local IndexedDB persistence, playback gain path, moderator placeholder wiring, and AC-focused client test coverage; status moved to `review`.
- 2026-03-02: YOLO code review fixed queued volume-persistence snapshot leakage across owner resets, added regression coverage, and moved story status to `done`.
