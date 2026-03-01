import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

const { wsSend, envelopeListeners, toastState } = vi.hoisted(() => {
  const envelopeListeners = new Set<(envelope: unknown) => void>()
  const wsSend = vi.fn(() => true)
  const toastState = {
    show: vi.fn(),
  }
  return { wsSend, envelopeListeners, toastState }
})

vi.mock('$lib/ws/client', () => ({
  wsClient: {
    send: wsSend,
    subscribe: vi.fn((listener: (envelope: unknown) => void) => {
      envelopeListeners.add(listener)
      return () => envelopeListeners.delete(listener)
    }),
  },
}))

vi.mock('$lib/feedback/toastStore.svelte', () => ({
  toastState,
}))

import {
  VOICE_FAILED_MESSAGE,
  VOICE_RETRY_MESSAGE,
  voiceState,
} from './voiceStore.svelte'

function emitEnvelope(envelope: unknown): void {
  for (const listener of envelopeListeners) {
    listener(envelope)
  }
}

describe('voiceState', () => {
  beforeEach(() => {
    vi.useFakeTimers()
    wsSend.mockReset()
    wsSend.mockReturnValue(true)
    toastState.show.mockReset()
    voiceState.clearActiveChannel()
  })

  afterEach(() => {
    vi.useRealTimers()
  })

  it('sends c_voice_join when a voice channel is activated', () => {
    voiceState.activateVoiceChannel('lobby', 'voice-room')
    expect(wsSend).toHaveBeenCalledWith('c_voice_join', {
      guild_slug: 'lobby',
      channel_slug: 'voice-room',
    })
    expect(voiceState.status).toBe('connecting')
  })

  it('shows retry copy then terminal failure copy when retries are exhausted', () => {
    voiceState.activateVoiceChannel('lobby', 'voice-room')

    emitEnvelope({
      op: 'error',
      d: {
        code: 'INTERNAL_ERROR',
        message: 'voice signaling failed',
        details: {
          op: 'c_voice_join',
        },
      },
    })

    expect(voiceState.status).toBe('retrying')
    expect(voiceState.statusMessage).toBe(VOICE_RETRY_MESSAGE)

    vi.advanceTimersByTime(400)
    expect(wsSend).toHaveBeenCalledTimes(2)

    emitEnvelope({
      op: 'error',
      d: {
        code: 'INTERNAL_ERROR',
        message: 'voice signaling failed again',
        details: {
          op: 'c_voice_join',
        },
      },
    })

    expect(voiceState.status).toBe('failed')
    expect(voiceState.statusMessage).toBe(VOICE_FAILED_MESSAGE)
    expect(toastState.show).toHaveBeenCalledWith({
      variant: 'error',
      message: VOICE_FAILED_MESSAGE,
    })
  })

  it('does not mark voice as connected from server state alone', () => {
    voiceState.activateVoiceChannel('lobby', 'voice-room')

    emitEnvelope({
      op: 'voice_connection_state',
      d: {
        guild_slug: 'lobby',
        channel_slug: 'voice-room',
        state: 'connected',
      },
    })

    expect(voiceState.status).toBe('connecting')
    expect(voiceState.statusMessage).toBeNull()
    expect(voiceState.joinLatencyMs).toBeNull()
  })
})
