import { beforeEach, describe, expect, it, vi } from 'vitest'

vi.mock('$lib/api', () => ({
  apiFetch: vi.fn(),
}))

import { apiFetch } from '$lib/api'
import { createInvite, listInvites, revokeInvite } from './guildApi'

describe('guildApi invites', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('listInvites maps invite wire payload', async () => {
    vi.mocked(apiFetch).mockResolvedValue([
      {
        code: 'code-1',
        type: 'single_use',
        uses_remaining: 1,
        created_by: 'user-1',
        creator_username: 'owner',
        created_at: '2026-02-28T00:00:00.000Z',
        revoked: false,
        invite_url: '/invite/code-1',
      },
    ])

    await expect(listInvites('makers')).resolves.toEqual([
      {
        code: 'code-1',
        type: 'single_use',
        usesRemaining: 1,
        createdBy: 'user-1',
        creatorUsername: 'owner',
        createdAt: '2026-02-28T00:00:00.000Z',
        revoked: false,
        inviteUrl: '/invite/code-1',
      },
    ])

    expect(apiFetch).toHaveBeenCalledWith('/api/v1/guilds/makers/invites')
  })

  it('createInvite sends type payload and maps response', async () => {
    vi.mocked(apiFetch).mockResolvedValue({
      code: 'code-2',
      type: 'reusable',
      uses_remaining: 0,
      created_by: 'user-1',
      creator_username: 'owner',
      created_at: '2026-02-28T00:00:00.000Z',
      revoked: false,
      invite_url: '/invite/code-2',
    })

    await createInvite('makers', { type: 'reusable' })

    expect(apiFetch).toHaveBeenCalledWith('/api/v1/guilds/makers/invites', {
      method: 'POST',
      body: JSON.stringify({ type: 'reusable' }),
    })
  })

  it('revokeInvite calls delete endpoint and maps response', async () => {
    vi.mocked(apiFetch).mockResolvedValue({
      code: 'code-3',
      revoked: true,
    })

    await expect(revokeInvite('makers', 'code-3')).resolves.toEqual({
      code: 'code-3',
      revoked: true,
    })

    expect(apiFetch).toHaveBeenCalledWith(
      '/api/v1/guilds/makers/invites/code-3',
      {
        method: 'DELETE',
      },
    )
  })
})
