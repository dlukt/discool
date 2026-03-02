<script lang="ts">
// biome-ignore-all lint/correctness/noUnusedVariables: Svelte template usage isn't detected reliably.
import {
  goto,
  type RouteResult,
  route as routerLink,
} from '@mateothegreat/svelte5-router'
import { onMount, tick } from 'svelte'

// biome-ignore lint/correctness/noUnusedImports: Used in Svelte markup; Biome doesn't detect template usage.
import ChannelList from '$lib/features/channel/ChannelList.svelte'
import { channelState } from '$lib/features/channel/channelStore.svelte'
// biome-ignore lint/correctness/noUnusedImports: Used in Svelte markup; Biome doesn't detect template usage.
import MessageArea from '$lib/features/chat/MessageArea.svelte'
import { messageState } from '$lib/features/chat/messageStore.svelte'
import { dmState } from '$lib/features/dm/dmStore.svelte'
// biome-ignore lint/correctness/noUnusedImports: Used in Svelte markup; Biome doesn't detect template usage.
import GuildRail from '$lib/features/guild/GuildRail.svelte'
// biome-ignore lint/correctness/noUnusedImports: Used in Svelte markup; Biome doesn't detect template usage.
import GuildSettings from '$lib/features/guild/GuildSettings.svelte'
import { guildState } from '$lib/features/guild/guildStore.svelte'
// biome-ignore lint/correctness/noUnusedImports: Used in Svelte markup; Biome doesn't detect template usage.
import InviteModal from '$lib/features/guild/InviteModal.svelte'
import { blockState } from '$lib/features/identity/blockStore.svelte'
// biome-ignore lint/correctness/noUnusedImports: Used in Svelte markup; Biome doesn't detect template usage.
import MemberList from '$lib/features/members/MemberList.svelte'
import { presenceState } from '$lib/features/members/presenceStore.svelte'
// biome-ignore lint/correctness/noUnusedImports: Used in Svelte markup; Biome doesn't detect template usage.
import VoiceBar from '$lib/features/voice/VoiceBar.svelte'
// biome-ignore lint/correctness/noUnusedImports: Used in Svelte markup; Biome doesn't detect template usage.
import VoicePanel from '$lib/features/voice/VoicePanel.svelte'
import {
  VOICE_CONNECTION_LOST_MESSAGE,
  voiceState,
} from '$lib/features/voice/voiceStore.svelte'
import { wsClient } from '$lib/ws/client'

type ShellMode = 'home' | 'channel' | 'dm' | 'settings' | 'admin'
type ViewportMode = 'mobile' | 'tablet' | 'desktop'
type MobilePanel = 'guilds' | 'channels' | 'messages' | 'members'
type QuickSwitcherSection = 'Channels' | 'DMs' | 'Guilds'
type QuickSwitcherResult = {
  id: string
  section: QuickSwitcherSection
  label: string
  description: string
  path: string
  searchTokens: string[]
  recencyOrder: number
  stableKey: string
}
type QuickSwitcherRankedResult = QuickSwitcherResult & {
  matchBucket: 0 | 1 | 2
  score: number
}
type QuickSwitcherResultRow = QuickSwitcherRankedResult & {
  selectionIndex: number
}
type QuickSwitcherResultGroup = {
  section: QuickSwitcherSection
  results: QuickSwitcherResultRow[]
}

type Props = {
  route?: RouteResult
  mode: ShellMode
  isAdmin: boolean
  displayName: string
  showRecoveryNudge: boolean
  onOpenSettings?: () => void | Promise<void>
  onDismissRecoveryNudge?: () => void | Promise<void>
  onLogout?: () => void | Promise<void>
  onRouteResolved?: (path: string) => void
}

let {
  route,
  mode,
  isAdmin,
  displayName,
  showRecoveryNudge,
  onOpenSettings,
  onDismissRecoveryNudge,
  onLogout,
  onRouteResolved,
}: Props = $props()

const routerAction = routerLink
let shellMode = $derived(mode)
let canAccessAdmin = $derived(isAdmin)
let shellDisplayName = $derived(displayName)
let shellRecoveryNudge = $derived(showRecoveryNudge)

let viewport = $state<ViewportMode>('desktop')
let tabletMembersVisible = $state(false)
let mobilePanel = $state<MobilePanel>('messages')
let mobileVoiceSheetOpen = $state(false)
let mobileVoiceSheetHandleStartY = $state<number | null>(null)
let guildSettingsOpen = $state(false)
let inviteModalOpen = $state(false)
let mainContent = $state<HTMLElement | null>(null)
let lastFocusedPath = $state<string | null>(null)
let quickSwitcherOpen = $state(false)
let quickSwitcherQuery = $state('')
let quickSwitcherOpenedPath = $state<string | null>(null)
let quickSwitcherInput = $state<HTMLInputElement | null>(null)
let quickSwitcherDialog = $state<HTMLDivElement | null>(null)
let quickSwitcherActiveIndex = $state(-1)
let quickSwitcherTriggerElement = $state<HTMLElement | null>(null)
let quickSwitcherHydrating = $state(false)
let quickSwitcherHydrationError = $state<string | null>(null)
let quickSwitcherHydrationRequestId = $state(0)
let wsLifecycle = $state(wsClient.getLifecycleState())
let isTabletMembersVisible = $derived(tabletMembersVisible)
let activeMobilePanel = $derived(mobilePanel)
let showReconnectingStatus = $derived(wsLifecycle === 'reconnecting')

const QUICK_SWITCHER_SECTION_ORDER: QuickSwitcherSection[] = [
  'Channels',
  'DMs',
  'Guilds',
]

function viewportFromWidth(width: number): ViewportMode {
  if (width < 768) return 'mobile'
  if (width < 1024) return 'tablet'
  return 'desktop'
}

function syncViewport() {
  if (typeof window === 'undefined') return
  viewport = viewportFromWidth(window.innerWidth)
}

function normalizeQuickSwitcherToken(value: string | null | undefined): string {
  return value?.trim().toLowerCase() ?? ''
}

