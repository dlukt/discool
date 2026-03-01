<script lang="ts">
import { ApiError } from '$lib/api'

import { blockState } from './blockStore.svelte'
import { identityState } from './identityStore.svelte'

const MAX_AVATAR_BYTES = 2 * 1024 * 1024
const allowedAvatarTypes = new Set(['image/png', 'image/jpeg', 'image/webp'])
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

type AvatarMode = 'color' | 'image'
type AvatarColorValue = (typeof avatarColors)[number]['value']

let sessionUser = $derived(identityState.session?.user ?? null)
let recoveryEmailStatus = $derived(identityState.recoveryEmailStatus)
let displayName = $state('')
let avatarColor = $state<AvatarColorValue>(avatarColors[0].value)
let avatarMode = $state<AvatarMode>('color')
let selectedFile = $state<File | null>(null)
let previewUrl = $state<string | null>(null)
let displayNameError = $state<string | null>(null)
// biome-ignore lint/correctness/noUnusedVariables: Used in Svelte markup; Biome doesn't detect template usage.
let avatarError = $state<string | null>(null)
// biome-ignore lint/correctness/noUnusedVariables: Used in Svelte markup; Biome doesn't detect template usage.
let statusMessage = $state<string | null>(null)
// biome-ignore lint/correctness/noUnusedVariables: Used in Svelte markup; Biome doesn't detect template usage.
let errorMessage = $state<string | null>(null)
let saving = $state(false)
let recoveryEmailInput = $state('')
let recoverySending = $state(false)
// biome-ignore lint/correctness/noUnusedVariables: Used in Svelte markup; Biome doesn't detect template usage.
let recoveryStatusMessage = $state<string | null>(null)
// biome-ignore lint/correctness/noUnusedVariables: Used in Svelte markup; Biome doesn't detect template usage.
let recoveryErrorMessage = $state<string | null>(null)
let blockActionPendingUserId = $state<string | null>(null)
// biome-ignore lint/correctness/noUnusedVariables: Used in Svelte markup; Biome doesn't detect template usage.
let blockActionStatusMessage = $state<string | null>(null)
// biome-ignore lint/correctness/noUnusedVariables: Used in Svelte markup; Biome doesn't detect template usage.
let blockActionErrorMessage = $state<string | null>(null)

// biome-ignore lint/correctness/noUnusedVariables: Used in Svelte markup; Biome doesn't detect template usage.
let blockedUsers = $derived.by(() => {
  const _blockVersion = blockState.version
  void _blockVersion
  return blockState
    .blockedUsers()
    .map((record) => {
      const activeInterval = [...record.intervals]
        .reverse()
        .find((interval) => interval.unblockedAt === null)
      return {
        userId: record.userId,
        displayName:
          record.displayName?.trim() ||
          record.username?.trim() ||
          record.userId,
        username: record.username?.trim() || null,
        blockedAt:
          activeInterval?.blockedAt ??
          record.intervals[record.intervals.length - 1]?.blockedAt ??
          '',
      }
    })
    .sort((left, right) => left.displayName.localeCompare(right.displayName))
})

let initialized = false
$effect(() => {
  if (!sessionUser || initialized) return
  displayName = sessionUser.displayName
  const matchingColor = avatarColors.find(
    (color) => color.value === sessionUser.avatarColor,
  )
  avatarColor = matchingColor?.value ?? avatarColors[0].value
  avatarMode = sessionUser.avatarUrl ? 'image' : 'color'
  initialized = true
})

$effect(() => {
  return () => {
    if (previewUrl) {
      URL.revokeObjectURL(previewUrl)
    }
  }
})

function validateDisplayName(value: string): string | null {
  const trimmed = value.trim()
  if (!trimmed) return 'Display name is required.'
  if (trimmed.length > 64) return 'Display name must be 64 characters or less.'
  return null
}

// biome-ignore lint/correctness/noUnusedVariables: Used in Svelte markup; Biome doesn't detect template usage.
function avatarColorClass(value: AvatarColorValue): string {
  return (
    avatarColors.find((color) => color.value === value)?.className ??
    'bg-blue-500'
  )
}

// biome-ignore lint/correctness/noUnusedVariables: Used in Svelte markup; Biome doesn't detect template usage.
function onDisplayNameBlur() {
  displayNameError = validateDisplayName(displayName)
}

function clearSelectedAvatar() {
  if (previewUrl) {
    URL.revokeObjectURL(previewUrl)
  }
  selectedFile = null
  previewUrl = null
  avatarError = null
}

