import {
  addUserBlock,
  listUserBlocks,
  removeUserBlock,
} from '$lib/features/identity/identityApi'
import type { UserBlockEntry } from '$lib/features/identity/types'

type BlockInterval = {
  blockedAt: string
  unblockedAt: string | null
}

type BlockProfileSnapshot = {
  displayName: string | null
  username: string | null
  avatarColor: string | null
}

export type BlockRecord = {
  userId: string
  displayName: string | null
  username: string | null
  avatarColor: string | null
  intervals: BlockInterval[]
}

export type BlockMutationResult = {
  synced: boolean
  syncError: string | null
}

type StoredBlockInterval = {
  blocked_at: string
  unblocked_at: string | null
}

type StoredBlockRecord = {
  user_id: string
  display_name: string | null
  username: string | null
  avatar_color: string | null
  intervals: StoredBlockInterval[]
}

type StoredBlockState = {
  owner_user_id: string
  records: StoredBlockRecord[]
}

const BLOCK_DB_NAME = 'discool-blocks'
const BLOCK_DB_VERSION = 1
const BLOCK_STORE = 'user_blocks'
const NOW_ISO_FALLBACK = '1970-01-01T00:00:00.000Z'

function messageFromError(err: unknown, fallback: string): string {
  if (err instanceof Error && err.message.trim()) {
    return err.message
  }
  return fallback
}

function parseIsoMillis(value: string | null): number | null {
  if (!value) return null
  const parsed = Date.parse(value)
  if (!Number.isFinite(parsed)) return null
  return parsed
}

function compareIso(left: string, right: string): number {
  const leftMs = parseIsoMillis(left)
  const rightMs = parseIsoMillis(right)
  if (leftMs !== null && rightMs !== null) {
    return leftMs - rightMs
  }
  return left.localeCompare(right)
}

function minIso(left: string, right: string): string {
  return compareIso(left, right) <= 0 ? left : right
}

function maxIso(left: string | null, right: string | null): string | null {
  if (!left) return right
  if (!right) return left
  return compareIso(left, right) >= 0 ? left : right
}

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

function normalizeOptionalText(
  value: string | null | undefined,
): string | null {
  if (typeof value !== 'string') return null
  const normalized = value.trim()
  return normalized.length > 0 ? normalized : null
}

function normalizeInterval(interval: BlockInterval): BlockInterval | null {
  const blockedAt = interval.blockedAt.trim()
  if (!blockedAt) return null
  const blockedAtMs = parseIsoMillis(blockedAt)
  if (blockedAtMs === null) return null
  const unblockedAt = normalizeOptionalText(interval.unblockedAt)
  if (!unblockedAt) {
    return { blockedAt, unblockedAt: null }
  }
  const unblockedAtMs = parseIsoMillis(unblockedAt)
  if (unblockedAtMs === null || unblockedAtMs <= blockedAtMs) {
    return null
  }
  return { blockedAt, unblockedAt }
}

function sortIntervals(intervals: BlockInterval[]): BlockInterval[] {
  return [...intervals].sort((left, right) => {
    const byBlocked = compareIso(left.blockedAt, right.blockedAt)
    if (byBlocked !== 0) return byBlocked
    if (left.unblockedAt === null && right.unblockedAt !== null) return 1
    if (left.unblockedAt !== null && right.unblockedAt === null) return -1
    return compareIso(
      left.unblockedAt ?? NOW_ISO_FALLBACK,
      right.unblockedAt ?? NOW_ISO_FALLBACK,
    )
  })
}

function normalizeRecord(record: BlockRecord): BlockRecord | null {
  const normalizedUserId = normalizeOptionalText(record.userId)
  if (!normalizedUserId) return null
  const normalizedIntervals = sortIntervals(
    record.intervals
      .map(normalizeInterval)
      .filter((interval): interval is BlockInterval => interval !== null),
  )
  if (normalizedIntervals.length === 0) return null
  return {
    userId: normalizedUserId,
    displayName: normalizeOptionalText(record.displayName),
    username: normalizeOptionalText(record.username),
    avatarColor: normalizeOptionalText(record.avatarColor),
    intervals: normalizedIntervals,
  }
}

