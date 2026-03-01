import {
  createCategory as createCategoryApi,
  createChannel as createChannelApi,
  deleteCategory as deleteCategoryApi,
  deleteChannel as deleteChannelApi,
  deleteChannelPermissionOverride as deleteChannelPermissionOverrideApi,
  listCategories as listCategoriesApi,
  listChannelPermissionOverrides as listChannelPermissionOverridesApi,
  listChannels as listChannelsApi,
  reorderCategories as reorderCategoriesApi,
  reorderChannels as reorderChannelsApi,
  setCategoryCollapsed as setCategoryCollapsedApi,
  updateCategory as updateCategoryApi,
  updateChannel as updateChannelApi,
  upsertChannelPermissionOverride as upsertChannelPermissionOverrideApi,
} from './channelApi'
import type {
  Channel,
  ChannelCategory,
  ChannelPermissionOverride,
  ChannelPermissionOverrides,
  CreateCategoryInput,
  CreateChannelInput,
  DeleteCategoryResult,
  DeleteChannelPermissionOverrideResult,
  DeleteChannelResult,
  ReorderChannelPositionInput,
  UpdateCategoryInput,
  UpdateChannelInput,
  UpsertChannelPermissionOverrideInput,
} from './types'

let latestLoadRequestToken = 0

function unreadChannelKey(guildSlug: string, channelSlug: string): string {
  return `${guildSlug}:${channelSlug}`
}

function withUnreadFlag(
  guildSlug: string,
  channel: Channel,
  unreadByChannelKey: Record<string, boolean>,
): Channel {
  return {
    ...channel,
    hasUnreadActivity:
      unreadByChannelKey[unreadChannelKey(guildSlug, channel.slug)] === true,
  }
}

function applyUnreadFlags(
  guildSlug: string,
  channels: Channel[],
  unreadByChannelKey: Record<string, boolean>,
): Channel[] {
  return channels.map((channel) =>
    withUnreadFlag(guildSlug, channel, unreadByChannelKey),
  )
}

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

