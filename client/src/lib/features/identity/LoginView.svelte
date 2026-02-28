<script lang="ts">
import { ApiError } from '$lib/api'

import { encryptAndStoreKey, generateIdentity } from './crypto'
import { identityState } from './identityStore.svelte'

const avatarColors = [
  { name: 'Blue', value: '#3b82f6', className: 'bg-blue-500' },
  { name: 'Red', value: '#ef4444', className: 'bg-red-500' },
  { name: 'Green', value: '#22c55e', className: 'bg-green-500' },
  { name: 'Amber', value: '#f59e0b', className: 'bg-amber-500' },
  { name: 'Purple', value: '#8b5cf6', className: 'bg-violet-500' },
  { name: 'Pink', value: '#ec4899', className: 'bg-pink-500' },
  { name: 'Cyan', value: '#06b6d4', className: 'bg-cyan-500' },
  { name: 'Orange', value: '#f97316', className: 'bg-orange-500' },
] as const

type AvatarColorValue = (typeof avatarColors)[number]['value']

function isAvatarColorValue(value: string): value is AvatarColorValue {
  return avatarColors.some((color) => color.value === value)
}

type Props = {
  oncomplete?: () => void
  onrecover?: () => void
  mode?: 'create' | 'reregister'
  inviteGuildName?: string | null
  inviteGuildIconUrl?: string | null
  inviteErrorMessage?: string | null
}

let {
  oncomplete,
  // biome-ignore lint/correctness/noUnusedVariables: Used in Svelte markup; Biome doesn't detect template usage.
  onrecover,
  mode = 'create',
  inviteGuildName = null,
  // biome-ignore lint/correctness/noUnusedVariables: Used in Svelte markup; Biome doesn't detect template usage.
  inviteGuildIconUrl = null,
  // biome-ignore lint/correctness/noUnusedVariables: Used in Svelte markup; Biome doesn't detect template usage.
  inviteErrorMessage = null,
}: Props = $props()

let username = $state('')
let avatarColor = $state<AvatarColorValue>(avatarColors[0].value)
let usernameInput = $state<HTMLInputElement | null>(null)

let submitting = $state(false)
// biome-ignore lint/correctness/noUnusedVariables: Used in Svelte markup; Biome doesn't detect template usage.
let serverError = $state<string | null>(null)
let usernameError = $state<string | null>(null)

// Avoid `binding_property_non_reactive` warnings for `bind:this={avatarButtons[i]}`.
let avatarButtons: Array<HTMLButtonElement | null> = $state([])

let usernameTrimmed = $derived(username.trim())
let usernameValidation = $derived(validateUsername(usernameTrimmed))
let inviteTargetName = $derived(inviteGuildName?.trim() || null)
// biome-ignore lint/correctness/noUnusedVariables: Used in Svelte markup; Biome doesn't detect template usage.
let titleText = $derived(
  mode === 'reregister'
    ? 'Choose a different name'
    : inviteTargetName
      ? `Pick a username to join ${inviteTargetName}`
      : 'Pick a username',
)
// biome-ignore lint/correctness/noUnusedVariables: Used in Svelte markup; Biome doesn't detect template usage.
let subtitleText = $derived(
  mode === 'create' && inviteTargetName
    ? 'Create your identity to continue.'
    : null,
)
// biome-ignore lint/correctness/noUnusedVariables: Used in Svelte markup; Biome doesn't detect template usage.
let canSubmit = $derived(usernameValidation === null && !submitting)

let didInit = false
$effect(() => {
  if (didInit) return

  if (mode === 'reregister') {
    const identity = identityState.identity
    if (!identity) return

    username = identity.username
    if (identity.avatarColor && isAvatarColorValue(identity.avatarColor)) {
      avatarColor = identity.avatarColor
    }
  }

  didInit = true
})

let didFocus = false
$effect(() => {
  if (!didFocus && usernameInput) {
    usernameInput.focus()
    didFocus = true
  }
})