function cloneRecords(
  recordsByUser: Record<string, BlockRecord>,
): Record<string, BlockRecord> {
  const cloned: Record<string, BlockRecord> = {}
  for (const [userId, record] of Object.entries(recordsByUser)) {
    cloned[userId] = {
      ...record,
      intervals: record.intervals.map((interval) => ({ ...interval })),
    }
  }
  return cloned
}

function toStoredRecord(record: BlockRecord): StoredBlockRecord {
  return {
    user_id: record.userId,
    display_name: record.displayName,
    username: record.username,
    avatar_color: record.avatarColor,
    intervals: record.intervals.map((interval) => ({
      blocked_at: interval.blockedAt,
      unblocked_at: interval.unblockedAt,
    })),
  }
}

function fromStoredRecord(record: StoredBlockRecord): BlockRecord | null {
  const userId = normalizeOptionalText(record.user_id)
  if (!userId) return null
  const normalized: BlockRecord = {
    userId,
    displayName: normalizeOptionalText(record.display_name),
    username: normalizeOptionalText(record.username),
    avatarColor: normalizeOptionalText(record.avatar_color),
    intervals: record.intervals
      .map((interval) =>
        normalizeInterval({
          blockedAt: normalizeOptionalText(interval.blocked_at) ?? '',
          unblockedAt: normalizeOptionalText(interval.unblocked_at),
        }),
      )
      .filter((interval): interval is BlockInterval => interval !== null),
  }
  return normalizeRecord(normalized)
}

function isActiveInterval(interval: BlockInterval, atMs: number): boolean {
  const blockedAtMs = parseIsoMillis(interval.blockedAt)
  if (blockedAtMs === null || atMs < blockedAtMs) {
    return false
  }
  const unblockedAtMs = parseIsoMillis(interval.unblockedAt)
  if (unblockedAtMs === null) return true
  return atMs < unblockedAtMs
}

function nowIso(): string {
  return new Date().toISOString()
}

function isBlockedAt(record: BlockRecord | null, atIso: string): boolean {
  if (!record) return false
  const atMs = parseIsoMillis(atIso)
  if (atMs === null) return false
  return record.intervals.some((interval) => isActiveInterval(interval, atMs))
}

function isHiddenByWindowAt(
  record: BlockRecord | null,
  activityAt: string,
): boolean {
  if (!record) return false
  const activityAtMs = parseIsoMillis(activityAt)
  if (activityAtMs === null) return false
  return record.intervals.some((interval) =>
    isActiveInterval(interval, activityAtMs),
  )
}

function mergeServerInterval(
  existingIntervals: BlockInterval[],
  entry: UserBlockEntry,
): BlockInterval[] {
  const blockedAt = entry.blockedAt.trim()
  if (!blockedAt) return existingIntervals
  const next = existingIntervals.map((interval) => ({ ...interval }))
  const unblockedAt = normalizeOptionalText(entry.unblockedAt)

  if (unblockedAt === null) {
    const activeIndex = next.findIndex(
      (interval) => interval.unblockedAt === null,
    )
    if (activeIndex >= 0) {
      const active = next[activeIndex]
      next[activeIndex] = {
        blockedAt: minIso(active.blockedAt, blockedAt),
        unblockedAt: null,
      }
      return sortIntervals(next)
    }
    next.push({ blockedAt, unblockedAt: null })
    return sortIntervals(next)
  }

  const sameStartIndex = next.findIndex(
    (interval) => interval.blockedAt === blockedAt,
  )
  if (sameStartIndex >= 0) {
    const current = next[sameStartIndex]
    next[sameStartIndex] = {
      blockedAt: current.blockedAt,
      unblockedAt: maxIso(current.unblockedAt, unblockedAt),
    }
    return sortIntervals(next)
  }

  const activeIndex = next.findIndex(
    (interval) => interval.unblockedAt === null,
  )
  if (activeIndex >= 0) {
    const current = next[activeIndex]
    next[activeIndex] = {
      blockedAt: minIso(current.blockedAt, blockedAt),
      unblockedAt: maxIso(current.unblockedAt, unblockedAt),
    }
    return sortIntervals(next)
  }

  next.push({ blockedAt, unblockedAt })
  return sortIntervals(next)
}

