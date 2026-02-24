<script lang="ts">
import { ApiError } from '$lib/api'

import { identityState } from './identityStore.svelte'

type Props = {
  onusedifferentname?: () => void
}

function safeAvatarColor(value: string | null): string {
  if (typeof value === 'string' && /^#[0-9a-fA-F]{6}$/.test(value)) return value
  return '#3b82f6'
}

// biome-ignore lint/correctness/noUnusedVariables: Used in Svelte markup; Biome doesn't detect template usage.
let { onusedifferentname }: Props = $props()

let submitting = $state(false)
// biome-ignore lint/correctness/noUnusedVariables: Used in Svelte markup; Biome doesn't detect template usage.
let errorMessage = $state<string | null>(null)
// biome-ignore lint/correctness/noUnusedVariables: Used in Svelte markup; Biome doesn't detect template usage.
let showTryDifferentInstance = $state(false)

let username = $derived(identityState.identity?.username ?? '')
// biome-ignore lint/correctness/noUnusedVariables: Used in Svelte markup; Biome doesn't detect template usage.
let avatarBg = $derived(
  safeAvatarColor(identityState.identity?.avatarColor ?? null),
)
// biome-ignore lint/correctness/noUnusedVariables: Used in Svelte markup; Biome doesn't detect template usage.
let avatarInitial = $derived(username.trim().slice(0, 1).toUpperCase() || '?')

// biome-ignore lint/correctness/noUnusedVariables: Used in Svelte markup; Biome doesn't detect template usage.
async function onRegister() {
  if (submitting) return
  errorMessage = null

  const identity = identityState.identity
  if (!identity) {
    errorMessage = 'We could not find your saved identity.'
    return
  }

  submitting = true
  try {
    await identityState.register(
      identity.didKey,
      identity.username,
      identity.avatarColor,
    )
  } catch (err) {
    if (
      err instanceof ApiError &&
      err.code === 'CONFLICT' &&
      err.message === 'Username already taken'
    ) {
      errorMessage =
        'That name is already taken on this instance. Choose a different name.'
    } else if (
      err instanceof ApiError &&
      err.code === 'CONFLICT' &&
      err.message === 'Identity already registered on this instance'
    ) {
      // If the server already has the user, just sign in.
      identityState.identityNotRegistered = false
      void identityState.authenticate()
    } else if (err instanceof ApiError) {
      errorMessage = 'Network error. Please try again.'
    } else {
      errorMessage = 'Something went wrong. Please try again.'
    }
  } finally {
    submitting = false
  }
}
</script>

<main class="min-h-screen bg-background p-8">
  <div class="mx-auto flex w-full max-w-md flex-col gap-6 rounded-lg border border-border bg-card p-8">
    <header class="space-y-2 text-center">
      <p class="text-sm font-medium text-muted-foreground">Discool</p>
      <h1 class="text-3xl font-semibold tracking-tight">Welcome back!</h1>
      <p class="text-sm text-muted-foreground">
        Your identity isn't registered on this instance yet. Would you like to register?
      </p>
    </header>

    <div class="flex items-center gap-4 rounded-md bg-muted p-4">
      <div
        class="flex h-10 w-10 items-center justify-center rounded-full text-sm font-semibold text-white"
        style={`background-color: ${avatarBg}`}
        role="img"
        aria-label="Avatar preview"
      >
        {avatarInitial}
      </div>
      <div class="min-w-0">
        <p class="truncate text-sm font-semibold">{username}</p>
        <p class="text-xs text-muted-foreground">We'll keep your saved data and set you up again.</p>
      </div>
    </div>

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
        onclick={onRegister}
        disabled={!username.trim() || submitting}
      >
        {#if submitting}
          <span
            class="inline-block h-4 w-4 animate-spin rounded-full border-2 border-fire-foreground border-t-transparent"
            aria-hidden="true"
          ></span>
          Registering...
        {:else}
          Register as {username}
        {/if}
      </button>

      <button
        type="button"
        class="inline-flex w-full items-center justify-center rounded-md bg-muted px-4 py-2 text-sm font-medium text-foreground transition-opacity hover:opacity-90"
        onclick={onusedifferentname}
      >
        Use a different name
      </button>

      <button
        type="button"
        class="inline-flex w-full items-center justify-center rounded-md border border-border bg-background px-4 py-2 text-sm font-medium text-foreground transition-colors hover:bg-muted"
        onclick={() => (showTryDifferentInstance = !showTryDifferentInstance)}
      >
        Try a different instance
      </button>
    </div>

    {#if showTryDifferentInstance}
      <div class="rounded-md bg-muted p-4 text-sm text-muted-foreground">
        If you're trying to use a different Discool server, open that instance URL in your browser
        (or a new tab) and sign in there.
      </div>
    {/if}
  </div>
</main>
