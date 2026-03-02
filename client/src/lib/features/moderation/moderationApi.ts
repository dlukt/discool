import { ApiError, apiFetch, apiFetchCursorList } from '$lib/api'

export type CreateMuteInput = {
  targetUserId: string
  reason: string
  durationSeconds?: number | null
  isPermanent?: boolean
}

export type CreateKickInput = {
  targetUserId: string
  reason: string
}

export type CreateVoiceKickInput = {
  targetUserId: string
  channelSlug: string
  reason: string
}

export type BanDeleteMessageWindow = 'none' | '1h' | '24h' | '7d'

export type CreateBanInput = {
  targetUserId: string
  reason: string
  deleteMessageWindow: BanDeleteMessageWindow
}

export type MuteAction = {
  id: string
  guildSlug: string
  actorUserId: string
  targetUserId: string
  reason: string
  durationSeconds: number | null
  expiresAt: string | null
  isPermanent: boolean
  createdAt: string
  updatedAt: string
}

export type MuteStatus = {
  active: boolean
  isPermanent: boolean
  expiresAt: string | null
  reason: string | null
}

export type KickAction = {
  id: string
  guildSlug: string
  actorUserId: string
  targetUserId: string
  reason: string
  createdAt: string
  updatedAt: string
}

export type VoiceKickAction = {
  id: string
  guildSlug: string
  channelSlug: string
  actorUserId: string
  targetUserId: string
  reason: string
  createdAt: string
  updatedAt: string
}

export type BanAction = {
  id: string
  banId: string
  guildSlug: string
  actorUserId: string
  targetUserId: string
  reason: string
  deleteMessageWindow: BanDeleteMessageWindow
  deleteMessagesWindowSeconds: number | null
  deletedMessagesCount: number
  createdAt: string
  updatedAt: string
}

export type ModerationLogOrder = 'asc' | 'desc'
export type ModerationLogActionType =
  | 'mute'
  | 'kick'
  | 'ban'
  | 'voice_kick'
  | 'message_delete'
  | 'warn'

export type FetchModerationLogInput = {
  limit?: number
  cursor?: string | null
  order?: ModerationLogOrder
  actionType?: ModerationLogActionType | null
}

export type ModerationLogEntry = {
  id: string
  actionType: ModerationLogActionType
  reason: string
  createdAt: string
  actorUserId: string
  actorUsername: string
  actorDisplayName: string
  actorAvatarColor: string | null
  targetUserId: string
  targetUsername: string
  targetDisplayName: string
  targetAvatarColor: string | null
}

export type ModerationLogPage = {
  entries: ModerationLogEntry[]
  cursor: string | null
}

type CreateMuteWire = {
  target_user_id: string
  reason: string
  duration_seconds?: number
  is_permanent: boolean
}

type MuteActionWire = {
  id: string
  guild_slug: string
  actor_user_id: string
  target_user_id: string
  reason: string
  duration_seconds?: number
  expires_at?: string
  is_permanent: boolean
  created_at: string
  updated_at: string
}

type MuteStatusWire = {
  active: boolean
  is_permanent: boolean
  expires_at?: string
  reason?: string
}

type CreateKickWire = {
  target_user_id: string
  reason: string
}

type CreateVoiceKickWire = {
  target_user_id: string
  channel_slug: string
  reason: string
}

type KickActionWire = {
  id: string
  guild_slug: string
  actor_user_id: string
  target_user_id: string
  reason: string
  created_at: string
  updated_at: string
}

type VoiceKickActionWire = {
  id: string
  guild_slug: string
  channel_slug: string
  actor_user_id: string
  target_user_id: string
  reason: string
  created_at: string
  updated_at: string
}

type CreateBanWire = {
  target_user_id: string
  reason: string
  delete_message_window: BanDeleteMessageWindow
}

type BanActionWire = {
  id: string
  ban_id: string
  guild_slug: string
  actor_user_id: string
  target_user_id: string
  reason: string
  delete_message_window: BanDeleteMessageWindow
  delete_messages_window_seconds?: number
  deleted_messages_count: number
  created_at: string
  updated_at: string
}

type ModerationLogEntryWire = {
  id: string
  action_type: ModerationLogActionType
  reason: string
  created_at: string
  actor_user_id: string
  actor_username: string
  actor_display_name: string
  actor_avatar_color?: string
  target_user_id: string
  target_username: string
  target_display_name: string
  target_avatar_color?: string
}

function normalizePathPart(value: string, field: string): string {
  const trimmed = value.trim()
  if (!trimmed) {
    throw new ApiError('VALIDATION_ERROR', `${field} is required`)
  }
  return encodeURIComponent(trimmed)
}

function normalizeRequiredText(value: string, field: string): string {
  const trimmed = value.trim()
  if (!trimmed) {
    throw new ApiError('VALIDATION_ERROR', `${field} is required`)
  }
  return trimmed
}

