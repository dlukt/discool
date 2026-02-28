export type Guild = {
  id: string
  slug: string
  name: string
  description?: string
  defaultChannelSlug: string
  lastViewedChannelSlug?: string
  hasUnreadActivity?: boolean
  isOwner: boolean
  iconUrl?: string
  createdAt: string
}

export type CreateGuildInput = {
  name: string
  description?: string
}

export type UpdateGuildInput = {
  name?: string
  description?: string | null
}

export type GuildRole = {
  id: string
  name: string
  color: string
  position: number
  permissionsBitflag: number
  isDefault: boolean
  isSystem: boolean
  canEdit: boolean
  canDelete: boolean
  createdAt: string
}

export type CreateGuildRoleInput = {
  name: string
  color: string
}

export type UpdateGuildRoleInput = {
  name?: string
  color?: string
  permissionsBitflag?: number
}

export type ReorderGuildRolesInput = {
  roleIds: string[]
}

export type DeleteGuildRoleResult = {
  deletedId: string
  removedAssignmentCount: number
}

export type GuildMember = {
  userId: string
  username: string
  displayName: string
  avatarColor?: string
  highestRoleColor: string
  roleIds: string[]
  isOwner: boolean
  canAssignRoles: boolean
}

export type GuildMemberRoleData = {
  members: GuildMember[]
  roles: GuildRole[]
  assignableRoleIds: string[]
  canManageRoles: boolean
}

export type UpdateGuildMemberRolesInput = {
  roleIds: string[]
}

export type InviteType = 'reusable' | 'single_use'

export type GuildInvite = {
  code: string
  type: InviteType
  usesRemaining: number
  createdBy: string
  creatorUsername: string
  createdAt: string
  revoked: boolean
  inviteUrl: string
}

export type CreateGuildInviteInput = {
  type: InviteType
}

export type RevokeGuildInviteResult = {
  code: string
  revoked: boolean
}

export type InviteWelcomeScreen = {
  enabled: boolean
  title?: string
  rules?: string
  acceptLabel?: string
}

export type InviteMetadata = {
  code: string
  guildSlug: string
  guildName: string
  guildIconUrl?: string
  defaultChannelSlug: string
  welcomeScreen: InviteWelcomeScreen
}

export type JoinGuildByInviteResult = {
  guildSlug: string
  guildName: string
  guildIconUrl?: string
  defaultChannelSlug: string
  alreadyMember: boolean
  welcomeScreen: InviteWelcomeScreen
}

export type GuildWire = {
  id: string
  slug: string
  name: string
  description?: string
  default_channel_slug: string
  last_viewed_channel_slug?: string
  has_unread_activity?: boolean
  is_owner: boolean
  icon_url?: string
  created_at: string
}

export type CreateGuildInputWire = {
  name: string
  description?: string
}

export type UpdateGuildInputWire = {
  name?: string
  description?: string | null
}

export type GuildRoleWire = {
  id: string
  name: string
  color: string
  position: number
  permissions_bitflag: number
  is_default: boolean
  is_system: boolean
  can_edit: boolean
  can_delete: boolean
  created_at: string
}

export type CreateGuildRoleInputWire = {
  name: string
  color: string
}

export type UpdateGuildRoleInputWire = {
  name?: string
  color?: string
  permissions_bitflag?: number
}

export type ReorderGuildRolesInputWire = {
  role_ids: string[]
}

export type DeleteGuildRoleResultWire = {
  deleted_id: string
  removed_assignment_count: number
}

export type GuildMemberWire = {
  user_id: string
  username: string
  display_name: string
  avatar_color?: string
  highest_role_color: string
  role_ids: string[]
  is_owner: boolean
  can_assign_roles: boolean
}

export type GuildMemberRoleDataWire = {
  members: GuildMemberWire[]
  roles: GuildRoleWire[]
  assignable_role_ids: string[]
  can_manage_roles: boolean
}

export type UpdateGuildMemberRolesInputWire = {
  role_ids: string[]
}

export type GuildInviteWire = {
  code: string
  type: InviteType
  uses_remaining: number
  created_by: string
  creator_username: string
  created_at: string
  revoked: boolean
  invite_url: string
}

export type CreateGuildInviteInputWire = {
  type: InviteType
}

export type RevokeGuildInviteResultWire = {
  code: string
  revoked: boolean
}

export type InviteWelcomeScreenWire = {
  enabled: boolean
  title?: string
  rules?: string
  accept_label?: string
}

export type InviteMetadataWire = {
  code: string
  guild_slug: string
  guild_name: string
  guild_icon_url?: string
  default_channel_slug: string
  welcome_screen: InviteWelcomeScreenWire
}

export type JoinGuildByInviteResultWire = {
  guild_slug: string
  guild_name: string
  guild_icon_url?: string
  default_channel_slug: string
  already_member: boolean
  welcome_screen: InviteWelcomeScreenWire
}

