import type { WsClientOp } from '$lib/ws/protocol'
import {
  normalizeParticipantVolumePercent,
  PARTICIPANT_VOLUME_DEFAULT_PERCENT,
  participantVolumePercentToAudioScalar,
} from './participantVolume'
import type { VoiceJoinContext, VoiceParticipantAudioBinding } from './types'

type SendSignal = (op: WsClientOp, payload: Record<string, unknown>) => boolean

type PeerStateListener = (state: RTCPeerConnectionState) => void

type RemoteAudioOutput = {
  streamId: string
  participantUserId: string | null
  audioElement: HTMLAudioElement
  audioContext: AudioContext | null
  sourceNode: MediaElementAudioSourceNode | null
  gainNode: GainNode | null
}

export class VoiceWebRtcClient {
  private peerConnection: RTCPeerConnection | null = null

  private localStream: MediaStream | null = null

  private localAudioTrack: MediaStreamTrack | null = null

  private remoteOutputsByStreamId: Map<string, RemoteAudioOutput> = new Map()

  private participantByStreamId: Map<string, string> = new Map()

  private participantVolumeByUserId: Map<string, number> = new Map()

  private anonymousStreamCounter = 0

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

  syncParticipantBindings(bindings: VoiceParticipantAudioBinding[]): void {
    this.participantByStreamId.clear()
    for (const binding of bindings) {
      const normalizedUserId = binding.userId.trim()
      if (!normalizedUserId) continue
      const normalizedStreamId = binding.audioStreamId?.trim() ?? ''
      if (!normalizedStreamId) continue
      this.participantByStreamId.set(normalizedStreamId, normalizedUserId)
    }
    this.rebindRemoteOutputs()
  }

  clearParticipantVolumes(): void {
    this.participantVolumeByUserId.clear()
    for (const output of this.remoteOutputsByStreamId.values()) {
      this.applyOutputVolume(output, PARTICIPANT_VOLUME_DEFAULT_PERCENT)
    }
  }

  setParticipantVolume(participantUserId: string, volumePercent: number): void {
    const normalizedUserId = participantUserId.trim()
    if (!normalizedUserId) return
    const normalizedVolumePercent =
      normalizeParticipantVolumePercent(volumePercent)
    this.participantVolumeByUserId.set(
      normalizedUserId,
      normalizedVolumePercent,
    )
    for (const output of this.remoteOutputsByStreamId.values()) {
      if (output.participantUserId !== normalizedUserId) continue
      this.applyOutputVolume(output, normalizedVolumePercent)
    }
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
    for (const output of this.remoteOutputsByStreamId.values()) {
      output.audioElement.muted = isDeafened
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

  private resolveStreamId(stream: MediaStream): string {
    const streamId = stream.id.trim()
    if (streamId.length > 0) return streamId
    this.anonymousStreamCounter += 1
    return `voice-anon-stream-${this.anonymousStreamCounter}`
  }

  private resolveParticipantUserId(streamId: string): string | null {
    const mappedUserId = this.participantByStreamId.get(streamId)
    if (mappedUserId) return mappedUserId
    if (this.participantVolumeByUserId.has(streamId)) return streamId
    if (this.participantByStreamId.size === 1) {
      return this.participantByStreamId.values().next().value ?? null
    }
    return null
  }

  private rebindRemoteOutputs(): void {
    for (const output of this.remoteOutputsByStreamId.values()) {
      output.participantUserId = this.resolveParticipantUserId(output.streamId)
      const targetVolume =
        output.participantUserId &&
        this.participantVolumeByUserId.has(output.participantUserId)
          ? this.participantVolumeByUserId.get(output.participantUserId)
          : PARTICIPANT_VOLUME_DEFAULT_PERCENT
      this.applyOutputVolume(
        output,
        targetVolume ?? PARTICIPANT_VOLUME_DEFAULT_PERCENT,
      )
    }
  }

  private attachRemoteStream(stream: MediaStream): void {
    if (typeof Audio === 'undefined') return
    const streamId = this.resolveStreamId(stream)
    if (this.remoteOutputsByStreamId.has(streamId)) {
      const existing = this.remoteOutputsByStreamId.get(streamId)
      if (!existing) return
      existing.audioElement.srcObject = stream
      existing.audioElement.muted = this.isDeafened
      return
    }
    const audioElement = new Audio()
    audioElement.autoplay = true
    ;(
      audioElement as HTMLAudioElement & { playsInline?: boolean }
    ).playsInline = true
    audioElement.muted = this.isDeafened
    audioElement.srcObject = stream
    const output: RemoteAudioOutput = {
      streamId,
      participantUserId: this.resolveParticipantUserId(streamId),
      audioElement,
      audioContext: null,
      sourceNode: null,
      gainNode: null,
    }
    this.remoteOutputsByStreamId.set(streamId, output)
    const initialVolumePercent =
      output.participantUserId &&
      this.participantVolumeByUserId.has(output.participantUserId)
        ? this.participantVolumeByUserId.get(output.participantUserId)
        : PARTICIPANT_VOLUME_DEFAULT_PERCENT
    this.applyOutputVolume(
      output,
      initialVolumePercent ?? PARTICIPANT_VOLUME_DEFAULT_PERCENT,
    )
    void audioElement.play().catch(() => {})
  }

  private ensureGainNode(output: RemoteAudioOutput): GainNode | null {
    if (output.gainNode) {
      return output.gainNode
    }
    if (typeof AudioContext === 'undefined') {
      return null
    }
    try {
      const audioContext = new AudioContext()
      const sourceNode = audioContext.createMediaElementSource(
        output.audioElement,
      )
      const gainNode = audioContext.createGain()
      sourceNode.connect(gainNode)
      gainNode.connect(audioContext.destination)
      void audioContext.resume().catch(() => {})
      output.audioContext = audioContext
      output.sourceNode = sourceNode
      output.gainNode = gainNode
      return gainNode
    } catch {
      this.teardownGainNode(output)
      return null
    }
  }

  private teardownGainNode(output: RemoteAudioOutput): void {
    output.gainNode?.disconnect()
    output.sourceNode?.disconnect()
    output.gainNode = null
    output.sourceNode = null
    const context = output.audioContext
    output.audioContext = null
    if (context) {
      void context.close()
    }
  }

  private applyOutputVolume(
    output: RemoteAudioOutput,
    volumePercent: number,
  ): void {
    const normalizedVolumePercent =
      normalizeParticipantVolumePercent(volumePercent)
    if (normalizedVolumePercent <= 100) {
      this.teardownGainNode(output)
      output.audioElement.volume = participantVolumePercentToAudioScalar(
        normalizedVolumePercent,
      )
      return
    }
    output.audioElement.volume = 1
    const gainNode = this.ensureGainNode(output)
    if (!gainNode) return
    gainNode.gain.value = participantVolumePercentToAudioScalar(
      normalizedVolumePercent,
    )
  }

  private cleanupRemoteAudioOutput(output: RemoteAudioOutput): void {
    this.teardownGainNode(output)
    output.audioElement.pause()
    output.audioElement.srcObject = null
  }

  private cleanupRemoteAudio(): void {
    for (const output of this.remoteOutputsByStreamId.values()) {
      this.cleanupRemoteAudioOutput(output)
    }
    this.remoteOutputsByStreamId.clear()
    this.anonymousStreamCounter = 0
  }
}
