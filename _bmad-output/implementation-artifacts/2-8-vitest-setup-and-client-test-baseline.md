# Story 2.8: Vitest Setup and Client Test Baseline

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a **developer**,
I want a frontend unit/component test harness (Vitest + @testing-library/svelte),
So that we stop accruing client test debt and can safely evolve identity/auth UX.

## Acceptance Criteria

1. **Given** the client project dependencies are installed
   **When** `cd client && npm run test` is executed
   **Then** Vitest runs in CI-friendly mode (non-watch) and exits non-zero on failures
   **And** the default test environment is `jsdom`
   **And** `$lib` alias resolution works in tests
   **And** `@testing-library/svelte` is configured for component tests

2. **Given** GitHub Actions CI runs on a pull request
   **When** the client job executes
   **Then** `npm run test` is executed in addition to lint/check/build

3. **Given** the identity/auth client modules from Stories 2.1–2.3 exist
   **When** the test suite is run
   **Then** there are deterministic unit tests covering core non-UI logic (crypto helpers, storage utilities, session persistence/restore)
   **And** there are smoke-level component tests for the identity recovery prompts

## Tasks / Subtasks

- [x] Task 1: Add Vitest + testing dependencies and npm scripts
  - [x] 1.1 Add devDependencies in `client/package.json`:
    - `vitest`
    - `jsdom`
    - `@testing-library/svelte`
    - `@testing-library/jest-dom`
    - `@vitest/coverage-v8` (optional but recommended)
  - [x] 1.2 Add scripts:
    - `test` (CI-friendly, non-watch)
    - `test:watch` (local dev)
    - `test:ui` (optional)

- [x] Task 2: Configure Vitest for Svelte + TypeScript + $lib alias
  - [x] 2.1 Create `client/vitest.config.ts`:
    - Uses the Svelte Vite plugin
    - `test.environment = 'jsdom'`
    - `test.setupFiles` points at a test setup file that enables jest-dom matchers
    - Ensures `$lib` alias works the same as in `vite.config.ts`
  - [x] 2.2 Create `client/src/test/setup.ts` that imports `@testing-library/jest-dom/vitest`
  - [x] 2.3 Ensure `vitest` types are available to TypeScript (avoid `any` / implicit globals)

- [x] Task 3: Pay down client test debt for Epic 2 (Stories 2.1–2.3)
  - [x] 3.1 Unit tests for `client/src/lib/features/identity/crypto.ts`:
    - `didKeyFromPublicKey()` produces `did:key:z6Mk...` format
    - base58 helpers match known vectors
    - `loadStoredIdentity()` returns `'none' | 'found' | 'corrupted'` correctly (Story 2.3 behavior)
  - [x] 3.2 Unit tests for `client/src/lib/features/identity/navigationState.ts`:
    - save/get/clear round-trip on localStorage
  - [x] 3.3 Unit tests for session persistence in `identityStore.svelte.ts` (Story 2.3 behavior):
    - session persists to localStorage
    - session restores on initialize
    - expired session triggers re-authentication

- [x] Task 4: Smoke-level component tests for identity recovery UI (Story 2.3)
  - [x] 4.1 `RecoveryPrompt.svelte`:
    - renders expected copy
    - "Start fresh" triggers the clear/start-fresh action
    - "Recover via email" is disabled (placeholder)
  - [x] 4.2 `ReRegisterPrompt.svelte`:
    - renders stored username
    - "Register" triggers re-register flow (mock identity API)

- [x] Task 5: Enforce tests in CI
  - [x] 5.1 Update `.github/workflows/ci.yml` client job to run `npm run test`

- [x] Task 6: Verify quality gates
  - [x] 6.1 `cd client && npm ci && npm run lint && npm run check && npm run test && npm run build`

## Notes

- Stories 1.5–1.7 deferred Svelte component tests due to missing Vitest setup; this story introduces the harness and establishes a baseline pattern for future client tests.
- Going forward, client-facing stories should not mark "client tests deferred" unless the deferred tests are explicitly tracked as a dedicated follow-up story in `sprint-status.yaml`.

## Dev Agent Record

### Agent Model Used

Copilot CLI 0.0.414

