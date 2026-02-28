const LAST_LOCATION_KEY = 'discool-last-location'
const LAST_VIEWED_CHANNELS_KEY = 'discool-last-viewed-channels'
const GUILD_ORDER_KEY = 'discool-guild-order'

function getLocalStorage(): Storage | null {
  if (typeof window === 'undefined') return null
  try {
    return window.localStorage
  } catch {
    return null
  }
}

export function saveLastLocation(path: string): void {
  const storage = getLocalStorage()
  if (!storage) return
  try {
    storage.setItem(LAST_LOCATION_KEY, path)
  } catch {
    // Storage can be disabled or full; last-location persistence is best-effort.
    return
  }
}

export function getLastLocation(): string | null {
  const storage = getLocalStorage()
  if (!storage) return null
  try {
    return storage.getItem(LAST_LOCATION_KEY)
  } catch {
    return null
  }
}

export function clearLastLocation(): void {
  const storage = getLocalStorage()
  if (!storage) return
  try {
    storage.removeItem(LAST_LOCATION_KEY)
    storage.removeItem(LAST_VIEWED_CHANNELS_KEY)
    storage.removeItem(GUILD_ORDER_KEY)
  } catch {
    // best-effort
    return
  }
}

function readLastViewedChannels(storage: Storage): Record<string, string> {
  const raw = storage.getItem(LAST_VIEWED_CHANNELS_KEY)
  if (!raw) return {}
  const parsed = JSON.parse(raw) as unknown
  if (!parsed || typeof parsed !== 'object') return {}

  return Object.entries(parsed).reduce<Record<string, string>>(
    (acc, [guildSlug, channelSlug]) => {
      if (typeof channelSlug === 'string' && channelSlug.trim()) {
        acc[guildSlug] = channelSlug
      }
      return acc
    },
    {},
  )
}

export function saveLastViewedChannel(
  guildSlug: string,
  channelSlug: string,
): void {
  const normalizedGuildSlug = guildSlug.trim()
  const normalizedChannelSlug = channelSlug.trim()
  if (!normalizedGuildSlug || !normalizedChannelSlug) return

  const storage = getLocalStorage()
  if (!storage) return
  try {
    const next = readLastViewedChannels(storage)
    next[normalizedGuildSlug] = normalizedChannelSlug
    storage.setItem(LAST_VIEWED_CHANNELS_KEY, JSON.stringify(next))
  } catch {
    return
  }
}

export function getLastViewedChannel(guildSlug: string): string | null {
  const normalizedGuildSlug = guildSlug.trim()
  if (!normalizedGuildSlug) return null

  const storage = getLocalStorage()
  if (!storage) return null
  try {
    return readLastViewedChannels(storage)[normalizedGuildSlug] ?? null
  } catch {
    return null
  }
}

function normalizeGuildOrder(order: string[]): string[] {
  return [...new Set(order.map((slug) => slug.trim()).filter(Boolean))]
}

export function saveGuildOrder(order: string[]): void {
  const storage = getLocalStorage()
  if (!storage) return
  try {
    storage.setItem(GUILD_ORDER_KEY, JSON.stringify(normalizeGuildOrder(order)))
  } catch {
    return
  }
}

export function getGuildOrder(): string[] {
  const storage = getLocalStorage()
  if (!storage) return []
  try {
    const raw = storage.getItem(GUILD_ORDER_KEY)
    if (!raw) return []
    const parsed = JSON.parse(raw) as unknown
    if (!Array.isArray(parsed)) return []
    const order = parsed.filter(
      (slug): slug is string => typeof slug === 'string',
    )
    return normalizeGuildOrder(order)
  } catch {
    return []
  }
}
