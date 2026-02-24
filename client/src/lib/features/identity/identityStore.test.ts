import { waitFor } from '@testing-library/svelte'
import { beforeEach, describe, expect, it, vi } from 'vitest'

vi.mock('./crypto', () => ({
  clearStoredIdentity: vi.fn(),
  finalizeIdentityRegistration: vi.fn(),
  loadStoredIdentity: vi.fn(),
  signChallenge: vi.fn(),
}))

vi.mock('./identityApi', () => ({
  logout: vi.fn(),
  register: vi.fn(),
  requestChallenge: vi.fn(),
  verifyChallenge: vi.fn(),
}))

import { loadStoredIdentity, signChallenge } from './crypto'
import { logout, requestChallenge, verifyChallenge } from './identityApi'
import { identityState } from './identityStore.svelte'

function resetIdentityState() {
  identityState.identity = null
  identityState.identityCorrupted = false
  identityState.identityNotRegistered = false
  identityState.loading = false
  identityState.error = null
  identityState.session = null
  identityState.authenticating = false
  identityState.authError = null
}

describe('identityStore session persistence', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    localStorage.clear()
    resetIdentityState()
  })

  it('persists session to localStorage after authenticate()', async () => {
    identityState.identity = {
      publicKey: new Uint8Array(32).fill(1),
      didKey: 'did:key:z6Mk-test',
      username: 'alice',
      avatarColor: null,
      registeredAt: '2026-02-24T00:00:00.000Z',
    }

    vi.mocked(requestChallenge).mockResolvedValue({
      challenge: 'challenge',
      expiresIn: 60,
    })
    vi.mocked(signChallenge).mockResolvedValue('signature')
    vi.mocked(verifyChallenge).mockResolvedValue({
      token: 'token-1',
      expiresAt: new Date(Date.now() + 60_000).toISOString(),
      user: {
        id: 'user-1',
        didKey: 'did:key:z6Mk-test',
        username: 'alice',
        avatarColor: null,
        createdAt: '2026-02-24T00:00:00.000Z',
      },
    })

    await identityState.authenticate()

    expect(identityState.session?.token).toBe('token-1')
    const raw = localStorage.getItem('discool-session')
    expect(raw).not.toBeNull()
    expect(JSON.parse(raw ?? '')).toMatchObject({ token: 'token-1' })
  })

  it('restores session from localStorage on initialize()', async () => {
    localStorage.setItem(
      'discool-session',
      JSON.stringify({
        token: 'token-2',
        expiresAt: new Date(Date.now() + 60_000).toISOString(),
        user: {
          id: 'user-2',
          didKey: 'did:key:z6Mk-test',
          username: 'alice',
          avatarColor: null,
          createdAt: '2026-02-24T00:00:00.000Z',
        },
      }),
    )

    vi.mocked(loadStoredIdentity).mockResolvedValue({
      status: 'found',
      identity: {
        publicKey: new Uint8Array(32).fill(1),
        didKey: 'did:key:z6Mk-test',
        username: 'alice',
        avatarColor: null,
        registeredAt: '2026-02-24T00:00:00.000Z',
      },
    })

    await identityState.initialize()

    expect(identityState.session?.token).toBe('token-2')
    expect(requestChallenge).not.toHaveBeenCalled()
  })

  it('expired session triggers re-authentication on initialize()', async () => {
    localStorage.setItem(
      'discool-session',
      JSON.stringify({
        token: 'expired',
        expiresAt: new Date(Date.now() - 60_000).toISOString(),
        user: {
          id: 'user-3',
          didKey: 'did:key:z6Mk-test',
          username: 'alice',
          avatarColor: null,
          createdAt: '2026-02-24T00:00:00.000Z',
        },
      }),
    )

    vi.mocked(loadStoredIdentity).mockResolvedValue({
      status: 'found',
      identity: {
        publicKey: new Uint8Array(32).fill(1),
        didKey: 'did:key:z6Mk-test',
        username: 'alice',
        avatarColor: null,
        registeredAt: '2026-02-24T00:00:00.000Z',
      },
    })

    vi.mocked(requestChallenge).mockResolvedValue({
      challenge: 'challenge',
      expiresIn: 60,
    })
    vi.mocked(signChallenge).mockResolvedValue('signature')
    vi.mocked(verifyChallenge).mockResolvedValue({
      token: 'token-3',
      expiresAt: new Date(Date.now() + 60_000).toISOString(),
      user: {
        id: 'user-3',
        didKey: 'did:key:z6Mk-test',
        username: 'alice',
        avatarColor: null,
        createdAt: '2026-02-24T00:00:00.000Z',
      },
    })

    await identityState.initialize()
    await waitFor(() => expect(identityState.session?.token).toBe('token-3'))

    expect(requestChallenge).toHaveBeenCalled()
  })

  it('StorageEvent logout clears the in-memory session', async () => {
    vi.mocked(loadStoredIdentity).mockResolvedValue({ status: 'none' })
    await identityState.initialize()

    identityState.session = {
      token: 'token-4',
      expiresAt: new Date(Date.now() + 60_000).toISOString(),
      user: {
        id: 'user-4',
        didKey: 'did:key:z6Mk-test',
        username: 'alice',
        avatarColor: null,
        createdAt: '2026-02-24T00:00:00.000Z',
      },
    }

    window.dispatchEvent(
      new StorageEvent('storage', { key: 'discool-session', newValue: null }),
    )

    expect(identityState.session).toBeNull()
    expect(identityState.authError).toBe('Signed out in another tab.')
  })

  it('logout clears localStorage and calls server logout', async () => {
    localStorage.setItem(
      'discool-session',
      JSON.stringify({
        token: 'token-5',
        expiresAt: new Date(Date.now() + 60_000).toISOString(),
        user: {
          id: 'user-5',
          didKey: 'did:key:z6Mk-test',
          username: 'alice',
          avatarColor: null,
          createdAt: '2026-02-24T00:00:00.000Z',
        },
      }),
    )
    identityState.session = JSON.parse(
      localStorage.getItem('discool-session') ?? 'null',
    )

    vi.mocked(logout).mockResolvedValue(undefined)

    await identityState.logout()

    expect(localStorage.getItem('discool-session')).toBeNull()
    expect(logout).toHaveBeenCalledWith('token-5')
  })
})