function scoreFuzzySubsequence(query: string, target: string): number | null {
  if (!query || !target) return null
  let queryIndex = 0
  let firstMatchIndex = -1
  let previousMatchIndex = -1
  let gaps = 0
  for (let targetIndex = 0; targetIndex < target.length; targetIndex += 1) {
    if (target[targetIndex] !== query[queryIndex]) continue
    if (firstMatchIndex < 0) firstMatchIndex = targetIndex
    if (previousMatchIndex >= 0) {
      gaps += targetIndex - previousMatchIndex - 1
    }
    previousMatchIndex = targetIndex
    queryIndex += 1
    if (queryIndex === query.length) break
  }
  if (queryIndex !== query.length) return null
  if (firstMatchIndex < 0 || previousMatchIndex < 0) return null
  const span = previousMatchIndex - firstMatchIndex + 1
  if (span <= 0) return null
  const compactness = query.length / span
  const startBonus = 1 / (firstMatchIndex + 1)
  const gapPenalty = gaps / Math.max(1, target.length)
  return compactness + startBonus - gapPenalty
}

function compareQuickSwitcherRankedResults(
  left: QuickSwitcherRankedResult,
  right: QuickSwitcherRankedResult,
): number {
  if (left.matchBucket !== right.matchBucket) {
    return left.matchBucket - right.matchBucket
  }
  if (left.score !== right.score) {
    return right.score - left.score
  }
  if (left.recencyOrder !== right.recencyOrder) {
    return left.recencyOrder - right.recencyOrder
  }
  return left.stableKey.localeCompare(right.stableKey)
}

function rankQuickSwitcherResult(
  query: string,
  result: QuickSwitcherResult,
): QuickSwitcherRankedResult | null {
  let bestCandidate: { matchBucket: 0 | 1 | 2; score: number } | null = null
  for (const token of result.searchTokens) {
    if (!token) continue
    if (token === query) {
      const candidate = { matchBucket: 0 as const, score: query.length }
      if (
        !bestCandidate ||
        candidate.matchBucket < bestCandidate.matchBucket ||
        (candidate.matchBucket === bestCandidate.matchBucket &&
          candidate.score > bestCandidate.score)
      ) {
        bestCandidate = candidate
      }
      continue
    }
    if (token.startsWith(query)) {
      const candidate = {
        matchBucket: 1 as const,
        score: query.length / Math.max(1, token.length),
      }
      if (
        !bestCandidate ||
        candidate.matchBucket < bestCandidate.matchBucket ||
        (candidate.matchBucket === bestCandidate.matchBucket &&
          candidate.score > bestCandidate.score)
      ) {
        bestCandidate = candidate
      }
      continue
    }
    const fuzzyScore = scoreFuzzySubsequence(query, token)
    if (fuzzyScore === null) continue
    const candidate = { matchBucket: 2 as const, score: fuzzyScore }
    if (
      !bestCandidate ||
      candidate.matchBucket < bestCandidate.matchBucket ||
      (candidate.matchBucket === bestCandidate.matchBucket &&
        candidate.score > bestCandidate.score)
    ) {
      bestCandidate = candidate
    }
  }
  if (!bestCandidate) return null
  return {
    ...result,
    matchBucket: bestCandidate.matchBucket,
    score: bestCandidate.score,
  }
}

function resolveGuildChannelSlug(guild: {
  slug: string
  defaultChannelSlug: string
  lastViewedChannelSlug?: string | null
}): string | null {
  const orderedChannels = channelState.orderedChannelsForGuild(guild.slug)
  return (
    guild.lastViewedChannelSlug ??
    orderedChannels[0]?.slug ??
    guild.defaultChannelSlug ??
    null
  )
}

function resolveChannelType(channel: {
  channelType?: unknown
  kind?: unknown
}): string | null {
  if (typeof channel.channelType === 'string') {
    return channel.channelType.trim().toLowerCase()
  }
  if (typeof channel.kind === 'string') {
    return channel.kind.trim().toLowerCase()
  }
  return null
}

function getQuickSwitcherFocusableElements(): HTMLElement[] {
  if (!quickSwitcherDialog || typeof document === 'undefined') return []
  const focusableSelector =
    'a[href],button:not([disabled]),input:not([disabled]),select:not([disabled]),textarea:not([disabled]),[tabindex]:not([tabindex="-1"])'
  return Array.from(
    quickSwitcherDialog.querySelectorAll<HTMLElement>(focusableSelector),
  ).filter((element) => !element.hasAttribute('disabled'))
}

function restoreQuickSwitcherTriggerFocus(): void {
  const trigger = quickSwitcherTriggerElement
  quickSwitcherTriggerElement = null
  if (!trigger || typeof document === 'undefined') return
  void tick().then(() => {
    if (!document.contains(trigger)) return
    trigger.focus()
  })
}

function closeQuickSwitcher(options?: { restoreFocus?: boolean }): void {
  const shouldRestoreFocus = options?.restoreFocus ?? true
  quickSwitcherOpen = false
  quickSwitcherQuery = ''
  quickSwitcherActiveIndex = -1
  quickSwitcherHydrationError = null
  if (shouldRestoreFocus) {
    restoreQuickSwitcherTriggerFocus()
  } else {
    quickSwitcherTriggerElement = null
  }
}

function openQuickSwitcher(): void {
  if (quickSwitcherOpen) return
  if (
    typeof document !== 'undefined' &&
    document.activeElement instanceof HTMLElement
  ) {
    quickSwitcherTriggerElement = document.activeElement
  } else {
    quickSwitcherTriggerElement = null
  }
  quickSwitcherQuery = ''
  quickSwitcherActiveIndex = 0
  quickSwitcherOpen = true
}

async function hydrateQuickSwitcherChannels(): Promise<void> {
  const guildsToHydrate = guildState.guilds.filter(
    (guild) =>
      guild.slug !== activeGuild &&
      channelState.orderedChannelsForGuild(guild.slug).length === 0,
  )
  if (guildsToHydrate.length === 0) return

  const hydrationRequestId = quickSwitcherHydrationRequestId + 1
  quickSwitcherHydrationRequestId = hydrationRequestId
  quickSwitcherHydrating = true
  quickSwitcherHydrationError = null
  try {
    for (const guild of guildsToHydrate) {
      await channelState.loadChannels(guild.slug)
    }
  } catch (error) {
    if (quickSwitcherHydrationRequestId !== hydrationRequestId) return
    quickSwitcherHydrationError =
      error instanceof Error
        ? error.message
        : 'Failed to hydrate quick switcher channels'
  } finally {
    const isLatestRequest =
      quickSwitcherHydrationRequestId === hydrationRequestId
    if (isLatestRequest && activeGuild) {
      try {
        await channelState.loadChannels(activeGuild)
      } catch (error) {
        quickSwitcherHydrationError =
          error instanceof Error
            ? error.message
            : 'Failed to restore active guild channels'
      }
    }
    if (isLatestRequest) {
      quickSwitcherHydrating = false
    }
  }
}

