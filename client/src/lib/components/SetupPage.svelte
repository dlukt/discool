<script lang="ts">
import { createEventDispatcher } from 'svelte'
import {
  ApiError,
  type InstanceStatus,
  type SetupRequest,
  submitSetup,
} from '$lib/api'

const dispatch = createEventDispatcher<{ complete: InstanceStatus }>()

const avatarColors = [
  { name: 'Blue', value: '#3b82f6' },
  { name: 'Red', value: '#ef4444' },
  { name: 'Green', value: '#22c55e' },
  { name: 'Amber', value: '#f59e0b' },
  { name: 'Purple', value: '#8b5cf6' },
  { name: 'Pink', value: '#ec4899' },
  { name: 'Cyan', value: '#06b6d4' },
  { name: 'Orange', value: '#f97316' },
] as const

type AvatarColorValue = (typeof avatarColors)[number]['value']

let adminUsername = ''
let avatarColor: AvatarColorValue = avatarColors[0].value
let instanceName = ''
let instanceDescription = ''
let discoveryEnabled = true

let submitting = false
let serverError: string | null = null

let adminUsernameError: string | null = null
let instanceNameError: string | null = null

let avatarButtons: Array<HTMLButtonElement | null> = []

function validateAdminUsername(): string | null {
  if (!adminUsername.trim()) return 'Username is required.'
  return null
}

function validateInstanceName(): string | null {
  if (!instanceName.trim()) return 'Instance name is required.'
  return null
}

function setAvatarColorIndex(index: number) {
  const len = avatarColors.length
  const next = ((index % len) + len) % len
  avatarColor = avatarColors[next].value
  avatarButtons[next]?.focus()
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

// biome-ignore lint/correctness/noUnusedVariables: Used in Svelte markup; Biome doesn't detect template usage.
async function onSubmit() {
  if (submitting) return
  if (serverError) serverError = null

  adminUsernameError = validateAdminUsername()
  instanceNameError = validateInstanceName()

  if (adminUsernameError || instanceNameError) return

  const payload: SetupRequest = {
    adminUsername: adminUsername.trim(),
    avatarColor,
    instanceName: instanceName.trim(),
    instanceDescription: instanceDescription.trim() || undefined,
    discoveryEnabled,
  }

  submitting = true
  try {
    const status = await submitSetup(payload)
    dispatch('complete', status)
  } catch (err) {
    if (err instanceof ApiError) {
      serverError = err.message
    } else if (err instanceof Error) {
      serverError = err.message
    } else {
      serverError = 'Unexpected error.'
    }
  } finally {
    submitting = false
  }
}
</script>

<main class="min-h-screen bg-background p-8">
  <div class="mx-auto flex w-full max-w-md flex-col gap-6 rounded-lg border border-border bg-card p-8">
    <header class="space-y-2">
      <p class="text-sm font-medium text-muted-foreground">Discool</p>
      <h1 class="text-3xl font-semibold tracking-tight">Set up your instance</h1>
      <p class="text-sm text-muted-foreground">
        Create the admin identity and configure basic settings.
      </p>
    </header>

    {#if serverError}
      <div
        class="rounded-md border border-destructive/30 bg-destructive/10 p-3 text-sm text-destructive"
        role="alert"
      >
        {serverError}
      </div>
    {/if}

    <form class="space-y-6" on:submit|preventDefault={onSubmit} novalidate>
      <section class="space-y-4">
        <h2 class="text-sm font-semibold text-foreground">Admin identity</h2>

        <div class="space-y-2">
          <label class="text-sm font-medium" for="admin-username">Admin username</label>
          <input
            id="admin-username"
            class={`w-full rounded-md border bg-background px-3 py-2 text-base placeholder:text-muted-foreground focus:outline-none focus:ring-2 ${
              adminUsernameError
                ? 'border-destructive focus:ring-destructive'
                : 'border-input focus:ring-ring'
            }`}
            type="text"
      placeholder="Pick a username"
      bind:value={adminUsername}
      on:blur={() => {
        adminUsernameError = validateAdminUsername()
      }}
      autocomplete="username"
      required
    />
          {#if adminUsernameError}
            <p class="text-sm text-destructive">{adminUsernameError}</p>
          {/if}
        </div>

        <div class="space-y-2">
          <p class="text-sm font-medium">Avatar color</p>
          <div class="flex items-center gap-4">
            <div
              class="flex h-10 w-10 items-center justify-center rounded-full text-sm font-semibold text-white"
              style={`background-color: ${avatarColor}`}
              role="img"
              aria-label="Avatar preview"
             >
               {adminUsername.trim().slice(0, 1).toUpperCase() || '?'}
             </div>

              <div class="flex flex-wrap gap-2" role="radiogroup" aria-label="Avatar color picker">
                {#each avatarColors as color, i}
                  <button
                    type="button"
                    role="radio"
                    aria-checked={avatarColor === color.value}
                    tabindex={avatarColor === color.value ? 0 : -1}
                    class={`h-8 w-8 rounded-full border ${
                      avatarColor === color.value
                        ? 'border-fire ring-2 ring-fire'
                        : 'border-border'
                    }`}
                    style={`background-color: ${color.value}`}
                    aria-label={`Select ${color.name}`}
                    bind:this={avatarButtons[i]}
                    on:click={() => (avatarColor = color.value)}
                    on:keydown={(event) => onAvatarKeydown(event, i)}
                  ></button>
                {/each}
              </div>
          </div>
        </div>
      </section>

      <section class="space-y-4">
        <h2 class="text-sm font-semibold text-foreground">Instance settings</h2>

        <div class="space-y-2">
          <label class="text-sm font-medium" for="instance-name">Instance name</label>
          <input
            id="instance-name"
            class={`w-full rounded-md border bg-background px-3 py-2 text-base placeholder:text-muted-foreground focus:outline-none focus:ring-2 ${
              instanceNameError
                ? 'border-destructive focus:ring-destructive'
                : 'border-input focus:ring-ring'
            }`}
            type="text"
            placeholder="My Instance"
            bind:value={instanceName}
            on:blur={() => {
              instanceNameError = validateInstanceName()
            }}
            required
          />
          {#if instanceNameError}
            <p class="text-sm text-destructive">{instanceNameError}</p>
          {/if}
        </div>

        <div class="space-y-2">
          <label class="text-sm font-medium" for="instance-description">Description</label>
          <textarea
            id="instance-description"
            class="w-full resize-none rounded-md border border-input bg-background px-3 py-2 text-base placeholder:text-muted-foreground focus:outline-none focus:ring-2 focus:ring-ring"
            rows="3"
            placeholder="A short description of your instance"
            bind:value={instanceDescription}
          ></textarea>
        </div>

        <label class="flex items-start gap-3 text-sm">
          <input
            class="mt-1 h-4 w-4 accent-fire"
            type="checkbox"
            bind:checked={discoveryEnabled}
          />
          <span class="text-muted-foreground">
            Allow this instance to be discovered by others
          </span>
        </label>
      </section>

      <button
        type="submit"
        class="inline-flex w-full items-center justify-center rounded-md bg-fire px-4 py-2 text-sm font-medium text-fire-foreground transition-opacity hover:opacity-90 disabled:cursor-not-allowed disabled:opacity-50"
        disabled={submitting}
      >
        {submitting ? 'Setting up...' : 'Complete Setup'}
      </button>
    </form>
  </div>
</main>
