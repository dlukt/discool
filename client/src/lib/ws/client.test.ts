import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import { RECONNECT_MAX_DELAY_MS, wsClient } from './client'
import type { WsLifecycleState } from './protocol'

class MockWebSocket {
  static readonly CONNECTING = 0

  static readonly OPEN = 1

  static readonly CLOSING = 2

  static readonly CLOSED = 3

  static instances: MockWebSocket[] = []

  readonly url: string

  readyState = MockWebSocket.CONNECTING

  onopen: ((event: Event) => void) | null = null

  onmessage: ((event: MessageEvent) => void) | null = null

  onerror: ((event: Event) => void) | null = null

  onclose: ((event: CloseEvent) => void) | null = null

  sentPayloads: string[] = []

  constructor(url: string) {
    this.url = url
    MockWebSocket.instances.push(this)
  }

  send(payload: string): void {
    this.sentPayloads.push(payload)
  }

  close(): void {
    this.readyState = MockWebSocket.CLOSED
    this.onclose?.(new Event('close') as CloseEvent)
  }

  triggerOpen(): void {
    this.readyState = MockWebSocket.OPEN
    this.onopen?.(new Event('open'))
  }

  triggerClose(): void {
    this.readyState = MockWebSocket.CLOSED
    this.onclose?.(new Event('close') as CloseEvent)
  }
}

function latestSocket(): MockWebSocket {
  const socket = MockWebSocket.instances.at(-1)
  if (!socket) {
    throw new Error('Expected websocket instance to exist')
  }
  return socket
}

function collectTimeoutDelays(
  setTimeoutSpy: ReturnType<typeof vi.spyOn>,
): number[] {
  return setTimeoutSpy.mock.calls
    .map((call: unknown[]) => Number(call[1]))
    .filter((delay: number) => Number.isFinite(delay))
}

describe('wsClient', () => {
  beforeEach(() => {
    vi.useFakeTimers()
    vi.stubGlobal('WebSocket', MockWebSocket as unknown as typeof WebSocket)
    MockWebSocket.instances = []
    wsClient.disconnect()
  })

  afterEach(() => {
    wsClient.disconnect()
    vi.restoreAllMocks()
    vi.unstubAllGlobals()
    vi.useRealTimers()
  })

  it('reconnects with exponential backoff capped at 5 seconds', () => {
    const lifecycleStates: WsLifecycleState[] = []
    const setTimeoutSpy = vi.spyOn(window, 'setTimeout')
    const unsubscribe = wsClient.subscribeLifecycle((state) => {
      lifecycleStates.push(state)
    })

    wsClient.ensureConnected('session-token')
    const initialSocket = latestSocket()
    initialSocket.triggerOpen()
    initialSocket.triggerClose()

    const observedDelays: number[] = []
    for (let attempt = 0; attempt < 5; attempt += 1) {
      const delay = collectTimeoutDelays(setTimeoutSpy)[attempt]
      observedDelays.push(delay)
      vi.advanceTimersByTime(delay)
      const reconnectSocket = latestSocket()
      reconnectSocket.triggerClose()
    }

    expect(observedDelays).toEqual([
      500,
      1000,
      2000,
      4000,
      RECONNECT_MAX_DELAY_MS,
    ])
    expect(Math.max(...observedDelays)).toBe(RECONNECT_MAX_DELAY_MS)
    expect(lifecycleStates).toContain('connecting')
    expect(lifecycleStates).toContain('connected')
    expect(lifecycleStates).toContain('reconnecting')

    unsubscribe()
  })

  it('uses c_ prefixed subscribe and unsubscribe operations', () => {
    wsClient.ensureConnected('session-token')
    const socket = latestSocket()
    socket.triggerOpen()

    wsClient.setSubscription('lobby', 'general')
    wsClient.setSubscription('lobby', 'random')
    wsClient.setSubscription(null, null)

    const ops = socket.sentPayloads
      .map((payload) => JSON.parse(payload) as { op: string })
      .map((parsed) => parsed.op)

    expect(ops).toContain('c_heartbeat')
    expect(ops).toContain('c_subscribe')
    expect(ops).toContain('c_unsubscribe')
  })
})