function moveQuickSwitcherSelection(delta: 1 | -1): void {
  const total = quickSwitcherFlatResults.length
  if (total === 0) return
  if (quickSwitcherActiveIndex < 0 || quickSwitcherActiveIndex >= total) {
    quickSwitcherActiveIndex = 0
    return
  }
  quickSwitcherActiveIndex = (quickSwitcherActiveIndex + delta + total) % total
}

function trapQuickSwitcherFocus(event: KeyboardEvent): void {
  if (event.key !== 'Tab') return
  const focusable = getQuickSwitcherFocusableElements()
  if (focusable.length === 0) {
    event.preventDefault()
    quickSwitcherInput?.focus()
    return
  }
  const first = focusable[0]
  const last = focusable[focusable.length - 1]
  const activeElement =
    typeof document !== 'undefined'
      ? (document.activeElement as HTMLElement | null)
      : null

  if (event.shiftKey) {
    if (!activeElement || activeElement === first) {
      event.preventDefault()
      last?.focus()
    }
    return
  }

  if (!activeElement || activeElement === last) {
    event.preventDefault()
    first?.focus()
  }
}

function handleQuickSwitcherKeydown(event: KeyboardEvent): void {
  if (!quickSwitcherOpen) return
  const isInputTarget = event.target === quickSwitcherInput
  if (event.key === 'Escape') {
    event.preventDefault()
    closeQuickSwitcher()
    return
  }
  if (event.key === 'ArrowDown') {
    if (!isInputTarget) return
    event.preventDefault()
    moveQuickSwitcherSelection(1)
    return
  }
  if (event.key === 'ArrowUp') {
    if (!isInputTarget) return
    event.preventDefault()
    moveQuickSwitcherSelection(-1)
    return
  }
  if (event.key === 'Enter') {
    if (!isInputTarget) return
    if (quickSwitcherActiveIndex < 0) return
    const selectedResult = quickSwitcherFlatResults[quickSwitcherActiveIndex]
    if (!selectedResult) return
    event.preventDefault()
    void handleQuickSwitcherPick(selectedResult.path)
    return
  }
  trapQuickSwitcherFocus(event)
}

async function navigateUnreadChannel(direction: 'up' | 'down'): Promise<void> {
  const unreadSlugs = messageState.unreadChannelSlugsForGuild(activeGuild)
  if (unreadSlugs.length === 0) return
  const currentIndex = unreadSlugs.indexOf(activeChannel)
  let nextIndex = 0
  if (currentIndex >= 0) {
    const delta = direction === 'down' ? 1 : -1
    nextIndex = (currentIndex + delta + unreadSlugs.length) % unreadSlugs.length
  } else {
    nextIndex = direction === 'down' ? 0 : unreadSlugs.length - 1
  }
  const nextChannel = unreadSlugs[nextIndex]
  if (!nextChannel || nextChannel === activeChannel) return
  await goto(`/${activeGuild}/${nextChannel}`)
}

function isEditableShortcutTarget(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false
  if (target.isContentEditable) return true
  const tagName = target.tagName.toLowerCase()
  if (tagName === 'input' || tagName === 'textarea') return true
  return target.closest('[contenteditable="true"]') !== null
}

onMount(() => {
  syncViewport()
  void dmState.ensureLoaded().catch(() => {})
  const unsubscribeLifecycle = wsClient.subscribeLifecycle((state) => {
    wsLifecycle = state
  })
  if (typeof window === 'undefined') return undefined
  const handleUnreadHotkey = (event: KeyboardEvent) => {
    if (!event.altKey || !event.shiftKey || event.ctrlKey || event.metaKey)
      return
    if (event.key === 'ArrowDown') {
      event.preventDefault()
      void navigateUnreadChannel('down')
      return
    }
    if (event.key === 'ArrowUp') {
      event.preventDefault()
      void navigateUnreadChannel('up')
    }
  }
  const handleQuickSwitcherHotkey = (event: KeyboardEvent) => {
    if (
      (event.ctrlKey || event.metaKey) &&
      !event.altKey &&
      event.key.toLowerCase() === 'k'
    ) {
      event.preventDefault()
      if (quickSwitcherOpen) {
        closeQuickSwitcher()
        return
      }
      openQuickSwitcher()
    }
  }
  const handleVoiceHotkey = (event: KeyboardEvent) => {
    if (isEditableShortcutTarget(event.target)) return
    if (event.altKey || event.metaKey) return
    if (voiceState.status !== 'connected') return
    const normalizedKey = event.key.toLowerCase()
    if (event.ctrlKey && normalizedKey === 'd') {
      event.preventDefault()
      voiceState.disconnect()
      return
    }
    if (event.ctrlKey || event.shiftKey) return
    if (normalizedKey === 'm') {
      event.preventDefault()
      voiceState.toggleMute()
      return
    }
    if (normalizedKey === 'd') {
      event.preventDefault()
      voiceState.toggleDeafen()
    }
  }
  const handleOpenDmIntent = (
    event: Event & { detail?: { userId?: unknown } },
  ) => {
    const detail = event.detail
    if (!detail || typeof detail.userId !== 'string') return
    if (blockState.isBlocked(detail.userId)) return
    void dmState
      .openOrCreateDm(detail.userId)
      .then((conversation) => goto(`/dm/${conversation.dmSlug}`))
      .catch(() => {})
  }
  const handleOpenMessageHistoryIntent = (
    event: Event & {
      detail?: {
        guildSlug?: unknown
        channelSlug?: unknown
        messageId?: unknown
      }
    },
  ) => {
    const detail = event.detail
    if (
      !detail ||
      typeof detail.guildSlug !== 'string' ||
      typeof detail.channelSlug !== 'string' ||
      typeof detail.messageId !== 'string'
    ) {
      return
    }
    const guildSlug = detail.guildSlug.trim()
    const channelSlug = detail.channelSlug.trim()
    const messageId = detail.messageId.trim()
    if (!guildSlug || !channelSlug || !messageId) return
    messageState.requestChannelMessageJump(guildSlug, channelSlug, messageId)
    void goto(`/${guildSlug}/${channelSlug}`)
  }
  window.addEventListener('resize', syncViewport)
  window.addEventListener('keydown', handleUnreadHotkey)
  window.addEventListener('keydown', handleQuickSwitcherHotkey)
  window.addEventListener('keydown', handleVoiceHotkey)
  window.addEventListener(
    'discool:open-dm-intent',
    handleOpenDmIntent as EventListener,
  )
  window.addEventListener(
    'discool:open-message-history-intent',
    handleOpenMessageHistoryIntent as EventListener,
  )
  return () => {
    unsubscribeLifecycle()
    window.removeEventListener('resize', syncViewport)
    window.removeEventListener('keydown', handleUnreadHotkey)
    window.removeEventListener('keydown', handleQuickSwitcherHotkey)
    window.removeEventListener('keydown', handleVoiceHotkey)
    window.removeEventListener(
      'discool:open-dm-intent',
      handleOpenDmIntent as EventListener,
    )
    window.removeEventListener(
      'discool:open-message-history-intent',
      handleOpenMessageHistoryIntent as EventListener,
    )
  }
})

