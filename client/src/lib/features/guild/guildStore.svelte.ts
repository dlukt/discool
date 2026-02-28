import {
  createGuild as createGuildApi,
  listGuilds as listGuildsApi,
  updateGuild as updateGuildApi,
  uploadGuildIcon as uploadGuildIconApi,
} from './guildApi'
import type { CreateGuildInput, Guild, UpdateGuildInput } from './types'

function upsertGuild(guild: Guild): void {
  const index = guildState.guilds.findIndex((item) => item.slug === guild.slug)
  if (index >= 0) {
    guildState.guilds[index] = guild
    guildState.guilds = [...guildState.guilds]
  } else {
    guildState.guilds = [...guildState.guilds, guild]
  }
}

export const guildState = $state({
  guilds: [] as Guild[],
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
      guildState.guilds = await listGuildsApi()
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

  bySlug: (guildSlug: string): Guild | null =>
    guildState.guilds.find((guild) => guild.slug === guildSlug) ?? null,

  clear: (): void => {
    guildState.guilds = []
    guildState.loading = false
    guildState.saving = false
    guildState.loaded = false
    guildState.error = null
  },
})