// biome-ignore lint/correctness/noUnusedVariables: Used in Svelte markup; Biome doesn't detect template usage.
function onAvatarFileChange(event: Event) {
  avatarError = null
  const input = event.currentTarget as HTMLInputElement | null
  const file = input?.files?.[0]
  if (!file) {
    clearSelectedAvatar()
    return
  }
  if (!allowedAvatarTypes.has(file.type)) {
    clearSelectedAvatar()
    avatarError = 'Only PNG, JPEG, and WEBP images are supported.'
    return
  }
  if (file.size > MAX_AVATAR_BYTES) {
    clearSelectedAvatar()
    avatarError = 'Avatar image must be 2 MB or smaller.'
    return
  }

  if (previewUrl) {
    URL.revokeObjectURL(previewUrl)
  }
  selectedFile = file
  previewUrl = URL.createObjectURL(file)
}

// biome-ignore lint/correctness/noUnusedVariables: Used in Svelte markup; Biome doesn't detect template usage.
async function onSubmit(event: SubmitEvent) {
  event.preventDefault()
  if (saving || !sessionUser) return

  statusMessage = null
  errorMessage = null
  displayNameError = validateDisplayName(displayName)
  if (displayNameError) return
  if (avatarMode === 'image' && !selectedFile && !sessionUser.avatarUrl) {
    avatarError = 'Choose an avatar image to continue.'
    return
  }

  saving = true
  try {
    const profileInput: {
      displayName: string
      avatarColor?: AvatarColorValue
    } = {
      displayName: displayName.trim(),
    }
    if (avatarMode === 'color') {
      profileInput.avatarColor = avatarColor
    }

    await identityState.saveProfile(
      profileInput,
      avatarMode === 'image' ? selectedFile : null,
    )
    if (avatarMode === 'image') {
      clearSelectedAvatar()
    }
    statusMessage = 'Profile saved.'
  } catch (err) {
    if (err instanceof ApiError) {
      errorMessage = err.message
    } else if (err instanceof Error) {
      errorMessage = err.message
    } else {
      errorMessage = 'Failed to save profile settings.'
    }
  } finally {
    saving = false
  }
}

// biome-ignore lint/correctness/noUnusedVariables: Used in Svelte markup; Biome doesn't detect template usage.
function recoveryStatusLabel(): string {
  if (!recoveryEmailStatus?.associated) return 'Not configured'
  if (recoveryEmailStatus.verified) return 'Verified'
  return 'Unverified'
}

// biome-ignore lint/correctness/noUnusedVariables: Used in Svelte markup; Biome doesn't detect template usage.
function recoveryActionLabel(): string {
  if (recoveryEmailStatus?.associated && !recoveryEmailStatus.verified) {
    return 'Resend verification'
  }
  return 'Send verification'
}

// biome-ignore lint/correctness/noUnusedVariables: Used in Svelte markup; Biome doesn't detect template usage.
async function onRecoverySubmit(event: SubmitEvent) {
  event.preventDefault()
  if (recoverySending || !sessionUser) return

  recoveryStatusMessage = null
  recoveryErrorMessage = null
  recoverySending = true
  try {
    await identityState.startRecoveryEmailAssociation(recoveryEmailInput)
    recoveryStatusMessage =
      'Verification email sent. Check your inbox and click the link to verify.'
  } catch (err) {
    if (err instanceof ApiError) {
      recoveryErrorMessage = err.message
    } else if (err instanceof Error) {
      recoveryErrorMessage = err.message
    } else {
      recoveryErrorMessage = 'Failed to send recovery verification email.'
    }
  } finally {
    recoverySending = false
  }
}

// biome-ignore lint/correctness/noUnusedVariables: Used in Svelte markup; Biome doesn't detect template usage.
function blockedAtLabel(blockedAt: string): string {
  const parsed = new Date(blockedAt)
  if (Number.isNaN(parsed.getTime())) return blockedAt
  return parsed.toLocaleString()
}

// biome-ignore lint/correctness/noUnusedVariables: Used in Svelte markup; Biome doesn't detect template usage.
async function unblockFromSettings(userId: string, displayName: string) {
  if (blockActionPendingUserId) return
  blockActionStatusMessage = null
  blockActionErrorMessage = null
  blockActionPendingUserId = userId
  try {
    const result = await blockState.unblockUser(userId)
    blockActionStatusMessage = result.synced
      ? `Unblocked ${displayName}.`
      : `Unblocked ${displayName}. Local change saved, but sync failed: ${result.syncError}`
  } catch (err) {
    if (err instanceof ApiError) {
      blockActionErrorMessage = err.message
    } else if (err instanceof Error) {
      blockActionErrorMessage = err.message
    } else {
      blockActionErrorMessage = 'Failed to unblock user.'
    }
  } finally {
    blockActionPendingUserId = null
  }
}
</script>

