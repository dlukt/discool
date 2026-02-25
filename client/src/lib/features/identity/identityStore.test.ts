import { waitFor } from '@testing-library/svelte'
import { beforeEach, describe, expect, it, vi } from 'vitest'

vi.mock('./crypto', () => ({
  clearStoredIdentity: vi.fn(),
  decryptSecretKey: vi.fn(),
  finalizeIdentityRegistration: vi.fn(),
  loadStoredIdentity: vi.fn(),
  restoreIdentityFromRecovery: vi.fn(),
  signChallenge: vi.fn(),
}))

vi.mock('./identityApi', () => ({
  getRecoveryEmailStatus: vi.fn(),
  getProfile: vi.fn(),
  logout: vi.fn(),
  recoverIdentityByToken: vi.fn(),
  register: vi.fn(),
  requestChallenge: vi.fn(),
  startIdentityRecovery: vi.fn(),
  startRecoveryEmailAssociation: vi.fn(),
  updateProfile: vi.fn(),
  uploadAvatar: vi.fn(),
  verifyChallenge: vi.fn(),
}))

import {
  decryptSecretKey,
  loadStoredIdentity,
  restoreIdentityFromRecovery,
  signChallenge,
} from './crypto'
import {
  getRecoveryEmailStatus,
  logout,
  recoverIdentityByToken,
  requestChallenge,
  startIdentityRecovery,
  startRecoveryEmailAssociation,
  updateProfile,
  uploadAvatar,
  verifyChallenge,
} from './identityApi'
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
  identityState.crossInstanceJoining = false
  identityState.crossInstanceJoinError = null
  identityState.recoveryEmailStatus = null
  identityState.recoveryEmailLoading = false
  identityState.recoveryEmailError = null
  identityState.recoveryNudgeDismissed = false
}

