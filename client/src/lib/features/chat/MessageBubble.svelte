<script lang="ts">
// biome-ignore-all lint/correctness/noUnusedVariables: Svelte template usage isn't detected reliably.
import type { ChatMessage } from './types'

type Props = {
  message: ChatMessage
  compact?: boolean
}

let { message, compact = false }: Props = $props()

let timestampLabel = $derived(formatTimestamp(message.createdAt))

function formatTimestamp(value: string): string {
  const date = new Date(value)
  if (Number.isNaN(date.getTime())) return value
  return date.toLocaleTimeString([], {
    hour: '2-digit',
    minute: '2-digit',
  })
}

function initials(value: string): string {
  const trimmed = value.trim()
  if (!trimmed) return '?'
  return trimmed[0]?.toUpperCase() ?? '?'
}
</script>

{#if message.isSystem}
  <div
    class="rounded-md px-3 py-2 text-center text-xs text-muted-foreground"
    data-testid={`message-system-${message.id}`}
  >
    {message.content}
  </div>
{:else}
  <div
    class={`flex gap-3 rounded-md px-2 py-1 ${message.optimistic ? 'opacity-80' : ''}`}
    data-testid={`message-row-${message.id}`}
    data-compact={compact}
  >
    {#if compact}
      <div class="w-8 shrink-0"></div>
    {:else}
      <span
        class="inline-flex h-8 w-8 shrink-0 items-center justify-center rounded-full text-xs font-semibold text-black"
        style={`background-color: ${message.authorAvatarColor ?? message.authorRoleColor};`}
        data-testid={`message-avatar-${message.id}`}
      >
        {initials(message.authorDisplayName)}
      </span>
    {/if}
    <div class="min-w-0 flex-1">
      {#if !compact}
        <div class="mb-0.5 flex items-baseline gap-2">
          <span
            class="truncate text-sm font-medium"
            style={`color: ${message.authorRoleColor};`}
            data-testid={`message-author-${message.id}`}
          >
            {message.authorDisplayName}
          </span>
          <time class="text-xs text-muted-foreground" datetime={message.createdAt}>
            {timestampLabel}
          </time>
        </div>
      {/if}
      <p
        class="whitespace-pre-wrap break-words text-sm text-foreground"
        data-testid={`message-content-${message.id}`}
      >
        {message.content}
      </p>
    </div>
  </div>
{/if}
