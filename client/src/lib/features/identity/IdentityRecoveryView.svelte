<script lang="ts">
import { ApiError } from '$lib/api'

import { identityState } from './identityStore.svelte'

type Props = {
  token?: string | null
  oncancel?: () => void | Promise<void>
  oncleartoken?: () => void
}

// biome-ignore lint/correctness/noUnusedVariables: Used in Svelte markup; Biome doesn't detect template usage.
let { token = null, oncancel, oncleartoken }: Props = $props()

let email = $state('')
let submitting = $state(false)
let redeeming = $state(false)
// biome-ignore lint/correctness/noUnusedVariables: Used in Svelte markup; Biome doesn't detect template usage.
let statusMessage = $state<string | null>(null)
// biome-ignore lint/correctness/noUnusedVariables: Used in Svelte markup; Biome doesn't detect template usage.
let helpMessage = $state<string | null>(null)
// biome-ignore lint/correctness/noUnusedVariables: Used in Svelte markup; Biome doesn't detect template usage.
let errorMessage = $state<string | null>(null)
let redeemedToken = $state<string | null>(null)

function toUserError(err: unknown): string {
  if (err instanceof ApiError) {
    if (err.message.trim()) return err.message
    return 'We could not complete recovery. Please try again.'
  }
  if (err instanceof Error && err.message.trim()) {
    return err.message
  }
  return 'We could not complete recovery. Please try again.'
}

function clearRecoveryTokenFromUrl() {
  if (typeof window === 'undefined') return
  const url = new URL(window.location.href)
  url.searchParams.delete('recovery_token')
  window.history.replaceState({}, '', `${url.pathname}${url.search}${url.hash}`)
  oncleartoken?.()
}

async function redeemWithToken(value: string) {
  redeeming = true
  submitting = false
  errorMessage = null
  statusMessage = 'Restoring your identity...'
  helpMessage = null
  try {
    await identityState.recoverIdentityByToken(value)
    clearRecoveryTokenFromUrl()
  } catch (err) {
    errorMessage = toUserError(err)
  } finally {
    redeeming = false
  }
}

$effect(() => {
  const nextToken = token?.trim() ?? ''
  if (!nextToken || nextToken === redeemedToken || redeeming) return
  redeemedToken = nextToken
  void redeemWithToken(nextToken)
})

// biome-ignore lint/correctness/noUnusedVariables: Used in Svelte markup; Biome doesn't detect template usage.
async function onSubmit(event: SubmitEvent) {
  event.preventDefault()
  if (submitting || redeeming) return
  const normalizedEmail = email.trim()
  if (!normalizedEmail) {
    errorMessage = 'Email is required'
    return
  }

  submitting = true
  errorMessage = null
  statusMessage = null
  helpMessage = null
  try {
    const response = await identityState.startIdentityRecovery(normalizedEmail)
    statusMessage = response.message
    helpMessage = response.helpMessage
  } catch (err) {
    errorMessage = toUserError(err)
  } finally {
    submitting = false
  }
}
</script>

<main class="min-h-screen bg-background p-8">
  <div class="mx-auto w-full max-w-md space-y-6 rounded-lg border border-border bg-card p-8">
    <header class="space-y-2 text-center">
      <h1 class="text-2xl font-semibold tracking-tight">Recover existing identity</h1>
      <p class="text-sm text-muted-foreground">
        Enter your recovery email and we will send you a link to restore your identity.
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

    {#if statusMessage}
      <div
        class="rounded-md border border-border bg-muted p-3 text-sm text-foreground"
        role="status"
      >
        {statusMessage}
      </div>
    {/if}

    {#if helpMessage}
      <p class="text-sm text-muted-foreground">{helpMessage}</p>
    {/if}

    <form class="space-y-4" onsubmit={onSubmit} novalidate>
      <div class="space-y-2">
        <label class="text-sm font-medium" for="recovery-email">Recovery email</label>
        <input
          id="recovery-email"
          class="w-full rounded-md border border-input bg-background px-3 py-2 text-base placeholder:text-muted-foreground focus:outline-none focus:ring-2 focus:ring-ring"
          type="email"
          placeholder="you@example.com"
          bind:value={email}
          autocomplete="email"
          required
          disabled={submitting || redeeming}
        />
      </div>

      <button
        type="submit"
        class="inline-flex w-full items-center justify-center gap-2 rounded-md bg-fire px-4 py-2 text-sm font-medium text-fire-foreground transition-opacity hover:opacity-90 disabled:cursor-not-allowed disabled:opacity-50"
        disabled={submitting || redeeming}
      >
        {#if submitting}
          <span
            class="inline-block h-4 w-4 animate-spin rounded-full border-2 border-fire-foreground border-t-transparent"
            aria-hidden="true"
          ></span>
          Sending recovery email...
        {:else if redeeming}
          <span
            class="inline-block h-4 w-4 animate-spin rounded-full border-2 border-fire-foreground border-t-transparent"
            aria-hidden="true"
          ></span>
          Restoring identity...
        {:else}
          Send recovery email
        {/if}
      </button>
    </form>

    <button
      type="button"
      class="inline-flex w-full items-center justify-center rounded-md bg-muted px-4 py-2 text-sm font-medium text-foreground transition-opacity hover:opacity-90 disabled:cursor-not-allowed disabled:opacity-50"
      disabled={submitting || redeeming}
      onclick={() => {
        clearRecoveryTokenFromUrl()
        void oncancel?.()
      }}
    >
      Create new identity
    </button>
  </div>
</main>
