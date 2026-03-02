import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

const {
  wsSend,
  envelopeListeners,
  toastState,
  loadParticipantVolumePreferences,
  saveParticipantVolumePreferences,
} = vi.hoisted(() => {
  const envelopeListeners = new Set<(envelope: unknown) => void>()
  const wsSend = vi.fn(
    (_op: string, _payload?: Record<string, unknown>) => true,
  )
  const toastState = {
    show: vi.fn(),
  }
  const loadParticipantVolumePreferences = vi.fn(
    async (_viewerUserId: string): Promise<Record<string, number>> => ({}),
  )
  const saveParticipantVolumePreferences = vi.fn(
    async (
      _viewerUserId: string,
      _preferencesByParticipant: Record<string, number>,
    ): Promise<void> => {},
  )
  return {
    wsSend,
    envelopeListeners,
    toastState,
    loadParticipantVolumePreferences,
    saveParticipantVolumePreferences,
  }
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

vi.mock('./participantVolumeStore.svelte', () => ({
  loadParticipantVolumePreferences,
  saveParticipantVolumePreferences,
}))

import {
  VOICE_CONNECTION_LOST_MESSAGE,
  VOICE_FAILED_MESSAGE,
  VOICE_RECONNECTING_MESSAGE,
  VOICE_RETRY_MESSAGE,
  voiceState,
} from './voiceStore.svelte'

function emitEnvelope(envelope: unknown): void {
  for (const listener of envelopeListeners) {
    listener(envelope)
  }
}

async function flushMicrotasks(iterations = 4): Promise<void> {
  for (let index = 0; index < iterations; index += 1) {
    await Promise.resolve()
  }
}

describe('voiceState', () => {
  beforeEach(async () => {
    vi.useFakeTimers()
    voiceState.clearActiveChannel()
    wsSend.mockReset()
    wsSend.mockReturnValue(true)
    toastState.show.mockReset()
    loadParticipantVolumePreferences.mockReset()
    loadParticipantVolumePreferences.mockResolvedValue({})
    saveParticipantVolumePreferences.mockReset()
    saveParticipantVolumePreferences.mockResolvedValue(undefined)
    await voiceState.initializeParticipantVolumes(null)
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

  it('switches channels by leaving old context before joining the target channel', () => {
    voiceState.activateVoiceChannel('lobby', 'voice-a')
    wsSend.mockClear()

    voiceState.activateVoiceChannel('lobby', 'voice-b')

    expect(wsSend.mock.calls).toEqual([
      [
        'c_voice_leave',
        {
          guild_slug: 'lobby',
          channel_slug: 'voice-a',
        },
      ],
      [
        'c_voice_join',
        {
          guild_slug: 'lobby',
          channel_slug: 'voice-b',
        },
      ],
    ])
    expect(voiceState.activeChannelSlug).toBe('voice-b')
    expect(voiceState.status).toBe('connecting')
  })

  it('is idempotent for repeated activation of the current active voice channel', () => {
    voiceState.activateVoiceChannel('lobby', 'voice-room')
    voiceState.activateVoiceChannel('lobby', 'voice-room')

    const joinCalls = wsSend.mock.calls.filter(
      (call) => call[0] === 'c_voice_join',
    )
    const leaveCalls = wsSend.mock.calls.filter(
      (call) => call[0] === 'c_voice_leave',
    )
    expect(joinCalls).toHaveLength(1)
    expect(leaveCalls).toHaveLength(0)
  })

  it('ignores stale join errors tied to a previous channel after switching', () => {
    voiceState.activateVoiceChannel('lobby', 'voice-a')
    voiceState.activateVoiceChannel('lobby', 'voice-b')

    emitEnvelope({
      op: 'error',
      d: {
        code: 'INTERNAL_ERROR',
        message: 'voice signaling failed',
        details: {
          op: 'c_voice_join',
          guild_slug: 'lobby',
          channel_slug: 'voice-a',
        },
      },
    })

    expect(voiceState.activeChannelSlug).toBe('voice-b')
    expect(voiceState.status).toBe('connecting')
    expect(voiceState.statusMessage).toBeNull()
  })

  it('ingests voice_state_update envelopes and exposes participant selectors', () => {
    emitEnvelope({
      op: 'voice_state_update',
      d: {
        guild_slug: 'lobby',
        channel_slug: 'voice-room',
        participant_count: 2,
        participants: [
          {
            user_id: 'user-1',
            username: 'alice',
            display_name: 'Alice',
            avatar_color: '#3366ff',
            audio_stream_id: 'stream-1',
            is_muted: false,
            is_deafened: false,
            is_speaking: true,
          },
          {
            user_id: 'user-2',
            username: 'bob',
            display_name: null,
            avatar_color: '#ff6633',
            audio_stream_id: 'stream-2',
            is_muted: true,
            is_deafened: false,
            is_speaking: false,
          },
        ],
      },
    })

    expect(voiceState.participantCountForChannel('lobby', 'voice-room')).toBe(2)
    expect(voiceState.participantsForChannel('lobby', 'voice-room')).toEqual([
      {
        userId: 'user-1',
        username: 'alice',
        displayName: 'Alice',
        avatarColor: '#3366ff',
        audioStreamId: 'stream-1',
        isMuted: false,
        isDeafened: false,
        isSpeaking: true,
        volumePercent: 100,
        volumeScalar: 1,
      },
      {
        userId: 'user-2',
        username: 'bob',
        displayName: null,
        avatarColor: '#ff6633',
        audioStreamId: 'stream-2',
        isMuted: true,
        isDeafened: false,
        isSpeaking: false,
        volumePercent: 100,
        volumeScalar: 1,
      },
    ])

    voiceState.activateVoiceChannel('lobby', 'voice-room')
    expect(voiceState.activeChannelParticipants()).toHaveLength(2)
  })

  it('loads persisted volumes and applies clamping/default fallback', async () => {
    loadParticipantVolumePreferences.mockResolvedValue({
      'user-2': 175,
      'user-3': -12,
    })

    await voiceState.initializeParticipantVolumes('viewer-1')

    expect(loadParticipantVolumePreferences).toHaveBeenCalledWith('viewer-1')
    expect(voiceState.participantVolumeForUser('user-2').volumePercent).toBe(
      175,
    )
    expect(voiceState.participantVolumeForUser('user-3').volumePercent).toBe(0)
    expect(
      voiceState.participantVolumeForUser('missing-user').volumePercent,
    ).toBe(100)
  })

  it('persists participant volume changes scoped to current viewer identity', async () => {
    await voiceState.initializeParticipantVolumes('viewer-1')

    voiceState.setParticipantVolume('user-2', 220)
    await flushMicrotasks()

    expect(saveParticipantVolumePreferences).toHaveBeenCalledWith('viewer-1', {
      'user-2': 200,
    })

    loadParticipantVolumePreferences.mockResolvedValue({})
    await voiceState.initializeParticipantVolumes('viewer-2')

    expect(loadParticipantVolumePreferences).toHaveBeenCalledWith('viewer-2')
    expect(voiceState.participantVolumeForUser('user-2').volumePercent).toBe(
      100,
    )
  })

  it('queues persistence snapshots without leaking across owner resets', async () => {
    const firstSaveGate = (() => {
      let resolve: (() => void) | null = null
      const promise = new Promise<void>((complete) => {
        resolve = () => complete()
      })
      return {
        promise,
        release: () => {
          if (!resolve) {
            throw new Error('Expected first save resolver to be assigned')
          }
          resolve()
        },
      }
    })()
    saveParticipantVolumePreferences
      .mockImplementationOnce(() => firstSaveGate.promise)
      .mockResolvedValue(undefined)

    await voiceState.initializeParticipantVolumes('viewer-1')

    voiceState.setParticipantVolume('user-2', 110)
    voiceState.setParticipantVolume('user-3', 120)
    await voiceState.initializeParticipantVolumes(null)

    await flushMicrotasks()
    firstSaveGate.release()
    await flushMicrotasks(8)

    expect(saveParticipantVolumePreferences).toHaveBeenNthCalledWith(
      1,
      'viewer-1',
      {
        'user-2': 110,
      },
    )
    expect(saveParticipantVolumePreferences).toHaveBeenNthCalledWith(
      2,
      'viewer-1',
      {
        'user-2': 110,
        'user-3': 120,
      },
    )
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

  it('enforces deafen implying mute and blocks unmute while deafened', () => {
    voiceState.activateVoiceChannel('lobby', 'voice-room')
    voiceState.status = 'connected'

    voiceState.toggleDeafen()
    expect(voiceState.isDeafened).toBe(true)
    expect(voiceState.isMuted).toBe(true)

    voiceState.toggleMute()
    expect(voiceState.isMuted).toBe(true)

    voiceState.toggleDeafen()
    expect(voiceState.isDeafened).toBe(false)
    expect(voiceState.isMuted).toBe(true)

    voiceState.toggleMute()
    expect(voiceState.isMuted).toBe(false)
  })

  it('sends c_voice_state_update when local control state changes while connected', () => {
    voiceState.activateVoiceChannel('lobby', 'voice-room')
    voiceState.status = 'connected'
    wsSend.mockClear()

    voiceState.toggleMute()
    expect(wsSend).toHaveBeenCalledWith('c_voice_state_update', {
      guild_slug: 'lobby',
      channel_slug: 'voice-room',
      is_muted: true,
      is_deafened: false,
      is_speaking: false,
    })

    voiceState.toggleDeafen()
    expect(wsSend).toHaveBeenCalledWith('c_voice_state_update', {
      guild_slug: 'lobby',
      channel_slug: 'voice-room',
      is_muted: true,
      is_deafened: true,
      is_speaking: false,
    })
  })

  it('sends c_voice_leave and resets voice control state on disconnect', () => {
    voiceState.activateVoiceChannel('lobby', 'voice-room')
    voiceState.status = 'connected'
    voiceState.toggleMute()
    wsSend.mockClear()

    voiceState.disconnect()

    expect(wsSend).toHaveBeenCalledWith('c_voice_leave', {
      guild_slug: 'lobby',
      channel_slug: 'voice-room',
    })
    expect(voiceState.status).toBe('idle')
    expect(voiceState.activeGuildSlug).toBeNull()
    expect(voiceState.activeChannelSlug).toBeNull()
    expect(voiceState.isMuted).toBe(false)
    expect(voiceState.isDeafened).toBe(false)

    voiceState.disconnect()
    const leaveCalls = wsSend.mock.calls.filter(
      (call) => call[0] === 'c_voice_leave',
    )
    expect(leaveCalls).toHaveLength(1)
  })

  it('starts reconnect lifecycle when connection-state becomes disconnected', () => {
    voiceState.activateVoiceChannel('lobby', 'voice-room')
    voiceState.status = 'connected'
    emitEnvelope({
      op: 'voice_state_update',
      d: {
        guild_slug: 'lobby',
        channel_slug: 'voice-room',
        participant_count: 1,
        participants: [
          {
            user_id: 'user-1',
            username: 'alice',
            display_name: 'Alice',
            avatar_color: '#3366ff',
            is_muted: false,
            is_deafened: false,
            is_speaking: false,
          },
        ],
      },
    })
    wsSend.mockClear()

    emitEnvelope({
      op: 'voice_connection_state',
      d: {
        guild_slug: 'lobby',
        channel_slug: 'voice-room',
        state: 'disconnected',
      },
    })

    expect(voiceState.status).toBe('retrying')
    expect(voiceState.statusMessage).toBe(VOICE_RECONNECTING_MESSAGE)
    expect(voiceState.activeGuildSlug).toBe('lobby')
    expect(voiceState.activeChannelSlug).toBe('voice-room')
    expect(voiceState.participantCountForChannel('lobby', 'voice-room')).toBe(0)
    expect(wsSend.mock.calls.slice(0, 2)).toEqual([
      [
        'c_voice_leave',
        {
          guild_slug: 'lobby',
          channel_slug: 'voice-room',
        },
      ],
      [
        'c_voice_join',
        {
          guild_slug: 'lobby',
          channel_slug: 'voice-room',
        },
      ],
    ])
  })

  it('restores connected state when reconnect succeeds within fast-recovery window', () => {
    voiceState.activateVoiceChannel('lobby', 'voice-room')
    voiceState.status = 'connected'

    emitEnvelope({
      op: 'voice_connection_state',
      d: {
        guild_slug: 'lobby',
        channel_slug: 'voice-room',
        state: 'disconnected',
      },
    })

    vi.advanceTimersByTime(1_000)
    emitEnvelope({
      op: 'voice_connection_state',
      d: {
        guild_slug: 'lobby',
        channel_slug: 'voice-room',
        state: 'connected',
      },
    })

    expect(voiceState.status).toBe('connected')
    expect(voiceState.statusMessage).toBeNull()
    expect(voiceState.activeGuildSlug).toBe('lobby')
    expect(voiceState.activeChannelSlug).toBe('voice-room')
    expect(voiceState.joinLatencyMs).toBeNull()
  })

  it('shows connection-lost copy after 5 seconds and terminally disconnects at 30 seconds', () => {
    voiceState.activateVoiceChannel('lobby', 'voice-room')
    voiceState.status = 'connected'

    emitEnvelope({
      op: 'voice_connection_state',
      d: {
        guild_slug: 'lobby',
        channel_slug: 'voice-room',
        state: 'disconnected',
      },
    })

    vi.advanceTimersByTime(6_000)
    expect(voiceState.status).toBe('failed')
    expect(voiceState.statusMessage).toBe(VOICE_CONNECTION_LOST_MESSAGE)

    vi.advanceTimersByTime(30_000)
    expect(voiceState.status).toBe('idle')
    expect(voiceState.activeGuildSlug).toBeNull()
    expect(voiceState.activeChannelSlug).toBeNull()
  })
})
