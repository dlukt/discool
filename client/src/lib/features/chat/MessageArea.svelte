<script lang="ts">
// biome-ignore-all lint/correctness/noUnusedVariables: Svelte template usage isn't detected reliably.
// biome-ignore-all lint/correctness/noUnusedImports: Svelte template usage isn't detected reliably.
import { onMount, tick } from 'svelte'
import AdminPanel from '$lib/components/AdminPanel.svelte'
import { guildState } from '$lib/features/guild/guildStore.svelte'
import { identityState } from '$lib/features/identity/identityStore.svelte'
import ProfileSettingsView from '$lib/features/identity/ProfileSettingsView.svelte'
import { wsClient } from '$lib/ws/client'
import type { WsLifecycleState } from '$lib/ws/protocol'
import MessageBubble from './MessageBubble.svelte'
import { type ChatAuthorInput, messageState } from './messageStore.svelte'
import type { ChatMessage } from './types'

type Props = {
  mode: 'home' | 'channel' | 'settings' | 'admin'
  activeGuild: string
  activeChannel: string
  displayName: string
  isAdmin: boolean
  showRecoveryNudge: boolean
  onOpenSettings?: () => void | Promise<void>
  onDismissRecoveryNudge?: () => void | Promise<void>
}

type VirtualTimelineRow = {
  id: string
  top: number
  height: number
  compact: boolean
  message: ChatMessage
}

type ComposerEditState = {
  messageId: string
}

const MESSAGE_ROW_HEIGHT = 74
const COMPACT_MESSAGE_ROW_HEIGHT = 46
const SYSTEM_ROW_HEIGHT = 36
const VIRTUAL_OVERSCAN_PX = 320
const HISTORY_LOAD_TRIGGER_PX = 120
const JUMP_TO_PRESENT_THRESHOLD_PX = 320
const HISTORY_SKELETON_COUNT = 4

let {
  mode,
  activeGuild,
  activeChannel,
  displayName,
  isAdmin,
  showRecoveryNudge,
  onOpenSettings,
  onDismissRecoveryNudge,
}: Props = $props()

let detailText = $derived(
  mode === 'channel'
    ? `#${activeChannel} in ${activeGuild}`
    : `Signed in as ${displayName}.`,
)
let canShowAdminPanel = $derived(mode === 'admin' && isAdmin)
let shouldShowRecoveryNudge = $derived(
  showRecoveryNudge && (mode === 'home' || mode === 'channel'),
)
let wsLifecycleState = $state<WsLifecycleState>(wsClient.getLifecycleState())
let showReconnectingBanner = $derived(wsLifecycleState === 'reconnecting')
let composerInput = $state<HTMLTextAreaElement | null>(null)
let composerValue = $state('')
let composerEdit = $state<ComposerEditState | null>(null)
let timelineViewport = $state<HTMLDivElement | null>(null)
let scrollTop = $state(0)
let viewportHeight = $state(320)
let distanceFromBottomPx = $state(0)
let restoringChannelKey = $state<string | null>(null)
let loadHistoryInFlight = $state(false)
let lastTailMessageId = $state<string | null>(null)
let skipNextTailChange = $state(false)
let pendingDeleteMessage = $state<ChatMessage | null>(null)

let channelKey = $derived(
  mode === 'channel' ? `${activeGuild}:${activeChannel}` : null,
)

let timelineMessages = $derived.by(() => {
  const _messageVersion = messageState.version
  if (mode !== 'channel') return []
  return messageState.timeline(activeGuild, activeChannel)
})

let channelHistory = $derived.by(() => {
  const _messageVersion = messageState.version
  if (mode !== 'channel') {
    return {
      initialized: false,
      loadingHistory: false,
      hasMoreHistory: false,
      cursor: null,
      scrollTop: 0,
      pendingNewCount: 0,
    }
  }
  return messageState.historyStateForChannel(activeGuild, activeChannel)
})

