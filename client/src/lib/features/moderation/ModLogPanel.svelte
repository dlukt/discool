<script lang="ts">
// biome-ignore-all lint/correctness/noUnusedVariables: Svelte template usage isn't detected reliably.
// biome-ignore-all lint/correctness/noUnusedImports: Svelte template usage isn't detected reliably.
import { ApiError } from '$lib/api'
import ModLogEntry from './ModLogEntry.svelte'
import {
  fetchModerationLog,
  type ModerationLogActionType,
  type ModerationLogEntry,
  type ModerationLogOrder,
} from './moderationApi'

type Props = {
  activeGuild: string
}

type ActionFilterValue = 'all' | ModerationLogActionType

const LOG_ROW_HEIGHT = 132
const VIRTUAL_OVERSCAN_PX = 264
const LOAD_MORE_THRESHOLD_PX = 160

const ACTION_FILTER_OPTIONS: Array<{
  value: ActionFilterValue
  label: string
}> = [
  { value: 'all', label: 'All actions' },
  { value: 'mute', label: 'Mute' },
  { value: 'kick', label: 'Kick' },
  { value: 'ban', label: 'Ban' },
  { value: 'voice_kick', label: 'Voice kick' },
  { value: 'message_delete', label: 'Message delete' },
  { value: 'warn', label: 'Warn' },
]

let { activeGuild }: Props = $props()

let entries = $state<ModerationLogEntry[]>([])
let cursor = $state<string | null>(null)
let loading = $state(false)
let loadingMore = $state(false)
let errorMessage = $state<string | null>(null)
let order = $state<ModerationLogOrder>('desc')
let actionFilter = $state<ActionFilterValue>('all')
let scrollTop = $state(0)
let viewportHeight = $state(320)
let listViewport = $state<HTMLDivElement | null>(null)
let requestId = 0

let totalHeight = $derived(entries.length * LOG_ROW_HEIGHT)
let visibleRows = $derived.by(() => {
  const start = Math.max(0, scrollTop - VIRTUAL_OVERSCAN_PX)
  const end = scrollTop + viewportHeight + VIRTUAL_OVERSCAN_PX
  return entries
    .map((entry, index) => ({
      entry,
      top: index * LOG_ROW_HEIGHT,
    }))
    .filter((row) => row.top + LOG_ROW_HEIGHT >= start && row.top <= end)
})
let hasMore = $derived(Boolean(cursor))

function messageFromError(err: unknown, fallback: string): string {
  if (err instanceof ApiError) return err.message
  if (err instanceof Error) return err.message
  return fallback
}

function selectedActionType(): ModerationLogActionType | null {
  return actionFilter === 'all' ? null : actionFilter
}

function mergeEntries(
  previous: ModerationLogEntry[],
  next: ModerationLogEntry[],
): ModerationLogEntry[] {
  const byId = new Map<string, ModerationLogEntry>()
  for (const item of previous) byId.set(item.id, item)
  for (const item of next) byId.set(item.id, item)
  return [...byId.values()]
}

async function loadModerationLog(reset: boolean): Promise<void> {
  if (!activeGuild) return
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
    const page = await fetchModerationLog(activeGuild, {
      limit: 50,
      cursor: reset ? null : cursor,
      order,
      actionType: selectedActionType(),
    })
    if (requestId !== nextRequestId) return
    entries = reset ? page.entries : mergeEntries(entries, page.entries)
    cursor = page.cursor
  } catch (err) {
    if (requestId !== nextRequestId) return
    errorMessage = messageFromError(err, 'Failed to load moderation log.')
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
    void loadModerationLog(false)
  }
}

$effect(() => {
  if (!activeGuild) return
  void order
  void actionFilter
  void loadModerationLog(true)
})

$effect(() => {
  viewportHeight = listViewport?.clientHeight || 320
})
</script>

<section class="flex h-full min-h-0 flex-col bg-card p-4" data-testid="mod-log-panel">
  <div class="mb-3 flex items-center justify-between gap-2">
    <h3 class="text-sm font-semibold text-foreground">Moderation log</h3>
    <span class="text-xs text-muted-foreground" data-testid="mod-log-entry-count">
      {entries.length}
    </span>
  </div>

  <div class="mb-3 grid grid-cols-2 gap-2">
    <label class="space-y-1 text-xs text-muted-foreground">
      <span>Action</span>
      <select
        class="w-full rounded-md border border-input bg-background px-2 py-1.5 text-xs text-foreground focus:outline-none focus:ring-2 focus:ring-ring"
        bind:value={actionFilter}
        data-testid="mod-log-filter-select"
      >
        {#each ACTION_FILTER_OPTIONS as option (option.value)}
          <option value={option.value}>{option.label}</option>
        {/each}
      </select>
    </label>
    <label class="space-y-1 text-xs text-muted-foreground">
      <span>Sort</span>
      <select
        class="w-full rounded-md border border-input bg-background px-2 py-1.5 text-xs text-foreground focus:outline-none focus:ring-2 focus:ring-ring"
        bind:value={order}
        data-testid="mod-log-order-select"
      >
        <option value="desc">Newest first</option>
        <option value="asc">Oldest first</option>
      </select>
    </label>
  </div>

  {#if errorMessage}
    <p
      class="mb-3 rounded-md border border-destructive/30 bg-destructive/10 p-2 text-xs text-destructive"
      role="alert"
      data-testid="mod-log-error"
    >
      {errorMessage}
    </p>
  {/if}

  {#if loading && entries.length === 0}
    <p class="text-xs text-muted-foreground" data-testid="mod-log-loading">
      Loading moderation log…
    </p>
  {:else if entries.length === 0}
    <p class="text-xs text-muted-foreground" data-testid="mod-log-empty">
      No moderation actions yet. That's a good thing.
    </p>
  {:else}
    <div
      class="relative min-h-0 flex-1 overflow-y-auto"
      bind:this={listViewport}
      onscroll={handleScroll}
      data-testid="mod-log-scroll"
    >
      <div class="relative" style={`height: ${totalHeight}px;`}>
        {#each visibleRows as row (row.entry.id)}
          <div
            class="absolute inset-x-0 px-0.5"
            style={`top: ${row.top}px; height: ${LOG_ROW_HEIGHT}px;`}
          >
            <ModLogEntry entry={row.entry} />
          </div>
        {/each}
      </div>
      {#if loadingMore}
        <p class="py-2 text-center text-xs text-muted-foreground" data-testid="mod-log-loading-more">
          Loading more…
        </p>
      {/if}
    </div>
  {/if}
</section>
