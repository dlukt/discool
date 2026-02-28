export type ChannelType = 'text' | 'voice'

export type Channel = {
  id: string
  slug: string
  name: string
  channelType: ChannelType
  position: number
  isDefault: boolean
  categorySlug: string | null
  createdAt: string
}

export type ChannelCategory = {
  id: string
  slug: string
  name: string
  position: number
  collapsed: boolean
  createdAt: string
}

export type CreateChannelInput = {
  name: string
  channelType: ChannelType
  categorySlug?: string | null
}

export type UpdateChannelInput = {
  name: string
}

export type ReorderChannelPositionInput = {
  channelSlug: string
  categorySlug: string | null
  position: number
}

export type ReorderChannelsInput = {
  channelSlugs?: string[]
  channelPositions?: ReorderChannelPositionInput[]
}

export type DeleteChannelResult = {
  deletedSlug: string
  fallbackChannelSlug: string
}

export type CreateCategoryInput = {
  name: string
}

export type UpdateCategoryInput = {
  name: string
}

export type ReorderCategoriesInput = {
  categorySlugs: string[]
}

export type DeleteCategoryResult = {
  deletedSlug: string
  reassignedChannelCount: number
}

export type ChannelWire = {
  id: string
  slug: string
  name: string
  channel_type: ChannelType
  position: number
  is_default: boolean
  category_slug?: string | null
  created_at: string
}

export type ChannelCategoryWire = {
  id: string
  slug: string
  name: string
  position: number
  collapsed: boolean
  created_at: string
}

export type CreateChannelInputWire = {
  name: string
  channel_type: ChannelType
  category_slug?: string | null
}

export type UpdateChannelInputWire = {
  name: string
}

export type ReorderChannelPositionInputWire = {
  channel_slug: string
  category_slug: string | null
  position: number
}

export type ReorderChannelsInputWire = {
  channel_slugs?: string[]
  channel_positions?: ReorderChannelPositionInputWire[]
}

export type DeleteChannelResultWire = {
  deleted_slug: string
  fallback_channel_slug: string
}

export type CreateCategoryInputWire = {
  name: string
}

export type UpdateCategoryInputWire = {
  name: string
}

export type ReorderCategoriesInputWire = {
  category_slugs: string[]
}

export type UpdateCategoryCollapseInputWire = {
  collapsed: boolean
}

export type DeleteCategoryResultWire = {
  deleted_slug: string
  reassigned_channel_count: number
}

export function toChannel(wire: ChannelWire): Channel {
  return {
    id: wire.id,
    slug: wire.slug,
    name: wire.name,
    channelType: wire.channel_type,
    position: wire.position,
    isDefault: wire.is_default,
    categorySlug: wire.category_slug ?? null,
    createdAt: wire.created_at,
  }
}

export function toChannelCategory(wire: ChannelCategoryWire): ChannelCategory {
  return {
    id: wire.id,
    slug: wire.slug,
    name: wire.name,
    position: wire.position,
    collapsed: wire.collapsed,
    createdAt: wire.created_at,
  }
}

export function toCreateChannelInputWire(
  input: CreateChannelInput,
): CreateChannelInputWire {
  const wire: CreateChannelInputWire = {
    name: input.name,
    channel_type: input.channelType,
  }
  if (input.categorySlug !== undefined) {
    wire.category_slug = input.categorySlug
  }
  return wire
}

export function toUpdateChannelInputWire(
  input: UpdateChannelInput,
): UpdateChannelInputWire {
  return {
    name: input.name,
  }
}

export function toReorderChannelsInputWire(
  input: ReorderChannelsInput,
): ReorderChannelsInputWire {
  const wire: ReorderChannelsInputWire = {}
  if (input.channelSlugs !== undefined) {
    wire.channel_slugs = input.channelSlugs
  }
  if (input.channelPositions !== undefined) {
    wire.channel_positions = input.channelPositions.map((item) => ({
      channel_slug: item.channelSlug,
      category_slug: item.categorySlug,
      position: item.position,
    }))
  }
  return wire
}

export function toDeleteChannelResult(
  wire: DeleteChannelResultWire,
): DeleteChannelResult {
  return {
    deletedSlug: wire.deleted_slug,
    fallbackChannelSlug: wire.fallback_channel_slug,
  }
}

export function toCreateCategoryInputWire(
  input: CreateCategoryInput,
): CreateCategoryInputWire {
  return {
    name: input.name,
  }
}

export function toUpdateCategoryInputWire(
  input: UpdateCategoryInput,
): UpdateCategoryInputWire {
  return {
    name: input.name,
  }
}

export function toReorderCategoriesInputWire(
  input: ReorderCategoriesInput,
): ReorderCategoriesInputWire {
  return {
    category_slugs: input.categorySlugs,
  }
}

export function toUpdateCategoryCollapseInputWire(
  collapsed: boolean,
): UpdateCategoryCollapseInputWire {
  return {
    collapsed,
  }
}

export function toDeleteCategoryResult(
  wire: DeleteCategoryResultWire,
): DeleteCategoryResult {
  return {
    deletedSlug: wire.deleted_slug,
    reassignedChannelCount: wire.reassigned_channel_count,
  }
}
