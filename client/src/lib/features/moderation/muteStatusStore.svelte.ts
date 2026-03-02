import { fetchMyMuteStatus, type MuteStatus } from './moderationApi'

const INACTIVE_STATUS: MuteStatus = {
  active: false,
  isPermanent: false,
  expiresAt: null,
  reason: null,
}

function normalizeGuildSlug(guildSlug: string): string | null {
  const normalized = guildSlug.trim()
  return normalized.length > 0 ? normalized : null
}

function sameStatus(left: MuteStatus | undefined, right: MuteStatus): boolean {
  if (!left) return false
  return (
    left.active === right.active &&
    left.isPermanent === right.isPermanent &&
    left.expiresAt === right.expiresAt &&
    left.reason === right.reason
  )
}

export const muteStatusState = $state({
  version: 0,
  statusByGuild: {} as Record<string, MuteStatus>,
  loadingByGuild: {} as Record<string, boolean>,

  statusForGuild: (guildSlug: string): MuteStatus => {
    const normalized = normalizeGuildSlug(guildSlug)
    if (!normalized) return INACTIVE_STATUS
    return muteStatusState.statusByGuild[normalized] ?? INACTIVE_STATUS
  },

  isLoadingForGuild: (guildSlug: string): boolean => {
    const normalized = normalizeGuildSlug(guildSlug)
    if (!normalized) return false
    return muteStatusState.loadingByGuild[normalized] === true
  },

  refresh: async (guildSlug: string): Promise<MuteStatus> => {
    const normalized = normalizeGuildSlug(guildSlug)
    if (!normalized) return INACTIVE_STATUS

    if (muteStatusState.loadingByGuild[normalized] !== true) {
      muteStatusState.loadingByGuild = {
        ...muteStatusState.loadingByGuild,
        [normalized]: true,
      }
      muteStatusState.version += 1
    }

    try {
      const status = await fetchMyMuteStatus(normalized)
      if (!sameStatus(muteStatusState.statusByGuild[normalized], status)) {
        muteStatusState.statusByGuild = {
          ...muteStatusState.statusByGuild,
          [normalized]: status,
        }
        muteStatusState.version += 1
      }
      return status
    } finally {
      if (muteStatusState.loadingByGuild[normalized]) {
        const nextLoading = { ...muteStatusState.loadingByGuild }
        delete nextLoading[normalized]
        muteStatusState.loadingByGuild = nextLoading
        muteStatusState.version += 1
      }
    }
  },

  clearGuild: (guildSlug: string): void => {
    const normalized = normalizeGuildSlug(guildSlug)
    if (!normalized) return

    let changed = false
    if (normalized in muteStatusState.statusByGuild) {
      const next = { ...muteStatusState.statusByGuild }
      delete next[normalized]
      muteStatusState.statusByGuild = next
      changed = true
    }
    if (normalized in muteStatusState.loadingByGuild) {
      const next = { ...muteStatusState.loadingByGuild }
      delete next[normalized]
      muteStatusState.loadingByGuild = next
      changed = true
    }
    if (changed) {
      muteStatusState.version += 1
    }
  },

  clearAll: (): void => {
    if (
      Object.keys(muteStatusState.statusByGuild).length === 0 &&
      Object.keys(muteStatusState.loadingByGuild).length === 0
    ) {
      return
    }
    muteStatusState.statusByGuild = {}
    muteStatusState.loadingByGuild = {}
    muteStatusState.version += 1
  },
})
