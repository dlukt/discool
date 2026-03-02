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

export type CreateMessageDeleteInput = {
  messageId: string
  channelSlug: string
  reason: string
}

export type ReportCategory = 'spam' | 'harassment' | 'rule_violation' | 'other'

export type CreateMessageReportInput = {
  messageId: string
  reason: string
  category?: ReportCategory | null
}

export type CreateUserReportInput = {
  targetUserId: string
  reason: string
  category?: ReportCategory | null
}

export type UserContentReport = {
  id: string
  guildSlug: string
  reporterUserId: string
  targetType: 'message' | 'user'
  targetMessageId: string | null
  targetUserId: string | null
  reason: string
  category: ReportCategory | null
  status: 'pending' | 'reviewed' | 'actioned' | 'dismissed'
  createdAt: string
  updatedAt: string
}

export type ReportQueueStatus =
  | 'pending'
  | 'reviewed'
  | 'actioned'
  | 'dismissed'

export type ReportQueueActionType = 'warn' | 'mute' | 'kick' | 'ban'

export type FetchReportQueueInput = {
  limit?: number
  cursor?: string | null
  status?: ReportQueueStatus | null
}

export type DismissReportInput = {
  dismissalReason?: string | null
}

export type ActOnReportInput = {
  actionType: ReportQueueActionType
  reason?: string | null
  durationSeconds?: number | null
  deleteMessageWindow?: BanDeleteMessageWindow | null
}

export type ReportQueueItem = {
  id: string
  guildSlug: string
  reporterUserId: string
  reporterUsername: string
  reporterDisplayName: string
  reporterAvatarColor: string | null
  targetType: 'message' | 'user'
  targetMessageId: string | null
  targetUserId: string | null
  targetUsername: string | null
  targetDisplayName: string | null
  targetAvatarColor: string | null
  targetMessagePreview: string | null
  reason: string
  category: ReportCategory | null
  status: ReportQueueStatus
  reviewedAt: string | null
  actionedAt: string | null
  dismissedAt: string | null
  dismissalReason: string | null
  moderationActionId: string | null
  createdAt: string
  updatedAt: string
}

