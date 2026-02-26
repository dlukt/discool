# Story 4.1: SPA Navigation Shell and Routing

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a **user**,
I want a familiar navigation layout with guild icons, channel list, and message area,
so that I can navigate Discool intuitively without learning a new pattern.

## Acceptance Criteria

1. **Given** a user is authenticated  
   **When** the SPA loads  
   **Then** the four-panel desktop layout is rendered: GuildRail (72px) | ChannelList (240px) | MessageArea (flex-1) | MemberList (240px)

2. **Given** a user is navigating guild/channel views  
   **When** they change location  
   **Then** `@mateothegreat/svelte5-router` handles client-side navigation with unique URLs per guild/channel (`instance.com/guild/channel`)

3. **Given** browser navigation controls are used  
   **When** the user presses back/forward  
   **Then** channel navigation follows browser history correctly

4. **Given** the user refreshes on a guild/channel URL  
   **When** the app reloads  
   **Then** they return to the same view

5. **Given** a tablet viewport (768-1023px)  
   **When** the navigation shell renders  
   **Then** the member list is hidden by default and available via toggle

6. **Given** a mobile viewport (<768px)  
   **When** the navigation shell renders  
   **Then** a single-panel drill-down with bottom navigation bar is shown

7. **Given** initial app load on 4G  
   **When** loading completes  
   **Then** SPA initial load is under 3 seconds (NFR4)

8. **Given** the initial frontend bundle is built  
   **When** size is measured  
   **Then** initial bundle is under 500KB gzipped (NFR6)

9. **Given** a keyboard-only user enters the app  
   **When** focus starts at the top of the page  
   **Then** a skip-to-content link is the first focusable element

## Tasks / Subtasks

- [x] Task 1: Introduce router-based authenticated shell and route model (AC: 2, 3, 4)
  - [x] Add `@mateothegreat/svelte5-router` dependency and create `client/src/routes/routes.ts` route definitions for home, guild/channel, settings, and admin.
  - [x] Replace `view`-only authenticated branching in `App.svelte` with router-driven view rendering while preserving setup/login/recovery flows.
  - [x] Ensure deep links and browser `popstate` navigation behave consistently for guild/channel routes.

- [x] Task 2: Build desktop four-panel shell components (AC: 1)
  - [x] Add baseline components for GuildRail, ChannelList, MessageArea, and MemberList in feature folders.
  - [x] Implement desktop widths exactly: 72px / 240px / flex / 240px.
  - [x] Keep existing authenticated actions (admin/settings/logout) reachable from the new shell.

- [x] Task 3: Implement responsive behavior for tablet and mobile (AC: 5, 6)
  - [x] Tablet: hide member list by default at 768-1023px and expose a toggle in shell header controls.
  - [x] Mobile: implement single-panel drill-down flow and bottom navigation bar placeholders.
  - [x] Preserve keyboard/touch usability for navigation transitions.

- [x] Task 4: Wire route persistence and loading placeholders (AC: 4, 8)
  - [x] Integrate `saveLastLocation`/`getLastLocation` with route changes so last guild/channel can be restored when valid.
  - [x] Keep admin/settings paths excluded from persisted last location.
  - [x] Show skeleton loaders during initial shell bootstrap.

- [x] Task 5: Accessibility baseline for routed shell (AC: 9, 2, 3)
  - [x] Add skip-to-content link as the first focusable element in authenticated shell.
  - [x] Ensure skip target is focusable (`main` with `tabindex="-1"`) and receives focus on route change.
  - [x] Add/confirm ARIA labels and keyboard navigation semantics for guild and channel controls.

- [x] Task 6: Performance and bundle-size guardrails (AC: 7, 8)
  - [x] Use route/component lazy loading where appropriate to protect initial bundle budget.
  - [x] Verify load-time and size targets using Lighthouse/build output before completion.

- [x] Task 7: Testing and quality gates (AC: all)
  - [x] Add/extend tests for route deep-link restore, history navigation, responsive panel behavior, and skip-link focus behavior.
  - [x] Run client quality gate: `cd client && npm run lint && npm run check && npm run test`.