describe('identityStore session persistence', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    localStorage.clear()
    resetIdentityState()
    vi.mocked(getRecoveryEmailStatus).mockResolvedValue({
      associated: false,
      emailMasked: null,
      verified: false,
      verifiedAt: null,
    })
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
        displayName: 'alice',
        avatarColor: null,
        avatarUrl: null,
        createdAt: '2026-02-24T00:00:00.000Z',
      },
    })

    await identityState.authenticate()

    expect(identityState.session?.token).toBe('token-1')
    const raw = localStorage.getItem('discool-session')
    expect(raw).not.toBeNull()
    expect(JSON.parse(raw ?? '')).toMatchObject({ token: 'token-1' })
  })

  it('authenticateCrossInstance sends cross payload and persists session', async () => {
    identityState.identity = {
      publicKey: new Uint8Array(32).fill(1),
      didKey: 'did:key:z6Mk-test',
      username: 'alice',
      avatarColor: '#3b82f6',
      registeredAt: '2026-02-24T00:00:00.000Z',
    }
    identityState.identityNotRegistered = true

    vi.mocked(requestChallenge).mockResolvedValue({
      challenge: 'challenge',
      expiresIn: 60,
    })
    vi.mocked(signChallenge).mockResolvedValue('signature')
    vi.mocked(verifyChallenge).mockResolvedValue({
      token: 'token-cross',
      expiresAt: new Date(Date.now() + 60_000).toISOString(),
      user: {
        id: 'user-1',
        didKey: 'did:key:z6Mk-test',
        username: 'alice',
        displayName: 'Alice',
        avatarColor: '#3b82f6',
        avatarUrl: null,
        createdAt: '2026-02-24T00:00:00.000Z',
      },
    })

    await identityState.authenticateCrossInstance()

    expect(requestChallenge).toHaveBeenCalledWith('did:key:z6Mk-test', {
      username: 'alice',
      displayName: 'alice',
      avatarColor: '#3b82f6',
    })
    expect(verifyChallenge).toHaveBeenCalledWith(
      'did:key:z6Mk-test',
      'challenge',
      'signature',
      true,
    )
    expect(identityState.session?.token).toBe('token-cross')
    expect(identityState.identityNotRegistered).toBe(false)
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
          displayName: 'alice',
          avatarColor: null,
          avatarUrl: null,
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
          displayName: 'alice',
          avatarColor: null,
          avatarUrl: null,
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
        displayName: 'alice',
        avatarColor: null,
        avatarUrl: null,
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
        displayName: 'alice',
        avatarColor: null,
        avatarUrl: null,
        createdAt: '2026-02-24T00:00:00.000Z',
      },
    }

    window.dispatchEvent(
      new StorageEvent('storage', { key: 'discool-session', newValue: null }),
    )

    expect(identityState.session).toBeNull()
    expect(identityState.authError).toBe('Signed out in another tab.')
  })

  it('StorageEvent login restores session from localStorage', async () => {
    vi.mocked(loadStoredIdentity).mockResolvedValue({ status: 'none' })
    await identityState.initialize()

    localStorage.setItem(
      'discool-session',
      JSON.stringify({
        token: 'token-6',
        expiresAt: new Date(Date.now() + 60_000).toISOString(),
        user: {
          id: 'user-6',
          didKey: 'did:key:z6Mk-test',
          username: 'alice',
          displayName: 'alice',
          avatarColor: null,
          avatarUrl: null,
          createdAt: '2026-02-24T00:00:00.000Z',
        },
      }),
    )

    window.dispatchEvent(
      new StorageEvent('storage', {
        key: 'discool-session',
        newValue: localStorage.getItem('discool-session'),
      }),
    )

    await waitFor(() => expect(identityState.session?.token).toBe('token-6'))
    expect(requestChallenge).not.toHaveBeenCalled()
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
          displayName: 'alice',
          avatarColor: null,
          avatarUrl: null,
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

  it('keeps successful profile updates in session when avatar upload fails', async () => {
    const expiresAt = new Date(Date.now() + 60_000).toISOString()
    identityState.session = {
      token: 'token-7',
      expiresAt,
      user: {
        id: 'user-7',
        didKey: 'did:key:z6Mk-test',
        username: 'alice',
        displayName: 'Alice',
        avatarColor: '#3b82f6',
        avatarUrl: null,
        createdAt: '2026-02-24T00:00:00.000Z',
      },
    }

    vi.mocked(updateProfile).mockResolvedValue({
      id: 'user-7',
      didKey: 'did:key:z6Mk-test',
      username: 'alice',
      displayName: 'Alice Cooper',
      avatarColor: '#3b82f6',
      avatarUrl: null,
      createdAt: '2026-02-24T00:00:00.000Z',
    })
    vi.mocked(uploadAvatar).mockRejectedValue(new Error('upload failed'))

    const file = new File(['avatar'], 'avatar.png', { type: 'image/png' })
    await expect(
      identityState.saveProfile({ displayName: 'Alice Cooper' }, file),
    ).rejects.toThrow('upload failed')

    expect(identityState.session?.user.displayName).toBe('Alice Cooper')
    expect(
      JSON.parse(localStorage.getItem('discool-session') ?? '{}'),
    ).toMatchObject({
      token: 'token-7',
      expiresAt,
      user: { displayName: 'Alice Cooper' },
    })
  })

  it('startRecoveryEmailAssociation decrypts key material and stores status', async () => {
    identityState.session = {
      token: 'token-8',
      expiresAt: new Date(Date.now() + 60_000).toISOString(),
      user: {
        id: 'user-8',
        didKey: 'did:key:z6Mk-test',
        username: 'alice',
        displayName: 'Alice',
        avatarColor: '#3b82f6',
        avatarUrl: null,
        createdAt: '2026-02-24T00:00:00.000Z',
      },
    }
    vi.mocked(decryptSecretKey).mockResolvedValue(new Uint8Array([1, 2, 3, 4]))
    vi.mocked(startRecoveryEmailAssociation).mockResolvedValue({
      associated: true,
      emailMasked: 'a***@example.com',
      verified: false,
      verifiedAt: null,
    })

    await identityState.startRecoveryEmailAssociation('alice@example.com')

    expect(startRecoveryEmailAssociation).toHaveBeenCalledWith({
      email: 'alice@example.com',
      encryptedPrivateKey: 'AQIDBA==',
      encryptionContext: {
        algorithm: 'aes-256-gcm',
        version: 1,
      },
    })
    expect(identityState.recoveryEmailStatus).toEqual({
      associated: true,
      emailMasked: 'a***@example.com',
      verified: false,
      verifiedAt: null,
    })
  })

  it('startRecoveryEmailAssociation clears loading state when decrypt fails', async () => {
    identityState.session = {
      token: 'token-9',
      expiresAt: new Date(Date.now() + 60_000).toISOString(),
      user: {
        id: 'user-9',
        didKey: 'did:key:z6Mk-test',
        username: 'alice',
        displayName: 'Alice',
        avatarColor: '#3b82f6',
        avatarUrl: null,
        createdAt: '2026-02-24T00:00:00.000Z',
      },
    }
    vi.mocked(decryptSecretKey).mockRejectedValue(new Error('decrypt failed'))

    await expect(
      identityState.startRecoveryEmailAssociation('alice@example.com'),
    ).rejects.toThrow('decrypt failed')

    expect(identityState.recoveryEmailLoading).toBe(false)
    expect(identityState.recoveryEmailError).toBe('decrypt failed')
    expect(startRecoveryEmailAssociation).not.toHaveBeenCalled()
  })

  it('startIdentityRecovery validates email and forwards request', async () => {
    vi.mocked(startIdentityRecovery).mockResolvedValue({
      message: 'Recovery email sent. Check your inbox for a recovery link.',
      helpMessage: "Didn't receive the email? Check spam, or try again.",
    })

    await expect(
      identityState.startIdentityRecovery('  liam@example.com  '),
    ).resolves.toEqual({
      message: 'Recovery email sent. Check your inbox for a recovery link.',
      helpMessage: "Didn't receive the email? Check spam, or try again.",
    })

    expect(startIdentityRecovery).toHaveBeenCalledWith('liam@example.com')
  })

  it('recoverIdentityByToken restores identity then authenticates', async () => {
    const restoredIdentity = {
      publicKey: new Uint8Array(32).fill(1),
      didKey: 'did:key:z6Mk-test',
      username: 'liam',
      avatarColor: '#3b82f6',
      registeredAt: '2026-02-24T00:00:00.000Z',
    }

    vi.mocked(recoverIdentityByToken).mockResolvedValue({
      didKey: 'did:key:z6Mk-test',
      username: 'liam',
      avatarColor: '#3b82f6',
      registeredAt: '2026-02-24T00:00:00.000Z',
      encryptedPrivateKey: 'c2VjcmV0',
      encryptionContext: {
        algorithm: 'aes-256-gcm',
        version: 1,
      },
    })
    vi.mocked(restoreIdentityFromRecovery).mockResolvedValue(restoredIdentity)
    vi.mocked(requestChallenge).mockResolvedValue({
      challenge: 'challenge',
      expiresIn: 60,
    })
    vi.mocked(signChallenge).mockResolvedValue('signature')
    vi.mocked(verifyChallenge).mockResolvedValue({
      token: 'token-recovered',
      expiresAt: new Date(Date.now() + 60_000).toISOString(),
      user: {
        id: 'user-recovered',
        didKey: 'did:key:z6Mk-test',
        username: 'liam',
        displayName: 'Liam',
        avatarColor: '#3b82f6',
        avatarUrl: null,
        createdAt: '2026-02-24T00:00:00.000Z',
      },
    })

    await identityState.recoverIdentityByToken('token-123')

    expect(recoverIdentityByToken).toHaveBeenCalledWith('token-123')
    expect(restoreIdentityFromRecovery).toHaveBeenCalled()
    expect(identityState.identity).toEqual(restoredIdentity)
    expect(identityState.session?.token).toBe('token-recovered')
  })
})
