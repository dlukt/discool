import { channelState } from '$lib/features/channel/channelStore.svelte'
import { guildState } from '$lib/features/guild/guildStore.svelte'
import { wsClient } from '$lib/ws/client'
import type { WsEnvelope } from '$lib/ws/protocol'
import { fetchChannelHistory, uploadMessageAttachment } from './messageApi'
import {
  type ChatMessage,
  type ChatMessageReaction,
  toChatMessage,
  toChatMessageReactions,
} from './types'

const DEFAULT_ROLE_COLOR = '#99aab5'
const HISTORY_PAGE_LIMIT = 50
const MAX_ACTIVE_CHANNEL_MESSAGES = 4_000
const MAX_INACTIVE_CHANNEL_MESSAGES = 1_200
const MAX_TOTAL_TIMELINE_MESSAGES = 10_000
const MIN_CHANNEL_RETAINED_MESSAGES = 200
const TYPING_EXPIRY_MS = 5_000

const typingExpiryTimers = new Map<string, ReturnType<typeof setTimeout>>()

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
  updated_at?: string
  client_nonce?: string
  attachments?: unknown
  reactions?: unknown
  embeds?: unknown
}

type MessageDeletePayload = {
  id?: string
  guild_slug?: string
  channel_slug?: string
}

type MessageReactionUpdatePayload = {
  guild_slug?: string
  channel_slug?: string
  message_id?: string
  actor_user_id?: string
  reactions?: unknown
}

type TypingStartPayload = {
  guild_slug?: string
  channel_slug?: string
  user_id?: string
}

type ChannelActivityPayload = {
  guild_slug?: string
  channel_slug?: string
}

export type ChatAuthorInput = {
  userId: string
  username: string
  displayName: string
  avatarColor: string | null
  roleColor: string
}

export type ChatAttachmentUploadInput = {
  file: File
  content: string
  onProgress?: (percentage: number) => void
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

function typingTimerKey(channelKey: string, userId: string): string {
  return `${channelKey}:${userId}`
}

function clearTypingExpiryTimer(timerKey: string): void {
  const existing = typingExpiryTimers.get(timerKey)
  if (!existing) return
  clearTimeout(existing)
  typingExpiryTimers.delete(timerKey)
}

function clearTypingExpiryTimersForChannel(channelKey: string): void {
  const prefix = `${channelKey}:`
  for (const timerKey of typingExpiryTimers.keys()) {
    if (!timerKey.startsWith(prefix)) continue
    clearTypingExpiryTimer(timerKey)
  }
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null
}

function normalizeOutboundContent(content: string): string {
  return content.replace(/\r\n/g, '\n').replace(/\r/g, '\n')
}

function normalizeOutboundEmoji(emoji: string): string {
  return emoji.trim()
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

function parseMessageMutationEnvelope(
  envelope: WsEnvelope,
  expectedOp: 'message_create' | 'message_update',
): ChatMessage | null {
  if (envelope.op !== expectedOp) return null
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
    updated_at: payload.updated_at?.trim() || createdAt,
    client_nonce: payload.client_nonce?.trim() || undefined,
    attachments: Array.isArray(payload.attachments)
      ? (payload.attachments as Array<{
          id?: string
          storage_key?: string
          original_filename?: string
          mime_type?: string
          size_bytes?: number
          is_image?: boolean
          url?: string
        }>)
      : undefined,
    reactions: Array.isArray(payload.reactions)
      ? (payload.reactions as Array<{
          emoji?: string
          count?: number
          reacted?: boolean
        }>)
      : undefined,
    embeds: Array.isArray(payload.embeds)
      ? (payload.embeds as Array<{
          id?: string
          url?: string
          domain?: string
          title?: string | null
          description?: string | null
          thumbnail_url?: string | null
        }>)
      : undefined,
  })
}

function parseMessageCreateEnvelope(envelope: WsEnvelope): ChatMessage | null {
  return parseMessageMutationEnvelope(envelope, 'message_create')
}

function parseMessageUpdateEnvelope(envelope: WsEnvelope): ChatMessage | null {
  return parseMessageMutationEnvelope(envelope, 'message_update')
}