### Debug Log References

- `cd client && npm ci --no-audit --no-fund && npm run lint && npm run check && npm run test && npm run build`

### Completion Notes List

- Added Vitest + jsdom + Testing Library test harness with CI-friendly `npm run test` (`vitest run`).
- Configured Vitest for Svelte 5 component mounting (browser export conditions + inlined `svelte`), plus `$lib` alias + jest-dom matchers.
- Added deterministic unit tests for `crypto.ts`, `navigationState.ts`, and `identityStore.svelte.ts`.
- Added smoke component tests for `RecoveryPrompt.svelte` and `ReRegisterPrompt.svelte`.
- Enforced client tests in GitHub Actions CI.
- Aligned CI to use `npm run lint` / `npm run check` (matches local scripts; includes node TS typecheck).
- Removed unnecessary Vitest global injection (`globals`) since tests import Vitest APIs explicitly.
- Enabled npm dependency caching in the CI client job (faster installs).
- Replaced `setTimeout(0)`-style async waiting in `identityStore` tests with `waitFor()` to reduce flake risk.
- Verified client quality gates pass.

### File List

- `.github/workflows/ci.yml`
- `.github/workflows/release.yml`
- `_bmad-output/implementation-artifacts/2-8-vitest-setup-and-client-test-baseline.md`
- `_bmad-output/implementation-artifacts/sprint-status.yaml`
- `client/package.json`
- `client/package-lock.json`
- `client/tsconfig.node.json`
- `client/vitest.config.ts`
- `client/src/test/setup.ts`
- `client/src/lib/features/identity/crypto.test.ts`
- `client/src/lib/features/identity/navigationState.test.ts`
- `client/src/lib/features/identity/identityStore.test.ts`
- `client/src/lib/features/identity/RecoveryPrompt.test.ts`
- `client/src/lib/features/identity/ReRegisterPrompt.test.ts`

## Change Log

- 2026-02-24: Started implementation (Vitest setup, baseline tests, CI enforcement).
- 2026-02-24: Added Vitest harness + baseline identity tests; updated CI to run `npm run test`; verified `npm ci/lint/check/test/build`.
- 2026-02-24: CI now uses `npm run lint` + `npm run check`; removed Vitest globals injection.
- 2026-02-24: Added npm caching to the CI client job.
- 2026-02-24: Hardened identityStore async test waiting (use `waitFor()`).
- 2026-02-24: Senior Developer Review (AI) - documented release workflow manual trigger (build quota); corrected story File List; marked story done.
- 2026-02-24: Senior Developer Review (AI) - ensured tsconfig checks vitest config; re-verified client quality gates; **FFR TO CHECK CURRENT VERSIONS** for GitHub Actions action versions.

## Senior Developer Review (AI)

Reviewer: Darko  
Date: 2026-02-24

### Findings

- **MEDIUM:** `.github/workflows/release.yml` trigger change should be documented clearly (it is intentionally manual-only to conserve build quota).
- **MEDIUM:** `npm run check` did not typecheck `client/vitest.config.ts` (tsconfig.node.json only included vite.config.ts).
- **LOW (FFR TO CHECK CURRENT VERSIONS):** Periodically verify GitHub Actions action major versions are still current (`actions/checkout@v6`, `actions/setup-node@v6`, `actions/cache@v5`) and bump when appropriate.
- **LOW:** `npm ci` emits a few deprecation warnings (for example `glob@10.x`, `whatwg-encoding@3.x`); not blocking, but worth tracking.

### Fixes Applied

- Kept `.github/workflows/release.yml` manual-only and added an explicit note that it's to conserve build quota.
- Reverted the GitHub Actions major-version change (keep `actions/checkout@v6`, `actions/setup-node@v6`, `actions/cache@v5`) and recorded **FFR TO CHECK CURRENT VERSIONS**.
- Corrected this story's File List to match actual git changes (added `client/tsconfig.node.json`).
- Ensured `npm run check` typechecks both `vite.config.ts` and `vitest.config.ts`.
- Verified client quality gates locally: `npm ci --no-audit --no-fund && npm run lint && npm run check && npm run test && npm run build`.
