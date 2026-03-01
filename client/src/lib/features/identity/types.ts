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

export type CrossInstanceChallengeInput = {
  username: string
  displayName?: string | null
  avatarColor?: string | null
}

export type RecoveryEmailStatus = {
  associated: boolean
  emailMasked: string | null
  verified: boolean
  verifiedAt: string | null
}

export type UserBlockEntry = {
  blockedUserId: string
  blockedAt: string
  unblockedAt: string | null
  blockedUserDisplayName?: string | null
  blockedUserUsername?: string | null
  blockedUserAvatarColor?: string | null
}

export type RecoveryEmailStatusWire = {
  associated: boolean
  email_masked?: string
  verified: boolean
  verified_at?: string
}

export type UserBlockEntryWire = {
  blocked_user_id?: string
  blocked_at?: string
  unblocked_at?: string | null
  blocked_user_display_name?: string | null
  blocked_user_username?: string | null
  blocked_user_avatar_color?: string | null
}

export type RecoveryEmailEncryptionContextInput = {
  algorithm: string
  version: number
}

export type StartRecoveryEmailInput = {
  email: string
  encryptedPrivateKey: string
  encryptionContext: RecoveryEmailEncryptionContextInput
}

export type StartRecoveryEmailInputWire = {
  email: string
  encrypted_private_key: string
  encryption_context: {
    algorithm: string
    version: number
  }
}

export type IdentityRecoveryStartResponse = {
  message: string
  helpMessage: string
}

export type IdentityRecoveryStartResponseWire = {
  message: string
  help_message: string
}

export type RecoveryIdentityPayload = {
  didKey: string
  username: string
  avatarColor: string | null
  registeredAt: string
  encryptedPrivateKey: string
  encryptionContext: RecoveryEmailEncryptionContextInput
}

export type RecoveryIdentityPayloadWire = {
  did_key: string
  username: string
  avatar_color?: string
  registered_at: string
  encrypted_private_key: string
  encryption_context: {
    algorithm: string
    version: number
  }
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

export function toRecoveryEmailStatus(
  wire: RecoveryEmailStatusWire,
): RecoveryEmailStatus {
  return {
    associated: wire.associated,
    emailMasked: wire.email_masked ?? null,
    verified: wire.verified,
    verifiedAt: wire.verified_at ?? null,
  }
}

export function toUserBlockEntry(wire: UserBlockEntryWire): UserBlockEntry {
  const blockedUserId = wire.blocked_user_id?.trim()
  const blockedAt = wire.blocked_at?.trim()
  const unblockedAt =
    typeof wire.unblocked_at === 'string' ? wire.unblocked_at.trim() : null
  const blockedUserDisplayName =
    typeof wire.blocked_user_display_name === 'string'
      ? wire.blocked_user_display_name.trim()
      : null
  const blockedUserUsername =
    typeof wire.blocked_user_username === 'string'
      ? wire.blocked_user_username.trim()
      : null
  const blockedUserAvatarColor =
    typeof wire.blocked_user_avatar_color === 'string'
      ? wire.blocked_user_avatar_color.trim()
      : null
  if (!blockedUserId || !blockedAt) {
    throw new Error('Invalid user block response')
  }
  return {
    blockedUserId,
    blockedAt,
    unblockedAt: unblockedAt && unblockedAt.length > 0 ? unblockedAt : null,
    blockedUserDisplayName:
      blockedUserDisplayName && blockedUserDisplayName.length > 0
        ? blockedUserDisplayName
        : null,
    blockedUserUsername:
      blockedUserUsername && blockedUserUsername.length > 0
        ? blockedUserUsername
        : null,
    blockedUserAvatarColor:
      blockedUserAvatarColor && blockedUserAvatarColor.length > 0
        ? blockedUserAvatarColor
        : null,
  }
}

export function toStartRecoveryEmailInputWire(
  input: StartRecoveryEmailInput,
): StartRecoveryEmailInputWire {
  return {
    email: input.email,
    encrypted_private_key: input.encryptedPrivateKey,
    encryption_context: {
      algorithm: input.encryptionContext.algorithm,
      version: input.encryptionContext.version,
    },
  }
}

export function toIdentityRecoveryStartResponse(
  wire: IdentityRecoveryStartResponseWire,
): IdentityRecoveryStartResponse {
  return {
    message: wire.message,
    helpMessage: wire.help_message,
  }
}

export function toRecoveryIdentityPayload(
  wire: RecoveryIdentityPayloadWire,
): RecoveryIdentityPayload {
  return {
    didKey: wire.did_key,
    username: wire.username,
    avatarColor: wire.avatar_color ?? null,
    registeredAt: wire.registered_at,
    encryptedPrivateKey: wire.encrypted_private_key,
    encryptionContext: {
      algorithm: wire.encryption_context.algorithm,
      version: wire.encryption_context.version,
    },
  }
}