function channelPermissionOverrideCacheKey(
  guildSlug: string,
  channelSlug: string,
): string {
  return `${guildSlug}:${channelSlug}`
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
  unreadByChannelKey: {} as Record<string, boolean>,
  permissionOverridesByChannel: {} as Record<
    string,
    ChannelPermissionOverrides
  >,
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
      channelState.channels = applyUnreadFlags(
        guildSlug,
        channels,
        channelState.unreadByChannelKey,
      )
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
      const nextChannel = withUnreadFlag(
        guildSlug,
        created,
        channelState.unreadByChannelKey,
      )
      channelState.activeGuild = guildSlug
      replaceChannel(nextChannel)
      channelState.loadedByGuild[guildSlug] = true
      cacheGuildData(guildSlug, channelState.channels, channelState.categories)
      return nextChannel
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
      const nextChannel = withUnreadFlag(
        guildSlug,
        updated,
        channelState.unreadByChannelKey,
      )
      channelState.activeGuild = guildSlug
      replaceChannel(nextChannel)
      channelState.loadedByGuild[guildSlug] = true
      cacheGuildData(guildSlug, channelState.channels, channelState.categories)
      return nextChannel
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
      delete channelState.permissionOverridesByChannel[
        channelPermissionOverrideCacheKey(guildSlug, channelSlug)
      ]
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
      channelState.channels = applyUnreadFlags(
        guildSlug,
        reordered,
        channelState.unreadByChannelKey,
      )
      channelState.activeGuild = guildSlug
      channelState.loadedByGuild[guildSlug] = true
      cacheGuildData(guildSlug, channelState.channels, channelState.categories)
      return channelState.channels
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
      channelState.channels = applyUnreadFlags(
        guildSlug,
        reordered,
        channelState.unreadByChannelKey,
      )
      channelState.activeGuild = guildSlug
      channelState.loadedByGuild[guildSlug] = true
      cacheGuildData(guildSlug, channelState.channels, channelState.categories)
      return channelState.channels
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
      channelState.channels = applyUnreadFlags(
        guildSlug,
        refreshed,
        channelState.unreadByChannelKey,
      )
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

  loadChannelPermissionOverrides: async (
    guildSlug: string,
    channelSlug: string,
    force = false,
  ): Promise<ChannelPermissionOverrides> => {
    if (!guildSlug || !channelSlug) {
      return { roles: [], overrides: [] }
    }
    const cacheKey = channelPermissionOverrideCacheKey(guildSlug, channelSlug)
    if (!force && channelState.permissionOverridesByChannel[cacheKey]) {
      return channelState.permissionOverridesByChannel[cacheKey]
    }

    channelState.error = null
    try {
      const loaded = await listChannelPermissionOverridesApi(
        guildSlug,
        channelSlug,
      )
      channelState.permissionOverridesByChannel[cacheKey] = {
        roles: [...loaded.roles].sort((a, b) => a.position - b.position),
        overrides: [...loaded.overrides],
      }
      return channelState.permissionOverridesByChannel[cacheKey]
    } catch (err) {
      channelState.error =
        err instanceof Error
          ? err.message
          : 'Failed to load channel permission overrides'
      throw err
    }
  },

  upsertChannelPermissionOverride: async (
    guildSlug: string,
    channelSlug: string,
    roleId: string,
    input: UpsertChannelPermissionOverrideInput,
  ): Promise<ChannelPermissionOverride> => {
    channelState.saving = true
    channelState.error = null
    try {
      const updated = await upsertChannelPermissionOverrideApi(
        guildSlug,
        channelSlug,
        roleId,
        input,
      )
      const cacheKey = channelPermissionOverrideCacheKey(guildSlug, channelSlug)
      const existing = channelState.permissionOverridesByChannel[cacheKey]
      if (existing) {
        const index = existing.overrides.findIndex(
          (item) => item.roleId === updated.roleId,
        )
        const nextOverrides = [...existing.overrides]
        if (index >= 0) {
          nextOverrides[index] = updated
        } else {
          nextOverrides.push(updated)
        }
        channelState.permissionOverridesByChannel[cacheKey] = {
          ...existing,
          overrides: nextOverrides,
        }
      }
      return updated
    } catch (err) {
      channelState.error =
        err instanceof Error
          ? err.message
          : 'Failed to save channel permission override'
      throw err
    } finally {
      channelState.saving = false
    }
  },

  deleteChannelPermissionOverride: async (
    guildSlug: string,
    channelSlug: string,
    roleId: string,
  ): Promise<DeleteChannelPermissionOverrideResult> => {
    channelState.saving = true
    channelState.error = null
    try {
      const deleted = await deleteChannelPermissionOverrideApi(
        guildSlug,
        channelSlug,
        roleId,
      )
      if (deleted.removed) {
        const cacheKey = channelPermissionOverrideCacheKey(
          guildSlug,
          channelSlug,
        )
        const existing = channelState.permissionOverridesByChannel[cacheKey]
        if (existing) {
          channelState.permissionOverridesByChannel[cacheKey] = {
            ...existing,
            overrides: existing.overrides.filter(
              (item) => item.roleId !== deleted.roleId,
            ),
          }
        }
      }
      return deleted
    } catch (err) {
      channelState.error =
        err instanceof Error
          ? err.message
          : 'Failed to delete channel permission override'
      throw err
    } finally {
      channelState.saving = false
    }
  },

  bySlug: (channelSlug: string): Channel | null =>
    channelState.channels.find((channel) => channel.slug === channelSlug) ??
    null,

  setChannelUnreadActivity: (
    guildSlug: string,
    channelSlug: string,
    hasUnreadActivity: boolean,
  ): void => {
    const normalizedGuild = guildSlug.trim()
    const normalizedChannel = channelSlug.trim()
    if (!normalizedGuild || !normalizedChannel) return
    const key = unreadChannelKey(normalizedGuild, normalizedChannel)
    const currentlyUnread = channelState.unreadByChannelKey[key] === true
    if (currentlyUnread === hasUnreadActivity) return

    if (hasUnreadActivity) {
      channelState.unreadByChannelKey = {
        ...channelState.unreadByChannelKey,
        [key]: true,
      }
    } else {
      const { [key]: _ignored, ...rest } = channelState.unreadByChannelKey
      channelState.unreadByChannelKey = rest
    }

    const apply = (channels: Channel[] | undefined): Channel[] | undefined => {
      if (!channels) return channels
      let changed = false
      const nextChannels = channels.map((channel) => {
        if (channel.slug !== normalizedChannel) return channel
        if (Boolean(channel.hasUnreadActivity) === hasUnreadActivity) {
          return channel
        }
        changed = true
        return { ...channel, hasUnreadActivity }
      })
      return changed ? nextChannels : channels
    }

    if (channelState.activeGuild === normalizedGuild) {
      const nextActive = apply(channelState.channels)
      if (nextActive && nextActive !== channelState.channels) {
        channelState.channels = nextActive
      }
    }

    const cached = channelState.cachedChannelsByGuild[normalizedGuild]
    const nextCached = apply(cached)
    if (nextCached && nextCached !== cached) {
      channelState.cachedChannelsByGuild[normalizedGuild] = nextCached
    }
  },

  hasGuildUnreadActivity: (guildSlug: string): boolean => {
    const normalizedGuild = guildSlug.trim()
    if (!normalizedGuild) return false
    const prefix = `${normalizedGuild}:`
    return Object.keys(channelState.unreadByChannelKey).some((key) =>
      key.startsWith(prefix),
    )
  },

  orderedChannelsForGuild: (guildSlug: string): Channel[] => {
    const normalizedGuild = guildSlug.trim()
    if (!normalizedGuild) return []

    const sourceChannels =
      channelState.activeGuild === normalizedGuild
        ? channelState.channels
        : (channelState.cachedChannelsByGuild[normalizedGuild] ?? [])
    const sourceCategories =
      channelState.activeGuild === normalizedGuild
        ? channelState.categories
        : (channelState.cachedCategoriesByGuild[normalizedGuild] ?? [])

    const orderedCategories = [...sourceCategories].sort(
      (left, right) => left.position - right.position,
    )
    const ordered: Channel[] = []
    for (const category of orderedCategories) {
      ordered.push(
        ...sourceChannels
          .filter((channel) => channel.categorySlug === category.slug)
          .sort((left, right) => left.position - right.position),
      )
    }
    ordered.push(
      ...sourceChannels
        .filter((channel) => channel.categorySlug === null)
        .sort((left, right) => left.position - right.position),
    )
    return ordered
  },

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
    channelState.unreadByChannelKey = {}
    channelState.permissionOverridesByChannel = {}
    channelState.error = null
  },
})
