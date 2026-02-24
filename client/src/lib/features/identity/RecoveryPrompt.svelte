<script lang="ts">
type Props = {
  onstartfresh?: () => void | Promise<void>
  onrecover?: () => void | Promise<void>
}

// biome-ignore lint/correctness/noUnusedVariables: Used in Svelte markup; Biome doesn't detect template usage.
let { onstartfresh, onrecover }: Props = $props()

let submitting = $state(false)
// biome-ignore lint/correctness/noUnusedVariables: Used in Svelte markup; Biome doesn't detect template usage.
let errorMessage = $state<string | null>(null)

// biome-ignore lint/correctness/noUnusedVariables: Used in Svelte markup; Biome doesn't detect template usage.
async function handleStartFresh() {
  if (submitting) return
  submitting = true
  errorMessage = null
  try {
    await onstartfresh?.()
  } catch {
    errorMessage = "We couldn't clear your stored data. Please try again."
  } finally {
    submitting = false
  }
}
</script>

<main class="min-h-screen bg-background p-8">
  <div class="mx-auto w-full max-w-md space-y-6 rounded-lg border border-border bg-card p-8">
    <header class="space-y-2 text-center">
      <h1 class="text-2xl font-semibold tracking-tight">
        Your stored identity appears to be damaged
      </h1>
      <p class="text-sm text-muted-foreground">
        We couldn't load your saved identity. This can happen if browser data was partially cleared.
      </p>
    </header>

    {#if errorMessage}
      <div
        class="rounded-md border border-destructive/30 bg-destructive/10 p-3 text-sm text-destructive"
        role="alert"
      >
        {errorMessage}
      </div>
    {/if}

    <div class="space-y-3">
      <button
        type="button"
        class="inline-flex w-full items-center justify-center gap-2 rounded-md bg-fire px-4 py-2 text-sm font-medium text-fire-foreground transition-opacity hover:opacity-90 disabled:cursor-not-allowed disabled:opacity-50"
        onclick={() => void handleStartFresh()}
        disabled={submitting}
      >
        {#if submitting}
          <span
            class="inline-block h-4 w-4 animate-spin rounded-full border-2 border-fire-foreground border-t-transparent"
            aria-hidden="true"
          ></span>
          Starting fresh...
        {:else}
          Create a new identity
        {/if}
      </button>

      <button
        type="button"
        class="inline-flex w-full items-center justify-center rounded-md bg-muted px-4 py-2 text-sm font-medium text-foreground transition-opacity hover:opacity-90 disabled:cursor-not-allowed disabled:opacity-50"
        disabled
        title="Coming soon"
        onclick={() => void onrecover?.()}
      >
        Recover via email
      </button>
    </div>
  </div>
</main>
