import { fireEvent, render, waitFor, within } from '@testing-library/svelte'
import { beforeEach, describe, expect, it, vi } from 'vitest'

const { goto, guildState, channelState, identityState } = vi.hoisted(() => {
  const goto = vi.fn()
  const guildState = {
    loadGuilds: vi.fn(),
    bySlug: vi.fn(),
    loadMembers: vi.fn(),
    memberRoleDataForGuild: vi.fn(),
  }
  const channelState = {
    activeGuild: null as string | null,
    channels: [] as Array<{
      id: string
      slug: string
      name: string
      channelType: 'text' | 'voice'
      position: number
      isDefault: boolean
      categorySlug?: string | null
      createdAt: string
    }>,
    categories: [] as Array<{
      id: string
      slug: string
      name: string
      position: number
      collapsed: boolean
      createdAt: string
    }>,
    loading: false,
    loadChannels: vi.fn(),
    createChannel: vi.fn(),
    createCategory: vi.fn(),
    updateChannel: vi.fn(),
    updateCategory: vi.fn(),
    deleteChannel: vi.fn(),
    deleteCategory: vi.fn(),
    reorderChannels: vi.fn(),
    reorderCategories: vi.fn(),
    setCategoryCollapsed: vi.fn(),
    moveChannel: vi.fn(),
    loadChannelPermissionOverrides: vi.fn(),
    upsertChannelPermissionOverride: vi.fn(),
    deleteChannelPermissionOverride: vi.fn(),
  }
  const identityState = {
    session: {
      user: {
        id: 'user-owner',
      },
    },
  }

  return {
    goto,
    guildState,
    channelState,
    identityState,
  }
})

vi.mock('@mateothegreat/svelte5-router', () => ({
  goto,
  route: () => undefined,
}))

vi.mock('$lib/features/guild/guildStore.svelte', () => ({
  guildState,
}))

vi.mock('$lib/features/identity/identityStore.svelte', () => ({
  identityState,
}))

vi.mock('./channelStore.svelte', () => ({
  channelState,
}))

import ChannelList from './ChannelList.svelte'

