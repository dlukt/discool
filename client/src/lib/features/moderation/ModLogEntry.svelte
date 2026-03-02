<script lang="ts">
// biome-ignore-all lint/correctness/noUnusedVariables: Svelte template usage isn't detected reliably.
import type { ModerationLogEntry } from './moderationApi'

type Props = {
  entry: ModerationLogEntry
}

let { entry }: Props = $props()

function actionLabel(actionType: string): string {
  return actionType
    .replace(/_/g, ' ')
    .replace(/\b\w/g, (char) => char.toUpperCase())
}

function actionBadgeClass(actionType: string): string {
  if (actionType === 'mute') {
    return 'border-sky-300/40 bg-sky-500/20 text-sky-200'
  }
  if (actionType === 'kick') {
    return 'border-orange-300/40 bg-orange-500/20 text-orange-200'
  }
  if (actionType === 'ban') {
    return 'border-red-300/40 bg-red-500/20 text-red-200'
  }
  if (actionType === 'voice_kick') {
    return 'border-violet-300/40 bg-violet-500/20 text-violet-200'
  }
  return 'border-border bg-muted text-muted-foreground'
}

function formatTimestamp(value: string): string {
  const timestamp = Date.parse(value)
  if (Number.isNaN(timestamp)) {
    return value
  }
  return new Date(timestamp).toLocaleString()
}
</script>

<article
  class="rounded-md border border-border bg-card/60 p-3"
  data-testid={`mod-log-entry-${entry.id}`}
>
  <div class="mb-2 flex items-start justify-between gap-3">
    <div class="min-w-0">
      <p class="truncate text-xs font-semibold text-foreground">
        {entry.actorDisplayName}
      </p>
      <p class="truncate text-[11px] text-muted-foreground">
        @{entry.actorUsername}
      </p>
    </div>
    <span
      class="inline-flex h-7 w-7 shrink-0 items-center justify-center rounded-full text-[10px] font-semibold text-black"
      style={`background-color: ${entry.actorAvatarColor ?? '#99aab5'};`}
      aria-hidden="true"
    >
      {entry.actorDisplayName.slice(0, 1).toUpperCase()}
    </span>
  </div>

  <div class="mb-2 flex flex-wrap items-center gap-2 text-xs">
    <span
      class={`rounded border px-2 py-0.5 font-medium ${actionBadgeClass(entry.actionType)}`}
      data-testid={`mod-log-action-badge-${entry.id}`}
    >
      {actionLabel(entry.actionType)}
    </span>
    <span class="text-muted-foreground">
      Target: {entry.targetDisplayName}
    </span>
  </div>

  <p class="line-clamp-2 text-xs text-foreground" data-testid={`mod-log-reason-${entry.id}`}>
    {entry.reason}
  </p>
  <p class="mt-2 text-[11px] text-muted-foreground" data-testid={`mod-log-timestamp-${entry.id}`}>
    {formatTimestamp(entry.createdAt)}
  </p>
</article>
