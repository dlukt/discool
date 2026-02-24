import { apiFetch } from '$lib/api'

import {
  type AuthSession,
  type RegisteredUser,
  type RegisteredUserWire,
  toRegisteredUser,
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
}

type ChallengeResponseWire = {
  challenge: string
  expires_in: number
}

type VerifyRequestWire = {
  did_key: string
  challenge: string
  signature: string
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
): Promise<{ challenge: string; expiresIn: number }> {
  const body: ChallengeRequestWire = { did_key: didKey }
  return apiFetch<ChallengeResponseWire>('/api/v1/auth/challenge', {
    method: 'POST',
    body: JSON.stringify(body),
  }).then((wire) => ({ challenge: wire.challenge, expiresIn: wire.expires_in }))
}

export function verifyChallenge(
  didKey: string,
  challenge: string,
  signature: string,
): Promise<AuthSession> {
  const body: VerifyRequestWire = { did_key: didKey, challenge, signature }
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
