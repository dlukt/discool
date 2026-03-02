import { toastState } from '$lib/feedback/toastStore.svelte'
import { wsClient } from '$lib/ws/client'
import type { WsEnvelope } from '$lib/ws/protocol'
import {
  normalizeParticipantVolumePercent,
  PARTICIPANT_VOLUME_DEFAULT_PERCENT,
  participantVolumePercentToAudioScalar,
} from './participantVolume'
import {
  loadParticipantVolumePreferences,
  saveParticipantVolumePreferences,
} from './participantVolumeStore.svelte'
import type {
  VoiceConnectionStateWire,
  VoiceConnectionStatus,
  VoiceIceCandidateWire,
  VoiceJoinContext,
  VoiceOfferWire,
  VoiceParticipant,
  VoiceParticipantAudioBinding,
  VoiceParticipantVolumePreference,
  VoiceStateUpdateWire,
} from './types'
import { VoiceWebRtcClient } from './webrtcClient'

export const VOICE_RETRY_MESSAGE = 'Could not connect to voice. Retrying...'
export const VOICE_FAILED_MESSAGE =
  'Voice connection failed. Check your network.'
export const VOICE_RECONNECTING_MESSAGE = 'Reconnecting...'
export const VOICE_CONNECTION_LOST_MESSAGE = 'Connection lost'

const JOIN_TIMEOUT_MS = 2_000
const RETRY_INITIAL_MS = 400
const RETRY_MAX_MS = 1_600
const RETRY_MAX_ATTEMPTS = 2
const RECONNECT_FAST_RECOVERY_MS = 5_000
const RECONNECT_TERMINAL_TIMEOUT_MS = 30_000

type VoiceChannelSnapshot = {
  participantCount: number
  participants: VoiceParticipantSnapshot[]
}

type VoiceParticipantSnapshot = Omit<
  VoiceParticipant,
  'volumePercent' | 'volumeScalar'
>

const voiceClient = new VoiceWebRtcClient((op, payload) =>
  wsClient.send(op, payload),
)

let retryTimer: ReturnType<typeof setTimeout> | null = null
let joinTimeoutTimer: ReturnType<typeof setTimeout> | null = null
let participantVolumeLoadToken = 0
let participantVolumePersistQueue: Promise<void> = Promise.resolve()

voiceClient.setSpeakingStateListener((isSpeaking) => {
  handleLocalSpeakingStateChange(isSpeaking)
})

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null
}

function messageFromError(err: unknown, fallback: string): string {
  if (err instanceof Error && err.message.trim()) {
    return err.message
  }
  return fallback
}

function normalizeOptionalId(value: string | null | undefined): string | null {
  if (typeof value !== 'string') return null
  const normalized = value.trim()
  return normalized.length > 0 ? normalized : null
}

function createVolumePreference(
  participantUserId: string,
  volumePercent: number,
): VoiceParticipantVolumePreference {
  const normalizedVolumePercent =
    normalizeParticipantVolumePercent(volumePercent)
  return {
    participantUserId,
    volumePercent: normalizedVolumePercent,
    audioScalar: participantVolumePercentToAudioScalar(normalizedVolumePercent),
  }
}

function participantVolumeMapFromStorage(
  persistedVolumes: Record<string, number>,
): Record<string, VoiceParticipantVolumePreference> {
  const preferencesByUserId: Record<string, VoiceParticipantVolumePreference> =
    {}
  for (const [participantUserId, volumePercent] of Object.entries(
    persistedVolumes,
  )) {
    const normalizedUserId = normalizeOptionalId(participantUserId)
    if (!normalizedUserId) continue
    preferencesByUserId[normalizedUserId] = createVolumePreference(
      normalizedUserId,
      volumePercent,
    )
  }
  return preferencesByUserId
}

function participantVolumePercentMap(
  preferencesByUserId: Record<string, VoiceParticipantVolumePreference>,
): Record<string, number> {
  const persistedVolumes: Record<string, number> = {}
  for (const [participantUserId, preference] of Object.entries(
    preferencesByUserId,
  )) {
    persistedVolumes[participantUserId] = normalizeParticipantVolumePercent(
      preference.volumePercent,
    )
  }
  return persistedVolumes
}

function sameParticipantVolumePreferences(
  left: Record<string, VoiceParticipantVolumePreference>,
  right: Record<string, VoiceParticipantVolumePreference>,
): boolean {
  const leftEntries = Object.entries(left)
  const rightEntries = Object.entries(right)
  if (leftEntries.length !== rightEntries.length) return false
  for (const [participantUserId, leftPreference] of leftEntries) {
    const rightPreference = right[participantUserId]
    if (
      !rightPreference ||
      leftPreference.volumePercent !== rightPreference.volumePercent ||
      leftPreference.audioScalar !== rightPreference.audioScalar
    ) {
      return false
    }
  }
  return true
}