function parseMessageDeleteEnvelope(envelope: WsEnvelope): {
  id: string
  guildSlug: string
  channelSlug: string
} | null {
  if (envelope.op !== 'message_delete') return null
  if (!isRecord(envelope.d)) return null
  const payload = envelope.d as MessageDeletePayload
  const id = payload.id?.trim()
  const guildSlug = payload.guild_slug?.trim()
  const channelSlug = payload.channel_slug?.trim()
  if (!id || !guildSlug || !channelSlug) return null
  return { id, guildSlug, channelSlug }
}

function parseMessageReactionUpdateEnvelope(envelope: WsEnvelope): {
  guildSlug: string
  channelSlug: string
  messageId: string
  actorUserId: string | null
  reactions: ChatMessageReaction[]
} | null {
  if (envelope.op !== 'message_reaction_update') return null
  if (!isRecord(envelope.d)) return null
  const payload = envelope.d as MessageReactionUpdatePayload
  const guildSlug = payload.guild_slug?.trim()
  const channelSlug = payload.channel_slug?.trim()
  const messageId = payload.message_id?.trim()
  if (!guildSlug || !channelSlug || !messageId) return null

  return {
    guildSlug,
    channelSlug,
    messageId,
    actorUserId: payload.actor_user_id?.trim() || null,
    reactions: toChatMessageReactions(
      Array.isArray(payload.reactions)
        ? (payload.reactions as Array<{
            emoji?: string
            count?: number
            reacted?: boolean
          }>)
        : undefined,
    ),
  }
}

function parseTypingStartEnvelope(envelope: WsEnvelope): {
  guildSlug: string
  channelSlug: string
  userId: string
} | null {
  if (envelope.op !== 'typing_start') return null
  if (!isRecord(envelope.d)) return null
  const payload = envelope.d as TypingStartPayload
  const guildSlug = payload.guild_slug?.trim()
  const channelSlug = payload.channel_slug?.trim()
  const userId = payload.user_id?.trim()
  if (!guildSlug || !channelSlug || !userId) return null
  return { guildSlug, channelSlug, userId }
}

