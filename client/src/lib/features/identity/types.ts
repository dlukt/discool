export type StoredIdentity = {
  publicKey: Uint8Array
  didKey: string
  username: string
  avatarColor: string | null
  registeredAt: string
}

export type RegisteredUser = {
  id: string
  didKey: string
  username: string
  displayName: string
  avatarColor: string | null
  avatarUrl: string | null
  createdAt: string
}

export type AuthSession = {
  token: string
  expiresAt: string
  user: RegisteredUser
}

export type RegisteredUserWire = {
  id: string
  did_key: string
  username: string
  display_name: string
  avatar_color?: string
  avatar_url?: string
  created_at: string
}

export type UpdateProfileInput = {
  displayName?: string | null
  avatarColor?: string | null
}

export type UpdateProfileInputWire = {
  display_name?: string | null
  avatar_color?: string | null
}

export function toRegisteredUser(wire: RegisteredUserWire): RegisteredUser {
  return {
    id: wire.id,
    didKey: wire.did_key,
    username: wire.username,
    displayName: wire.display_name,
    avatarColor: wire.avatar_color ?? null,
    avatarUrl: wire.avatar_url ?? null,
    createdAt: wire.created_at,
  }
}

export function toUpdateProfileInputWire(
  input: UpdateProfileInput,
): UpdateProfileInputWire {
  const wire: UpdateProfileInputWire = {}
  if ('displayName' in input) {
    wire.display_name = input.displayName ?? null
  }
  if ('avatarColor' in input) {
    wire.avatar_color = input.avatarColor ?? null
  }
  return wire
}
