import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import { VoiceWebRtcClient } from './webrtcClient'

class MockPeerConnection {
  static instances: MockPeerConnection[] = []

  localDescription: RTCSessionDescription | null = null

  remoteDescription: RTCSessionDescription | null = null

  connectionState: RTCPeerConnectionState = 'new'

  onconnectionstatechange: (() => void) | null = null

  onicecandidate: ((event: RTCPeerConnectionIceEvent) => void) | null = null

  ontrack: ((event: RTCTrackEvent) => void) | null = null

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

type MockAudioContextInstance = {
  close: ReturnType<typeof vi.fn>
  resume: ReturnType<typeof vi.fn>
  createMediaElementSource: ReturnType<typeof vi.fn>
  createGain: ReturnType<typeof vi.fn>
  gainNode: {
    gain: { value: number }
    connect: ReturnType<typeof vi.fn>
    disconnect: ReturnType<typeof vi.fn>
  }
}

const mockAudioContextInstances: MockAudioContextInstance[] = []

class MockAudioContext {
  destination = {} as AudioNode

  private readonly sourceNode = {
    connect: vi.fn(),
    disconnect: vi.fn(),
  }

  private readonly gainNode = {
    gain: { value: 1 },
    connect: vi.fn(),
    disconnect: vi.fn(),
  }

  createMediaElementSource = vi.fn(() => this.sourceNode)

  createGain = vi.fn(() => this.gainNode)

  resume = vi.fn(async () => {})

  close = vi.fn(async () => {})

  constructor() {
    mockAudioContextInstances.push({
      close: this.close,
      resume: this.resume,
      createMediaElementSource: this.createMediaElementSource,
      createGain: this.createGain,
      gainNode: this.gainNode,
    })
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
    mockAudioContextInstances.length = 0
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
      enabled: true,
    } as unknown as MediaStreamTrack
    const mediaStream = {
      getAudioTracks: () => [audioTrack],
      getTracks: () => [audioTrack],
    } as unknown as MediaStream
    const getUserMedia = vi.fn(async () => mediaStream)
    setMediaDevices(getUserMedia)
    vi.stubGlobal('AudioContext', undefined)

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
    client.setMuted(true)
    expect(audioTrack.enabled).toBe(false)
    client.setMuted(false)
    expect(audioTrack.enabled).toBe(true)

