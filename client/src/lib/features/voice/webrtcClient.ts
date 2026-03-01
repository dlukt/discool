import type { WsClientOp } from '$lib/ws/protocol'
import type { VoiceJoinContext } from './types'

type SendSignal = (op: WsClientOp, payload: Record<string, unknown>) => boolean

type PeerStateListener = (state: RTCPeerConnectionState) => void

export class VoiceWebRtcClient {
  private peerConnection: RTCPeerConnection | null = null

  private localStream: MediaStream | null = null

  private localAudioTrack: MediaStreamTrack | null = null

  private remoteAudioElements: Set<HTMLAudioElement> = new Set()

  private isMuted = false

  private isDeafened = false

  private pendingRemoteCandidates: RTCIceCandidateInit[] = []

  private speakingStateListener: ((isSpeaking: boolean) => void) | null = null

  private audioContext: AudioContext | null = null

  private speakingAnalyser: AnalyserNode | null = null

  private speakingSampleBuffer: Uint8Array<ArrayBuffer> | null = null

  private speakingInterval: ReturnType<typeof setInterval> | null = null

  private isSpeaking = false

  constructor(private readonly sendSignal: SendSignal) {}

  setSpeakingStateListener(
    listener: ((isSpeaking: boolean) => void) | null,
  ): void {
    this.speakingStateListener = listener
  }

  async applyOffer(
    context: VoiceJoinContext,
    sdp: string,
    onPeerState: PeerStateListener,
  ): Promise<void> {
    this.close()
    const localAudioTrack = await this.requestLocalAudioTrack()
    const connection = this.createPeerConnection(context, onPeerState)
    this.peerConnection = connection

    connection.addTransceiver(localAudioTrack, { direction: 'sendrecv' })
    await connection.setRemoteDescription({ type: 'offer', sdp })
    await this.flushPendingCandidates()

    const answer = await connection.createAnswer()
    await connection.setLocalDescription(answer)
    const localSdp = connection.localDescription?.sdp?.trim() || ''
    if (!localSdp) {
      throw new Error('Voice answer SDP is missing.')
    }
    const sent = this.sendSignal('c_voice_answer', {
      guild_slug: context.guildSlug,
      channel_slug: context.channelSlug,
      sdp: localSdp,
      sdp_type: 'answer',
    })
    if (!sent) {
      throw new Error('Voice signaling channel is unavailable.')
    }
  }

  async addRemoteCandidate(candidate: RTCIceCandidateInit): Promise<void> {
    if (!candidate.candidate?.trim()) return
    const connection = this.peerConnection
    if (!connection || !connection.remoteDescription) {
      this.pendingRemoteCandidates.push(candidate)
      return
    }
    await connection.addIceCandidate(candidate)
  }

  setMuted(isMuted: boolean): void {
    this.isMuted = isMuted
    if (this.localAudioTrack) {
      this.localAudioTrack.enabled = !isMuted
    }
    if (isMuted) {
      this.emitSpeakingState(false)
    }
  }

  setDeafened(isDeafened: boolean): void {
    this.isDeafened = isDeafened
    for (const element of this.remoteAudioElements) {
      element.muted = isDeafened
    }
  }

  close(): void {
    if (this.peerConnection) {
      this.peerConnection.onicecandidate = null
      this.peerConnection.onconnectionstatechange = null
      this.peerConnection.ontrack = null
      this.peerConnection.close()
      this.peerConnection = null
    }
    this.stopLocalStream()
    this.stopSpeakingDetection()
    this.cleanupRemoteAudio()
    this.pendingRemoteCandidates = []
  }

  private createPeerConnection(
    context: VoiceJoinContext,
    onPeerState: PeerStateListener,
  ): RTCPeerConnection {
    if (typeof RTCPeerConnection === 'undefined') {
      throw new Error('WebRTC is not supported in this browser.')
    }
    const connection = new RTCPeerConnection()
    connection.onconnectionstatechange = () => {
      onPeerState(connection.connectionState)
    }
    connection.onicecandidate = (event) => {
      const candidate = event.candidate
      if (!candidate) return
      const sent = this.sendSignal('c_voice_ice_candidate', {
        guild_slug: context.guildSlug,
        channel_slug: context.channelSlug,
        candidate: candidate.candidate,
        sdp_mid: candidate.sdpMid ?? null,
        sdp_mline_index: candidate.sdpMLineIndex ?? null,
      })
      if (!sent) {
        onPeerState('failed')
      }
    }
    connection.ontrack = (event) => {
      const [stream] = event.streams
      if (!stream) return
      this.attachRemoteStream(stream)
    }
    return connection
  }