let virtualRows = $derived.by(() => {
  let top = 0
  const rows: VirtualTimelineRow[] = []

  for (const [index, message] of timelineMessages.entries()) {
    const previous = index > 0 ? timelineMessages[index - 1] : null
    const compact =
      Boolean(
        previous &&
          !previous.isSystem &&
          !message.isSystem &&
          previous.authorUserId === message.authorUserId &&
          message.updatedAt === message.createdAt,
      ) && !message.isSystem
    const height = message.isSystem
      ? SYSTEM_ROW_HEIGHT
      : compact
        ? COMPACT_MESSAGE_ROW_HEIGHT
        : MESSAGE_ROW_HEIGHT

    rows.push({
      id: message.id,
      top,
      height,
      compact,
      message,
    })
    top += height
  }

  return {
    rows,
    totalHeight: top,
  }
})

let visibleRows = $derived.by(() => {
  const start = Math.max(0, scrollTop - VIRTUAL_OVERSCAN_PX)
  const end = scrollTop + viewportHeight + VIRTUAL_OVERSCAN_PX
  return virtualRows.rows.filter(
    (row) => row.top + row.height >= start && row.top <= end,
  )
})

let currentSessionUser = $derived(identityState.session?.user ?? null)
let currentMember = $derived(
  currentSessionUser
    ? guildState.memberByUserId(activeGuild, currentSessionUser.id)
    : null,
)
let currentRoleColor = $derived(
  currentMember?.highestRoleColor ??
    currentSessionUser?.avatarColor ??
    '#99aab5',
)
let emptyStateCopy = $derived(
  `This is the beginning of #${activeChannel}. Say something!`,
)

let showJumpToPresent = $derived(
  mode === 'channel' &&
    (distanceFromBottomPx > JUMP_TO_PRESENT_THRESHOLD_PX ||
      channelHistory.pendingNewCount > 0),
)

async function handleOpenSettings() {
  await onOpenSettings?.()
}

async function handleDismissRecoveryNudge() {
  await onDismissRecoveryNudge?.()
}

function buildCurrentAuthor(): ChatAuthorInput | null {
  if (!currentSessionUser) return null
  return {
    userId: currentSessionUser.id,
    username: currentSessionUser.username,
    displayName: currentSessionUser.displayName,
    avatarColor: currentSessionUser.avatarColor,
    roleColor: currentRoleColor,
  }
}

function sendComposerMessage() {
  const author = buildCurrentAuthor()
  if (!author || mode !== 'channel') return

  if (composerEdit) {
    const sent = messageState.sendMessageUpdate(
      activeGuild,
      activeChannel,
      composerEdit.messageId,
      composerValue,
    )
    if (sent) {
      composerValue = ''
      composerEdit = null
    }
    return
  }

  const sent = messageState.sendMessage(
    activeGuild,
    activeChannel,
    composerValue,
    author,
  )
  if (sent) {
    composerValue = ''
  }
}

function findLatestOwnMessage(): ChatMessage | null {
  const currentUserId = currentSessionUser?.id
  if (!currentUserId || mode !== 'channel') return null
  for (let index = timelineMessages.length - 1; index >= 0; index -= 1) {
    const message = timelineMessages[index]
    if (!message || message.isSystem) continue
    if (message.authorUserId !== currentUserId) continue
    return message
  }
  return null
}

function startEditingMessage(message: ChatMessage): void {
  const currentUserId = currentSessionUser?.id
  if (!currentUserId || mode !== 'channel') return
  if (message.isSystem || message.authorUserId !== currentUserId) return
  if (
    message.guildSlug !== activeGuild ||
    message.channelSlug !== activeChannel
  )
    return
  composerEdit = { messageId: message.id }
  composerValue = message.content
  void tick().then(() => {
    if (!composerInput) return
    composerInput.focus()
    const end = composerValue.length
    composerInput.selectionStart = end
    composerInput.selectionEnd = end
  })
}