    client.close()
    expect(audioTrack.stop).toHaveBeenCalledTimes(1)
  })

  it('applies 0..100 participant volume via media element volume', async () => {
    const sendSignal = vi.fn(() => true)
    const audioTrack = {
      stop: vi.fn(),
      readyState: 'live',
      enabled: true,
    } as unknown as MediaStreamTrack
    const mediaStream = {
      getAudioTracks: () => [audioTrack],
      getTracks: () => [audioTrack],
    } as unknown as MediaStream
    const getUserMedia = vi.fn(async () => mediaStream)
    setMediaDevices(getUserMedia)
    vi.stubGlobal('AudioContext', undefined)
    const remoteAudioElement = {
      autoplay: false,
      playsInline: false,
      muted: false,
      volume: 1,
      srcObject: null as MediaStream | null,
      play: vi.fn(async () => {}),
      pause: vi.fn(),
    } as unknown as HTMLAudioElement
    const audioCtor = vi.fn(function MockAudio() {
      return remoteAudioElement
    })
    vi.stubGlobal('Audio', audioCtor as unknown as typeof Audio)

    const client = new VoiceWebRtcClient(sendSignal)
    await client.applyOffer(
      { guildSlug: 'lobby', channelSlug: 'voice-room' },
      'v=0\r\n',
      vi.fn(),
    )

    client.syncParticipantBindings([
      { userId: 'user-remote', audioStreamId: 'remote-stream' },
    ])

    const connection = latestConnection()
    const remoteStream = { id: 'remote-stream' } as unknown as MediaStream
    connection.ontrack?.({
      streams: [remoteStream],
    } as unknown as RTCTrackEvent)

    client.setParticipantVolume('user-remote', 40)
    expect(remoteAudioElement.volume).toBeCloseTo(0.4)
  })

  it('applies >100 participant volume via gain node and cleans up audio resources', async () => {
    const sendSignal = vi.fn(() => true)
    const audioTrack = {
      stop: vi.fn(),
      readyState: 'live',
      enabled: true,
    } as unknown as MediaStreamTrack
    const mediaStream = {
      getAudioTracks: () => [audioTrack],
      getTracks: () => [audioTrack],
    } as unknown as MediaStream
    const getUserMedia = vi.fn(async () => mediaStream)
    setMediaDevices(getUserMedia)
    vi.stubGlobal('AudioContext', undefined)
    const remoteAudioElement = {
      autoplay: false,
      playsInline: false,
      muted: false,
      volume: 1,
      srcObject: null as MediaStream | null,
      play: vi.fn(async () => {}),
      pause: vi.fn(),
    } as unknown as HTMLAudioElement
    const audioCtor = vi.fn(function MockAudio() {
      return remoteAudioElement
    })
    vi.stubGlobal('Audio', audioCtor as unknown as typeof Audio)

    const client = new VoiceWebRtcClient(sendSignal)
    await client.applyOffer(
      { guildSlug: 'lobby', channelSlug: 'voice-room' },
      'v=0\r\n',
      vi.fn(),
    )
    client.syncParticipantBindings([
      { userId: 'user-remote', audioStreamId: 'remote-stream' },
    ])

    const connection = latestConnection()
    const remoteStream = { id: 'remote-stream' } as unknown as MediaStream
    connection.ontrack?.({
      streams: [remoteStream],
    } as unknown as RTCTrackEvent)

    vi.stubGlobal(
      'AudioContext',
      MockAudioContext as unknown as typeof AudioContext,
    )
    client.setParticipantVolume('user-remote', 150)

    expect(remoteAudioElement.volume).toBe(1)
    const gainContextInstance = mockAudioContextInstances.at(-1)
    expect(gainContextInstance).toBeDefined()
    expect(gainContextInstance?.gainNode.gain.value).toBeCloseTo(1.5)

    client.close()
    expect(remoteAudioElement.pause).toHaveBeenCalledTimes(1)
    expect(remoteAudioElement.srcObject).toBeNull()
    expect(gainContextInstance?.close).toHaveBeenCalledTimes(1)
  })

  it('applies deafen state to tracked remote audio elements', async () => {
    const sendSignal = vi.fn(() => true)
    const audioTrack = {
      stop: vi.fn(),
      readyState: 'live',
      enabled: true,
    } as unknown as MediaStreamTrack
    const mediaStream = {
      getAudioTracks: () => [audioTrack],
      getTracks: () => [audioTrack],
    } as unknown as MediaStream
    const getUserMedia = vi.fn(async () => mediaStream)
    setMediaDevices(getUserMedia)
    vi.stubGlobal('AudioContext', undefined)
    const remoteAudioElement = {
      autoplay: false,
      playsInline: false,
      muted: false,
      volume: 1,
      srcObject: null as MediaStream | null,
      play: vi.fn(async () => {}),
      pause: vi.fn(),
    } as unknown as HTMLAudioElement
    const audioCtor = vi.fn(function MockAudio() {
      return remoteAudioElement
    })
    vi.stubGlobal('Audio', audioCtor as unknown as typeof Audio)

    const client = new VoiceWebRtcClient(sendSignal)
    await client.applyOffer(
      { guildSlug: 'lobby', channelSlug: 'voice-room' },
      'v=0\r\n',
      vi.fn(),
    )

    const connection = latestConnection()
    const remoteStream = { id: 'remote-stream' } as unknown as MediaStream
    connection.ontrack?.({
      streams: [remoteStream],
    } as unknown as RTCTrackEvent)

    expect(audioCtor).toHaveBeenCalledTimes(1)
    expect(remoteAudioElement.srcObject).toBe(remoteStream)
    client.setDeafened(true)
    expect(remoteAudioElement.muted).toBe(true)
    client.setDeafened(false)
    expect(remoteAudioElement.muted).toBe(false)

    client.close()
    expect(remoteAudioElement.pause).toHaveBeenCalledTimes(1)
    expect(remoteAudioElement.srcObject).toBeNull()
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
