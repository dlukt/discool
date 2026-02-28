import {
  getGuildOrder,
  saveGuildOrder,
} from '$lib/features/identity/navigationState'
import {
  createGuild as createGuildApi,
  createRole as createRoleApi,
  deleteRole as deleteRoleApi,
  listGuilds as listGuildsApi,
  listRoles as listRolesApi,
  updateGuild as updateGuildApi,
  updateRole as updateRoleApi,
  uploadGuildIcon as uploadGuildIconApi,
} from './guildApi'
import type {
  CreateGuildInput,
  CreateGuildRoleInput,
  DeleteGuildRoleResult,
  Guild,
  GuildRole,
  UpdateGuildInput,
  UpdateGuildRoleInput,
} from './types'

function normalizeGuildOrder(order: string[]): string[] {
  return [...new Set(order.map((slug) => slug.trim()).filter(Boolean))]
}

function applyGuildOrder(guilds: Guild[], order = getGuildOrder()): Guild[] {
  const normalizedOrder = normalizeGuildOrder(order)
  if (normalizedOrder.length === 0) {
    return [...guilds]
  }

  const orderIndex = new Map(
    normalizedOrder.map((slug, index) => [slug, index] as const),
  )
  const sourceIndex = new Map(
    guilds.map((guild, index) => [guild.slug, index] as const),
  )

  return [...guilds].sort((left, right) => {
    const leftOrder = orderIndex.get(left.slug)
    const rightOrder = orderIndex.get(right.slug)
    if (leftOrder !== undefined && rightOrder !== undefined) {
      return leftOrder - rightOrder
    }
    if (leftOrder !== undefined) return -1
    if (rightOrder !== undefined) return 1
    return (
      (sourceIndex.get(left.slug) ?? 0) - (sourceIndex.get(right.slug) ?? 0)
    )
  })
}

function upsertGuild(guild: Guild): void {
  const index = guildState.guilds.findIndex((item) => item.slug === guild.slug)
  let nextGuilds: Guild[]
  if (index >= 0) {
    nextGuilds = [...guildState.guilds]
    nextGuilds[index] = guild
  } else {
    nextGuilds = [...guildState.guilds, guild]
  }
  guildState.guilds = applyGuildOrder(nextGuilds)
}

function setRolesForGuild(guildSlug: string, roles: GuildRole[]): GuildRole[] {
  guildState.rolesByGuild[guildSlug] = [...roles]
  return guildState.rolesByGuild[guildSlug]
}

function replaceRoleInGuild(guildSlug: string, role: GuildRole): GuildRole[] {
  const existing = guildState.rolesByGuild[guildSlug] ?? []
  const index = existing.findIndex((item) => item.id === role.id)
  let nextRoles: GuildRole[]
  if (index >= 0) {
    nextRoles = [...existing]
    nextRoles[index] = role
  } else {
    nextRoles = [...existing, role]
  }
  nextRoles.sort((left, right) => left.position - right.position)
  guildState.rolesByGuild[guildSlug] = nextRoles
  return nextRoles
}

