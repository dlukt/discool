import { fireEvent, render, waitFor, within } from '@testing-library/svelte'
import { beforeEach, describe, expect, it, vi } from 'vitest'

const {
  wsLifecycle,
  lifecycleListeners,
  wsEnvelopeListeners,
  toastState,
  voiceState,
  timelineByChannel,
  historyByChannel,
  messageState,
  identityState,
  guildState,
} = vi.hoisted(() => {
  const wsLifecycle = {
    value: 'connected' as
      | 'connecting'
      | 'connected'
      | 'reconnecting'
      | 'disconnected',
  }
  const lifecycleListeners = new Set<
    (state: typeof wsLifecycle.value) => void
  >()
  const wsEnvelopeListeners = new Set<(envelope: unknown) => void>()
  const toastState = {
    show: vi.fn(),
    dismiss: vi.fn(),
    clearAll: vi.fn(),
  }
  const voiceState = {
    statusMessageForChannel: vi.fn<
      (guildSlug: string, channelSlug: string) => string | null
    >(() => null),
    statusForChannel: vi.fn<
      (
        guildSlug: string,
        channelSlug: string,
      ) =>
        | 'idle'
        | 'connecting'
        | 'connected'
        | 'disconnected'
        | 'retrying'
        | 'failed'
    >(() => 'idle'),
    activeChannelParticipants: vi.fn<
      () => Array<{
        userId: string
        username: string
        displayName: string | null
        avatarColor: string | null
        isMuted: boolean
        isDeafened: boolean
        isSpeaking: boolean
      }>
    >(() => []),
    isMuted: false,
    isDeafened: false,
    toggleMute: vi.fn(),
    toggleDeafen: vi.fn(),
    disconnect: vi.fn(),
  }

  const timelineByChannel: Record<
    string,
    Array<{
      id: string
      guildSlug: string
      channelSlug: string
      dmSlug?: string
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

  function dmKey(dmSlug: string): string {
    return `dm:${dmSlug}`
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
    timelineForDm: vi.fn((dmSlug: string) => {
      return timelineByChannel[dmKey(dmSlug)] ?? []
    }),
    sendMessage: vi.fn(() => true),
    sendDmMessage: vi.fn(() => true),
    sendTypingStart: vi.fn(() => true),
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
    setActiveDm: vi.fn(),
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
    historyStateForDm: vi.fn((dmSlug: string) => ({
      ...ensureHistory(dmKey(dmSlug)),
    })),
    loadOlderHistory: vi.fn(async () => {}),
    ensureDmHistoryLoaded: vi.fn(async (_dmSlug: string) => {}),
    loadOlderDmHistory: vi.fn(async (_dmSlug: string) => {}),
    setScrollTop: vi.fn(
      (guildSlug: string, channelSlug: string, top: number) => {
        ensureHistory(channelKey(guildSlug, channelSlug)).scrollTop =
          Math.round(top)
      },
    ),
    setScrollTopForDm: vi.fn((_dmSlug: string, _top: number) => {}),
    scrollTopForChannel: vi.fn((guildSlug: string, channelSlug: string) => {
      return ensureHistory(channelKey(guildSlug, channelSlug)).scrollTop
    }),
    scrollTopForDm: vi.fn((_dmSlug: string) => 0),
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
    addPendingNewForDm: vi.fn((_dmSlug: string, _count = 1) => {}),
    clearPendingNewForDm: vi.fn((_dmSlug: string) => {}),
    typingUserIdsForChannel: vi.fn(() => [] as string[]),
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
      members: [] as Array<Record<string, unknown>>,
      roles: [] as Array<Record<string, unknown>>,
      assignableRoleIds: [],
      canManageRoles: false,
    })),
    loadMembers: vi.fn(async () => ({
      members: [] as Array<Record<string, unknown>>,
      roles: [] as Array<Record<string, unknown>>,
      assignableRoleIds: [],
      canManageRoles: false,
    })),
  }

  return {
    wsLifecycle,
    lifecycleListeners,
    wsEnvelopeListeners,
    toastState,
    voiceState,
    timelineByChannel,
    historyByChannel,
    dmKey,
    messageState,
    identityState,
    guildState,
  }
})