{#if !sessionUser}
  <div class="mx-auto w-full max-w-xl rounded-lg border border-border bg-card p-6">
    <p class="text-sm text-muted-foreground">Sign in to manage your profile settings.</p>
  </div>
{:else}
  <div class="mx-auto w-full max-w-xl rounded-lg border border-border bg-card p-6">
    <header class="mb-5 space-y-1">
      <h2 class="text-2xl font-semibold tracking-tight">Profile settings</h2>
      <p class="text-sm text-muted-foreground">
        Update your public display name and avatar appearance.
      </p>
    </header>

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

    <form class="space-y-5" onsubmit={onSubmit} novalidate>
      <div class="space-y-2">
        <label for="display-name" class="text-sm font-medium">Display name</label>
        <input
          id="display-name"
          type="text"
          class={`w-full rounded-md border bg-background px-3 py-2 text-base focus:outline-none focus:ring-2 ${
            displayNameError
              ? 'border-destructive focus:ring-destructive'
              : 'border-input focus:ring-ring'
          }`}
          bind:value={displayName}
          onblur={onDisplayNameBlur}
          maxlength={64}
          autocomplete="nickname"
        />
        {#if displayNameError}
          <p class="text-sm text-destructive">{displayNameError}</p>
        {/if}
      </div>

      <fieldset class="space-y-2">
        <legend class="text-sm font-medium">Avatar mode</legend>
        <div class="flex flex-wrap gap-4 text-sm">
          <label class="inline-flex items-center gap-2">
            <input
              type="radio"
              name="avatar-mode"
              value="color"
              checked={avatarMode === 'color'}
              onchange={() => {
                avatarMode = 'color'
                clearSelectedAvatar()
              }}
            />
            Use avatar color
          </label>
          <label class="inline-flex items-center gap-2">
            <input
              type="radio"
              name="avatar-mode"
              value="image"
              checked={avatarMode === 'image'}
              onchange={() => (avatarMode = 'image')}
            />
            Upload image
          </label>
        </div>
      </fieldset>

      <div class="space-y-2">
        <p class="text-sm font-medium">Avatar color</p>
        <div class="flex flex-wrap gap-2" role="radiogroup" aria-label="Avatar color picker">
          {#each avatarColors as color}
            <button
              type="button"
              class={`h-8 w-8 rounded-full border ${
                avatarColor === color.value ? 'border-fire ring-2 ring-fire' : 'border-border'
              } ${color.className}`}
              aria-label={`Select ${color.name}`}
              aria-pressed={avatarColor === color.value}
              onclick={() => (avatarColor = color.value)}
            ></button>
          {/each}
        </div>
      </div>

      {#if avatarMode === 'image'}
        <div class="space-y-2">
          <label for="avatar-file" class="text-sm font-medium">Avatar image</label>
          <input
            id="avatar-file"
            type="file"
            accept="image/png,image/jpeg,image/webp"
            class="block w-full text-sm text-muted-foreground file:mr-4 file:rounded-md file:border-0 file:bg-muted file:px-3 file:py-2 file:text-sm file:font-medium"
            onchange={onAvatarFileChange}
          />
          {#if avatarError}
            <p class="text-sm text-destructive">{avatarError}</p>
          {/if}
        </div>
      {/if}

      <div class="space-y-3 rounded-md border border-border bg-muted p-4">
        <p class="text-sm font-medium">Avatar preview</p>
        {#if avatarMode === 'image' && previewUrl}
          <img src={previewUrl} alt="Avatar preview" class="h-16 w-16 rounded-full object-cover" />
        {:else if avatarMode === 'image' && sessionUser.avatarUrl}
          <img
            src={sessionUser.avatarUrl}
            alt="Avatar preview"
            class="h-16 w-16 rounded-full object-cover"
          />
        {:else}
          <div
            class={`flex h-16 w-16 items-center justify-center rounded-full text-lg font-semibold text-white ${avatarColorClass(
              avatarColor,
            )}`}
            role="img"
            aria-label="Avatar preview"
          >
            {(displayName.trim().slice(0, 1) || '?').toUpperCase()}
          </div>
        {/if}

        {#if previewUrl}
          <button
            type="button"
            class="text-sm text-muted-foreground underline underline-offset-2 hover:text-foreground"
            onclick={clearSelectedAvatar}
          >
            Remove selected image
          </button>
        {/if}
      </div>

      <button
        type="submit"
        class="inline-flex w-full items-center justify-center rounded-md bg-fire px-4 py-2 text-sm font-medium text-fire-foreground transition-opacity hover:opacity-90 disabled:cursor-not-allowed disabled:opacity-60"
        disabled={saving}
      >
        {saving ? 'Saving...' : 'Save profile'}
      </button>
    </form>

    <section class="mt-6 rounded-md border border-border bg-muted p-4">
      <header class="mb-3 space-y-1">
        <h3 class="text-sm font-semibold">Recovery email (optional)</h3>
        <p class="text-sm text-muted-foreground">
          Add an email so this identity can be recovered later if browser storage is lost.
        </p>
      </header>

      <p class="text-sm text-muted-foreground">
        Status:
        <span class="font-medium text-foreground">{recoveryStatusLabel()}</span>
        {#if recoveryEmailStatus?.emailMasked}
          ({recoveryEmailStatus.emailMasked})
        {/if}
      </p>

      {#if recoveryStatusMessage}
        <p class="mt-3 rounded-md border border-emerald-500/30 bg-emerald-500/10 p-3 text-sm text-emerald-300">
          {recoveryStatusMessage}
        </p>
      {/if}
      {#if recoveryErrorMessage}
        <p class="mt-3 rounded-md border border-destructive/30 bg-destructive/10 p-3 text-sm text-destructive" role="alert">
          {recoveryErrorMessage}
        </p>
      {/if}

      <form class="mt-3 flex flex-col gap-3 sm:flex-row" onsubmit={onRecoverySubmit} novalidate>
        <input
          type="email"
          class="w-full rounded-md border border-input bg-background px-3 py-2 text-base focus:outline-none focus:ring-2 focus:ring-ring"
          bind:value={recoveryEmailInput}
          placeholder="name@example.com"
          autocomplete="email"
          required
        />
        <button
          type="submit"
          class="inline-flex items-center justify-center rounded-md bg-fire px-4 py-2 text-sm font-medium text-fire-foreground transition-opacity hover:opacity-90 disabled:cursor-not-allowed disabled:opacity-60"
          disabled={recoverySending || identityState.recoveryEmailLoading}
        >
          {recoverySending || identityState.recoveryEmailLoading
            ? 'Sending...'
            : recoveryActionLabel()}
        </button>
      </form>
    </section>

    <section class="mt-6 rounded-md border border-border bg-muted p-4">
      <header class="mb-3 space-y-1">
        <h3 class="text-sm font-semibold">Blocked users</h3>
        <p class="text-sm text-muted-foreground">
          Manage users you have blocked. Blocking only affects your own view.
        </p>
      </header>

      {#if blockActionStatusMessage}
        <p class="mb-3 rounded-md border border-emerald-500/30 bg-emerald-500/10 p-3 text-sm text-emerald-300">
          {blockActionStatusMessage}
        </p>
      {/if}
      {#if blockActionErrorMessage}
        <p class="mb-3 rounded-md border border-destructive/30 bg-destructive/10 p-3 text-sm text-destructive" role="alert">
          {blockActionErrorMessage}
        </p>
      {/if}

      {#if blockedUsers.length === 0}
        <p class="text-sm text-muted-foreground">You have no blocked users.</p>
      {:else}
        <ul class="space-y-2">
          {#each blockedUsers as blockedUser}
            <li class="flex flex-col gap-2 rounded-md border border-border bg-card p-3 sm:flex-row sm:items-center sm:justify-between">
              <div class="min-w-0">
                <p class="truncate text-sm font-medium text-foreground">
                  {blockedUser.displayName}
                </p>
                <p class="truncate text-xs text-muted-foreground">
                  {#if blockedUser.username}
                    @{blockedUser.username} ·
                  {/if}
                  blocked {blockedAtLabel(blockedUser.blockedAt)}
                </p>
              </div>
              <button
                type="button"
                class="inline-flex items-center justify-center rounded-md border border-input px-3 py-1.5 text-xs font-medium text-foreground hover:bg-background disabled:cursor-not-allowed disabled:opacity-60"
                disabled={blockActionPendingUserId === blockedUser.userId}
                onclick={() =>
                  void unblockFromSettings(
                    blockedUser.userId,
                    blockedUser.displayName,
                  )}
              >
                {#if blockActionPendingUserId === blockedUser.userId}
                  Unblocking...
                {:else}
                  Unblock
                {/if}
              </button>
            </li>
          {/each}
        </ul>
      {/if}
    </section>
  </div>
{/if}
