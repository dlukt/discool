<script lang="ts">
import { ApiError } from '$lib/api'

import { createInvite, listInvites, revokeInvite } from './guildApi'
import { guildState } from './guildStore.svelte'
import type { GuildInvite, InviteType } from './types'

type Props = {
  open: boolean
  guildSlug: string
  onClose?: () => void | Promise<void>
}

let { open, guildSlug, onClose }: Props = $props()
let guild = $derived(guildState.bySlug(guildSlug))
let canManageInvites = $derived(Boolean(guild?.isOwner))

let inviteType = $state<InviteType>('reusable')
let invites = $state<GuildInvite[]>([])
// biome-ignore lint/correctness/noUnusedVariables: Used in Svelte markup; Biome doesn't detect template usage.
let loading = $state(false)
let generating = $state(false)
let revokingCode = $state<string | null>(null)
// biome-ignore lint/correctness/noUnusedVariables: Used in Svelte markup; Biome doesn't detect template usage.
let errorMessage = $state<string | null>(null)
// biome-ignore lint/correctness/noUnusedVariables: Used in Svelte markup; Biome doesn't detect template usage.
let statusMessage = $state<string | null>(null)

// biome-ignore lint/correctness/noUnusedVariables: Used in Svelte markup; Biome doesn't detect template usage.
function formatInviteDate(value: string): string {
  const timestamp = Date.parse(value)
  if (Number.isNaN(timestamp)) return value
  return new Date(timestamp).toLocaleString()
}

function toInviteLink(invite: GuildInvite): string {
  if (typeof window === 'undefined') return invite.inviteUrl
  const url = new URL(invite.inviteUrl, window.location.origin)
  if (guild?.name) {
    url.searchParams.set('guild_name', guild.name)
  }
  return url.toString()
}

async function loadInvites() {
  if (!canManageInvites) return
  loading = true
  errorMessage = null
  try {
    invites = await listInvites(guildSlug)
  } catch (err) {
    if (err instanceof ApiError) {
      errorMessage = err.message
    } else if (err instanceof Error) {
      errorMessage = err.message
    } else {
      errorMessage = 'Failed to load invites.'
    }
  } finally {
    loading = false
  }
}

// biome-ignore lint/correctness/noUnusedVariables: Used in Svelte markup; Biome doesn't detect template usage.
async function handleGenerateInvite() {
  if (generating || !canManageInvites) return
  errorMessage = null
  statusMessage = null
  generating = true
  try {
    const created = await createInvite(guildSlug, { type: inviteType })
    invites = [
      created,
      ...invites.filter((invite) => invite.code !== created.code),
    ]
  } catch (err) {
    if (err instanceof ApiError) {
      errorMessage = err.message
    } else if (err instanceof Error) {
      errorMessage = err.message
    } else {
      errorMessage = 'Failed to generate invite.'
    }
  } finally {
    generating = false
  }
}

// biome-ignore lint/correctness/noUnusedVariables: Used in Svelte markup; Biome doesn't detect template usage.
async function handleCopyInvite(invite: GuildInvite) {
  errorMessage = null
  statusMessage = null
  try {
    await navigator.clipboard.writeText(toInviteLink(invite))
    statusMessage = 'Invite link copied'
  } catch {
    errorMessage = 'Failed to copy invite link.'
  }
}

// biome-ignore lint/correctness/noUnusedVariables: Used in Svelte markup; Biome doesn't detect template usage.
async function handleRevokeInvite(invite: GuildInvite) {
  if (!canManageInvites || revokingCode) return
  errorMessage = null
  statusMessage = null
  revokingCode = invite.code
  try {
    await revokeInvite(guildSlug, invite.code)
    invites = invites.filter((item) => item.code !== invite.code)
  } catch (err) {
    if (err instanceof ApiError) {
      errorMessage = err.message
    } else if (err instanceof Error) {
      errorMessage = err.message
    } else {
      errorMessage = 'Failed to revoke invite.'
    }
  } finally {
    revokingCode = null
  }
}

// biome-ignore lint/correctness/noUnusedVariables: Used in Svelte markup; Biome doesn't detect template usage.
async function handleClose() {
  await onClose?.()
}

$effect(() => {
  if (!open) return
  void loadInvites()
})

$effect(() => {
  if (open) return
  invites = []
  inviteType = 'reusable'
  loading = false
  generating = false
  revokingCode = null
  errorMessage = null
  statusMessage = null
})
</script>