function normalizeDurationSeconds(
  value: number | null | undefined,
  required: boolean,
): number | null {
  if (value === null || value === undefined) {
    if (required) {
      throw new ApiError(
        'VALIDATION_ERROR',
        'durationSeconds is required unless isPermanent is true',
      )
    }
    return null
  }
  if (!Number.isFinite(value)) {
    throw new ApiError('VALIDATION_ERROR', 'durationSeconds must be finite')
  }
  const normalized = Math.trunc(value)
  if (normalized <= 0) {
    throw new ApiError(
      'VALIDATION_ERROR',
      'durationSeconds must be greater than zero',
    )
  }
  return normalized
}

function toMuteAction(wire: MuteActionWire): MuteAction {
  return {
    id: wire.id,
    guildSlug: wire.guild_slug,
    actorUserId: wire.actor_user_id,
    targetUserId: wire.target_user_id,
    reason: wire.reason,
    durationSeconds:
      typeof wire.duration_seconds === 'number' ? wire.duration_seconds : null,
    expiresAt: wire.expires_at ?? null,
    isPermanent: wire.is_permanent,
    createdAt: wire.created_at,
    updatedAt: wire.updated_at,
  }
}

function toMuteStatus(wire: MuteStatusWire): MuteStatus {
  return {
    active: wire.active,
    isPermanent: wire.is_permanent,
    expiresAt: wire.expires_at ?? null,
    reason: wire.reason ?? null,
  }
}

function toKickAction(wire: KickActionWire): KickAction {
  return {
    id: wire.id,
    guildSlug: wire.guild_slug,
    actorUserId: wire.actor_user_id,
    targetUserId: wire.target_user_id,
    reason: wire.reason,
    createdAt: wire.created_at,
    updatedAt: wire.updated_at,
  }
}

function toVoiceKickAction(wire: VoiceKickActionWire): VoiceKickAction {
  return {
    id: wire.id,
    guildSlug: wire.guild_slug,
    channelSlug: wire.channel_slug,
    actorUserId: wire.actor_user_id,
    targetUserId: wire.target_user_id,
    reason: wire.reason,
    createdAt: wire.created_at,
    updatedAt: wire.updated_at,
  }
}

function normalizeDeleteMessageWindow(value: string): BanDeleteMessageWindow {
  const normalized = value.trim().toLowerCase()
  if (
    normalized === 'none' ||
    normalized === '1h' ||
    normalized === '24h' ||
    normalized === '7d'
  ) {
    return normalized
  }
  throw new ApiError(
    'VALIDATION_ERROR',
    'deleteMessageWindow must be one of: none, 1h, 24h, 7d',
  )
}

function toBanAction(wire: BanActionWire): BanAction {
  return {
    id: wire.id,
    banId: wire.ban_id,
    guildSlug: wire.guild_slug,
    actorUserId: wire.actor_user_id,
    targetUserId: wire.target_user_id,
    reason: wire.reason,
    deleteMessageWindow: wire.delete_message_window,
    deleteMessagesWindowSeconds:
      typeof wire.delete_messages_window_seconds === 'number'
        ? wire.delete_messages_window_seconds
        : null,
    deletedMessagesCount: wire.deleted_messages_count,
    createdAt: wire.created_at,
    updatedAt: wire.updated_at,
  }
}

function toModerationLogEntry(
  wire: ModerationLogEntryWire,
): ModerationLogEntry {
  return {
    id: wire.id,
    actionType: wire.action_type,
    reason: wire.reason,
    createdAt: wire.created_at,
    actorUserId: wire.actor_user_id,
    actorUsername: wire.actor_username,
    actorDisplayName: wire.actor_display_name,
    actorAvatarColor: wire.actor_avatar_color ?? null,
    targetUserId: wire.target_user_id,
    targetUsername: wire.target_username,
    targetDisplayName: wire.target_display_name,
    targetAvatarColor: wire.target_avatar_color ?? null,
  }
}

function muteCreatePath(guildSlug: string): string {
  const guild = normalizePathPart(guildSlug, 'guildSlug')
  return `/api/v1/guilds/${guild}/moderation/mutes`
}

function muteStatusPath(guildSlug: string): string {
  const guild = normalizePathPart(guildSlug, 'guildSlug')
  return `/api/v1/guilds/${guild}/moderation/me/mute-status`
}

function kickCreatePath(guildSlug: string): string {
  const guild = normalizePathPart(guildSlug, 'guildSlug')
  return `/api/v1/guilds/${guild}/moderation/kicks`
}

function banCreatePath(guildSlug: string): string {
  const guild = normalizePathPart(guildSlug, 'guildSlug')
  return `/api/v1/guilds/${guild}/moderation/bans`
}

function voiceKickCreatePath(guildSlug: string): string {
  const guild = normalizePathPart(guildSlug, 'guildSlug')
  return `/api/v1/guilds/${guild}/moderation/voice-kicks`
}

