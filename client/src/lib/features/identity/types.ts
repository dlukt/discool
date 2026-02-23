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
  avatarColor: string | null
  createdAt: string
}

export type AuthSession = {
  token: string
  expiresAt: string
  user: RegisteredUser
}