function cancelComposerEdit(): void {
  composerEdit = null
  composerValue = ''
  composerInput?.focus()
}

function requestDeleteMessage(message: ChatMessage): void {
  const currentUserId = currentSessionUser?.id
  if (!currentUserId) return
  if (message.isSystem || message.authorUserId !== currentUserId) return
  pendingDeleteMessage = message
}

function closeDeleteDialog(): void {
  pendingDeleteMessage = null
}

function confirmDeleteMessage(): void {
  if (!pendingDeleteMessage || mode !== 'channel') return
  const sent = messageState.sendMessageDelete(
    pendingDeleteMessage.guildSlug,
    pendingDeleteMessage.channelSlug,
    pendingDeleteMessage.id,
  )
  if (!sent) return
  closeDeleteDialog()
}

function handleComposerKeydown(event: KeyboardEvent) {
  if (
    event.key === 'ArrowUp' &&
    !event.altKey &&
    !event.ctrlKey &&
    !event.metaKey &&
    !event.shiftKey
  ) {
    const target = event.currentTarget as HTMLTextAreaElement | null
    const selectionStart = target?.selectionStart ?? 0
    const selectionEnd = target?.selectionEnd ?? 0
    if (
      composerValue.trim().length === 0 &&
      selectionStart === 0 &&
      selectionEnd === 0
    ) {
      const latestOwn = findLatestOwnMessage()
      if (latestOwn) {
        event.preventDefault()
        startEditingMessage(latestOwn)
      }
    }
    return
  }

  if (event.key === 'Escape' && composerEdit) {
    event.preventDefault()
    cancelComposerEdit()
    return
  }

  if (event.key !== 'Enter') return

  if (event.shiftKey) {
    event.preventDefault()
    const target = event.currentTarget as HTMLTextAreaElement | null
    const start = target?.selectionStart ?? composerValue.length
    const end = target?.selectionEnd ?? composerValue.length
    composerValue = `${composerValue.slice(0, start)}\n${composerValue.slice(end)}`
    if (target) {
      void tick().then(() => {
        target.selectionStart = start + 1
        target.selectionEnd = start + 1
      })
    }
    return
  }

  event.preventDefault()
  sendComposerMessage()
}

function updateViewportMetrics(
  target: HTMLDivElement | null,
  guildSlug = activeGuild,
  channelSlug = activeChannel,
): void {
  if (!target) {
    scrollTop = 0
    viewportHeight = 320
    distanceFromBottomPx = 0
    return
  }

  scrollTop = target.scrollTop
  viewportHeight = target.clientHeight || 320
  distanceFromBottomPx = Math.max(
    0,
    target.scrollHeight - (target.scrollTop + target.clientHeight),
  )

  if (mode === 'channel') {
    messageState.setScrollTop(guildSlug, channelSlug, target.scrollTop)
    if (distanceFromBottomPx <= 24) {
      messageState.clearPendingNew(guildSlug, channelSlug)
    }
  }
}

function isNearBottom(threshold = 24): boolean {
  return distanceFromBottomPx <= threshold
}

async function loadOlderHistoryIfNeeded(): Promise<void> {
  if (mode !== 'channel' || loadHistoryInFlight) return
  if (!channelHistory.hasMoreHistory || channelHistory.loadingHistory) return
  if (scrollTop > HISTORY_LOAD_TRIGGER_PX) return

  const viewport = timelineViewport
  if (!viewport) return
  const loadGuild = activeGuild
  const loadChannel = activeChannel
  const loadKey = `${loadGuild}:${loadChannel}`

  loadHistoryInFlight = true
  const beforeHeight = viewport.scrollHeight
  const beforeTop = viewport.scrollTop

  try {
    await messageState.loadOlderHistory(loadGuild, loadChannel)
  } finally {
    await tick()
    const nextViewport = timelineViewport
    if (nextViewport && `${activeGuild}:${activeChannel}` === loadKey) {
      const delta = Math.max(0, nextViewport.scrollHeight - beforeHeight)
      nextViewport.scrollTop = beforeTop + delta
      updateViewportMetrics(nextViewport, loadGuild, loadChannel)
    }
    loadHistoryInFlight = false
  }
}

