<script lang="ts">
// biome-ignore-all lint/correctness/noUnusedVariables: Svelte template usage isn't detected reliably.
// biome-ignore-all lint/correctness/noUnusedImports: Svelte template usage isn't detected reliably.
import { ApiError } from '$lib/api'
import {
  fetchUserMessageHistory,
  type UserMessageHistoryEntry,
} from './moderationApi'

type Props = {
  activeGuild: string
  targetUserId: string
  targetDisplayName: string
  onClose?: () => void
}

const HISTORY_ROW_HEIGHT = 88
const VIRTUAL_OVERSCAN_PX = 176
const LOAD_MORE_THRESHOLD_PX = 120

let { activeGuild, targetUserId, targetDisplayName, onClose }: Props = $props()

let entries = $state<UserMessageHistoryEntry[]>([])
let cursor = $state<string | null>(null)
let loading = $state(false)
let loadingMore = $state(false)
let errorMessage = $state<string | null>(null)
let channelSlugFilter = $state('')
let fromFilter = $state('')
let toFilter = $state('')
let scrollTop = $state(0)
let viewportHeight = $state(320)
let listViewport = $state<HTMLDivElement | null>(null)
let requestId = 0

let totalHeight = $derived(entries.length * HISTORY_ROW_HEIGHT)
let hasMore = $derived(Boolean(cursor))
let visibleRows = $derived.by(() => {
  const start = Math.max(0, scrollTop - VIRTUAL_OVERSCAN_PX)
  const end = scrollTop + viewportHeight + VIRTUAL_OVERSCAN_PX
  return entries
    .map((entry, index) => ({
      entry,
      top: index * HISTORY_ROW_HEIGHT,
    }))
    .filter((row) => row.top + HISTORY_ROW_HEIGHT >= start && row.top <= end)
})

function messageFromError(err: unknown, fallback: string): string {
  if (err instanceof ApiError) return err.message
  if (err instanceof Error) return err.message
  return fallback
}

function mergeEntries(
  previous: UserMessageHistoryEntry[],
  next: UserMessageHistoryEntry[],
): UserMessageHistoryEntry[] {
  const byId = new Map<string, UserMessageHistoryEntry>()
  for (const item of previous) byId.set(item.id, item)
  for (const item of next) byId.set(item.id, item)
  return [...byId.values()]
}

function formatTimestamp(value: string): string {
  const parsed = new Date(value)
  if (Number.isNaN(parsed.getTime())) return value
  return parsed.toLocaleString()
}

async function loadHistory(reset: boolean): Promise<void> {
  if (!activeGuild || !targetUserId) return
  const nextRequestId = requestId + 1
  requestId = nextRequestId

  if (reset) {
    loading = true
    errorMessage = null
    scrollTop = 0
  } else {
    if (!cursor) return
    loadingMore = true
  }

  try {
    const page = await fetchUserMessageHistory(activeGuild, targetUserId, {
      limit: 50,
      cursor: reset ? null : cursor,
      channelSlug: channelSlugFilter || null,
      from: fromFilter || null,
      to: toFilter || null,
    })
    if (requestId !== nextRequestId) return
    entries = reset ? page.entries : mergeEntries(entries, page.entries)
    cursor = page.cursor
  } catch (err) {
    if (requestId !== nextRequestId) return
    errorMessage = messageFromError(err, 'Failed to load user message history.')
    if (reset) {
      entries = []
      cursor = null
    }
  } finally {
    if (requestId === nextRequestId) {
      loading = false
      loadingMore = false
    }
  }
}

function handleScroll(event: Event): void {
  const target = event.currentTarget as HTMLDivElement | null
  if (!target) return
  scrollTop = target.scrollTop
  viewportHeight = target.clientHeight || 320
  if (
    !loading &&
    !loadingMore &&
    hasMore &&
    target.scrollTop + target.clientHeight >=
      target.scrollHeight - LOAD_MORE_THRESHOLD_PX
  ) {
    void loadHistory(false)
  }
}

function openMessage(entry: UserMessageHistoryEntry): void {
  if (typeof window === 'undefined') return
  window.dispatchEvent(
    new CustomEvent('discool:open-message-history-intent', {
      detail: {
        guildSlug: activeGuild,
        channelSlug: entry.channelSlug,
        messageId: entry.id,
      },
    }),
  )
  onClose?.()
}

$effect(() => {
  if (!activeGuild || !targetUserId) return
  void channelSlugFilter
  void fromFilter
  void toFilter
  void loadHistory(true)
})

