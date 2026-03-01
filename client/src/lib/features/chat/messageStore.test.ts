import { beforeEach, describe, expect, it, vi } from 'vitest'

const wsMock = vi.hoisted(() => {
  const state = {
    sendResult: true,
    listener: null as ((envelope: unknown) => void) | null,
  }

  const wsClient = {
    send: vi.fn((_: string, __?: Record<string, unknown>) => state.sendResult),
    subscribe: vi.fn((listener: (envelope: unknown) => void) => {
      state.listener = listener
      return () => {
        if (state.listener === listener) {
          state.listener = null
        }
      }
    }),
  }

  return { state, wsClient }
})

const historyApiMock = vi.hoisted(() => {
  const state = {
    responses: [] as Array<{
      messages: Array<{
        id: string
        guildSlug: string
        channelSlug: string
        authorUserId: string
        authorUsername: string
        authorDisplayName: string
        authorAvatarColor: string | null
        authorRoleColor: string
        content: string
        isSystem: boolean
        createdAt: string
        updatedAt: string
        optimistic: boolean
        clientNonce?: string
        attachments: Array<{
          id: string
          storageKey: string
          originalFilename: string
          mimeType: string
          sizeBytes: number
          isImage: boolean
          url: string
        }>
        reactions: Array<{
          emoji: string
          count: number
          reacted: boolean
        }>
        embeds: Array<{
          id: string
          url: string
          domain: string
          title: string | null
          description: string | null
          thumbnailUrl: string | null
        }>
      }>
      cursor: string | null
    }>,
    dmResponses: [] as Array<{
      messages: Array<{
        id: string
        guildSlug: string
        channelSlug: string
        dmSlug?: string | null
        authorUserId: string
        authorUsername: string
        authorDisplayName: string
        authorAvatarColor: string | null
        authorRoleColor: string
        content: string
        isSystem: boolean
        createdAt: string
        updatedAt: string
        optimistic: boolean
        clientNonce?: string
        attachments: Array<{
          id: string
          storageKey: string
          originalFilename: string
          mimeType: string
          sizeBytes: number
          isImage: boolean
          url: string
        }>
        reactions: Array<{
          emoji: string
          count: number
          reacted: boolean
        }>
        embeds: Array<{
          id: string
          url: string
          domain: string
          title: string | null
          description: string | null
          thumbnailUrl: string | null
        }>
      }>
      cursor: string | null
    }>,
    uploadResult: null as {
      id: string
      guildSlug: string
      channelSlug: string
      authorUserId: string
      authorUsername: string
      authorDisplayName: string
      authorAvatarColor: string | null
      authorRoleColor: string
      content: string
      isSystem: boolean
      createdAt: string
      updatedAt: string
      optimistic: boolean
      clientNonce?: string
      attachments: Array<{
        id: string
        storageKey: string
        originalFilename: string
        mimeType: string
        sizeBytes: number
        isImage: boolean
        url: string
      }>
      reactions: Array<{
        emoji: string
        count: number
        reacted: boolean
      }>
      embeds: Array<{
        id: string
        url: string
        domain: string
        title: string | null
        description: string | null
        thumbnailUrl: string | null
      }>
    } | null,
  }

  const fetchChannelHistory = vi.fn(async () => {
    return state.responses.shift() ?? { messages: [], cursor: null }
  })
  const fetchDmHistory = vi.fn(async () => {
    return state.dmResponses.shift() ?? { messages: [], cursor: null }
  })
  const uploadMessageAttachment = vi.fn(async () => {
    if (!state.uploadResult) {
      throw new Error('uploadResult not configured')
    }
    return state.uploadResult
  })

  return { state, fetchChannelHistory, fetchDmHistory, uploadMessageAttachment }
})

