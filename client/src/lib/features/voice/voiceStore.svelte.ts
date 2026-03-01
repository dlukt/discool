import { toastState } from '$lib/feedback/toastStore.svelte'
import { wsClient } from '$lib/ws/client'
import type { WsEnvelope } from '$lib/ws/protocol'
import type {
  VoiceConnectionStateWire,
  VoiceConnectionStatus,
  VoiceIceCandidateWire,
  VoiceJoinContext,
  VoiceOfferWire,
  VoiceParticipant,
  VoiceStateUpdateWire,
} from './types'
import { VoiceWebRtcClient } from './webrtcClient'

export const VOICE_RETRY_MESSAGE = 'Could not connect to voice. Retrying...'
export const VOICE_FAILED_MESSAGE =
  'Voice connection failed. Check your network.'

const JOIN_TIMEOUT_MS = 2_000
const RETRY_INITIAL_MS = 400
const RETRY_MAX_MS = 1_600
const RETRY_MAX_ATTEMPTS = 2

type VoiceChannelSnapshot = {
  participantCount: number
  participants: VoiceParticipant[]
}

const voiceClient = new VoiceWebRtcClient((op, payload) =>
  wsClient.send(op, payload),
)

let retryTimer: ReturnType<typeof setTimeout> | null = null
let joinTimeoutTimer: ReturnType<typeof setTimeout> | null = null

voiceClient.setSpeakingStateListener((isSpeaking) => {
  handleLocalSpeakingStateChange(isSpeaking)
})

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

function sendLeaveSignal(context: VoiceJoinContext): void {
  wsClient.send('c_voice_leave', {
    guild_slug: context.guildSlug,
    channel_slug: context.channelSlug,
  })
}

function channelSnapshotKey(guildSlug: string, channelSlug: string): string {
  return `${guildSlug}:${channelSlug}`
}

function sendVoiceStateUpdate(context: VoiceJoinContext): void {
  if (voiceState.status !== 'connected' || !matchesActiveContext(context))
    return
  wsClient.send('c_voice_state_update', {
    guild_slug: context.guildSlug,
    channel_slug: context.channelSlug,
    is_muted: voiceState.isMuted,
    is_deafened: voiceState.isDeafened,
    is_speaking: voiceState.isSpeaking,
  })
}

function removeChannelSnapshot(context: VoiceJoinContext): void {
  const key = channelSnapshotKey(context.guildSlug, context.channelSlug)
  if (!(key in voiceState.channelSnapshots)) return
  const { [key]: _removed, ...rest } = voiceState.channelSnapshots
  voiceState.channelSnapshots = rest
  voiceState.version += 1
}

function resetControlState(): void {
  voiceState.isMuted = false
  voiceState.isDeafened = false
  voiceState.isSpeaking = false
  voiceClient.setMuted(false)
  voiceClient.setDeafened(false)
}

