<script lang="ts">
import { ApiError } from '$lib/api'

import { guildState } from './guildStore.svelte'

const MAX_ICON_BYTES = 2 * 1024 * 1024
const allowedIconTypes = new Set(['image/png', 'image/jpeg', 'image/webp'])

type Props = {
  open: boolean
  guildSlug: string
  onClose?: () => void | Promise<void>
}

let { open, guildSlug, onClose }: Props = $props()
let guild = $derived(guildState.bySlug(guildSlug))
let canEditGuild = $derived(Boolean(guild?.isOwner))

let initializedForSlug = $state<string | null>(null)
let name = $state('')
let description = $state('')
let selectedIcon = $state<File | null>(null)
let nameError = $state<string | null>(null)
let iconError = $state<string | null>(null)
// biome-ignore lint/correctness/noUnusedVariables: Used in Svelte markup; Biome doesn't detect template usage.
let errorMessage = $state<string | null>(null)
// biome-ignore lint/correctness/noUnusedVariables: Used in Svelte markup; Biome doesn't detect template usage.
let statusMessage = $state<string | null>(null)
let saving = $state(false)

function validateName(value: string): string | null {
  const trimmed = value.trim()
  if (!trimmed) return 'Guild name is required.'
  if (trimmed.length > 64) return 'Guild name must be 64 characters or less.'
  return null
}

function resetForm() {
  if (!guild) return
  name = guild.name
  description = guild.description ?? ''
  selectedIcon = null
  nameError = null
  iconError = null
  errorMessage = null
  statusMessage = null
}

$effect(() => {
  if (!open || !guild) return
  if (initializedForSlug === guild.slug) return
  initializedForSlug = guild.slug
  resetForm()
})

$effect(() => {
  if (open) return
  initializedForSlug = null
})

// biome-ignore lint/correctness/noUnusedVariables: Used in Svelte markup; Biome doesn't detect template usage.
function onNameBlur() {
  nameError = validateName(name)
}

// biome-ignore lint/correctness/noUnusedVariables: Used in Svelte markup; Biome doesn't detect template usage.
function onIconChange(event: Event) {
  iconError = null
  const input = event.currentTarget as HTMLInputElement | null
  const file = input?.files?.[0]
  if (!file) {
    selectedIcon = null
    return
  }

  if (!allowedIconTypes.has(file.type)) {
    selectedIcon = null
    iconError = 'Only PNG, JPEG, and WEBP images are supported.'
    return
  }
  if (file.size > MAX_ICON_BYTES) {
    selectedIcon = null
    iconError = 'Guild icon image must be 2 MB or smaller.'
    return
  }

  selectedIcon = file
}

// biome-ignore lint/correctness/noUnusedVariables: Used in Svelte markup; Biome doesn't detect template usage.
async function handleSubmit(event: SubmitEvent) {
  event.preventDefault()
  if (saving || !guild || !canEditGuild) return

  errorMessage = null
  statusMessage = null
  nameError = validateName(name)
  if (nameError || iconError) return

  saving = true
  try {
    await guildState.updateGuild(
      guild.slug,
      {
        name: name.trim(),
        description: description.trim() ? description.trim() : null,
      },
      selectedIcon,
    )
    selectedIcon = null
    statusMessage = 'Guild settings saved.'
  } catch (err) {
    if (err instanceof ApiError) {
      errorMessage = err.message
    } else if (err instanceof Error) {
      errorMessage = err.message
    } else {
      errorMessage = 'Failed to save guild settings.'
    }
  } finally {
    saving = false
  }
}

// biome-ignore lint/correctness/noUnusedVariables: Used in Svelte markup; Biome doesn't detect template usage.
async function handleClose() {
  await onClose?.()
}
</script>

{#if open}
  <div
    class="fixed inset-0 z-50 flex items-center justify-center bg-black/50 p-4"
    role="presentation"
  >
    <div
      class="w-full max-w-lg rounded-lg border border-border bg-card p-6 shadow-2xl"
      role="dialog"
      aria-modal="true"
      aria-label="Guild settings"
    >
      <header class="mb-4 flex items-center justify-between">
        <h2 class="text-lg font-semibold">Guild settings</h2>
        <button
          type="button"
          class="rounded-md bg-muted px-3 py-1 text-sm text-foreground hover:opacity-90"
          onclick={() => void handleClose()}
          aria-label="Close guild settings"
        >
          Close
        </button>
      </header>

      {#if !guild}
        <p class="rounded-md border border-destructive/30 bg-destructive/10 p-3 text-sm text-destructive">
          Guild not found.
        </p>
      {:else if !canEditGuild}
        <p class="rounded-md border border-destructive/30 bg-destructive/10 p-3 text-sm text-destructive">
          Only guild owners can edit guild settings.
        </p>
      {:else}
        {#if statusMessage}
          <p class="mb-4 rounded-md border border-emerald-500/30 bg-emerald-500/10 p-3 text-sm text-emerald-300">
            {statusMessage}
          </p>
        {/if}
        {#if errorMessage}
          <p class="mb-4 rounded-md border border-destructive/30 bg-destructive/10 p-3 text-sm text-destructive" role="alert">
            {errorMessage}
          </p>
        {/if}

        <form class="space-y-4" onsubmit={handleSubmit} novalidate>
          <div class="space-y-1">
            <label for="guild-settings-name" class="text-sm font-medium">Guild name</label>
            <input
              id="guild-settings-name"
              type="text"
              class={`w-full rounded-md border bg-background px-3 py-2 text-base focus:outline-none focus:ring-2 ${
                nameError
                  ? 'border-destructive focus:ring-destructive'
                  : 'border-input focus:ring-ring'
              }`}
              bind:value={name}
              onblur={onNameBlur}
              maxlength={64}
              required
            />
            {#if nameError}
              <p class="text-sm text-destructive">{nameError}</p>
            {/if}
          </div>

          <div class="space-y-1">
            <label for="guild-settings-description" class="text-sm font-medium">Description</label>
            <textarea
              id="guild-settings-description"
              class="h-24 w-full rounded-md border border-input bg-background px-3 py-2 text-base focus:outline-none focus:ring-2 focus:ring-ring"
              bind:value={description}
              maxlength={512}
            ></textarea>
          </div>

          <div class="space-y-1">
            <label for="guild-settings-icon" class="text-sm font-medium">Guild icon (optional)</label>
            <input
              id="guild-settings-icon"
              type="file"
              accept="image/png,image/jpeg,image/webp"
              class="block w-full text-sm text-muted-foreground file:mr-4 file:rounded-md file:border-0 file:bg-muted file:px-3 file:py-2 file:text-sm file:font-medium"
              onchange={onIconChange}
            />
            {#if iconError}
              <p class="text-sm text-destructive">{iconError}</p>
            {/if}
          </div>

          <button
            type="submit"
            class="inline-flex w-full items-center justify-center rounded-md bg-fire px-4 py-2 text-sm font-medium text-fire-foreground transition-opacity hover:opacity-90 disabled:cursor-not-allowed disabled:opacity-60"
            disabled={saving}
          >
            {saving ? 'Saving...' : 'Save Guild'}
          </button>
        </form>
      {/if}
    </div>
  </div>
{/if}
