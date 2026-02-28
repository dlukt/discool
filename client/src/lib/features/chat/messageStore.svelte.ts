import { wsClient } from '$lib/ws/client'
import type { WsEnvelope } from '$lib/ws/protocol'
import { fetchChannelHistory } from './messageApi'
import { type ChatMessage, toChatMessage } from './types'

const DEFAULT_ROLE_COLOR = '#99aab5'
const HISTORY_PAGE_LIMIT = 50
const MAX_ACTIVE_CHANNEL_MESSAGES = 4_000
const MAX_INACTIVE_CHANNEL_MESSAGES = 1_200
const MAX_TOTAL_TIMELINE_MESSAGES = 10_000
const MIN_CHANNEL_RETAINED_MESSAGES = 200

type MessageCreatePayload = {
  id?: string
  guild_slug?: string
  channel_slug?: string
  author_user_id?: string
  author_username?: string
  author_display_name?: string
  author_avatar_color?: string | null
  author_role_color?: string
  content?: string
  is_system?: boolean
  created_at?: string
  client_nonce?: string
}

export type ChatAuthorInput = {
  userId: string
  username: string
  displayName: string
  avatarColor: string | null
  roleColor: string
}

type PendingOptimisticEntry = {
  channelKey: string
  messageId: string
}

type ChannelHistoryState = {
  initialized: boolean
  loadingHistory: boolean
  hasMoreHistory: boolean
  cursor: string | null
  scrollTop: number
  pendingNewCount: number
}

type ActiveTrimMode = 'drop_oldest' | 'drop_newest'

function defaultHistoryState(): ChannelHistoryState {
  return {
    initialized: false,
    loadingHistory: false,
    hasMoreHistory: true,
    cursor: null,
    scrollTop: 0,
    pendingNewCount: 0,
  }
}

function readHistoryState(
  historyByChannel: Record<string, ChannelHistoryState>,
  channelKey: string,
): ChannelHistoryState {
  return historyByChannel[channelKey] ?? defaultHistoryState()
}

function toChannelKey(guildSlug: string, channelSlug: string): string {
  return `${guildSlug}:${channelSlug}`
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null
}

function normalizeOutboundContent(content: string): string {
  return content.replace(/\r\n/g, '\n').replace(/\r/g, '\n')
}

function generateClientNonce(): string {
  if (
    typeof crypto !== 'undefined' &&
    typeof crypto.randomUUID === 'function'
  ) {
    return crypto.randomUUID()
  }
  return `nonce-${Date.now()}-${Math.random().toString(16).slice(2)}`
}

function sortMessages(messages: ChatMessage[]): ChatMessage[] {
  return [...messages].sort(
    (left, right) =>
      left.createdAt.localeCompare(right.createdAt) ||
      left.id.localeCompare(right.id),
  )
}

function mergeMessages(
  existing: ChatMessage[],
  incoming: ChatMessage[],
): ChatMessage[] {
  const byId = new Map<string, ChatMessage>()
  for (const message of existing) {
    byId.set(message.id, message)
  }
  for (const message of incoming) {
    const current = byId.get(message.id)
    if (!current || current.optimistic) {
      byId.set(message.id, { ...message, optimistic: false })
    }
  }
  return sortMessages([...byId.values()])
}

function parseMessageCreateEnvelope(envelope: WsEnvelope): ChatMessage | null {
  if (envelope.op !== 'message_create') return null
  if (!isRecord(envelope.d)) return null
  const payload = envelope.d as MessageCreatePayload

  const id = payload.id?.trim()
  const guildSlug = payload.guild_slug?.trim()
  const channelSlug = payload.channel_slug?.trim()
  const authorUserId = payload.author_user_id?.trim()
  const authorUsername = payload.author_username?.trim()
  const createdAt = payload.created_at?.trim()

  if (
    !id ||
    !guildSlug ||
    !channelSlug ||
    !authorUserId ||
    !authorUsername ||
    !createdAt
  ) {
    return null
  }

  return toChatMessage({
    id,
    guild_slug: guildSlug,
    channel_slug: channelSlug,
    author_user_id: authorUserId,
    author_username: authorUsername,
    author_display_name: payload.author_display_name?.trim() || authorUsername,
    author_avatar_color:
      typeof payload.author_avatar_color === 'string'
        ? payload.author_avatar_color
        : null,
    author_role_color: payload.author_role_color?.trim() || DEFAULT_ROLE_COLOR,
    content: typeof payload.content === 'string' ? payload.content : '',
    is_system: payload.is_system === true,
    created_at: createdAt,
    client_nonce: payload.client_nonce?.trim() || undefined,
  })
}

