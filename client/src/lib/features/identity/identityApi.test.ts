import { describe, expect, it, vi } from 'vitest'

vi.mock('$lib/api', () => ({
  apiFetch: vi.fn(),
}))

import { apiFetch } from '$lib/api'
import {
  logout,
  register,
  requestChallenge,
  verifyChallenge,
} from './identityApi'

describe('identityApi', () => {
  it('register sends wire format and maps response', async () => {
    vi.mocked(apiFetch).mockResolvedValue({
      id: 'user-1',
      did_key: 'did:key:z6Mk-test',
      username: 'alice',
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
      avatarColor: '#3b82f6',
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

  it('verifyChallenge maps session response', async () => {
    vi.mocked(apiFetch).mockResolvedValue({
      token: 'token-1',
      expires_at: '2026-03-01T00:00:00.000Z',
      user: {
        id: 'user-1',
        did_key: 'did:key:z6Mk-test',
        username: 'alice',
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
        avatarColor: null,
        createdAt: '2026-02-24T00:00:00.000Z',
      },
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
})
