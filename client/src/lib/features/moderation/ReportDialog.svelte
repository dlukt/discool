<script lang="ts">
// biome-ignore-all lint/correctness/noUnusedVariables: Svelte template usage isn't detected reliably.
// biome-ignore-all lint/correctness/noUnusedImports: Svelte template usage isn't detected reliably.
import type { ReportCategory } from './moderationApi'

type ReportDialogPayload = {
  reason: string
  category: ReportCategory | null
}

type Props = {
  open: boolean
  title: string
  description: string
  submitLabel?: string
  submitting?: boolean
  errorMessage?: string | null
  onCancel?: () => void
  onSubmit?: (payload: ReportDialogPayload) => void | Promise<void>
}

const MAX_REPORT_REASON_CHARS = 500

let {
  open,
  title,
  description,
  submitLabel = 'Submit report',
  submitting = false,
  errorMessage = null,
  onCancel,
  onSubmit,
}: Props = $props()

let reason = $state('')
let category = $state<ReportCategory | ''>('')
let localError = $state<string | null>(null)

$effect(() => {
  if (!open) return
  reason = ''
  category = ''
  localError = null
})

function closeDialog(): void {
  if (submitting) return
  onCancel?.()
}

function clearLocalError(): void {
  if (localError) localError = null
}

function submit(): void {
  if (submitting) return
  const trimmedReason = reason.trim()
  if (!trimmedReason) {
    localError = 'Reason is required.'
    return
  }
  if (trimmedReason.length > MAX_REPORT_REASON_CHARS) {
    localError = `Reason must be ${MAX_REPORT_REASON_CHARS} characters or less.`
    return
  }
  localError = null
  void onSubmit?.({
    reason: trimmedReason,
    category: category || null,
  })
}
</script>

{#if open}
  <div class="fixed inset-0 z-40 flex items-center justify-center bg-black/60 p-4">
    <div
      class="w-full max-w-md rounded-md border border-border bg-card p-4 shadow-lg"
      role="dialog"
      aria-modal="true"
      aria-label={title}
      data-testid="report-dialog"
    >
      <h3 class="text-base font-semibold text-foreground">{title}</h3>
      <p class="mt-2 text-sm text-muted-foreground">{description}</p>
      <label class="mt-3 block space-y-1 text-xs text-muted-foreground">
        <span class="font-medium text-foreground">Reason</span>
        <textarea
          class="min-h-[96px] w-full rounded-md border border-input bg-background px-3 py-2 text-sm text-foreground focus:outline-none focus:ring-2 focus:ring-ring"
          bind:value={reason}
          data-testid="report-reason-input"
          maxlength={MAX_REPORT_REASON_CHARS}
          placeholder="Required report reason"
          oninput={clearLocalError}
        ></textarea>
      </label>
      <label class="mt-3 block space-y-1 text-xs text-muted-foreground">
        <span class="font-medium text-foreground">Category (optional)</span>
        <select
          class="w-full rounded-md border border-input bg-background px-3 py-2 text-sm text-foreground focus:outline-none focus:ring-2 focus:ring-ring"
          bind:value={category}
          data-testid="report-category-select"
          oninput={clearLocalError}
        >
          <option value="">No category</option>
          <option value="spam">Spam</option>
          <option value="harassment">Harassment</option>
          <option value="rule_violation">Rule violation</option>
          <option value="other">Other</option>
        </select>
      </label>
      {#if localError || errorMessage}
        <p
          class="mt-3 rounded-md border border-destructive/40 bg-destructive/10 px-2 py-1 text-xs text-destructive"
          data-testid="report-error"
        >
          {localError ?? errorMessage}
        </p>
      {/if}
      <div class="mt-4 flex justify-end gap-2">
        <button
          type="button"
          class="rounded-md border border-border bg-background px-3 py-2 text-sm font-medium text-foreground hover:bg-muted disabled:cursor-not-allowed disabled:opacity-60"
          onclick={closeDialog}
          disabled={submitting}
        >
          Cancel
        </button>
        <button
          type="button"
          class="rounded-md bg-destructive px-3 py-2 text-sm font-medium text-destructive-foreground hover:opacity-90 disabled:cursor-not-allowed disabled:opacity-60"
          onclick={submit}
          disabled={submitting}
          data-testid="report-submit-button"
        >
          {submitting ? 'Submitting...' : submitLabel}
        </button>
      </div>
    </div>
  </div>
{/if}
