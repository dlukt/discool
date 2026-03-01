import type { WsClientOp } from '$lib/ws/protocol'
import type { VoiceJoinContext } from './types'

type SendSignal = (op: WsClientOp, payload: Record<string, unknown>) => boolean

type PeerStateListener = (state: RTCPeerConnectionState) => void

export class VoiceWebRtcClient {
  private peerConnection: RTCPeerConnection | null = null

  private localStream: MediaStream | null = null

  private pendingRemoteCandidates: RTCIceCandidateInit[] = []

  constructor(private readonly sendSignal: SendSignal) {}

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

  close(): void {
    if (this.peerConnection) {
      this.peerConnection.onicecandidate = null
      this.peerConnection.onconnectionstatechange = null
      this.peerConnection.close()
      this.peerConnection = null
    }
    this.stopLocalStream()
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
    return audioTrack
  }

  private stopLocalStream(): void {
    if (!this.localStream) return
    for (const track of this.localStream.getTracks()) {
      track.stop()
    }
    this.localStream = null
  }
}
