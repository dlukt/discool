<script lang="ts">
// biome-ignore-all lint/correctness/noUnusedVariables: Svelte template usage isn't detected reliably.
// biome-ignore-all lint/correctness/noUnusedImports: Svelte template usage isn't detected reliably.
import { ApiError } from '$lib/api'
import {
  type ActOnReportInput,
  actOnReport,
  dismissReport,
  fetchReportQueue,
  type ReportQueueActionType,
  type ReportQueueItem as ReportQueueItemModel,
  type ReportQueueStatus,
  reviewReport,
} from './moderationApi'
import ReportQueueItem from './ReportQueueItem.svelte'

type Props = {
  activeGuild: string
}

type ReportFilterValue = ReportQueueStatus | 'all'
type QueueAction = 'review' | 'dismiss' | ReportQueueActionType

const FILTER_OPTIONS: Array<{ value: ReportFilterValue; label: string }> = [
  { value: 'pending', label: 'Pending' },
  { value: 'reviewed', label: 'Reviewed' },
  { value: 'actioned', label: 'Actioned' },
  { value: 'dismissed', label: 'Dismissed' },
  { value: 'all', label: 'All statuses' },
]

let { activeGuild }: Props = $props()

let entries = $state<ReportQueueItemModel[]>([])
let cursor = $state<string | null>(null)
let loading = $state(false)
let loadingMore = $state(false)
let errorMessage = $state<string | null>(null)
let statusMessage = $state<string | null>(null)
let statusFilter = $state<ReportFilterValue>('pending')
let pendingActions = $state<Record<string, QueueAction | null>>({})
let requestId = 0

let hasMore = $derived(Boolean(cursor))

function messageFromError(err: unknown, fallback: string): string {
  if (err instanceof ApiError) return err.message
  if (err instanceof Error) return err.message
  return fallback
}

function selectedStatus(): ReportQueueStatus | null {
  return statusFilter === 'all' ? null : statusFilter
}

function mergeEntries(
  previous: ReportQueueItemModel[],
  next: ReportQueueItemModel[],
): ReportQueueItemModel[] {
  const byId = new Map<string, ReportQueueItemModel>()
  for (const item of previous) byId.set(item.id, item)
  for (const item of next) byId.set(item.id, item)
  return [...byId.values()]
}

function upsertEntry(entry: ReportQueueItemModel): void {
  const filterStatus = selectedStatus()
  if (filterStatus !== null && entry.status !== filterStatus) {
    entries = entries.filter((item) => item.id !== entry.id)
    return
  }

  let replaced = false
  entries = entries.map((item) => {
    if (item.id !== entry.id) return item
    replaced = true
    return entry
  })
  if (!replaced) {
    entries = [entry, ...entries]
  }
}

function setPending(reportId: string, action: QueueAction): void {
  pendingActions = {
    ...pendingActions,
    [reportId]: action,
  }
}

function clearPending(reportId: string): void {
  pendingActions = {
    ...pendingActions,
    [reportId]: null,
  }
}