function replaceParticipantVolumePreferences(
  nextPreferencesByUserId: Record<string, VoiceParticipantVolumePreference>,
): void {
  if (
    sameParticipantVolumePreferences(
      voiceState.participantVolumePreferencesByUserId,
      nextPreferencesByUserId,
    )
  ) {
    return
  }
  voiceState.participantVolumePreferencesByUserId = nextPreferencesByUserId
  voiceState.version += 1
}

function participantVolumePreferenceForUserId(
  participantUserId: string,
): VoiceParticipantVolumePreference {
  const normalizedUserId = participantUserId.trim()
  if (!normalizedUserId) {
    return createVolumePreference('', PARTICIPANT_VOLUME_DEFAULT_PERCENT)
  }
  const existing =
    voiceState.participantVolumePreferencesByUserId[normalizedUserId] ?? null
  return (
    existing ??
    createVolumePreference(normalizedUserId, PARTICIPANT_VOLUME_DEFAULT_PERCENT)
  )
}

function participantAudioBindingsFromSnapshots(
  participants: VoiceParticipantSnapshot[],
): VoiceParticipantAudioBinding[] {
  return participants.map((participant) => ({
    userId: participant.userId,
    audioStreamId: participant.audioStreamId ?? participant.userId,
  }))
}

function applyParticipantVolumesToVoiceClient(
  participants: VoiceParticipantSnapshot[],
): void {
  voiceClient.syncParticipantBindings(
    participantAudioBindingsFromSnapshots(participants),
  )
  for (const participant of participants) {
    const preference = participantVolumePreferenceForUserId(participant.userId)
    voiceClient.setParticipantVolume(
      participant.userId,
      preference.volumePercent,
    )
  }
}

function syncVoiceClientParticipantBindingsForContext(
  context: VoiceJoinContext | null,
): void {
  if (!context) return
  const key = channelSnapshotKey(context.guildSlug, context.channelSlug)
  const participants = voiceState.channelSnapshots[key]?.participants ?? []
  applyParticipantVolumesToVoiceClient(participants)
}

