# Story 7.7: Mobile Voice Controls

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a **mobile user**,
I want touch-friendly voice controls,
so that I can manage voice on my phone without precision tapping.

## Acceptance Criteria

1. **Given** a user is in a voice channel on a mobile device (<768px)  
   **When** they view the voice controls  
   **Then** a compact voice bar appears at the top of the message area (channel name + mute icon).

2. **Given** a user is on mobile with voice active  
   **When** they swipe up on the voice bar  
   **Then** a full bottom sheet opens.

3. **Given** the voice bottom sheet is open  
   **When** controls are displayed  
   **Then** the sheet shows: large mute button (64px, center), large deafen button (64px), disconnect button (fire red, 64px), participant list with volume controls, and connection quality indicator.

4. **Given** mobile voice controls are shown  
   **When** the user interacts with them  
   **Then** all touch targets are at minimum 48px with 8px spacing between adjacent targets.

5. **Given** the user navigates mobile panels while still in voice  
   **When** browsing channels/guild/member panels  
   **Then** the bottom sheet stays available for voice control while navigation continues.

6. **Given** the bottom sheet is open  
   **When** the user swipes down or taps outside  
   **Then** the sheet dismisses.

## Tasks / Subtasks

- [x] Task 1: Add mobile compact voice bar behavior and placement (AC: 1, 4)
  - [x] Extend `VoiceBar.svelte` with a mobile variant (`<768px`) that keeps the compact header pattern (channel name + mute-first control) while preserving current desktop/tablet behavior.
  - [x] Ensure the mobile bar is rendered at the top of the message area and remains visually distinct from reconnect/error banners.
  - [x] Enforce minimum 48px touch targets and 8px spacing for mobile controls.

- [x] Task 2: Implement mobile voice bottom sheet UX (AC: 2, 3, 4, 6)
  - [x] Add a dedicated mobile sheet container for voice controls (new component or `VoicePanel` mobile mode) with swipe-up open and swipe-down/tap-outside dismiss.
  - [x] Render required large controls (64px mute/deafen/disconnect), participant list, participant volume controls, and quality indicator.
  - [x] Ensure bottom-sheet transitions respect reduced-motion preferences and keep keyboard/screen-reader accessibility intact.

- [x] Task 3: Keep mobile voice controls available across panel navigation (AC: 5)
  - [x] Lift mobile voice-sheet visibility state to a scope that survives `ShellRoute` mobile panel switches (`guilds/channels/messages/members`), instead of coupling solely to `MessageArea` mount state.
  - [x] Preserve existing voice connection lifecycle logic in `voiceStore.svelte.ts` and avoid changing signaling contracts.

- [x] Task 4: Preserve and validate accessibility and interaction semantics (AC: 1-6)
  - [x] Keep explicit `aria-label` values for mute/deafen/disconnect and maintain meaningful live-region announcements for connection state.
  - [x] Ensure touch interaction uses pointer/touch-safe behavior without disabling page zoom globally.
  - [x] Validate that dismissal gestures do not interfere with normal timeline scrolling or focus management.

- [x] Task 5: Expand regression coverage for mobile voice controls (AC: all)
  - [x] Update/add component tests in `VoiceBar.test.ts`, `VoicePanel.test.ts`, and `MessageArea.test.ts` for mobile compact mode, large control sizing hooks, and sheet open/close behavior.
  - [x] Update/add shell tests in `ShellRoute.test.ts` for mobile panel switching while voice controls remain available.
  - [x] Run quality gates:
    - [x] `cd client && npm run lint && npm run check && npm run test && npm run build`
    - [x] `cd server && cargo fmt --check && cargo clippy -- -D warnings && cargo test`

## Dev Notes

### Developer Context

- Current mobile shell mode swaps between panel components (`GuildRail`, `ChannelList`, `MemberList`, `MessageArea`) using `mobilePanel`; `MessageArea` unmounts outside the `messages` panel. This is the key constraint for AC5.  
  [Source: client/src/lib/features/shell/ShellRoute.svelte]
