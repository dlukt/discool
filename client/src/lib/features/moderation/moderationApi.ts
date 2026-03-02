import { ApiError, apiFetch } from '$lib/api'

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

type KickActionWire = {
  id: string
  guild_slug: string
  actor_user_id: string
  target_user_id: string
  reason: string
  created_at: string
  updated_at: string
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