## Dev Notes

### Developer Context

- Current authenticated UI in `client/src/App.svelte` is state-driven (`view = home|admin|settings`) and does not yet use route definitions.
- Story 2.3 intentionally deferred true route persistence to Epic 4 and still has an open router-related AC gap; this story is the integration point.
- Server static handler already supports SPA deep-link refresh (`/guild/channel` falls back to `index.html`), so this story should stay client-focused.

### Technical Requirements

- Preserve all existing setup/auth/recovery state-machine branches while introducing routed authenticated views.
- Follow Svelte 5 runes conventions and existing event attribute style (`onclick`), matching current codebase patterns.
- Keep API boundary contracts intact (`{"data": ...}` envelope, snake_case wire fields) and avoid speculative backend changes for later Epic 4 stories.
- Stabilize route contracts now so upcoming stories (4.2-4.7) can attach real guild/channel data and behavior without route churn.

### Architecture Compliance

1. Keep route declarations in `client/src/routes/routes.ts` and app composition in `client/src/App.svelte`.  
2. Place new shell modules in architecture-aligned feature folders (`features/guild`, `features/channel`, `features/chat`, `features/members`).  
3. Enforce responsive breakpoints and panel proportions from UX/architecture docs without introducing a separate mobile codebase.  
4. Preserve current server/client boundaries; no `guilds/channels` backend handlers exist yet in runtime code.

### Library & Framework Requirements

- Router: `@mateothegreat/svelte5-router` (architecture baseline 2.15.x; latest stable is 2.16.19 — use a compatible 2.16.x release unless pinned constraints require 2.15.x).
- Framework/tooling: stay on existing Svelte 5 + Vite + Tailwind v4 stack already in repo.
- Styling: use existing design tokens in `client/src/app.css` and shadcn-svelte-compatible utility patterns.

### File Structure Requirements

Expected primary touch points:

- `client/package.json`
- `client/src/App.svelte`
- `client/src/routes/routes.ts` (new)
- `client/src/lib/features/guild/GuildRail.svelte` (new)
- `client/src/lib/features/channel/ChannelList.svelte` (new)
- `client/src/lib/features/chat/MessageArea.svelte` (new)
- `client/src/lib/features/members/MemberList.svelte` (new)
- `client/src/lib/features/identity/navigationState.ts` (integration)
- `client/src/lib/features/**/**/*.test.ts` (new/updated)

### Testing Requirements

- Validate AC coverage for route history/deep-link behavior, responsive shell behavior, skeleton loading, and skip-link accessibility.
- Re-run current client quality gate:
  - `cd client && npm run lint`
  - `cd client && npm run check`
  - `cd client && npm run test`

### Latest Technical Information

1. `@mateothegreat/svelte5-router` latest stable release is **2.16.19**, with native History API support (`pushState`/`replaceState`/`popstate`) and named-parameter route configs.
2. Svelte latest stable in the Svelte 5 line is **5.53.x**; this story can keep the repo's pinned Svelte version while applying compatible router patterns.
3. SPA accessibility best practice for skip links: make skip link first in tab order, target focusable main content (`tabindex="-1"`), and move focus on route transitions.

### Project Context Reference

- No `project-context.md` was discovered via `**/project-context.md`.
- Story context comes from epics/PRD/architecture/UX artifacts and current client/server source.

### Story Completion Status

- Ultimate context engine analysis completed - comprehensive developer guide created.
- Story marked `ready-for-dev`.

### Project Structure Notes

- Story aligns with architecture-defined route and feature-module structure.
- Introducing `guild/channel/chat/members` feature folders is intentional and expected for Epic 4 foundation work.

### References

