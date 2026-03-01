<script lang="ts">
// biome-ignore-all lint/correctness/noUnusedVariables: Svelte template usage isn't detected reliably.
// biome-ignore-all lint/correctness/noUnusedImports: Svelte template usage isn't detected reliably.
import { type ToastVariant, toastState } from '$lib/feedback/toastStore.svelte'

function classesForVariant(variant: ToastVariant): string {
  if (variant === 'error') {
    return 'border-destructive/40 bg-destructive/10 text-destructive'
  }
  if (variant === 'success') {
    return 'border-emerald-500/40 bg-emerald-500/10 text-emerald-200'
  }
  return 'border-border bg-card text-foreground'
}

function handleFocusOut(event: FocusEvent, toastId: string): void {
  const currentTarget = event.currentTarget as HTMLElement | null
  const nextTarget = event.relatedTarget
  if (currentTarget && nextTarget instanceof Node) {
    if (currentTarget.contains(nextTarget)) {
      return
    }
  }
  toastState.resume(toastId)
}

let visibleToasts = $derived.by(() => {
  const _toastVersion = toastState.version
  void _toastVersion
  return toastState.toasts
})
</script>

<div
  class="pointer-events-none fixed bottom-4 right-4 z-50 flex w-full max-w-sm flex-col gap-2 px-3 sm:px-0"
  data-testid="toast-viewport"
  aria-live="polite"
  aria-atomic="false"
>
  {#each visibleToasts as toast (toast.id)}
    <section
      class={`pointer-events-auto rounded-md border p-3 shadow-lg ${classesForVariant(toast.variant)}`}
      role={toast.variant === 'error' ? 'alert' : 'status'}
      data-testid="toast-item"
      onmouseenter={() => toastState.pause(toast.id)}
      onmouseleave={() => toastState.resume(toast.id)}
      onfocusin={() => toastState.pause(toast.id)}
      onfocusout={(event) => handleFocusOut(event, toast.id)}
    >
      <div class="flex items-start gap-2">
        <p class="min-w-0 flex-1 text-sm">{toast.message}</p>
        <button
          type="button"
          class="rounded px-1 text-xs hover:bg-background/20"
          onclick={() => toastState.dismiss(toast.id)}
          aria-label="Dismiss notification"
          data-testid="toast-dismiss"
        >
          Dismiss
        </button>
      </div>

      {#if toast.actionLabel && toast.onAction}
        <div class="mt-2 flex justify-end">
          <button
            type="button"
            class="rounded-md border border-current/30 px-2 py-1 text-xs font-medium hover:bg-background/20"
            onclick={() => toastState.runAction(toast.id)}
            data-testid="toast-action"
          >
            {toast.actionLabel}
          </button>
        </div>
      {/if}
    </section>
  {/each}
</div>