- `MessageArea` currently owns `participantsOpen` and renders `VoiceBar` + `VoicePanel` when in channel mode; this ownership is likely too local for persistent mobile voice sheet availability across panel switches.  
  [Source: client/src/lib/features/chat/MessageArea.svelte]
- Voice connection lifecycle is centralized in `voiceStore.svelte.ts` (connect/retry/reconnect/disconnect/status messaging). Story 7.7 should keep this logic intact and focus on mobile interaction/presentation.  
  [Source: client/src/lib/features/voice/voiceStore.svelte.ts]

### Technical Requirements

- Preserve Story 7.6 reconnect semantics and copy (`Reconnecting...`, `Connection lost`, and terminal failure messaging).
- Do not introduce new websocket operations or backend API changes for this story.
- Keep touch targets >=48px and spacing >=8px on mobile controls.
- Support swipe-up to open and swipe-down/tap-outside to dismiss without breaking vertical content scrolling.
- Ensure mobile layout works at narrow widths and with safe-area insets.

### Architecture Compliance

1. Keep voice state ownership in `voiceStore.svelte.ts`; UI components consume derived state.
2. Preserve existing frontend feature boundaries (`features/shell`, `features/chat`, `features/voice`).
3. Maintain accessibility-first interaction patterns (labels, focus, live regions, keyboard parity).
4. Keep existing route-driven voice activation behavior in `ShellRoute` unless AC implementation requires controlled UI-state relocation only.
5. Preserve no-SSR SPA constraints and Svelte 5 runes conventions.

### Library & Framework Requirements

