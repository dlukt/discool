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

export type DeleteAccountInput = {
  confirmUsername: string
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

export type PersonalDataExportProfile = {
  userId: string
  didKey: string
  username: string
  displayName: string
  avatarColor: string | null
  avatarUrl: string | null
  email: string | null
  emailVerifiedAt: string | null
  createdAt: string
  updatedAt: string
}

export type PersonalDataExportGuildMembership = {
  guildId: string
  joinedAt: string
  joinedViaInviteCode: string | null
}

export type PersonalDataExportMessage = {
  id: string
  guildId: string
  channelId: string
  authorUserId: string
  content: string
  isSystem: boolean
  createdAt: string
  updatedAt: string
}

export type PersonalDataExportDmMessage = {
  id: string
  dmChannelId: string
  authorUserId: string
  content: string
  isSystem: boolean
  createdAt: string
  updatedAt: string
}

export type PersonalDataExportReaction = {
  messageId: string
  emoji: string
  createdAt: string
}

export type PersonalDataExportUploadedFile = {
  id: string
  messageId: string
  storageKey: string
  originalFilename: string
  mimeType: string
  sizeBytes: number
  createdAt: string
}

export type PersonalDataExportBlockEntry = {
  blockedUserId: string
  blockedAt: string
  unblockedAt: string | null
}

export type PersonalDataExport = {
  profile: PersonalDataExportProfile
  guildMemberships: PersonalDataExportGuildMembership[]
  messages: PersonalDataExportMessage[]
  dmMessages: PersonalDataExportDmMessage[]
  reactions: PersonalDataExportReaction[]
  uploadedFiles: PersonalDataExportUploadedFile[]
  blockList: PersonalDataExportBlockEntry[]
  exportedAt: string
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

export type PersonalDataExportProfileWire = {
  user_id?: string
  did_key?: string
  username?: string
  display_name?: string
  avatar_color?: string | null
  avatar_url?: string | null
  email?: string | null
  email_verified_at?: string | null
  created_at?: string
  updated_at?: string
}

export type PersonalDataExportGuildMembershipWire = {
  guild_id?: string
  joined_at?: string
  joined_via_invite_code?: string | null
}

export type PersonalDataExportMessageWire = {
  id?: string
  guild_id?: string
  channel_id?: string
  author_user_id?: string
  content?: string
  is_system?: boolean
  created_at?: string
  updated_at?: string
}

export type PersonalDataExportDmMessageWire = {
  id?: string
  dm_channel_id?: string
  author_user_id?: string
  content?: string
  is_system?: boolean
  created_at?: string
  updated_at?: string
}

export type PersonalDataExportReactionWire = {
  message_id?: string
  emoji?: string
  created_at?: string
}

export type PersonalDataExportUploadedFileWire = {
  id?: string
  message_id?: string
  storage_key?: string
  original_filename?: string
  mime_type?: string
  size_bytes?: number
  created_at?: string
}

export type PersonalDataExportBlockEntryWire = {
  blocked_user_id?: string
  blocked_at?: string
  unblocked_at?: string | null
}

export type PersonalDataExportWire = {
  profile?: PersonalDataExportProfileWire
  guild_memberships?: PersonalDataExportGuildMembershipWire[]
  messages?: PersonalDataExportMessageWire[]
  dm_messages?: PersonalDataExportDmMessageWire[]
  reactions?: PersonalDataExportReactionWire[]
  uploaded_files?: PersonalDataExportUploadedFileWire[]
  block_list?: PersonalDataExportBlockEntryWire[]
  exported_at?: string
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

export type DeleteAccountInputWire = {
  confirm_username: string
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

export function toDeleteAccountInputWire(
  input: DeleteAccountInput,
): DeleteAccountInputWire {
  return {
    confirm_username: input.confirmUsername,
  }
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

export function toPersonalDataExport(
  wire: PersonalDataExportWire,
): PersonalDataExport {
  const profile = wire.profile
  if (
    !profile?.user_id ||
    !profile.did_key ||
    !profile.username ||
    !profile.display_name ||
    !profile.created_at ||
    !profile.updated_at
  ) {
    throw new Error('Invalid personal data export response')
  }
  if (!wire.exported_at) {
    throw new Error('Invalid personal data export response')
  }

  return {
    profile: {
      userId: profile.user_id,
      didKey: profile.did_key,
      username: profile.username,
      displayName: profile.display_name,
      avatarColor: profile.avatar_color ?? null,
      avatarUrl: profile.avatar_url ?? null,
      email: profile.email ?? null,
      emailVerifiedAt: profile.email_verified_at ?? null,
      createdAt: profile.created_at,
      updatedAt: profile.updated_at,
    },
    guildMemberships: (wire.guild_memberships ?? []).flatMap((entry) =>
      entry.guild_id && entry.joined_at
        ? [
            {
              guildId: entry.guild_id,
              joinedAt: entry.joined_at,
              joinedViaInviteCode: entry.joined_via_invite_code ?? null,
            },
          ]
        : [],
    ),
    messages: (wire.messages ?? []).flatMap((entry) =>
      entry.id &&
      entry.guild_id &&
      entry.channel_id &&
      entry.author_user_id &&
      typeof entry.content === 'string' &&
      typeof entry.is_system === 'boolean' &&
      entry.created_at &&
      entry.updated_at
        ? [
            {
              id: entry.id,
              guildId: entry.guild_id,
              channelId: entry.channel_id,
              authorUserId: entry.author_user_id,
              content: entry.content,
              isSystem: entry.is_system,
              createdAt: entry.created_at,
              updatedAt: entry.updated_at,
            },
          ]
        : [],
    ),
    dmMessages: (wire.dm_messages ?? []).flatMap((entry) =>
      entry.id &&
      entry.dm_channel_id &&
      entry.author_user_id &&
      typeof entry.content === 'string' &&
      typeof entry.is_system === 'boolean' &&
      entry.created_at &&
      entry.updated_at
        ? [
            {
              id: entry.id,
              dmChannelId: entry.dm_channel_id,
              authorUserId: entry.author_user_id,
              content: entry.content,
              isSystem: entry.is_system,
              createdAt: entry.created_at,
              updatedAt: entry.updated_at,
            },
          ]
        : [],
    ),
    reactions: (wire.reactions ?? []).flatMap((entry) =>
      entry.message_id && entry.emoji && entry.created_at
        ? [
            {
              messageId: entry.message_id,
              emoji: entry.emoji,
              createdAt: entry.created_at,
            },
          ]
        : [],
    ),
    uploadedFiles: (wire.uploaded_files ?? []).flatMap((entry) =>
      entry.id &&
      entry.message_id &&
      entry.storage_key &&
      entry.original_filename &&
      entry.mime_type &&
      typeof entry.size_bytes === 'number' &&
      entry.created_at
        ? [
            {
              id: entry.id,
              messageId: entry.message_id,
              storageKey: entry.storage_key,
              originalFilename: entry.original_filename,
              mimeType: entry.mime_type,
              sizeBytes: entry.size_bytes,
              createdAt: entry.created_at,
            },
          ]
        : [],
    ),
    blockList: (wire.block_list ?? []).flatMap((entry) =>
      entry.blocked_user_id && entry.blocked_at
        ? [
            {
              blockedUserId: entry.blocked_user_id,
              blockedAt: entry.blocked_at,
              unblockedAt:
                typeof entry.unblocked_at === 'string'
                  ? entry.unblocked_at
                  : null,
            },
          ]
        : [],
    ),
    exportedAt: wire.exported_at,
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
