import { toastState } from '$lib/feedback/toastStore.svelte'
import { wsClient } from '$lib/ws/client'
import type { WsEnvelope } from '$lib/ws/protocol'
import type {
  VoiceConnectionStateWire,
  VoiceConnectionStatus,
  VoiceIceCandidateWire,
  VoiceJoinContext,
  VoiceOfferWire,
} from './types'
import { VoiceWebRtcClient } from './webrtcClient'

export const VOICE_RETRY_MESSAGE = 'Could not connect to voice. Retrying...'
export const VOICE_FAILED_MESSAGE =
  'Voice connection failed. Check your network.'

const JOIN_TIMEOUT_MS = 2_000
const RETRY_INITIAL_MS = 400
const RETRY_MAX_MS = 1_600
const RETRY_MAX_ATTEMPTS = 2

const voiceClient = new VoiceWebRtcClient((op, payload) =>
  wsClient.send(op, payload),
)

let retryTimer: ReturnType<typeof setTimeout> | null = null
let joinTimeoutTimer: ReturnType<typeof setTimeout> | null = null

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null
}

function clearRetryTimer(): void {
  if (retryTimer === null) return
  clearTimeout(retryTimer)
  retryTimer = null
}

function clearJoinTimeoutTimer(): void {
  if (joinTimeoutTimer === null) return
  clearTimeout(joinTimeoutTimer)
  joinTimeoutTimer = null
}

function matchesActiveContext(context: VoiceJoinContext): boolean {
  return (
    voiceState.activeGuildSlug === context.guildSlug &&
    voiceState.activeChannelSlug === context.channelSlug
  )
}

function activeContext(): VoiceJoinContext | null {
  if (!voiceState.activeGuildSlug || !voiceState.activeChannelSlug) return null
  return {
    guildSlug: voiceState.activeGuildSlug,
    channelSlug: voiceState.activeChannelSlug,
  }
}

function retryDelayForAttempt(attempt: number): number {
  const exponent = Math.max(0, attempt - 1)
  return Math.min(RETRY_INITIAL_MS * 2 ** exponent, RETRY_MAX_MS)
}

function setStatus(
  status: VoiceConnectionStatus,
  message: string | null,
): void {
  voiceState.status = status
  voiceState.statusMessage = message
  voiceState.version += 1
}

function markConnectedIfActive(context: VoiceJoinContext): void {
  if (!matchesActiveContext(context)) return
  clearRetryTimer()
  clearJoinTimeoutTimer()
  setStatus('connected', null)
  const connectedAt = Date.now()
  voiceState.connectedAt = connectedAt
  voiceState.joinLatencyMs = voiceState.joinStartedAt
    ? Math.max(0, connectedAt - voiceState.joinStartedAt)
    : null
}

function markFailed(): void {
  clearRetryTimer()
  clearJoinTimeoutTimer()
  voiceClient.close()
  setStatus('failed', VOICE_FAILED_MESSAGE)
  toastState.show({ variant: 'error', message: VOICE_FAILED_MESSAGE })
}

function scheduleRetry(): void {
  const context = activeContext()
  if (!context) return
  if (voiceState.attempt >= RETRY_MAX_ATTEMPTS) {
    markFailed()
    return
  }
  const delay = retryDelayForAttempt(voiceState.attempt)
  setStatus('retrying', VOICE_RETRY_MESSAGE)
  clearRetryTimer()
  retryTimer = setTimeout(() => {
    retryTimer = null
    beginJoinAttempt(context, true)
  }, delay)
}

function handleJoinFailure(): void {
  if (voiceState.status === 'connected') return
  scheduleRetry()
}