function validateUsername(value: string): string | null {
  if (!value) return 'Username is required.'
  if (value.length > 32) return 'Username must be 32 characters or less.'
  if (!/^[a-zA-Z0-9_-]+$/.test(value)) {
    return 'Use only letters, numbers, underscore, or hyphen.'
  }
  return null
}

function setAvatarColorIndex(index: number) {
  const len = avatarColors.length
  const next = ((index % len) + len) % len
  avatarColor = avatarColors[next].value
  avatarButtons[next]?.focus()
}

// biome-ignore lint/correctness/noUnusedVariables: Used in Svelte markup; Biome doesn't detect template usage.
function avatarColorClass(value: AvatarColorValue): string {
  return avatarColors.find((c) => c.value === value)?.className ?? 'bg-blue-500'
}

// biome-ignore lint/correctness/noUnusedVariables: Used in Svelte markup; Biome doesn't detect template usage.
function onAvatarKeydown(event: KeyboardEvent, index: number) {
  switch (event.key) {
    case 'ArrowRight':
    case 'ArrowDown': {
      event.preventDefault()
      setAvatarColorIndex(index + 1)
      break
    }
    case 'ArrowLeft':
    case 'ArrowUp': {
      event.preventDefault()
      setAvatarColorIndex(index - 1)
      break
    }
    case 'Home': {
      event.preventDefault()
      setAvatarColorIndex(0)
      break
    }
    case 'End': {
      event.preventDefault()
      setAvatarColorIndex(avatarColors.length - 1)
      break
    }
  }
}

function userFacingError(err: unknown): string {
  if (err instanceof ApiError) {
    if (err.code === 'CONFLICT') {
      if (err.message === 'Username already taken') return err.message
      if (err.message === 'Identity already registered on this instance') {
        return "You're already set up on this instance."
      }
      return 'Something went wrong. Please try again.'
    }
    if (err.code === 'VALIDATION_ERROR') {
      // Only surface validation errors the user can act on.
      const msg = err.message.toLowerCase()
      if (msg.includes('username') || msg.includes('avatar')) {
        return err.message
      }
      return 'Something went wrong. Please try again.'
    }
    return 'Network error. Please try again.'
  }
  if (err instanceof Error) {
    const msg = err.message.toLowerCase()
    if (err.name === 'TypeError') return 'Network error. Please try again.'
    if (
      msg.includes('indexeddb') ||
      msg.includes('storage') ||
      msg.includes('database')
    ) {
      return 'Browser storage is unavailable. Please enable it and try again.'
    }
  }
  return 'Something went wrong. Please try again.'
}

