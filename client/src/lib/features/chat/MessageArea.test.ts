import { fireEvent, render, waitFor } from '@testing-library/svelte'
import { beforeEach, describe, expect, it, vi } from 'vitest'

const {
  wsLifecycle,
  lifecycleListeners,
  timelineState,
  messageState,
  identityState,
  guildState,
} = vi.hoisted(() => {
  const wsLifecycle = { value: 'connected' as const }
  const lifecycleListeners = new Set<
    (state: typeof wsLifecycle.value) => void
  >()
  const timelineState = {
    messages: [] as Array<{
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
      optimistic: boolean
      clientNonce?: string
    }>,
  }

  const messageState = {
    version: 1,
    timeline: vi.fn(() => timelineState.messages),
    sendMessage: vi.fn(() => true),
  }

  const identityState = {
    session: {
      user: {
        id: 'user-1',
        username: 'alice',
        displayName: 'Alice',
        avatarColor: '#3366ff' as string | null,
      },
    },
  }

  const guildState = {
    memberByUserId: vi.fn(() => ({ highestRoleColor: '#3366ff' })),
  }

  return {
    wsLifecycle,
    lifecycleListeners,
    timelineState,
    messageState,
    identityState,
    guildState,
  }
})

vi.mock('$lib/ws/client', () => ({
  wsClient: {
    getLifecycleState: vi.fn(() => wsLifecycle.value),
    subscribeLifecycle: vi.fn(
      (listener: (state: typeof wsLifecycle.value) => void) => {
        lifecycleListeners.add(listener)
        listener(wsLifecycle.value)
        return () => lifecycleListeners.delete(listener)
      },
    ),
  },
}))

vi.mock('$lib/features/identity/identityStore.svelte', () => ({
  identityState,
}))

vi.mock('$lib/features/guild/guildStore.svelte', () => ({
  guildState,
}))

vi.mock('./messageStore.svelte', () => ({
  messageState,
}))

import MessageArea from './MessageArea.svelte'

describe('MessageArea', () => {
  beforeEach(() => {
    timelineState.messages = []
    messageState.version += 1
    messageState.timeline.mockClear()
    messageState.sendMessage.mockClear()
    guildState.memberByUserId.mockClear()
  })

  it('sends on Enter and inserts newline on Shift+Enter', async () => {
    const { getByTestId } = render(MessageArea, {
      mode: 'channel',
      activeGuild: 'lobby',
      activeChannel: 'general',
      displayName: 'Alice',
      isAdmin: false,
      showRecoveryNudge: false,
    })

    const composer = getByTestId(
      'message-composer-input',
    ) as HTMLTextAreaElement

    await fireEvent.input(composer, { target: { value: 'hello' } })
    await fireEvent.keyDown(composer, { key: 'Enter' })

    expect(messageState.sendMessage).toHaveBeenCalledTimes(1)
    expect(messageState.sendMessage).toHaveBeenCalledWith(
      'lobby',
      'general',
      'hello',
      expect.objectContaining({
        userId: 'user-1',
        username: 'alice',
        displayName: 'Alice',
        roleColor: '#3366ff',
      }),
    )

    await waitFor(() => {
      expect(composer.value).toBe('')
    })

    messageState.sendMessage.mockClear()
    await fireEvent.input(composer, { target: { value: 'line one' } })
    composer.setSelectionRange(composer.value.length, composer.value.length)
    await fireEvent.keyDown(composer, { key: 'Enter', shiftKey: true })

    expect(messageState.sendMessage).not.toHaveBeenCalled()
    expect(composer.value).toBe('line one\n')
  })

  it('renders empty-state copy and compact grouping/system rows', () => {
    const emptyView = render(MessageArea, {
      mode: 'channel',
      activeGuild: 'lobby',
      activeChannel: 'general',
      displayName: 'Alice',
      isAdmin: false,
      showRecoveryNudge: false,
    })

    expect(
      emptyView.getByText('This is the beginning of #general. Say something!'),
    ).toBeInTheDocument()

    emptyView.unmount()

    timelineState.messages = [
      {
        id: 'm1',
        guildSlug: 'lobby',
        channelSlug: 'general',
        authorUserId: 'user-1',
        authorUsername: 'alice',
        authorDisplayName: 'Alice',
        authorAvatarColor: '#3366ff',
        authorRoleColor: '#3366ff',
        content: 'first',
        isSystem: false,
        createdAt: '2026-02-28T00:00:00Z',
        optimistic: false,
      },
      {
        id: 'm2',
        guildSlug: 'lobby',
        channelSlug: 'general',
        authorUserId: 'user-1',
        authorUsername: 'alice',
        authorDisplayName: 'Alice',
        authorAvatarColor: '#3366ff',
        authorRoleColor: '#3366ff',
        content: 'second',
        isSystem: false,
        createdAt: '2026-02-28T00:00:01Z',
        optimistic: false,
      },
      {
        id: 'sys',
        guildSlug: 'lobby',
        channelSlug: 'general',
        authorUserId: 'system',
        authorUsername: 'system',
        authorDisplayName: 'System',
        authorAvatarColor: null,
        authorRoleColor: '#99aab5',
        content: 'Alice joined the channel',
        isSystem: true,
        createdAt: '2026-02-28T00:00:02Z',
        optimistic: false,
      },
    ]
    messageState.version += 1

    const groupedView = render(MessageArea, {
      mode: 'channel',
      activeGuild: 'lobby',
      activeChannel: 'general',
      displayName: 'Alice',
      isAdmin: false,
      showRecoveryNudge: false,
    })

    expect(groupedView.getByTestId('message-avatar-m1')).toBeInTheDocument()
    expect(
      groupedView.queryByTestId('message-avatar-m2'),
    ).not.toBeInTheDocument()
    expect(groupedView.getByTestId('message-system-sys')).toBeInTheDocument()
  })
})