function handleTimelineScroll(event: Event): void {
  const target = event.currentTarget as HTMLDivElement | null
  if (!target) return
  updateViewportMetrics(target, activeGuild, activeChannel)
  if (target.scrollTop <= HISTORY_LOAD_TRIGGER_PX) {
    void loadOlderHistoryIfNeeded()
  }
}

function jumpToPresent(): void {
  const target = timelineViewport
  if (!target) return
  target.scrollTop = target.scrollHeight
  updateViewportMetrics(target, activeGuild, activeChannel)
  if (mode === 'channel') {
    messageState.clearPendingNew(activeGuild, activeChannel)
  }
}

$effect(() => {
  if (mode !== 'channel') return
  activeGuild
  activeChannel
  void tick().then(() => {
    composerInput?.focus()
  })
})

$effect(() => {
  if (mode !== 'channel') {
    composerEdit = null
    pendingDeleteMessage = null
    return
  }
  activeGuild
  activeChannel
  if (
    pendingDeleteMessage &&
    (pendingDeleteMessage.guildSlug !== activeGuild ||
      pendingDeleteMessage.channelSlug !== activeChannel)
  ) {
    pendingDeleteMessage = null
  }
  if (!composerEdit) return
  const exists = timelineMessages.some(
    (message) => message.id === composerEdit?.messageId,
  )
  if (!exists) {
    composerEdit = null
  }
})

$effect(() => {
  viewportHeight = timelineViewport?.clientHeight || 320
})

$effect(() => {
  if (mode !== 'channel') {
    restoringChannelKey = null
    lastTailMessageId = null
    skipNextTailChange = false
    return
  }

  const currentKey = `${activeGuild}:${activeChannel}`
  const restoreGuild = activeGuild
  const restoreChannel = activeChannel
  const currentHistory = messageState.historyStateForChannel(
    activeGuild,
    activeChannel,
  )
  messageState.setActiveChannel(activeGuild, activeChannel)
  void messageState
    .ensureHistoryLoaded(activeGuild, activeChannel)
    .catch(() => {})

  if (restoringChannelKey === currentKey) return

  restoringChannelKey = currentKey
  skipNextTailChange = !currentHistory.initialized
  lastTailMessageId = timelineMessages.at(-1)?.id ?? null

  void tick().then(() => {
    if (
      mode !== 'channel' ||
      `${activeGuild}:${activeChannel}` !== currentKey
    ) {
      return
    }
    const viewport = timelineViewport
    if (!viewport) return

    const savedTop = messageState.scrollTopForChannel(
      restoreGuild,
      restoreChannel,
    )
    if (savedTop > 0) {
      viewport.scrollTop = savedTop
    } else {
      viewport.scrollTop = viewport.scrollHeight
      messageState.clearPendingNew(restoreGuild, restoreChannel)
    }
    updateViewportMetrics(viewport, restoreGuild, restoreChannel)

    if (viewport.scrollTop <= HISTORY_LOAD_TRIGGER_PX) {
      void loadOlderHistoryIfNeeded()
    }
  })
})

$effect(() => {
  if (mode !== 'channel') return

  const _messageVersion = messageState.version
  const currentTail = timelineMessages.at(-1)?.id ?? null
  if (!currentTail || currentTail === lastTailMessageId) return

  if (skipNextTailChange) {
    skipNextTailChange = false
    lastTailMessageId = currentTail
    return
  }

  lastTailMessageId = currentTail

  if (isNearBottom(JUMP_TO_PRESENT_THRESHOLD_PX)) {
    const tailGuild = activeGuild
    const tailChannel = activeChannel
    const tailKey = `${tailGuild}:${tailChannel}`
    void tick().then(() => {
      if (mode !== 'channel' || `${activeGuild}:${activeChannel}` !== tailKey) {
        return
      }
      const viewport = timelineViewport
      if (!viewport) return
      viewport.scrollTop = viewport.scrollHeight
      updateViewportMetrics(viewport, tailGuild, tailChannel)
      messageState.clearPendingNew(tailGuild, tailChannel)
    })
    return
  }

  messageState.addPendingNew(activeGuild, activeChannel, 1)
})