async function loadQueue(reset: boolean): Promise<void> {
  if (!activeGuild) return
  const nextRequestId = requestId + 1
  requestId = nextRequestId

  if (reset) {
    loading = true
    errorMessage = null
    statusMessage = null
    pendingActions = {}
  } else {
    if (!cursor) return
    loadingMore = true
  }

  try {
    const page = await fetchReportQueue(activeGuild, {
      limit: 50,
      cursor: reset ? null : cursor,
      status: selectedStatus(),
    })
    if (requestId !== nextRequestId) return
    entries = reset ? page.entries : mergeEntries(entries, page.entries)
    cursor = page.cursor
  } catch (err) {
    if (requestId !== nextRequestId) return
    errorMessage = messageFromError(err, 'Failed to load report queue.')
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

async function handleReview(reportId: string): Promise<void> {
  if (!activeGuild) return
  setPending(reportId, 'review')
  errorMessage = null
  statusMessage = null
  try {
    const updated = await reviewReport(activeGuild, reportId)
    upsertEntry(updated)
    statusMessage = 'Report reviewed.'
  } catch (err) {
    errorMessage = messageFromError(err, 'Failed to review report.')
  } finally {
    clearPending(reportId)
  }
}

async function handleDismiss(reportId: string): Promise<void> {
  if (!activeGuild) return
  setPending(reportId, 'dismiss')
  errorMessage = null
  statusMessage = null

  let dismissalReason: string | null = null
  if (typeof window !== 'undefined') {
    const value = window.prompt('Optional dismissal reason', '')
    if (value !== null) {
      dismissalReason = value
    }
  }

  try {
    const updated = await dismissReport(activeGuild, reportId, {
      dismissalReason,
    })
    upsertEntry(updated)
    statusMessage = 'Report dismissed.'
  } catch (err) {
    errorMessage = messageFromError(err, 'Failed to dismiss report.')
  } finally {
    clearPending(reportId)
  }
}

function defaultActionInput(action: ReportQueueActionType): ActOnReportInput {
  if (action === 'mute') {
    return {
      actionType: 'mute',
      durationSeconds: 24 * 60 * 60,
    }
  }
  if (action === 'ban') {
    return {
      actionType: 'ban',
      deleteMessageWindow: 'none',
    }
  }
  return { actionType: action }
}

async function handleAction(
  reportId: string,
  action: ReportQueueActionType,
): Promise<void> {
  if (!activeGuild) return
  setPending(reportId, action)
  errorMessage = null
  statusMessage = null
  try {
    const updated = await actOnReport(
      activeGuild,
      reportId,
      defaultActionInput(action),
    )
    upsertEntry(updated)
    statusMessage = `Report ${action} action applied.`
  } catch (err) {
    errorMessage = messageFromError(err, `Failed to ${action} from report.`)
  } finally {
    clearPending(reportId)
  }
}

$effect(() => {
  if (!activeGuild) return
  void statusFilter
  void loadQueue(true)
})
</script>

<section class="flex h-full min-h-0 flex-col bg-card p-4" data-testid="report-queue-panel">
  <div class="mb-3 flex items-center justify-between gap-2">
    <h3 class="text-sm font-semibold text-foreground">Report queue</h3>
    <span class="text-xs text-muted-foreground" data-testid="report-queue-entry-count">
      {entries.length}
    </span>
  </div>

  <div class="mb-3">
    <label class="space-y-1 text-xs text-muted-foreground">
      <span>Status</span>
      <select
        class="w-full rounded-md border border-input bg-background px-2 py-1.5 text-xs text-foreground focus:outline-none focus:ring-2 focus:ring-ring"
        bind:value={statusFilter}
        data-testid="report-queue-filter-select"
      >
        {#each FILTER_OPTIONS as option (option.value)}
          <option value={option.value}>{option.label}</option>
        {/each}
      </select>
    </label>
  </div>

  {#if statusMessage}
    <p class="mb-3 rounded-md border border-emerald-500/30 bg-emerald-500/10 p-2 text-xs text-emerald-300">
      {statusMessage}
    </p>
  {/if}

  {#if errorMessage}
    <p
      class="mb-3 rounded-md border border-destructive/30 bg-destructive/10 p-2 text-xs text-destructive"
      role="alert"
      data-testid="report-queue-error"
    >
      {errorMessage}
    </p>
  {/if}

  {#if loading && entries.length === 0}
    <p class="text-xs text-muted-foreground" data-testid="report-queue-loading">
      Loading report queue...
    </p>
  {:else if entries.length === 0}
    <p class="text-xs text-muted-foreground" data-testid="report-queue-empty">
      {statusFilter === 'pending' ? 'No pending reports.' : 'No reports for this status.'}
    </p>
  {:else}
    <div class="min-h-0 flex-1 space-y-2 overflow-y-auto" data-testid="report-queue-list">
      {#each entries as entry (entry.id)}
        <ReportQueueItem
          entry={entry}
          pendingAction={pendingActions[entry.id] ?? null}
          onReview={handleReview}
          onDismiss={handleDismiss}
          onAct={handleAction}
        />
      {/each}
      {#if hasMore}
        <button
          type="button"
          class="w-full rounded-md border border-border px-3 py-2 text-xs text-muted-foreground hover:bg-muted"
          onclick={() => void loadQueue(false)}
          disabled={loadingMore}
          data-testid="report-queue-load-more"
        >
          {loadingMore ? 'Loading more...' : 'Load more'}
        </button>
      {/if}
    </div>
  {/if}
</section>