function updateHistoryState(
  channelKey: string,
  updates: Partial<ChannelHistoryState>,
): void {
  const current = readHistoryState(messageState.historyByChannel, channelKey)
  messageState.historyByChannel = {
    ...messageState.historyByChannel,
    [channelKey]: {
      ...current,
      ...updates,
    },
  }
}

function trimMessages(
  messages: ChatMessage[],
  maxItems: number,
  mode: ActiveTrimMode,
): ChatMessage[] {
  if (messages.length <= maxItems) return messages
  if (mode === 'drop_newest') {
    return messages.slice(0, maxItems)
  }
  return messages.slice(messages.length - maxItems)
}

function enforceMemoryBudget(
  activeTrimMode: ActiveTrimMode = 'drop_oldest',
): boolean {
  let changed = false
  const activeKey = messageState.activeChannelKey

  for (const [channelKey, messages] of Object.entries(
    messageState.messagesByChannel,
  )) {
    const maxPerChannel =
      channelKey === activeKey
        ? MAX_ACTIVE_CHANNEL_MESSAGES
        : MAX_INACTIVE_CHANNEL_MESSAGES
    const nextMessages = trimMessages(
      messages,
      maxPerChannel,
      channelKey === activeKey ? activeTrimMode : 'drop_oldest',
    )
    if (nextMessages === messages) continue
    messageState.messagesByChannel[channelKey] = nextMessages
    changed = true
  }

  const entries = Object.entries(messageState.messagesByChannel)
  const total = entries.reduce((sum, [, messages]) => sum + messages.length, 0)
  let overflow = total - MAX_TOTAL_TIMELINE_MESSAGES
  if (overflow <= 0) return changed

  const nonActive = entries
    .filter(([channelKey]) => channelKey !== activeKey)
    .sort((left, right) => right[1].length - left[1].length)

  for (const [channelKey, messages] of nonActive) {
    if (overflow <= 0) break
    const removable = Math.max(
      0,
      messages.length - MIN_CHANNEL_RETAINED_MESSAGES,
    )
    if (removable <= 0) continue
    const dropCount = Math.min(removable, overflow)
    if (dropCount <= 0) continue
    messageState.messagesByChannel[channelKey] = messages.slice(dropCount)
    overflow -= dropCount
    changed = true
  }

  if (overflow > 0 && activeKey) {
    const activeMessages = messageState.messagesByChannel[activeKey] ?? []
    const removable = Math.max(
      0,
      activeMessages.length - MIN_CHANNEL_RETAINED_MESSAGES,
    )
    const dropCount = Math.min(removable, overflow)
    if (dropCount > 0) {
      const keepCount = activeMessages.length - dropCount
      messageState.messagesByChannel[activeKey] =
        activeTrimMode === 'drop_newest'
          ? activeMessages.slice(0, keepCount)
          : activeMessages.slice(dropCount)
      changed = true
    }
  }

  return changed
}