export function toGuild(wire: GuildWire): Guild {
  return {
    id: wire.id,
    slug: wire.slug,
    name: wire.name,
    description: wire.description,
    defaultChannelSlug: wire.default_channel_slug,
    lastViewedChannelSlug: wire.last_viewed_channel_slug,
    hasUnreadActivity: wire.has_unread_activity,
    isOwner: wire.is_owner,
    iconUrl: wire.icon_url,
    createdAt: wire.created_at,
  }
}

export function toCreateGuildInputWire(
  input: CreateGuildInput,
): CreateGuildInputWire {
  return {
    name: input.name,
    description: input.description,
  }
}

export function toUpdateGuildInputWire(
  input: UpdateGuildInput,
): UpdateGuildInputWire {
  const wire: UpdateGuildInputWire = {}
  if (input.name !== undefined) {
    wire.name = input.name
  }
  if (input.description !== undefined) {
    wire.description = input.description
  }
  return wire
}

export function toGuildRole(wire: GuildRoleWire): GuildRole {
  return {
    id: wire.id,
    name: wire.name,
    color: wire.color,
    position: wire.position,
    permissionsBitflag: wire.permissions_bitflag,
    isDefault: wire.is_default,
    isSystem: wire.is_system,
    canEdit: wire.can_edit,
    canDelete: wire.can_delete,
    createdAt: wire.created_at,
  }
}

export function toCreateGuildRoleInputWire(
  input: CreateGuildRoleInput,
): CreateGuildRoleInputWire {
  return {
    name: input.name,
    color: input.color,
  }
}

export function toUpdateGuildRoleInputWire(
  input: UpdateGuildRoleInput,
): UpdateGuildRoleInputWire {
  const wire: UpdateGuildRoleInputWire = {}
  if (input.name !== undefined) {
    wire.name = input.name
  }
  if (input.color !== undefined) {
    wire.color = input.color
  }
  if (input.permissionsBitflag !== undefined) {
    wire.permissions_bitflag = input.permissionsBitflag
  }
  return wire
}

export function toReorderGuildRolesInputWire(
  input: ReorderGuildRolesInput,
): ReorderGuildRolesInputWire {
  return {
    role_ids: input.roleIds,
  }
}

export function toDeleteGuildRoleResult(
  wire: DeleteGuildRoleResultWire,
): DeleteGuildRoleResult {
  return {
    deletedId: wire.deleted_id,
    removedAssignmentCount: wire.removed_assignment_count,
  }
}

export function toGuildMember(wire: GuildMemberWire): GuildMember {
  return {
    userId: wire.user_id,
    username: wire.username,
    displayName: wire.display_name,
    avatarColor: wire.avatar_color,
    highestRoleColor: wire.highest_role_color,
    roleIds: [...wire.role_ids],
    isOwner: wire.is_owner,
    canAssignRoles: wire.can_assign_roles,
  }
}

export function toGuildMemberRoleData(
  wire: GuildMemberRoleDataWire,
): GuildMemberRoleData {
  return {
    members: wire.members.map(toGuildMember),
    roles: wire.roles.map(toGuildRole),
    assignableRoleIds: [...wire.assignable_role_ids],
    canManageRoles: wire.can_manage_roles,
  }
}

export function toUpdateGuildMemberRolesInputWire(
  input: UpdateGuildMemberRolesInput,
): UpdateGuildMemberRolesInputWire {
  return {
    role_ids: input.roleIds,
  }
}

export function toGuildInvite(wire: GuildInviteWire): GuildInvite {
  return {
    code: wire.code,
    type: wire.type,
    usesRemaining: wire.uses_remaining,
    createdBy: wire.created_by,
    creatorUsername: wire.creator_username,
    createdAt: wire.created_at,
    revoked: wire.revoked,
    inviteUrl: wire.invite_url,
  }
}

export function toCreateGuildInviteInputWire(
  input: CreateGuildInviteInput,
): CreateGuildInviteInputWire {
  return {
    type: input.type,
  }
}

export function toRevokeGuildInviteResult(
  wire: RevokeGuildInviteResultWire,
): RevokeGuildInviteResult {
  return {
    code: wire.code,
    revoked: wire.revoked,
  }
}

function toInviteWelcomeScreen(
  wire: InviteWelcomeScreenWire,
): InviteWelcomeScreen {
  return {
    enabled: wire.enabled,
    title: wire.title,
    rules: wire.rules,
    acceptLabel: wire.accept_label,
  }
}

export function toInviteMetadata(wire: InviteMetadataWire): InviteMetadata {
  return {
    code: wire.code,
    guildSlug: wire.guild_slug,
    guildName: wire.guild_name,
    guildIconUrl: wire.guild_icon_url,
    defaultChannelSlug: wire.default_channel_slug,
    welcomeScreen: toInviteWelcomeScreen(wire.welcome_screen),
  }
}

export function toJoinGuildByInviteResult(
  wire: JoinGuildByInviteResultWire,
): JoinGuildByInviteResult {
  return {
    guildSlug: wire.guild_slug,
    guildName: wire.guild_name,
    guildIconUrl: wire.guild_icon_url,
    defaultChannelSlug: wire.default_channel_slug,
    alreadyMember: wire.already_member,
    welcomeScreen: toInviteWelcomeScreen(wire.welcome_screen),
  }
}
