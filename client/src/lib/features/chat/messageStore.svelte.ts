import { wsClient } from '$lib/ws/client'
import type { WsEnvelope } from '$lib/ws/protocol'

const DEFAULT_ROLE_COLOR = '#99aab5'
const MAX_CHANNEL_TIMELINE_ITEMS = 500

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

export type ChatMessage = {
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
}

type PendingOptimisticEntry = {
  channelKey: string
  messageId: string
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

function trimTimeline(messages: ChatMessage[]): ChatMessage[] {
  if (messages.length <= MAX_CHANNEL_TIMELINE_ITEMS) return messages
  return messages.slice(messages.length - MAX_CHANNEL_TIMELINE_ITEMS)
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
  const content = payload.content ?? ''

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

  const authorDisplayName =
    payload.author_display_name?.trim() || authorUsername

  return {
    id,
    guildSlug,
    channelSlug,
    authorUserId,
    authorUsername,
    authorDisplayName,
    authorAvatarColor:
      typeof payload.author_avatar_color === 'string'
        ? payload.author_avatar_color
        : null,
    authorRoleColor: payload.author_role_color?.trim() || DEFAULT_ROLE_COLOR,
    content,
    isSystem: payload.is_system === true,
    createdAt,
    optimistic: false,
    clientNonce: payload.client_nonce?.trim() || undefined,
  }
}

export const messageState = $state({
  version: 0,
  messagesByChannel: {} as Record<string, ChatMessage[]>,
  optimisticByNonce: {} as Record<string, PendingOptimisticEntry>,

  timeline: (guildSlug: string, channelSlug: string): ChatMessage[] => {
    const key = toChannelKey(guildSlug, channelSlug)
    return [...(messageState.messagesByChannel[key] ?? [])]
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

    messageState.messagesByChannel[channelKey] = trimTimeline([
      ...(messageState.messagesByChannel[channelKey] ?? []),
      optimisticMessage,
    ])
    messageState.optimisticByNonce = {
      ...messageState.optimisticByNonce,
      [nonce]: { channelKey, messageId: optimisticMessageId },
    }
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
        messageState.messagesByChannel[channelKey] = trimTimeline(replaced)
        const { [nonce]: _removed, ...rest } = messageState.optimisticByNonce
        messageState.optimisticByNonce = rest
        messageState.version += 1
        return
      }
    }

    const nextMessages = [...existing, { ...incoming, optimistic: false }].sort(
      (left, right) =>
        left.createdAt.localeCompare(right.createdAt) ||
        left.id.localeCompare(right.id),
    )
    messageState.messagesByChannel[channelKey] = trimTimeline(nextMessages)
    messageState.version += 1
  },

  clearAll: (): void => {
    messageState.messagesByChannel = {}
    messageState.optimisticByNonce = {}
    messageState.version += 1
  },
})

wsClient.subscribe((envelope) => {
  const parsed = parseMessageCreateEnvelope(envelope)
  if (!parsed) return
  messageState.ingestServerMessage(parsed)
})