const channelStoreMock = vi.hoisted(() => {
  type ChannelStub = { slug: string; hasUnreadActivity?: boolean }
  const state = {
    byGuild: {
      lobby: [{ slug: 'general' }, { slug: 'random' }] as ChannelStub[],
    } as Record<string, ChannelStub[]>,
  }

  const channelState = {
    setChannelUnreadActivity: vi.fn(
      (guildSlug: string, channelSlug: string, hasUnreadActivity: boolean) => {
        const channels = state.byGuild[guildSlug] ?? []
        const index = channels.findIndex(
          (channel) => channel.slug === channelSlug,
        )
        if (index < 0) {
          state.byGuild[guildSlug] = [
            ...channels,
            { slug: channelSlug, hasUnreadActivity },
          ]
          return
        }
        const next = [...channels]
        next[index] = { ...next[index], hasUnreadActivity }
        state.byGuild[guildSlug] = next
      },
    ),
    hasGuildUnreadActivity: vi.fn((guildSlug: string) => {
      return (state.byGuild[guildSlug] ?? []).some(
        (channel) => channel.hasUnreadActivity === true,
      )
    }),
    orderedChannelsForGuild: vi.fn((guildSlug: string) => {
      return [...(state.byGuild[guildSlug] ?? [])]
    }),
  }

  return { state, channelState }
})

const guildStoreMock = vi.hoisted(() => {
  const state = {
    unreadByGuild: {} as Record<string, boolean>,
  }
  const guildState = {
    setGuildUnreadActivity: vi.fn(
      (guildSlug: string, hasUnreadActivity: boolean) => {
        state.unreadByGuild[guildSlug] = hasUnreadActivity
      },
    ),
  }
  return { state, guildState }
})

const dmStoreMock = vi.hoisted(() => {
  const dmState = {
    setDmUnreadActivity: vi.fn(),
    noteMessageActivity: vi.fn(),
    setActiveDm: vi.fn(),
  }
  return { dmState }
})

const blockStoreMock = vi.hoisted(() => {
  const state = {
    blockedUsers: new Set<string>(),
    hiddenByWindow: new Set<string>(),
  }
  const blockState = {
    version: 0,
    isBlocked: vi.fn((userId: string) => state.blockedUsers.has(userId)),
    isHiddenByBlockWindow: vi.fn((userId: string, activityAt: string) =>
      state.hiddenByWindow.has(`${userId}|${activityAt}`),
    ),
  }
  return { state, blockState }
})

vi.mock('$lib/ws/client', () => ({
  wsClient: wsMock.wsClient,
}))

vi.mock('./messageApi', () => ({
  fetchChannelHistory: historyApiMock.fetchChannelHistory,
  uploadMessageAttachment: historyApiMock.uploadMessageAttachment,
}))

vi.mock('$lib/features/dm/dmApi', () => ({
  fetchDmHistory: historyApiMock.fetchDmHistory,
}))

vi.mock('$lib/features/channel/channelStore.svelte', () => ({
  channelState: channelStoreMock.channelState,
}))

vi.mock('$lib/features/guild/guildStore.svelte', () => ({
  guildState: guildStoreMock.guildState,
}))

vi.mock('$lib/features/dm/dmStore.svelte', () => ({
  dmState: dmStoreMock.dmState,
}))

vi.mock('$lib/features/identity/blockStore.svelte', () => ({
  blockState: blockStoreMock.blockState,
}))

import { messageState } from './messageStore.svelte'

function makeMessage(
  id: string,
  createdAt: string,
  overrides: Partial<ReturnType<typeof makeMessageBase>> = {},
) {
  return {
    ...makeMessageBase(id, createdAt),
    ...overrides,
  }
}

function makeMessageBase(id: string, createdAt: string) {
  return {
    id,
    guildSlug: 'lobby',
    channelSlug: 'general',
    authorUserId: 'user-1',
    authorUsername: 'alice',
    authorDisplayName: 'Alice',
    authorAvatarColor: '#3366ff',
    authorRoleColor: '#3366ff',
    content: id,
    isSystem: false,
    createdAt,
    updatedAt: createdAt,
    optimistic: false,
    attachments: [],
    reactions: [],
    embeds: [],
  }
}

function makeDmMessage(id: string, createdAt: string, dmSlug = 'dm-1') {
  return {
    ...makeMessage(id, createdAt),
    guildSlug: '',
    channelSlug: '',
    dmSlug,
  }
}