function moderationLogPath(
  guildSlug: string,
  input: FetchModerationLogInput,
): string {
  const guild = normalizePathPart(guildSlug, 'guildSlug')
  const params = new URLSearchParams()

  if (input.limit !== undefined) {
    if (!Number.isFinite(input.limit)) {
      throw new ApiError('VALIDATION_ERROR', 'limit must be finite')
    }
    const normalizedLimit = Math.trunc(input.limit)
    if (normalizedLimit <= 0) {
      throw new ApiError('VALIDATION_ERROR', 'limit must be greater than zero')
    }
    params.set('limit', String(normalizedLimit))
  }

  if (input.cursor !== undefined && input.cursor !== null) {
    const normalizedCursor = input.cursor.trim()
    if (normalizedCursor) {
      params.set('cursor', normalizedCursor)
    }
  }

  if (input.order !== undefined) {
    if (input.order !== 'asc' && input.order !== 'desc') {
      throw new ApiError('VALIDATION_ERROR', 'order must be one of: asc, desc')
    }
    params.set('order', input.order)
  }

  if (input.actionType !== undefined && input.actionType !== null) {
    const actionType = input.actionType.trim()
    if (!actionType) {
      throw new ApiError('VALIDATION_ERROR', 'actionType must not be empty')
    }
    params.set('action_type', actionType)
  }

  const query = params.toString()
  if (!query) {
    return `/api/v1/guilds/${guild}/moderation/log`
  }
  return `/api/v1/guilds/${guild}/moderation/log?${query}`
}

export async function createMute(
  guildSlug: string,
  input: CreateMuteInput,
): Promise<MuteAction> {
  const targetUserId = normalizeRequiredText(input.targetUserId, 'targetUserId')
  const reason = normalizeRequiredText(input.reason, 'reason')
  const isPermanent = input.isPermanent === true
  const durationSeconds = normalizeDurationSeconds(
    input.durationSeconds,
    !isPermanent,
  )

  const payload: CreateMuteWire = {
    target_user_id: targetUserId,
    reason,
    is_permanent: isPermanent,
  }
  if (!isPermanent && durationSeconds !== null) {
    payload.duration_seconds = durationSeconds
  }

  const wire = await apiFetch<MuteActionWire>(muteCreatePath(guildSlug), {
    method: 'POST',
    body: JSON.stringify(payload),
  })
  return toMuteAction(wire)
}

export async function fetchMyMuteStatus(
  guildSlug: string,
): Promise<MuteStatus> {
  const wire = await apiFetch<MuteStatusWire>(muteStatusPath(guildSlug))
  return toMuteStatus(wire)
}

export async function createKick(
  guildSlug: string,
  input: CreateKickInput,
): Promise<KickAction> {
  const payload: CreateKickWire = {
    target_user_id: normalizeRequiredText(input.targetUserId, 'targetUserId'),
    reason: normalizeRequiredText(input.reason, 'reason'),
  }
  const wire = await apiFetch<KickActionWire>(kickCreatePath(guildSlug), {
    method: 'POST',
    body: JSON.stringify(payload),
  })
  return toKickAction(wire)
}

export async function createVoiceKick(
  guildSlug: string,
  input: CreateVoiceKickInput,
): Promise<VoiceKickAction> {
  const payload: CreateVoiceKickWire = {
    target_user_id: normalizeRequiredText(input.targetUserId, 'targetUserId'),
    channel_slug: normalizeRequiredText(input.channelSlug, 'channelSlug'),
    reason: normalizeRequiredText(input.reason, 'reason'),
  }
  const wire = await apiFetch<VoiceKickActionWire>(
    voiceKickCreatePath(guildSlug),
    {
      method: 'POST',
      body: JSON.stringify(payload),
    },
  )
  return toVoiceKickAction(wire)
}

export async function createBan(
  guildSlug: string,
  input: CreateBanInput,
): Promise<BanAction> {
  const payload: CreateBanWire = {
    target_user_id: normalizeRequiredText(input.targetUserId, 'targetUserId'),
    reason: normalizeRequiredText(input.reason, 'reason'),
    delete_message_window: normalizeDeleteMessageWindow(
      input.deleteMessageWindow,
    ),
  }
  const wire = await apiFetch<BanActionWire>(banCreatePath(guildSlug), {
    method: 'POST',
    body: JSON.stringify(payload),
  })
  return toBanAction(wire)
}

export async function fetchModerationLog(
  guildSlug: string,
  input: FetchModerationLogInput = {},
): Promise<ModerationLogPage> {
  const path = moderationLogPath(guildSlug, input)
  const page = await apiFetchCursorList<ModerationLogEntryWire[]>(path)
  return {
    entries: page.data.map(toModerationLogEntry),
    cursor: page.cursor,
  }
}
