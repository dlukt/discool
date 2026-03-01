import { blockState } from '$lib/features/identity/blockStore.svelte'
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

function conversationActivityAt(conversation: DmConversation): string {
  return (
    conversation.lastMessageAt ||
    conversation.updatedAt ||
    conversation.createdAt
  )
}

function isConversationHidden(conversation: DmConversation): boolean {
  const participantUserId = conversation.participant.userId
  if (!participantUserId) return false
  if (blockState.isBlocked(participantUserId)) return true
  return blockState.isHiddenByBlockWindow(
    participantUserId,
    conversationActivityAt(conversation),
  )
}

function sameConversationList(
  left: DmConversation[],
  right: DmConversation[],
): boolean {
  if (left.length !== right.length) return false
  for (let index = 0; index < left.length; index += 1) {
    const leftItem = left[index]
    const rightItem = right[index]
    if (!rightItem) return false
    if (
      leftItem.dmSlug !== rightItem.dmSlug ||
      leftItem.participant.userId !== rightItem.participant.userId ||
      leftItem.participant.displayName !== rightItem.participant.displayName ||
      leftItem.participant.username !== rightItem.participant.username ||
      leftItem.updatedAt !== rightItem.updatedAt ||
      leftItem.createdAt !== rightItem.createdAt ||
      leftItem.lastMessageAt !== rightItem.lastMessageAt ||
      leftItem.lastMessagePreview !== rightItem.lastMessagePreview ||
      leftItem.hasUnreadActivity !== rightItem.hasUnreadActivity
    ) {
      return false
    }
  }
  return true
}

export const dmState = $state({
  version: 0,
  loading: false,
  loadedOnce: false,
  activeDmSlug: null as string | null,
  allConversations: [] as DmConversation[],
  conversations: [] as DmConversation[],
  unreadBySlug: {} as Record<string, boolean>,

  refreshVisibleConversations: (incrementVersion = true): void => {
    const _blockVersion = blockState.version
    void _blockVersion
    const nextVisible = sortConversations(
      dmState.allConversations.filter(
        (conversation) => !isConversationHidden(conversation),
      ),
    )
    if (sameConversationList(dmState.conversations, nextVisible)) {
      return
    }
    dmState.conversations = nextVisible
    if (incrementVersion) {
      dmState.version += 1
    }
  },

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
      dmState.allConversations = sortConversations(parsed)
      dmState.refreshVisibleConversations(false)
      dmState.loadedOnce = true
      dmState.version += 1
    } finally {
      dmState.loading = false
    }
  },

  openOrCreateDm: async (userId: string): Promise<DmConversation> => {
    const normalizedUserId = userId.trim()
    if (!normalizedUserId) {
      throw new Error('userId is required')
    }
    if (blockState.isBlocked(normalizedUserId)) {
      throw new Error('Unblock this user before opening a direct message')
    }

    const wire = await openDm(normalizedUserId)
    const parsed = toDmConversation(
      wire,
      wire.dm_slug ? dmState.unreadBySlug[wire.dm_slug] === true : false,
    )
    if (!parsed) {
      throw new Error('Invalid DM conversation response')
    }

    const withoutCurrent = dmState.allConversations.filter(
      (conversation) => conversation.dmSlug !== parsed.dmSlug,
    )
    dmState.allConversations = sortConversations([...withoutCurrent, parsed])
    dmState.refreshVisibleConversations(false)
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

    const index = dmState.allConversations.findIndex(
      (conversation) => conversation.dmSlug === normalizedDmSlug,
    )
    if (index < 0) {
      dmState.version += 1
      return
    }

    const next = [...dmState.allConversations]
    const current = next[index]
    if (!current) return
    if (current && current.hasUnreadActivity === hasUnreadActivity) return
    next[index] = {
      ...current,
      hasUnreadActivity:
        isConversationHidden(current) && hasUnreadActivity
          ? false
          : hasUnreadActivity,
    }
    dmState.allConversations = next
    dmState.refreshVisibleConversations(false)
    dmState.version += 1
  },

  noteMessageActivity: (
    dmSlug: string,
    messagePreview: string,
    createdAt: string,
  ): void => {
    const normalizedDmSlug = normalizeDmSlug(dmSlug)
    if (!normalizedDmSlug) return
    const index = dmState.allConversations.findIndex(
      (conversation) => conversation.dmSlug === normalizedDmSlug,
    )
    if (index < 0) return

    const next = [...dmState.allConversations]
    const current = next[index]
    if (!current) return
    next[index] = {
      ...current,
      lastMessagePreview: messagePreview.trim() || current.lastMessagePreview,
      lastMessageAt: createdAt || current.lastMessageAt,
      updatedAt: createdAt || current.updatedAt,
    }
    dmState.allConversations = sortConversations(next)
    dmState.refreshVisibleConversations(false)
    dmState.version += 1
  },

  clearAll: (): void => {
    dmState.loading = false
    dmState.loadedOnce = false
    dmState.activeDmSlug = null
    dmState.allConversations = []
    dmState.conversations = []
    dmState.unreadBySlug = {}
    dmState.version += 1
  },
})

$effect(() => {
  const _blockVersion = blockState.version
  void _blockVersion
  dmState.refreshVisibleConversations()
})
