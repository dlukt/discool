import { fireEvent, render, waitFor } from '@testing-library/svelte'
import { beforeEach, describe, expect, it, vi } from 'vitest'

const { routerGoto } = vi.hoisted(() => ({
  routerGoto: vi.fn(async () => {}),
}))

type MockLifecycle =
  | 'connecting'
  | 'connected'
  | 'reconnecting'
  | 'disconnected'

const { wsLifecycleState, lifecycleListeners } = vi.hoisted(() => ({
  wsLifecycleState: { value: 'disconnected' as MockLifecycle },
  lifecycleListeners: new Set<(state: MockLifecycle) => void>(),
}))

const { blockState } = vi.hoisted(() => ({
  blockState: {
    isBlocked: vi.fn((_userId: string) => false),
  },
}))

const { voiceState } = vi.hoisted(() => ({
  voiceState: {
    status: 'connected' as
      | 'idle'
      | 'connecting'
      | 'connected'
      | 'disconnected'
      | 'retrying'
      | 'failed',
    activateVoiceChannel: vi.fn(),
    clearActiveChannel: vi.fn(),
    toggleMute: vi.fn(),
    toggleDeafen: vi.fn(),
    disconnect: vi.fn(),
  },
}))

const { guildState, channelState, dmState } = vi.hoisted(() => {
  const guildState = {
    version: 0,
    guilds: [
      {
        id: 'guild-1',
        slug: 'lobby',
        name: 'Lobby',
        defaultChannelSlug: 'general',
        lastViewedChannelSlug: 'general',
      },
    ],
    bySlug: vi.fn(
      (slug: string) =>
        guildState.guilds.find((guild) => guild.slug === slug) ?? null,
    ),
    loadGuilds: vi.fn(async () => guildState.guilds),
    hasUnreadActivity: vi.fn(() => false),
    memberRoleDataForGuild: vi.fn(() => ({
      members: [
        {
          userId: 'user-1',
          username: 'darko',
          displayName: 'Darko',
          roleIds: ['role-everyone'],
          isOwner: true,
          presenceStatus: 'online',
        },
      ],
      roles: [
        {
          id: 'role-everyone',
          name: '@everyone',
          color: '#94a3b8',
          position: 0,
          isDefault: true,
          permissionsBitflag: Number.MAX_SAFE_INTEGER,
        },
      ],
      assignableRoleIds: [] as string[],
    })),
    loadMembers: vi.fn(async () => undefined),
    updateMemberRoles: vi.fn(async () => ({
      members: [],
      roles: [],
      assignableRoleIds: [],
    })),
    memberByUserId: vi.fn(() => null),
    roleNameForMember: vi.fn(() => null),
  }

  const channelState = {
    activeGuild: 'lobby',
    loading: false,
    channels: [
      {
        id: 'channel-general',
        guildId: 'guild-1',
        slug: 'general',
        name: 'general',
        topic: null as string | null,
        kind: 'text',
        position: 0,
        categorySlug: null as string | null,
        isDefault: true,
      },
      {
        id: 'channel-random',
        guildId: 'guild-1',
        slug: 'random',
        name: 'random',
        topic: null as string | null,
        kind: 'text',
        position: 1,
        categorySlug: null as string | null,
        isDefault: false,
      },
    ] as Array<{
      id: string
      guildId: string
      slug: string
      name: string
      topic: string | null
      kind: string
      position: number
      categorySlug: string | null
      isDefault: boolean
    }>,
    channelsByGuild: {} as Record<
      string,
      Array<{
        id: string
        guildId: string
        slug: string
        name: string
        topic: string | null
        kind: string
        position: number
        categorySlug: string | null
        isDefault: boolean
      }>
    >,
    categories: [] as Array<{ slug: string; name: string; position: number }>,
    orderedChannelsForGuild: vi.fn(
      (guildSlug: string) => channelState.channelsByGuild[guildSlug] ?? [],
    ),
    loadChannels: vi.fn(async (guildSlug: string) => {
      channelState.activeGuild = guildSlug
      const nextChannels = channelState.channelsByGuild[guildSlug] ?? []
      channelState.channels = nextChannels
      return nextChannels
    }),
    noteMessageActivity: vi.fn(),
    setChannelUnreadActivity: vi.fn(),
    createChannel: vi.fn(async () => channelState.channels[0]),
    createCategory: vi.fn(async () => ({
      slug: 'new-category',
      name: 'New Category',
      position: 0,
    })),
    updateChannel: vi.fn(async () => channelState.channels[0]),
    updateCategory: vi.fn(async () => ({
      slug: 'updated-category',
      name: 'Updated Category',
      position: 0,
    })),
    deleteChannel: vi.fn(async () => null),
    deleteCategory: vi.fn(async () => undefined),
    loadChannelPermissionOverrides: vi.fn(async () => ({
      overrides: [],
      roles: [],
    })),
    deleteChannelPermissionOverride: vi.fn(async () => undefined),
    upsertChannelPermissionOverride: vi.fn(async () => ({
      allowBitflag: 0,
      denyBitflag: 0,
    })),
    moveChannel: vi.fn(async () => undefined),
    reorderCategories: vi.fn(async () => undefined),
    setCategoryCollapsed: vi.fn(async () => undefined),
  }

  const dmState = {
    version: 0,
    conversations: [] as Array<{
      dmSlug: string
      participant: {
        userId: string
        username: string
        displayName: string
        avatarColor: string | null
      }
      createdAt: string
      updatedAt: string
      lastMessagePreview: string | null
      lastMessageAt: string | null
      hasUnreadActivity: boolean
    }>,
    ensureLoaded: vi.fn(async () => {}),
    openOrCreateDm: vi.fn(async () => ({
      dmSlug: 'dm-1',
      participant: {
        userId: 'user-2',
        username: 'bob',
        displayName: 'Bob',
        avatarColor: '#22aa88',
      },
      createdAt: '2026-02-28T00:00:00Z',
      updatedAt: '2026-02-28T00:00:00Z',
      lastMessagePreview: null,
      lastMessageAt: null,
      hasUnreadActivity: false,
    })),
    setActiveDm: vi.fn(),
    hasUnreadActivity: vi.fn(() => false),
  }

  return { guildState, channelState, dmState }
})

