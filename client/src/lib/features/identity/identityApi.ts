import { apiFetch } from '$lib/api'

import {
  type AuthSession,
  type CrossInstanceChallengeInput,
  type IdentityRecoveryStartResponse,
  type IdentityRecoveryStartResponseWire,
  type RecoveryEmailStatus,
  type RecoveryEmailStatusWire,
  type RecoveryIdentityPayload,
  type RecoveryIdentityPayloadWire,
  type RegisteredUser,
  type RegisteredUserWire,
  type StartRecoveryEmailInput,
  toIdentityRecoveryStartResponse,
  toRecoveryEmailStatus,
  toRecoveryIdentityPayload,
  toRegisteredUser,
  toStartRecoveryEmailInputWire,
  toUpdateProfileInputWire,
  type UpdateProfileInput,
} from './types'

type RegisterRequestWire = {
  did_key: string
  username: string
  avatar_color?: string
}

type ChallengeRequestWire = {
  did_key: string
  cross_instance?: CrossInstanceChallengeWire
}

type ChallengeResponseWire = {
  challenge: string
  expires_in: number
}

type CrossInstanceChallengeWire = {
  enabled: true
  username: string
  display_name?: string
  avatar_color?: string
}

type VerifyRequestWire = {
  did_key: string
  challenge: string
  signature: string
  cross_instance?: CrossInstanceVerifyWire
}

type CrossInstanceVerifyWire = {
  enabled: true
}

type VerifyResponseWire = {
  token: string
  expires_at: string
  user: RegisteredUserWire
}

function toAuthSession(wire: VerifyResponseWire): AuthSession {
  return {
    token: wire.token,
    expiresAt: wire.expires_at,
    user: toRegisteredUser(wire.user),
  }
}

function toRegisterRequestWire(
  didKey: string,
  username: string,
  avatarColor?: string,
): RegisterRequestWire {
  return {
    did_key: didKey,
    username,
    avatar_color: avatarColor,
  }
}

function toCrossInstanceChallengeWire(
  input: CrossInstanceChallengeInput,
): CrossInstanceChallengeWire {
  const wire: CrossInstanceChallengeWire = {
    enabled: true,
    username: input.username,
  }
  const displayName = input.displayName?.trim()
  if (displayName) {
    wire.display_name = displayName
  }
  const avatarColor = input.avatarColor?.trim()
  if (avatarColor) {
    wire.avatar_color = avatarColor
  }
  return wire
}

export function register(
  didKey: string,
  username: string,
  avatarColor?: string,
): Promise<RegisteredUser> {
  return apiFetch<RegisteredUserWire>('/api/v1/auth/register', {
    method: 'POST',
    body: JSON.stringify(toRegisterRequestWire(didKey, username, avatarColor)),
  }).then(toRegisteredUser)
}

export function requestChallenge(
  didKey: string,
  crossInstance?: CrossInstanceChallengeInput,
): Promise<{ challenge: string; expiresIn: number }> {
  const body: ChallengeRequestWire = {
    did_key: didKey,
    cross_instance: crossInstance
      ? toCrossInstanceChallengeWire(crossInstance)
      : undefined,
  }
  return apiFetch<ChallengeResponseWire>('/api/v1/auth/challenge', {
    method: 'POST',
    body: JSON.stringify(body),
  }).then((wire) => ({ challenge: wire.challenge, expiresIn: wire.expires_in }))
}

export function verifyChallenge(
  didKey: string,
  challenge: string,
  signature: string,
  crossInstance = false,
): Promise<AuthSession> {
  const body: VerifyRequestWire = {
    did_key: didKey,
    challenge,
    signature,
    cross_instance: crossInstance ? { enabled: true } : undefined,
  }
  return apiFetch<VerifyResponseWire>('/api/v1/auth/verify', {
    method: 'POST',
    body: JSON.stringify(body),
  }).then(toAuthSession)
}

export function logout(token: string): Promise<void> {
  return apiFetch<void>('/api/v1/auth/logout', {
    method: 'DELETE',
    headers: { authorization: `Bearer ${token}` },
  })
}

export function getProfile(): Promise<RegisteredUser> {
  return apiFetch<RegisteredUserWire>('/api/v1/users/me/profile').then(
    toRegisteredUser,
  )
}

export function updateProfile(
  input: UpdateProfileInput,
): Promise<RegisteredUser> {
  return apiFetch<RegisteredUserWire>('/api/v1/users/me/profile', {
    method: 'PATCH',
    body: JSON.stringify(toUpdateProfileInputWire(input)),
  }).then(toRegisteredUser)
}

export function uploadAvatar(file: File): Promise<RegisteredUser> {
  const formData = new FormData()
  formData.append('avatar', file)
  return apiFetch<RegisteredUserWire>('/api/v1/users/me/avatar', {
    method: 'POST',
    body: formData,
  }).then(toRegisteredUser)
}

export function getRecoveryEmailStatus(): Promise<RecoveryEmailStatus> {
  return apiFetch<RecoveryEmailStatusWire>(
    '/api/v1/users/me/recovery-email',
  ).then(toRecoveryEmailStatus)
}

export function startRecoveryEmailAssociation(
  input: StartRecoveryEmailInput,
): Promise<RecoveryEmailStatus> {
  return apiFetch<RecoveryEmailStatusWire>('/api/v1/users/me/recovery-email', {
    method: 'POST',
    body: JSON.stringify(toStartRecoveryEmailInputWire(input)),
  }).then(toRecoveryEmailStatus)
}

export function startIdentityRecovery(
  email: string,
): Promise<IdentityRecoveryStartResponse> {
  return apiFetch<IdentityRecoveryStartResponseWire>(
    '/api/v1/auth/recovery-email/start',
    {
      method: 'POST',
      body: JSON.stringify({ email }),
    },
  ).then(toIdentityRecoveryStartResponse)
}

export function recoverIdentityByToken(
  token: string,
): Promise<RecoveryIdentityPayload> {
  const query = new URLSearchParams({ token })
  return apiFetch<RecoveryIdentityPayloadWire>(
    `/api/v1/auth/recovery-email/recover?${query.toString()}`,
  ).then(toRecoveryIdentityPayload)
}