let currentPath = $derived(
  route?.result?.path?.original ??
    (typeof window !== 'undefined' ? window.location.pathname : '/'),
)
let params = $derived(
  (route?.result?.path?.params ?? {}) as Record<string, string | undefined>,
)
let activeGuild = $derived(params.guild ?? 'lobby')
let activeChannel = $derived(params.channel ?? 'general')
let activeDm = $derived(params.dm ?? null)
let voiceStatusMessage = $derived(
  shellMode === 'channel'
    ? voiceState.statusMessageForChannel(activeGuild, activeChannel)
    : null,
)
let voiceConnectionState = $derived(
  shellMode === 'channel'
    ? voiceState.statusForChannel(activeGuild, activeChannel)
    : 'idle',
)
let showMobileVoiceControls = $derived(
  viewport === 'mobile' &&
    shellMode === 'channel' &&
    (voiceConnectionState === 'connected' ||
      voiceConnectionState === 'retrying' ||
      (voiceConnectionState === 'failed' &&
        voiceStatusMessage === VOICE_CONNECTION_LOST_MESSAGE)),
)
let activeVoiceParticipants = $derived(
  showMobileVoiceControls ? voiceState.activeChannelParticipants() : [],
)
let mobileVoiceQuality = $derived.by(() => {
  if (voiceConnectionState === 'connected') return 'green'
  if (
    voiceConnectionState === 'connecting' ||
    voiceConnectionState === 'retrying'
  )
    return 'yellow'
  return 'red'
})
let mobileVoiceQualityLabel = $derived.by(() => {
  if (mobileVoiceQuality === 'green') return 'Good'
  if (mobileVoiceQuality === 'yellow') return 'Limited'
  return 'Poor'
})
let canOpenGuildSettings = $derived(shellMode === 'channel')
let canOpenInvites = $derived(shellMode === 'channel')
let quickQueryNormalized = $derived(
  normalizeQuickSwitcherToken(quickSwitcherQuery),
)
let quickDmResults = $derived.by(() => {
  return dmState.conversations.map((conversation, index) => {
    const displayName =
      conversation.participant.displayName.trim() ||
      conversation.participant.username
    const username = conversation.participant.username.trim()
    return {
      id: `dm:${conversation.dmSlug}`,
      section: 'DMs' as const,
      label: displayName,
      description: `@${username}`,
      path: `/dm/${conversation.dmSlug}`,
      searchTokens: [
        normalizeQuickSwitcherToken(displayName),
        normalizeQuickSwitcherToken(username),
        normalizeQuickSwitcherToken(`@${username}`),
      ],
      recencyOrder: index,
      stableKey: `dm:${username}:${conversation.dmSlug}`,
    }
  })
})
let quickGuildResults = $derived.by(() => {
  const results: QuickSwitcherResult[] = []
  for (const [index, guild] of guildState.guilds.entries()) {
    const channelSlug = resolveGuildChannelSlug(guild)
    if (!channelSlug) continue
    results.push({
      id: `guild:${guild.slug}`,
      section: 'Guilds',
      label: guild.name,
      description: `/${guild.slug}`,
      path: `/${guild.slug}/${channelSlug}`,
      searchTokens: [
        normalizeQuickSwitcherToken(guild.name),
        normalizeQuickSwitcherToken(guild.slug),
      ],
      recencyOrder: guild.slug === activeGuild ? -1 : index * 1_000 + 200,
      stableKey: `guild:${guild.slug}`,
    })
  }
  return results
})
let quickChannelResults = $derived.by(() => {
  const results: QuickSwitcherResult[] = []
  for (const [guildIndex, guild] of guildState.guilds.entries()) {
    const orderedChannels = channelState.orderedChannelsForGuild(guild.slug)
    const baseRecency = guildIndex * 1_000
    const fallbackChannelSlug = resolveGuildChannelSlug(guild)

    if (orderedChannels.length === 0 && fallbackChannelSlug) {
      const label = `#${fallbackChannelSlug}`
      results.push({
        id: `channel:${guild.slug}:${fallbackChannelSlug}`,
        section: 'Channels',
        label,
        description: `${guild.name} • /${guild.slug}`,
        path: `/${guild.slug}/${fallbackChannelSlug}`,
        searchTokens: [
          normalizeQuickSwitcherToken(guild.name),
          normalizeQuickSwitcherToken(guild.slug),
          normalizeQuickSwitcherToken(fallbackChannelSlug),
          normalizeQuickSwitcherToken(`${guild.name} ${fallbackChannelSlug}`),
        ],
        recencyOrder:
          guild.slug === activeGuild && fallbackChannelSlug === activeChannel
            ? -3
            : baseRecency + 10,
        stableKey: `channel:${guild.slug}:${fallbackChannelSlug}`,
      })
      continue
    }

    for (const [channelIndex, channel] of orderedChannels.entries()) {
      const channelLabel = channel.name.trim() || channel.slug
      const isActiveChannel =
        guild.slug === activeGuild && channel.slug === activeChannel
      const isLastViewedChannel = channel.slug === guild.lastViewedChannelSlug
      let recencyOrder = baseRecency + channelIndex + 10
      if (isActiveChannel) {
        recencyOrder = -3
      } else if (guild.slug === activeGuild && isLastViewedChannel) {
        recencyOrder = -2
      } else if (guild.slug === activeGuild) {
        recencyOrder = -1 + channelIndex
      } else if (isLastViewedChannel) {
        recencyOrder = baseRecency
      }

      results.push({
        id: `channel:${guild.slug}:${channel.slug}`,
        section: 'Channels',
        label: `#${channelLabel}`,
        description: `${guild.name} • /${guild.slug}`,
        path: `/${guild.slug}/${channel.slug}`,
        searchTokens: [
          normalizeQuickSwitcherToken(channelLabel),
          normalizeQuickSwitcherToken(channel.slug),
          normalizeQuickSwitcherToken(guild.name),
          normalizeQuickSwitcherToken(guild.slug),
          normalizeQuickSwitcherToken(`${guild.name} ${channelLabel}`),
          normalizeQuickSwitcherToken(`${guild.slug} ${channel.slug}`),
        ],
        recencyOrder,
        stableKey: `channel:${guild.slug}:${channel.slug}`,
      })
    }
  }
  return results
})
let quickSwitcherRankedResults = $derived.by(() => {
  const allResults = [
    ...quickChannelResults,
    ...quickDmResults,
    ...quickGuildResults,
  ] as QuickSwitcherResult[]

  if (!quickQueryNormalized) {
    const rankedDefaults = allResults.map((result) => ({
      ...result,
      matchBucket: 2 as const,
      score: 0,
    }))
    rankedDefaults.sort(compareQuickSwitcherRankedResults)
    return rankedDefaults
  }

  const rankedResults = allResults
    .map((result) => rankQuickSwitcherResult(quickQueryNormalized, result))
    .filter((result): result is QuickSwitcherRankedResult => Boolean(result))
  rankedResults.sort(compareQuickSwitcherRankedResults)
  return rankedResults
})
let quickSwitcherGroupedResults = $derived.by(() => {
  const groupedBySection: Record<
    QuickSwitcherSection,
    QuickSwitcherRankedResult[]
  > = {
    Channels: [],
    DMs: [],
    Guilds: [],
  }

  for (const result of quickSwitcherRankedResults) {
    groupedBySection[result.section].push(result)
  }

  const groupedResults: QuickSwitcherResultGroup[] = []
  let selectionIndex = 0
  for (const section of QUICK_SWITCHER_SECTION_ORDER) {
    const sectionResults = groupedBySection[section]
    if (sectionResults.length === 0) continue
    groupedResults.push({
      section,
      results: sectionResults.map((result) => ({
        ...result,
        selectionIndex: selectionIndex++,
      })),
    })
  }

  return groupedResults
})
let quickSwitcherFlatResults = $derived.by(() =>
  quickSwitcherGroupedResults.flatMap((group) => group.results),
)
const MOBILE_VOICE_SHEET_CLOSE_SWIPE_PX = 40

