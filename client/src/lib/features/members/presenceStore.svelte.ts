import type { GuildMember, PresenceStatus } from '$lib/features/guild/types'

type PresenceUpdateWire = {
  op?: string
  d?: {
    user_id?: string
    status?: PresenceStatus
  }
}

const HEARTBEAT_INTERVAL_MS = 15_000
const RECONNECT_BASE_DELAY_MS = 1_000
const RECONNECT_MAX_DELAY_MS = 15_000

function isPresenceStatus(value: unknown): value is PresenceStatus {
  return value === 'online' || value === 'idle' || value === 'offline'
}

function normalizePresenceStatus(value: unknown): PresenceStatus {
  return isPresenceStatus(value) ? value : 'offline'
}

function buildWebSocketUrl(token: string): string | null {
  if (typeof window === 'undefined') return null
  const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:'
  const host = window.location.host
  return `${protocol}//${host}/ws?token=${encodeURIComponent(token)}`
}

function clearHeartbeatTimer(): void {
  if (presenceState.heartbeatTimer !== null) {
    clearInterval(presenceState.heartbeatTimer)
    presenceState.heartbeatTimer = null
  }
}

function clearReconnectTimer(): void {
  if (presenceState.reconnectTimer !== null) {
    clearTimeout(presenceState.reconnectTimer)
    presenceState.reconnectTimer = null
  }
}

function sendHeartbeat(socket: WebSocket): void {
  if (socket.readyState !== WebSocket.OPEN) return
  socket.send(JSON.stringify({ op: 'heartbeat' }))
}

function startHeartbeat(socket: WebSocket): void {
  clearHeartbeatTimer()
  if (typeof window === 'undefined') return
  presenceState.heartbeatTimer = window.setInterval(() => {
    sendHeartbeat(socket)
  }, HEARTBEAT_INTERVAL_MS)
}

function scheduleReconnect(): void {
  if (typeof window === 'undefined') return
  if (!presenceState.token || presenceState.reconnectTimer !== null) return

  const delay = Math.min(presenceState.reconnectDelayMs, RECONNECT_MAX_DELAY_MS)
  presenceState.reconnectTimer = window.setTimeout(() => {
    presenceState.reconnectTimer = null
    if (
      !presenceState.token ||
      presenceState.socket ||
      presenceState.connecting
    ) {
      return
    }
    openSocket(presenceState.token)
  }, delay)

  presenceState.reconnectDelayMs = Math.min(delay * 2, RECONNECT_MAX_DELAY_MS)
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

function parsePresenceMessage(
  data: unknown,
): { userId: string; status: PresenceStatus } | null {
  if (typeof data !== 'string') return null
  let parsed: unknown
  try {
    parsed = JSON.parse(data)
  } catch {
    return null
  }

  const wire = parsed as PresenceUpdateWire
  if (wire.op !== 'presence_update') return null
  if (!wire.d?.user_id || !isPresenceStatus(wire.d.status)) return null
  return { userId: wire.d.user_id, status: wire.d.status }
}

function openSocket(token: string): void {
  const url = buildWebSocketUrl(token)
  if (!url) return

  clearReconnectTimer()
  presenceState.connecting = true
  const socket = new WebSocket(url)
  presenceState.socket = socket

  socket.onopen = () => {
    if (presenceState.socket !== socket) {
      socket.close()
      return
    }
    presenceState.connecting = false
    presenceState.reconnectDelayMs = RECONNECT_BASE_DELAY_MS
    sendHeartbeat(socket)
    startHeartbeat(socket)
  }

  socket.onmessage = (event) => {
    const update = parsePresenceMessage(event.data)
    if (!update) return
    setPresenceStatus(update.userId, update.status)
  }

  socket.onclose = () => {
    if (presenceState.socket === socket) {
      presenceState.socket = null
    }
    presenceState.connecting = false
    clearHeartbeatTimer()
    scheduleReconnect()
  }

  socket.onerror = () => {
    // onclose handles reconnect behavior
  }
}

export const presenceState = $state({
  statusesByUser: {} as Record<string, PresenceStatus>,
  version: 0,
  socket: null as WebSocket | null,
  connecting: false,
  token: null as string | null,
  heartbeatTimer: null as ReturnType<typeof setInterval> | null,
  reconnectTimer: null as ReturnType<typeof setTimeout> | null,
  reconnectDelayMs: RECONNECT_BASE_DELAY_MS,

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
      (presenceState.socket || presenceState.connecting)
    ) {
      return
    }

    if (presenceState.token !== normalized) {
      presenceState.disconnect()
      presenceState.token = normalized
    }

    if (presenceState.socket || presenceState.connecting) return
    openSocket(normalized)
  },

  disconnect: (): void => {
    clearHeartbeatTimer()
    clearReconnectTimer()
    presenceState.connecting = false
    presenceState.token = null
    presenceState.reconnectDelayMs = RECONNECT_BASE_DELAY_MS
    const socket = presenceState.socket
    presenceState.socket = null
    if (socket) {
      socket.close()
    }
  },
})