vi.mock('@mateothegreat/svelte5-router', () => ({
  goto: routerGoto,
  route: vi.fn(() => () => {}),
}))

vi.mock('$lib/ws/client', () => ({
  wsClient: {
    subscribe: vi.fn(() => () => {}),
    getLifecycleState: vi.fn(() => wsLifecycleState.value),
    subscribeLifecycle: vi.fn((listener: (state: MockLifecycle) => void) => {
      lifecycleListeners.add(listener)
      listener(wsLifecycleState.value)
      return () => lifecycleListeners.delete(listener)
    }),
    ensureConnected: vi.fn(),
    disconnect: vi.fn(),
    setSubscription: vi.fn(),
    send: vi.fn(() => true),
  },
}))

vi.mock('$lib/features/guild/guildStore.svelte', () => ({
  guildState,
}))

vi.mock('$lib/features/channel/channelStore.svelte', () => ({
  channelState,
}))

vi.mock('$lib/features/dm/dmStore.svelte', () => ({
  dmState,
}))

vi.mock('$lib/features/identity/blockStore.svelte', () => ({
  blockState,
}))

vi.mock('$lib/features/voice/voiceStore.svelte', () => ({
  voiceState,
}))

vi.mock('$lib/features/guild/GuildRail.svelte', async () => ({
  default: (await import('./__mocks__/GuildRailMock.svelte')).default,
}))

vi.mock('$lib/features/channel/ChannelList.svelte', async () => ({
  default: (await import('./__mocks__/ChannelListMock.svelte')).default,
}))

vi.mock('$lib/features/chat/MessageArea.svelte', async () => ({
  default: (await import('./__mocks__/MessageAreaMock.svelte')).default,
}))

