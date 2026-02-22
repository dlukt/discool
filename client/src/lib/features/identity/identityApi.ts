import { apiFetch } from '$lib/api'

import type { RegisteredUser } from './types'

type RegisterRequestWire = {
  did_key: string
  username: string
  avatar_color?: string
}

type RegisteredUserWire = {
  id: string
  did_key: string
  username: string
  avatar_color?: string
  created_at: string
}

function toRegisteredUser(wire: RegisteredUserWire): RegisteredUser {
  return {
    id: wire.id,
    didKey: wire.did_key,
    username: wire.username,
    avatarColor: wire.avatar_color ?? null,
    createdAt: wire.created_at,
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
