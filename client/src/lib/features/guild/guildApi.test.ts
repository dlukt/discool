import { beforeEach, describe, expect, it, vi } from 'vitest'

vi.mock('$lib/api', () => ({
  apiFetch: vi.fn(),
}))

import { apiFetch } from '$lib/api'
import {
  createInvite,
  getInviteMetadata,
  joinGuildByInvite,
  listBans,
  listGuilds,
  listInvites,
  listMembers,
  revokeInvite,
  unban,
  updateMemberRoles,
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

describe('guildApi guild listing', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('maps optional activity and navigation fields with backward compatibility', async () => {
    vi.mocked(apiFetch).mockResolvedValue([
      {
        id: 'guild-1',
        slug: 'lobby',
        name: 'Lobby',
        default_channel_slug: 'general',
        is_owner: true,
        created_at: '2026-02-28T00:00:00.000Z',
        has_unread_activity: true,
        last_viewed_channel_slug: 'announcements',
      },
      {
        id: 'guild-2',
        slug: 'makers',
        name: 'Makers',
        default_channel_slug: 'general',
        is_owner: false,
        created_at: '2026-02-28T00:00:00.000Z',
      },
    ])

    await expect(listGuilds()).resolves.toEqual([
      {
        id: 'guild-1',
        slug: 'lobby',
        name: 'Lobby',
        description: undefined,
        defaultChannelSlug: 'general',
        lastViewedChannelSlug: 'announcements',
        hasUnreadActivity: true,
        isOwner: true,
        iconUrl: undefined,
        createdAt: '2026-02-28T00:00:00.000Z',
      },
      {
        id: 'guild-2',
        slug: 'makers',
        name: 'Makers',
        description: undefined,
        defaultChannelSlug: 'general',
        lastViewedChannelSlug: undefined,
        hasUnreadActivity: undefined,
        isOwner: false,
        iconUrl: undefined,
        createdAt: '2026-02-28T00:00:00.000Z',
      },
    ])
  })
})

describe('guildApi member role assignment', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('listMembers maps member + role assignment payload', async () => {
    vi.mocked(apiFetch).mockResolvedValue({
      members: [
        {
          user_id: 'user-1',
          username: 'manager',
          display_name: 'Role Manager',
          presence_status: 'online',
          highest_role_color: '#3366ff',
          role_ids: ['role-manager'],
          is_owner: false,
          can_assign_roles: true,
        },
      ],
      roles: [
        {
          id: 'role-manager',
          name: 'Role Manager',
          color: '#3366ff',
          position: 1,
          permissions_bitflag: 80,
          is_default: false,
          is_system: false,
          can_edit: true,
          can_delete: true,
          created_at: '2026-02-28T00:00:00.000Z',
        },
      ],
      assignable_role_ids: ['role-manager'],
      can_manage_roles: true,
    })

    await expect(listMembers('makers')).resolves.toEqual({
      members: [
        {
          userId: 'user-1',
          username: 'manager',
          displayName: 'Role Manager',
          avatarColor: undefined,
          presenceStatus: 'online',
          highestRoleColor: '#3366ff',
          roleIds: ['role-manager'],
          isOwner: false,
          canAssignRoles: true,
        },
      ],
      roles: [
        {
          id: 'role-manager',
          name: 'Role Manager',
          color: '#3366ff',
          position: 1,
          permissionsBitflag: 80,
          isDefault: false,
          isSystem: false,
          canEdit: true,
          canDelete: true,
          createdAt: '2026-02-28T00:00:00.000Z',
        },
      ],
      assignableRoleIds: ['role-manager'],
      canManageRoles: true,
    })

    expect(apiFetch).toHaveBeenCalledWith('/api/v1/guilds/makers/members')
  })

  it('updateMemberRoles posts expected payload and maps member response', async () => {
    vi.mocked(apiFetch).mockResolvedValue({
      user_id: 'user-2',
      username: 'target',
      display_name: 'Target User',
      presence_status: 'offline',
      highest_role_color: '#22aa88',
      role_ids: ['role-helper'],
      is_owner: false,
      can_assign_roles: true,
    })

    await expect(
      updateMemberRoles('makers', 'user-2', { roleIds: ['role-helper'] }),
    ).resolves.toEqual({
      userId: 'user-2',
      username: 'target',
      displayName: 'Target User',
      avatarColor: undefined,
      presenceStatus: 'offline',
      highestRoleColor: '#22aa88',
      roleIds: ['role-helper'],
      isOwner: false,
      canAssignRoles: true,
    })

    expect(apiFetch).toHaveBeenCalledWith(
      '/api/v1/guilds/makers/members/user-2/roles',
      {
        method: 'PATCH',
        body: JSON.stringify({ role_ids: ['role-helper'] }),
      },
    )
  })
})

describe('guildApi bans', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('listBans maps ban list payload', async () => {
    vi.mocked(apiFetch).mockResolvedValue([
      {
        id: 'ban-1',
        target_user_id: 'user-target',
        target_username: 'target-user',
        target_display_name: 'Target User',
        actor_user_id: 'user-owner',
        actor_username: 'owner',
        actor_display_name: 'Owner',
        reason: 'repeat abuse',
        delete_messages_window_seconds: 86400,
        created_at: '2026-03-01T00:00:00.000Z',
      },
    ])

    await expect(listBans('makers')).resolves.toEqual([
      {
        id: 'ban-1',
        targetUserId: 'user-target',
        targetUsername: 'target-user',
        targetDisplayName: 'Target User',
        actorUserId: 'user-owner',
        actorUsername: 'owner',
        actorDisplayName: 'Owner',
        reason: 'repeat abuse',
        deleteMessagesWindowSeconds: 86400,
        createdAt: '2026-03-01T00:00:00.000Z',
      },
    ])

    expect(apiFetch).toHaveBeenCalledWith(
      '/api/v1/guilds/makers/moderation/bans',
    )
  })

  it('unban calls delete endpoint and maps response', async () => {
    vi.mocked(apiFetch).mockResolvedValue({
      id: 'ban-1',
      guild_slug: 'makers',
      target_user_id: 'user-target',
      unbanned_by_user_id: 'user-owner',
      unbanned_at: '2026-03-02T00:00:00.000Z',
      updated_at: '2026-03-02T00:00:00.000Z',
    })

    await expect(unban('makers', 'ban-1')).resolves.toEqual({
      id: 'ban-1',
      guildSlug: 'makers',
      targetUserId: 'user-target',
      unbannedByUserId: 'user-owner',
      unbannedAt: '2026-03-02T00:00:00.000Z',
      updatedAt: '2026-03-02T00:00:00.000Z',
    })

    expect(apiFetch).toHaveBeenCalledWith(
      '/api/v1/guilds/makers/moderation/bans/ban-1',
      { method: 'DELETE' },
    )
  })
})