vi.mock('$lib/features/members/MemberList.svelte', async () => ({
  default: (await import('./__mocks__/MemberListMock.svelte')).default,
}))

import { messageState } from '$lib/features/chat/messageStore.svelte'
import ShellRoute from './ShellRoute.svelte'

type RenderProps = {
  mode: 'home' | 'channel' | 'dm' | 'settings' | 'admin'
  route: {
    result: {
      path: {
        condition: 'exact-match'
        original: string
        params?: Record<string, string>
      }
      querystring: {
        condition: 'exact-match'
        original: Record<
          string,
          string | number | boolean | string[] | number[] | boolean[]
        >
        params?: Record<
          string,
          string | number | boolean | string[] | number[] | boolean[]
        >
      }
      status: number
    }
  }
  isAdmin: boolean
  displayName: string
  showRecoveryNudge: boolean
  onOpenSettings: () => void
  onDismissRecoveryNudge: () => void
  onLogout: () => void
  onRouteResolved: (path: string) => void
}

function setViewport(width: number) {
  Object.defineProperty(window, 'innerWidth', {
    value: width,
    writable: true,
    configurable: true,
  })
  window.dispatchEvent(new Event('resize'))
}

function createLobbyGuild() {
  return {
    id: 'guild-1',
    slug: 'lobby',
    name: 'Lobby',
    defaultChannelSlug: 'general',
    lastViewedChannelSlug: 'general',
  }
}

function createEngineeringGuild() {
  return {
    id: 'guild-2',
    slug: 'engineering',
    name: 'Engineering',
    defaultChannelSlug: 'announcements',
    lastViewedChannelSlug: 'announcements',
  }
}

function createLobbyChannels() {
  return [
    {
      id: 'channel-general',
      guildId: 'guild-1',
      slug: 'general',
      name: 'general',
      topic: null,
      kind: 'text',
      position: 0,
      categorySlug: null,
      isDefault: true,
    },
    {
      id: 'channel-random',
      guildId: 'guild-1',
      slug: 'random',
      name: 'random',
      topic: null,
      kind: 'text',
      position: 1,
      categorySlug: null,
      isDefault: false,
    },
  ]
}

function createEngineeringChannels() {
  return [
    {
      id: 'channel-announcements',
      guildId: 'guild-2',
      slug: 'announcements',
      name: 'announcements',
      topic: null,
      kind: 'text',
      position: 0,
      categorySlug: null,
      isDefault: true,
    },
  ]
}

function buildProps(overrides: Partial<RenderProps> = {}): RenderProps {
  return {
    mode: 'channel',
    route: {
      result: {
        path: {
          condition: 'exact-match',
          original: '/lobby/general',
          params: { guild: 'lobby', channel: 'general' },
        },
        querystring: {
          condition: 'exact-match',
          original: {},
          params: {},
        },
        status: 200,
      },
    },
    isAdmin: false,
    displayName: 'Darko',
    showRecoveryNudge: false,
    onOpenSettings: vi.fn(),
    onDismissRecoveryNudge: vi.fn(),
    onLogout: vi.fn(),
    onRouteResolved: vi.fn(),
    ...overrides,
  }
}

function setWsLifecycleState(state: MockLifecycle) {
  wsLifecycleState.value = state
  for (const listener of lifecycleListeners) {
    listener(state)
  }
}