function openMobilePanel(panel: MobilePanel) {
  mobilePanel = panel
}

function openMobileVoiceSheet() {
  if (!showMobileVoiceControls) return
  mobileVoiceSheetOpen = true
}

function closeMobileVoiceSheet() {
  mobileVoiceSheetOpen = false
  mobileVoiceSheetHandleStartY = null
}

function handleMobileVoiceSheetKeydown(event: KeyboardEvent) {
  if (event.key !== 'Escape') return
  event.preventDefault()
  closeMobileVoiceSheet()
}

function handleMobileVoiceSheetHandlePointerDown(event: PointerEvent) {
  if (event.pointerType === 'mouse') return
  mobileVoiceSheetHandleStartY = event.clientY
}

function handleMobileVoiceSheetHandlePointerCancel() {
  mobileVoiceSheetHandleStartY = null
}

function handleMobileVoiceSheetHandlePointerUp(event: PointerEvent) {
  if (mobileVoiceSheetHandleStartY === null) return
  const swipeDelta = event.clientY - mobileVoiceSheetHandleStartY
  mobileVoiceSheetHandleStartY = null
  if (swipeDelta >= MOBILE_VOICE_SHEET_CLOSE_SWIPE_PX) {
    closeMobileVoiceSheet()
  }
}

function toggleTabletMembers() {
  tabletMembersVisible = !tabletMembersVisible
}

async function handleLogout() {
  await onLogout?.()
}

async function handleOpenSettings() {
  await onOpenSettings?.()
}

async function handleDismissRecoveryNudge() {
  await onDismissRecoveryNudge?.()
}

async function handleQuickSwitcherPick(path: string): Promise<void> {
  closeQuickSwitcher({ restoreFocus: false })
  await goto(path)
}

function openGuildSettings() {
  guildSettingsOpen = true
}

function openInviteModal() {
  inviteModalOpen = true
}

async function handleCloseGuildSettings() {
  guildSettingsOpen = false
}

async function handleCloseInviteModal() {
  inviteModalOpen = false
}

$effect(() => {
  if (viewport !== 'tablet') tabletMembersVisible = false
})

$effect(() => {
  if (viewport !== 'mobile') mobilePanel = 'messages'
})

$effect(() => {
  if (showMobileVoiceControls) return
  mobileVoiceSheetOpen = false
})

$effect(() => {
  if (shellMode !== 'channel') return
  if (guildState.loading) return
  if (guildState.bySlug(activeGuild)) return

  const fallbackGuild = guildState.guilds[0]
  if (!fallbackGuild) {
    if (currentPath !== '/') {
      void goto('/')
    }
    return
  }

  const fallbackChannel = resolveGuildChannelSlug(fallbackGuild)
  if (!fallbackChannel) {
    if (currentPath !== '/') {
      void goto('/')
    }
    return
  }

  const fallbackPath = `/${fallbackGuild.slug}/${fallbackChannel}`
  if (currentPath === fallbackPath) return
  void goto(fallbackPath)
})

$effect(() => {
  if (shellMode === 'channel') {
    presenceState.setRouting(activeGuild, activeChannel)
    return
  }
  presenceState.clearRouting()
})

$effect(() => {
  if (shellMode !== 'channel') {
    voiceState.clearActiveChannel()
    return
  }
  const active = channelState
    .orderedChannelsForGuild(activeGuild)
    .find((channel) => channel.slug === activeChannel)
  if (active && resolveChannelType(active) === 'voice') {
    voiceState.activateVoiceChannel(activeGuild, activeChannel)
    return
  }
  voiceState.clearActiveChannel()
})

