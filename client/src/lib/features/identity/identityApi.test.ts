import { beforeEach, describe, expect, it, vi } from 'vitest'

vi.mock('$lib/api', () => ({
  apiFetch: vi.fn(),
  ApiError: class ApiError extends Error {
    code: string

    constructor(code: string, message: string) {
      super(message)
      this.name = 'ApiError'
      this.code = code
    }
  },
}))

import { apiFetch } from '$lib/api'
import {
  addUserBlock,
  deleteMyAccount,
  getProfile,
  getRecoveryEmailStatus,
  listUserBlocks,
  logout,
  recoverIdentityByToken,
  register,
  removeUserBlock,
  requestChallenge,
  requestPersonalDataExport,
  startIdentityRecovery,
  startRecoveryEmailAssociation,
  updateProfile,
  uploadAvatar,
  verifyChallenge,
} from './identityApi'

describe('identityApi', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('register sends wire format and maps response', async () => {
    vi.mocked(apiFetch).mockResolvedValue({
      id: 'user-1',
      did_key: 'did:key:z6Mk-test',
      username: 'alice',
      display_name: 'alice',
      avatar_color: '#3b82f6',
      created_at: '2026-02-24T00:00:00.000Z',
    })

    const user = await register('did:key:z6Mk-test', 'alice', '#3b82f6')

    expect(apiFetch).toHaveBeenCalledWith('/api/v1/auth/register', {
      method: 'POST',
      body: JSON.stringify({
        did_key: 'did:key:z6Mk-test',
        username: 'alice',
        avatar_color: '#3b82f6',
      }),
    })

    expect(user).toEqual({
      id: 'user-1',
      didKey: 'did:key:z6Mk-test',
      username: 'alice',
      displayName: 'alice',
      avatarColor: '#3b82f6',
      avatarUrl: null,
      createdAt: '2026-02-24T00:00:00.000Z',
    })
  })

  it('requestChallenge maps expires_in to expiresIn', async () => {
    vi.mocked(apiFetch).mockResolvedValue({ challenge: 'c1', expires_in: 300 })

    await expect(requestChallenge('did:key:z6Mk-test')).resolves.toEqual({
      challenge: 'c1',
      expiresIn: 300,
    })

    expect(apiFetch).toHaveBeenCalledWith('/api/v1/auth/challenge', {
      method: 'POST',
      body: JSON.stringify({ did_key: 'did:key:z6Mk-test' }),
    })
  })

  it('requestChallenge sends optional cross_instance payload in snake_case', async () => {
    vi.mocked(apiFetch).mockResolvedValue({ challenge: 'c2', expires_in: 300 })

    await requestChallenge('did:key:z6Mk-test', {
      username: 'alice',
      displayName: 'Alice',
      avatarColor: '#3b82f6',
    })

    expect(apiFetch).toHaveBeenCalledWith('/api/v1/auth/challenge', {
      method: 'POST',
      body: JSON.stringify({
        did_key: 'did:key:z6Mk-test',
        cross_instance: {
          enabled: true,
          username: 'alice',
          display_name: 'Alice',
          avatar_color: '#3b82f6',
        },
      }),
    })
  })

  it('verifyChallenge maps session response', async () => {
    vi.mocked(apiFetch).mockResolvedValue({
      token: 'token-1',
      expires_at: '2026-03-01T00:00:00.000Z',
      user: {
        id: 'user-1',
        did_key: 'did:key:z6Mk-test',
        username: 'alice',
        display_name: 'Alice',
        created_at: '2026-02-24T00:00:00.000Z',
      },
    })

    const session = await verifyChallenge(
      'did:key:z6Mk-test',
      'challenge',
      'signature',
    )

    expect(apiFetch).toHaveBeenCalledWith('/api/v1/auth/verify', {
      method: 'POST',
      body: JSON.stringify({
        did_key: 'did:key:z6Mk-test',
        challenge: 'challenge',
        signature: 'signature',
      }),
    })

    expect(session).toEqual({
      token: 'token-1',
      expiresAt: '2026-03-01T00:00:00.000Z',
      user: {
        id: 'user-1',
        didKey: 'did:key:z6Mk-test',
        username: 'alice',
        displayName: 'Alice',
        avatarColor: null,
        avatarUrl: null,
        createdAt: '2026-02-24T00:00:00.000Z',
      },
    })
  })

  it('verifyChallenge sends cross_instance when enabled', async () => {
    vi.mocked(apiFetch).mockResolvedValue({
      token: 'token-1',
      expires_at: '2026-03-01T00:00:00.000Z',
      user: {
        id: 'user-1',
        did_key: 'did:key:z6Mk-test',
        username: 'alice',
        display_name: 'Alice',
        created_at: '2026-02-24T00:00:00.000Z',
      },
    })

    await verifyChallenge('did:key:z6Mk-test', 'challenge', 'signature', true)

    expect(apiFetch).toHaveBeenCalledWith('/api/v1/auth/verify', {
      method: 'POST',
      body: JSON.stringify({
        did_key: 'did:key:z6Mk-test',
        challenge: 'challenge',
        signature: 'signature',
        cross_instance: { enabled: true },
      }),
    })
  })

  it('logout uses Authorization header', async () => {
    vi.mocked(apiFetch).mockResolvedValue(undefined)

    await logout('token-2')

    expect(apiFetch).toHaveBeenCalledWith('/api/v1/auth/logout', {
      method: 'DELETE',
      headers: { authorization: 'Bearer token-2' },
    })
  })

  it('deleteMyAccount sends confirm_username payload', async () => {
    vi.mocked(apiFetch).mockResolvedValue(undefined)

    await deleteMyAccount({ confirmUsername: 'alice' })

    expect(apiFetch).toHaveBeenCalledWith('/api/v1/users/me', {
      method: 'DELETE',
      body: JSON.stringify({
        confirm_username: 'alice',
      }),
    })
  })

  it('deleteMyAccount rejects blank confirm_username before request', () => {
    expect(() => deleteMyAccount({ confirmUsername: '   ' })).toThrow(
      'confirmUsername is required',
    )
    expect(apiFetch).not.toHaveBeenCalled()
  })

  it('getProfile maps profile payload', async () => {
    vi.mocked(apiFetch).mockResolvedValue({
      id: 'user-1',
      did_key: 'did:key:z6Mk-test',
      username: 'alice',
      display_name: 'Alice',
      avatar_color: '#3b82f6',
      avatar_url: '/api/v1/users/me/avatar',
      created_at: '2026-02-24T00:00:00.000Z',
    })

    await expect(getProfile()).resolves.toEqual({
      id: 'user-1',
      didKey: 'did:key:z6Mk-test',
      username: 'alice',
      displayName: 'Alice',
      avatarColor: '#3b82f6',
      avatarUrl: '/api/v1/users/me/avatar',
      createdAt: '2026-02-24T00:00:00.000Z',
    })
    expect(apiFetch).toHaveBeenCalledWith('/api/v1/users/me/profile')
  })

  it('updateProfile sends patch wire payload', async () => {
    vi.mocked(apiFetch).mockResolvedValue({
      id: 'user-1',
      did_key: 'did:key:z6Mk-test',
      username: 'alice',
      display_name: 'Alice',
      avatar_color: '#3b82f6',
      created_at: '2026-02-24T00:00:00.000Z',
    })

    await updateProfile({ displayName: 'Alice', avatarColor: '#3b82f6' })

    expect(apiFetch).toHaveBeenCalledWith('/api/v1/users/me/profile', {
      method: 'PATCH',
      body: JSON.stringify({
        display_name: 'Alice',
        avatar_color: '#3b82f6',
      }),
    })
  })

  it('uploadAvatar sends multipart form data', async () => {
    vi.mocked(apiFetch).mockResolvedValue({
      id: 'user-1',
      did_key: 'did:key:z6Mk-test',
      username: 'alice',
      display_name: 'Alice',
      avatar_color: '#3b82f6',
      avatar_url: '/api/v1/users/me/avatar',
      created_at: '2026-02-24T00:00:00.000Z',
    })
    const file = new File(['hello'], 'avatar.png', { type: 'image/png' })

    await uploadAvatar(file)

    const lastCall = vi.mocked(apiFetch).mock.calls.at(-1)
    expect(lastCall).toBeDefined()
    expect(lastCall?.[0]).toBe('/api/v1/users/me/avatar')
    expect(lastCall?.[1]?.method).toBe('POST')
    expect(lastCall?.[1]?.body).toBeInstanceOf(FormData)
  })

  it('getRecoveryEmailStatus maps recovery status payload', async () => {
    vi.mocked(apiFetch).mockResolvedValue({
      associated: true,
      email_masked: 'l***@example.com',
      verified: false,
    })

    await expect(getRecoveryEmailStatus()).resolves.toEqual({
      associated: true,
      emailMasked: 'l***@example.com',
      verified: false,
      verifiedAt: null,
    })
    expect(apiFetch).toHaveBeenCalledWith('/api/v1/users/me/recovery-email')
  })

  it('startRecoveryEmailAssociation sends snake_case payload', async () => {
    vi.mocked(apiFetch).mockResolvedValue({
      associated: true,
      email_masked: 'l***@example.com',
      verified: false,
    })

    await startRecoveryEmailAssociation({
      email: 'liam@example.com',
      encryptedPrivateKey: 'c2VjcmV0',
      encryptionContext: {
        algorithm: 'aes-256-gcm',
        version: 1,
      },
    })

    expect(apiFetch).toHaveBeenCalledWith('/api/v1/users/me/recovery-email', {
      method: 'POST',
      body: JSON.stringify({
        email: 'liam@example.com',
        encrypted_private_key: 'c2VjcmV0',
        encryption_context: {
          algorithm: 'aes-256-gcm',
          version: 1,
        },
      }),
    })
  })

  it('requestPersonalDataExport posts endpoint and maps response', async () => {
    vi.mocked(apiFetch).mockResolvedValue({
      profile: {
        user_id: 'user-1',
        did_key: 'did:key:z6Mk-test',
        username: 'alice',
        display_name: 'Alice',
        avatar_color: '#3b82f6',
        created_at: '2026-02-24T00:00:00.000Z',
        updated_at: '2026-02-24T00:00:00.000Z',
      },
      guild_memberships: [
        { guild_id: 'guild-1', joined_at: '2026-02-25T00:00:00.000Z' },
      ],
      messages: [],
      dm_messages: [],
      reactions: [],
      uploaded_files: [],
      block_list: [],
      exported_at: '2026-03-02T00:00:00.000Z',
    })

    const data = await requestPersonalDataExport()

    expect(apiFetch).toHaveBeenCalledWith('/api/v1/users/me/data-export', {
      method: 'POST',
    })
    expect(data.profile.userId).toBe('user-1')
    expect(data.guildMemberships[0]?.guildId).toBe('guild-1')
    expect(data.exportedAt).toBe('2026-03-02T00:00:00.000Z')
  })

  it('startIdentityRecovery posts email and maps response', async () => {
    vi.mocked(apiFetch).mockResolvedValue({
      message: 'Recovery email sent. Check your inbox for a recovery link.',
      help_message: "Didn't receive the email? Check spam, or try again.",
    })

    await expect(startIdentityRecovery('liam@example.com')).resolves.toEqual({
      message: 'Recovery email sent. Check your inbox for a recovery link.',
      helpMessage: "Didn't receive the email? Check spam, or try again.",
    })

    expect(apiFetch).toHaveBeenCalledWith('/api/v1/auth/recovery-email/start', {
      method: 'POST',
      body: JSON.stringify({ email: 'liam@example.com' }),
    })
  })

  it('recoverIdentityByToken maps recovery payload', async () => {
    vi.mocked(apiFetch).mockResolvedValue({
      did_key: 'did:key:z6Mk-test',
      username: 'liam',
      avatar_color: '#3b82f6',
      registered_at: '2026-02-24T00:00:00.000Z',
      encrypted_private_key: 'c2VjcmV0',
      encryption_context: {
        algorithm: 'aes-256-gcm',
        version: 1,
      },
    })

    await expect(recoverIdentityByToken('abc-token')).resolves.toEqual({
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

    expect(apiFetch).toHaveBeenCalledWith(
      '/api/v1/auth/recovery-email/recover?token=abc-token',
    )
  })

  it('listUserBlocks maps blocked-user entries', async () => {
    vi.mocked(apiFetch).mockResolvedValue([
      {
        blocked_user_id: 'user-2',
        blocked_at: '2026-03-01T00:00:00.000Z',
        unblocked_at: null,
        blocked_user_display_name: 'Bob',
        blocked_user_username: 'bob',
        blocked_user_avatar_color: '#22aa88',
      },
    ])

    await expect(listUserBlocks()).resolves.toEqual([
      {
        blockedUserId: 'user-2',
        blockedAt: '2026-03-01T00:00:00.000Z',
        unblockedAt: null,
        blockedUserDisplayName: 'Bob',
        blockedUserUsername: 'bob',
        blockedUserAvatarColor: '#22aa88',
      },
    ])
    expect(apiFetch).toHaveBeenCalledWith('/api/v1/users/me/blocks')
  })

  it('addUserBlock posts blocked_user_id and maps response', async () => {
    vi.mocked(apiFetch).mockResolvedValue({
      blocked_user_id: 'user-2',
      blocked_at: '2026-03-01T00:00:00.000Z',
      unblocked_at: null,
    })

    await expect(addUserBlock('user-2')).resolves.toEqual({
      blockedUserId: 'user-2',
      blockedAt: '2026-03-01T00:00:00.000Z',
      unblockedAt: null,
      blockedUserDisplayName: null,
      blockedUserUsername: null,
      blockedUserAvatarColor: null,
    })

    expect(apiFetch).toHaveBeenCalledWith('/api/v1/users/me/blocks', {
      method: 'POST',
      body: JSON.stringify({
        blocked_user_id: 'user-2',
      }),
    })
  })

  it('removeUserBlock sends delete request', async () => {
    vi.mocked(apiFetch).mockResolvedValue(undefined)

    await expect(removeUserBlock('user-2')).resolves.toBeUndefined()

    expect(apiFetch).toHaveBeenCalledWith('/api/v1/users/me/blocks/user-2', {
      method: 'DELETE',
    })
  })
})