// biome-ignore lint/correctness/noUnusedVariables: Used in Svelte markup; Biome doesn't detect template usage.
async function onSubmit(event: SubmitEvent) {
  event.preventDefault()
  if (submitting) return

  serverError = null
  usernameError = usernameValidation
  if (usernameError) return

  submitting = true
  try {
    if (mode === 'reregister') {
      await identityState.reRegister(
        usernameTrimmed,
        identityState.identity?.avatarColor ?? null,
      )
    } else {
      const { secretKey, publicKey, didKey } = await generateIdentity()
      await encryptAndStoreKey(
        secretKey,
        publicKey,
        didKey,
        usernameTrimmed,
        avatarColor,
      )
      await identityState.register(didKey, usernameTrimmed, avatarColor)
    }
    oncomplete?.()
  } catch (err) {
    if (
      err instanceof ApiError &&
      err.code === 'CONFLICT' &&
      err.message === 'Username already taken'
    ) {
      serverError = null
      usernameError = err.message
      usernameInput?.focus()
    } else {
      serverError = userFacingError(err)
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
      {#if mode === 'create' && inviteGuildIconUrl}
        <img
          src={inviteGuildIconUrl}
          alt="Guild icon"
          class="mx-auto h-12 w-12 rounded-full border border-border object-cover"
        />
      {/if}
      <h1 class="text-3xl font-semibold tracking-tight">{titleText}</h1>
      {#if subtitleText}
        <p class="text-sm text-muted-foreground">{subtitleText}</p>
      {/if}
    </header>

    {#if inviteErrorMessage}
      <div
        class="rounded-md border border-destructive/30 bg-destructive/10 p-3 text-sm text-destructive"
        role="alert"
      >
        {inviteErrorMessage}
      </div>
    {/if}

    {#if serverError}
      <div
        class="rounded-md border border-destructive/30 bg-destructive/10 p-3 text-sm text-destructive"
        role="alert"
      >
        {serverError}
      </div>
    {/if}

    <form class="space-y-6" onsubmit={onSubmit} novalidate>
      <div class="space-y-2">
        <label class="text-sm font-medium" for="username">Username</label>
        <input
          id="username"
          class={`w-full rounded-md border bg-background px-3 py-2 text-base placeholder:text-muted-foreground focus:outline-none focus:ring-2 ${
            usernameError ? 'border-destructive focus:ring-destructive' : 'border-input focus:ring-ring'
          }`}
          type="text"
          placeholder="Pick a username"
          bind:this={usernameInput}
          bind:value={username}
          onblur={() => {
            usernameError = usernameValidation
          }}
          autocomplete="username"
          required
        />
        {#if usernameError}
          <p class="text-sm text-destructive">{usernameError}</p>
        {/if}
      </div>

      <div class="space-y-2">
        <p class="text-sm font-medium">Avatar color</p>
        <div class="flex items-center gap-4">
          <div
            class={`flex h-10 w-10 items-center justify-center rounded-full text-sm font-semibold text-white ${avatarColorClass(
              avatarColor,
            )}`}
            role="img"
            aria-label="Avatar preview"
          >
            {usernameTrimmed.slice(0, 1).toUpperCase() || '?'}
          </div>

          <div class="flex flex-wrap gap-2" role="radiogroup" aria-label="Avatar color picker">
            {#each avatarColors as color, i}
              <button
                type="button"
                role="radio"
                aria-checked={avatarColor === color.value}
                tabindex={avatarColor === color.value ? 0 : -1}
                disabled={mode === 'reregister'}
                class={`h-8 w-8 rounded-full border ${
                  avatarColor === color.value
                    ? 'border-fire ring-2 ring-fire'
                    : 'border-border'
                } ${color.className}`}
                aria-label={`Select ${color.name}`}
                bind:this={avatarButtons[i]}
                onclick={() => (avatarColor = color.value)}
                onkeydown={(event) => onAvatarKeydown(event, i)}
              ></button>
            {/each}
          </div>
        </div>
      </div>

      <button
        type="submit"
        class="inline-flex w-full items-center justify-center gap-2 rounded-md bg-fire px-4 py-2 text-sm font-medium text-fire-foreground transition-opacity hover:opacity-90 disabled:cursor-not-allowed disabled:opacity-50"
        disabled={!canSubmit}
      >
        {#if submitting}
          <span
            class="inline-block h-4 w-4 animate-spin rounded-full border-2 border-fire-foreground border-t-transparent"
            aria-hidden="true"
          ></span>
          {mode === 'reregister' ? 'Registering...' : 'Creating...'}
        {:else}
          {mode === 'reregister' ? 'Register' : 'Create'}
        {/if}
      </button>

      {#if mode === 'create'}
        <button
          type="button"
          class="inline-flex w-full items-center justify-center rounded-md bg-muted px-4 py-2 text-sm font-medium text-foreground transition-opacity hover:opacity-90 disabled:cursor-not-allowed disabled:opacity-50"
          disabled={submitting}
          onclick={() => onrecover?.()}
        >
          Recover existing identity
        </button>
      {/if}
    </form>
  </div>
</main>
