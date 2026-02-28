const DEFAULT_ROLE_COLOR = '#99aab5'

export type ChatMessageReaction = {
  emoji: string
  count: number
  reacted: boolean
}

export type ChatMessageAttachment = {
  id: string
  storageKey: string
  originalFilename: string
  mimeType: string
  sizeBytes: number
  isImage: boolean
  url: string
}

export type ChatMessageReactionWire = {
  emoji?: string
  count?: number
  reacted?: boolean
}

export type ChatMessageAttachmentWire = {
  id?: string
  storage_key?: string
  original_filename?: string
  mime_type?: string
  size_bytes?: number
  is_image?: boolean
  url?: string
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
  updatedAt: string
  optimistic: boolean
  clientNonce?: string
  attachments: ChatMessageAttachment[]
  reactions: ChatMessageReaction[]
}

export type ChatMessageWire = {
  id: string
  guild_slug: string
  channel_slug: string
  author_user_id: string
  author_username: string
  author_display_name: string
  author_avatar_color?: string | null
  author_role_color?: string
  content: string
  is_system: boolean
  created_at: string
  updated_at?: string
  client_nonce?: string | null
  attachments?: ChatMessageAttachmentWire[]
  reactions?: ChatMessageReactionWire[]
}

export function toChatMessageReactions(
  wireReactions: ChatMessageReactionWire[] | undefined,
): ChatMessageReaction[] {
  if (!Array.isArray(wireReactions)) return []

  const normalized = wireReactions
    .map((reaction) => {
      const emoji = reaction.emoji?.trim()
      if (!emoji) return null
      const count =
        typeof reaction.count === 'number' && Number.isFinite(reaction.count)
          ? Math.max(0, Math.trunc(reaction.count))
          : 0
      if (count <= 0) return null
      return {
        emoji,
        count,
        reacted: reaction.reacted === true,
      }
    })
    .filter((reaction): reaction is ChatMessageReaction => reaction !== null)

  normalized.sort(
    (left, right) =>
      right.count - left.count || left.emoji.localeCompare(right.emoji),
  )
  return normalized
}

export function toChatMessageAttachments(
  wireAttachments: ChatMessageAttachmentWire[] | undefined,
): ChatMessageAttachment[] {
  if (!Array.isArray(wireAttachments)) return []

  return wireAttachments
    .map((attachment) => {
      const id = attachment.id?.trim()
      const storageKey = attachment.storage_key?.trim()
      const originalFilename = attachment.original_filename?.trim()
      const mimeType = attachment.mime_type?.trim()
      const url = attachment.url?.trim()
      if (!id || !storageKey || !originalFilename || !mimeType || !url) {
        return null
      }
      const sizeBytes =
        typeof attachment.size_bytes === 'number' &&
        Number.isFinite(attachment.size_bytes)
          ? Math.max(0, Math.trunc(attachment.size_bytes))
          : 0
      if (sizeBytes <= 0) return null
      return {
        id,
        storageKey,
        originalFilename,
        mimeType,
        sizeBytes,
        isImage:
          attachment.is_image === true ||
          mimeType.toLowerCase().startsWith('image/'),
        url,
      }
    })
    .filter(
      (attachment): attachment is ChatMessageAttachment => attachment !== null,
    )
}

export function toChatMessage(wire: ChatMessageWire): ChatMessage {
  return {
    id: wire.id,
    guildSlug: wire.guild_slug,
    channelSlug: wire.channel_slug,
    authorUserId: wire.author_user_id,
    authorUsername: wire.author_username,
    authorDisplayName: wire.author_display_name || wire.author_username,
    authorAvatarColor:
      typeof wire.author_avatar_color === 'string'
        ? wire.author_avatar_color
        : null,
    authorRoleColor: wire.author_role_color || DEFAULT_ROLE_COLOR,
    content: wire.content,
    isSystem: wire.is_system,
    createdAt: wire.created_at,
    updatedAt: wire.updated_at || wire.created_at,
    optimistic: false,
    clientNonce: wire.client_nonce || undefined,
    attachments: toChatMessageAttachments(wire.attachments),
    reactions: toChatMessageReactions(wire.reactions),
  }
}