describe('ShellRoute', () => {
  beforeEach(() => {
    setViewport(1280)
    wsLifecycleState.value = 'disconnected'
    lifecycleListeners.clear()
    routerGoto.mockClear()
    guildState.guilds = [createLobbyGuild()]
    channelState.activeGuild = 'lobby'
    channelState.channelsByGuild = {
      lobby: createLobbyChannels(),
    }
    channelState.channels = [...channelState.channelsByGuild.lobby]
    channelState.orderedChannelsForGuild.mockClear()
    channelState.orderedChannelsForGuild.mockImplementation(
      (guildSlug: string) => channelState.channelsByGuild[guildSlug] ?? [],
    )
    channelState.loadChannels.mockClear()
    channelState.loadChannels.mockImplementation(async (guildSlug: string) => {
      channelState.activeGuild = guildSlug
      const nextChannels = channelState.channelsByGuild[guildSlug] ?? []
      channelState.channels = nextChannels
      return nextChannels
    })
    dmState.conversations = []
    dmState.ensureLoaded.mockClear()
    dmState.openOrCreateDm.mockClear()
    blockState.isBlocked.mockReset()
    blockState.isBlocked.mockReturnValue(false)
    voiceState.status = 'connected'
    voiceState.activateVoiceChannel.mockClear()
    voiceState.clearActiveChannel.mockClear()
    voiceState.toggleMute.mockClear()
    voiceState.toggleDeafen.mockClear()
    voiceState.disconnect.mockClear()
  })

  it('renders skip link as the first focusable element', async () => {
    const props = buildProps()
    const { container } = render(ShellRoute, props)
    const skipLink = container.querySelector('a[href="#main-content"]')
    const firstFocusable = container.querySelector(
      'a[href],button,[tabindex]:not([tabindex="-1"])',
    )

    expect(skipLink).toBeInTheDocument()
    expect(firstFocusable).toBe(skipLink)

    await waitFor(() => {
      expect(container.querySelector('#main-content')).toHaveFocus()
    })
  })

  it('shows tablet member list only after toggle', async () => {
    setViewport(900)
    const props = buildProps()
    const { getByRole, queryByTestId, findByTestId } = render(ShellRoute, props)

    expect(queryByTestId('tablet-member-list')).not.toBeInTheDocument()
    await fireEvent.click(getByRole('button', { name: 'Toggle members' }))
    expect(await findByTestId('tablet-member-list')).toBeInTheDocument()
  })

  it('keeps desktop member rail mounted at fixed sidebar width', () => {
    setViewport(1280)
    const props = buildProps()
    const { getByTestId } = render(ShellRoute, props)

    const memberList = getByTestId('member-list')
    expect(memberList).toBeInTheDocument()
    expect(memberList.parentElement).toHaveClass('w-[240px]')
    expect(memberList.parentElement).toHaveClass('border-l')
  })

  it('shows mobile drill-down with bottom navigation', async () => {
    setViewport(600)
    const props = buildProps()
    const { getByRole, queryByRole } = render(ShellRoute, props)

    expect(
      getByRole('navigation', { name: 'Mobile shell navigation' }),
    ).toBeInTheDocument()
    expect(getByRole('heading', { name: 'Messages' })).toBeInTheDocument()

    await fireEvent.click(getByRole('button', { name: 'Members' }))
    expect(queryByRole('heading', { name: 'Messages' })).not.toBeInTheDocument()
    expect(getByRole('heading', { name: 'Members' })).toBeInTheDocument()
  })

  it('shows invite action only in channel mode', async () => {
    const props = buildProps()
    const view = render(ShellRoute, props)
    expect(
      view.getByRole('button', { name: 'Invite people' }),
    ).toBeInTheDocument()

    await view.rerender(
      buildProps({
        mode: 'settings',
        route: {
          result: {
            path: {
              condition: 'exact-match',
              original: '/settings',
              params: {},
            },
            querystring: {
              condition: 'exact-match',
              original: {},
              params: {},
            },
            status: 200,
          },
        },
      }),
    )

    expect(
      view.queryByRole('button', { name: 'Invite people' }),
    ).not.toBeInTheDocument()
  })

  it('renders GuildRail home button in channel mode', () => {
    const props = buildProps()
    const view = render(ShellRoute, props)
    expect(view.getByRole('button', { name: 'Home' })).toBeInTheDocument()
  })

  it('emits route path changes for persistence integration', async () => {
    const onRouteResolved = vi.fn()
    const props = buildProps({ onRouteResolved })
    const view = render(ShellRoute, props)

    await waitFor(() => {
      expect(onRouteResolved).toHaveBeenCalledWith('/lobby/general')
    })

    await view.rerender(
      buildProps({
        onRouteResolved,
        route: {
          result: {
            path: {
              condition: 'exact-match',
              original: '/engineering/announcements',
              params: { guild: 'engineering', channel: 'announcements' },
            },
            querystring: {
              condition: 'exact-match',
              original: {},
              params: {},
            },
            status: 200,
          },
        },
      }),
    )

    await waitFor(() => {
      expect(onRouteResolved).toHaveBeenCalledWith('/engineering/announcements')
    })
  })

  it('activates voice when route switches between voice channels', async () => {
    const voiceChannels = [
      {
        id: 'channel-voice-a',
        guildId: 'guild-1',
        slug: 'voice-a',
        name: 'voice-a',
        topic: null,
        kind: 'voice',
        position: 0,
        categorySlug: null,
        isDefault: true,
      },
      {
        id: 'channel-voice-b',
        guildId: 'guild-1',
        slug: 'voice-b',
        name: 'voice-b',
        topic: null,
        kind: 'voice',
        position: 1,
        categorySlug: null,
        isDefault: false,
      },
    ]
    channelState.channelsByGuild = { lobby: voiceChannels }
    channelState.channels = [...voiceChannels]

    const view = render(
      ShellRoute,
      buildProps({
        route: {
          result: {
            path: {
              condition: 'exact-match',
              original: '/lobby/voice-a',
              params: { guild: 'lobby', channel: 'voice-a' },
            },
            querystring: {
              condition: 'exact-match',
              original: {},
              params: {},
            },
            status: 200,
          },
        },
      }),
    )

    await waitFor(() => {
      expect(voiceState.activateVoiceChannel).toHaveBeenCalledWith(
        'lobby',
        'voice-a',
      )
    })
    voiceState.activateVoiceChannel.mockClear()
    voiceState.clearActiveChannel.mockClear()

    await view.rerender(
      buildProps({
        route: {
          result: {
            path: {
              condition: 'exact-match',
              original: '/lobby/voice-b',
              params: { guild: 'lobby', channel: 'voice-b' },
            },
            querystring: {
              condition: 'exact-match',
              original: {},
              params: {},
            },
            status: 200,
          },
        },
      }),
    )

    await waitFor(() => {
      expect(voiceState.activateVoiceChannel).toHaveBeenCalledWith(
        'lobby',
        'voice-b',
      )
    })
    expect(voiceState.clearActiveChannel).not.toHaveBeenCalled()
  })

  it('shows a non-blocking reconnecting status message while websocket reconnects', async () => {
    const props = buildProps()
    const view = render(ShellRoute, props)

    setWsLifecycleState('reconnecting')

    await waitFor(() => {
      expect(view.getByTestId('reconnecting-status')).toBeInTheDocument()
    })
    expect(view.getByText('Reconnecting...')).toBeInTheDocument()
  })

  it('jumps between unread channels using Alt+Shift+Arrow', async () => {
    const unreadSpy = vi
      .spyOn(messageState, 'unreadChannelSlugsForGuild')
      .mockReturnValue(['random', 'announcements'])
    const props = buildProps()
    render(ShellRoute, props)

    await fireEvent.keyDown(window, {
      key: 'ArrowDown',
      altKey: true,
      shiftKey: true,
    })

    await waitFor(() => {
      expect(routerGoto).toHaveBeenCalledWith('/lobby/random')
    })

    routerGoto.mockClear()
    await fireEvent.keyDown(window, {
      key: 'ArrowUp',
      altKey: true,
      shiftKey: true,
    })

    await waitFor(() => {
      expect(routerGoto).toHaveBeenCalledWith('/lobby/announcements')
    })

    unreadSpy.mockRestore()
  })

  it('supports Alt+Shift+Arrow unread navigation from editable fields', async () => {
    const unreadSpy = vi
      .spyOn(messageState, 'unreadChannelSlugsForGuild')
      .mockReturnValue(['random'])
    const props = buildProps()
    render(ShellRoute, props)

    const textarea = document.createElement('textarea')
    document.body.append(textarea)
    textarea.focus()
    try {
      await fireEvent.keyDown(textarea, {
        key: 'ArrowDown',
        altKey: true,
        shiftKey: true,
      })

      await waitFor(() => {
        expect(routerGoto).toHaveBeenCalledWith('/lobby/random')
      })
    } finally {
      textarea.remove()
      unreadSpy.mockRestore()
    }
  })

  it('handles global voice shortcuts and prevents default for Ctrl+D', async () => {
    const props = buildProps()
    render(ShellRoute, props)

    await fireEvent.keyDown(window, { key: 'm' })
    await fireEvent.keyDown(window, { key: 'd' })

    const ctrlDisconnect = new KeyboardEvent('keydown', {
      key: 'd',
      ctrlKey: true,
      bubbles: true,
      cancelable: true,
    })
    const notCanceled = window.dispatchEvent(ctrlDisconnect)

    expect(voiceState.toggleMute).toHaveBeenCalledTimes(1)
    expect(voiceState.toggleDeafen).toHaveBeenCalledTimes(1)
    expect(voiceState.disconnect).toHaveBeenCalledTimes(1)
    expect(notCanceled).toBe(false)
  })

  it('ignores voice shortcuts in editable fields', async () => {
    const props = buildProps()
    render(ShellRoute, props)

    const textarea = document.createElement('textarea')
    document.body.append(textarea)
    try {
      await fireEvent.keyDown(textarea, { key: 'm' })
      await fireEvent.keyDown(textarea, { key: 'd' })
      await fireEvent.keyDown(textarea, { key: 'd', ctrlKey: true })
      expect(voiceState.toggleMute).not.toHaveBeenCalled()
      expect(voiceState.toggleDeafen).not.toHaveBeenCalled()
      expect(voiceState.disconnect).not.toHaveBeenCalled()
    } finally {
      textarea.remove()
    }
  })

  it('opens DM route from member-list DM intent events', async () => {
    const props = buildProps()
    render(ShellRoute, props)

    window.dispatchEvent(
      new CustomEvent('discool:open-dm-intent', {
        detail: { guildSlug: 'lobby', userId: 'user-2' },
      }),
    )

    await waitFor(() => {
      expect(dmState.openOrCreateDm).toHaveBeenCalledWith('user-2')
      expect(routerGoto).toHaveBeenCalledWith('/dm/dm-1')
    })
  })

  it('ignores member-list DM intent events for blocked users', async () => {
    blockState.isBlocked.mockImplementation(
      (userId: string) => userId === 'user-2',
    )
    const props = buildProps()
    render(ShellRoute, props)

    window.dispatchEvent(
      new CustomEvent('discool:open-dm-intent', {
        detail: { guildSlug: 'lobby', userId: 'user-2' },
      }),
    )

    await waitFor(() => {
      expect(dmState.openOrCreateDm).not.toHaveBeenCalled()
      expect(routerGoto).not.toHaveBeenCalled()
    })
  })

  it('opens quick switcher with Ctrl+K and Cmd+K and autofocuses input', async () => {
    const props = buildProps()
    const view = render(ShellRoute, props)

    await fireEvent.keyDown(window, { key: 'k', ctrlKey: true })
    await waitFor(() => {
      expect(view.getByTestId('quick-switcher')).toBeInTheDocument()
    })
    expect(view.getByTestId('quick-switcher-overlay').className).toContain(
      'items-center',
    )
    const input = view.getByTestId('quick-switcher-input')
    await waitFor(() => {
      expect(input).toHaveFocus()
    })

    await fireEvent.keyDown(input, { key: 'Escape' })
    await waitFor(() => {
      expect(view.queryByTestId('quick-switcher')).not.toBeInTheDocument()
    })

    await fireEvent.keyDown(window, { key: 'k', metaKey: true })
    await waitFor(() => {
      expect(view.getByTestId('quick-switcher')).toBeInTheDocument()
    })
  })

  it('does not navigate when Enter is pressed on the quick-switcher close button', async () => {
    const props = buildProps()
    const view = render(ShellRoute, props)

    await fireEvent.keyDown(window, { key: 'k', ctrlKey: true })
    await waitFor(() => {
      expect(view.getByTestId('quick-switcher')).toBeInTheDocument()
    })

    const closeButton = view.getByRole('button', { name: 'Close' })
    closeButton.focus()
    await fireEvent.keyDown(closeButton, { key: 'Enter' })

    expect(routerGoto).not.toHaveBeenCalled()
  })

  it('shows grouped quick switcher results with recent-first defaults and DM navigation', async () => {
    guildState.guilds = [createLobbyGuild(), createEngineeringGuild()]
    channelState.channelsByGuild = {
      lobby: createLobbyChannels(),
      engineering: createEngineeringChannels(),
    }
    channelState.channels = [...channelState.channelsByGuild.lobby]
    dmState.conversations = [
      {
        dmSlug: 'dm-1',
        participant: {
          userId: 'user-2',
          username: 'bob',
          displayName: 'Bob',
          avatarColor: '#22aa88',
        },
        createdAt: '2026-02-28T00:00:00Z',
        updatedAt: '2026-02-28T00:00:00Z',
        lastMessagePreview: 'Hello',
        lastMessageAt: '2026-02-28T00:00:00Z',
        hasUnreadActivity: false,
      },
    ]
    const props = buildProps()
    const view = render(ShellRoute, props)

    await fireEvent.keyDown(window, { key: 'k', ctrlKey: true })
    await waitFor(() => {
      expect(view.getByTestId('quick-switcher')).toBeInTheDocument()
    })
    const resultsList = view.getByTestId('quick-switcher-results')
    expect(resultsList).toHaveTextContent('Channels')
    expect(resultsList).toHaveTextContent('DMs')
    expect(resultsList).toHaveTextContent('Guilds')
    expect(
      view.getByTestId('quick-switcher-result-channel:lobby:general'),
    ).toBeInTheDocument()
    expect(
      view.getByTestId('quick-switcher-result-dm:dm-1'),
    ).toBeInTheDocument()
    expect(
      view.getByTestId('quick-switcher-result-guild:lobby'),
    ).toBeInTheDocument()

    const options = Array.from(
      resultsList.querySelectorAll('button[role="option"]'),
    )
    expect(options[0]).toHaveAttribute(
      'data-testid',
      'quick-switcher-result-channel:lobby:general',
    )

    await fireEvent.click(view.getByTestId('quick-switcher-result-dm:dm-1'))
    await waitFor(() => {
      expect(routerGoto).toHaveBeenCalledWith('/dm/dm-1')
    })
  })

  it('uses deterministic fuzzy ranking with exact/prefix priority', async () => {
    dmState.conversations = [
      {
        dmSlug: 'dm-bobby',
        participant: {
          userId: 'user-3',
          username: 'bobby',
          displayName: 'Bobby',
          avatarColor: '#22aa88',
        },
        createdAt: '2026-02-28T00:00:00Z',
        updatedAt: '2026-02-28T00:00:00Z',
        lastMessagePreview: 'Hello',
        lastMessageAt: '2026-02-28T00:00:00Z',
        hasUnreadActivity: false,
      },
      {
        dmSlug: 'dm-bob',
        participant: {
          userId: 'user-2',
          username: 'bob',
          displayName: 'Bob',
          avatarColor: '#22aa88',
        },
        createdAt: '2026-02-28T00:00:00Z',
        updatedAt: '2026-02-28T00:00:00Z',
        lastMessagePreview: 'Hello',
        lastMessageAt: '2026-02-28T00:00:00Z',
        hasUnreadActivity: false,
      },
    ]
    const props = buildProps()
    const view = render(ShellRoute, props)

    await fireEvent.keyDown(window, { key: 'k', ctrlKey: true })
    const input = view.getByTestId('quick-switcher-input')
    await fireEvent.input(input, { target: { value: 'bob' } })

    await waitFor(() => {
      expect(
        view.getByTestId('quick-switcher-result-dm:dm-bobby'),
      ).toBeInTheDocument()
      expect(
        view.getByTestId('quick-switcher-result-dm:dm-bob'),
      ).toBeInTheDocument()
    })

    const dmOptions = Array.from(
      view
        .getByTestId('quick-switcher-results')
        .querySelectorAll('button[data-testid^="quick-switcher-result-dm:"]'),
    )
    expect(dmOptions[0]).toHaveAttribute(
      'data-testid',
      'quick-switcher-result-dm:dm-bob',
    )
  })

  it('supports arrow-key navigation, Enter activation, and Escape close', async () => {
    const props = buildProps()
    const view = render(ShellRoute, props)

    await fireEvent.keyDown(window, { key: 'k', ctrlKey: true })
    const input = view.getByTestId('quick-switcher-input')

    await fireEvent.keyDown(input, { key: 'ArrowDown' })
    await fireEvent.keyDown(input, { key: 'Enter' })
    await waitFor(() => {
      expect(routerGoto).toHaveBeenCalledWith('/lobby/random')
    })

    await fireEvent.keyDown(window, { key: 'k', ctrlKey: true })
    await waitFor(() => {
      expect(view.getByTestId('quick-switcher')).toBeInTheDocument()
    })
    await fireEvent.keyDown(view.getByTestId('quick-switcher-input'), {
      key: 'Escape',
    })
    await waitFor(() => {
      expect(view.queryByTestId('quick-switcher')).not.toBeInTheDocument()
    })
  })

  it('traps focus while open and restores focus to trigger on close', async () => {
    const trigger = document.createElement('button')
    trigger.type = 'button'
    trigger.textContent = 'Trigger'
    document.body.append(trigger)
    trigger.focus()
    const props = buildProps()
    const view = render(ShellRoute, props)

    try {
      await fireEvent.keyDown(window, { key: 'k', ctrlKey: true })
      const input = view.getByTestId('quick-switcher-input')
      await waitFor(() => {
        expect(input).toHaveFocus()
      })

      await fireEvent.keyDown(input, { key: 'Tab', shiftKey: true })
      await waitFor(() => {
        const dialog = view.getByTestId('quick-switcher')
        expect(dialog.contains(document.activeElement)).toBe(true)
      })

      await fireEvent.click(view.getByRole('button', { name: 'Close' }))
      await waitFor(() => {
        expect(view.queryByTestId('quick-switcher')).not.toBeInTheDocument()
      })
      expect(trigger).toHaveFocus()
    } finally {
      trigger.remove()
    }
  })

  it('hydrates channels for non-active guilds when quick switcher opens', async () => {
    guildState.guilds = [createLobbyGuild(), createEngineeringGuild()]
    channelState.channelsByGuild = {
      lobby: createLobbyChannels(),
    }
    channelState.channels = [...channelState.channelsByGuild.lobby]
    channelState.activeGuild = 'lobby'
    channelState.loadChannels.mockImplementation(async (guildSlug: string) => {
      channelState.activeGuild = guildSlug
      if (guildSlug === 'engineering') {
        channelState.channelsByGuild.engineering = createEngineeringChannels()
      }
      const nextChannels = channelState.channelsByGuild[guildSlug] ?? []
      channelState.channels = nextChannels
      return nextChannels
    })
    const props = buildProps()
    const view = render(ShellRoute, props)

    await fireEvent.keyDown(window, { key: 'k', ctrlKey: true })
    await waitFor(() => {
      expect(channelState.loadChannels).toHaveBeenCalledWith('engineering')
    })
    await waitFor(() => {
      expect(channelState.loadChannels).toHaveBeenCalledWith('lobby')
    })
    await waitFor(() => {
      expect(
        view.getByTestId(
          'quick-switcher-result-channel:engineering:announcements',
        ),
      ).toBeInTheDocument()
    })
  })
})
