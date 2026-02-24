# Story 2.8: Vitest Setup and Client Test Baseline

Status: ready-for-dev

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

- [ ] Task 1: Add Vitest + testing dependencies and npm scripts
  - [ ] 1.1 Add devDependencies in `client/package.json`:
    - `vitest`
    - `jsdom`
    - `@testing-library/svelte`
    - `@testing-library/jest-dom`
    - `@vitest/coverage-v8` (optional but recommended)
  - [ ] 1.2 Add scripts:
    - `test` (CI-friendly, non-watch)
    - `test:watch` (local dev)
    - `test:ui` (optional)

- [ ] Task 2: Configure Vitest for Svelte + TypeScript + $lib alias
  - [ ] 2.1 Create `client/vitest.config.ts`:
    - Uses the Svelte Vite plugin
    - `test.environment = 'jsdom'`
    - `test.setupFiles` points at a test setup file that enables jest-dom matchers
    - Ensures `$lib` alias works the same as in `vite.config.ts`
  - [ ] 2.2 Create `client/src/test/setup.ts` that imports `@testing-library/jest-dom/vitest`
  - [ ] 2.3 Ensure `vitest` types are available to TypeScript (avoid `any` / implicit globals)

- [ ] Task 3: Pay down client test debt for Epic 2 (Stories 2.1–2.3)
  - [ ] 3.1 Unit tests for `client/src/lib/features/identity/crypto.ts`:
    - `didKeyFromPublicKey()` produces `did:key:z6Mk...` format
    - base58 helpers match known vectors
    - `loadStoredIdentity()` returns `'none' | 'found' | 'corrupted'` correctly (Story 2.3 behavior)
  - [ ] 3.2 Unit tests for `client/src/lib/features/identity/navigationState.ts`:
    - save/get/clear round-trip on localStorage
  - [ ] 3.3 Unit tests for session persistence in `identityStore.svelte.ts` (Story 2.3 behavior):
    - session persists to localStorage
    - session restores on initialize
    - expired session triggers re-authentication

- [ ] Task 4: Smoke-level component tests for identity recovery UI (Story 2.3)
  - [ ] 4.1 `RecoveryPrompt.svelte`:
    - renders expected copy
    - "Start fresh" triggers the clear/start-fresh action
    - "Recover via email" is disabled (placeholder)
  - [ ] 4.2 `ReRegisterPrompt.svelte`:
    - renders stored username
    - "Register" triggers re-register flow (mock identity API)

- [ ] Task 5: Enforce tests in CI
  - [ ] 5.1 Update `.github/workflows/ci.yml` client job to run `npm run test`

- [ ] Task 6: Verify quality gates
  - [ ] 6.1 `cd client && npm ci && npm run lint && npm run check && npm run test && npm run build`

## Notes

- Stories 1.5–1.7 deferred Svelte component tests due to missing Vitest setup; this story introduces the harness and establishes a baseline pattern for future client tests.
- Going forward, client-facing stories should not mark "client tests deferred" unless the deferred tests are explicitly tracked as a dedicated follow-up story in `sprint-status.yaml`.