export type ReportQueuePage = {
  entries: ReportQueueItem[]
  cursor: string | null
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

export type MessageDeleteAction = {
  id: string
  messageId: string
  guildSlug: string
  channelSlug: string
  actorUserId: string
  targetUserId: string
  reason: string
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

export type FetchUserMessageHistoryInput = {
  limit?: number
  cursor?: string | null
  channelSlug?: string | null
  from?: string | null
  to?: string | null
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

export type UserMessageHistoryEntry = {
  id: string
  channelSlug: string
  channelName: string
  content: string
  createdAt: string
}

export type UserMessageHistoryPage = {
  entries: UserMessageHistoryEntry[]
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

type CreateMessageDeleteWire = {
  channel_slug: string
  reason: string
}

type MessageDeleteActionWire = {
  id: string
  message_id: string
  guild_slug: string
  channel_slug: string
  actor_user_id: string
  target_user_id: string
  reason: string
  created_at: string
  updated_at: string
}

type CreateReportWire = {
  reason: string
  category?: ReportCategory
}

type UserContentReportWire = {
  id: string
  guild_slug: string
  reporter_user_id: string
  target_type: 'message' | 'user'
  target_message_id?: string
  target_user_id?: string
  reason: string
  category?: ReportCategory
  status: 'pending' | 'reviewed' | 'actioned' | 'dismissed'
  created_at: string
  updated_at: string
}

type ReportQueueItemWire = {
  id: string
  guild_slug: string
  reporter_user_id: string
  reporter_username: string
  reporter_display_name: string
  reporter_avatar_color?: string
  target_type: 'message' | 'user'
  target_message_id?: string
  target_user_id?: string
  target_username?: string
  target_display_name?: string
  target_avatar_color?: string
  target_message_preview?: string
  reason: string
  category?: ReportCategory
  status: ReportQueueStatus
  reviewed_at?: string
  actioned_at?: string
  dismissed_at?: string
  dismissal_reason?: string
  moderation_action_id?: string
  created_at: string
  updated_at: string
}

type DismissReportWire = {
  dismissal_reason?: string
}

type ActOnReportWire = {
  action_type: ReportQueueActionType
  reason?: string
  duration_seconds?: number
  delete_message_window?: BanDeleteMessageWindow
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

type UserMessageHistoryEntryWire = {
  id: string
  channel_slug: string
  channel_name: string
  content: string
  created_at: string
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

function normalizeOptionalReportCategory(
  value: string | null | undefined,
): ReportCategory | null {
  if (value === undefined || value === null) return null
  const normalized = value.trim().toLowerCase()
  if (!normalized) return null
  if (
    normalized === 'spam' ||
    normalized === 'harassment' ||
    normalized === 'rule_violation' ||
    normalized === 'other'
  ) {
    return normalized
  }
  throw new ApiError(
    'VALIDATION_ERROR',
    'category must be one of: spam, harassment, rule_violation, other',
  )
}

function normalizeReportQueueStatus(
  value: string | null | undefined,
): ReportQueueStatus | null {
  if (value === undefined || value === null) return null
  const normalized = value.trim().toLowerCase()
  if (!normalized) return null
  if (
    normalized === 'pending' ||
    normalized === 'reviewed' ||
    normalized === 'actioned' ||
    normalized === 'dismissed'
  ) {
    return normalized
  }
  throw new ApiError(
    'VALIDATION_ERROR',
    'status must be one of: pending, reviewed, actioned, dismissed',
  )
}

function normalizeReportQueueActionType(value: string): ReportQueueActionType {
  const normalized = value.trim().toLowerCase()
  if (
    normalized === 'warn' ||
    normalized === 'mute' ||
    normalized === 'kick' ||
    normalized === 'ban'
  ) {
    return normalized
  }
  throw new ApiError(
    'VALIDATION_ERROR',
    'actionType must be one of: warn, mute, kick, ban',
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

function toMessageDeleteAction(
  wire: MessageDeleteActionWire,
): MessageDeleteAction {
  return {
    id: wire.id,
    messageId: wire.message_id,
    guildSlug: wire.guild_slug,
    channelSlug: wire.channel_slug,
    actorUserId: wire.actor_user_id,
    targetUserId: wire.target_user_id,
    reason: wire.reason,
    createdAt: wire.created_at,
    updatedAt: wire.updated_at,
  }
}

function toUserContentReport(wire: UserContentReportWire): UserContentReport {
  return {
    id: wire.id,
    guildSlug: wire.guild_slug,
    reporterUserId: wire.reporter_user_id,
    targetType: wire.target_type,
    targetMessageId: wire.target_message_id ?? null,
    targetUserId: wire.target_user_id ?? null,
    reason: wire.reason,
    category: wire.category ?? null,
    status: wire.status,
    createdAt: wire.created_at,
    updatedAt: wire.updated_at,
  }
}

function toReportQueueItem(wire: ReportQueueItemWire): ReportQueueItem {
  return {
    id: wire.id,
    guildSlug: wire.guild_slug,
    reporterUserId: wire.reporter_user_id,
    reporterUsername: wire.reporter_username,
    reporterDisplayName: wire.reporter_display_name,
    reporterAvatarColor: wire.reporter_avatar_color ?? null,
    targetType: wire.target_type,
    targetMessageId: wire.target_message_id ?? null,
    targetUserId: wire.target_user_id ?? null,
    targetUsername: wire.target_username ?? null,
    targetDisplayName: wire.target_display_name ?? null,
    targetAvatarColor: wire.target_avatar_color ?? null,
    targetMessagePreview: wire.target_message_preview ?? null,
    reason: wire.reason,
    category: wire.category ?? null,
    status: wire.status,
    reviewedAt: wire.reviewed_at ?? null,
    actionedAt: wire.actioned_at ?? null,
    dismissedAt: wire.dismissed_at ?? null,
    dismissalReason: wire.dismissal_reason ?? null,
    moderationActionId: wire.moderation_action_id ?? null,
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

function toUserMessageHistoryEntry(
  wire: UserMessageHistoryEntryWire,
): UserMessageHistoryEntry {
  return {
    id: wire.id,
    channelSlug: wire.channel_slug,
    channelName: wire.channel_name,
    content: wire.content,
    createdAt: wire.created_at,
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

function messageDeleteCreatePath(guildSlug: string, messageId: string): string {
  const guild = normalizePathPart(guildSlug, 'guildSlug')
  const message = normalizePathPart(messageId, 'messageId')
  return `/api/v1/guilds/${guild}/moderation/messages/${message}/delete`
}

function messageReportCreatePath(guildSlug: string, messageId: string): string {
  const guild = normalizePathPart(guildSlug, 'guildSlug')
  const message = normalizePathPart(messageId, 'messageId')
  return `/api/v1/guilds/${guild}/moderation/reports/messages/${message}`
}

function userReportCreatePath(guildSlug: string, targetUserId: string): string {
  const guild = normalizePathPart(guildSlug, 'guildSlug')
  const target = normalizePathPart(targetUserId, 'targetUserId')
  return `/api/v1/guilds/${guild}/moderation/reports/users/${target}`
}

function reportQueuePath(
  guildSlug: string,
  input: FetchReportQueueInput,
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

  if (input.status !== undefined && input.status !== null) {
    const status = normalizeReportQueueStatus(input.status)
    if (status !== null) {
      params.set('status', status)
    }
  }

  const query = params.toString()
  if (!query) {
    return `/api/v1/guilds/${guild}/moderation/reports`
  }
  return `/api/v1/guilds/${guild}/moderation/reports?${query}`
}

function reportReviewPath(guildSlug: string, reportId: string): string {
  const guild = normalizePathPart(guildSlug, 'guildSlug')
  const report = normalizePathPart(reportId, 'reportId')
  return `/api/v1/guilds/${guild}/moderation/reports/${report}/review`
}

function reportDismissPath(guildSlug: string, reportId: string): string {
  const guild = normalizePathPart(guildSlug, 'guildSlug')
  const report = normalizePathPart(reportId, 'reportId')
  return `/api/v1/guilds/${guild}/moderation/reports/${report}/dismiss`
}

function reportActionPath(guildSlug: string, reportId: string): string {
  const guild = normalizePathPart(guildSlug, 'guildSlug')
  const report = normalizePathPart(reportId, 'reportId')
  return `/api/v1/guilds/${guild}/moderation/reports/${report}/actions`
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

function userMessageHistoryPath(
  guildSlug: string,
  targetUserId: string,
  input: FetchUserMessageHistoryInput,
): string {
  const guild = normalizePathPart(guildSlug, 'guildSlug')
  const target = normalizePathPart(targetUserId, 'targetUserId')
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

  if (input.channelSlug !== undefined && input.channelSlug !== null) {
    const normalizedChannelSlug = input.channelSlug.trim()
    if (normalizedChannelSlug) {
      params.set('channel_slug', normalizedChannelSlug)
    }
  }

  if (input.from !== undefined && input.from !== null) {
    const normalizedFrom = input.from.trim()
    if (normalizedFrom) {
      params.set('from', normalizedFrom)
    }
  }

  if (input.to !== undefined && input.to !== null) {
    const normalizedTo = input.to.trim()
    if (normalizedTo) {
      params.set('to', normalizedTo)
    }
  }

  const query = params.toString()
  if (!query) {
    return `/api/v1/guilds/${guild}/moderation/users/${target}/messages`
  }
  return `/api/v1/guilds/${guild}/moderation/users/${target}/messages?${query}`
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

export async function createMessageDelete(
  guildSlug: string,
  input: CreateMessageDeleteInput,
): Promise<MessageDeleteAction> {
  const payload: CreateMessageDeleteWire = {
    channel_slug: normalizeRequiredText(input.channelSlug, 'channelSlug'),
    reason: normalizeRequiredText(input.reason, 'reason'),
  }
  const wire = await apiFetch<MessageDeleteActionWire>(
    messageDeleteCreatePath(guildSlug, input.messageId),
    {
      method: 'POST',
      body: JSON.stringify(payload),
    },
  )
  return toMessageDeleteAction(wire)
}

export async function createMessageReport(
  guildSlug: string,
  input: CreateMessageReportInput,
): Promise<UserContentReport> {
  const payload: CreateReportWire = {
    reason: normalizeRequiredText(input.reason, 'reason'),
  }
  const category = normalizeOptionalReportCategory(input.category)
  if (category !== null) {
    payload.category = category
  }
  const wire = await apiFetch<UserContentReportWire>(
    messageReportCreatePath(guildSlug, input.messageId),
    {
      method: 'POST',
      body: JSON.stringify(payload),
    },
  )
  return toUserContentReport(wire)
}

export async function createUserReport(
  guildSlug: string,
  input: CreateUserReportInput,
): Promise<UserContentReport> {
  const payload: CreateReportWire = {
    reason: normalizeRequiredText(input.reason, 'reason'),
  }
  const category = normalizeOptionalReportCategory(input.category)
  if (category !== null) {
    payload.category = category
  }
  const wire = await apiFetch<UserContentReportWire>(
    userReportCreatePath(guildSlug, input.targetUserId),
    {
      method: 'POST',
      body: JSON.stringify(payload),
    },
  )
  return toUserContentReport(wire)
}

export async function fetchReportQueue(
  guildSlug: string,
  input: FetchReportQueueInput = {},
): Promise<ReportQueuePage> {
  const path = reportQueuePath(guildSlug, input)
  const page = await apiFetchCursorList<ReportQueueItemWire[]>(path)
  return {
    entries: page.data.map(toReportQueueItem),
    cursor: page.cursor,
  }
}

export async function reviewReport(
  guildSlug: string,
  reportId: string,
): Promise<ReportQueueItem> {
  const wire = await apiFetch<ReportQueueItemWire>(
    reportReviewPath(guildSlug, reportId),
    {
      method: 'POST',
    },
  )
  return toReportQueueItem(wire)
}

export async function dismissReport(
  guildSlug: string,
  reportId: string,
  input: DismissReportInput = {},
): Promise<ReportQueueItem> {
  const payload: DismissReportWire = {}
  if (input.dismissalReason !== undefined && input.dismissalReason !== null) {
    const normalizedReason = input.dismissalReason.trim()
    if (normalizedReason) {
      payload.dismissal_reason = normalizedReason
    }
  }
  const wire = await apiFetch<ReportQueueItemWire>(
    reportDismissPath(guildSlug, reportId),
    {
      method: 'POST',
      body: JSON.stringify(payload),
    },
  )
  return toReportQueueItem(wire)
}

export async function actOnReport(
  guildSlug: string,
  reportId: string,
  input: ActOnReportInput,
): Promise<ReportQueueItem> {
  const payload: ActOnReportWire = {
    action_type: normalizeReportQueueActionType(input.actionType),
  }
  if (input.reason !== undefined && input.reason !== null) {
    const normalizedReason = input.reason.trim()
    if (normalizedReason) {
      payload.reason = normalizedReason
    }
  }
  if (input.durationSeconds !== undefined && input.durationSeconds !== null) {
    if (!Number.isFinite(input.durationSeconds)) {
      throw new ApiError('VALIDATION_ERROR', 'durationSeconds must be finite')
    }
    const normalized = Math.trunc(input.durationSeconds)
    if (normalized <= 0) {
      throw new ApiError(
        'VALIDATION_ERROR',
        'durationSeconds must be greater than zero',
      )
    }
    payload.duration_seconds = normalized
  }
  if (
    input.deleteMessageWindow !== undefined &&
    input.deleteMessageWindow !== null
  ) {
    payload.delete_message_window = normalizeDeleteMessageWindow(
      input.deleteMessageWindow,
    )
  }
  const wire = await apiFetch<ReportQueueItemWire>(
    reportActionPath(guildSlug, reportId),
    {
      method: 'POST',
      body: JSON.stringify(payload),
    },
  )
  return toReportQueueItem(wire)
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

export async function fetchUserMessageHistory(
  guildSlug: string,
  targetUserId: string,
  input: FetchUserMessageHistoryInput = {},
): Promise<UserMessageHistoryPage> {
  const path = userMessageHistoryPath(guildSlug, targetUserId, input)
  const page = await apiFetchCursorList<UserMessageHistoryEntryWire[]>(path)
  return {
    entries: page.data.map(toUserMessageHistoryEntry),
    cursor: page.cursor,
  }
}