describe('messageState', () => {
  beforeEach(() => {
    messageState.clearAll()
    wsMock.state.sendResult = true
    wsMock.wsClient.send.mockClear()
    historyApiMock.fetchChannelHistory.mockClear()
    historyApiMock.fetchDmHistory.mockClear()
    historyApiMock.uploadMessageAttachment.mockClear()
    historyApiMock.state.responses = []
    historyApiMock.state.dmResponses = []
    historyApiMock.state.uploadResult = null
    channelStoreMock.state.byGuild = {
      lobby: [{ slug: 'general' }, { slug: 'random' }],
    }
    channelStoreMock.channelState.setChannelUnreadActivity.mockClear()
    channelStoreMock.channelState.hasGuildUnreadActivity.mockClear()
    channelStoreMock.channelState.orderedChannelsForGuild.mockClear()
    guildStoreMock.state.unreadByGuild = {}
    guildStoreMock.guildState.setGuildUnreadActivity.mockClear()
    dmStoreMock.dmState.setDmUnreadActivity.mockClear()
    dmStoreMock.dmState.noteMessageActivity.mockClear()
    dmStoreMock.dmState.setActiveDm.mockClear()
    blockStoreMock.state.blockedUsers.clear()
    blockStoreMock.state.hiddenByWindow.clear()
    blockStoreMock.blockState.version = 0
    blockStoreMock.blockState.isBlocked.mockClear()
    blockStoreMock.blockState.isHiddenByBlockWindow.mockClear()
  })

  it('creates optimistic message and reconciles when message_create arrives', () => {
    const sent = messageState.sendMessage(
      'lobby',
      'general',
      'Hello <b>team</b>',
      {
        userId: 'user-1',
        username: 'alice',
        displayName: 'Alice',
        avatarColor: '#3366ff',
        roleColor: '#3366ff',
      },
    )

    expect(sent).toBe(true)
    expect(wsMock.wsClient.send).toHaveBeenCalledTimes(1)
    const firstSendCall = wsMock.wsClient.send.mock.calls[0]
    expect(firstSendCall?.[0]).toBe('c_message_create')

    const firstTimeline = messageState.timeline('lobby', 'general')
    expect(firstTimeline).toHaveLength(1)
    expect(firstTimeline[0]?.optimistic).toBe(true)

    const payload = firstSendCall?.[1] as {
      client_nonce: string
    }
    wsMock.state.listener?.({
      op: 'message_create',
      d: {
        id: 'message-1',
        guild_slug: 'lobby',
        channel_slug: 'general',
        author_user_id: 'user-1',
        author_username: 'alice',
        author_display_name: 'Alice',
        author_avatar_color: '#3366ff',
        author_role_color: '#3366ff',
        content: 'Hello &lt;b&gt;team&lt;/b&gt;',
        is_system: false,
        created_at: '2026-02-28T00:00:00Z',
        updated_at: '2026-02-28T00:00:00Z',
        client_nonce: payload.client_nonce,
        attachments: [],
        embeds: [
          {
            id: 'embed-1',
            url: 'https://example.com/post',
            domain: 'example.com',
            title: 'Example',
            description: 'Embed description',
            thumbnail_url: 'https://example.com/thumb.png',
          },
        ],
      },
    })

    const reconciled = messageState.timeline('lobby', 'general')
    expect(reconciled).toHaveLength(1)
    expect(reconciled[0]?.id).toBe('message-1')
    expect(reconciled[0]?.optimistic).toBe(false)
    expect(reconciled[0]?.content).toBe('Hello &lt;b&gt;team&lt;/b&gt;')
    expect(reconciled[0]?.embeds).toEqual([
      {
        id: 'embed-1',
        url: 'https://example.com/post',
        domain: 'example.com',
        title: 'Example',
        description: 'Embed description',
        thumbnailUrl: 'https://example.com/thumb.png',
      },
    ])
  })

  it('loads initial and older message history pages with cursor state', async () => {
    historyApiMock.state.responses = [
      {
        messages: [
          makeMessage('msg-003', '2026-02-28T00:00:02Z'),
          makeMessage('msg-004', '2026-02-28T00:00:03Z'),
        ],
        cursor: 'cursor-1',
      },
      {
        messages: [
          makeMessage('msg-001', '2026-02-28T00:00:00Z'),
          makeMessage('msg-002', '2026-02-28T00:00:01Z'),
        ],
        cursor: null,
      },
    ]

    await messageState.ensureHistoryLoaded('lobby', 'general')

    expect(historyApiMock.fetchChannelHistory).toHaveBeenCalledWith(
      'lobby',
      'general',
      {
        limit: 50,
      },
    )
    expect(
      messageState.timeline('lobby', 'general').map((message) => message.id),
    ).toEqual(['msg-003', 'msg-004'])
    expect(messageState.historyStateForChannel('lobby', 'general').cursor).toBe(
      'cursor-1',
    )
    expect(
      messageState.historyStateForChannel('lobby', 'general').hasMoreHistory,
    ).toBe(true)

    await messageState.loadOlderHistory('lobby', 'general')

    expect(historyApiMock.fetchChannelHistory).toHaveBeenLastCalledWith(
      'lobby',
      'general',
      {
        limit: 50,
        before: 'cursor-1',
      },
    )
    expect(
      messageState.timeline('lobby', 'general').map((message) => message.id),
    ).toEqual(['msg-001', 'msg-002', 'msg-003', 'msg-004'])
    expect(
      messageState.historyStateForChannel('lobby', 'general').hasMoreHistory,
    ).toBe(false)
  })

  it('tracks scroll restoration and pending-new counters per channel', () => {
    messageState.setScrollTop('lobby', 'general', 187.6)
    expect(messageState.scrollTopForChannel('lobby', 'general')).toBe(188)

    messageState.addPendingNew('lobby', 'general', 2)
    expect(messageState.pendingNewCountForChannel('lobby', 'general')).toBe(2)

    messageState.clearPendingNew('lobby', 'general')
    expect(messageState.pendingNewCountForChannel('lobby', 'general')).toBe(0)
  })

  it('keeps older pages visible when active timeline hits memory cap', async () => {
    const channelKey = 'lobby:general'
    messageState.setActiveChannel('lobby', 'general')
    messageState.messagesByChannel[channelKey] = Array.from(
      { length: 4_000 },
      (_, index) =>
        makeMessage(
          `seed-${index}`,
          `t-${String(index + 1_000).padStart(5, '0')}`,
        ),
    )
    messageState.version += 1

    historyApiMock.state.responses = [
      { messages: [], cursor: 'cursor-older' },
      {
        messages: [makeMessage('older-1', 't-00999')],
        cursor: 'cursor-next',
      },
    ]

    await messageState.ensureHistoryLoaded('lobby', 'general')
    await messageState.loadOlderHistory('lobby', 'general')

    const ids = messageState
      .timeline('lobby', 'general')
      .map((message) => message.id)
    expect(ids).toHaveLength(4_000)
    expect(ids[0]).toBe('older-1')
    expect(ids).not.toContain('seed-3999')
    expect(messageState.historyStateForChannel('lobby', 'general').cursor).toBe(
      'cursor-next',
    )
  })

  it('removes optimistic message immediately when websocket send fails', () => {
    wsMock.state.sendResult = false

    const sent = messageState.sendMessage('lobby', 'general', 'Will fail', {
      userId: 'user-2',
      username: 'bob',
      displayName: 'Bob',
      avatarColor: '#22aa88',
      roleColor: '#22aa88',
    })

    expect(sent).toBe(false)
    expect(messageState.timeline('lobby', 'general')).toHaveLength(0)
  })

  it('sends typing_start websocket op for active composer state', () => {
    const sent = messageState.sendTypingStart('lobby', 'general')
    expect(sent).toBe(true)
    expect(wsMock.wsClient.send).toHaveBeenCalledWith('c_typing_start', {
      guild_slug: 'lobby',
      channel_slug: 'general',
    })
  })

  it('ingests typing_start events and expires indicators after timeout', () => {
    vi.useFakeTimers()
    try {
      messageState.setCurrentUser('user-1')
      wsMock.state.listener?.({
        op: 'typing_start',
        d: {
          guild_slug: 'lobby',
          channel_slug: 'general',
          user_id: 'user-2',
        },
      })

      expect(messageState.typingUserIdsForChannel('lobby', 'general')).toEqual([
        'user-2',
      ])

      vi.advanceTimersByTime(5_001)
      expect(messageState.typingUserIdsForChannel('lobby', 'general')).toEqual(
        [],
      )
    } finally {
      vi.useRealTimers()
    }
  })

  it('ignores typing_start events from blocked users', () => {
    messageState.setCurrentUser('user-1')
    blockStoreMock.state.blockedUsers.add('user-2')
    blockStoreMock.blockState.version += 1

    wsMock.state.listener?.({
      op: 'typing_start',
      d: {
        guild_slug: 'lobby',
        channel_slug: 'general',
        user_id: 'user-2',
      },
    })

    expect(messageState.typingUserIdsForChannel('lobby', 'general')).toEqual([])
  })

  it('tracks channel_activity unread state and clears on active channel switch', () => {
    messageState.setActiveChannel('lobby', 'general')

    wsMock.state.listener?.({
      op: 'channel_activity',
      d: {
        guild_slug: 'lobby',
        channel_slug: 'random',
      },
    })

    expect(messageState.isChannelUnread('lobby', 'random')).toBe(true)
    expect(messageState.unreadChannelSlugsForGuild('lobby')).toEqual(['random'])
    expect(
      guildStoreMock.guildState.setGuildUnreadActivity,
    ).toHaveBeenCalledWith('lobby', true)

    messageState.setActiveChannel('lobby', 'random')
    expect(messageState.isChannelUnread('lobby', 'random')).toBe(false)
  })

  it('ignores channel_activity from blocked actors', () => {
    blockStoreMock.state.blockedUsers.add('user-2')
    blockStoreMock.blockState.version += 1
    messageState.setActiveChannel('lobby', 'general')

    wsMock.state.listener?.({
      op: 'channel_activity',
      d: {
        guild_slug: 'lobby',
        channel_slug: 'random',
        actor_user_id: 'user-2',
      },
    })

    expect(messageState.isChannelUnread('lobby', 'random')).toBe(false)
  })

  it('uploads attachment via REST API and appends returned message', async () => {
    historyApiMock.state.uploadResult = {
      ...makeMessage('msg-upload', '2026-02-28T00:00:10Z'),
      content: 'file upload',
      attachments: [
        {
          id: 'att-1',
          storageKey: 'attachment-1.png',
          originalFilename: 'image.png',
          mimeType: 'image/png',
          sizeBytes: 1234,
          isImage: true,
          url: '/api/v1/guilds/lobby/channels/general/messages/attachments/att-1',
        },
      ],
    }
    const onProgress = vi.fn()
    const file = new File(['hello'], 'image.png', { type: 'image/png' })

    await messageState.uploadAttachment('lobby', 'general', {
      file,
      content: 'file upload',
      onProgress,
    })

    expect(historyApiMock.uploadMessageAttachment).toHaveBeenCalledWith(
      'lobby',
      'general',
      expect.objectContaining({
        file,
        content: 'file upload',
        clientNonce: expect.any(String),
      }),
    )
    const timeline = messageState.timeline('lobby', 'general')
    expect(timeline).toHaveLength(1)
    expect(timeline[0]?.attachments).toEqual(
      expect.arrayContaining([
        expect.objectContaining({ id: 'att-1', isImage: true }),
      ]),
    )
  })

  it('ingests message_update and message_delete without breaking ordering', () => {
    messageState.ingestServerMessage(
      makeMessage('msg-001', '2026-02-28T00:00:01Z'),
    )
    messageState.ingestServerMessage(
      makeMessage('msg-002', '2026-02-28T00:00:02Z'),
    )

    wsMock.state.listener?.({
      op: 'message_update',
      d: {
        id: 'msg-001',
        guild_slug: 'lobby',
        channel_slug: 'general',
        author_user_id: 'user-1',
        author_username: 'alice',
        author_display_name: 'Alice',
        author_avatar_color: '#3366ff',
        author_role_color: '#3366ff',
        content: 'edited',
        is_system: false,
        created_at: '2026-02-28T00:00:01Z',
        updated_at: '2026-02-28T00:00:05Z',
        attachments: [],
      },
    })

    let timeline = messageState.timeline('lobby', 'general')
    expect(timeline.map((message) => message.id)).toEqual([
      'msg-001',
      'msg-002',
    ])
    expect(timeline[0]?.content).toBe('edited')
    expect(timeline[0]?.updatedAt).toBe('2026-02-28T00:00:05Z')

    wsMock.state.listener?.({
      op: 'message_delete',
      d: {
        id: 'msg-002',
        guild_slug: 'lobby',
        channel_slug: 'general',
      },
    })

    timeline = messageState.timeline('lobby', 'general')
    expect(timeline.map((message) => message.id)).toEqual(['msg-001'])
  })

  it('filters timeline messages for active and historical blocks', () => {
    messageState.ingestServerMessage(
      makeMessage('msg-blocked', '2026-02-28T00:00:01Z', {
        authorUserId: 'user-2',
      }),
    )
    messageState.ingestServerMessage(
      makeMessage('msg-window', '2026-02-28T00:00:02Z', {
        authorUserId: 'user-3',
      }),
    )
    messageState.ingestServerMessage(
      makeMessage('msg-visible', '2026-02-28T00:00:03Z', {
        authorUserId: 'user-4',
      }),
    )

    blockStoreMock.state.blockedUsers.add('user-2')
    blockStoreMock.state.hiddenByWindow.add('user-3|2026-02-28T00:00:02Z')
    blockStoreMock.blockState.version += 1

    const timeline = messageState.timeline('lobby', 'general')
    expect(timeline.map((message) => message.id)).toEqual(['msg-visible'])
  })

  it('sends reaction toggle op and ingests message_reaction_update snapshots', () => {
    messageState.setCurrentUser('user-1')
    messageState.ingestServerMessage(
      makeMessage('msg-010', '2026-02-28T00:00:01Z'),
    )

    const sent = messageState.sendMessageReactionToggle(
      'lobby',
      'general',
      'msg-010',
      '🎉',
    )
    expect(sent).toBe(true)
    expect(wsMock.wsClient.send).toHaveBeenCalledWith(
      'c_message_reaction_toggle',
      expect.objectContaining({
        guild_slug: 'lobby',
        channel_slug: 'general',
        message_id: 'msg-010',
        emoji: '🎉',
      }),
    )

    wsMock.state.listener?.({
      op: 'message_reaction_update',
      d: {
        guild_slug: 'lobby',
        channel_slug: 'general',
        message_id: 'msg-010',
        actor_user_id: 'user-1',
        reactions: [{ emoji: '🎉', count: 1, reacted: true }],
      },
    })

    let timeline = messageState.timeline('lobby', 'general')
    expect(timeline[0]?.reactions).toEqual([
      { emoji: '🎉', count: 1, reacted: true },
    ])

    wsMock.state.listener?.({
      op: 'message_reaction_update',
      d: {
        guild_slug: 'lobby',
        channel_slug: 'general',
        message_id: 'msg-010',
        actor_user_id: 'user-2',
        reactions: [{ emoji: '🎉', count: 2, reacted: true }],
      },
    })
    timeline = messageState.timeline('lobby', 'general')
    expect(timeline[0]?.reactions).toEqual([
      { emoji: '🎉', count: 2, reacted: true },
    ])

    messageState.setCurrentUser('user-3')
    wsMock.state.listener?.({
      op: 'message_reaction_update',
      d: {
        guild_slug: 'lobby',
        channel_slug: 'general',
        message_id: 'msg-010',
        actor_user_id: 'user-4',
        reactions: [{ emoji: '🎉', count: 3, reacted: true }],
      },
    })
    timeline = messageState.timeline('lobby', 'general')
    expect(timeline[0]?.reactions).toEqual([
      { emoji: '🎉', count: 3, reacted: true },
    ])
  })

  it('filters blocked reaction actors from message_reaction_update snapshots', () => {
    messageState.setCurrentUser('user-1')
    blockStoreMock.state.blockedUsers.add('user-2')
    blockStoreMock.blockState.version += 1
    messageState.ingestServerMessage(
      makeMessage('msg-020', '2026-02-28T00:00:01Z'),
    )

    wsMock.state.listener?.({
      op: 'message_reaction_update',
      d: {
        guild_slug: 'lobby',
        channel_slug: 'general',
        message_id: 'msg-020',
        actor_user_id: 'user-3',
        reactions: [
          {
            emoji: '🎉',
            count: 2,
            reacted: true,
            actors: [
              { user_id: 'user-1', created_at: '2026-02-28T00:00:02Z' },
              { user_id: 'user-2', created_at: '2026-02-28T00:00:03Z' },
            ],
          },
        ],
      },
    })

    const timeline = messageState.timeline('lobby', 'general')
    expect(timeline[0]?.reactions).toEqual([
      {
        emoji: '🎉',
        count: 1,
        reacted: false,
        actors: [{ userId: 'user-1', createdAt: '2026-02-28T00:00:02Z' }],
      },
    ])
  })

  it('creates optimistic DM message and reconciles on dm_message_create', () => {
    messageState.setCurrentUser('user-1')
    messageState.setActiveDm('dm-1')

    const sent = messageState.sendDmMessage('dm-1', 'hello dm', {
      userId: 'user-1',
      username: 'alice',
      displayName: 'Alice',
      avatarColor: '#3366ff',
      roleColor: '#3366ff',
    })

    expect(sent).toBe(true)
    expect(wsMock.wsClient.send).toHaveBeenCalledWith(
      'c_dm_message_create',
      expect.objectContaining({
        dm_slug: 'dm-1',
        content: 'hello dm',
      }),
    )
    expect(messageState.timelineForDm('dm-1')).toHaveLength(1)
    expect(messageState.timelineForDm('dm-1')[0]?.optimistic).toBe(true)

    const payload = wsMock.wsClient.send.mock.calls.find(
      (call) => call[0] === 'c_dm_message_create',
    )?.[1] as { client_nonce: string }

    wsMock.state.listener?.({
      op: 'dm_message_create',
      d: {
        id: 'dm-msg-1',
        dm_slug: 'dm-1',
        author_user_id: 'user-1',
        author_username: 'alice',
        author_display_name: 'Alice',
        content: 'hello dm',
        is_system: false,
        created_at: '2026-02-28T00:00:00Z',
        updated_at: '2026-02-28T00:00:00Z',
        client_nonce: payload.client_nonce,
      },
    })

    const reconciled = messageState.timelineForDm('dm-1')
    expect(reconciled).toHaveLength(1)
    expect(reconciled[0]?.id).toBe('dm-msg-1')
    expect(reconciled[0]?.optimistic).toBe(false)
    expect(dmStoreMock.dmState.noteMessageActivity).toHaveBeenCalled()
  })

  it('loads DM history pages with cursor state', async () => {
    historyApiMock.state.dmResponses = [
      {
        messages: [makeDmMessage('dm-msg-2', '2026-02-28T00:00:02Z')],
        cursor: 'dm-cursor-1',
      },
      {
        messages: [makeDmMessage('dm-msg-1', '2026-02-28T00:00:01Z')],
        cursor: null,
      },
    ]

    await messageState.ensureDmHistoryLoaded('dm-1')
    expect(historyApiMock.fetchDmHistory).toHaveBeenCalledWith('dm-1', {
      limit: 50,
    })
    expect(
      messageState.timelineForDm('dm-1').map((message) => message.id),
    ).toEqual(['dm-msg-2'])
    expect(messageState.historyStateForDm('dm-1').cursor).toBe('dm-cursor-1')

    await messageState.loadOlderDmHistory('dm-1')
    expect(historyApiMock.fetchDmHistory).toHaveBeenLastCalledWith('dm-1', {
      limit: 50,
      before: 'dm-cursor-1',
    })
    expect(
      messageState.timelineForDm('dm-1').map((message) => message.id),
    ).toEqual(['dm-msg-1', 'dm-msg-2'])
    expect(messageState.historyStateForDm('dm-1').hasMoreHistory).toBe(false)
  })

  it('marks DM unread activity only for inactive DM conversations', () => {
    messageState.setActiveDm('dm-1')

    wsMock.state.listener?.({
      op: 'dm_activity',
      d: {
        dm_slug: 'dm-1',
      },
    })
    expect(dmStoreMock.dmState.setDmUnreadActivity).not.toHaveBeenCalled()

    wsMock.state.listener?.({
      op: 'dm_activity',
      d: {
        dm_slug: 'dm-2',
      },
    })
    expect(dmStoreMock.dmState.setDmUnreadActivity).toHaveBeenCalledWith(
      'dm-2',
      true,
    )
  })

  it('ignores dm_activity from blocked actors', () => {
    blockStoreMock.state.blockedUsers.add('user-2')
    blockStoreMock.blockState.version += 1

    wsMock.state.listener?.({
      op: 'dm_activity',
      d: {
        dm_slug: 'dm-2',
        actor_user_id: 'user-2',
      },
    })

    expect(dmStoreMock.dmState.setDmUnreadActivity).not.toHaveBeenCalled()
  })
})