- Frontend stack remains Svelte 5 + TypeScript + Tailwind + Vitest.
- Router remains `@mateothegreat/svelte5-router` (current project choice); no routing library changes in this story.  
  [Source: _bmad-output/planning-artifacts/architecture.md#Frontend Architecture]  
  [Source: https://github.com/mateothegreat/svelte5-router]
- Keep server WebRTC path unchanged (webrtc-rs v0.17.x stability branch context still applies).  
  [Source: _bmad-output/planning-artifacts/architecture.md#Architecture Validation Results]  
  [Source: https://docs.rs/crate/webrtc/latest]
- For gesture-safe mobile behavior:
  - Use `touch-action` thoughtfully (avoid broad `none` that blocks expected browser behaviors).  
    [Source: https://developer.mozilla.org/en-US/docs/Web/CSS/touch-action]
  - Respect viewport safe areas with CSS `env(safe-area-inset-*)` where needed.  
    [Source: https://developer.mozilla.org/en-US/docs/Web/CSS/env]
  - Prefer pointer events for unified mouse/touch/pen handling in sheet drag/dismiss interactions.  
    [Source: https://developer.mozilla.org/en-US/docs/Web/API/Pointer_events]

### File Structure Requirements

Expected primary touch points:

- `client/src/lib/features/voice/VoiceBar.svelte`
- `client/src/lib/features/voice/VoicePanel.svelte` (or new mobile-sheet companion in same feature module)
- `client/src/lib/features/chat/MessageArea.svelte`
- `client/src/lib/features/shell/ShellRoute.svelte`
- `client/src/lib/features/voice/VoiceBar.test.ts`
- `client/src/lib/features/voice/VoicePanel.test.ts`
- `client/src/lib/features/chat/MessageArea.test.ts`
- `client/src/lib/features/shell/ShellRoute.test.ts`

### Testing Requirements

- Verify compact voice bar appears in mobile viewport when voice is active.
- Verify swipe/tap interactions open and dismiss mobile voice sheet.
- Verify sheet shows required controls and participant/volume sections.
- Verify mobile controls meet touch target/spacing requirements.
- Verify voice sheet remains available during mobile panel navigation.
- Verify reconnect state messaging remains correct in mobile and desktop variants.
- Verify keyboard and screen-reader accessibility is preserved.

### Previous Story Intelligence

- Story 7.6 already hardened reconnect lifecycle behavior; this story should reuse its state model and avoid logic duplication in UI components.
- Story 7.6 showed the importance of scoping voice UI visibility to correct connection states (`failed` only when it represents reconnect-lost, not first-time join failure).
- Voice UI tests and integration tests were expanded in 7.6; extend these patterns instead of introducing ad-hoc coverage.
  [Source: _bmad-output/implementation-artifacts/7-6-voice-auto-reconnect.md]

### Git Intelligence Summary

Recent voice development sequence:

- `e1f99f1` feat: complete story 7-6 voice auto-reconnect
- `97da3f1` feat: complete story 7-5 voice channel switching
- `759b02f` feat: complete story 7-4 individual volume control
- `a40554c` feat: complete story 7-3 voice participants display

Actionable implications:

- Continue incremental voice work in established files (`voiceStore`, `VoiceBar`, `VoicePanel`, `MessageArea`, `ShellRoute`).
- Preserve test-first guardrail pattern for voice stories.
- Avoid protocol churn; keep story scope centered on mobile UX behavior.

### Latest Technical Information

- `touch-action: manipulation` helps responsiveness while preserving pan/pinch behavior, and broad `touch-action: none` can harm accessibility if misused.
- `env(safe-area-inset-*)` should be considered for bottom-sheet and fixed control padding on notched devices.
- Pointer Events provide a unified model (`pointerdown/move/up/cancel`) suitable for swipe-open/swipe-dismiss sheet gestures.
- webrtc-rs guidance continues to position v0.17.x as the stable Tokio-coupled path; no migration is needed for this UI story.
  [Source: https://developer.mozilla.org/en-US/docs/Web/CSS/touch-action]
  [Source: https://developer.mozilla.org/en-US/docs/Web/CSS/env]
  [Source: https://developer.mozilla.org/en-US/docs/Web/API/Pointer_events]
  [Source: https://docs.rs/crate/webrtc/latest]

### Project Context Reference

- No `project-context.md` was discovered via `**/project-context.md`.
- Context for this story is derived from epics/PRD/architecture/UX artifacts, prior story files, current code, and web standards references.

### Story Completion Status

- Workflow analysis complete for story `7-7-mobile-voice-controls`.
- Story document generated and prepared for implementation handoff.
- Sprint status target for this story: `ready-for-dev`.
- Completion note: Ultimate context engine analysis completed - comprehensive developer guide created.

### Project Structure Notes

- Mobile panel routing currently lives in `ShellRoute`; voice controls rendered inside `MessageArea` do not persist when non-message mobile panels are active.
- Voice participant and volume data already flow through `voiceStore` and `VoicePanel`; mobile bottom-sheet work should reuse this data path.
- Keep custom voice UI components in `client/src/lib/features/voice/` to align with feature-module boundaries.

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Story 7.7: Mobile Voice Controls]
- [Source: _bmad-output/planning-artifacts/epics.md#Epic 7: Voice Communication]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Mobile voice]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Responsive Design & Accessibility]
- [Source: _bmad-output/planning-artifacts/architecture.md#Frontend Architecture]
- [Source: _bmad-output/planning-artifacts/architecture.md#Implementation Patterns & Consistency Rules]
- [Source: _bmad-output/implementation-artifacts/7-6-voice-auto-reconnect.md]
- [Source: _bmad-output/implementation-artifacts/sprint-status.yaml]
- [Source: client/src/lib/features/shell/ShellRoute.svelte]
- [Source: client/src/lib/features/chat/MessageArea.svelte]
- [Source: client/src/lib/features/voice/VoiceBar.svelte]
- [Source: client/src/lib/features/voice/VoicePanel.svelte]
- [Source: client/src/lib/features/voice/voiceStore.svelte.ts]
- [Source: client/src/lib/features/shell/ShellRoute.test.ts]
- [Source: client/src/lib/features/chat/MessageArea.test.ts]
- [Source: client/src/lib/features/voice/VoiceBar.test.ts]
- [Source: https://github.com/mateothegreat/svelte5-router]
- [Source: https://docs.rs/crate/webrtc/latest]
- [Source: https://developer.mozilla.org/en-US/docs/Web/CSS/touch-action]
- [Source: https://developer.mozilla.org/en-US/docs/Web/CSS/env]
- [Source: https://developer.mozilla.org/en-US/docs/Web/API/Pointer_events]

## Dev Agent Record

### Agent Model Used

GPT-5.3-Codex (model ID: gpt-5.3-codex)

### Debug Log References

- Implemented `VoiceBar` mobile compact variant with mute-first control, 48px+ touch target sizing, and tap/swipe-up open affordance.
- Added ShellRoute-managed mobile voice sheet (persistent across mobile panel switches) with backdrop/gesture dismiss, 64px mute/deafen/disconnect controls, quality indicator, and participant list.
- Kept voice lifecycle ownership in `voiceStore.svelte.ts`; no signaling/backend contract changes were made.
- Updated `MessageArea` to support shell-managed mobile voice controls while preserving desktop/tablet behavior.
- Expanded tests across voice/chat/shell modules for mobile controls and persistence behavior.
- Executed full quality gates for client and server.

### Completion Notes List

- ✅ AC1/AC4: Mobile compact voice bar now renders from ShellRoute at the top of mobile content with touch-safe control sizing and spacing.
- ✅ AC2/AC3/AC6: Mobile voice bottom sheet opens from compact bar, supports dismiss via tap-outside and swipe-down handle gesture, and includes required large controls + quality indicator + participant/volume controls.
- ✅ AC5: Mobile voice controls persist while switching mobile panels (`guilds/channels/messages/members`) because visibility state is managed in ShellRoute scope.
- ✅ Accessibility: Explicit aria-labels retained for mute/deafen/disconnect, live-region status messaging preserved, and dialog semantics added to mobile sheet.
- ✅ Validation: `cd client && npm run lint && npm run check && npm run test -- --run && npm run build` and `cd server && cargo fmt --check && cargo clippy -- -D warnings && cargo test`.

### File List

- client/src/lib/features/voice/VoiceBar.svelte
- client/src/lib/features/voice/VoicePanel.svelte
- client/src/lib/features/chat/MessageArea.svelte
- client/src/lib/features/shell/ShellRoute.svelte
- client/src/lib/features/voice/VoiceBar.test.ts
- client/src/lib/features/voice/VoicePanel.test.ts
- client/src/lib/features/chat/MessageArea.test.ts
- client/src/lib/features/shell/ShellRoute.test.ts
- client/src/lib/features/shell/__mocks__/MessageAreaMock.svelte
- _bmad-output/implementation-artifacts/sprint-status.yaml
- _bmad-output/implementation-artifacts/7-7-mobile-voice-controls.md

## Senior Developer Review (AI)

### Reviewer

Darko

### Outcome

Approved

### Summary

- Acceptance Criteria (AC1-AC6) validated against implementation in reviewed source files.
- Git vs Story File List discrepancies: 0.
- Findings: 0 High, 0 Medium, 0 Low.
- Quality gates verified:
  - `cd client && npm run lint && npm run check && npm run test -- --run && npm run build`
  - `cd server && cargo fmt --check && cargo clippy -- -D warnings && cargo test`

## Change Log

- 2026-03-02: Implemented Story 7.7 mobile voice controls, added persistent ShellRoute mobile voice sheet UX, updated related component/integration tests, and passed full client/server quality gates.
- 2026-03-02: Senior developer code review completed in YOLO mode with no findings; story status set to done.