function clearActiveChannelState(sendLeave = true): void {
  if (
    voiceState.activeGuildSlug === null &&
    voiceState.activeChannelSlug === null &&
    voiceState.attempt === 0 &&
    voiceState.joinStartedAt === null &&
    voiceState.connectedAt === null &&
    voiceState.joinLatencyMs === null &&
    voiceState.status === 'idle' &&
    voiceState.statusMessage === null &&
    voiceState.isMuted === false &&
    voiceState.isDeafened === false &&
    voiceState.isSpeaking === false
  ) {
    return
  }
  const context = activeContext()
  clearRetryTimer()
  clearJoinTimeoutTimer()
  if (sendLeave && context) {
    sendLeaveSignal(context)
  }
  if (context) {
    removeChannelSnapshot(context)
  }
  voiceClient.close()
  voiceState.activeGuildSlug = null
  voiceState.activeChannelSlug = null
  voiceState.attempt = 0
  voiceState.joinStartedAt = null
  voiceState.connectedAt = null
  voiceState.joinLatencyMs = null
  resetControlState()
  setStatus('idle', null)
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
  voiceClient.setMuted(voiceState.isMuted)
  voiceClient.setDeafened(voiceState.isDeafened)
  sendVoiceStateUpdate(context)
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

function parseVoiceParticipant(item: unknown): VoiceParticipant | null {
  if (!isRecord(item)) return null
  const userId = typeof item.user_id === 'string' ? item.user_id.trim() : ''
  const username = typeof item.username === 'string' ? item.username.trim() : ''
  if (!userId || !username) return null
  const displayName =
    typeof item.display_name === 'string' && item.display_name.trim()
      ? item.display_name.trim()
      : null
  const avatarColor =
    typeof item.avatar_color === 'string' && item.avatar_color.trim()
      ? item.avatar_color.trim()
      : null
  return {
    userId,
    username,
    displayName,
    avatarColor,
    isMuted: item.is_muted === true,
    isDeafened: item.is_deafened === true,
    isSpeaking: item.is_speaking === true,
  }
}

function handleVoiceStateUpdate(payload: VoiceStateUpdateWire): void {
  if (!isRecord(payload)) return
  const context = parseContextFromWire(payload as Record<string, unknown>)
  if (!context) return
  const participantsWire = Array.isArray(payload.participants)
    ? payload.participants
    : []
  const participants = participantsWire
    .map((item) => parseVoiceParticipant(item))
    .filter(
      (participant): participant is VoiceParticipant => participant !== null,
    )
  const participantCount =
    typeof payload.participant_count === 'number' &&
    Number.isFinite(payload.participant_count) &&
    payload.participant_count >= 0
      ? Math.floor(payload.participant_count)
      : participants.length
  const key = channelSnapshotKey(context.guildSlug, context.channelSlug)
  voiceState.channelSnapshots = {
    ...voiceState.channelSnapshots,
    [key]: {
      participantCount,
      participants,
    },
  }
  voiceState.version += 1
}

function handleLocalSpeakingStateChange(isSpeaking: boolean): void {
  if (voiceState.isSpeaking === isSpeaking) return
  voiceState.isSpeaking = isSpeaking
  const context = activeContext()
  if (!context || voiceState.status !== 'connected') return
  sendVoiceStateUpdate(context)
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
  if (state === 'disconnected') {
    removeChannelSnapshot(context)
    clearActiveChannelState(false)
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
    op !== 'c_voice_leave' &&
    op !== 'c_voice_answer' &&
    op !== 'c_voice_ice_candidate' &&
    op !== 'c_voice_state_update' &&
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
    case 'voice_state_update':
      handleVoiceStateUpdate(envelope.d as VoiceStateUpdateWire)
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
  isMuted: false,
  isDeafened: false,
  isSpeaking: false,
  activeGuildSlug: null as string | null,
  activeChannelSlug: null as string | null,
  attempt: 0,
  joinStartedAt: null as number | null,
  connectedAt: null as number | null,
  joinLatencyMs: null as number | null,
  channelSnapshots: {} as Record<string, VoiceChannelSnapshot>,

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

    const previousContext = activeContext()
    if (
      previousContext &&
      (previousContext.guildSlug !== normalizedGuild ||
        previousContext.channelSlug !== normalizedChannel)
    ) {
      sendLeaveSignal(previousContext)
      removeChannelSnapshot(previousContext)
    }
    clearRetryTimer()
    clearJoinTimeoutTimer()
    voiceClient.close()
    resetControlState()
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
    clearActiveChannelState(true)
  },

  toggleMute: (): void => {
    const context = activeContext()
    if (voiceState.status !== 'connected' || !context) return
    if (voiceState.isDeafened && voiceState.isMuted) return
    const nextMuted = !voiceState.isMuted
    if (voiceState.isDeafened && !nextMuted) return
    voiceState.isMuted = nextMuted
    if (nextMuted) {
      voiceState.isSpeaking = false
    }
    voiceClient.setMuted(nextMuted)
    sendVoiceStateUpdate(context)
  },

  toggleDeafen: (): void => {
    const context = activeContext()
    if (voiceState.status !== 'connected' || !context) return
    const nextDeafened = !voiceState.isDeafened
    voiceState.isDeafened = nextDeafened
    voiceClient.setDeafened(nextDeafened)
    if (nextDeafened) {
      voiceState.isMuted = true
      voiceState.isSpeaking = false
      voiceClient.setMuted(true)
    }
    sendVoiceStateUpdate(context)
  },

  disconnect: (): void => {
    clearActiveChannelState(true)
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

  participantCountForChannel: (
    guildSlug: string,
    channelSlug: string,
  ): number => {
    const _version = voiceState.version
    void _version
    const normalizedGuild = guildSlug.trim()
    const normalizedChannel = channelSlug.trim()
    if (!normalizedGuild || !normalizedChannel) return 0
    const key = channelSnapshotKey(normalizedGuild, normalizedChannel)
    return voiceState.channelSnapshots[key]?.participantCount ?? 0
  },

  participantsForChannel: (
    guildSlug: string,
    channelSlug: string,
  ): VoiceParticipant[] => {
    const _version = voiceState.version
    void _version
    const normalizedGuild = guildSlug.trim()
    const normalizedChannel = channelSlug.trim()
    if (!normalizedGuild || !normalizedChannel) return []
    const key = channelSnapshotKey(normalizedGuild, normalizedChannel)
    const participants = voiceState.channelSnapshots[key]?.participants ?? []
    return participants.map((participant) => ({ ...participant }))
  },

  activeChannelParticipants: (): VoiceParticipant[] => {
    const context = activeContext()
    if (!context) return []
    return voiceState.participantsForChannel(
      context.guildSlug,
      context.channelSlug,
    )
  },
})

wsClient.subscribe((envelope) => {
  handleEnvelope(envelope)
})