onMount(() => {
  return wsClient.subscribeLifecycle((state) => {
    wsLifecycleState = state
  })
})
</script>

{#if mode === 'admin'}
  {#if canShowAdminPanel}
    <AdminPanel />
  {:else}
    <section class="p-6">
      <p class="rounded-md border border-destructive/30 bg-destructive/10 p-3 text-sm text-destructive">
        Admin access is only available to the instance administrator.
      </p>
    </section>
  {/if}
{:else if mode === 'settings'}
  <ProfileSettingsView />
{:else}
  <section class="flex h-full flex-col gap-4 p-4 md:p-6">
    <header class="space-y-1">
      <h1 class="text-2xl font-semibold tracking-tight">Messages</h1>
      <p class="text-sm text-muted-foreground">{detailText}</p>
    </header>

    {#if showReconnectingBanner}
      <p
        class="rounded-md border border-amber-500/30 bg-amber-500/10 px-3 py-2 text-sm text-amber-200"
        data-testid="reconnecting-status"
      >
        Reconnecting...
      </p>
    {/if}

    {#if shouldShowRecoveryNudge}
      <section class="rounded-md border border-border bg-muted p-4">
        <p class="text-sm font-medium text-foreground">
          Add a recovery email to protect this identity.
        </p>
        <p class="mt-1 text-sm text-muted-foreground">
          Optional, and only shown after your first successful session.
        </p>
        <div class="mt-3 flex flex-wrap gap-2">
          <button
            type="button"
            class="inline-flex items-center justify-center rounded-md bg-fire px-3 py-2 text-sm font-medium text-fire-foreground transition-opacity hover:opacity-90"
            onclick={() => void handleOpenSettings()}
          >
            Set up recovery email
          </button>
          <button
            type="button"
            class="inline-flex items-center justify-center rounded-md bg-background px-3 py-2 text-sm font-medium text-foreground transition-opacity hover:opacity-90"
            onclick={() => void handleDismissRecoveryNudge()}
          >
            Not now
          </button>
        </div>
      </section>
    {/if}

    <section class="min-h-0 flex-1 rounded-md border border-border bg-card p-4">
      <h2 class="text-sm font-medium text-foreground">Channel Timeline</h2>
      <div class="mt-3 min-h-0 flex-1" data-testid="channel-timeline">
        <div
          class="h-full overflow-y-auto rounded-md border border-border/40 bg-background/20 px-2 py-2"
          bind:this={timelineViewport}
          onscroll={handleTimelineScroll}
          role="log"
          aria-live="polite"
          aria-relevant="additions text"
          data-testid="channel-timeline-scroll"
        >
          {#if channelHistory.loadingHistory}
            <div class="space-y-2 pb-2" data-testid="history-loading-skeletons">
              {#each Array.from({ length: HISTORY_SKELETON_COUNT }) as _, index (`skeleton-${index}`)}
                <div class="flex animate-pulse gap-3 rounded-md px-2 py-1">
                  <span class="h-8 w-8 shrink-0 rounded-full bg-muted"></span>
                  <span class="min-w-0 flex-1 space-y-1 pt-1">
                    <span class="block h-3 w-24 rounded bg-muted"></span>
                    <span class="block h-3 w-4/5 rounded bg-muted"></span>
                  </span>
                </div>
              {/each}
            </div>
          {/if}

          {#if timelineMessages.length === 0 && !channelHistory.loadingHistory}
            <p
              class="rounded-md bg-muted px-3 py-2 text-sm text-muted-foreground"
              data-testid="message-empty-state"
            >
              {emptyStateCopy}
            </p>
          {:else if timelineMessages.length > 0}
            <div class="relative" style={`height: ${virtualRows.totalHeight}px;`}>
              {#each visibleRows as row (row.id)}
                <div
                  class="absolute inset-x-0"
                  style={`top: ${row.top}px; height: ${row.height}px;`}
                  data-testid={`message-window-row-${row.message.id}`}
                >
                  <MessageBubble
                    message={row.message}
                    compact={row.compact}
                    currentUserId={currentSessionUser?.id ?? null}
                    onEditRequest={startEditingMessage}
                    onDeleteRequest={requestDeleteMessage}
                  />
                </div>
              {/each}
            </div>
          {/if}
        </div>
      </div>

      {#if mode === 'channel' && showJumpToPresent}
        <div class="mt-2 flex justify-end">
          <button
            type="button"
            class="rounded-md border border-border bg-background px-3 py-1.5 text-xs font-medium text-foreground hover:bg-muted"
            onclick={jumpToPresent}
            data-testid="jump-to-present"
          >
            {#if channelHistory.pendingNewCount > 0}
              Jump to present ({channelHistory.pendingNewCount} new)
            {:else}
              Jump to present
            {/if}
          </button>
        </div>
      {/if}
    </section>

    {#if mode === 'channel'}
      <section class="rounded-md border border-border bg-card p-4">
        <label
          for="message-composer"
          class="mb-2 block text-sm font-medium text-foreground"
        >
          Message
        </label>
        <div class="flex items-end gap-2">
          <textarea
            id="message-composer"
            data-testid="message-composer-input"
            class="min-h-[44px] w-full resize-y rounded-md border border-input bg-background px-3 py-2 text-sm text-foreground placeholder:text-muted-foreground focus:outline-none focus:ring-2 focus:ring-ring"
            placeholder={`Message #${activeChannel}`}
            bind:this={composerInput}
            bind:value={composerValue}
            onkeydown={handleComposerKeydown}
          ></textarea>
          <button
            type="button"
            class="inline-flex h-[44px] items-center justify-center rounded-md bg-fire px-4 text-sm font-medium text-fire-foreground transition-opacity hover:opacity-90 disabled:cursor-not-allowed disabled:opacity-50"
            onclick={sendComposerMessage}
            disabled={!currentSessionUser || composerValue.trim().length === 0}
            data-testid="message-composer-submit"
          >
            {composerEdit ? 'Save' : 'Send'}
          </button>
        </div>
        <p class="mt-2 text-xs text-muted-foreground">
          {#if composerEdit}
            Editing message · Enter to save · Escape to cancel
          {:else}
            Enter to send · Shift+Enter for newline · Up to edit latest own message
          {/if}
        </p>
      </section>
    {/if}
  </section>
{/if}

{#if pendingDeleteMessage}
  <div class="fixed inset-0 z-30 flex items-center justify-center bg-black/60 p-4">
    <div
      class="w-full max-w-md rounded-md border border-border bg-card p-4 shadow-lg"
      role="dialog"
      aria-modal="true"
      aria-label="Delete message"
    >
      <h3 class="text-base font-semibold text-foreground">Delete message</h3>
      <p class="mt-2 text-sm text-muted-foreground">
        This message will be removed for everyone in this channel.
      </p>
      <div class="mt-4 flex justify-end gap-2">
        <button
          type="button"
          class="rounded-md border border-border bg-background px-3 py-2 text-sm font-medium text-foreground hover:bg-muted"
          onclick={closeDeleteDialog}
        >
          Cancel
        </button>
        <button
          type="button"
          class="rounded-md bg-destructive px-3 py-2 text-sm font-medium text-destructive-foreground hover:opacity-90"
          onclick={confirmDeleteMessage}
        >
          Delete message
        </button>
      </div>
    </div>
  </div>
{/if}