vi.mock('$lib/ws/client', () => ({
  wsClient: {
    getLifecycleState: vi.fn(() => wsLifecycle.value),
    subscribe: vi.fn((listener: (envelope: unknown) => void) => {
      wsEnvelopeListeners.add(listener)
      return () => wsEnvelopeListeners.delete(listener)
    }),
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

vi.mock('$lib/feedback/toastStore.svelte', () => ({
  toastState,
}))

vi.mock('$lib/features/voice/voiceStore.svelte', () => ({
  voiceState,
}))

import { PERMISSION_DENIED_MESSAGE } from '$lib/feedback/userFacingError'
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
    embeds: [],
  }))
}

function seedDmMessages(dmSlug: string, count: number): void {
  timelineByChannel[`dm:${dmSlug}`] = Array.from(
    { length: count },
    (_, index) => ({
      id: `dm-${index}`,
      guildSlug: '',
      channelSlug: '',
      dmSlug,
      authorUserId: 'user-1',
      authorUsername: 'alice',
      authorDisplayName: 'Alice',
      authorAvatarColor: '#3366ff',
      authorRoleColor: '#3366ff',
      content: `dm-message-${index}`,
      isSystem: false,
      createdAt: `2026-02-28T00:10:${String(index).padStart(2, '0')}Z`,
      updatedAt: `2026-02-28T00:10:${String(index).padStart(2, '0')}Z`,
      optimistic: false,
      attachments: [],
      reactions: [],
      embeds: [],
    }),
  )
}

