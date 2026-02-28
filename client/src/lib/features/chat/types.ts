const DEFAULT_ROLE_COLOR = '#99aab5'

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
  }
}
