<script lang="ts">
import { tick } from 'svelte'
// biome-ignore lint/correctness/noUnusedImports: Used in Svelte markup; Biome doesn't detect template usage.
import AdminPanel from '$lib/components/AdminPanel.svelte'
// biome-ignore lint/correctness/noUnusedImports: Used in Svelte markup; Biome doesn't detect template usage.
import ProfileSettingsView from '$lib/features/identity/ProfileSettingsView.svelte'

type Props = {
  mode: 'home' | 'channel' | 'settings' | 'admin'
  activeGuild: string
  activeChannel: string
  displayName: string
  isAdmin: boolean
  showRecoveryNudge: boolean
  onOpenSettings?: () => void | Promise<void>
  onDismissRecoveryNudge?: () => void | Promise<void>
}

let {
  mode,
  activeGuild,
  activeChannel,
  displayName,
  isAdmin,
  showRecoveryNudge,
  onOpenSettings,
  onDismissRecoveryNudge,
}: Props = $props()

// biome-ignore lint/correctness/noUnusedVariables: Used in Svelte markup; Biome doesn't detect template usage.
let detailText = $derived(
  mode === 'channel'
    ? `#${activeChannel} in ${activeGuild}`
    : `Signed in as ${displayName}.`,
)
// biome-ignore lint/correctness/noUnusedVariables: Used in Svelte markup; Biome doesn't detect template usage.
let canShowAdminPanel = $derived(mode === 'admin' && isAdmin)
// biome-ignore lint/correctness/noUnusedVariables: Used in Svelte markup; Biome doesn't detect template usage.
let shouldShowRecoveryNudge = $derived(
  showRecoveryNudge && (mode === 'home' || mode === 'channel'),
)
let composerInput = $state<HTMLInputElement | null>(null)

// biome-ignore lint/correctness/noUnusedVariables: Used in Svelte markup; Biome doesn't detect template usage.
async function handleOpenSettings() {
  await onOpenSettings?.()
}

// biome-ignore lint/correctness/noUnusedVariables: Used in Svelte markup; Biome doesn't detect template usage.
async function handleDismissRecoveryNudge() {
  await onDismissRecoveryNudge?.()
}

$effect(() => {
  if (mode !== 'channel') return
  activeGuild
  activeChannel
  void tick().then(() => {
    composerInput?.focus()
  })
})
</script>

{#if mode === 'admin'}
  {#if canShowAdminPanel}
    <AdminPanel />
  {:else}
    <section class="p-6">
      <p class="rounded-md border border-destructive/30 bg-destructive/10 p-3 text-sm text-destructive">
        Admin access is only available to the instance administrator.
      </p>
    </section>
  {/if}
{:else if mode === 'settings'}
  <ProfileSettingsView />
{:else}
  <section class="flex h-full flex-col gap-4 p-4 md:p-6">
    <header class="space-y-1">
      <h1 class="text-2xl font-semibold tracking-tight">Messages</h1>
      <p class="text-sm text-muted-foreground">{detailText}</p>
    </header>

    {#if shouldShowRecoveryNudge}
      <section class="rounded-md border border-border bg-muted p-4">
        <p class="text-sm font-medium text-foreground">
          Add a recovery email to protect this identity.
        </p>
        <p class="mt-1 text-sm text-muted-foreground">
          Optional, and only shown after your first successful session.
        </p>
        <div class="mt-3 flex flex-wrap gap-2">
          <button
            type="button"
            class="inline-flex items-center justify-center rounded-md bg-fire px-3 py-2 text-sm font-medium text-fire-foreground transition-opacity hover:opacity-90"
            onclick={() => void handleOpenSettings()}
          >
            Set up recovery email
          </button>
          <button
            type="button"
            class="inline-flex items-center justify-center rounded-md bg-background px-3 py-2 text-sm font-medium text-foreground transition-opacity hover:opacity-90"
            onclick={() => void handleDismissRecoveryNudge()}
          >
            Not now
          </button>
        </div>
      </section>
    {/if}

    <section class="rounded-md border border-border bg-card p-4">
      <h2 class="text-sm font-medium text-foreground">Channel Timeline</h2>
      <ul class="mt-3 space-y-2 text-sm text-muted-foreground">
        <li class="rounded-md bg-muted px-3 py-2">Welcome to Discool.</li>
        <li class="rounded-md bg-muted px-3 py-2">
          Placeholder messages will be replaced in upcoming stories.
        </li>
        <li class="rounded-md bg-muted px-3 py-2">
          Current route: /{activeGuild}/{activeChannel}
        </li>
      </ul>
    </section>

    {#if mode === 'channel'}
      <section class="rounded-md border border-border bg-card p-4">
        <label
          for="message-composer"
          class="mb-2 block text-sm font-medium text-foreground"
        >
          Message
        </label>
        <input
          id="message-composer"
          data-testid="message-composer-input"
          class="w-full rounded-md border border-input bg-background px-3 py-2 text-sm text-foreground placeholder:text-muted-foreground focus:outline-none focus:ring-2 focus:ring-ring"
          type="text"
          placeholder={`Message #${activeChannel}`}
          bind:this={composerInput}
        />
      </section>
    {/if}
  </section>
{/if}