export const messageState = $state({
  version: 0,
  activeChannelKey: null as string | null,
  messagesByChannel: {} as Record<string, ChatMessage[]>,
  optimisticByNonce: {} as Record<string, PendingOptimisticEntry>,
  historyByChannel: {} as Record<string, ChannelHistoryState>,

  timeline: (guildSlug: string, channelSlug: string): ChatMessage[] => {
    const key = toChannelKey(guildSlug, channelSlug)
    return [...(messageState.messagesByChannel[key] ?? [])]
  },

  historyStateForChannel: (
    guildSlug: string,
    channelSlug: string,
  ): ChannelHistoryState => {
    const key = toChannelKey(guildSlug, channelSlug)
    return { ...readHistoryState(messageState.historyByChannel, key) }
  },

  setActiveChannel: (guildSlug: string, channelSlug: string): void => {
    const normalizedGuild = guildSlug.trim()
    const normalizedChannel = channelSlug.trim()
    if (!normalizedGuild || !normalizedChannel) return
    const nextKey = toChannelKey(normalizedGuild, normalizedChannel)
    if (messageState.activeChannelKey === nextKey) return
    messageState.activeChannelKey = nextKey
    if (enforceMemoryBudget()) {
      messageState.version += 1
    }
  },

  ensureHistoryLoaded: async (
    guildSlug: string,
    channelSlug: string,
  ): Promise<void> => {
    const normalizedGuild = guildSlug.trim()
    const normalizedChannel = channelSlug.trim()
    if (!normalizedGuild || !normalizedChannel) return

    const channelKey = toChannelKey(normalizedGuild, normalizedChannel)
    const state = readHistoryState(messageState.historyByChannel, channelKey)
    if (state.initialized || state.loadingHistory) return

    updateHistoryState(channelKey, { loadingHistory: true })
    messageState.version += 1

    try {
      const page = await fetchChannelHistory(
        normalizedGuild,
        normalizedChannel,
        {
          limit: HISTORY_PAGE_LIMIT,
        },
      )
      const existing = messageState.messagesByChannel[channelKey] ?? []
      messageState.messagesByChannel[channelKey] = mergeMessages(
        existing,
        page.messages,
      )
      updateHistoryState(channelKey, {
        initialized: true,
        loadingHistory: false,
        hasMoreHistory: page.cursor !== null,
        cursor: page.cursor,
      })
      enforceMemoryBudget()
      messageState.version += 1
    } catch (error) {
      updateHistoryState(channelKey, { loadingHistory: false })
      messageState.version += 1
      throw error
    }
  },

  loadOlderHistory: async (
    guildSlug: string,
    channelSlug: string,
  ): Promise<void> => {
    const normalizedGuild = guildSlug.trim()
    const normalizedChannel = channelSlug.trim()
    if (!normalizedGuild || !normalizedChannel) return

    const channelKey = toChannelKey(normalizedGuild, normalizedChannel)
    const state = readHistoryState(messageState.historyByChannel, channelKey)
    if (state.loadingHistory || !state.hasMoreHistory) return

    updateHistoryState(channelKey, { loadingHistory: true })
    messageState.version += 1

    try {
      const page = await fetchChannelHistory(
        normalizedGuild,
        normalizedChannel,
        {
          limit: HISTORY_PAGE_LIMIT,
          before: state.cursor,
        },
      )

      if (page.messages.length === 0) {
        updateHistoryState(channelKey, {
          loadingHistory: false,
          initialized: true,
          hasMoreHistory: false,
          cursor: null,
        })
        messageState.version += 1
        return
      }

      const existing = messageState.messagesByChannel[channelKey] ?? []
      messageState.messagesByChannel[channelKey] = mergeMessages(
        existing,
        page.messages,
      )
      updateHistoryState(channelKey, {
        loadingHistory: false,
        initialized: true,
        hasMoreHistory: page.cursor !== null,
        cursor: page.cursor,
      })
      enforceMemoryBudget('drop_newest')
      messageState.version += 1
    } catch (error) {
      updateHistoryState(channelKey, { loadingHistory: false })
      messageState.version += 1
      throw error
    }
  },

  setScrollTop: (
    guildSlug: string,
    channelSlug: string,
    scrollTop: number,
  ): void => {
    const normalizedGuild = guildSlug.trim()
    const normalizedChannel = channelSlug.trim()
    if (!normalizedGuild || !normalizedChannel || !Number.isFinite(scrollTop)) {
      return
    }
    const channelKey = toChannelKey(normalizedGuild, normalizedChannel)
    const normalizedTop = Math.max(0, Math.round(scrollTop))
    const current = readHistoryState(messageState.historyByChannel, channelKey)
    if (current.scrollTop === normalizedTop) return
    updateHistoryState(channelKey, { scrollTop: normalizedTop })
  },

  scrollTopForChannel: (guildSlug: string, channelSlug: string): number => {
    const channelKey = toChannelKey(guildSlug, channelSlug)
    return readHistoryState(messageState.historyByChannel, channelKey).scrollTop
  },

  addPendingNew: (guildSlug: string, channelSlug: string, count = 1): void => {
    const normalizedGuild = guildSlug.trim()
    const normalizedChannel = channelSlug.trim()
    if (!normalizedGuild || !normalizedChannel) return
    const channelKey = toChannelKey(normalizedGuild, normalizedChannel)
    const current = readHistoryState(messageState.historyByChannel, channelKey)
    const increment = Number.isFinite(count)
      ? Math.max(1, Math.floor(count))
      : 1
    updateHistoryState(channelKey, {
      pendingNewCount: current.pendingNewCount + increment,
    })
    messageState.version += 1
  },

  clearPendingNew: (guildSlug: string, channelSlug: string): void => {
    const normalizedGuild = guildSlug.trim()
    const normalizedChannel = channelSlug.trim()
    if (!normalizedGuild || !normalizedChannel) return
    const channelKey = toChannelKey(normalizedGuild, normalizedChannel)
    const current = readHistoryState(messageState.historyByChannel, channelKey)
    if (current.pendingNewCount === 0) return
    updateHistoryState(channelKey, { pendingNewCount: 0 })
    messageState.version += 1
  },

  pendingNewCountForChannel: (
    guildSlug: string,
    channelSlug: string,
  ): number => {
    const channelKey = toChannelKey(guildSlug, channelSlug)
    return readHistoryState(messageState.historyByChannel, channelKey)
      .pendingNewCount
  },

  sendMessage: (
    guildSlug: string,
    channelSlug: string,
    content: string,
    author: ChatAuthorInput,
  ): boolean => {
    const normalizedGuild = guildSlug.trim()
    const normalizedChannel = channelSlug.trim()
    const normalizedContent = normalizeOutboundContent(content)

    if (!normalizedGuild || !normalizedChannel) return false
    if (!author.userId.trim() || !author.username.trim()) return false
    if (!normalizedContent.trim()) return false

    const channelKey = toChannelKey(normalizedGuild, normalizedChannel)
    const nonce = generateClientNonce()
    const optimisticMessageId = `optimistic-${nonce}`
    const optimisticMessage: ChatMessage = {
      id: optimisticMessageId,
      guildSlug: normalizedGuild,
      channelSlug: normalizedChannel,
      authorUserId: author.userId,
      authorUsername: author.username,
      authorDisplayName: author.displayName || author.username,
      authorAvatarColor: author.avatarColor,
      authorRoleColor: author.roleColor || DEFAULT_ROLE_COLOR,
      content: normalizedContent,
      isSystem: false,
      createdAt: new Date().toISOString(),
      optimistic: true,
      clientNonce: nonce,
    }

    messageState.messagesByChannel[channelKey] = sortMessages([
      ...(messageState.messagesByChannel[channelKey] ?? []),
      optimisticMessage,
    ])
    messageState.optimisticByNonce = {
      ...messageState.optimisticByNonce,
      [nonce]: { channelKey, messageId: optimisticMessageId },
    }
    updateHistoryState(channelKey, { initialized: true })
    enforceMemoryBudget()
    messageState.version += 1

    const sent = wsClient.send('c_message_create', {
      guild_slug: normalizedGuild,
      channel_slug: normalizedChannel,
      content: normalizedContent,
      client_nonce: nonce,
    })

    if (sent) return true

    const pending = messageState.optimisticByNonce[nonce]
    if (pending) {
      const channelMessages =
        messageState.messagesByChannel[pending.channelKey] ?? []
      messageState.messagesByChannel[pending.channelKey] =
        channelMessages.filter((message) => message.id !== pending.messageId)
      const { [nonce]: _removed, ...rest } = messageState.optimisticByNonce
      messageState.optimisticByNonce = rest
      messageState.version += 1
    }

    return false
  },

  ingestServerMessage: (incoming: ChatMessage): void => {
    const channelKey = toChannelKey(incoming.guildSlug, incoming.channelSlug)
    const existing = messageState.messagesByChannel[channelKey] ?? []

    if (existing.some((message) => message.id === incoming.id)) {
      messageState.messagesByChannel[channelKey] = existing.map((message) =>
        message.id === incoming.id
          ? { ...incoming, optimistic: false }
          : message,
      )
      enforceMemoryBudget()
      messageState.version += 1
      return
    }

    const nonce = incoming.clientNonce
    if (nonce) {
      const pending = messageState.optimisticByNonce[nonce]
      if (pending && pending.channelKey === channelKey) {
        const replaced = existing.map((message) =>
          message.id === pending.messageId
            ? { ...incoming, optimistic: false }
            : message,
        )
        messageState.messagesByChannel[channelKey] = sortMessages(replaced)
        const { [nonce]: _removed, ...rest } = messageState.optimisticByNonce
        messageState.optimisticByNonce = rest
        updateHistoryState(channelKey, { initialized: true })
        enforceMemoryBudget()
        messageState.version += 1
        return
      }
    }

    messageState.messagesByChannel[channelKey] = sortMessages([
      ...existing,
      { ...incoming, optimistic: false },
    ])
    updateHistoryState(channelKey, { initialized: true })
    enforceMemoryBudget()
    messageState.version += 1
  },

  clearAll: (): void => {
    messageState.activeChannelKey = null
    messageState.messagesByChannel = {}
    messageState.optimisticByNonce = {}
    messageState.historyByChannel = {}
    messageState.version += 1
  },
})

wsClient.subscribe((envelope) => {
  const parsed = parseMessageCreateEnvelope(envelope)
  if (!parsed) return
  messageState.ingestServerMessage(parsed)
})
