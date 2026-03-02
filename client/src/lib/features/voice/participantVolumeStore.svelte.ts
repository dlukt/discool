import { normalizeParticipantVolumePercent } from './participantVolume'

type StoredParticipantVolumePreference = {
  participant_user_id: string
  volume_percent: number
}

type StoredParticipantVolumeState = {
  viewer_user_id: string
  preferences: StoredParticipantVolumePreference[]
}

const VOLUME_DB_NAME = 'discool-voice-volumes'
const VOLUME_DB_VERSION = 1
const VOLUME_STORE = 'participant_volume_preferences'

function normalizeUserId(value: string, field: string): string {
  const normalized = value.trim()
  if (!normalized) {
    throw new Error(`${field} is required`)
  }
  if (
    [...normalized].some((char) => {
      const codePoint = char.codePointAt(0) ?? 0
      return codePoint <= 0x1f || codePoint === 0x7f
    })
  ) {
    throw new Error(`${field} contains invalid characters`)
  }
  return normalized
}

function normalizeOptionalUserId(value: unknown): string | null {
  if (typeof value !== 'string') return null
  const normalized = value.trim()
  return normalized.length > 0 ? normalized : null
}

function openParticipantVolumeDb(): Promise<IDBDatabase> {
  if (typeof indexedDB === 'undefined') {
    throw new Error('IndexedDB is not available in this environment')
  }
  return new Promise((resolve, reject) => {
    const request = indexedDB.open(VOLUME_DB_NAME, VOLUME_DB_VERSION)
    request.onupgradeneeded = () => {
      const db = request.result
      if (!db.objectStoreNames.contains(VOLUME_STORE)) {
        db.createObjectStore(VOLUME_STORE, { keyPath: 'viewer_user_id' })
      }
    }
    request.onerror = () => {
      reject(
        request.error ??
          new Error('Failed to open participant volume storage database'),
      )
    }
    request.onsuccess = () => resolve(request.result)
  })
}

function requestToPromise<T>(request: IDBRequest<T>): Promise<T> {
  return new Promise((resolve, reject) => {
    request.onsuccess = () => resolve(request.result)
    request.onerror = () =>
      reject(request.error ?? new Error('IndexedDB request failed'))
  })
}

function transactionDone(tx: IDBTransaction): Promise<void> {
  return new Promise((resolve, reject) => {
    tx.oncomplete = () => resolve()
    tx.onabort = () =>
      reject(tx.error ?? new Error('IndexedDB transaction aborted'))
    tx.onerror = () =>
      reject(tx.error ?? new Error('IndexedDB transaction failed'))
  })
}

export async function loadParticipantVolumePreferences(
  viewerUserId: string,
): Promise<Record<string, number>> {
  const normalizedViewerUserId = normalizeUserId(viewerUserId, 'viewerUserId')
  const db = await openParticipantVolumeDb()
  try {
    const tx = db.transaction(VOLUME_STORE, 'readonly')
    const stored = await requestToPromise<
      StoredParticipantVolumeState | undefined
    >(tx.objectStore(VOLUME_STORE).get(normalizedViewerUserId))
    await transactionDone(tx)
    const preferencesByParticipant: Record<string, number> = {}
    for (const entry of stored?.preferences ?? []) {
      const participantUserId = normalizeOptionalUserId(
        entry.participant_user_id,
      )
      if (!participantUserId) continue
      preferencesByParticipant[participantUserId] =
        normalizeParticipantVolumePercent(entry.volume_percent)
    }
    return preferencesByParticipant
  } finally {
    db.close()
  }
}

export async function saveParticipantVolumePreferences(
  viewerUserId: string,
  preferencesByParticipant: Record<string, number>,
): Promise<void> {
  const normalizedViewerUserId = normalizeUserId(viewerUserId, 'viewerUserId')
  const db = await openParticipantVolumeDb()
  try {
    const tx = db.transaction(VOLUME_STORE, 'readwrite')
    const preferences = Object.entries(preferencesByParticipant)
      .map(([participantUserId, volumePercent]) => {
        const normalizedParticipantUserId =
          normalizeOptionalUserId(participantUserId)
        if (!normalizedParticipantUserId) return null
        return {
          participant_user_id: normalizedParticipantUserId,
          volume_percent: normalizeParticipantVolumePercent(volumePercent),
        }
      })
      .filter(
        (preference): preference is StoredParticipantVolumePreference =>
          preference !== null,
      )
    const payload: StoredParticipantVolumeState = {
      viewer_user_id: normalizedViewerUserId,
      preferences,
    }
    tx.objectStore(VOLUME_STORE).put(payload)
    await transactionDone(tx)
  } finally {
    db.close()
  }
}
