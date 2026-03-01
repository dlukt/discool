export type DmParticipant = {
  userId: string
  username: string
  displayName: string
  avatarColor: string | null
}

export type DmConversation = {
  dmSlug: string
  participant: DmParticipant
  createdAt: string
  updatedAt: string
  lastMessagePreview: string | null
  lastMessageAt: string | null
  hasUnreadActivity: boolean
}

export type DmParticipantWire = {
  user_id?: string
  username?: string
  display_name?: string
  avatar_color?: string | null
}

export type DmConversationWire = {
  dm_slug?: string
  participant?: DmParticipantWire
  created_at?: string
  updated_at?: string
  last_message_preview?: string | null
  last_message_at?: string | null
}

function normalizeDisplayName(participant: DmParticipantWire): string {
  const display = participant.display_name?.trim()
  if (display) return display
  return participant.username?.trim() || 'Unknown user'
}

export function toDmConversation(
  wire: DmConversationWire,
  hasUnreadActivity = false,
): DmConversation | null {
  const dmSlug = wire.dm_slug?.trim()
  const participant = wire.participant
  const participantUserId = participant?.user_id?.trim()
  const participantUsername = participant?.username?.trim()
  if (!dmSlug || !participant || !participantUserId || !participantUsername) {
    return null
  }

  return {
    dmSlug,
    participant: {
      userId: participantUserId,
      username: participantUsername,
      displayName: normalizeDisplayName(participant),
      avatarColor:
        typeof participant.avatar_color === 'string'
          ? participant.avatar_color
          : null,
    },
    createdAt: wire.created_at?.trim() || '',
    updatedAt: wire.updated_at?.trim() || '',
    lastMessagePreview:
      typeof wire.last_message_preview === 'string' &&
      wire.last_message_preview.trim().length > 0
        ? wire.last_message_preview
        : null,
    lastMessageAt:
      typeof wire.last_message_at === 'string' && wire.last_message_at.trim()
        ? wire.last_message_at
        : null,
    hasUnreadActivity,
  }
}