function queueParticipantVolumePersistence(
  ownerUserId: string,
  persistedVolumes: Record<string, number>,
): void {
  participantVolumePersistQueue = participantVolumePersistQueue
    .catch(() => undefined)
    .then(async () => {
      await saveParticipantVolumePreferences(ownerUserId, persistedVolumes)
      voiceState.participantVolumeError = null
    })
    .catch((err) => {
      const message = messageFromError(
        err,
        'Failed to save participant volume preferences to local storage',
      )
      voiceState.participantVolumeError = message
      toastState.show({ variant: 'error', message })
    })
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
    voiceState.reconnectStartedAt === null &&
    voiceState.reconnectLeaveSent === false &&
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
  voiceState.reconnectStartedAt = null
  voiceState.reconnectLeaveSent = false
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

function reconnectElapsedMs(): number {
  if (voiceState.reconnectStartedAt === null) return 0
  return Math.max(0, Date.now() - voiceState.reconnectStartedAt)
}

function applyReconnectStatus(context: VoiceJoinContext): boolean {
  if (!matchesActiveContext(context)) return false
  const elapsed = reconnectElapsedMs()
  if (elapsed >= RECONNECT_TERMINAL_TIMEOUT_MS) {
    clearActiveChannelState(false)
    return false
  }
  if (elapsed >= RECONNECT_FAST_RECOVERY_MS) {
    setStatus('failed', VOICE_CONNECTION_LOST_MESSAGE)
    return true
  }
  setStatus('retrying', VOICE_RECONNECTING_MESSAGE)
  return true
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
  voiceState.reconnectStartedAt = null
  voiceState.reconnectLeaveSent = false
  setStatus('connected', null)
  const connectedAt = Date.now()
  voiceState.connectedAt = connectedAt
  voiceState.joinLatencyMs = voiceState.joinStartedAt
    ? Math.max(0, connectedAt - voiceState.joinStartedAt)
    : null
  voiceClient.setMuted(voiceState.isMuted)
  voiceClient.setDeafened(voiceState.isDeafened)
  syncVoiceClientParticipantBindingsForContext(context)
  sendVoiceStateUpdate(context)
}

function markFailed(context: VoiceJoinContext): void {
  if (!matchesActiveContext(context)) return
  clearRetryTimer()
  clearJoinTimeoutTimer()
  voiceState.reconnectStartedAt = null
  voiceState.reconnectLeaveSent = false
  voiceClient.close()
  setStatus('failed', VOICE_FAILED_MESSAGE)
  toastState.show({ variant: 'error', message: VOICE_FAILED_MESSAGE })
}

function scheduleRetry(context: VoiceJoinContext): void {
  if (!matchesActiveContext(context)) return
  if (voiceState.reconnectStartedAt !== null) {
    if (!applyReconnectStatus(context)) return
    const delay = retryDelayForAttempt(voiceState.attempt)
    clearRetryTimer()
    retryTimer = setTimeout(() => {
      retryTimer = null
      beginJoinAttempt(context, true, true)
    }, delay)
    return
  }
  if (voiceState.attempt >= RETRY_MAX_ATTEMPTS) {
    markFailed(context)
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

function handleJoinFailure(
  context: VoiceJoinContext | null = activeContext(),
): void {
  if (!context || !matchesActiveContext(context)) return
  if (voiceState.status === 'connected') {
    beginReconnectLifecycle(context)
    return
  }
  scheduleRetry(context)
}

function beginReconnectLifecycle(context: VoiceJoinContext): void {
  if (!matchesActiveContext(context)) return
  if (voiceState.reconnectStartedAt !== null) return
  voiceState.reconnectStartedAt = Date.now()
  voiceState.reconnectLeaveSent = false
  voiceState.attempt = 0
  voiceState.joinStartedAt = null
  voiceState.connectedAt = null
  voiceState.joinLatencyMs = null
  voiceClient.requestIceRestart()
  beginJoinAttempt(context, true, true)
}

function beginJoinAttempt(
  context: VoiceJoinContext,
  isRetry: boolean,
  isReconnect = false,
): void {
  if (!matchesActiveContext(context)) return

  if (isReconnect) {
    if (!applyReconnectStatus(context)) return
    if (!voiceState.reconnectLeaveSent) {
      sendLeaveSignal(context)
      removeChannelSnapshot(context)
      voiceState.reconnectLeaveSent = true
    }
  }

  clearJoinTimeoutTimer()
  voiceClient.close()
  if (!isReconnect && voiceState.attempt === 0) {
    voiceState.joinStartedAt = Date.now()
    voiceState.joinLatencyMs = null
    voiceState.connectedAt = null
  }
  voiceState.attempt += 1
  if (!isReconnect) {
    setStatus(
      isRetry ? 'retrying' : 'connecting',
      isRetry ? VOICE_RETRY_MESSAGE : null,
    )
  }

  const sent = wsClient.send('c_voice_join', {
    guild_slug: context.guildSlug,
    channel_slug: context.channelSlug,
  })
  if (!sent) {
    handleJoinFailure(context)
    return
  }

  joinTimeoutTimer = setTimeout(() => {
    joinTimeoutTimer = null
    handleJoinFailure(context)
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

function parseVoiceParticipant(item: unknown): VoiceParticipantSnapshot | null {
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
  const audioStreamId =
    typeof item.audio_stream_id === 'string' && item.audio_stream_id.trim()
      ? item.audio_stream_id.trim()
      : null
  return {
    userId,
    username,
    displayName,
    avatarColor,
    audioStreamId,
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
      (participant): participant is VoiceParticipantSnapshot =>
        participant !== null,
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
  if (matchesActiveContext(context)) {
    applyParticipantVolumesToVoiceClient(participants)
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
    handleJoinFailure(context)
    return
  }
  const sdp = payload.sdp?.trim()
  if (!sdp) {
    handleJoinFailure(context)
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
        handleJoinFailure(context)
      }
    })
    .catch(() => {
      handleJoinFailure(context)
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
      handleJoinFailure(context)
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
    if (voiceState.reconnectStartedAt !== null) {
      markConnectedIfActive(context)
    }
    return
  }
  if (state === 'disconnected') {
    removeChannelSnapshot(context)
    if (voiceState.reconnectStartedAt !== null) {
      return
    }
    if (voiceState.status === 'connected') {
      beginReconnectLifecycle(context)
      return
    }
    clearActiveChannelState(false)
    return
  }
  if (state === 'failed') {
    handleJoinFailure(context)
  }
}

function handleVoiceWsError(envelope: WsEnvelope): void {
  if (!isRecord(envelope.d)) return
  const details = isRecord(envelope.d.details) ? envelope.d.details : null
  const errorContext = details ? parseContextFromWire(details) : null
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
  if (errorContext && !matchesActiveContext(errorContext)) return
  if (
    voiceState.status === 'connecting' ||
    voiceState.status === 'retrying' ||
    voiceState.reconnectStartedAt !== null
  ) {
    handleJoinFailure(errorContext ?? activeContext())
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
  reconnectStartedAt: null as number | null,
  reconnectLeaveSent: false,
  channelSnapshots: {} as Record<string, VoiceChannelSnapshot>,
  participantVolumeOwnerUserId: null as string | null,
  participantVolumesLoading: false,
  participantVolumesLoaded: false,
  participantVolumeError: null as string | null,
  participantVolumePreferencesByUserId: {} as Record<
    string,
    VoiceParticipantVolumePreference
  >,

  initializeParticipantVolumes: async (
    ownerUserId: string | null,
  ): Promise<void> => {
    const normalizedOwnerUserId = normalizeOptionalId(ownerUserId)
    if (!normalizedOwnerUserId) {
      participantVolumeLoadToken += 1
      voiceState.participantVolumeOwnerUserId = null
      voiceState.participantVolumesLoading = false
      voiceState.participantVolumesLoaded = true
      voiceState.participantVolumeError = null
      replaceParticipantVolumePreferences({})
      voiceClient.clearParticipantVolumes()
      return
    }
    if (
      voiceState.participantVolumeOwnerUserId === normalizedOwnerUserId &&
      voiceState.participantVolumesLoaded &&
      !voiceState.participantVolumesLoading
    ) {
      return
    }
    voiceState.participantVolumeOwnerUserId = normalizedOwnerUserId
    voiceState.participantVolumesLoading = true
    voiceState.participantVolumesLoaded = false
    voiceState.participantVolumeError = null
    const loadToken = ++participantVolumeLoadToken
    let loadedPreferencesByUserId: Record<
      string,
      VoiceParticipantVolumePreference
    > = {}
    try {
      const persistedVolumes = await loadParticipantVolumePreferences(
        normalizedOwnerUserId,
      )
      loadedPreferencesByUserId =
        participantVolumeMapFromStorage(persistedVolumes)
      if (loadToken !== participantVolumeLoadToken) {
        return
      }
      replaceParticipantVolumePreferences(loadedPreferencesByUserId)
      voiceState.participantVolumeError = null
      voiceClient.clearParticipantVolumes()
      for (const preference of Object.values(loadedPreferencesByUserId)) {
        voiceClient.setParticipantVolume(
          preference.participantUserId,
          preference.volumePercent,
        )
      }
      syncVoiceClientParticipantBindingsForContext(activeContext())
    } catch (err) {
      if (loadToken !== participantVolumeLoadToken) {
        return
      }
      replaceParticipantVolumePreferences({})
      voiceClient.clearParticipantVolumes()
      const message = messageFromError(
        err,
        'Failed to load participant volume preferences from local storage',
      )
      voiceState.participantVolumeError = message
      toastState.show({ variant: 'error', message })
    } finally {
      if (loadToken === participantVolumeLoadToken) {
        voiceState.participantVolumesLoading = false
        voiceState.participantVolumesLoaded = true
      }
    }
  },

  participantVolumeForUser: (
    participantUserId: string,
  ): VoiceParticipantVolumePreference => {
    return participantVolumePreferenceForUserId(participantUserId)
  },

  setParticipantVolume: (
    participantUserId: string,
    volumePercent: number,
  ): void => {
    const normalizedParticipantUserId = participantUserId.trim()
    if (!normalizedParticipantUserId) return
    const nextPreference = createVolumePreference(
      normalizedParticipantUserId,
      volumePercent,
    )
    const currentPreference =
      voiceState.participantVolumePreferencesByUserId[
        normalizedParticipantUserId
      ]
    if (
      currentPreference &&
      currentPreference.volumePercent === nextPreference.volumePercent
    ) {
      return
    }
    replaceParticipantVolumePreferences({
      ...voiceState.participantVolumePreferencesByUserId,
      [normalizedParticipantUserId]: nextPreference,
    })
    voiceClient.setParticipantVolume(
      normalizedParticipantUserId,
      nextPreference.volumePercent,
    )
    const ownerUserId = voiceState.participantVolumeOwnerUserId
    if (!ownerUserId) return
    const persistedVolumes = participantVolumePercentMap(
      voiceState.participantVolumePreferencesByUserId,
    )
    queueParticipantVolumePersistence(ownerUserId, persistedVolumes)
  },

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
        voiceState.status === 'connected' ||
        voiceState.status === 'failed')
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
    voiceState.reconnectStartedAt = null
    voiceState.reconnectLeaveSent = false
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
    return participants.map((participant) => {
      const preference = participantVolumePreferenceForUserId(
        participant.userId,
      )
      return {
        ...participant,
        volumePercent: preference.volumePercent,
        volumeScalar: preference.audioScalar,
      }
    })
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