$effect(() => {
  if (!currentPath || currentPath === lastFocusedPath) return
  lastFocusedPath = currentPath
  onRouteResolved?.(currentPath)
  void tick().then(() => {
    mainContent?.focus()
  })
})

$effect(() => {
  if (!quickSwitcherOpen) return
  quickSwitcherActiveIndex = 0
  void tick().then(() => {
    quickSwitcherInput?.focus()
    quickSwitcherInput?.select()
  })
  void hydrateQuickSwitcherChannels()
})

$effect(() => {
  if (!quickSwitcherOpen) {
    quickSwitcherActiveIndex = -1
    return
  }
  const totalResults = quickSwitcherFlatResults.length
  if (totalResults === 0) {
    quickSwitcherActiveIndex = -1
    return
  }
  if (
    quickSwitcherActiveIndex < 0 ||
    quickSwitcherActiveIndex >= totalResults
  ) {
    quickSwitcherActiveIndex = 0
  }
})

$effect(() => {
  if (!quickSwitcherOpen) {
    quickSwitcherOpenedPath = null
    return
  }
  if (quickSwitcherOpenedPath === null) {
    quickSwitcherOpenedPath = currentPath
    return
  }
  if (currentPath === quickSwitcherOpenedPath) return
  closeQuickSwitcher({ restoreFocus: false })
  quickSwitcherOpenedPath = currentPath
})
</script>

<a
  href="#main-content"
  class="sr-only focus:not-sr-only focus:fixed focus:left-4 focus:top-4 focus:z-50 focus:rounded-md focus:bg-background focus:px-3 focus:py-2 focus:text-sm focus:text-foreground focus:ring-2 focus:ring-primary"
>
  Skip to content
</a>

