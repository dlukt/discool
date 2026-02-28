import {
  createCategory as createCategoryApi,
  createChannel as createChannelApi,
  deleteCategory as deleteCategoryApi,
  deleteChannel as deleteChannelApi,
  listCategories as listCategoriesApi,
  listChannels as listChannelsApi,
  reorderCategories as reorderCategoriesApi,
  reorderChannels as reorderChannelsApi,
  setCategoryCollapsed as setCategoryCollapsedApi,
  updateCategory as updateCategoryApi,
  updateChannel as updateChannelApi,
} from './channelApi'
import type {
  Channel,
  ChannelCategory,
  CreateCategoryInput,
  CreateChannelInput,
  DeleteCategoryResult,
  DeleteChannelResult,
  ReorderChannelPositionInput,
  UpdateCategoryInput,
  UpdateChannelInput,
} from './types'

let latestLoadRequestToken = 0

function replaceChannel(updated: Channel): void {
  const index = channelState.channels.findIndex(
    (item) => item.id === updated.id,
  )
  if (index >= 0) {
    channelState.channels[index] = updated
    channelState.channels = [...channelState.channels]
    return
  }
  channelState.channels = [...channelState.channels, updated]
}

function replaceCategory(updated: ChannelCategory): void {
  const index = channelState.categories.findIndex(
    (item) => item.id === updated.id,
  )
  if (index >= 0) {
    channelState.categories[index] = updated
    channelState.categories = [...channelState.categories]
    return
  }
  channelState.categories = [...channelState.categories, updated].sort(
    (a, b) => a.position - b.position,
  )
}

function cacheGuildData(
  guildSlug: string,
  channels: Channel[],
  categories: ChannelCategory[],
): void {
  channelState.cachedChannelsByGuild[guildSlug] = [...channels]
  channelState.cachedCategoriesByGuild[guildSlug] = [...categories]
}

function activateGuildFromCache(guildSlug: string): Channel[] {
  channelState.channels = [
    ...(channelState.cachedChannelsByGuild[guildSlug] ?? []),
  ]
  channelState.categories = [
    ...(channelState.cachedCategoriesByGuild[guildSlug] ?? []),
  ]
  channelState.activeGuild = guildSlug
  channelState.loadedByGuild[guildSlug] = true
  return channelState.channels
}

