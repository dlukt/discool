<script lang="ts">
// biome-ignore-all lint/correctness/noUnusedVariables: Svelte template usage isn't detected reliably.
import type { ChatMessage } from './types'

type Props = {
  message: ChatMessage
  compact?: boolean
  currentUserId?: string | null
  onEditRequest?: (message: ChatMessage) => void
  onDeleteRequest?: (message: ChatMessage) => void
  onReplyRequest?: (message: ChatMessage) => void
  onReactRequest?: (message: ChatMessage) => void
}

let {
  message,
  compact = false,
  currentUserId = null,
  onEditRequest,
  onDeleteRequest,
  onReplyRequest,
  onReactRequest,
}: Props = $props()

let timestampLabel = $derived(formatTimestamp(message.createdAt))
let isEdited = $derived(message.updatedAt !== message.createdAt)
let isOwnMessage = $derived(
  Boolean(currentUserId && currentUserId === message.authorUserId),
)
let contextMenuOpen = $state(false)

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

function openContextMenu(): void {
  if (!isOwnMessage) return
  contextMenuOpen = true
}

function closeContextMenu(): void {
  contextMenuOpen = false
}

function requestEdit(): void {
  if (!isOwnMessage) return
  onEditRequest?.(message)
  closeContextMenu()
}

function requestDelete(): void {
  if (!isOwnMessage) return
  onDeleteRequest?.(message)
  closeContextMenu()
}

function requestReply(): void {
  onReplyRequest?.(message)
}

function requestReact(): void {
  onReactRequest?.(message)
}

function handleRowContextMenu(event: MouseEvent): void {
  if (!isOwnMessage) return
  event.preventDefault()
  openContextMenu()
}

function handleRowKeydown(event: KeyboardEvent): void {
  if (event.key === 'ContextMenu' || (event.shiftKey && event.key === 'F10')) {
    if (!isOwnMessage) return
    event.preventDefault()
    openContextMenu()
    return
  }

  if (event.key === 'Delete') {
    if (!isOwnMessage) return
    event.preventDefault()
    requestDelete()
    return
  }

  if (event.key === 'Escape') {
    closeContextMenu()
  }
}

function handleRowFocusOut(event: FocusEvent): void {
  const target = event.currentTarget as HTMLElement | null
  const nextFocus = event.relatedTarget as Node | null
  if (target && nextFocus && target.contains(nextFocus)) return
  closeContextMenu()
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
    class={`group relative flex gap-3 rounded-md px-2 py-1 ${message.optimistic ? 'opacity-80' : ''}`}
    data-testid={`message-row-${message.id}`}
    data-compact={compact}
    role="button"
    aria-label={`Message from ${message.authorDisplayName}`}
    oncontextmenu={handleRowContextMenu}
    onkeydown={handleRowKeydown}
    onfocusout={handleRowFocusOut}
    tabindex="0"
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
          {#if isEdited}
            <span
              class="text-[11px] text-muted-foreground"
              data-testid={`message-edited-${message.id}`}
            >
              (edited)
            </span>
          {/if}
        </div>
      {/if}
      <p
        class="whitespace-pre-wrap break-words text-sm text-foreground"
        data-testid={`message-content-${message.id}`}
      >
        {message.content}
      </p>
    </div>

    <div
      class="absolute right-2 top-1 z-10 hidden items-center gap-1 rounded-md border border-border/70 bg-card/95 p-1 shadow-sm group-hover:flex group-focus-within:flex"
      data-testid={`message-action-bar-${message.id}`}
    >
      <button
        type="button"
        class="rounded px-1.5 py-1 text-xs text-foreground hover:bg-muted"
        onclick={requestEdit}
        disabled={!isOwnMessage}
        aria-label="Edit message"
        title="Edit message"
      >
        Edit
      </button>
      <button
        type="button"
        class="rounded px-1.5 py-1 text-xs text-foreground hover:bg-muted"
        onclick={requestDelete}
        disabled={!isOwnMessage}
        aria-label="Delete message"
        title="Delete message"
      >
        Delete
      </button>
      <button
        type="button"
        class="rounded px-1.5 py-1 text-xs text-foreground hover:bg-muted"
        onclick={requestReact}
        aria-label="React to message"
        title="React to message"
      >
        React
      </button>
      <button
        type="button"
        class="rounded px-1.5 py-1 text-xs text-foreground hover:bg-muted"
        onclick={requestReply}
        aria-label="Reply to message"
        title="Reply to message"
      >
        Reply
      </button>
    </div>

    {#if contextMenuOpen}
      <div
        class="absolute right-2 top-10 z-20 min-w-36 rounded-md border border-border bg-card p-1 shadow-lg"
        role="menu"
        aria-label="Message actions"
        data-testid={`message-context-menu-${message.id}`}
      >
        <button
          type="button"
          class="block w-full rounded px-2 py-1 text-left text-sm text-foreground hover:bg-muted disabled:cursor-not-allowed disabled:opacity-50"
          role="menuitem"
          onclick={requestEdit}
          disabled={!isOwnMessage}
        >
          Edit
        </button>
        <button
          type="button"
          class="mt-1 block w-full rounded px-2 py-1 text-left text-sm text-destructive hover:bg-destructive/10 disabled:cursor-not-allowed disabled:opacity-50"
          role="menuitem"
          onclick={requestDelete}
          disabled={!isOwnMessage}
        >
          Delete message
        </button>
      </div>
    {/if}
  </div>
{/if}
