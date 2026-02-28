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
        reactions: Array<{
          emoji: string
          count: number
          reacted: boolean
        }>
      }>
      cursor: string | null
    }>,
  }

  const fetchChannelHistory = vi.fn(async () => {
    return state.responses.shift() ?? { messages: [], cursor: null }
  })

  return { state, fetchChannelHistory }
})

vi.mock('$lib/ws/client', () => ({
  wsClient: wsMock.wsClient,
}))

vi.mock('./messageApi', () => ({
  fetchChannelHistory: historyApiMock.fetchChannelHistory,
}))

import { messageState } from './messageStore.svelte'

function makeMessage(id: string, createdAt: string) {
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
    reactions: [],
  }
}

describe('messageState', () => {
  beforeEach(() => {
    messageState.clearAll()
    wsMock.state.sendResult = true
    wsMock.wsClient.send.mockClear()
    historyApiMock.fetchChannelHistory.mockClear()
    historyApiMock.state.responses = []
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
      },
    })

    const reconciled = messageState.timeline('lobby', 'general')
    expect(reconciled).toHaveLength(1)
    expect(reconciled[0]?.id).toBe('message-1')
    expect(reconciled[0]?.optimistic).toBe(false)
    expect(reconciled[0]?.content).toBe('Hello &lt;b&gt;team&lt;/b&gt;')
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
})