{#if showReconnectingStatus}
  <div
    class="fixed right-3 top-3 z-50 rounded-md border border-border bg-card px-3 py-2 text-xs text-muted-foreground shadow"
    data-testid="reconnecting-status"
  >
    Reconnecting...
  </div>
{/if}

{#if viewport === 'mobile'}
  <div class="flex min-h-screen flex-col bg-background">
    <header class="border-b border-border px-3 py-2">
      <div class="flex items-center justify-between gap-2">
        <span class="text-sm font-semibold text-foreground">Discool</span>
        <div class="flex items-center gap-2">
          <a
            class="rounded-md bg-muted px-2 py-1 text-xs text-foreground"
            href="/"
            use:routerAction
          >
            Home
          </a>
          {#if canAccessAdmin}
            <a
              class="rounded-md bg-muted px-2 py-1 text-xs text-foreground"
              href="/admin"
              use:routerAction
            >
              Admin
            </a>
          {/if}
          <a
            class="rounded-md bg-muted px-2 py-1 text-xs text-foreground"
            href="/settings"
            use:routerAction
          >
            Settings
          </a>
          {#if canOpenInvites}
            <button
              type="button"
              class="rounded-md bg-muted px-2 py-1 text-xs text-foreground"
              onclick={openInviteModal}
            >
              Invite people
            </button>
          {/if}
          {#if canOpenGuildSettings}
            <button
              type="button"
              class="rounded-md bg-muted px-2 py-1 text-xs text-foreground"
              onclick={openGuildSettings}
            >
              Guild settings
            </button>
          {/if}
          <button
            type="button"
            class="rounded-md bg-destructive px-2 py-1 text-xs font-medium text-destructive-foreground"
            onclick={() => void handleLogout()}
          >
            Log out
          </button>
        </div>
      </div>
    </header>

    <main
      id="main-content"
      tabindex="-1"
      bind:this={mainContent}
      class="flex min-h-0 flex-1 flex-col overflow-hidden outline-none"
    >
      {#if showMobileVoiceControls}
        <div
          class="border-b border-border px-2 pb-2 pt-[calc(env(safe-area-inset-top,0px)+0.5rem)]"
          data-testid="mobile-voice-bar-container"
        >
          <VoiceBar
            guildName={activeGuild}
            channelName={activeChannel}
            connectionState={voiceConnectionState}
            variant="mobile"
            isMuted={voiceState.isMuted}
            isDeafened={voiceState.isDeafened}
            isParticipantsOpen={mobileVoiceSheetOpen}
            onToggleParticipants={openMobileVoiceSheet}
            onToggleMute={() => voiceState.toggleMute()}
            onToggleDeafen={() => voiceState.toggleDeafen()}
            onDisconnect={() => voiceState.disconnect()}
            onOpenSheet={openMobileVoiceSheet}
          />
        </div>
      {/if}

      <div class="min-h-0 flex-1 overflow-auto">
        {#if activeMobilePanel === 'guilds'}
          <GuildRail
            activeGuild={activeGuild}
            activeChannel={activeChannel}
            activeDm={activeDm}
            mode={shellMode}
          />
        {:else if activeMobilePanel === 'channels'}
          <ChannelList activeGuild={activeGuild} activeChannel={activeChannel} />
        {:else if activeMobilePanel === 'members'}
          <MemberList activeGuild={activeGuild} />
        {:else}
          <MessageArea
            mode={shellMode}
            activeGuild={activeGuild}
            activeChannel={activeChannel}
            activeDm={activeDm}
            displayName={shellDisplayName}
            isAdmin={canAccessAdmin}
            showRecoveryNudge={shellRecoveryNudge}
            showVoiceControls={false}
            onOpenSettings={handleOpenSettings}
            onDismissRecoveryNudge={handleDismissRecoveryNudge}
          />
        {/if}
      </div>
    </main>

    <nav
      class="grid grid-cols-4 gap-1 border-t border-border bg-card p-2"
      aria-label="Mobile shell navigation"
    >
      <button
        type="button"
        class="rounded-md px-2 py-2 text-xs font-medium text-foreground hover:bg-muted"
        onclick={() => openMobilePanel('guilds')}
        aria-label="Guilds"
      >
        Guilds
      </button>
      <button
        type="button"
        class="rounded-md px-2 py-2 text-xs font-medium text-foreground hover:bg-muted"
        onclick={() => openMobilePanel('channels')}
        aria-label="Channels"
      >
        Channels
      </button>
      <button
        type="button"
        class="rounded-md px-2 py-2 text-xs font-medium text-foreground hover:bg-muted"
        onclick={() => openMobilePanel('messages')}
        aria-label="Messages"
      >
        Messages
      </button>
      <button
        type="button"
        class="rounded-md px-2 py-2 text-xs font-medium text-foreground hover:bg-muted"
        onclick={() => openMobilePanel('members')}
        aria-label="Members"
      >
        Members
      </button>
    </nav>

    {#if showMobileVoiceControls && mobileVoiceSheetOpen}
      <div
        class="fixed inset-0 z-30 bg-black/40"
        data-testid="mobile-voice-sheet-backdrop"
        role="presentation"
        onpointerdown={closeMobileVoiceSheet}
      ></div>
      <div
        class="fixed inset-x-0 bottom-0 z-40 max-h-[85vh] overflow-hidden rounded-t-2xl border border-border bg-card shadow-2xl"
        role="dialog"
        tabindex="-1"
        aria-modal="true"
        aria-label="Mobile voice controls"
        data-testid="mobile-voice-sheet"
        onkeydown={handleMobileVoiceSheetKeydown}
      >
        <div class="space-y-4 px-4 pb-[calc(env(safe-area-inset-bottom,0px)+1rem)] pt-3">
          <div class="flex items-center justify-between gap-2">
            <button
              type="button"
              class="mx-auto h-12 w-12 touch-manipulation rounded-full text-muted-foreground"
              aria-label="Swipe down to close voice controls"
              data-testid="mobile-voice-sheet-handle"
              onpointerdown={handleMobileVoiceSheetHandlePointerDown}
              onpointerup={handleMobileVoiceSheetHandlePointerUp}
              onpointercancel={handleMobileVoiceSheetHandlePointerCancel}
            >
              <span class="mx-auto block h-1.5 w-12 rounded-full bg-muted"></span>
            </button>
            <button
              type="button"
              class="inline-flex h-12 w-12 items-center justify-center rounded-md border border-border bg-background text-foreground hover:bg-muted"
              aria-label="Close voice controls"
              data-testid="mobile-voice-sheet-close"
              onclick={closeMobileVoiceSheet}
            >
              ✕
            </button>
          </div>

          <div class="flex items-center justify-between gap-2">
            <p class="truncate text-sm font-semibold text-foreground">
              #{activeChannel}
            </p>
            <div class="flex items-center gap-2 text-xs text-muted-foreground">
              <span
                class={`h-2.5 w-2.5 rounded-full ${mobileVoiceQuality === 'green' ? 'bg-emerald-500' : mobileVoiceQuality === 'yellow' ? 'bg-amber-400' : 'bg-destructive'}`}
                data-testid="mobile-voice-sheet-quality-dot"
                data-quality={mobileVoiceQuality}
              ></span>
              <span data-testid="mobile-voice-sheet-quality-label">
                {mobileVoiceQualityLabel}
              </span>
            </div>
          </div>

          {#if voiceStatusMessage}
            <p
              class="rounded-md border border-destructive/40 bg-destructive/10 px-3 py-2 text-xs text-destructive"
              data-testid="mobile-voice-sheet-status"
              aria-live={voiceConnectionState === 'failed' ? 'assertive' : 'polite'}
            >
              {voiceStatusMessage}
            </p>
          {/if}

          <div class="flex items-center justify-center gap-2">
            <button
              type="button"
              class={`inline-flex h-16 w-16 items-center justify-center rounded-full border text-2xl hover:bg-muted ${voiceState.isMuted ? 'border-amber-500/40 bg-amber-500/10 text-amber-200' : 'border-border bg-background text-foreground'}`}
              aria-label={voiceState.isMuted ? 'Unmute microphone' : 'Mute microphone'}
              data-testid="mobile-voice-sheet-mute"
              onclick={() => voiceState.toggleMute()}
            >
              {voiceState.isMuted ? '🎤✕' : '🎤'}
            </button>
            <button
              type="button"
              class={`inline-flex h-16 w-16 items-center justify-center rounded-full border text-2xl hover:bg-muted ${voiceState.isDeafened ? 'border-amber-500/40 bg-amber-500/10 text-amber-200' : 'border-border bg-background text-foreground'}`}
              aria-label={voiceState.isDeafened ? 'Undeafen audio' : 'Deafen audio'}
              data-testid="mobile-voice-sheet-deafen"
              onclick={() => voiceState.toggleDeafen()}
            >
              {voiceState.isDeafened ? '🎧✕' : '🎧'}
            </button>
            <button
              type="button"
              class="inline-flex h-16 w-16 items-center justify-center rounded-full border border-fire/40 bg-fire text-2xl text-fire-foreground hover:opacity-90"
              aria-label="Disconnect from voice channel"
              data-testid="mobile-voice-sheet-disconnect"
              onclick={() => voiceState.disconnect()}
            >
              📞
            </button>
          </div>

          <VoicePanel
            channelName={activeChannel}
            participants={activeVoiceParticipants}
            variant="mobile-sheet"
            onParticipantVolumeChange={(participantUserId, volumePercent) => {
              voiceState.setParticipantVolume(participantUserId, volumePercent)
            }}
          />
        </div>
      </div>
    {/if}
  </div>
{:else}
  <div class="relative flex min-h-screen bg-background">
    <aside class="w-[72px] shrink-0">
      <GuildRail
        activeGuild={activeGuild}
        activeChannel={activeChannel}
        activeDm={activeDm}
        mode={shellMode}
      />
    </aside>
    <aside class="w-[240px] shrink-0">
      <ChannelList activeGuild={activeGuild} activeChannel={activeChannel} />
    </aside>

    <main
      id="main-content"
      tabindex="-1"
      bind:this={mainContent}
      class="flex min-w-0 flex-1 flex-col outline-none"
    >
      <header class="flex flex-wrap items-center justify-between gap-2 border-b border-border px-4 py-3">
        <p class="text-sm text-muted-foreground">
          {#if shellMode === 'dm' && activeDm}
            DM / {activeDm}
          {:else}
            {activeGuild} / {activeChannel}
          {/if}
        </p>
        <div class="flex items-center gap-2">
          {#if viewport === 'tablet'}
            <button
              type="button"
              class="rounded-md bg-muted px-3 py-2 text-xs font-medium text-foreground"
              onclick={toggleTabletMembers}
              aria-label="Toggle members"
            >
              Members
            </button>
          {/if}
          <a
            class="rounded-md bg-muted px-3 py-2 text-xs font-medium text-foreground"
            href="/"
            use:routerAction
          >
            Home
          </a>
          {#if canAccessAdmin}
            <a
              class="rounded-md bg-muted px-3 py-2 text-xs font-medium text-foreground"
              href="/admin"
              use:routerAction
            >
              Admin
            </a>
          {/if}
          <a
            class="rounded-md bg-muted px-3 py-2 text-xs font-medium text-foreground"
            href="/settings"
            use:routerAction
          >
            Settings
          </a>
          {#if canOpenInvites}
            <button
              type="button"
              class="rounded-md bg-muted px-3 py-2 text-xs font-medium text-foreground"
              onclick={openInviteModal}
            >
              Invite people
            </button>
          {/if}
          {#if canOpenGuildSettings}
            <button
              type="button"
              class="rounded-md bg-muted px-3 py-2 text-xs font-medium text-foreground"
              onclick={openGuildSettings}
            >
              Guild settings
            </button>
          {/if}
          <button
            type="button"
            class="rounded-md bg-destructive px-3 py-2 text-xs font-medium text-destructive-foreground"
            onclick={() => void handleLogout()}
          >
            Log out
          </button>
        </div>
      </header>

      <div class="min-h-0 flex-1 overflow-auto">
        <MessageArea
          mode={shellMode}
          activeGuild={activeGuild}
          activeChannel={activeChannel}
          activeDm={activeDm}
          displayName={shellDisplayName}
          isAdmin={canAccessAdmin}
          showRecoveryNudge={shellRecoveryNudge}
          onOpenSettings={handleOpenSettings}
          onDismissRecoveryNudge={handleDismissRecoveryNudge}
        />
      </div>
    </main>

    {#if viewport === 'desktop'}
      <aside class="w-[240px] shrink-0 border-l border-border">
        <MemberList activeGuild={activeGuild} />
      </aside>
    {:else if isTabletMembersVisible}
      <aside
        class="absolute inset-y-0 right-0 z-10 w-[240px] border-l border-border bg-card shadow-2xl"
        data-testid="tablet-member-list"
      >
        <MemberList activeGuild={activeGuild} />
      </aside>
    {/if}
  </div>
{/if}

{#if quickSwitcherOpen}
  <div
    class="fixed inset-0 z-40 flex items-center justify-center bg-black/50 p-4"
    role="presentation"
    data-testid="quick-switcher-overlay"
  >
    <div
      bind:this={quickSwitcherDialog}
      class="w-full max-w-xl rounded-lg border border-border bg-card p-3 shadow-2xl"
      role="dialog"
      tabindex="-1"
      aria-modal="true"
      aria-labelledby="quick-switcher-title"
      data-testid="quick-switcher"
      onkeydown={handleQuickSwitcherKeydown}
    >
      <h2 id="quick-switcher-title" class="sr-only">Quick switcher</h2>
      <div class="mb-2 flex items-center gap-2">
        <input
          bind:this={quickSwitcherInput}
          type="text"
          class="w-full rounded-md border border-input bg-background px-3 py-2 text-sm text-foreground focus:outline-none focus:ring-2 focus:ring-ring"
          placeholder="Jump to guilds, channels, or DMs…"
          bind:value={quickSwitcherQuery}
          role="combobox"
          aria-expanded="true"
          aria-controls="quick-switcher-results"
          aria-activedescendant={
            quickSwitcherActiveIndex >= 0
              ? `quick-switcher-option-${quickSwitcherActiveIndex}`
              : undefined
          }
          data-testid="quick-switcher-input"
        />
        <button
          type="button"
          class="rounded-md bg-muted px-2 py-2 text-xs text-foreground"
          onclick={() => closeQuickSwitcher()}
        >
          Close
        </button>
      </div>
      <ul
        id="quick-switcher-results"
        class="max-h-72 overflow-y-auto"
        role="listbox"
        aria-label="Quick switcher results"
        data-testid="quick-switcher-results"
      >
        {#if quickSwitcherGroupedResults.length === 0}
          <li class="px-2 py-2 text-sm text-muted-foreground">No matches</li>
        {:else}
          {#each quickSwitcherGroupedResults as group (group.section)}
            <li
              role="presentation"
              class="px-2 pb-1 pt-2 text-[11px] font-semibold uppercase tracking-wide text-muted-foreground"
            >
              {group.section}
            </li>
            {#each group.results as result (result.id)}
              <li role="presentation">
                <button
                  id={`quick-switcher-option-${result.selectionIndex}`}
                  type="button"
                  role="option"
                  aria-selected={quickSwitcherActiveIndex === result.selectionIndex}
                  class={`flex w-full items-center justify-between rounded-md px-2 py-2 text-left text-sm text-foreground ${
                    quickSwitcherActiveIndex === result.selectionIndex
                      ? 'bg-muted'
                      : 'hover:bg-muted'
                  }`}
                  onclick={() => void handleQuickSwitcherPick(result.path)}
                  onmousemove={() => {
                    quickSwitcherActiveIndex = result.selectionIndex
                  }}
                  onfocus={() => {
                    quickSwitcherActiveIndex = result.selectionIndex
                  }}
                  data-testid={`quick-switcher-result-${result.id}`}
                >
                  <span class="min-w-0 flex-1 truncate">{result.label}</span>
                  <span class="ml-2 shrink-0 text-[11px] uppercase tracking-wide text-muted-foreground">
                    {result.section}
                  </span>
                </button>
              </li>
            {/each}
          {/each}
        {/if}
      </ul>
      {#if quickSwitcherHydrating}
        <p class="mt-2 px-2 text-xs text-muted-foreground">Loading channels…</p>
      {:else if quickSwitcherHydrationError}
        <p class="mt-2 px-2 text-xs text-destructive">
          {quickSwitcherHydrationError}
        </p>
      {/if}
    </div>
  </div>
{/if}

<GuildSettings
  open={guildSettingsOpen}
  guildSlug={activeGuild}
  onClose={handleCloseGuildSettings}
/>
<InviteModal
  open={inviteModalOpen}
  guildSlug={activeGuild}
  onClose={handleCloseInviteModal}
/>