$effect(() => {
  viewportHeight = listViewport?.clientHeight || 320
})
</script>

<section
  class="fixed inset-0 z-40 flex items-center justify-center bg-black/60 p-4"
  role="presentation"
  data-testid="user-message-history-overlay"
>
  <div
    class="flex h-[min(80vh,720px)] w-full max-w-2xl min-h-0 flex-col rounded-md border border-border bg-card p-4 shadow-xl"
    role="dialog"
    aria-modal="true"
    aria-label={`Message history for ${targetDisplayName}`}
    data-testid="user-message-history-panel"
  >
    <header class="mb-3 flex items-center justify-between gap-2">
      <h3 class="text-sm font-semibold text-foreground">
        Message history · {targetDisplayName}
      </h3>
      <button
        type="button"
        class="rounded-md bg-muted px-2 py-1 text-xs hover:opacity-90"
        onclick={() => onClose?.()}
        data-testid="user-message-history-close"
      >
        Close
      </button>
    </header>

    <div class="mb-3 grid grid-cols-1 gap-2 md:grid-cols-3">
      <label class="space-y-1 text-xs text-muted-foreground">
        <span>Channel slug</span>
        <input
          class="w-full rounded-md border border-input bg-background px-2 py-1.5 text-xs text-foreground focus:outline-none focus:ring-2 focus:ring-ring"
          type="text"
          bind:value={channelSlugFilter}
          placeholder="general"
          data-testid="user-message-history-filter-channel"
        />
      </label>
      <label class="space-y-1 text-xs text-muted-foreground">
        <span>From (RFC3339)</span>
        <input
          class="w-full rounded-md border border-input bg-background px-2 py-1.5 text-xs text-foreground focus:outline-none focus:ring-2 focus:ring-ring"
          type="text"
          bind:value={fromFilter}
          placeholder="2026-03-01T00:00:00Z"
          data-testid="user-message-history-filter-from"
        />
      </label>
      <label class="space-y-1 text-xs text-muted-foreground">
        <span>To (RFC3339)</span>
        <input
          class="w-full rounded-md border border-input bg-background px-2 py-1.5 text-xs text-foreground focus:outline-none focus:ring-2 focus:ring-ring"
          type="text"
          bind:value={toFilter}
          placeholder="2026-03-02T00:00:00Z"
          data-testid="user-message-history-filter-to"
        />
      </label>
    </div>

    {#if errorMessage}
      <p
        class="mb-3 rounded-md border border-destructive/30 bg-destructive/10 p-2 text-xs text-destructive"
        role="alert"
        data-testid="user-message-history-error"
      >
        {errorMessage}
      </p>
    {/if}

    {#if loading && entries.length === 0}
      <p class="text-xs text-muted-foreground" data-testid="user-message-history-loading">
        Loading user message history…
      </p>
    {:else if entries.length === 0}
      <p class="text-xs text-muted-foreground" data-testid="user-message-history-empty">
        No messages found for this user.
      </p>
    {:else}
      <div
        class="relative min-h-0 flex-1 overflow-y-auto"
        bind:this={listViewport}
        onscroll={handleScroll}
        data-testid="user-message-history-scroll"
      >
        <div class="relative" style={`height: ${totalHeight}px;`}>
          {#each visibleRows as row (row.entry.id)}
            <div
              class="absolute inset-x-0 px-0.5"
              style={`top: ${row.top}px; height: ${HISTORY_ROW_HEIGHT}px;`}
            >
              <button
                type="button"
                class="flex h-full w-full flex-col items-start gap-1 rounded-md border border-border/60 bg-background/30 px-3 py-2 text-left hover:bg-muted/60"
                onclick={() => openMessage(row.entry)}
                data-testid={`user-message-history-row-${row.entry.id}`}
              >
                <span class="text-xs font-medium text-foreground">
                  #{row.entry.channelName}
                  <span class="text-muted-foreground">({row.entry.channelSlug})</span>
                </span>
                <span class="line-clamp-2 text-xs text-foreground">
                  {row.entry.content || '(no content)'}
                </span>
                <span class="text-[11px] text-muted-foreground">
                  {formatTimestamp(row.entry.createdAt)}
                </span>
              </button>
            </div>
          {/each}
        </div>
        {#if loadingMore}
          <p
            class="py-2 text-center text-xs text-muted-foreground"
            data-testid="user-message-history-loading-more"
          >
            Loading more…
          </p>
        {/if}
      </div>
    {/if}
  </div>
</section>
