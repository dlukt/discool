export type Guild = {
  id: string
  slug: string
  name: string
  description?: string
  defaultChannelSlug: string
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