export const guildState = $state({
  guilds: [] as Guild[],
  rolesByGuild: {} as Record<string, GuildRole[]>,
  loading: false,
  saving: false,
  loaded: false,
  error: null as string | null,

  loadGuilds: async (force = false): Promise<Guild[]> => {
    if (guildState.loading) return guildState.guilds
    if (guildState.loaded && !force) return guildState.guilds

    guildState.loading = true
    guildState.error = null
    try {
      guildState.guilds = applyGuildOrder(await listGuildsApi())
      guildState.loaded = true
      return guildState.guilds
    } catch (err) {
      guildState.error =
        err instanceof Error ? err.message : 'Failed to load guilds'
      throw err
    } finally {
      guildState.loading = false
    }
  },

  createGuild: async (
    input: CreateGuildInput,
    iconFile: File | null = null,
  ): Promise<Guild> => {
    guildState.saving = true
    guildState.error = null
    try {
      let created = await createGuildApi(input)
      if (iconFile) {
        created = await uploadGuildIconApi(created.slug, iconFile)
      }
      upsertGuild(created)
      guildState.loaded = true
      return created
    } catch (err) {
      guildState.error =
        err instanceof Error ? err.message : 'Failed to create guild'
      throw err
    } finally {
      guildState.saving = false
    }
  },

  updateGuild: async (
    guildSlug: string,
    input: UpdateGuildInput,
    iconFile: File | null = null,
  ): Promise<Guild> => {
    guildState.saving = true
    guildState.error = null
    try {
      let updated = await updateGuildApi(guildSlug, input)
      if (iconFile) {
        updated = await uploadGuildIconApi(guildSlug, iconFile)
      }
      upsertGuild(updated)
      return updated
    } catch (err) {
      guildState.error =
        err instanceof Error ? err.message : 'Failed to update guild'
      throw err
    } finally {
      guildState.saving = false
    }
  },

  loadRoles: async (guildSlug: string, force = false): Promise<GuildRole[]> => {
    if (!guildSlug) return []
    if (guildState.rolesByGuild[guildSlug] && !force) {
      return guildState.rolesByGuild[guildSlug]
    }

    guildState.error = null
    try {
      return setRolesForGuild(guildSlug, await listRolesApi(guildSlug))
    } catch (err) {
      guildState.error =
        err instanceof Error ? err.message : 'Failed to load roles'
      throw err
    }
  },

  createRole: async (
    guildSlug: string,
    input: CreateGuildRoleInput,
  ): Promise<GuildRole> => {
    guildState.saving = true
    guildState.error = null
    try {
      const created = await createRoleApi(guildSlug, input)
      replaceRoleInGuild(guildSlug, created)
      await guildState.loadRoles(guildSlug, true)
      return created
    } catch (err) {
      guildState.error =
        err instanceof Error ? err.message : 'Failed to create role'
      throw err
    } finally {
      guildState.saving = false
    }
  },

  updateRole: async (
    guildSlug: string,
    roleId: string,
    input: UpdateGuildRoleInput,
  ): Promise<GuildRole> => {
    guildState.saving = true
    guildState.error = null
    try {
      const updated = await updateRoleApi(guildSlug, roleId, input)
      replaceRoleInGuild(guildSlug, updated)
      await guildState.loadRoles(guildSlug, true)
      return updated
    } catch (err) {
      guildState.error =
        err instanceof Error ? err.message : 'Failed to update role'
      throw err
    } finally {
      guildState.saving = false
    }
  },

  deleteRole: async (
    guildSlug: string,
    roleId: string,
  ): Promise<DeleteGuildRoleResult> => {
    guildState.saving = true
    guildState.error = null
    try {
      const deleted = await deleteRoleApi(guildSlug, roleId)
      const existing = guildState.rolesByGuild[guildSlug] ?? []
      guildState.rolesByGuild[guildSlug] = existing.filter(
        (role) => role.id !== deleted.deletedId,
      )
      await guildState.loadRoles(guildSlug, true)
      return deleted
    } catch (err) {
      guildState.error =
        err instanceof Error ? err.message : 'Failed to delete role'
      throw err
    } finally {
      guildState.saving = false
    }
  },

  rolesForGuild: (guildSlug: string): GuildRole[] => [
    ...(guildState.rolesByGuild[guildSlug] ?? []),
  ],

  bySlug: (guildSlug: string): Guild | null =>
    guildState.guilds.find((guild) => guild.slug === guildSlug) ?? null,

  setGuildOrder: (order: string[]): void => {
    const normalizedOrder = normalizeGuildOrder(order)
    saveGuildOrder(normalizedOrder)
    guildState.guilds = applyGuildOrder(guildState.guilds, normalizedOrder)
  },

  clear: (): void => {
    guildState.guilds = []
    guildState.loading = false
    guildState.saving = false
    guildState.loaded = false
    guildState.rolesByGuild = {}
    guildState.error = null
  },
})
