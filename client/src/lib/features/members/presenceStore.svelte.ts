import type { GuildMember, PresenceStatus } from '$lib/features/guild/types'
import { wsClient } from '$lib/ws/client'
import type { WsEnvelope } from '$lib/ws/protocol'

type PresencePayload = {
  user_id?: string
  status?: PresenceStatus
}

function isPresenceStatus(value: unknown): value is PresenceStatus {
  return value === 'online' || value === 'idle' || value === 'offline'
}

function normalizePresenceStatus(value: unknown): PresenceStatus {
  return isPresenceStatus(value) ? value : 'offline'
}

function setPresenceStatus(userId: string, status: PresenceStatus): void {
  if (!userId) return
  if (presenceState.statusesByUser[userId] === status) return
  presenceState.statusesByUser = {
    ...presenceState.statusesByUser,
    [userId]: status,
  }
  presenceState.version += 1
}

function parsePresenceEnvelope(
  envelope: WsEnvelope,
): { userId: string; status: PresenceStatus } | null {
  if (envelope.op !== 'presence_update') return null
  const payload = envelope.d as PresencePayload
  if (!payload.user_id || !isPresenceStatus(payload.status)) return null
  return { userId: payload.user_id, status: payload.status }
}

export const presenceState = $state({
  statusesByUser: {} as Record<string, PresenceStatus>,
  version: 0,
  token: null as string | null,
  activeGuild: null as string | null,
  activeChannel: null as string | null,

  seedFromMembers: (members: GuildMember[]): void => {
    let changed = false
    const next = { ...presenceState.statusesByUser }
    for (const member of members) {
      const status = normalizePresenceStatus(member.presenceStatus)
      if (next[member.userId] !== status) {
        next[member.userId] = status
        changed = true
      }
    }
    if (changed) {
      presenceState.statusesByUser = next
      presenceState.version += 1
    }
  },

  statusFor: (
    userId: string,
    fallback: PresenceStatus = 'offline',
  ): PresenceStatus => presenceState.statusesByUser[userId] ?? fallback,

  ensureConnected: (token: string | null): void => {
    const normalized = token?.trim() || null
    if (!normalized) {
      presenceState.disconnect()
      return
    }

    if (
      presenceState.token === normalized &&
      wsClient.getLifecycleState() !== 'disconnected'
    ) {
      return
    }

    if (presenceState.token !== normalized) {
      presenceState.token = normalized
    }

    wsClient.ensureConnected(normalized)
  },

  setRouting: (guildSlug: string | null, channelSlug: string | null): void => {
    const nextGuild = guildSlug?.trim() || null
    const nextChannel = channelSlug?.trim() || null
    if (
      presenceState.activeGuild === nextGuild &&
      presenceState.activeChannel === nextChannel
    ) {
      return
    }
    presenceState.activeGuild = nextGuild
    presenceState.activeChannel = nextChannel
    wsClient.setSubscription(nextGuild, nextChannel)
  },

  clearRouting: (): void => {
    presenceState.activeGuild = null
    presenceState.activeChannel = null
    wsClient.setSubscription(null, null)
  },

  disconnect: (): void => {
    presenceState.token = null
    presenceState.activeGuild = null
    presenceState.activeChannel = null
    wsClient.disconnect()
  },
})

wsClient.subscribe((envelope) => {
  const update = parsePresenceEnvelope(envelope)
  if (!update) return
  setPresenceStatus(update.userId, update.status)
})
