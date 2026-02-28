<script lang="ts">
// biome-ignore-all lint/correctness/noUnusedVariables: Svelte template usage isn't detected reliably.
import type { ChatMessage, ChatMessageAttachment } from './types'

const EMOJI_PICKER_OPTIONS = ['😀', '😂', '😍', '👍', '🎉', '🔥', '👏', '😢']

type Props = {
  message: ChatMessage
  compact?: boolean
  currentUserId?: string | null
  onEditRequest?: (message: ChatMessage) => void
  onDeleteRequest?: (message: ChatMessage) => void
  onReplyRequest?: (message: ChatMessage) => void
  onReactRequest?: (message: ChatMessage, emoji: string) => void
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
let hasContent = $derived(message.content.trim().length > 0)
let isOwnMessage = $derived(
  Boolean(currentUserId && currentUserId === message.authorUserId),
)
let contextMenuOpen = $state(false)
let pickerOpen = $state(false)
let imagePreviewAttachment = $state<ChatMessageAttachment | null>(null)

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
  contextMenuOpen = true
}

function closeContextMenu(): void {
  contextMenuOpen = false
}

function openPicker(): void {
  pickerOpen = true
}

function closePicker(): void {
  pickerOpen = false
}

function closePopovers(): void {
  closeContextMenu()
  closePicker()
}

function formatFileSize(sizeBytes: number): string {
  if (!Number.isFinite(sizeBytes) || sizeBytes <= 0) return '0 B'
  if (sizeBytes < 1024) return `${Math.round(sizeBytes)} B`
  if (sizeBytes < 1024 * 1024) return `${(sizeBytes / 1024).toFixed(1)} KB`
  return `${(sizeBytes / (1024 * 1024)).toFixed(1)} MB`
}

function openImagePreview(attachment: ChatMessageAttachment): void {
  imagePreviewAttachment = attachment
}

function closeImagePreview(): void {
  imagePreviewAttachment = null
}

function handleImagePreviewBackdropClick(event: MouseEvent): void {
  if (event.target !== event.currentTarget) return
  closeImagePreview()
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
  openPicker()
}

function requestReactionToggle(emoji: string): void {
  const normalized = emoji.trim()
  if (!normalized) return
  onReactRequest?.(message, normalized)
  closePopovers()
}

function handleRowContextMenu(event: MouseEvent): void {
  event.preventDefault()
  openContextMenu()
}

function handleRowKeydown(event: KeyboardEvent): void {
  if (imagePreviewAttachment && event.key === 'Escape') {
    event.preventDefault()
    closeImagePreview()
    return
  }

  if (event.key === 'ContextMenu' || (event.shiftKey && event.key === 'F10')) {
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
    closePopovers()
  }
}