function mergeServerEntry(
  recordsByUser: Record<string, BlockRecord>,
  entry: UserBlockEntry,
): Record<string, BlockRecord> {
  const userId = entry.blockedUserId.trim()
  if (!userId) return recordsByUser
  const current = recordsByUser[userId]
  const mergedIntervals = mergeServerInterval(current?.intervals ?? [], entry)
  if (mergedIntervals.length === 0) return recordsByUser
  const normalizedRecord = normalizeRecord({
    userId,
    displayName:
      normalizeOptionalText(entry.blockedUserDisplayName) ??
      current?.displayName ??
      null,
    username:
      normalizeOptionalText(entry.blockedUserUsername) ??
      current?.username ??
      null,
    avatarColor:
      normalizeOptionalText(entry.blockedUserAvatarColor) ??
      current?.avatarColor ??
      null,
    intervals: mergedIntervals,
  })
  if (!normalizedRecord) return recordsByUser
  return {
    ...recordsByUser,
    [userId]: normalizedRecord,
  }
}

function isSameRecords(
  left: Record<string, BlockRecord>,
  right: Record<string, BlockRecord>,
): boolean {
  const leftEntries = Object.entries(left)
  const rightEntries = Object.entries(right)
  if (leftEntries.length !== rightEntries.length) return false
  for (const [userId, leftRecord] of leftEntries) {
    const rightRecord = right[userId]
    if (!rightRecord) return false
    if (
      leftRecord.displayName !== rightRecord.displayName ||
      leftRecord.username !== rightRecord.username ||
      leftRecord.avatarColor !== rightRecord.avatarColor ||
      leftRecord.intervals.length !== rightRecord.intervals.length
    ) {
      return false
    }
    for (let index = 0; index < leftRecord.intervals.length; index += 1) {
      const leftInterval = leftRecord.intervals[index]
      const rightInterval = rightRecord.intervals[index]
      if (
        !rightInterval ||
        leftInterval.blockedAt !== rightInterval.blockedAt ||
        leftInterval.unblockedAt !== rightInterval.unblockedAt
      ) {
        return false
      }
    }
  }
  return true
}

function replaceRecords(nextRecords: Record<string, BlockRecord>): void {
  if (isSameRecords(blockState.recordsByUser, nextRecords)) {
    return
  }
  blockState.recordsByUser = nextRecords
  blockState.version += 1
}

function withProfileSnapshot(
  record: BlockRecord | null,
  profile: Partial<BlockProfileSnapshot> | undefined,
): BlockProfileSnapshot {
  return {
    displayName:
      normalizeOptionalText(profile?.displayName) ??
      normalizeOptionalText(record?.displayName) ??
      null,
    username:
      normalizeOptionalText(profile?.username) ??
      normalizeOptionalText(record?.username) ??
      null,
    avatarColor:
      normalizeOptionalText(profile?.avatarColor) ??
      normalizeOptionalText(record?.avatarColor) ??
      null,
  }
}

function applyLocalBlock(
  recordsByUser: Record<string, BlockRecord>,
  blockedUserId: string,
  blockedAt: string,
  profile: Partial<BlockProfileSnapshot> | undefined,
): Record<string, BlockRecord> {
  const current = recordsByUser[blockedUserId] ?? null
  const snapshot = withProfileSnapshot(current, profile)
  const currentlyBlocked = isBlockedAt(current, blockedAt)
  const intervals = currentlyBlocked
    ? (current?.intervals ?? [])
    : sortIntervals([
        ...(current?.intervals ?? []),
        { blockedAt, unblockedAt: null },
      ])
  const normalized = normalizeRecord({
    userId: blockedUserId,
    displayName: snapshot.displayName,
    username: snapshot.username,
    avatarColor: snapshot.avatarColor,
    intervals,
  })
  if (!normalized) return recordsByUser
  return {
    ...recordsByUser,
    [blockedUserId]: normalized,
  }
}

