import { apiFetch } from '$lib/api'

import {
  type Channel,
  type ChannelCategory,
  type ChannelCategoryWire,
  type ChannelWire,
  type CreateCategoryInput,
  type CreateChannelInput,
  type DeleteCategoryResult,
  type DeleteCategoryResultWire,
  type DeleteChannelResult,
  type DeleteChannelResultWire,
  type ReorderCategoriesInput,
  type ReorderChannelsInput,
  toChannel,
  toChannelCategory,
  toCreateCategoryInputWire,
  toCreateChannelInputWire,
  toDeleteCategoryResult,
  toDeleteChannelResult,
  toReorderCategoriesInputWire,
  toReorderChannelsInputWire,
  toUpdateCategoryCollapseInputWire,
  toUpdateCategoryInputWire,
  toUpdateChannelInputWire,
  type UpdateCategoryInput,
  type UpdateChannelInput,
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