describe('MessageArea', () => {
  beforeEach(() => {
    wsLifecycle.value = 'connected'
    lifecycleListeners.clear()
    wsEnvelopeListeners.clear()
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
    messageState.timelineForDm.mockClear()
    messageState.sendMessage.mockClear()
    messageState.sendDmMessage.mockClear()
    messageState.sendTypingStart.mockClear()
    messageState.uploadAttachment.mockClear()
    messageState.sendMessageUpdate.mockClear()
    messageState.sendMessageDelete.mockClear()
    messageState.sendMessageReactionToggle.mockClear()
    messageState.setActiveChannel.mockClear()
    messageState.setActiveDm.mockClear()
    messageState.setCurrentUser.mockClear()
    messageState.ensureHistoryLoaded.mockClear()
    messageState.ensureDmHistoryLoaded.mockClear()
    messageState.historyStateForChannel.mockClear()
    messageState.historyStateForDm.mockClear()
    messageState.loadOlderHistory.mockClear()
    messageState.loadOlderDmHistory.mockClear()
    messageState.setScrollTop.mockClear()
    messageState.setScrollTopForDm.mockClear()
    messageState.scrollTopForChannel.mockClear()
    messageState.scrollTopForDm.mockClear()
    messageState.addPendingNew.mockClear()
    messageState.addPendingNewForDm.mockClear()
    messageState.clearPendingNew.mockClear()
    messageState.clearPendingNewForDm.mockClear()
    messageState.typingUserIdsForChannel.mockClear()
    messageState.typingUserIdsForChannel.mockReturnValue([])
    toastState.show.mockClear()
    toastState.dismiss.mockClear()
    toastState.clearAll.mockClear()
    voiceState.statusMessageForChannel.mockClear()
    voiceState.statusMessageForChannel.mockReturnValue(null)
    voiceState.statusForChannel.mockClear()
    voiceState.statusForChannel.mockReturnValue('idle')
    voiceState.activeChannelParticipants.mockClear()
    voiceState.activeChannelParticipants.mockReturnValue([])
    voiceState.isMuted = false
    voiceState.isDeafened = false
    voiceState.toggleMute.mockClear()
    voiceState.toggleDeafen.mockClear()
    voiceState.disconnect.mockClear()
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

  it('reuses composer and timeline behavior for DM mode', async () => {
    seedDmMessages('dm-1', 1)
    historyByChannel['dm:dm-1'] = {
      initialized: true,
      loadingHistory: false,
      hasMoreHistory: true,
      cursor: 'dm-cursor-1',
      scrollTop: 0,
      pendingNewCount: 0,
    }

    const { getByTestId } = render(MessageArea, {
      mode: 'dm',
      activeGuild: 'lobby',
      activeChannel: 'general',
      activeDm: 'dm-1',
      displayName: 'Alice',
      isAdmin: false,
      showRecoveryNudge: false,
    })

    expect(messageState.setActiveDm).toHaveBeenCalledWith('dm-1')
    const composer = getByTestId(
      'message-composer-input',
    ) as HTMLTextAreaElement

    await fireEvent.input(composer, { target: { value: 'hello dm' } })
    await fireEvent.keyDown(composer, { key: 'Enter' })

    expect(messageState.sendDmMessage).toHaveBeenCalledWith(
      'dm-1',
      'hello dm',
      expect.objectContaining({
        userId: 'user-1',
        username: 'alice',
        displayName: 'Alice',
      }),
    )
  })

  it('emits typing_start only for non-empty composer drafts', async () => {
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

    await fireEvent.input(composer, { target: { value: '   ' } })
    expect(messageState.sendTypingStart).not.toHaveBeenCalled()

    await fireEvent.input(composer, { target: { value: 'hello' } })
    expect(messageState.sendTypingStart).toHaveBeenCalledWith(
      'lobby',
      'general',
    )
  })

  it('renders typing indicator copy for active channel typers', () => {
    guildState.memberRoleDataForGuild.mockReturnValue({
      members: [
        {
          userId: 'user-1',
          username: 'alice',
          displayName: 'Alice',
          avatarColor: '#3366ff',
          presenceStatus: 'online',
          highestRoleColor: '#3366ff',
          roleIds: [],
          isOwner: true,
          canAssignRoles: false,
        },
        {
          userId: 'user-2',
          username: 'bob',
          displayName: 'Bob',
          avatarColor: '#44aa99',
          presenceStatus: 'online',
          highestRoleColor: '#44aa99',
          roleIds: [],
          isOwner: false,
          canAssignRoles: false,
        },
      ],
      roles: [],
      assignableRoleIds: [],
      canManageRoles: false,
    })
    messageState.typingUserIdsForChannel.mockReturnValue(['user-2'])

    const { getByTestId } = render(MessageArea, {
      mode: 'channel',
      activeGuild: 'lobby',
      activeChannel: 'general',
      displayName: 'Alice',
      isAdmin: false,
      showRecoveryNudge: false,
    })

    expect(getByTestId('typing-indicator')).toHaveTextContent(
      'Bob is typing...',
    )
  })

  it('renders generic typing indicator copy when more than three users are typing', () => {
    guildState.memberRoleDataForGuild.mockReturnValue({
      members: [
        {
          userId: 'user-1',
          username: 'alice',
          displayName: 'Alice',
          avatarColor: '#3366ff',
          presenceStatus: 'online',
          highestRoleColor: '#3366ff',
          roleIds: [],
          isOwner: true,
          canAssignRoles: false,
        },
        {
          userId: 'user-2',
          username: 'bob',
          displayName: 'Bob',
          avatarColor: '#44aa99',
          presenceStatus: 'online',
          highestRoleColor: '#44aa99',
          roleIds: [],
          isOwner: false,
          canAssignRoles: false,
        },
        {
          userId: 'user-3',
          username: 'charlie',
          displayName: 'Charlie',
          avatarColor: '#aa7755',
          presenceStatus: 'online',
          highestRoleColor: '#aa7755',
          roleIds: [],
          isOwner: false,
          canAssignRoles: false,
        },
        {
          userId: 'user-4',
          username: 'dana',
          displayName: 'Dana',
          avatarColor: '#9966cc',
          presenceStatus: 'online',
          highestRoleColor: '#9966cc',
          roleIds: [],
          isOwner: false,
          canAssignRoles: false,
        },
      ],
      roles: [],
      assignableRoleIds: [],
      canManageRoles: false,
    })
    messageState.typingUserIdsForChannel.mockReturnValue([
      'user-1',
      'user-2',
      'user-3',
      'user-4',
    ])

    const { getByTestId } = render(MessageArea, {
      mode: 'channel',
      activeGuild: 'lobby',
      activeChannel: 'general',
      displayName: 'Alice',
      isAdmin: false,
      showRecoveryNudge: false,
    })

    expect(getByTestId('typing-indicator')).toHaveTextContent(
      'Several people are typing...',
    )
  })

  it('applies markdown toolbar and keyboard shortcuts to selected text', async () => {
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
    await fireEvent.input(composer, { target: { value: 'hello world' } })

    composer.focus()
    composer.setSelectionRange(0, 5)
    await fireEvent.select(composer)
    await fireEvent.click(getByTestId('message-format-bold'))
    expect(composer.value).toBe('**hello** world')

    composer.setSelectionRange(10, 15)
    await fireEvent.select(composer)
    await fireEvent.keyDown(composer, { key: 'e', ctrlKey: true })
    expect(composer.value).toBe('**hello** `world`')

    composer.setSelectionRange(2, 7)
    await fireEvent.select(composer)
    await fireEvent.keyDown(composer, { key: 'i', ctrlKey: true })
    expect(composer.value).toBe('***hello*** `world`')
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

  it('shows standardized permission copy for blocked attachment sends', async () => {
    guildState.bySlug.mockReturnValue({ isOwner: false })
    guildState.memberRoleDataForGuild.mockReturnValue({
      members: [
        {
          userId: 'user-1',
          username: 'alice',
          displayName: 'Alice',
          avatarColor: '#3366ff',
          presenceStatus: 'online',
          highestRoleColor: '#3366ff',
          roleIds: [],
          isOwner: false,
          canAssignRoles: false,
        },
      ],
      roles: [{ id: 'role-default', permissionsBitflag: 0, isDefault: true }],
      assignableRoleIds: [],
      canManageRoles: false,
    })

    const { getByTestId } = render(MessageArea, {
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
    const file = new File(['image'], 'restricted.png', { type: 'image/png' })
    await fireEvent.change(attachmentInput, { target: { files: [file] } })
    await fireEvent.click(getByTestId('message-composer-submit'))

    expect(messageState.uploadAttachment).not.toHaveBeenCalled()
    expect(getByTestId('attachment-error')).toHaveTextContent(
      PERMISSION_DENIED_MESSAGE,
    )
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

  it('shows plain-language connection status text', () => {
    wsLifecycle.value = 'reconnecting'
    const { getByTestId } = render(MessageArea, {
      mode: 'channel',
      activeGuild: 'lobby',
      activeChannel: 'general',
      displayName: 'Alice',
      isAdmin: false,
      showRecoveryNudge: false,
    })

    expect(getByTestId('connection-status')).toHaveTextContent(
      'Connection lost. Reconnecting...',
    )
  })

  it('shows required voice retry/failure copy via aria-live status', () => {
    voiceState.statusMessageForChannel.mockReturnValue(
      'Could not connect to voice. Retrying...',
    )
    voiceState.statusForChannel.mockReturnValue('retrying')
    const { getByTestId, rerender } = render(MessageArea, {
      mode: 'channel',
      activeGuild: 'lobby',
      activeChannel: 'general',
      displayName: 'Alice',
      isAdmin: false,
      showRecoveryNudge: false,
    })
    const retryStatus = getByTestId('voice-connection-status')
    expect(retryStatus).toHaveTextContent(
      'Could not connect to voice. Retrying...',
    )
    expect(retryStatus).toHaveAttribute('aria-live', 'polite')

    voiceState.statusMessageForChannel.mockReturnValue(
      'Voice connection failed. Check your network.',
    )
    voiceState.statusForChannel.mockReturnValue('failed')
    rerender({
      mode: 'channel',
      activeGuild: 'lobby',
      activeChannel: 'general',
      displayName: 'Alice',
      isAdmin: false,
      showRecoveryNudge: false,
    })

    const failedStatus = getByTestId('voice-connection-status')
    expect(failedStatus).toHaveTextContent(
      'Voice connection failed. Check your network.',
    )
    expect(failedStatus).toHaveAttribute('aria-live', 'assertive')
  })

  it('renders connected VoiceBar controls and routes control actions', async () => {
    voiceState.statusForChannel.mockReturnValue('connected')
    const { getByTestId } = render(MessageArea, {
      mode: 'channel',
      activeGuild: 'lobby',
      activeChannel: 'general',
      displayName: 'Alice',
      isAdmin: false,
      showRecoveryNudge: false,
    })

    expect(getByTestId('voice-bar')).toBeInTheDocument()
    expect(getByTestId('voice-bar-quality-dot')).toHaveAttribute(
      'data-quality',
      'green',
    )

    await fireEvent.click(getByTestId('voice-bar-toggle-mute'))
    await fireEvent.click(getByTestId('voice-bar-toggle-deafen'))
    await fireEvent.click(getByTestId('voice-bar-disconnect'))

    expect(voiceState.toggleMute).toHaveBeenCalledTimes(1)
    expect(voiceState.toggleDeafen).toHaveBeenCalledTimes(1)
    expect(voiceState.disconnect).toHaveBeenCalledTimes(1)
  })

  it('toggles voice participant panel from VoiceBar expand control', async () => {
    voiceState.statusForChannel.mockReturnValue('connected')
    voiceState.activeChannelParticipants.mockReturnValue([
      {
        userId: 'user-2',
        username: 'bob',
        displayName: 'Bob',
        avatarColor: '#3366ff',
        isMuted: true,
        isDeafened: false,
        isSpeaking: true,
      },
    ])
    const { getByTestId, queryByTestId } = render(MessageArea, {
      mode: 'channel',
      activeGuild: 'lobby',
      activeChannel: 'general',
      displayName: 'Alice',
      isAdmin: false,
      showRecoveryNudge: false,
    })

    expect(queryByTestId('voice-panel')).not.toBeInTheDocument()
    await fireEvent.click(getByTestId('voice-bar-toggle-participants'))
    expect(getByTestId('voice-panel')).toBeInTheDocument()
    expect(getByTestId('voice-participant-user-2')).toBeInTheDocument()
    await fireEvent.click(getByTestId('voice-bar-toggle-participants'))
    expect(queryByTestId('voice-panel')).not.toBeInTheDocument()
  })

  it('shows retry toast when send fails and retries via toast action', async () => {
    messageState.sendMessage.mockReturnValue(false)

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

    await fireEvent.input(composer, { target: { value: 'hello retry' } })
    await fireEvent.keyDown(composer, { key: 'Enter' })

    expect(toastState.show).toHaveBeenCalledTimes(1)
    expect(toastState.show).toHaveBeenCalledWith(
      expect.objectContaining({
        variant: 'error',
        actionLabel: 'Retry?',
      }),
    )

    const toastInput = toastState.show.mock.calls[0]?.[0] as
      | { onAction?: () => void }
      | undefined
    expect(toastInput?.onAction).toBeTypeOf('function')

    messageState.sendMessage.mockReturnValue(true)
    toastInput?.onAction?.()
    expect(messageState.sendMessage).toHaveBeenCalledTimes(2)
  })
})