function parseChannelActivityEnvelope(envelope: WsEnvelope): {
  guildSlug: string
  channelSlug: string
} | null {
  if (envelope.op !== 'channel_activity') return null
  if (!isRecord(envelope.d)) return null
  const payload = envelope.d as ChannelActivityPayload
  const guildSlug = payload.guild_slug?.trim()
  const channelSlug = payload.channel_slug?.trim()
  if (!guildSlug || !channelSlug) return null
  return { guildSlug, channelSlug }
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
  currentUserId: null as string | null,
  messagesByChannel: {} as Record<string, ChatMessage[]>,
  optimisticByNonce: {} as Record<string, PendingOptimisticEntry>,
  historyByChannel: {} as Record<string, ChannelHistoryState>,
  typingByChannel: {} as Record<string, Record<string, number>>,

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

  typingUserIdsForChannel: (
    guildSlug: string,
    channelSlug: string,
  ): string[] => {
    const key = toChannelKey(guildSlug, channelSlug)
    const typingByUser = messageState.typingByChannel[key]
    if (!typingByUser) return []
    return Object.entries(typingByUser)
      .sort(
        ([leftUserId, leftSeenAt], [rightUserId, rightSeenAt]) =>
          rightSeenAt - leftSeenAt || leftUserId.localeCompare(rightUserId),
      )
      .map(([userId]) => userId)
  },

  clearTypingForChannel: (guildSlug: string, channelSlug: string): void => {
    const normalizedGuild = guildSlug.trim()
    const normalizedChannel = channelSlug.trim()
    if (!normalizedGuild || !normalizedChannel) return
    const key = toChannelKey(normalizedGuild, normalizedChannel)
    if (!messageState.typingByChannel[key]) return
    const { [key]: _ignored, ...rest } = messageState.typingByChannel
    messageState.typingByChannel = rest
    clearTypingExpiryTimersForChannel(key)
    messageState.version += 1
  },

  clearTypingState: (): void => {
    if (Object.keys(messageState.typingByChannel).length === 0) return
    for (const timerKey of typingExpiryTimers.keys()) {
      clearTypingExpiryTimer(timerKey)
    }
    messageState.typingByChannel = {}
    messageState.version += 1
  },

  ingestTypingStart: (
    guildSlug: string,
    channelSlug: string,
    userId: string,
  ): void => {
    const normalizedGuild = guildSlug.trim()
    const normalizedChannel = channelSlug.trim()
    const normalizedUserId = userId.trim()
    if (!normalizedGuild || !normalizedChannel || !normalizedUserId) return
    if (normalizedUserId === messageState.currentUserId) return

    const key = toChannelKey(normalizedGuild, normalizedChannel)
    const now = Date.now()
    const currentByUser = messageState.typingByChannel[key] ?? {}
    messageState.typingByChannel = {
      ...messageState.typingByChannel,
      [key]: {
        ...currentByUser,
        [normalizedUserId]: now,
      },
    }

    const timerKey = typingTimerKey(key, normalizedUserId)
    clearTypingExpiryTimer(timerKey)
    typingExpiryTimers.set(
      timerKey,
      setTimeout(() => {
        const channelTyping = messageState.typingByChannel[key]
        if (!channelTyping) {
          typingExpiryTimers.delete(timerKey)
          return
        }
        const lastSeenAt = channelTyping[normalizedUserId]
        if (typeof lastSeenAt !== 'number' || lastSeenAt > now) {
          typingExpiryTimers.delete(timerKey)
          return
        }
        const { [normalizedUserId]: _removed, ...remainingUsers } =
          channelTyping
        if (Object.keys(remainingUsers).length === 0) {
          const { [key]: _ignored, ...remainingChannels } =
            messageState.typingByChannel
          messageState.typingByChannel = remainingChannels
        } else {
          messageState.typingByChannel = {
            ...messageState.typingByChannel,
            [key]: remainingUsers,
          }
        }
        typingExpiryTimers.delete(timerKey)
        messageState.version += 1
      }, TYPING_EXPIRY_MS),
    )
    messageState.version += 1
  },

  ingestChannelActivity: (guildSlug: string, channelSlug: string): void => {
    const normalizedGuild = guildSlug.trim()
    const normalizedChannel = channelSlug.trim()
    if (!normalizedGuild || !normalizedChannel) return
    const key = toChannelKey(normalizedGuild, normalizedChannel)
    if (messageState.activeChannelKey === key) return
    channelState.setChannelUnreadActivity(
      normalizedGuild,
      normalizedChannel,
      true,
    )
    guildState.setGuildUnreadActivity(
      normalizedGuild,
      channelState.hasGuildUnreadActivity(normalizedGuild),
    )
  },

  isChannelUnread: (guildSlug: string, channelSlug: string): boolean => {
    const normalizedGuild = guildSlug.trim()
    const normalizedChannel = channelSlug.trim()
    if (!normalizedGuild || !normalizedChannel) return false
    return channelState
      .orderedChannelsForGuild(normalizedGuild)
      .some(
        (channel) =>
          channel.slug === normalizedChannel &&
          channel.hasUnreadActivity === true,
      )
  },

  unreadChannelSlugsForGuild: (guildSlug: string): string[] => {
    const normalizedGuild = guildSlug.trim()
    if (!normalizedGuild) return []
    return channelState
      .orderedChannelsForGuild(normalizedGuild)
      .filter((channel) => channel.hasUnreadActivity === true)
      .map((channel) => channel.slug)
  },

  setActiveChannel: (guildSlug: string, channelSlug: string): void => {
    const normalizedGuild = guildSlug.trim()
    const normalizedChannel = channelSlug.trim()
    if (!normalizedGuild || !normalizedChannel) return
    const nextKey = toChannelKey(normalizedGuild, normalizedChannel)
    if (messageState.activeChannelKey === nextKey) return

    const previousKey = messageState.activeChannelKey
    if (previousKey) {
      const separatorIndex = previousKey.indexOf(':')
      if (separatorIndex > 0) {
        const previousGuild = previousKey.slice(0, separatorIndex)
        const previousChannel = previousKey.slice(separatorIndex + 1)
        messageState.clearTypingForChannel(previousGuild, previousChannel)
      }
    }
    messageState.activeChannelKey = nextKey
    channelState.setChannelUnreadActivity(
      normalizedGuild,
      normalizedChannel,
      false,
    )
    guildState.setGuildUnreadActivity(
      normalizedGuild,
      channelState.hasGuildUnreadActivity(normalizedGuild),
    )
    if (enforceMemoryBudget()) {
      messageState.version += 1
    }
  },

  setCurrentUser: (userId: string | null): void => {
    const normalized = userId?.trim() || null
    if (messageState.currentUserId === normalized) return
    messageState.currentUserId = normalized
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
    const now = new Date().toISOString()
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
      createdAt: now,
      updatedAt: now,
      optimistic: true,
      clientNonce: nonce,
      attachments: [],
      reactions: [],
      embeds: [],
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

  sendTypingStart: (guildSlug: string, channelSlug: string): boolean => {
    const normalizedGuild = guildSlug.trim()
    const normalizedChannel = channelSlug.trim()
    if (!normalizedGuild || !normalizedChannel) return false
    return wsClient.send('c_typing_start', {
      guild_slug: normalizedGuild,
      channel_slug: normalizedChannel,
    })
  },

  uploadAttachment: async (
    guildSlug: string,
    channelSlug: string,
    input: ChatAttachmentUploadInput,
  ): Promise<void> => {
    const normalizedGuild = guildSlug.trim()
    const normalizedChannel = channelSlug.trim()
    if (!normalizedGuild || !normalizedChannel) {
      throw new Error('guildSlug and channelSlug are required')
    }

    const channelKey = toChannelKey(normalizedGuild, normalizedChannel)
    const normalizedContent = normalizeOutboundContent(input.content)
    const uploaded = await uploadMessageAttachment(
      normalizedGuild,
      normalizedChannel,
      {
        file: input.file,
        content: normalizedContent,
        clientNonce: generateClientNonce(),
        onProgress: input.onProgress,
      },
    )
    messageState.ingestServerMessage(uploaded)
    updateHistoryState(channelKey, { initialized: true })
  },

  sendMessageUpdate: (
    guildSlug: string,
    channelSlug: string,
    messageId: string,
    content: string,
  ): boolean => {
    const normalizedGuild = guildSlug.trim()
    const normalizedChannel = channelSlug.trim()
    const normalizedMessageId = messageId.trim()
    const normalizedContent = normalizeOutboundContent(content)

    if (!normalizedGuild || !normalizedChannel || !normalizedMessageId)
      return false
    if (!normalizedContent.trim()) return false

    return wsClient.send('c_message_update', {
      guild_slug: normalizedGuild,
      channel_slug: normalizedChannel,
      message_id: normalizedMessageId,
      content: normalizedContent,
    })
  },

  sendMessageDelete: (
    guildSlug: string,
    channelSlug: string,
    messageId: string,
  ): boolean => {
    const normalizedGuild = guildSlug.trim()
    const normalizedChannel = channelSlug.trim()
    const normalizedMessageId = messageId.trim()

    if (!normalizedGuild || !normalizedChannel || !normalizedMessageId)
      return false

    return wsClient.send('c_message_delete', {
      guild_slug: normalizedGuild,
      channel_slug: normalizedChannel,
      message_id: normalizedMessageId,
    })
  },

  sendMessageReactionToggle: (
    guildSlug: string,
    channelSlug: string,
    messageId: string,
    emoji: string,
  ): boolean => {
    const normalizedGuild = guildSlug.trim()
    const normalizedChannel = channelSlug.trim()
    const normalizedMessageId = messageId.trim()
    const normalizedEmoji = normalizeOutboundEmoji(emoji)

    if (
      !normalizedGuild ||
      !normalizedChannel ||
      !normalizedMessageId ||
      !normalizedEmoji
    ) {
      return false
    }

    return wsClient.send('c_message_reaction_toggle', {
      guild_slug: normalizedGuild,
      channel_slug: normalizedChannel,
      message_id: normalizedMessageId,
      emoji: normalizedEmoji,
    })
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

  ingestUpdatedMessage: (incoming: ChatMessage): void => {
    const channelKey = toChannelKey(incoming.guildSlug, incoming.channelSlug)
    const existing = messageState.messagesByChannel[channelKey] ?? []
    if (!existing.some((message) => message.id === incoming.id)) return

    messageState.messagesByChannel[channelKey] = existing.map((message) =>
      message.id === incoming.id
        ? {
            ...incoming,
            optimistic: false,
            clientNonce: message.clientNonce ?? incoming.clientNonce,
          }
        : message,
    )
    messageState.version += 1
  },

  ingestDeletedMessage: (
    guildSlug: string,
    channelSlug: string,
    messageId: string,
  ): void => {
    const channelKey = toChannelKey(guildSlug, channelSlug)
    const existing = messageState.messagesByChannel[channelKey] ?? []
    const removed = existing.find((message) => message.id === messageId)
    if (!removed) return

    messageState.messagesByChannel[channelKey] = existing.filter(
      (message) => message.id !== messageId,
    )
    if (removed.clientNonce) {
      const { [removed.clientNonce]: _removed, ...rest } =
        messageState.optimisticByNonce
      messageState.optimisticByNonce = rest
    }
    messageState.version += 1
  },

  ingestReactionUpdate: (
    guildSlug: string,
    channelSlug: string,
    messageId: string,
    reactions: ChatMessageReaction[],
    actorUserId: string | null,
  ): void => {
    const channelKey = toChannelKey(guildSlug, channelSlug)
    const existing = messageState.messagesByChannel[channelKey] ?? []
    if (!existing.some((message) => message.id === messageId)) return

    const preserveReactedFromExisting = Boolean(
      actorUserId &&
        messageState.currentUserId &&
        actorUserId !== messageState.currentUserId,
    )

    messageState.messagesByChannel[channelKey] = existing.map((message) => {
      if (message.id !== messageId) return message
      if (!preserveReactedFromExisting) {
        return { ...message, reactions }
      }

      const existingByEmoji = new Map(
        message.reactions.map((reaction) => [reaction.emoji, reaction]),
      )
      const merged = reactions.map((reaction) => ({
        ...reaction,
        reacted: existingByEmoji.get(reaction.emoji)?.reacted ?? false,
      }))
      return { ...message, reactions: merged }
    })
    messageState.version += 1
  },

  clearAll: (): void => {
    for (const timerKey of typingExpiryTimers.keys()) {
      clearTypingExpiryTimer(timerKey)
    }
    messageState.activeChannelKey = null
    messageState.currentUserId = null
    messageState.messagesByChannel = {}
    messageState.optimisticByNonce = {}
    messageState.historyByChannel = {}
    messageState.typingByChannel = {}
    messageState.version += 1
  },
})

wsClient.subscribe((envelope) => {
  const typingStart = parseTypingStartEnvelope(envelope)
  if (typingStart) {
    messageState.ingestTypingStart(
      typingStart.guildSlug,
      typingStart.channelSlug,
      typingStart.userId,
    )
    return
  }

  const channelActivity = parseChannelActivityEnvelope(envelope)
  if (channelActivity) {
    messageState.ingestChannelActivity(
      channelActivity.guildSlug,
      channelActivity.channelSlug,
    )
    return
  }

  const created = parseMessageCreateEnvelope(envelope)
  if (created) {
    messageState.ingestServerMessage(created)
    return
  }

  const updated = parseMessageUpdateEnvelope(envelope)
  if (updated) {
    messageState.ingestUpdatedMessage(updated)
    return
  }

  const deleted = parseMessageDeleteEnvelope(envelope)
  if (deleted) {
    messageState.ingestDeletedMessage(
      deleted.guildSlug,
      deleted.channelSlug,
      deleted.id,
    )
    return
  }

  const reactionUpdate = parseMessageReactionUpdateEnvelope(envelope)
  if (reactionUpdate) {
    messageState.ingestReactionUpdate(
      reactionUpdate.guildSlug,
      reactionUpdate.channelSlug,
      reactionUpdate.messageId,
      reactionUpdate.reactions,
      reactionUpdate.actorUserId,
    )
  }
})
