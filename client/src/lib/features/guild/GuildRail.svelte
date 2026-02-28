<script lang="ts">
// biome-ignore lint/correctness/noUnusedImports: Used in Svelte markup; Biome doesn't detect template usage.
import { goto, route as routerLink } from '@mateothegreat/svelte5-router'
import { onMount } from 'svelte'

import { ApiError } from '$lib/api'

import { guildState } from './guildStore.svelte'

const MAX_ICON_BYTES = 2 * 1024 * 1024
const allowedIconTypes = new Set(['image/png', 'image/jpeg', 'image/webp'])

type Props = {
  activeGuild: string
  activeChannel: string
}

// biome-ignore lint/correctness/noUnusedVariables: Used in Svelte markup; Biome doesn't detect template usage.
let { activeGuild, activeChannel }: Props = $props()

// biome-ignore lint/correctness/noUnusedVariables: Used in Svelte markup; Biome doesn't detect template usage.
let guilds = $derived(guildState.guilds)
// biome-ignore lint/correctness/noUnusedVariables: Used in Svelte markup; Biome doesn't detect template usage.
let createDialogOpen = $state(false)
let createName = $state('')
let createNameError = $state<string | null>(null)
// biome-ignore lint/correctness/noUnusedVariables: Used in Svelte markup; Biome doesn't detect template usage.
let createError = $state<string | null>(null)
let createSubmitting = $state(false)
let selectedIcon = $state<File | null>(null)
let iconError = $state<string | null>(null)
let failedIcons = $state<Record<string, boolean>>({})

onMount(() => {
  void guildState.loadGuilds().catch(() => {
    // errors are surfaced in the create form or other shell views; rail can remain usable.
  })
})

// biome-ignore lint/correctness/noUnusedVariables: Used in Svelte markup; Biome doesn't detect template usage.
function initials(value: string): string {
  const trimmed = value.trim()
  if (!trimmed) return '?'
  return trimmed.slice(0, 1).toUpperCase()
}

function validateGuildName(value: string): string | null {
  const trimmed = value.trim()
  if (!trimmed) return 'Guild name is required.'
  if (trimmed.length > 64) return 'Guild name must be 64 characters or less.'
  return null
}

// biome-ignore lint/correctness/noUnusedVariables: Used in Svelte markup; Biome doesn't detect template usage.
function openCreateDialog() {
  createDialogOpen = true
  createName = ''
  createNameError = null
  createError = null
  selectedIcon = null
  iconError = null
}

function closeCreateDialog() {
  createDialogOpen = false
}

// biome-ignore lint/correctness/noUnusedVariables: Used in Svelte markup; Biome doesn't detect template usage.
function onCreateNameBlur() {
  createNameError = validateGuildName(createName)
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
async function handleCreateSubmit(event: SubmitEvent) {
  event.preventDefault()
  if (createSubmitting) return

  createError = null
  createNameError = validateGuildName(createName)
  if (createNameError || iconError) return

  createSubmitting = true
  try {
    const guild = await guildState.createGuild(
      { name: createName.trim() },
      selectedIcon,
    )
    closeCreateDialog()
    await goto(`/${guild.slug}/${guild.defaultChannelSlug}`)
  } catch (err) {
    if (err instanceof ApiError) {
      createError = err.message
    } else if (err instanceof Error) {
      createError = err.message
    } else {
      createError = 'Failed to create guild.'
    }
  } finally {
    createSubmitting = false
  }
}

// biome-ignore lint/correctness/noUnusedVariables: Used in Svelte markup; Biome doesn't detect template usage.
function onGuildIconError(slug: string) {
  failedIcons = { ...failedIcons, [slug]: true }
}
</script>

<div
  class="flex h-full w-full flex-col items-center gap-3 border-r border-border bg-sidebar py-3"
  data-testid="guild-rail"
>
  <a
    class="inline-flex h-10 w-10 items-center justify-center rounded-xl bg-fire text-sm font-semibold text-fire-foreground"
    href="/"
    use:routerLink
    aria-label="Go to home"
  >
    D
  </a>

  <nav class="flex w-full flex-1 flex-col items-center gap-2 overflow-y-auto" aria-label="Guild navigation">
    {#each guilds as guild}
      <a
        class={`inline-flex h-10 w-10 items-center justify-center rounded-xl border text-xs font-semibold transition-colors ${
          guild.slug === activeGuild
            ? 'border-primary bg-primary/20 text-primary'
            : 'border-border bg-muted text-muted-foreground hover:bg-sidebar-accent hover:text-sidebar-accent-foreground'
        }`}
        href={`/${guild.slug}/${guild.defaultChannelSlug || activeChannel}`}
        use:routerLink
        aria-label={`Open ${guild.name} guild`}
        aria-current={guild.slug === activeGuild ? 'page' : undefined}
      >
        {#if guild.iconUrl && !failedIcons[guild.slug]}
          <img
            src={guild.iconUrl}
            alt={`${guild.name} icon`}
            class="h-10 w-10 rounded-xl object-cover"
            onerror={() => onGuildIconError(guild.slug)}
          />
        {:else}
          {initials(guild.name)}
        {/if}
      </a>
    {/each}
  </nav>

  <button
    type="button"
    class="inline-flex h-10 w-10 items-center justify-center rounded-xl border border-border bg-muted text-xl font-semibold text-muted-foreground transition-colors hover:bg-sidebar-accent hover:text-sidebar-accent-foreground"
    aria-label="Create guild"
    onclick={openCreateDialog}
  >
    +
  </button>
</div>

{#if createDialogOpen}
  <div
    class="fixed inset-0 z-50 flex items-center justify-center bg-black/50 p-4"
    role="presentation"
  >
    <div
      class="w-full max-w-md rounded-lg border border-border bg-card p-6 shadow-2xl"
      role="dialog"
      aria-modal="true"
      aria-label="Create Guild"
    >
      <header class="mb-4 flex items-center justify-between">
        <h2 class="text-lg font-semibold">Create Guild</h2>
        <button
          type="button"
          class="rounded-md bg-muted px-3 py-1 text-sm text-foreground hover:opacity-90"
          onclick={closeCreateDialog}
          aria-label="Close create guild dialog"
        >
          Close
        </button>
      </header>

      {#if createError}
        <p class="mb-4 rounded-md border border-destructive/30 bg-destructive/10 p-3 text-sm text-destructive" role="alert">
          {createError}
        </p>
      {/if}

      <form class="space-y-4" onsubmit={handleCreateSubmit} novalidate>
        <div class="space-y-1">
          <label for="guild-create-name" class="text-sm font-medium">Guild name</label>
          <input
            id="guild-create-name"
            type="text"
            class={`w-full rounded-md border bg-background px-3 py-2 text-base focus:outline-none focus:ring-2 ${
              createNameError
                ? 'border-destructive focus:ring-destructive'
                : 'border-input focus:ring-ring'
            }`}
            bind:value={createName}
            onblur={onCreateNameBlur}
            maxlength={64}
            required
          />
          {#if createNameError}
            <p class="text-sm text-destructive">{createNameError}</p>
          {/if}
        </div>

        <div class="space-y-1">
          <label for="guild-create-icon" class="text-sm font-medium">Guild icon (optional)</label>
          <input
            id="guild-create-icon"
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
          disabled={createSubmitting}
        >
          {createSubmitting ? 'Creating...' : 'Create Guild'}
        </button>
      </form>
    </div>
  </div>
{/if}