describe('ChannelList', () => {
  beforeEach(() => {
    vi.clearAllMocks()

    guildState.loadGuilds.mockResolvedValue([])
    guildState.bySlug.mockReturnValue({
      id: 'guild-1',
      slug: 'lobby',
      name: 'Lobby',
      defaultChannelSlug: 'general',
      isOwner: true,
      createdAt: '2026-02-28T00:00:00.000Z',
    })
    const memberRoleData = {
      members: [
        {
          userId: 'user-owner',
          username: 'owner',
          displayName: 'Owner',
          avatarColor: '#99aab5',
          highestRoleColor: '#99aab5',
          roleIds: [],
          isOwner: true,
          canAssignRoles: false,
        },
      ],
      roles: [
        {
          id: 'role-everyone',
          name: '@everyone',
          color: '#99aab5',
          position: 2_147_483_647,
          permissionsBitflag: 5633,
          isDefault: true,
          isSystem: true,
          canEdit: false,
          canDelete: false,
          createdAt: '2026-02-28T00:00:00.000Z',
        },
      ],
      assignableRoleIds: [] as string[],
      canManageRoles: false,
    }
    guildState.memberRoleDataForGuild.mockReturnValue(memberRoleData)
    guildState.loadMembers.mockResolvedValue(memberRoleData)
    identityState.session = {
      user: {
        id: 'user-owner',
      },
    }

    channelState.activeGuild = 'lobby'
    channelState.loading = false
    channelState.channels = [
      {
        id: 'channel-general',
        slug: 'general',
        name: 'general',
        channelType: 'text',
        position: 0,
        isDefault: true,
        categorySlug: null,
        createdAt: '2026-02-28T00:00:00.000Z',
      },
      {
        id: 'channel-voice',
        slug: 'team-voice',
        name: 'Team Voice',
        channelType: 'voice',
        position: 1,
        isDefault: false,
        categorySlug: null,
        createdAt: '2026-02-28T00:00:00.000Z',
      },
    ]
    channelState.categories = []

    channelState.loadChannels.mockImplementation(async (guildSlug: string) => {
      channelState.activeGuild = guildSlug
      return channelState.channels
    })

    channelState.createChannel.mockImplementation(
      async (
        _guildSlug: string,
        input: { name: string; channelType: 'text' | 'voice' },
      ) => {
        const created = {
          id: 'channel-created',
          slug:
            input.channelType === 'voice'
              ? 'created-voice'
              : input.name.toLowerCase().replaceAll(' ', '-'),
          name: input.name,
          channelType: input.channelType,
          position: channelState.channels.length,
          isDefault: false,
          createdAt: '2026-02-28T00:00:00.000Z',
        }
        channelState.channels = [...channelState.channels, created]
        return created
      },
    )
    channelState.createCategory.mockResolvedValue({
      id: 'category-created',
      slug: 'ops',
      name: 'Ops',
      position: 0,
      collapsed: false,
      createdAt: '2026-02-28T00:00:00.000Z',
    })

    channelState.updateChannel.mockImplementation(
      async (
        _guildSlug: string,
        _channelSlug: string,
        input: { name: string },
      ) => ({
        id: 'channel-general',
        slug: 'general-chat',
        name: input.name,
        channelType: 'text' as const,
        position: 0,
        isDefault: true,
        createdAt: '2026-02-28T00:00:00.000Z',
      }),
    )
    channelState.updateCategory.mockResolvedValue({
      id: 'category-updated',
      slug: 'operations',
      name: 'Operations',
      position: 0,
      collapsed: false,
      createdAt: '2026-02-28T00:00:00.000Z',
    })

    channelState.deleteChannel.mockResolvedValue({
      deletedSlug: 'team-voice',
      fallbackChannelSlug: 'general',
    })
    channelState.deleteCategory.mockResolvedValue({
      deletedSlug: 'ops',
      reassignedChannelCount: 1,
    })

    channelState.reorderChannels.mockImplementation(
      async (_guildSlug: string, channelSlugs: string[]) => {
        const bySlug = new Map(
          channelState.channels.map((item) => [item.slug, item]),
        )
        channelState.channels = channelSlugs
          .map((slug, index) => {
            const channel = bySlug.get(slug)
            if (!channel) return null
            return { ...channel, position: index }
          })
          .filter((channel) => channel !== null)
        return channelState.channels
      },
    )
    channelState.reorderCategories.mockResolvedValue(channelState.categories)

    channelState.setCategoryCollapsed.mockResolvedValue(undefined)
    channelState.moveChannel.mockImplementation(
      async (
        _guildSlug: string,
        channelSlug: string,
        categorySlug: string | null,
        position: number,
      ) => {
        const current = channelState.channels.find(
          (channel) => channel.slug === channelSlug,
        )
        if (!current) return channelState.channels
        const remaining = channelState.channels.filter(
          (channel) => channel.slug !== channelSlug,
        )
        const bucket = remaining
          .filter((channel) => (channel.categorySlug ?? null) === categorySlug)
          .sort((a, b) => a.position - b.position)
        const nextBucket = [...bucket]
        nextBucket.splice(position, 0, {
          ...current,
          categorySlug,
        })
        const bySlug = new Map(
          remaining.map((channel) => [channel.slug, channel]),
        )
        const ordered = [
          ...nextBucket,
          ...remaining.filter(
            (channel) => !nextBucket.some((item) => item.slug === channel.slug),
          ),
        ]
        channelState.channels = ordered.map((channel, index) => ({
          ...((bySlug.get(channel.slug) ??
            channel) as (typeof channelState.channels)[number]),
          categorySlug:
            channel.slug === channelSlug
              ? categorySlug
              : (bySlug.get(channel.slug)?.categorySlug ?? null),
          position: index,
        }))
        return channelState.channels
      },
    )

    channelState.loadChannelPermissionOverrides.mockResolvedValue({
      roles: [
        {
          id: 'role-manager',
          name: 'Channel Manager',
          color: '#3366ff',
          position: 0,
          isDefault: false,
          isSystem: false,
        },
        {
          id: 'role-everyone',
          name: '@everyone',
          color: '#99aab5',
          position: 1,
          isDefault: true,
          isSystem: true,
        },
      ],
      overrides: [],
    })
    channelState.upsertChannelPermissionOverride.mockResolvedValue({
      roleId: 'role-manager',
      allowBitflag: 4096,
      denyBitflag: 0,
    })
    channelState.deleteChannelPermissionOverride.mockResolvedValue({
      roleId: 'role-manager',
      removed: true,
    })
  })

  it('opens create dialog and validates required channel name on blur', async () => {
    const { getByLabelText, findByText } = render(ChannelList, {
      activeGuild: 'lobby',
      activeChannel: 'general',
    })

    await fireEvent.click(getByLabelText('Create channel'))
    const nameInput = getByLabelText('Channel name')
    await fireEvent.input(nameInput, { target: { value: '   ' } })
    await fireEvent.blur(nameInput)

    expect(await findByText('Channel name is required.')).toBeInTheDocument()
  })

  it('creates a voice channel with type selector + form submit', async () => {
    const { getByLabelText, getByRole } = render(ChannelList, {
      activeGuild: 'lobby',
      activeChannel: 'general',
    })

    await fireEvent.click(getByLabelText('Create channel'))
    const dialog = getByRole('dialog', { name: 'Create channel' })

    await fireEvent.input(within(dialog).getByLabelText('Channel name'), {
      target: { value: 'Created Voice' },
    })
    await fireEvent.change(within(dialog).getByLabelText('Channel type'), {
      target: { value: 'voice' },
    })
    await fireEvent.submit(within(dialog).getByTestId('create-channel-form'))

    await waitFor(() => {
      expect(channelState.createChannel).toHaveBeenCalledWith('lobby', {
        name: 'Created Voice',
        channelType: 'voice',
        categorySlug: null,
      })
    })
  })

  it('renders # for text channels and speaker icon for voice channels', () => {
    const { getByTestId } = render(ChannelList, {
      activeGuild: 'lobby',
      activeChannel: 'general',
    })

    expect(getByTestId('channel-icon-general')).toHaveTextContent('#')
    expect(getByTestId('channel-icon-team-voice')).toHaveTextContent('🔊')
  })

  it('handles context menu rename and delete flows with warning copy', async () => {
    const { getByLabelText, getByRole, findByText } = render(ChannelList, {
      activeGuild: 'lobby',
      activeChannel: 'general',
    })

    await fireEvent.click(getByLabelText('Open channel actions for general'))
    await fireEvent.click(getByRole('menuitem', { name: 'Edit channel' }))

    const editDialog = getByRole('dialog', { name: 'Edit channel' })
    await fireEvent.input(within(editDialog).getByLabelText('Channel name'), {
      target: { value: 'General Chat' },
    })
    await fireEvent.submit(
      within(editDialog).getByTestId('rename-channel-form'),
    )

    await waitFor(() => {
      expect(channelState.updateChannel).toHaveBeenCalledWith(
        'lobby',
        'general',
        {
          name: 'General Chat',
        },
      )
    })
    await waitFor(() => {
      expect(goto).toHaveBeenCalledWith('/lobby/general-chat')
    })

    await fireEvent.click(getByLabelText('Open channel actions for Team Voice'))
    await fireEvent.click(getByRole('menuitem', { name: 'Delete channel' }))

    const deleteDialog = getByRole('dialog', { name: 'Delete channel' })
    expect(
      await findByText(
        'This will permanently delete all messages in this channel',
      ),
    ).toBeInTheDocument()

    await fireEvent.click(
      within(deleteDialog).getByRole('button', { name: 'Delete channel' }),
    )

    await waitFor(() => {
      expect(channelState.deleteChannel).toHaveBeenCalledWith(
        'lobby',
        'team-voice',
      )
    })
  })

  it('supports non-pointer reorder via move actions', async () => {
    const { getByLabelText, getByRole } = render(ChannelList, {
      activeGuild: 'lobby',
      activeChannel: 'general',
    })

    await fireEvent.click(getByLabelText('Open channel actions for general'))
    await fireEvent.click(getByRole('menuitem', { name: 'Move down' }))

    await waitFor(() => {
      expect(channelState.moveChannel).toHaveBeenCalledWith(
        'lobby',
        'general',
        null,
        1,
      )
    })
  })

  it('allows MANAGE_CHANNELS members to access channel actions', async () => {
    guildState.bySlug.mockReturnValue({
      id: 'guild-1',
      slug: 'lobby',
      name: 'Lobby',
      defaultChannelSlug: 'general',
      isOwner: false,
      createdAt: '2026-02-28T00:00:00.000Z',
    })
    const memberRoleData = {
      members: [
        {
          userId: 'user-manager',
          username: 'manager',
          displayName: 'Manager',
          avatarColor: '#3366ff',
          highestRoleColor: '#3366ff',
          roleIds: ['role-manager'],
          isOwner: false,
          canAssignRoles: false,
        },
      ],
      roles: [
        {
          id: 'role-manager',
          name: 'Channel Manager',
          color: '#3366ff',
          position: 1,
          permissionsBitflag: 2,
          isDefault: false,
          isSystem: false,
          canEdit: true,
          canDelete: true,
          createdAt: '2026-02-28T00:00:00.000Z',
        },
        {
          id: 'role-everyone',
          name: '@everyone',
          color: '#99aab5',
          position: 2_147_483_647,
          permissionsBitflag: 5633,
          isDefault: true,
          isSystem: true,
          canEdit: false,
          canDelete: false,
          createdAt: '2026-02-28T00:00:00.000Z',
        },
      ],
      assignableRoleIds: [] as string[],
      canManageRoles: false,
    }
    guildState.memberRoleDataForGuild.mockReturnValue(memberRoleData)
    guildState.loadMembers.mockResolvedValue(memberRoleData)
    identityState.session = {
      user: {
        id: 'user-manager',
      },
    }

    const { getByLabelText } = render(ChannelList, {
      activeGuild: 'lobby',
      activeChannel: 'general',
    })

    expect(
      getByLabelText('Open channel actions for general'),
    ).toBeInTheDocument()
    await waitFor(() => {
      expect(guildState.loadMembers).toHaveBeenCalledWith('lobby')
    })
  })

  it('adjusts drag-drop position when moving downward in same category', async () => {
    channelState.channels = [
      {
        id: 'channel-general',
        slug: 'general',
        name: 'general',
        channelType: 'text',
        position: 0,
        isDefault: true,
        categorySlug: null,
        createdAt: '2026-02-28T00:00:00.000Z',
      },
      {
        id: 'channel-updates',
        slug: 'updates',
        name: 'updates',
        channelType: 'text',
        position: 1,
        isDefault: false,
        categorySlug: null,
        createdAt: '2026-02-28T00:00:00.000Z',
      },
      {
        id: 'channel-voice',
        slug: 'team-voice',
        name: 'Team Voice',
        channelType: 'voice',
        position: 2,
        isDefault: false,
        categorySlug: null,
        createdAt: '2026-02-28T00:00:00.000Z',
      },
    ]

    const { getByTestId } = render(ChannelList, {
      activeGuild: 'lobby',
      activeChannel: 'general',
    })

    const dataTransfer = {
      setData: vi.fn(),
      getData: vi.fn(() => 'general'),
      effectAllowed: 'move',
      dropEffect: 'move',
    }

    await fireEvent.dragStart(getByTestId('channel-item-general'), {
      dataTransfer,
    })
    await fireEvent.drop(getByTestId('channel-item-team-voice'), {
      dataTransfer,
    })

    await waitFor(() => {
      expect(channelState.moveChannel).toHaveBeenCalledWith(
        'lobby',
        'general',
        null,
        1,
      )
    })
  })

  it('renders channel permission overrides and persists tri-state selections', async () => {
    const { getByLabelText, getByRole } = render(ChannelList, {
      activeGuild: 'lobby',
      activeChannel: 'general',
    })

    await fireEvent.click(getByLabelText('Open channel actions for general'))
    await fireEvent.click(
      getByRole('menuitem', { name: 'Permission overrides' }),
    )

    const dialog = getByRole('dialog', { name: 'Channel permissions' })
    await waitFor(() => {
      expect(channelState.loadChannelPermissionOverrides).toHaveBeenCalledWith(
        'lobby',
        'general',
        true,
      )
    })

    await fireEvent.click(
      within(dialog).getByRole('button', { name: 'Channel Manager' }),
    )

    await fireEvent.click(
      within(dialog).getByLabelText('Allow View channel for Channel Manager'),
    )
    await waitFor(() => {
      expect(
        channelState.upsertChannelPermissionOverride,
      ).toHaveBeenNthCalledWith(1, 'lobby', 'general', 'role-manager', {
        allowBitflag: 4096,
        denyBitflag: 0,
      })
    })

    vi.mocked(
      channelState.upsertChannelPermissionOverride,
    ).mockResolvedValueOnce({
      roleId: 'role-manager',
      allowBitflag: 0,
      denyBitflag: 4096,
    })
    await fireEvent.click(
      within(dialog).getByLabelText('Deny View channel for Channel Manager'),
    )
    await waitFor(() => {
      expect(
        channelState.upsertChannelPermissionOverride,
      ).toHaveBeenNthCalledWith(2, 'lobby', 'general', 'role-manager', {
        allowBitflag: 0,
        denyBitflag: 4096,
      })
    })

    await fireEvent.click(
      within(dialog).getByLabelText('Inherit View channel for Channel Manager'),
    )
    await waitFor(() => {
      expect(channelState.deleteChannelPermissionOverride).toHaveBeenCalledWith(
        'lobby',
        'general',
        'role-manager',
      )
    })
  })

  it('opens channel actions from keyboard context-menu shortcut', async () => {
    const { getByRole, getByTestId } = render(ChannelList, {
      activeGuild: 'lobby',
      activeChannel: 'general',
    })

    await fireEvent.keyDown(getByTestId('channel-link-general'), {
      key: 'F10',
      shiftKey: true,
    })

    expect(
      getByRole('menuitem', { name: 'Permission overrides' }),
    ).toBeInTheDocument()
  })

  it('rolls back tri-state selection on permission override save failure', async () => {
    vi.mocked(
      channelState.upsertChannelPermissionOverride,
    ).mockRejectedValueOnce(new Error('Failed to save override'))

    const { getByLabelText, getByRole, findByText } = render(ChannelList, {
      activeGuild: 'lobby',
      activeChannel: 'general',
    })

    await fireEvent.click(getByLabelText('Open channel actions for general'))
    await fireEvent.click(
      getByRole('menuitem', { name: 'Permission overrides' }),
    )
    const dialog = getByRole('dialog', { name: 'Channel permissions' })

    const inherit = within(dialog).getByLabelText(
      'Inherit View channel for Channel Manager',
    ) as HTMLInputElement
    expect(inherit).toBeChecked()

    await fireEvent.click(
      within(dialog).getByLabelText('Allow View channel for Channel Manager'),
    )

    expect(await findByText('Failed to save override')).toBeInTheDocument()
    expect(inherit).toBeChecked()
  })

  it('renders category groups and toggles collapsed state from keyboard', async () => {
    channelState.categories = [
      {
        id: 'category-ops',
        slug: 'ops',
        name: 'Ops',
        position: 0,
        collapsed: false,
        createdAt: '2026-02-28T00:00:00.000Z',
      },
    ]
    channelState.channels = [
      {
        id: 'channel-general',
        slug: 'general',
        name: 'general',
        channelType: 'text',
        position: 0,
        isDefault: true,
        categorySlug: null,
        createdAt: '2026-02-28T00:00:00.000Z',
      },
      {
        id: 'channel-incidents',
        slug: 'incidents',
        name: 'Incidents',
        channelType: 'text',
        position: 0,
        isDefault: false,
        categorySlug: 'ops',
        createdAt: '2026-02-28T00:00:00.000Z',
      },
    ]

    const { getByRole, queryByTestId } = render(ChannelList, {
      activeGuild: 'lobby',
      activeChannel: 'general',
    })

    const header = getByRole('button', { name: /Toggle category OPS/i })
    expect(queryByTestId('channel-item-incidents')).toBeInTheDocument()

    await fireEvent.keyDown(header, { key: 'Enter' })

    await waitFor(() => {
      expect(channelState.setCategoryCollapsed).toHaveBeenCalledWith(
        'lobby',
        'ops',
        true,
      )
    })
  })
})
