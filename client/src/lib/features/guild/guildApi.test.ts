import { beforeEach, describe, expect, it, vi } from 'vitest'

vi.mock('$lib/api', () => ({
  apiFetch: vi.fn(),
}))

import { apiFetch } from '$lib/api'
import {
  createInvite,
  getInviteMetadata,
  joinGuildByInvite,
  listInvites,
  revokeInvite,
} from './guildApi'

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

  it('getInviteMetadata maps invite metadata payload', async () => {
    vi.mocked(apiFetch).mockResolvedValue({
      code: 'code-4',
      guild_slug: 'makers',
      guild_name: 'Makers Hub',
      guild_icon_url: '/api/v1/guilds/makers/icon',
      default_channel_slug: 'general',
      welcome_screen: {
        enabled: false,
      },
    })

    await expect(getInviteMetadata('code-4')).resolves.toEqual({
      code: 'code-4',
      guildSlug: 'makers',
      guildName: 'Makers Hub',
      guildIconUrl: '/api/v1/guilds/makers/icon',
      defaultChannelSlug: 'general',
      welcomeScreen: {
        enabled: false,
        title: undefined,
        rules: undefined,
        acceptLabel: undefined,
      },
    })

    expect(apiFetch).toHaveBeenCalledWith('/api/v1/invites/code-4')
  })

  it('joinGuildByInvite posts join request and maps response', async () => {
    vi.mocked(apiFetch).mockResolvedValue({
      guild_slug: 'makers',
      guild_name: 'Makers Hub',
      guild_icon_url: '/api/v1/guilds/makers/icon',
      default_channel_slug: 'general',
      already_member: false,
      welcome_screen: {
        enabled: false,
      },
    })

    await expect(joinGuildByInvite('code-5')).resolves.toEqual({
      guildSlug: 'makers',
      guildName: 'Makers Hub',
      guildIconUrl: '/api/v1/guilds/makers/icon',
      defaultChannelSlug: 'general',
      alreadyMember: false,
      welcomeScreen: {
        enabled: false,
        title: undefined,
        rules: undefined,
        acceptLabel: undefined,
      },
    })

    expect(apiFetch).toHaveBeenCalledWith('/api/v1/invites/code-5/join', {
      method: 'POST',
      body: '{}',
    })
  })
})
