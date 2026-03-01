<script lang="ts">
// biome-ignore-all lint/correctness/noUnusedVariables: Svelte template usage isn't detected reliably.
import { goto } from '@mateothegreat/svelte5-router'
import { onMount } from 'svelte'

import { ApiError } from '$lib/api'
// biome-ignore lint/correctness/noUnusedImports: Used in Svelte markup; Biome doesn't detect template usage.
import DMList from '$lib/features/dm/DMList.svelte'
import { dmState } from '$lib/features/dm/dmStore.svelte'
import { getLastViewedChannel } from '$lib/features/identity/navigationState'

import { guildState } from './guildStore.svelte'
import type { Guild } from './types'

const MAX_ICON_BYTES = 2 * 1024 * 1024
const allowedIconTypes = new Set(['image/png', 'image/jpeg', 'image/webp'])

type Props = {
  activeGuild: string
  activeChannel: string
  activeDm: string | null
  mode: 'home' | 'channel' | 'dm' | 'settings' | 'admin'
}

let { activeGuild, activeChannel, activeDm, mode }: Props = $props()

let guilds = $derived(guildState.guilds)
let createDialogOpen = $state(false)
let createName = $state('')
let createNameError = $state<string | null>(null)
let createError = $state<string | null>(null)
let createSubmitting = $state(false)
let selectedIcon = $state<File | null>(null)
let iconError = $state<string | null>(null)
let failedIcons = $state<Record<string, boolean>>({})
let draggedGuildSlug = $state<string | null>(null)
let hasUnreadDms = $derived(dmState.hasUnreadActivity())
let showDmList = $derived(mode === 'home')

onMount(() => {
  void guildState.loadGuilds().catch(() => {
    // errors are surfaced in the create form or other shell views; rail can remain usable.
  })
  void dmState.ensureLoaded().catch(() => {
    // DM list remains optional if loading fails.
  })
})

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

function onCreateNameBlur() {
  createNameError = validateGuildName(createName)
}

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

function onGuildIconError(slug: string) {
  failedIcons = { ...failedIcons, [slug]: true }
}

function resolveTargetChannel(guild: Guild): string {
  return (
    getLastViewedChannel(guild.slug) ??
    guild.lastViewedChannelSlug ??
    guild.defaultChannelSlug ??
    activeChannel
  )
}

async function goToGuild(guild: Guild): Promise<void> {
  await goto(`/${guild.slug}/${resolveTargetChannel(guild)}`)
}

async function goHome(): Promise<void> {
  await goto('/')
}

function tooltipIdForGuild(guildSlug: string): string {
  return `guild-tooltip-${guildSlug}`
}

function focusGuildButtonByOffset(source: HTMLElement, offset: number): void {
  const rail = source.closest('[data-testid="guild-rail"]')
  if (!rail) return
  const guildButtons = [
    ...rail.querySelectorAll<HTMLButtonElement>(
      '[data-guild-nav-button="true"]',
    ),
  ]
  if (guildButtons.length === 0) return
  const currentIndex = guildButtons.indexOf(source as HTMLButtonElement)
  if (currentIndex < 0) return
  const targetIndex =
    (currentIndex + offset + guildButtons.length) % guildButtons.length
  guildButtons[targetIndex]?.focus()
}

function onGuildKeydown(event: KeyboardEvent, guild: Guild): void {
  const source = event.currentTarget as HTMLElement | null
  if (!source) return

  if (event.key === 'ArrowDown' || event.key === 'ArrowRight') {
    event.preventDefault()
    focusGuildButtonByOffset(source, 1)
    return
  }
  if (event.key === 'ArrowUp' || event.key === 'ArrowLeft') {
    event.preventDefault()
    focusGuildButtonByOffset(source, -1)
    return
  }
  if (event.key === 'Enter') {
    event.preventDefault()
    void goToGuild(guild)
  }
}

function resolveDraggedGuildSlug(event: DragEvent): string | null {
  return draggedGuildSlug ?? event.dataTransfer?.getData('text/plain') ?? null
}

function onGuildDragStart(event: DragEvent, guildSlug: string): void {
  draggedGuildSlug = guildSlug
  event.dataTransfer?.setData('text/plain', guildSlug)
  if (event.dataTransfer) {
    event.dataTransfer.effectAllowed = 'move'
  }
}

function onGuildDragOver(event: DragEvent): void {
  const sourceSlug = resolveDraggedGuildSlug(event)
  if (!sourceSlug) return
  event.preventDefault()
  if (event.dataTransfer) {
    event.dataTransfer.dropEffect = 'move'
  }
}

