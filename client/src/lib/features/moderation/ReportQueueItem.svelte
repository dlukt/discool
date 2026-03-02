<script lang="ts">
// biome-ignore-all lint/correctness/noUnusedVariables: Svelte template usage isn't detected reliably.
import type {
  ReportQueueActionType,
  ReportQueueItem,
  ReportQueueStatus,
} from './moderationApi'

type QueueAction = 'review' | 'dismiss' | ReportQueueActionType

type Props = {
  entry: ReportQueueItem
  pendingAction: QueueAction | null
  onReview?: (reportId: string) => void
  onDismiss?: (reportId: string) => void
  onAct?: (reportId: string, action: ReportQueueActionType) => void
}

let { entry, pendingAction, onReview, onDismiss, onAct }: Props = $props()

function statusLabel(status: ReportQueueStatus): string {
  return status.charAt(0).toUpperCase() + status.slice(1)
}

function statusBadgeClass(status: ReportQueueStatus): string {
  if (status === 'pending') {
    return 'border-amber-300/40 bg-amber-500/20 text-amber-100'
  }
  if (status === 'reviewed') {
    return 'border-sky-300/30 bg-sky-500/15 text-sky-100'
  }
  if (status === 'actioned') {
    return 'border-emerald-300/30 bg-emerald-500/15 text-emerald-100'
  }
  return 'border-border bg-muted text-muted-foreground'
}

function cardClass(status: ReportQueueStatus): string {
  if (status === 'pending') {
    return 'border-amber-400/40 bg-amber-500/5'
  }
  return 'border-border bg-card/60'
}

function formatTimestamp(value: string): string {
  const timestamp = Date.parse(value)
  if (Number.isNaN(timestamp)) {
    return value
  }
  return new Date(timestamp).toLocaleString()
}

function targetSummary(entry: ReportQueueItem): string {
  if (entry.targetType === 'message') {
    return entry.targetMessagePreview ?? 'Message preview unavailable.'
  }
  return `Target: ${
    entry.targetDisplayName ??
    entry.targetUsername ??
    entry.targetUserId ??
    'Unknown user'
  }`
}

function isActionDisabled(action: QueueAction): boolean {
  return pendingAction !== null && pendingAction !== action
}
</script>

<article
  class={`rounded-md border p-3 ${cardClass(entry.status)}`}
  data-testid={`report-queue-item-${entry.id}`}
>
  <div class="mb-2 flex items-start justify-between gap-3">
    <div class="min-w-0">
      <p class="truncate text-xs font-semibold text-foreground">
        {entry.reporterDisplayName}
      </p>
      <p class="truncate text-[11px] text-muted-foreground">
        @{entry.reporterUsername}
      </p>
    </div>
    <span
      class="inline-flex h-7 w-7 shrink-0 items-center justify-center rounded-full text-[10px] font-semibold text-black"
      style={`background-color: ${entry.reporterAvatarColor ?? '#99aab5'};`}
      aria-hidden="true"
    >
      {entry.reporterDisplayName.slice(0, 1).toUpperCase()}
    </span>
  </div>

  <div class="mb-2 flex flex-wrap items-center gap-2 text-xs">
    <span
      class={`rounded border px-2 py-0.5 font-medium ${statusBadgeClass(entry.status)}`}
      data-testid={`report-status-badge-${entry.id}`}
    >
      {statusLabel(entry.status)}
    </span>
    <span class="text-muted-foreground">{entry.targetType === 'message' ? 'Message report' : 'User report'}</span>
  </div>

  <p class="line-clamp-2 text-xs text-foreground" data-testid={`report-target-preview-${entry.id}`}>
    {targetSummary(entry)}
  </p>
  <p class="mt-2 line-clamp-2 text-xs text-foreground" data-testid={`report-reason-${entry.id}`}>
    {entry.reason}
  </p>
  <p class="mt-2 text-[11px] text-muted-foreground" data-testid={`report-timestamp-${entry.id}`}>
    {formatTimestamp(entry.createdAt)}
  </p>

  {#if entry.status === 'pending' || entry.status === 'reviewed'}
    <div class="mt-3 flex flex-wrap gap-1.5">
      {#if entry.status === 'pending'}
        <button
          type="button"
          class="rounded border border-border px-2 py-1 text-[11px] text-foreground hover:bg-muted disabled:opacity-60"
          onclick={() => onReview?.(entry.id)}
          disabled={isActionDisabled('review')}
          data-testid={`report-action-review-${entry.id}`}
        >
          Review
        </button>
      {/if}
      <button
        type="button"
        class="rounded border border-border px-2 py-1 text-[11px] text-foreground hover:bg-muted disabled:opacity-60"
        onclick={() => onDismiss?.(entry.id)}
        disabled={isActionDisabled('dismiss')}
        data-testid={`report-action-dismiss-${entry.id}`}
      >
        Dismiss
      </button>
      <button
        type="button"
        class="rounded border border-border px-2 py-1 text-[11px] text-foreground hover:bg-muted disabled:opacity-60"
        onclick={() => onAct?.(entry.id, 'warn')}
        disabled={isActionDisabled('warn')}
        data-testid={`report-action-warn-${entry.id}`}
      >
        Warn
      </button>
      <button
        type="button"
        class="rounded border border-border px-2 py-1 text-[11px] text-foreground hover:bg-muted disabled:opacity-60"
        onclick={() => onAct?.(entry.id, 'mute')}
        disabled={isActionDisabled('mute')}
        data-testid={`report-action-mute-${entry.id}`}
      >
        Mute
      </button>
      <button
        type="button"
        class="rounded border border-border px-2 py-1 text-[11px] text-foreground hover:bg-muted disabled:opacity-60"
        onclick={() => onAct?.(entry.id, 'kick')}
        disabled={isActionDisabled('kick')}
        data-testid={`report-action-kick-${entry.id}`}
      >
        Kick
      </button>
      <button
        type="button"
        class="rounded border border-border px-2 py-1 text-[11px] text-foreground hover:bg-muted disabled:opacity-60"
        onclick={() => onAct?.(entry.id, 'ban')}
        disabled={isActionDisabled('ban')}
        data-testid={`report-action-ban-${entry.id}`}
      >
        Ban
      </button>
    </div>
  {/if}
</article>