function beginJoinAttempt(context: VoiceJoinContext, isRetry: boolean): void {
  if (!matchesActiveContext(context)) return

  clearJoinTimeoutTimer()
  voiceClient.close()
  if (voiceState.attempt === 0) {
    voiceState.joinStartedAt = Date.now()
    voiceState.joinLatencyMs = null
    voiceState.connectedAt = null
  }
  voiceState.attempt += 1
  setStatus(
    isRetry ? 'retrying' : 'connecting',
    isRetry ? VOICE_RETRY_MESSAGE : null,
  )

  const sent = wsClient.send('c_voice_join', {
    guild_slug: context.guildSlug,
    channel_slug: context.channelSlug,
  })
  if (!sent) {
    handleJoinFailure()
    return
  }

  joinTimeoutTimer = setTimeout(() => {
    joinTimeoutTimer = null
    handleJoinFailure()
  }, JOIN_TIMEOUT_MS)
}

function parseContextFromWire(
  payload: Record<string, unknown>,
): VoiceJoinContext | null {
  const guildSlug =
    typeof payload.guild_slug === 'string' ? payload.guild_slug.trim() : ''
  const channelSlug =
    typeof payload.channel_slug === 'string' ? payload.channel_slug.trim() : ''
  if (!guildSlug || !channelSlug) return null
  return { guildSlug, channelSlug }
}

function handleVoiceOffer(payload: VoiceOfferWire): void {
  if (!isRecord(payload)) return
  const context = parseContextFromWire(
    payload as unknown as Record<string, unknown>,
  )
  if (!context || !matchesActiveContext(context)) return
  if (payload.sdp_type !== 'offer') {
    handleJoinFailure()
    return
  }
  const sdp = payload.sdp?.trim()
  if (!sdp) {
    handleJoinFailure()
    return
  }

  void voiceClient
    .applyOffer(context, sdp, (state) => {
      if (!matchesActiveContext(context)) return
      if (state === 'connected') {
        markConnectedIfActive(context)
      } else if (
        state === 'failed' ||
        state === 'disconnected' ||
        state === 'closed'
      ) {
        handleJoinFailure()
      }
    })
    .catch(() => {
      handleJoinFailure()
    })
}

function handleVoiceIceCandidate(payload: VoiceIceCandidateWire): void {
  if (!isRecord(payload)) return
  const context = parseContextFromWire(
    payload as unknown as Record<string, unknown>,
  )
  if (!context || !matchesActiveContext(context)) return
  const candidate = payload.candidate?.trim()
  if (!candidate) return
  const sdpMid =
    typeof payload.sdp_mid === 'string' && payload.sdp_mid.trim()
      ? payload.sdp_mid.trim()
      : null
  const sdpMLineIndex =
    typeof payload.sdp_mline_index === 'number' &&
    Number.isFinite(payload.sdp_mline_index)
      ? payload.sdp_mline_index
      : null
  void voiceClient
    .addRemoteCandidate({
      candidate,
      sdpMid,
      sdpMLineIndex,
    })
    .catch(() => {
      handleJoinFailure()
    })
}

function handleVoiceConnectionState(payload: VoiceConnectionStateWire): void {
  if (!isRecord(payload)) return
  const context = parseContextFromWire(
    payload as unknown as Record<string, unknown>,
  )
  if (!context || !matchesActiveContext(context)) return
  const state = payload.state
  if (state === 'connected') {
    return
  }
  if (state === 'failed') {
    handleJoinFailure()
  }
}

function handleVoiceWsError(envelope: WsEnvelope): void {
  if (!isRecord(envelope.d)) return
  const details = isRecord(envelope.d.details) ? envelope.d.details : null
  const op = details && typeof details.op === 'string' ? details.op.trim() : ''
  const message =
    typeof envelope.d.message === 'string'
      ? envelope.d.message.toLowerCase()
      : ''
  if (
    op !== 'c_voice_join' &&
    op !== 'c_voice_answer' &&
    op !== 'c_voice_ice_candidate' &&
    !message.includes('voice')
  ) {
    return
  }
  if (voiceState.status === 'connecting' || voiceState.status === 'retrying') {
    handleJoinFailure()
  }
}

