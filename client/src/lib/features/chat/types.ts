const DEFAULT_ROLE_COLOR = '#99aab5'

export type ChatMessageReaction = {
  emoji: string
  count: number
  reacted: boolean
  actors?: ChatMessageReactionActor[]
}

export type ChatMessageReactionActor = {
  userId: string
  createdAt: string
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

export type ChatMessageEmbed = {
  id: string
  url: string
  domain: string
  title: string | null
  description: string | null
  thumbnailUrl: string | null
}

export type ChatMessageReactionWire = {
  emoji?: string
  count?: number
  reacted?: boolean
  actors?: ChatMessageReactionActorWire[]
}

export type ChatMessageReactionActorWire = {
  user_id?: string
  created_at?: string
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

export type ChatMessageEmbedWire = {
  id?: string
  url?: string
  domain?: string
  title?: string | null
  description?: string | null
  thumbnail_url?: string | null
}

export type ChatMessage = {
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
  attachments: ChatMessageAttachment[]
  reactions: ChatMessageReaction[]
  embeds: ChatMessageEmbed[]
}

export type ChatMessageWire = {
  id: string
  guild_slug?: string
  channel_slug?: string
  dm_slug?: string
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
  embeds?: ChatMessageEmbedWire[]
}

export function toChatMessageReactions(
  wireReactions: ChatMessageReactionWire[] | undefined,
): ChatMessageReaction[] {
  if (!Array.isArray(wireReactions)) return []

  const normalized: ChatMessageReaction[] = []
  for (const reaction of wireReactions) {
    const emoji = reaction.emoji?.trim()
    if (!emoji) continue
    const count =
      typeof reaction.count === 'number' && Number.isFinite(reaction.count)
        ? Math.max(0, Math.trunc(reaction.count))
        : 0
    const actors = Array.isArray(reaction.actors)
      ? reaction.actors
          .map((actor) => {
            const userId = actor.user_id?.trim()
            const createdAt = actor.created_at?.trim()
            if (!userId || !createdAt) return null
            return { userId, createdAt }
          })
          .filter(
            (actor): actor is { userId: string; createdAt: string } =>
              actor !== null,
          )
      : []
    const normalizedCount = actors.length > 0 ? actors.length : count
    if (normalizedCount <= 0) continue
    normalized.push({
      emoji,
      count: normalizedCount,
      reacted: reaction.reacted === true,
      actors: actors.length > 0 ? actors : undefined,
    })
  }

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

export function toChatMessageEmbeds(
  wireEmbeds: ChatMessageEmbedWire[] | undefined,
): ChatMessageEmbed[] {
  if (!Array.isArray(wireEmbeds)) return []

  return wireEmbeds
    .map((embed) => {
      const id = embed.id?.trim()
      const url = embed.url?.trim()
      const domain = embed.domain?.trim()
      if (!id || !url || !domain) return null

      const title = typeof embed.title === 'string' ? embed.title.trim() : null
      const description =
        typeof embed.description === 'string' ? embed.description.trim() : null
      const thumbnailUrl =
        typeof embed.thumbnail_url === 'string'
          ? embed.thumbnail_url.trim()
          : null
      return {
        id,
        url,
        domain,
        title: title && title.length > 0 ? title : null,
        description: description && description.length > 0 ? description : null,
        thumbnailUrl:
          thumbnailUrl && thumbnailUrl.length > 0 ? thumbnailUrl : null,
      }
    })
    .filter((embed): embed is ChatMessageEmbed => embed !== null)
}

export function toChatMessage(wire: ChatMessageWire): ChatMessage {
  const dmSlug =
    typeof wire.dm_slug === 'string' && wire.dm_slug.trim().length > 0
      ? wire.dm_slug.trim()
      : null
  return {
    id: wire.id,
    guildSlug:
      typeof wire.guild_slug === 'string' ? wire.guild_slug.trim() : '',
    channelSlug:
      typeof wire.channel_slug === 'string' ? wire.channel_slug.trim() : '',
    dmSlug,
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
    embeds: toChatMessageEmbeds(wire.embeds),
  }
}
