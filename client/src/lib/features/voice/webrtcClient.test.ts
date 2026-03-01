import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import { VoiceWebRtcClient } from './webrtcClient'

class MockPeerConnection {
  static instances: MockPeerConnection[] = []

  localDescription: RTCSessionDescription | null = null

  remoteDescription: RTCSessionDescription | null = null

  connectionState: RTCPeerConnectionState = 'new'

  onconnectionstatechange: (() => void) | null = null

  onicecandidate: ((event: RTCPeerConnectionIceEvent) => void) | null = null

  addTransceiver = vi.fn()

  setRemoteDescription = vi.fn(
    async (description: RTCSessionDescriptionInit): Promise<void> => {
      this.remoteDescription = description as RTCSessionDescription
    },
  )

  createAnswer = vi.fn(
    async (): Promise<RTCSessionDescriptionInit> => ({
      type: 'answer',
      sdp: 'v=0\r\n',
    }),
  )

  setLocalDescription = vi.fn(
    async (description: RTCSessionDescriptionInit): Promise<void> => {
      this.localDescription = description as RTCSessionDescription
    },
  )

  addIceCandidate = vi.fn(async (): Promise<void> => {})

  close = vi.fn()

  constructor() {
    MockPeerConnection.instances.push(this)
  }
}

function latestConnection(): MockPeerConnection {
  const connection = MockPeerConnection.instances.at(-1)
  if (!connection) {
    throw new Error('Expected RTCPeerConnection instance.')
  }
  return connection
}

const originalMediaDevices = Object.getOwnPropertyDescriptor(
  globalThis.navigator,
  'mediaDevices',
)

function setMediaDevices(getUserMedia: () => Promise<MediaStream>): void {
  Object.defineProperty(globalThis.navigator, 'mediaDevices', {
    configurable: true,
    value: { getUserMedia },
  })
}

describe('VoiceWebRtcClient', () => {
  beforeEach(() => {
    MockPeerConnection.instances = []
    vi.stubGlobal(
      'RTCPeerConnection',
      MockPeerConnection as unknown as typeof RTCPeerConnection,
    )
  })

  afterEach(() => {
    vi.restoreAllMocks()
    vi.unstubAllGlobals()
    if (originalMediaDevices) {
      Object.defineProperty(
        globalThis.navigator,
        'mediaDevices',
        originalMediaDevices,
      )
      return
    }
    Reflect.deleteProperty(globalThis.navigator, 'mediaDevices')
  })

  it('captures microphone audio and negotiates sendrecv answer', async () => {
    const sendSignal = vi.fn(() => true)
    const audioTrack = {
      stop: vi.fn(),
      readyState: 'live',
    } as unknown as MediaStreamTrack
    const mediaStream = {
      getAudioTracks: () => [audioTrack],
      getTracks: () => [audioTrack],
    } as unknown as MediaStream
    const getUserMedia = vi.fn(async () => mediaStream)
    setMediaDevices(getUserMedia)

    const client = new VoiceWebRtcClient(sendSignal)
    await client.applyOffer(
      { guildSlug: 'lobby', channelSlug: 'voice-room' },
      'v=0\r\n',
      vi.fn(),
    )

    const connection = latestConnection()
    expect(getUserMedia).toHaveBeenCalledWith({ audio: true })
    expect(connection.addTransceiver).toHaveBeenCalledWith(audioTrack, {
      direction: 'sendrecv',
    })
    expect(sendSignal).toHaveBeenCalledWith('c_voice_answer', {
      guild_slug: 'lobby',
      channel_slug: 'voice-room',
      sdp: 'v=0',
      sdp_type: 'answer',
    })

    client.close()
    expect(audioTrack.stop).toHaveBeenCalledTimes(1)
  })

  it('fails fast when microphone access is unavailable', async () => {
    Object.defineProperty(globalThis.navigator, 'mediaDevices', {
      configurable: true,
      value: undefined,
    })
    const client = new VoiceWebRtcClient(vi.fn(() => true))

    await expect(
      client.applyOffer(
        { guildSlug: 'lobby', channelSlug: 'voice-room' },
        'v=0\r\n',
        vi.fn(),
      ),
    ).rejects.toThrow('Microphone access is unavailable in this browser.')
    expect(MockPeerConnection.instances).toHaveLength(0)
  })
})
