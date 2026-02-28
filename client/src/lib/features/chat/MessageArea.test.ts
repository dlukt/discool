import { fireEvent, render, waitFor, within } from '@testing-library/svelte'
import { beforeEach, describe, expect, it, vi } from 'vitest'

const {
  wsLifecycle,
  lifecycleListeners,
  timelineByChannel,
  historyByChannel,
  messageState,
  identityState,
  guildState,
} = vi.hoisted(() => {
  const wsLifecycle = { value: 'connected' as const }
  const lifecycleListeners = new Set<
    (state: typeof wsLifecycle.value) => void
  >()

  const timelineByChannel: Record<
    string,
    Array<{
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
    }>
  > = {}

  const historyByChannel: Record<
    string,
    {
      initialized: boolean
      loadingHistory: boolean
      hasMoreHistory: boolean
      cursor: string | null
      scrollTop: number
      pendingNewCount: number
    }
  > = {}

  function channelKey(guildSlug: string, channelSlug: string): string {
    return `${guildSlug}:${channelSlug}`
  }

  function ensureHistory(key: string) {
    if (!historyByChannel[key]) {
      historyByChannel[key] = {
        initialized: false,
        loadingHistory: false,
        hasMoreHistory: true,
        cursor: null,
        scrollTop: 0,
        pendingNewCount: 0,
      }
    }
    return historyByChannel[key]
  }

  const messageState = {
    version: 1,
    currentUserId: null as string | null,
    timeline: vi.fn((guildSlug: string, channelSlug: string) => {
      return timelineByChannel[channelKey(guildSlug, channelSlug)] ?? []
    }),
    sendMessage: vi.fn(() => true),
    uploadAttachment: vi.fn(
      async (
        _guildSlug: string,
        _channelSlug: string,
        _input: { onProgress?: (value: number) => void },
      ) => {},
    ),
    sendMessageUpdate: vi.fn(() => true),
    sendMessageDelete: vi.fn(() => true),
    sendMessageReactionToggle: vi.fn(() => true),
    setActiveChannel: vi.fn(),
    setCurrentUser: vi.fn((userId: string | null) => {
      messageState.currentUserId = userId
    }),
    ensureHistoryLoaded: vi.fn(
      async (guildSlug: string, channelSlug: string) => {
        ensureHistory(channelKey(guildSlug, channelSlug)).initialized = true
      },
    ),
    historyStateForChannel: vi.fn((guildSlug: string, channelSlug: string) => ({
      ...ensureHistory(channelKey(guildSlug, channelSlug)),
    })),
    loadOlderHistory: vi.fn(async () => {}),
    setScrollTop: vi.fn(
      (guildSlug: string, channelSlug: string, top: number) => {
        ensureHistory(channelKey(guildSlug, channelSlug)).scrollTop =
          Math.round(top)
      },
    ),
    scrollTopForChannel: vi.fn((guildSlug: string, channelSlug: string) => {
      return ensureHistory(channelKey(guildSlug, channelSlug)).scrollTop
    }),
    addPendingNew: vi.fn(
      (guildSlug: string, channelSlug: string, count = 1) => {
        ensureHistory(channelKey(guildSlug, channelSlug)).pendingNewCount +=
          count
        messageState.version += 1
      },
    ),
    clearPendingNew: vi.fn((guildSlug: string, channelSlug: string) => {
      ensureHistory(channelKey(guildSlug, channelSlug)).pendingNewCount = 0
      messageState.version += 1
    }),
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
    bySlug: vi.fn(() => ({ isOwner: true })),
    memberByUserId: vi.fn(() => ({ highestRoleColor: '#3366ff' })),
    memberRoleDataForGuild: vi.fn(() => ({
      members: [],
      roles: [],
      assignableRoleIds: [],
      canManageRoles: false,
    })),
    loadMembers: vi.fn(async () => ({
      members: [],
      roles: [],
      assignableRoleIds: [],
      canManageRoles: false,
    })),
  }

  return {
    wsLifecycle,
    lifecycleListeners,
    timelineByChannel,
    historyByChannel,
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

function seedChannelMessages(channelKey: string, count: number): void {
  timelineByChannel[channelKey] = Array.from({ length: count }, (_, index) => ({
    id: `m-${index}`,
    guildSlug: 'lobby',
    channelSlug: channelKey.split(':')[1] ?? 'general',
    authorUserId: 'user-1',
    authorUsername: 'alice',
    authorDisplayName: 'Alice',
    authorAvatarColor: '#3366ff',
    authorRoleColor: '#3366ff',
    content: `message-${index}`,
    isSystem: false,
    createdAt: `2026-02-28T00:00:${String(index).padStart(2, '0')}Z`,
    updatedAt: `2026-02-28T00:00:${String(index).padStart(2, '0')}Z`,
    optimistic: false,
    attachments: [],
    reactions: [],
  }))
}

describe('MessageArea', () => {
  beforeEach(() => {
    Object.keys(timelineByChannel).forEach((key) => {
      delete timelineByChannel[key]
    })
    Object.keys(historyByChannel).forEach((key) => {
      delete historyByChannel[key]
    })

    seedChannelMessages('lobby:general', 0)
    seedChannelMessages('lobby:random', 0)

    historyByChannel['lobby:general'] = {
      initialized: true,
      loadingHistory: false,
      hasMoreHistory: true,
      cursor: 'cursor-1',
      scrollTop: 0,
      pendingNewCount: 0,
    }
    historyByChannel['lobby:random'] = {
      initialized: true,
      loadingHistory: false,
      hasMoreHistory: true,
      cursor: 'cursor-random',
      scrollTop: 0,
      pendingNewCount: 0,
    }

    messageState.version += 1
    messageState.timeline.mockClear()
    messageState.sendMessage.mockClear()
    messageState.uploadAttachment.mockClear()
    messageState.sendMessageUpdate.mockClear()
    messageState.sendMessageDelete.mockClear()
    messageState.sendMessageReactionToggle.mockClear()
    messageState.setActiveChannel.mockClear()
    messageState.setCurrentUser.mockClear()
    messageState.ensureHistoryLoaded.mockClear()
    messageState.historyStateForChannel.mockClear()
    messageState.loadOlderHistory.mockClear()
    messageState.setScrollTop.mockClear()
    messageState.scrollTopForChannel.mockClear()
    messageState.addPendingNew.mockClear()
    messageState.clearPendingNew.mockClear()
    guildState.memberByUserId.mockClear()
    guildState.memberRoleDataForGuild.mockClear()
    guildState.loadMembers.mockClear()
    guildState.bySlug.mockClear()
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

  it('virtualizes long timelines and loads older history on upward scroll', async () => {
    seedChannelMessages('lobby:general', 160)
    messageState.version += 1

    const { container, getByTestId } = render(MessageArea, {
      mode: 'channel',
      activeGuild: 'lobby',
      activeChannel: 'general',
      displayName: 'Alice',
      isAdmin: false,
      showRecoveryNudge: false,
    })

    const initiallyRenderedRows = container.querySelectorAll(
      '[data-testid^="message-window-row-"]',
    )
    expect(initiallyRenderedRows.length).toBeLessThan(90)

    const scroll = getByTestId('channel-timeline-scroll') as HTMLDivElement
    Object.defineProperty(scroll, 'clientHeight', {
      value: 240,
      configurable: true,
    })

    scroll.scrollTop = 0
    await fireEvent.scroll(scroll)

    await waitFor(() => {
      expect(messageState.loadOlderHistory).toHaveBeenCalledWith(
        'lobby',
        'general',
      )
    })
  })

  it('renders skeleton loading and jump-to-present CTA', async () => {
    seedChannelMessages('lobby:general', 12)
    historyByChannel['lobby:general'] = {
      initialized: true,
      loadingHistory: true,
      hasMoreHistory: true,
      cursor: 'cursor-1',
      scrollTop: 0,
      pendingNewCount: 3,
    }
    messageState.version += 1

    const { getByTestId } = render(MessageArea, {
      mode: 'channel',
      activeGuild: 'lobby',
      activeChannel: 'general',
      displayName: 'Alice',
      isAdmin: false,
      showRecoveryNudge: false,
    })

    expect(getByTestId('history-loading-skeletons')).toBeInTheDocument()

    const jumpButton = getByTestId('jump-to-present')
    expect(jumpButton).toHaveTextContent('Jump to present (3 new)')
    await fireEvent.click(jumpButton)

    expect(messageState.clearPendingNew).toHaveBeenCalledWith(
      'lobby',
      'general',
    )
  })

  it('restores saved scroll position when switching back to a channel', async () => {
    seedChannelMessages('lobby:general', 24)
    seedChannelMessages('lobby:random', 24)
    historyByChannel['lobby:general'].scrollTop = 180
    historyByChannel['lobby:random'].scrollTop = 56
    messageState.version += 1

    const view = render(MessageArea, {
      mode: 'channel',
      activeGuild: 'lobby',
      activeChannel: 'general',
      displayName: 'Alice',
      isAdmin: false,
      showRecoveryNudge: false,
    })

    await waitFor(() => {
      const scroll = view.getByTestId(
        'channel-timeline-scroll',
      ) as HTMLDivElement
      expect(scroll.scrollTop).toBe(180)
    })

    await view.rerender({
      mode: 'channel',
      activeGuild: 'lobby',
      activeChannel: 'random',
      displayName: 'Alice',
      isAdmin: false,
      showRecoveryNudge: false,
    })

    await waitFor(() => {
      const scroll = view.getByTestId(
        'channel-timeline-scroll',
      ) as HTMLDivElement
      expect(scroll.scrollTop).toBe(56)
    })

    await view.rerender({
      mode: 'channel',
      activeGuild: 'lobby',
      activeChannel: 'general',
      displayName: 'Alice',
      isAdmin: false,
      showRecoveryNudge: false,
    })

    await waitFor(() => {
      const scroll = view.getByTestId(
        'channel-timeline-scroll',
      ) as HTMLDivElement
      expect(scroll.scrollTop).toBe(180)
    })
  })

  it('supports composer edit mode with Up/Enter/Escape', async () => {
    seedChannelMessages('lobby:general', 3)
    messageState.version += 1

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

    await fireEvent.keyDown(composer, { key: 'ArrowUp' })
    expect(composer.value).toBe('message-2')

    await fireEvent.input(composer, { target: { value: 'edited text' } })
    await fireEvent.keyDown(composer, { key: 'Enter' })

    expect(messageState.sendMessageUpdate).toHaveBeenCalledWith(
      'lobby',
      'general',
      'm-2',
      'edited text',
    )

    await waitFor(() => {
      expect(composer.value).toBe('')
    })

    messageState.sendMessageUpdate.mockClear()
    await fireEvent.keyDown(composer, { key: 'ArrowUp' })
    expect(composer.value).toBe('message-2')
    await fireEvent.keyDown(composer, { key: 'Escape' })
    expect(messageState.sendMessageUpdate).not.toHaveBeenCalled()
    expect(composer.value).toBe('')
  })

  it('uploads selected attachment from composer', async () => {
    messageState.uploadAttachment.mockImplementation(
      (
        _guildSlug: string,
        _channelSlug: string,
        input: { onProgress?: (value: number) => void },
      ) =>
        new Promise<void>((resolve) => {
          input.onProgress?.(45)
          setTimeout(resolve, 10)
        }),
    )

    const { getByTestId, queryByTestId } = render(MessageArea, {
      mode: 'channel',
      activeGuild: 'lobby',
      activeChannel: 'general',
      displayName: 'Alice',
      isAdmin: false,
      showRecoveryNudge: false,
    })

    const attachmentInput = getByTestId(
      'message-attachment-input',
    ) as HTMLInputElement
    const file = new File(['image'], 'image.png', { type: 'image/png' })
    await fireEvent.change(attachmentInput, { target: { files: [file] } })
    expect(getByTestId('attachment-preview-chip')).toBeInTheDocument()

    const composer = getByTestId(
      'message-composer-input',
    ) as HTMLTextAreaElement
    await fireEvent.input(composer, { target: { value: 'with file' } })
    await fireEvent.click(getByTestId('message-composer-submit'))

    await waitFor(() => {
      expect(messageState.uploadAttachment).toHaveBeenCalledWith(
        'lobby',
        'general',
        expect.objectContaining({
          file,
          content: 'with file',
          onProgress: expect.any(Function),
        }),
      )
    })
    expect(getByTestId('attachment-upload-progress')).toBeInTheDocument()

    await waitFor(() => {
      expect(queryByTestId('attachment-upload-progress')).toBeNull()
    })
  })

  it('requires confirmation before delete operation is sent', async () => {
    seedChannelMessages('lobby:general', 1)
    messageState.version += 1

    const { getByTestId, getByRole, queryByRole } = render(MessageArea, {
      mode: 'channel',
      activeGuild: 'lobby',
      activeChannel: 'general',
      displayName: 'Alice',
      isAdmin: false,
      showRecoveryNudge: false,
    })

    const messageRow = getByTestId('message-row-m-0')
    await fireEvent.keyDown(messageRow, { key: 'Delete' })

    const dialog = getByRole('dialog', { name: 'Delete message' })
    expect(dialog).toBeInTheDocument()
    await fireEvent.click(
      within(dialog).getByRole('button', { name: 'Cancel' }),
    )
    expect(queryByRole('dialog', { name: 'Delete message' })).toBeNull()
    expect(messageState.sendMessageDelete).not.toHaveBeenCalled()

    await fireEvent.keyDown(messageRow, { key: 'Delete' })
    const confirmDialog = getByRole('dialog', { name: 'Delete message' })
    await fireEvent.click(
      within(confirmDialog).getByRole('button', { name: 'Delete message' }),
    )

    expect(messageState.sendMessageDelete).toHaveBeenCalledWith(
      'lobby',
      'general',
      'm-0',
    )
  })

  it('routes emoji reaction selection to websocket toggle operation', async () => {
    seedChannelMessages('lobby:general', 1)
    messageState.version += 1

    const { getByTestId } = render(MessageArea, {
      mode: 'channel',
      activeGuild: 'lobby',
      activeChannel: 'general',
      displayName: 'Alice',
      isAdmin: false,
      showRecoveryNudge: false,
    })

    await fireEvent.click(getByTestId('message-react-button-m-0'))
    await fireEvent.click(getByTestId('message-reaction-picker-option-m-0-3'))

    expect(messageState.sendMessageReactionToggle).toHaveBeenCalledWith(
      'lobby',
      'general',
      'm-0',
      '👍',
    )
  })
})