export const channelState = $state({
  activeGuild: null as string | null,
  channels: [] as Channel[],
  categories: [] as ChannelCategory[],
  loading: false,
  saving: false,
  loadedByGuild: {} as Record<string, boolean>,
  cachedChannelsByGuild: {} as Record<string, Channel[]>,
  cachedCategoriesByGuild: {} as Record<string, ChannelCategory[]>,
  error: null as string | null,

  loadChannels: async (
    guildSlug: string,
    force = false,
  ): Promise<Channel[]> => {
    if (!guildSlug) {
      channelState.channels = []
      channelState.categories = []
      channelState.activeGuild = null
      return channelState.channels
    }
    if (
      !force &&
      channelState.cachedChannelsByGuild[guildSlug] &&
      channelState.cachedCategoriesByGuild[guildSlug]
    ) {
      if (channelState.loading) {
        latestLoadRequestToken += 1
      }
      channelState.loading = false
      channelState.error = null
      return activateGuildFromCache(guildSlug)
    }
    if (
      channelState.loading &&
      channelState.activeGuild === guildSlug &&
      !force
    ) {
      return channelState.channels
    }
    if (
      channelState.loadedByGuild[guildSlug] &&
      channelState.activeGuild === guildSlug &&
      !force
    ) {
      return channelState.channels
    }

    const requestToken = latestLoadRequestToken + 1
    latestLoadRequestToken = requestToken
    channelState.loading = true
    channelState.error = null
    try {
      const [channels, categories] = await Promise.all([
        listChannelsApi(guildSlug),
        listCategoriesApi(guildSlug),
      ])
      if (requestToken !== latestLoadRequestToken) {
        return channelState.channels
      }
      channelState.channels = channels
      channelState.categories = categories
      channelState.activeGuild = guildSlug
      channelState.loadedByGuild[guildSlug] = true
      cacheGuildData(guildSlug, channelState.channels, channelState.categories)
      return channelState.channels
    } catch (err) {
      if (requestToken !== latestLoadRequestToken) {
        return channelState.channels
      }
      channelState.error =
        err instanceof Error ? err.message : 'Failed to load channels'
      throw err
    } finally {
      if (requestToken === latestLoadRequestToken) {
        channelState.loading = false
      }
    }
  },

  createChannel: async (
    guildSlug: string,
    input: CreateChannelInput,
  ): Promise<Channel> => {
    channelState.saving = true
    channelState.error = null
    try {
      const created = await createChannelApi(guildSlug, input)
      channelState.activeGuild = guildSlug
      replaceChannel(created)
      channelState.loadedByGuild[guildSlug] = true
      cacheGuildData(guildSlug, channelState.channels, channelState.categories)
      return created
    } catch (err) {
      channelState.error =
        err instanceof Error ? err.message : 'Failed to create channel'
      throw err
    } finally {
      channelState.saving = false
    }
  },

  updateChannel: async (
    guildSlug: string,
    channelSlug: string,
    input: UpdateChannelInput,
  ): Promise<Channel> => {
    channelState.saving = true
    channelState.error = null
    try {
      const updated = await updateChannelApi(guildSlug, channelSlug, input)
      channelState.activeGuild = guildSlug
      replaceChannel(updated)
      channelState.loadedByGuild[guildSlug] = true
      cacheGuildData(guildSlug, channelState.channels, channelState.categories)
      return updated
    } catch (err) {
      channelState.error =
        err instanceof Error ? err.message : 'Failed to update channel'
      throw err
    } finally {
      channelState.saving = false
    }
  },

  deleteChannel: async (
    guildSlug: string,
    channelSlug: string,
  ): Promise<DeleteChannelResult> => {
    channelState.saving = true
    channelState.error = null
    try {
      const deleted = await deleteChannelApi(guildSlug, channelSlug)
      channelState.channels = channelState.channels.filter(
        (channel) => channel.slug !== deleted.deletedSlug,
      )
      channelState.activeGuild = guildSlug
      channelState.loadedByGuild[guildSlug] = true
      cacheGuildData(guildSlug, channelState.channels, channelState.categories)
      return deleted
    } catch (err) {
      channelState.error =
        err instanceof Error ? err.message : 'Failed to delete channel'
      throw err
    } finally {
      channelState.saving = false
    }
  },

  reorderChannels: async (
    guildSlug: string,
    channelSlugs: string[],
  ): Promise<Channel[]> => {
    channelState.saving = true
    channelState.error = null
    try {
      const reordered = await reorderChannelsApi(guildSlug, { channelSlugs })
      channelState.channels = reordered
      channelState.activeGuild = guildSlug
      channelState.loadedByGuild[guildSlug] = true
      cacheGuildData(guildSlug, channelState.channels, channelState.categories)
      return reordered
    } catch (err) {
      channelState.error =
        err instanceof Error ? err.message : 'Failed to reorder channels'
      throw err
    } finally {
      channelState.saving = false
    }
  },

  reorderChannelPositions: async (
    guildSlug: string,
    channelPositions: ReorderChannelPositionInput[],
  ): Promise<Channel[]> => {
    channelState.saving = true
    channelState.error = null
    try {
      const reordered = await reorderChannelsApi(guildSlug, {
        channelPositions,
      })
      channelState.channels = reordered
      channelState.activeGuild = guildSlug
      channelState.loadedByGuild[guildSlug] = true
      cacheGuildData(guildSlug, channelState.channels, channelState.categories)
      return reordered
    } catch (err) {
      channelState.error =
        err instanceof Error
          ? err.message
          : 'Failed to move channels across categories'
      throw err
    } finally {
      channelState.saving = false
    }
  },

  moveChannel: async (
    guildSlug: string,
    channelSlug: string,
    categorySlug: string | null,
    position: number,
  ): Promise<Channel[]> => {
    const current = channelState.channels.find(
      (channel) => channel.slug === channelSlug,
    )
    if (!current) {
      return channelState.channels
    }

    const remaining = channelState.channels.filter(
      (channel) => channel.slug !== channelSlug,
    )

    const targetCategoryChannels = remaining
      .filter((channel) => channel.categorySlug === categorySlug)
      .sort((a, b) => a.position - b.position)
    const insertAt = Math.max(
      0,
      Math.min(position, targetCategoryChannels.length),
    )

    const movedChannel: Channel = {
      ...current,
      categorySlug,
      position: insertAt,
    }

    const nextCategoryChannels = [...targetCategoryChannels]
    nextCategoryChannels.splice(insertAt, 0, movedChannel)

    const bySlug = new Map(
      channelState.channels.map((channel) => [channel.slug, channel]),
    )
    const categoryOrder = [
      ...channelState.categories.map((category) => category.slug),
      null,
    ] as Array<string | null>

    const placements: ReorderChannelPositionInput[] = []
    for (const bucket of categoryOrder) {
      const bucketChannels =
        bucket === categorySlug
          ? nextCategoryChannels
          : remaining
              .filter((channel) => channel.categorySlug === bucket)
              .sort((a, b) => a.position - b.position)
      for (const [index, channel] of bucketChannels.entries()) {
        const source = bySlug.get(channel.slug)
        if (!source) continue
        placements.push({
          channelSlug: source.slug,
          categorySlug: bucket,
          position: index,
        })
      }
    }

    return channelState.reorderChannelPositions(guildSlug, placements)
  },

  createCategory: async (
    guildSlug: string,
    input: CreateCategoryInput,
  ): Promise<ChannelCategory> => {
    channelState.saving = true
    channelState.error = null
    try {
      const created = await createCategoryApi(guildSlug, input)
      channelState.activeGuild = guildSlug
      replaceCategory(created)
      channelState.loadedByGuild[guildSlug] = true
      cacheGuildData(guildSlug, channelState.channels, channelState.categories)
      return created
    } catch (err) {
      channelState.error =
        err instanceof Error ? err.message : 'Failed to create category'
      throw err
    } finally {
      channelState.saving = false
    }
  },

  updateCategory: async (
    guildSlug: string,
    categorySlug: string,
    input: UpdateCategoryInput,
  ): Promise<ChannelCategory> => {
    channelState.saving = true
    channelState.error = null
    try {
      const previous = channelState.categories.find(
        (item) => item.slug === categorySlug,
      )
      const updated = await updateCategoryApi(guildSlug, categorySlug, input)
      channelState.activeGuild = guildSlug
      replaceCategory(updated)
      channelState.channels = channelState.channels.map((channel) =>
        channel.categorySlug === categorySlug
          ? { ...channel, categorySlug: updated.slug }
          : channel,
      )
      if (previous && previous.slug !== updated.slug) {
        channelState.channels = channelState.channels.map((channel) =>
          channel.categorySlug === previous.slug
            ? { ...channel, categorySlug: updated.slug }
            : channel,
        )
      }
      channelState.loadedByGuild[guildSlug] = true
      cacheGuildData(guildSlug, channelState.channels, channelState.categories)
      return updated
    } catch (err) {
      channelState.error =
        err instanceof Error ? err.message : 'Failed to update category'
      throw err
    } finally {
      channelState.saving = false
    }
  },

  deleteCategory: async (
    guildSlug: string,
    categorySlug: string,
  ): Promise<DeleteCategoryResult> => {
    channelState.saving = true
    channelState.error = null
    try {
      const deleted = await deleteCategoryApi(guildSlug, categorySlug)
      channelState.categories = channelState.categories.filter(
        (category) => category.slug !== deleted.deletedSlug,
      )
      channelState.channels = channelState.channels.map((channel) =>
        channel.categorySlug === deleted.deletedSlug
          ? { ...channel, categorySlug: null }
          : channel,
      )
      const refreshed = await listChannelsApi(guildSlug)
      channelState.channels = refreshed
      channelState.activeGuild = guildSlug
      channelState.loadedByGuild[guildSlug] = true
      cacheGuildData(guildSlug, channelState.channels, channelState.categories)
      return deleted
    } catch (err) {
      channelState.error =
        err instanceof Error ? err.message : 'Failed to delete category'
      throw err
    } finally {
      channelState.saving = false
    }
  },

  reorderCategories: async (
    guildSlug: string,
    categorySlugs: string[],
  ): Promise<ChannelCategory[]> => {
    channelState.saving = true
    channelState.error = null
    try {
      const reordered = await reorderCategoriesApi(guildSlug, { categorySlugs })
      channelState.categories = reordered
      channelState.activeGuild = guildSlug
      channelState.loadedByGuild[guildSlug] = true
      cacheGuildData(guildSlug, channelState.channels, channelState.categories)
      return reordered
    } catch (err) {
      channelState.error =
        err instanceof Error ? err.message : 'Failed to reorder categories'
      throw err
    } finally {
      channelState.saving = false
    }
  },

  setCategoryCollapsed: async (
    guildSlug: string,
    categorySlug: string,
    collapsed: boolean,
  ): Promise<ChannelCategory> => {
    channelState.saving = true
    channelState.error = null
    try {
      const updated = await setCategoryCollapsedApi(
        guildSlug,
        categorySlug,
        collapsed,
      )
      replaceCategory(updated)
      channelState.activeGuild = guildSlug
      channelState.loadedByGuild[guildSlug] = true
      cacheGuildData(guildSlug, channelState.channels, channelState.categories)
      return updated
    } catch (err) {
      channelState.error =
        err instanceof Error ? err.message : 'Failed to persist category state'
      throw err
    } finally {
      channelState.saving = false
    }
  },

  bySlug: (channelSlug: string): Channel | null =>
    channelState.channels.find((channel) => channel.slug === channelSlug) ??
    null,

  clear: (): void => {
    latestLoadRequestToken = 0
    channelState.activeGuild = null
    channelState.channels = []
    channelState.categories = []
    channelState.loading = false
    channelState.saving = false
    channelState.loadedByGuild = {}
    channelState.cachedChannelsByGuild = {}
    channelState.cachedCategoriesByGuild = {}
    channelState.error = null
  },
})