function applyLocalUnblock(
  recordsByUser: Record<string, BlockRecord>,
  blockedUserId: string,
  unblockedAt: string,
): Record<string, BlockRecord> {
  const current = recordsByUser[blockedUserId]
  if (!current) return recordsByUser
  const nextIntervals = current.intervals.map((interval) => ({ ...interval }))
  const activeIndex = nextIntervals.findIndex(
    (interval) => interval.unblockedAt === null,
  )
  if (activeIndex < 0) return recordsByUser
  nextIntervals[activeIndex] = {
    blockedAt: nextIntervals[activeIndex].blockedAt,
    unblockedAt,
  }
  const normalized = normalizeRecord({
    ...current,
    intervals: nextIntervals,
  })
  if (!normalized) return recordsByUser
  return {
    ...recordsByUser,
    [blockedUserId]: normalized,
  }
}

function openBlockDb(): Promise<IDBDatabase> {
  if (typeof indexedDB === 'undefined') {
    throw new Error('IndexedDB is not available in this environment')
  }
  return new Promise((resolve, reject) => {
    const request = indexedDB.open(BLOCK_DB_NAME, BLOCK_DB_VERSION)
    request.onupgradeneeded = () => {
      const db = request.result
      if (!db.objectStoreNames.contains(BLOCK_STORE)) {
        db.createObjectStore(BLOCK_STORE, { keyPath: 'owner_user_id' })
      }
    }
    request.onerror = () => {
      reject(
        request.error ?? new Error('Failed to open block storage database'),
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

async function loadStoredRecords(
  ownerUserId: string,
): Promise<Record<string, BlockRecord>> {
  const db = await openBlockDb()
  try {
    const tx = db.transaction(BLOCK_STORE, 'readonly')
    const stored = await requestToPromise<StoredBlockState | undefined>(
      tx.objectStore(BLOCK_STORE).get(ownerUserId),
    )
    await transactionDone(tx)
    const records: Record<string, BlockRecord> = {}
    for (const storedRecord of stored?.records ?? []) {
      const normalized = fromStoredRecord(storedRecord)
      if (!normalized) continue
      records[normalized.userId] = normalized
    }
    return records
  } finally {
    db.close()
  }
}

async function saveStoredRecords(
  ownerUserId: string,
  recordsByUser: Record<string, BlockRecord>,
): Promise<void> {
  const db = await openBlockDb()
  try {
    const tx = db.transaction(BLOCK_STORE, 'readwrite')
    const payload: StoredBlockState = {
      owner_user_id: ownerUserId,
      records: Object.values(recordsByUser)
        .map((record) => normalizeRecord(record))
        .filter((record): record is BlockRecord => record !== null)
        .map(toStoredRecord),
    }
    tx.objectStore(BLOCK_STORE).put(payload)
    await transactionDone(tx)
  } finally {
    db.close()
  }
}

export const blockState = $state({
  version: 0,
  ownerUserId: null as string | null,
  initialized: false,
  loading: false,
  error: null as string | null,
  recordsByUser: {} as Record<string, BlockRecord>,

  initialize: async (ownerUserId: string | null): Promise<void> => {
    const normalizedOwner = ownerUserId?.trim() || null
    if (!normalizedOwner) {
      blockState.ownerUserId = null
      blockState.initialized = true
      blockState.loading = false
      blockState.error = null
      replaceRecords({})
      return
    }
    if (
      blockState.ownerUserId === normalizedOwner &&
      blockState.initialized &&
      !blockState.loading
    ) {
      return
    }

    blockState.ownerUserId = normalizedOwner
    blockState.initialized = false
    blockState.loading = true
    blockState.error = null

    let loadedRecords: Record<string, BlockRecord> = {}
    try {
      loadedRecords = await loadStoredRecords(normalizedOwner)
    } catch (err) {
      blockState.error = messageFromError(
        err,
        'Failed to load blocked-user data from local storage',
      )
    }
    replaceRecords(loadedRecords)

    try {
      const serverEntries = await listUserBlocks()
      let merged = cloneRecords(blockState.recordsByUser)
      for (const entry of serverEntries) {
        merged = mergeServerEntry(merged, entry)
      }
      if (!isSameRecords(blockState.recordsByUser, merged)) {
        replaceRecords(merged)
        await saveStoredRecords(normalizedOwner, merged)
      }
      blockState.error = null
    } catch (err) {
      blockState.error = messageFromError(
        err,
        'Blocked users were loaded locally, but sync from server failed',
      )
    } finally {
      blockState.loading = false
      blockState.initialized = true
    }
  },

  isBlocked: (blockedUserId: string, atIso: string | null = null): boolean => {
    const normalizedBlockedUserId = blockedUserId.trim()
    if (!normalizedBlockedUserId) return false
    const record = blockState.recordsByUser[normalizedBlockedUserId] ?? null
    return isBlockedAt(record, atIso ?? nowIso())
  },

  isHiddenByBlockWindow: (
    blockedUserId: string,
    activityAt: string,
  ): boolean => {
    const normalizedBlockedUserId = blockedUserId.trim()
    const normalizedActivityAt = activityAt.trim()
    if (!normalizedBlockedUserId || !normalizedActivityAt) return false
    const record = blockState.recordsByUser[normalizedBlockedUserId] ?? null
    if (!record) return false
    if (isBlockedAt(record, nowIso())) return true
    return isHiddenByWindowAt(record, normalizedActivityAt)
  },

  blockedUsers: (): BlockRecord[] => {
    const now = nowIso()
    return Object.values(blockState.recordsByUser)
      .filter((record) => isBlockedAt(record, now))
      .map((record) => ({
        ...record,
        intervals: record.intervals.map((interval) => ({ ...interval })),
      }))
      .sort((left, right) => {
        const leftName = left.displayName ?? left.username ?? left.userId
        const rightName = right.displayName ?? right.username ?? right.userId
        return leftName.localeCompare(rightName)
      })
  },

  blockUser: async (
    blockedUserId: string,
    profile?: Partial<BlockProfileSnapshot>,
  ): Promise<BlockMutationResult> => {
    const ownerUserId = blockState.ownerUserId
    if (!ownerUserId) {
      throw new Error('No authenticated user for blocked-user updates')
    }
    const normalizedBlockedUserId = normalizeUserId(
      blockedUserId,
      'blockedUserId',
    )
    if (normalizedBlockedUserId === ownerUserId) {
      throw new Error('You cannot block yourself')
    }

    const blockedAt = nowIso()
    const nextLocalRecords = applyLocalBlock(
      blockState.recordsByUser,
      normalizedBlockedUserId,
      blockedAt,
      profile,
    )
    replaceRecords(nextLocalRecords)
    await saveStoredRecords(ownerUserId, nextLocalRecords)
    blockState.error = null

    try {
      const serverEntry = await addUserBlock(normalizedBlockedUserId)
      const merged = mergeServerEntry(blockState.recordsByUser, serverEntry)
      if (!isSameRecords(blockState.recordsByUser, merged)) {
        replaceRecords(merged)
        await saveStoredRecords(ownerUserId, merged)
      }
      blockState.error = null
      return { synced: true, syncError: null }
    } catch (err) {
      const syncError = messageFromError(
        err,
        'Blocked locally, but server sync failed',
      )
      blockState.error = syncError
      return { synced: false, syncError }
    }
  },

  unblockUser: async (blockedUserId: string): Promise<BlockMutationResult> => {
    const ownerUserId = blockState.ownerUserId
    if (!ownerUserId) {
      throw new Error('No authenticated user for blocked-user updates')
    }
    const normalizedBlockedUserId = normalizeUserId(
      blockedUserId,
      'blockedUserId',
    )
    if (normalizedBlockedUserId === ownerUserId) {
      throw new Error('You cannot unblock yourself')
    }

    const nextLocalRecords = applyLocalUnblock(
      blockState.recordsByUser,
      normalizedBlockedUserId,
      nowIso(),
    )
    replaceRecords(nextLocalRecords)
    await saveStoredRecords(ownerUserId, nextLocalRecords)
    blockState.error = null

    try {
      await removeUserBlock(normalizedBlockedUserId)
      blockState.error = null
      return { synced: true, syncError: null }
    } catch (err) {
      const syncError = messageFromError(
        err,
        'Unblocked locally, but server sync failed',
      )
      blockState.error = syncError
      return { synced: false, syncError }
    }
  },

  clearAll: (): void => {
    blockState.ownerUserId = null
    blockState.initialized = false
    blockState.loading = false
    blockState.error = null
    replaceRecords({})
  },
})