{#if open}
  <div
    class="fixed inset-0 z-50 flex items-center justify-center bg-black/50 p-4"
    role="presentation"
  >
    <div
      class="w-full max-w-2xl rounded-lg border border-border bg-card p-6 shadow-2xl"
      role="dialog"
      aria-modal="true"
      aria-label="Invite people"
    >
      <header class="mb-4 flex items-center justify-between">
        <h2 class="text-lg font-semibold">Invite people</h2>
        <button
          type="button"
          class="rounded-md bg-muted px-3 py-1 text-sm text-foreground hover:opacity-90"
          onclick={() => void handleClose()}
          aria-label="Close invite modal"
        >
          Close
        </button>
      </header>

      {#if !guild}
        <p class="rounded-md border border-destructive/30 bg-destructive/10 p-3 text-sm text-destructive">
          Guild not found.
        </p>
      {:else if !canManageInvites}
        <p class="rounded-md border border-destructive/30 bg-destructive/10 p-3 text-sm text-destructive">
          Only guild owners can manage invites.
        </p>
      {:else}
        {#if statusMessage}
          <p class="mb-3 rounded-md border border-emerald-500/30 bg-emerald-500/10 p-3 text-sm text-emerald-300" aria-live="polite">
            {statusMessage}
          </p>
        {:else}
          <p class="sr-only" aria-live="polite"></p>
        {/if}

        {#if errorMessage}
          <p class="mb-3 rounded-md border border-destructive/30 bg-destructive/10 p-3 text-sm text-destructive" role="alert">
            {errorMessage}
          </p>
        {/if}

        <div class="mb-4 flex flex-wrap items-end gap-3 rounded-md border border-border bg-background p-3">
          <div class="space-y-1">
            <label for="invite-type" class="text-sm font-medium">Invite type</label>
            <select
              id="invite-type"
              class="rounded-md border border-input bg-background px-3 py-2 text-sm text-foreground focus:outline-none focus:ring-2 focus:ring-ring"
              bind:value={inviteType}
            >
              <option value="reusable">Reusable</option>
              <option value="single_use">Single-use</option>
            </select>
          </div>
          <button
            type="button"
            class="inline-flex items-center justify-center rounded-md bg-fire px-4 py-2 text-sm font-medium text-fire-foreground transition-opacity hover:opacity-90 disabled:cursor-not-allowed disabled:opacity-60"
            onclick={() => void handleGenerateInvite()}
            disabled={generating}
          >
            {generating ? 'Generating...' : 'Generate invite'}
          </button>
        </div>

        <section class="space-y-2" aria-label="Active invites">
          <h3 class="text-sm font-semibold text-foreground">Active invites</h3>
          {#if loading}
            <p class="text-sm text-muted-foreground">Loading invites...</p>
          {:else if invites.length === 0}
            <p class="text-sm text-muted-foreground">No active invites yet.</p>
          {:else}
            <ul class="space-y-2">
              {#each invites as invite (invite.code)}
                <li
                  class="rounded-md border border-border bg-background p-3"
                  data-testid={`invite-card-${invite.code}`}
                >
                  <div class="mb-2 flex flex-wrap items-center justify-between gap-2">
                    <span class="rounded bg-muted px-2 py-0.5 text-xs font-medium uppercase tracking-wide text-muted-foreground">
                      {invite.type === 'single_use' ? 'Single-use' : 'Reusable'}
                    </span>
                    <span class="text-xs text-muted-foreground">
                      {formatInviteDate(invite.createdAt)}
                    </span>
                  </div>
                  <p class="mb-2 truncate text-xs text-foreground">{toInviteLink(invite)}</p>
                  <div class="mb-2 flex flex-wrap gap-2 text-xs text-muted-foreground">
                    <span>Created by {invite.creatorUsername}</span>
                    {#if invite.type === 'single_use'}
                      <span>
                        {#if invite.usesRemaining > 0}
                          {invite.usesRemaining} use remaining
                        {:else}
                          Invalid
                        {/if}
                      </span>
                    {/if}
                  </div>
                  <div class="flex gap-2">
                    <button
                      type="button"
                      class="rounded-md bg-muted px-3 py-1 text-xs font-medium text-foreground hover:opacity-90"
                      onclick={() => void handleCopyInvite(invite)}
                    >
                      Copy
                    </button>
                    <button
                      type="button"
                      class="rounded-md bg-destructive px-3 py-1 text-xs font-medium text-destructive-foreground hover:opacity-90 disabled:cursor-not-allowed disabled:opacity-60"
                      onclick={() => void handleRevokeInvite(invite)}
                      disabled={revokingCode === invite.code}
                    >
                      {revokingCode === invite.code ? 'Revoking...' : 'Revoke'}
                    </button>
                  </div>
                </li>
              {/each}
            </ul>
          {/if}
        </section>
      {/if}
    </div>
  </div>
{/if}
