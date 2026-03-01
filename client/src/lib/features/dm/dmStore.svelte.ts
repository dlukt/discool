import { listDms, openDm } from './dmApi'
import { type DmConversation, toDmConversation } from './types'

function sortConversations(conversations: DmConversation[]): DmConversation[] {
  return [...conversations].sort((left, right) => {
    const leftSort = left.lastMessageAt || left.updatedAt || left.createdAt
    const rightSort = right.lastMessageAt || right.updatedAt || right.createdAt
    const byTimestamp = rightSort.localeCompare(leftSort)
    if (byTimestamp !== 0) return byTimestamp
    return left.participant.displayName.localeCompare(
      right.participant.displayName,
    )
  })
}

function normalizeDmSlug(value: string): string | null {
  const normalized = value.trim()
  if (!normalized) return null
  return normalized
}

export const dmState = $state({
  version: 0,
  loading: false,
  loadedOnce: false,
  activeDmSlug: null as string | null,
  conversations: [] as DmConversation[],
  unreadBySlug: {} as Record<string, boolean>,

  hasUnreadActivity: (): boolean => {
    return dmState.conversations.some(
      (conversation) => conversation.hasUnreadActivity,
    )
  },

  bySlug: (dmSlug: string): DmConversation | null => {
    const normalizedDmSlug = normalizeDmSlug(dmSlug)
    if (!normalizedDmSlug) return null
    return (
      dmState.conversations.find(
        (conversation) => conversation.dmSlug === normalizedDmSlug,
      ) ?? null
    )
  },

  ensureLoaded: async (force = false): Promise<void> => {
    if (dmState.loading) return
    if (dmState.loadedOnce && !force) return
    dmState.loading = true
    try {
      const wires = await listDms()
      const parsed = wires
        .map((wire) =>
          toDmConversation(
            wire,
            wire.dm_slug ? dmState.unreadBySlug[wire.dm_slug] === true : false,
          ),
        )
        .filter((conversation): conversation is DmConversation =>
          Boolean(conversation),
        )
      dmState.conversations = sortConversations(parsed)
      dmState.loadedOnce = true
      dmState.version += 1
    } finally {
      dmState.loading = false
    }
  },

  openOrCreateDm: async (userId: string): Promise<DmConversation> => {
    const wire = await openDm(userId)
    const parsed = toDmConversation(
      wire,
      wire.dm_slug ? dmState.unreadBySlug[wire.dm_slug] === true : false,
    )
    if (!parsed) {
      throw new Error('Invalid DM conversation response')
    }

    const withoutCurrent = dmState.conversations.filter(
      (conversation) => conversation.dmSlug !== parsed.dmSlug,
    )
    dmState.conversations = sortConversations([...withoutCurrent, parsed])
    dmState.loadedOnce = true
    dmState.version += 1
    return parsed
  },

  setActiveDm: (dmSlug: string | null): void => {
    const normalizedDmSlug = dmSlug?.trim() || null
    dmState.activeDmSlug = normalizedDmSlug
    if (!normalizedDmSlug) return
    dmState.setDmUnreadActivity(normalizedDmSlug, false)
  },

  setDmUnreadActivity: (dmSlug: string, hasUnreadActivity: boolean): void => {
    const normalizedDmSlug = normalizeDmSlug(dmSlug)
    if (!normalizedDmSlug) return

    dmState.unreadBySlug = {
      ...dmState.unreadBySlug,
      [normalizedDmSlug]: hasUnreadActivity,
    }

    const index = dmState.conversations.findIndex(
      (conversation) => conversation.dmSlug === normalizedDmSlug,
    )
    if (index < 0) {
      dmState.version += 1
      return
    }

    const next = [...dmState.conversations]
    const current = next[index]
    if (current && current.hasUnreadActivity === hasUnreadActivity) return
    next[index] = {
      ...current,
      hasUnreadActivity,
    }
    dmState.conversations = next
    dmState.version += 1
  },

  noteMessageActivity: (
    dmSlug: string,
    messagePreview: string,
    createdAt: string,
  ): void => {
    const normalizedDmSlug = normalizeDmSlug(dmSlug)
    if (!normalizedDmSlug) return
    const index = dmState.conversations.findIndex(
      (conversation) => conversation.dmSlug === normalizedDmSlug,
    )
    if (index < 0) return

    const next = [...dmState.conversations]
    const current = next[index]
    if (!current) return
    next[index] = {
      ...current,
      lastMessagePreview: messagePreview.trim() || current.lastMessagePreview,
      lastMessageAt: createdAt || current.lastMessageAt,
      updatedAt: createdAt || current.updatedAt,
    }
    dmState.conversations = sortConversations(next)
    dmState.version += 1
  },

  clearAll: (): void => {
    dmState.loading = false
    dmState.loadedOnce = false
    dmState.activeDmSlug = null
    dmState.conversations = []
    dmState.unreadBySlug = {}
    dmState.version += 1
  },
})