- [Source: _bmad-output/planning-artifacts/epics.md#Epic 4: Guilds, Channels & Invites]
- [Source: _bmad-output/planning-artifacts/epics.md#Story 4.1: SPA Navigation Shell and Routing]
- [Source: _bmad-output/planning-artifacts/prd.md#Guild Management]
- [Source: _bmad-output/planning-artifacts/prd.md#User Experience & Navigation]
- [Source: _bmad-output/planning-artifacts/prd.md#Non-Functional Requirements]
- [Source: _bmad-output/planning-artifacts/architecture.md#Frontend Architecture]
- [Source: _bmad-output/planning-artifacts/architecture.md#Structure Patterns]
- [Source: _bmad-output/planning-artifacts/architecture.md#Requirements to Structure Mapping]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Spacing & Layout Foundation]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Navigation Patterns]
- [Source: _bmad-output/planning-artifacts/ux-design-specification.md#Responsive Design & Accessibility]
- [Source: _bmad-output/implementation-artifacts/2-3-identity-persistence-and-auto-login.md]
- [Source: client/src/App.svelte]
- [Source: client/src/lib/features/identity/navigationState.ts]
- [Source: server/src/static_files.rs]
- [Source: https://www.npmjs.com/package/@mateothegreat/svelte5-router]
- [Source: https://github.com/mateothegreat/svelte5-router/releases]
- [Source: https://github.com/sveltejs/svelte/releases]
- [Source: https://www.w3.org/WAI/WCAG21/Techniques/general/G1]

## Dev Agent Record

### Agent Model Used

GPT-5.3-Codex (model ID: gpt-5.3-codex)

### Debug Log References

- Workflow engine loaded: `_bmad/core/tasks/workflow.xml`
- Workflow config loaded: `_bmad/bmm/workflows/4-implementation/dev-story/workflow.yaml`
- Red phase validation: `cd client && npm run test -- --run src/routes/routes.test.ts src/lib/features/shell/ShellRoute.test.ts`
- Quality gate validation: `cd client && npm run lint && npm run check && npm run test`

### Completion Notes List

- Replaced authenticated `view` switching in `App.svelte` with `@mateothegreat/svelte5-router` routes while preserving setup/login/recovery branches.
- Added routed shell foundation components in `features/guild`, `features/channel`, `features/chat`, `features/members`, and `features/shell`.
- Implemented desktop/tablet/mobile responsive shell behavior, including tablet member toggle and mobile drill-down with bottom navigation.
- Integrated route persistence restoration via `saveLastLocation`/`getLastLocation`, excluding `/admin` and `/settings`, with shell bootstrap skeleton.
- Added routed-shell accessibility baseline (skip link first, focusable main target, focus-on-route-change) and ARIA labels for guild/channel navigation.
- Added route and shell tests (`routes.test.ts`, `ShellRoute.test.ts`) and validated full client quality gate.
- Verified production build output from `npm run build`: `index` 38.27kB gzip + `ShellRoute` 8.96kB gzip + CSS 5.40kB gzip, keeping initial payload comfortably under the 500kB gzip budget.

### File List

- _bmad-output/implementation-artifacts/4-1-spa-navigation-shell-and-routing.md
- client/package.json
- client/package-lock.json
- client/src/App.svelte
- client/src/routes/routes.ts
- client/src/routes/routes.test.ts
- client/src/lib/features/guild/GuildRail.svelte
- client/src/lib/features/channel/ChannelList.svelte
- client/src/lib/features/chat/MessageArea.svelte
- client/src/lib/features/members/MemberList.svelte
- client/src/lib/features/shell/ShellRoute.svelte
- client/src/lib/features/shell/ShellRoute.test.ts
- _bmad-output/implementation-artifacts/sprint-status.yaml

## Change Log

- 2026-02-26: Story created and marked ready-for-dev with comprehensive implementation context.
- 2026-02-26: Implemented router-based authenticated shell, responsive panel layouts, route persistence, accessibility baseline, and routed shell tests; story moved to review.
- 2026-02-26: Senior developer review completed in YOLO mode; no material findings in reviewed source files; story marked done.

## Senior Developer Review (AI)

- Outcome: **Approved**
- Git vs Story File List discrepancies (source files): **0**
- Findings: **No high/medium/low material issues found** in reviewed `client/` source changes.
- Validation run: `cd client && npm run lint && npm run check && npm run test` (passed)
