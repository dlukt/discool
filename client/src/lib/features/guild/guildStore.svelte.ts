import {
  getGuildOrder,
  saveGuildOrder,
} from '$lib/features/identity/navigationState'
import {
  createGuild as createGuildApi,
  createRole as createRoleApi,
  deleteRole as deleteRoleApi,
  listGuilds as listGuildsApi,
  listMembers as listMembersApi,
  listRoles as listRolesApi,
  reorderRoles as reorderRolesApi,
  updateGuild as updateGuildApi,
  updateMemberRoles as updateMemberRolesApi,
  updateRole as updateRoleApi,
  uploadGuildIcon as uploadGuildIconApi,
} from './guildApi'
import type {
  CreateGuildInput,
  CreateGuildRoleInput,
  DeleteGuildRoleResult,
  Guild,
  GuildMember,
  GuildMemberRoleData,
  GuildRole,
  UpdateGuildInput,
  UpdateGuildMemberRolesInput,
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
    nextGuilds[index] = {
      ...guild,
      hasUnreadActivity:
        guild.hasUnreadActivity ?? nextGuilds[index]?.hasUnreadActivity,
    }
  } else {
    nextGuilds = [...guildState.guilds, guild]
  }
  guildState.guilds = applyGuildOrder(nextGuilds)
}

function setGuildUnreadActivity(
  guildSlug: string,
  hasUnreadActivity: boolean,
): void {
  const index = guildState.guilds.findIndex((guild) => guild.slug === guildSlug)
  if (index < 0) return
  const current = guildState.guilds[index]
  if (Boolean(current.hasUnreadActivity) === hasUnreadActivity) return
  const next = [...guildState.guilds]
  next[index] = { ...current, hasUnreadActivity }
  guildState.guilds = next
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

function setMemberRoleDataForGuild(
  guildSlug: string,
  data: GuildMemberRoleData,
): GuildMemberRoleData {
  const nextData: GuildMemberRoleData = {
    members: [...data.members],
    roles: [...data.roles],
    assignableRoleIds: [...data.assignableRoleIds],
    canManageRoles: data.canManageRoles,
  }
  guildState.memberRoleDataByGuild[guildSlug] = nextData
  guildState.rolesByGuild[guildSlug] = [...nextData.roles]
  return nextData
}

export const guildState = $state({
  guilds: [] as Guild[],
  rolesByGuild: {} as Record<string, GuildRole[]>,
  memberRoleDataByGuild: {} as Record<string, GuildMemberRoleData>,
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
      const unreadByGuildSlug = new Map(
        guildState.guilds.map((guild) => [guild.slug, guild.hasUnreadActivity]),
      )
      const loadedGuilds = await listGuildsApi()
      guildState.guilds = applyGuildOrder(
        loadedGuilds.map((guild) => ({
          ...guild,
          hasUnreadActivity:
            guild.hasUnreadActivity ?? unreadByGuildSlug.get(guild.slug),
        })),
      )
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

  loadMembers: async (
    guildSlug: string,
    force = false,
  ): Promise<GuildMemberRoleData> => {
    if (!guildSlug) {
      return {
        members: [],
        roles: [],
        assignableRoleIds: [],
        canManageRoles: false,
      }
    }
    if (guildState.memberRoleDataByGuild[guildSlug] && !force) {
      return guildState.memberRoleDataByGuild[guildSlug]
    }

    guildState.error = null
    try {
      return setMemberRoleDataForGuild(
        guildSlug,
        await listMembersApi(guildSlug),
      )
    } catch (err) {
      guildState.error =
        err instanceof Error ? err.message : 'Failed to load members'
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

  reorderRoles: async (
    guildSlug: string,
    roleIds: string[],
  ): Promise<GuildRole[]> => {
    guildState.saving = true
    guildState.error = null
    try {
      const reordered = await reorderRolesApi(guildSlug, { roleIds })
      setRolesForGuild(guildSlug, reordered)
      await guildState.loadRoles(guildSlug, true)
      return guildState.rolesByGuild[guildSlug] ?? reordered
    } catch (err) {
      guildState.error =
        err instanceof Error ? err.message : 'Failed to reorder roles'
      throw err
    } finally {
      guildState.saving = false
    }
  },

  rolesForGuild: (guildSlug: string): GuildRole[] => [
    ...(guildState.rolesByGuild[guildSlug] ?? []),
  ],

  updateMemberRoles: async (
    guildSlug: string,
    memberUserId: string,
    input: UpdateGuildMemberRolesInput,
  ): Promise<GuildMember> => {
    guildState.saving = true
    guildState.error = null
    try {
      const updated = await updateMemberRolesApi(guildSlug, memberUserId, input)
      const existingData = guildState.memberRoleDataByGuild[guildSlug]
      if (!existingData) {
        await guildState.loadMembers(guildSlug, true)
        return updated
      }
      guildState.memberRoleDataByGuild[guildSlug] = {
        ...existingData,
        members: existingData.members.map((member) =>
          member.userId === updated.userId ? updated : member,
        ),
      }
      return updated
    } catch (err) {
      guildState.error =
        err instanceof Error ? err.message : 'Failed to update member roles'
      throw err
    } finally {
      guildState.saving = false
    }
  },

  memberRoleDataForGuild: (guildSlug: string): GuildMemberRoleData => {
    const existing = guildState.memberRoleDataByGuild[guildSlug]
    if (existing) {
      return {
        members: [...existing.members],
        roles: [...existing.roles],
        assignableRoleIds: [...existing.assignableRoleIds],
        canManageRoles: existing.canManageRoles,
      }
    }
    return {
      members: [],
      roles: [],
      assignableRoleIds: [],
      canManageRoles: false,
    }
  },

  memberByUserId: (guildSlug: string, userId: string): GuildMember | null =>
    guildState.memberRoleDataByGuild[guildSlug]?.members.find(
      (member) => member.userId === userId,
    ) ?? null,

  bySlug: (guildSlug: string): Guild | null =>
    guildState.guilds.find((guild) => guild.slug === guildSlug) ?? null,

  setGuildUnreadActivity: (
    guildSlug: string,
    hasUnreadActivity: boolean,
  ): void => {
    const normalizedGuild = guildSlug.trim()
    if (!normalizedGuild) return
    setGuildUnreadActivity(normalizedGuild, hasUnreadActivity)
  },

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
    guildState.memberRoleDataByGuild = {}
    guildState.error = null
  },
})