function handleRowFocusOut(event: FocusEvent): void {
  const target = event.currentTarget as HTMLElement | null
  const nextFocus = event.relatedTarget as Node | null
  if (target && nextFocus && target.contains(nextFocus)) return
  closePopovers()
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
      {#if hasContent}
        <p
          class="whitespace-pre-wrap break-words text-sm text-foreground"
          data-testid={`message-content-${message.id}`}
        >
          {message.content}
        </p>
      {/if}
      {#if message.attachments.length > 0}
        <div
          class="mt-2 space-y-2"
          data-testid={`message-attachments-${message.id}`}
        >
          {#each message.attachments as attachment, index (`${attachment.id}-${index}`)}
            {#if attachment.isImage}
              <button
                type="button"
                class="group/image relative block max-w-sm overflow-hidden rounded-md border border-border/60 bg-background"
                onclick={() => openImagePreview(attachment)}
                aria-label={`Open image ${attachment.originalFilename}`}
                data-testid={`message-attachment-image-${message.id}-${index}`}
              >
                <img
                  src={attachment.url}
                  alt={attachment.originalFilename}
                  loading="lazy"
                  class="max-h-56 w-full object-cover transition-opacity group-hover/image:opacity-95"
                />
                <span class="absolute bottom-1 right-1 rounded bg-black/60 px-1.5 py-0.5 text-[11px] text-white">
                  {formatFileSize(attachment.sizeBytes)}
                </span>
              </button>
            {:else}
              <a
                href={attachment.url}
                target="_blank"
                rel="noopener noreferrer"
                download={attachment.originalFilename}
                class="flex items-center gap-2 rounded-md border border-border/70 bg-background px-3 py-2 text-sm text-foreground hover:bg-muted/60"
                data-testid={`message-attachment-file-${message.id}-${index}`}
              >
                <span aria-hidden="true">📄</span>
                <span class="min-w-0 flex-1 truncate">
                  {attachment.originalFilename}
                </span>
                <span class="shrink-0 text-xs text-muted-foreground">
                  {formatFileSize(attachment.sizeBytes)}
                </span>
              </a>
            {/if}
          {/each}
        </div>
      {/if}
      {#if message.reactions.length > 0}
        <div
          class="mt-2 flex flex-wrap gap-1"
          data-testid={`message-reaction-badges-${message.id}`}
        >
          {#each message.reactions as reaction, index (`${reaction.emoji}-${index}`)}
            <button
              type="button"
              class={`inline-flex items-center gap-1 rounded-full border px-2 py-0.5 text-xs transition-colors ${
                reaction.reacted
                  ? 'border-fire/60 bg-fire/20 text-fire-foreground'
                  : 'border-border bg-muted/60 text-foreground hover:bg-muted'
              }`}
              onclick={() => requestReactionToggle(reaction.emoji)}
              aria-label={`Toggle reaction ${reaction.emoji}`}
              data-testid={`message-reaction-badge-${message.id}-${index}`}
            >
              <span>{reaction.emoji}</span>
              <span>{reaction.count}</span>
            </button>
          {/each}
        </div>
      {/if}
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
        data-testid={`message-react-button-${message.id}`}
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
          class="block w-full rounded px-2 py-1 text-left text-sm text-foreground hover:bg-muted"
          role="menuitem"
          onclick={requestReact}
        >
          React
        </button>
        <button
          type="button"
          class="mt-1 block w-full rounded px-2 py-1 text-left text-sm text-foreground hover:bg-muted disabled:cursor-not-allowed disabled:opacity-50"
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

    {#if pickerOpen}
      <div
        class="absolute right-2 top-10 z-20 w-44 rounded-md border border-border bg-card p-2 shadow-lg"
        role="dialog"
        aria-label="Emoji picker"
        data-testid={`message-reaction-picker-${message.id}`}
      >
        <div class="grid grid-cols-4 gap-1">
          {#each EMOJI_PICKER_OPTIONS as emoji, index (`${emoji}-${index}`)}
            <button
              type="button"
              class="rounded px-1 py-1 text-base leading-none hover:bg-muted"
              onclick={() => requestReactionToggle(emoji)}
              aria-label={`React with ${emoji}`}
              data-testid={`message-reaction-picker-option-${message.id}-${index}`}
            >
              {emoji}
            </button>
          {/each}
        </div>
      </div>
    {/if}
  </div>

  {#if imagePreviewAttachment}
    <div
      class="fixed inset-0 z-40 flex items-center justify-center bg-black/85 p-4"
      role="dialog"
      aria-modal="true"
      aria-label="Attachment preview"
      onkeydown={handleRowKeydown}
      onclick={handleImagePreviewBackdropClick}
      tabindex="0"
      data-testid={`message-image-preview-${message.id}`}
    >
      <div class="max-h-full max-w-5xl">
        <img
          src={imagePreviewAttachment.url}
          alt={imagePreviewAttachment.originalFilename}
          class="max-h-[85vh] max-w-full rounded-md object-contain"
        />
        <div class="mt-3 flex justify-end gap-2">
          <a
            href={imagePreviewAttachment.url}
            target="_blank"
            rel="noopener noreferrer"
            download={imagePreviewAttachment.originalFilename}
            class="rounded-md border border-border bg-card px-3 py-1.5 text-xs text-foreground hover:bg-muted"
          >
            Download
          </a>
          <button
            type="button"
            class="rounded-md bg-fire px-3 py-1.5 text-xs text-fire-foreground"
            onclick={closeImagePreview}
          >
            Close
          </button>
        </div>
      </div>
    </div>
  {/if}
{/if}
