import {
  parseWsEnvelope,
  type WsClientOp,
  type WsEnvelope,
  type WsLifecycleState,
} from './protocol'

const HEARTBEAT_INTERVAL_MS = 15_000
export const RECONNECT_BASE_DELAY_MS = 500
export const RECONNECT_MAX_DELAY_MS = 5_000

type EnvelopeListener = (envelope: WsEnvelope) => void
type LifecycleListener = (state: WsLifecycleState) => void

function buildWebSocketUrl(token: string): string | null {
  if (typeof window === 'undefined') return null
  const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:'
  const host = window.location.host
  return `${protocol}//${host}/ws?token=${encodeURIComponent(token)}`
}

class SharedWsClient {
  private socket: WebSocket | null = null

  private token: string | null = null

  private reconnectTimer: ReturnType<typeof setTimeout> | null = null

  private heartbeatTimer: ReturnType<typeof setInterval> | null = null

  private reconnectDelayMs = RECONNECT_BASE_DELAY_MS

  private lifecycleState: WsLifecycleState = 'disconnected'

  private envelopeListeners = new Set<EnvelopeListener>()

  private lifecycleListeners = new Set<LifecycleListener>()

  private subscribedGuild: string | null = null

  private subscribedChannel: string | null = null

  private subscribedDm: string | null = null

  getLifecycleState(): WsLifecycleState {
    return this.lifecycleState
  }

  subscribe(listener: EnvelopeListener): () => void {
    this.envelopeListeners.add(listener)
    return () => this.envelopeListeners.delete(listener)
  }

  subscribeLifecycle(listener: LifecycleListener): () => void {
    this.lifecycleListeners.add(listener)
    listener(this.lifecycleState)
    return () => this.lifecycleListeners.delete(listener)
  }

  ensureConnected(token: string | null): void {
    const normalizedToken = token?.trim() || null
    if (!normalizedToken) {
      this.disconnect()
      return
    }

    if (this.token !== normalizedToken) {
      this.token = normalizedToken
      this.reconnectDelayMs = RECONNECT_BASE_DELAY_MS
      this.clearReconnectTimer()
      this.closeSocket()
    }

    if (
      this.socket &&
      (this.socket.readyState === WebSocket.OPEN ||
        this.socket.readyState === WebSocket.CONNECTING)
    ) {
      return
    }

    this.openSocket(
      this.lifecycleState === 'reconnecting' ? 'reconnecting' : 'connecting',
    )
  }

  disconnect(): void {
    this.token = null
    this.subscribedGuild = null
    this.subscribedChannel = null
    this.subscribedDm = null
    this.reconnectDelayMs = RECONNECT_BASE_DELAY_MS
    this.clearReconnectTimer()
    this.clearHeartbeatTimer()
    this.closeSocket()
    this.setLifecycle('disconnected')
  }

  setSubscription(guildSlug: string | null, channelSlug: string | null): void {
    const nextGuild = guildSlug?.trim() || null
    const nextChannel = channelSlug?.trim() || null
    if (
      this.subscribedGuild === nextGuild &&
      this.subscribedChannel === nextChannel
    ) {
      return
    }

    const previousGuild = this.subscribedGuild
    const previousChannel = this.subscribedChannel
    this.subscribedGuild = nextGuild
    this.subscribedChannel = nextChannel

    if (!this.isSocketOpen()) return

    if (previousGuild) {
      this.send('c_unsubscribe', {
        guild_slug: previousGuild,
        channel_slug: previousChannel,
      })
    }

    if (nextGuild) {
      this.send('c_subscribe', {
        guild_slug: nextGuild,
        channel_slug: nextChannel,
      })
    }
  }

  setDmSubscription(dmSlug: string | null): void {
    const nextDmSlug = dmSlug?.trim() || null
    if (this.subscribedDm === nextDmSlug) return
    this.subscribedDm = nextDmSlug
    if (!this.isSocketOpen()) return
    this.send('c_dm_subscribe', { dm_slug: nextDmSlug })
  }