function persistGuildOrder(sourceSlug: string, targetSlug: string): void {
  const slugs = guilds.map((guild) => guild.slug)
  const sourceIndex = slugs.indexOf(sourceSlug)
  const targetIndex = slugs.indexOf(targetSlug)
  if (sourceIndex < 0 || targetIndex < 0 || sourceIndex === targetIndex) return
  const [moved] = slugs.splice(sourceIndex, 1)
  slugs.splice(targetIndex, 0, moved)
  guildState.setGuildOrder(slugs)
}

function onGuildDrop(event: DragEvent, targetSlug: string): void {
  event.preventDefault()
  const sourceSlug = resolveDraggedGuildSlug(event)
  draggedGuildSlug = null
  if (!sourceSlug || sourceSlug === targetSlug) return
  persistGuildOrder(sourceSlug, targetSlug)
}

function onGuildDragEnd(): void {
  draggedGuildSlug = null
}
</script>

<div
  class="flex h-full w-full flex-col items-center gap-3 border-r border-border bg-sidebar py-3"
  data-testid="guild-rail"
>
  <div class="relative">
    <button
      type="button"
      class="inline-flex h-12 w-12 items-center justify-center rounded-full bg-fire text-sm font-semibold text-fire-foreground transition-opacity hover:opacity-90"
      aria-label="Home"
      data-testid="guild-rail-home"
      onclick={() => void goHome()}
    >
      H
    </button>
    {#if hasUnreadDms}
      <span
        class="absolute -right-0.5 -top-0.5 h-2.5 w-2.5 rounded-full bg-fire-foreground ring-2 ring-fire"
        data-testid="home-dm-unread-badge"
      ></span>
    {/if}
  </div>

  {#if showDmList}
    <DMList activeDm={activeDm} />
  {/if}

  <nav
    class="flex w-full flex-1 flex-col items-center gap-2 overflow-y-auto"
    aria-label="Guild navigation"
    role="list"
  >
    {#each guilds as guild}
      <div
        class="group relative flex w-full items-center justify-center"
        role="listitem"
        draggable={true}
        ondragstart={(event) => onGuildDragStart(event, guild.slug)}
        ondragover={onGuildDragOver}
        ondrop={(event) => onGuildDrop(event, guild.slug)}
        ondragend={onGuildDragEnd}
      >
        {#if guild.slug === activeGuild}
          <span
            class="absolute -left-0.5 h-7 w-1 rounded-r-full bg-primary"
            data-testid={`guild-active-indicator-${guild.slug}`}
            aria-hidden="true"
          ></span>
        {/if}

        <button
          type="button"
          class={`inline-flex h-12 w-12 items-center justify-center overflow-hidden rounded-full border text-xs font-semibold transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary ${
            guild.slug === activeGuild
              ? 'border-primary bg-primary/20 text-primary'
              : 'border-border bg-muted text-muted-foreground hover:bg-sidebar-accent hover:text-sidebar-accent-foreground'
          }`}
          data-guild-nav-button="true"
          title={guild.name}
          aria-label={guild.name}
          aria-describedby={tooltipIdForGuild(guild.slug)}
          aria-current={guild.slug === activeGuild ? 'page' : undefined}
          onclick={() => void goToGuild(guild)}
          onkeydown={(event) => onGuildKeydown(event, guild)}
        >
          {#if guild.iconUrl && !failedIcons[guild.slug]}
            <img
              src={guild.iconUrl}
              alt=""
              class="h-12 w-12 rounded-full object-cover"
              onerror={() => onGuildIconError(guild.slug)}
            />
          {:else}
            {initials(guild.name)}
          {/if}
        </button>

        {#if guild.hasUnreadActivity}
          <span
            class="absolute right-2 top-1.5 h-2.5 w-2.5 rounded-full bg-fire ring-2 ring-sidebar"
            data-testid={`guild-unread-badge-${guild.slug}`}
            aria-label={`${guild.name} has unread activity`}
          ></span>
        {/if}

        <span
          id={tooltipIdForGuild(guild.slug)}
          class="pointer-events-none absolute left-full ml-2 whitespace-nowrap rounded-md bg-card px-2 py-1 text-xs text-foreground opacity-0 shadow transition-opacity group-hover:opacity-100 group-focus-within:opacity-100"
          role="tooltip"
        >
          {guild.name}
        </span>
      </div>
    {/each}
  </nav>

  <button
    type="button"
    class="inline-flex h-12 w-12 items-center justify-center rounded-full border border-border bg-muted text-xl font-semibold text-muted-foreground transition-colors hover:bg-sidebar-accent hover:text-sidebar-accent-foreground"
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
