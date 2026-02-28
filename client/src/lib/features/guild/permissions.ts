export type GuildPermissionKey =
  | 'VIEW_CHANNEL'
  | 'SEND_MESSAGES'
  | 'MANAGE_CHANNELS'
  | 'KICK_MEMBERS'
  | 'BAN_MEMBERS'
  | 'MANAGE_ROLES'
  | 'MANAGE_GUILD'
  | 'MANAGE_INVITES'
  | 'MUTE_MEMBERS'
  | 'VIEW_MOD_LOG'
  | 'ATTACH_FILES'
  | 'ADD_REACTIONS'
  | 'MANAGE_MESSAGES'

export type GuildPermissionDefinition = {
  key: GuildPermissionKey
  label: string
  description: string
  bit: number
  mask: number
}

const permissionSpecs: Array<{
  key: GuildPermissionKey
  label: string
  description: string
  bit: number
}> = [
  {
    key: 'VIEW_CHANNEL',
    label: 'View channel',
    description: 'Can see and open channels.',
    bit: 12,
  },
  {
    key: 'SEND_MESSAGES',
    label: 'Send messages',
    description: 'Can send messages in text channels.',
    bit: 0,
  },
  {
    key: 'MANAGE_CHANNELS',
    label: 'Manage channels',
    description:
      'Can create, edit, reorder, and delete channels and categories.',
    bit: 1,
  },
  {
    key: 'KICK_MEMBERS',
    label: 'Kick members',
    description: 'Can remove members from the guild.',
    bit: 2,
  },
  {
    key: 'BAN_MEMBERS',
    label: 'Ban members',
    description: 'Can ban members from the guild.',
    bit: 3,
  },
  {
    key: 'MANAGE_ROLES',
    label: 'Manage roles',
    description: 'Can create, edit, and delete roles.',
    bit: 4,
  },
  {
    key: 'MANAGE_GUILD',
    label: 'Manage guild',
    description: 'Can update guild profile and settings.',
    bit: 5,
  },
  {
    key: 'MANAGE_INVITES',
    label: 'Manage invites',
    description: 'Can create, list, and revoke guild invites.',
    bit: 6,
  },
  {
    key: 'MUTE_MEMBERS',
    label: 'Mute members',
    description: 'Can mute members in guild voice contexts.',
    bit: 7,
  },
  {
    key: 'VIEW_MOD_LOG',
    label: 'View mod log',
    description: 'Can view moderation logs.',
    bit: 8,
  },
  {
    key: 'ATTACH_FILES',
    label: 'Attach files',
    description: 'Can upload attachments in text channels.',
    bit: 9,
  },
  {
    key: 'ADD_REACTIONS',
    label: 'Add reactions',
    description: 'Can react to messages with emoji.',
    bit: 10,
  },
  {
    key: 'MANAGE_MESSAGES',
    label: 'Manage messages',
    description: "Can edit or remove other users' messages.",
    bit: 11,
  },
]

export const GUILD_PERMISSION_CATALOG: GuildPermissionDefinition[] =
  permissionSpecs.map((permission) => ({
    ...permission,
    mask: 1 << permission.bit,
  }))

export const ALL_ROLE_PERMISSIONS_BITFLAG = GUILD_PERMISSION_CATALOG.reduce(
  (mask, permission) => mask | permission.mask,
  0,
)

export const DEFAULT_EVERYONE_PERMISSIONS_BITFLAG =
  (1 << 12) | (1 << 0) | (1 << 9) | (1 << 10)

export function hasGuildPermission(
  bitflag: number,
  permission: GuildPermissionDefinition,
): boolean {
  return (bitflag & permission.mask) === permission.mask
}

export function toggleGuildPermission(
  bitflag: number,
  permissionMask: number,
  enabled: boolean,
): number {
  return enabled ? bitflag | permissionMask : bitflag & ~permissionMask
}
