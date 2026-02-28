import { apiFetch } from '$lib/api'

import {
  type Channel,
  type ChannelCategory,
  type ChannelCategoryWire,
  type ChannelPermissionOverride,
  type ChannelPermissionOverrides,
  type ChannelPermissionOverridesWire,
  type ChannelPermissionOverrideWire,
  type ChannelWire,
  type CreateCategoryInput,
  type CreateChannelInput,
  type DeleteCategoryResult,
  type DeleteCategoryResultWire,
  type DeleteChannelPermissionOverrideResult,
  type DeleteChannelPermissionOverrideResultWire,
  type DeleteChannelResult,
  type DeleteChannelResultWire,
  type ReorderCategoriesInput,
  type ReorderChannelsInput,
  toChannel,
  toChannelCategory,
  toChannelPermissionOverride,
  toChannelPermissionOverrides,
  toCreateCategoryInputWire,
  toCreateChannelInputWire,
  toDeleteCategoryResult,
  toDeleteChannelPermissionOverrideResult,
  toDeleteChannelResult,
  toReorderCategoriesInputWire,
  toReorderChannelsInputWire,
  toUpdateCategoryCollapseInputWire,
  toUpdateCategoryInputWire,
  toUpdateChannelInputWire,
  toUpsertChannelPermissionOverrideInputWire,
  type UpdateCategoryInput,
  type UpdateChannelInput,
  type UpsertChannelPermissionOverrideInput,
} from './types'

export function listChannels(guildSlug: string): Promise<Channel[]> {
  return apiFetch<ChannelWire[]>(
    `/api/v1/guilds/${encodeURIComponent(guildSlug)}/channels`,
  ).then((channels) => channels.map(toChannel))
}

export function createChannel(
  guildSlug: string,
  input: CreateChannelInput,
): Promise<Channel> {
  return apiFetch<ChannelWire>(
    `/api/v1/guilds/${encodeURIComponent(guildSlug)}/channels`,
    {
      method: 'POST',
      body: JSON.stringify(toCreateChannelInputWire(input)),
    },
  ).then(toChannel)
}

export function updateChannel(
  guildSlug: string,
  channelSlug: string,
  input: UpdateChannelInput,
): Promise<Channel> {
  return apiFetch<ChannelWire>(
    `/api/v1/guilds/${encodeURIComponent(guildSlug)}/channels/${encodeURIComponent(channelSlug)}`,
    {
      method: 'PATCH',
      body: JSON.stringify(toUpdateChannelInputWire(input)),
    },
  ).then(toChannel)
}

export function deleteChannel(
  guildSlug: string,
  channelSlug: string,
): Promise<DeleteChannelResult> {
  return apiFetch<DeleteChannelResultWire>(
    `/api/v1/guilds/${encodeURIComponent(guildSlug)}/channels/${encodeURIComponent(channelSlug)}`,
    { method: 'DELETE' },
  ).then(toDeleteChannelResult)
}

export function listChannelPermissionOverrides(
  guildSlug: string,
  channelSlug: string,
): Promise<ChannelPermissionOverrides> {
  return apiFetch<ChannelPermissionOverridesWire>(
    `/api/v1/guilds/${encodeURIComponent(guildSlug)}/channels/${encodeURIComponent(channelSlug)}/permission-overrides`,
  ).then(toChannelPermissionOverrides)
}

export function upsertChannelPermissionOverride(
  guildSlug: string,
  channelSlug: string,
  roleId: string,
  input: UpsertChannelPermissionOverrideInput,
): Promise<ChannelPermissionOverride> {
  return apiFetch<ChannelPermissionOverrideWire>(
    `/api/v1/guilds/${encodeURIComponent(guildSlug)}/channels/${encodeURIComponent(channelSlug)}/permission-overrides/${encodeURIComponent(roleId)}`,
    {
      method: 'PUT',
      body: JSON.stringify(toUpsertChannelPermissionOverrideInputWire(input)),
    },
  ).then(toChannelPermissionOverride)
}

export function deleteChannelPermissionOverride(
  guildSlug: string,
  channelSlug: string,
  roleId: string,
): Promise<DeleteChannelPermissionOverrideResult> {
  return apiFetch<DeleteChannelPermissionOverrideResultWire>(
    `/api/v1/guilds/${encodeURIComponent(guildSlug)}/channels/${encodeURIComponent(channelSlug)}/permission-overrides/${encodeURIComponent(roleId)}`,
    { method: 'DELETE' },
  ).then(toDeleteChannelPermissionOverrideResult)
}

export function reorderChannels(
  guildSlug: string,
  input: ReorderChannelsInput,
): Promise<Channel[]> {
  return apiFetch<ChannelWire[]>(
    `/api/v1/guilds/${encodeURIComponent(guildSlug)}/channels/reorder`,
    {
      method: 'PATCH',
      body: JSON.stringify(toReorderChannelsInputWire(input)),
    },
  ).then((channels) => channels.map(toChannel))
}

export function listCategories(guildSlug: string): Promise<ChannelCategory[]> {
  return apiFetch<ChannelCategoryWire[]>(
    `/api/v1/guilds/${encodeURIComponent(guildSlug)}/categories`,
  ).then((categories) => categories.map(toChannelCategory))
}

export function createCategory(
  guildSlug: string,
  input: CreateCategoryInput,
): Promise<ChannelCategory> {
  return apiFetch<ChannelCategoryWire>(
    `/api/v1/guilds/${encodeURIComponent(guildSlug)}/categories`,
    {
      method: 'POST',
      body: JSON.stringify(toCreateCategoryInputWire(input)),
    },
  ).then(toChannelCategory)
}

export function updateCategory(
  guildSlug: string,
  categorySlug: string,
  input: UpdateCategoryInput,
): Promise<ChannelCategory> {
  return apiFetch<ChannelCategoryWire>(
    `/api/v1/guilds/${encodeURIComponent(guildSlug)}/categories/${encodeURIComponent(categorySlug)}`,
    {
      method: 'PATCH',
      body: JSON.stringify(toUpdateCategoryInputWire(input)),
    },
  ).then(toChannelCategory)
}

export function deleteCategory(
  guildSlug: string,
  categorySlug: string,
): Promise<DeleteCategoryResult> {
  return apiFetch<DeleteCategoryResultWire>(
    `/api/v1/guilds/${encodeURIComponent(guildSlug)}/categories/${encodeURIComponent(categorySlug)}`,
    { method: 'DELETE' },
  ).then(toDeleteCategoryResult)
}

export function reorderCategories(
  guildSlug: string,
  input: ReorderCategoriesInput,
): Promise<ChannelCategory[]> {
  return apiFetch<ChannelCategoryWire[]>(
    `/api/v1/guilds/${encodeURIComponent(guildSlug)}/categories/reorder`,
    {
      method: 'PATCH',
      body: JSON.stringify(toReorderCategoriesInputWire(input)),
    },
  ).then((categories) => categories.map(toChannelCategory))
}

export function setCategoryCollapsed(
  guildSlug: string,
  categorySlug: string,
  collapsed: boolean,
): Promise<ChannelCategory> {
  return apiFetch<ChannelCategoryWire>(
    `/api/v1/guilds/${encodeURIComponent(guildSlug)}/categories/${encodeURIComponent(categorySlug)}/collapse`,
    {
      method: 'PATCH',
      body: JSON.stringify(toUpdateCategoryCollapseInputWire(collapsed)),
    },
  ).then(toChannelCategory)
}
