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
