const LAST_LOCATION_KEY = 'discool-last-location'

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
  } catch {
    // best-effort
    return
  }
}
