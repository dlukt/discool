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

vi.mock('$lib/ws/client', () => ({
  wsClient: wsMock.wsClient,
}))

import { messageState } from './messageStore.svelte'

describe('messageState', () => {
  beforeEach(() => {
    messageState.clearAll()
    wsMock.state.sendResult = true
    wsMock.wsClient.send.mockClear()
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
        client_nonce: payload.client_nonce,
      },
    })

    const reconciled = messageState.timeline('lobby', 'general')
    expect(reconciled).toHaveLength(1)
    expect(reconciled[0]?.id).toBe('message-1')
    expect(reconciled[0]?.optimistic).toBe(false)
    expect(reconciled[0]?.content).toBe('Hello &lt;b&gt;team&lt;/b&gt;')
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
})