function handleEnvelope(envelope: WsEnvelope): void {
  switch (envelope.op) {
    case 'voice_offer':
      handleVoiceOffer(envelope.d as VoiceOfferWire)
      return
    case 'voice_ice_candidate':
      handleVoiceIceCandidate(envelope.d as VoiceIceCandidateWire)
      return
    case 'voice_connection_state':
      handleVoiceConnectionState(envelope.d as VoiceConnectionStateWire)
      return
    case 'error':
      handleVoiceWsError(envelope)
      return
    default:
      return
  }
}

export const voiceState = $state({
  version: 0,
  status: 'idle' as VoiceConnectionStatus,
  statusMessage: null as string | null,
  activeGuildSlug: null as string | null,
  activeChannelSlug: null as string | null,
  attempt: 0,
  joinStartedAt: null as number | null,
  connectedAt: null as number | null,
  joinLatencyMs: null as number | null,

  activateVoiceChannel: (guildSlug: string, channelSlug: string): void => {
    const normalizedGuild = guildSlug.trim()
    const normalizedChannel = channelSlug.trim()
    if (!normalizedGuild || !normalizedChannel) {
      voiceState.clearActiveChannel()
      return
    }
    if (
      voiceState.activeGuildSlug === normalizedGuild &&
      voiceState.activeChannelSlug === normalizedChannel &&
      (voiceState.status === 'connecting' ||
        voiceState.status === 'retrying' ||
        voiceState.status === 'connected')
    ) {
      return
    }

    clearRetryTimer()
    clearJoinTimeoutTimer()
    voiceClient.close()
    voiceState.activeGuildSlug = normalizedGuild
    voiceState.activeChannelSlug = normalizedChannel
    voiceState.attempt = 0
    voiceState.joinStartedAt = null
    voiceState.connectedAt = null
    voiceState.joinLatencyMs = null
    beginJoinAttempt(
      { guildSlug: normalizedGuild, channelSlug: normalizedChannel },
      false,
    )
  },

  clearActiveChannel: (): void => {
    if (
      voiceState.activeGuildSlug === null &&
      voiceState.activeChannelSlug === null &&
      voiceState.attempt === 0 &&
      voiceState.joinStartedAt === null &&
      voiceState.connectedAt === null &&
      voiceState.joinLatencyMs === null &&
      voiceState.status === 'idle' &&
      voiceState.statusMessage === null
    ) {
      return
    }
    clearRetryTimer()
    clearJoinTimeoutTimer()
    voiceClient.close()
    voiceState.activeGuildSlug = null
    voiceState.activeChannelSlug = null
    voiceState.attempt = 0
    voiceState.joinStartedAt = null
    voiceState.connectedAt = null
    voiceState.joinLatencyMs = null
    setStatus('idle', null)
  },

  statusForChannel: (
    guildSlug: string,
    channelSlug: string,
  ): VoiceConnectionStatus => {
    const normalizedGuild = guildSlug.trim()
    const normalizedChannel = channelSlug.trim()
    if (
      normalizedGuild &&
      normalizedChannel &&
      voiceState.activeGuildSlug === normalizedGuild &&
      voiceState.activeChannelSlug === normalizedChannel
    ) {
      return voiceState.status
    }
    return 'idle'
  },

  statusMessageForChannel: (
    guildSlug: string,
    channelSlug: string,
  ): string | null => {
    const normalizedGuild = guildSlug.trim()
    const normalizedChannel = channelSlug.trim()
    if (
      normalizedGuild &&
      normalizedChannel &&
      voiceState.activeGuildSlug === normalizedGuild &&
      voiceState.activeChannelSlug === normalizedChannel
    ) {
      return voiceState.statusMessage
    }
    return null
  },
})

wsClient.subscribe((envelope) => {
  handleEnvelope(envelope)
})