  private async flushPendingCandidates(): Promise<void> {
    const pending = this.pendingRemoteCandidates
    this.pendingRemoteCandidates = []
    for (const candidate of pending) {
      await this.addRemoteCandidate(candidate)
    }
  }

  private async requestLocalAudioTrack(): Promise<MediaStreamTrack> {
    if (
      typeof navigator === 'undefined' ||
      !navigator.mediaDevices ||
      typeof navigator.mediaDevices.getUserMedia !== 'function'
    ) {
      throw new Error('Microphone access is unavailable in this browser.')
    }
    const stream = await navigator.mediaDevices.getUserMedia({ audio: true })
    const [audioTrack] = stream.getAudioTracks()
    if (!audioTrack) {
      for (const track of stream.getTracks()) {
        track.stop()
      }
      throw new Error('No microphone audio track available.')
    }
    this.localStream = stream
    this.localAudioTrack = audioTrack
    audioTrack.enabled = !this.isMuted
    this.startSpeakingDetection(stream)
    return audioTrack
  }

  private stopLocalStream(): void {
    if (!this.localStream) return
    for (const track of this.localStream.getTracks()) {
      track.stop()
    }
    this.localStream = null
    this.localAudioTrack = null
  }

  private emitSpeakingState(isSpeaking: boolean): void {
    if (this.isSpeaking === isSpeaking) return
    this.isSpeaking = isSpeaking
    this.speakingStateListener?.(isSpeaking)
  }

  private startSpeakingDetection(stream: MediaStream): void {
    this.stopSpeakingDetection()
    if (typeof AudioContext === 'undefined') return
    const audioContext = new AudioContext()
    const sourceNode = audioContext.createMediaStreamSource(stream)
    const analyser = audioContext.createAnalyser()
    analyser.fftSize = 512
    sourceNode.connect(analyser)
    this.audioContext = audioContext
    this.speakingAnalyser = analyser
    this.speakingSampleBuffer = new Uint8Array(
      new ArrayBuffer(analyser.fftSize),
    )
    this.speakingInterval = setInterval(() => {
      const activeAnalyser = this.speakingAnalyser
      const samples = this.speakingSampleBuffer
      if (!activeAnalyser || !samples) return
      activeAnalyser.getByteTimeDomainData(samples)
      let peak = 0
      for (const sample of samples) {
        const centered = Math.abs(sample - 128) / 128
        if (centered > peak) {
          peak = centered
        }
      }
      const speaking = !this.isMuted && peak >= 0.06
      this.emitSpeakingState(speaking)
    }, 150)
  }

  private stopSpeakingDetection(): void {
    if (this.speakingInterval) {
      clearInterval(this.speakingInterval)
      this.speakingInterval = null
    }
    const context = this.audioContext
    this.audioContext = null
    this.speakingAnalyser = null
    this.speakingSampleBuffer = null
    if (context) {
      void context.close()
    }
    this.emitSpeakingState(false)
  }

  private attachRemoteStream(stream: MediaStream): void {
    if (typeof Audio === 'undefined') return
    for (const existing of this.remoteAudioElements) {
      if (existing.srcObject === stream) {
        existing.muted = this.isDeafened
        return
      }
    }
    const audioElement = new Audio()
    audioElement.autoplay = true
    ;(
      audioElement as HTMLAudioElement & { playsInline?: boolean }
    ).playsInline = true
    audioElement.muted = this.isDeafened
    audioElement.srcObject = stream
    this.remoteAudioElements.add(audioElement)
    void audioElement.play().catch(() => {})
  }

  private cleanupRemoteAudio(): void {
    for (const element of this.remoteAudioElements) {
      element.pause()
      element.srcObject = null
    }
    this.remoteAudioElements.clear()
  }
}