  send(op: WsClientOp, d: Record<string, unknown> = {}): boolean {
    if (!this.isSocketOpen()) return false
    this.socket?.send(JSON.stringify({ op, d }))
    return true
  }

  private emitEnvelope(envelope: WsEnvelope): void {
    for (const listener of this.envelopeListeners) {
      listener(envelope)
    }
  }

  private setLifecycle(next: WsLifecycleState): void {
    if (this.lifecycleState === next) return
    this.lifecycleState = next
    for (const listener of this.lifecycleListeners) {
      listener(next)
    }
  }

  private isSocketOpen(): boolean {
    return this.socket?.readyState === WebSocket.OPEN
  }

  private clearReconnectTimer(): void {
    if (this.reconnectTimer !== null) {
      clearTimeout(this.reconnectTimer)
      this.reconnectTimer = null
    }
  }

  private clearHeartbeatTimer(): void {
    if (this.heartbeatTimer !== null) {
      clearInterval(this.heartbeatTimer)
      this.heartbeatTimer = null
    }
  }

  private closeSocket(): void {
    const current = this.socket
    this.socket = null
    if (current) {
      current.close()
    }
  }

  private sendHeartbeat(): void {
    this.send('c_heartbeat')
  }

  private startHeartbeat(): void {
    this.clearHeartbeatTimer()
    if (typeof window === 'undefined') return
    this.heartbeatTimer = window.setInterval(() => {
      this.sendHeartbeat()
    }, HEARTBEAT_INTERVAL_MS)
  }

  private syncSubscription(): void {
    if (this.subscribedGuild) {
      this.send('c_subscribe', {
        guild_slug: this.subscribedGuild,
        channel_slug: this.subscribedChannel,
      })
    }
    this.send('c_dm_subscribe', { dm_slug: this.subscribedDm })
  }

  private scheduleReconnect(): void {
    if (typeof window === 'undefined') return
    if (!this.token || this.reconnectTimer !== null) return

    const delay = Math.min(this.reconnectDelayMs, RECONNECT_MAX_DELAY_MS)
    this.setLifecycle('reconnecting')
    this.reconnectTimer = window.setTimeout(() => {
      this.reconnectTimer = null
      if (!this.token) {
        this.setLifecycle('disconnected')
        return
      }
      if (
        this.socket &&
        (this.socket.readyState === WebSocket.OPEN ||
          this.socket.readyState === WebSocket.CONNECTING)
      ) {
        return
      }
      this.openSocket('reconnecting')
    }, delay)

    this.reconnectDelayMs = Math.min(delay * 2, RECONNECT_MAX_DELAY_MS)
  }

  private openSocket(connectingState: WsLifecycleState): void {
    if (
      typeof window === 'undefined' ||
      typeof WebSocket === 'undefined' ||
      !this.token
    ) {
      return
    }

    const url = buildWebSocketUrl(this.token)
    if (!url) return

    this.clearReconnectTimer()
    this.setLifecycle(connectingState)
    const socket = new WebSocket(url)
    this.socket = socket

    socket.onopen = () => {
      if (this.socket !== socket) {
        socket.close()
        return
      }
      this.reconnectDelayMs = RECONNECT_BASE_DELAY_MS
      this.setLifecycle('connected')
      this.sendHeartbeat()
      this.startHeartbeat()
      this.syncSubscription()
    }

    socket.onmessage = (event) => {
      if (this.socket !== socket) return
      if (typeof event.data !== 'string') return
      let raw: unknown
      try {
        raw = JSON.parse(event.data)
      } catch {
        return
      }
      const envelope = parseWsEnvelope(raw)
      if (!envelope) return
      this.emitEnvelope(envelope)
    }

    socket.onclose = () => {
      if (this.socket === socket) {
        this.socket = null
      }
      this.clearHeartbeatTimer()
      if (!this.token) {
        this.setLifecycle('disconnected')
        return
      }
      this.scheduleReconnect()
    }

    socket.onerror = () => {
      // reconnect is handled by onclose
    }
  }
}

export const wsClient = new SharedWsClient()
