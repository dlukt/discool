<script lang="ts">
import { onMount } from 'svelte'
import { ApiError, getInstanceStatus, type InstanceStatus } from '$lib/api'

// biome-ignore lint/correctness/noUnusedImports: Used in Svelte markup; Biome doesn't detect template usage.
import AdminPanel from '$lib/components/AdminPanel.svelte'
// biome-ignore lint/correctness/noUnusedImports: Used in Svelte markup; Biome doesn't detect template usage.
import SetupPage from '$lib/components/SetupPage.svelte'
// biome-ignore lint/correctness/noUnusedImports: Used in Svelte markup; Biome doesn't detect template usage.
import CrossInstanceJoinPrompt from '$lib/features/identity/CrossInstanceJoinPrompt.svelte'
// biome-ignore lint/correctness/noUnusedImports: Used in Svelte markup; Biome doesn't detect template usage.
import IdentityRecoveryView from '$lib/features/identity/IdentityRecoveryView.svelte'
import { identityState } from '$lib/features/identity/identityStore.svelte'
// biome-ignore lint/correctness/noUnusedImports: Used in Svelte markup; Biome doesn't detect template usage.
import LoginView from '$lib/features/identity/LoginView.svelte'
import {
  getLastLocation,
  saveLastLocation,
} from '$lib/features/identity/navigationState'
// biome-ignore lint/correctness/noUnusedImports: Used in Svelte markup; Biome doesn't detect template usage.
import ProfileSettingsView from '$lib/features/identity/ProfileSettingsView.svelte'
// biome-ignore lint/correctness/noUnusedImports: Used in Svelte markup; Biome doesn't detect template usage.
import RecoveryPrompt from '$lib/features/identity/RecoveryPrompt.svelte'

// biome-ignore lint/correctness/noUnusedVariables: Used in Svelte markup; Biome doesn't detect template usage.
let loading = $state(true)
// biome-ignore lint/correctness/noUnusedVariables: Used in Svelte markup; Biome doesn't detect template usage.
let errorMessage = $state<string | null>(null)
let status = $state<InstanceStatus | null>(null)
let view = $state<'home' | 'admin' | 'settings'>('home')
// biome-ignore lint/correctness/noUnusedVariables: Used in Svelte markup; Biome doesn't detect template usage.
let reRegistering = $state(false)
// biome-ignore lint/correctness/noUnusedVariables: Used in Svelte markup; Biome doesn't detect template usage.
let showIdentityRecovery = $state(false)

function readRecoveryTokenFromLocation(): string | null {
  if (typeof window === 'undefined') return null
  const token = new URLSearchParams(window.location.search)
    .get('recovery_token')
    ?.trim()
  return token || null
}

let currentPath = $derived(
  typeof window !== 'undefined'
    ? window.location.pathname
    : view === 'admin'
      ? '/admin'
      : '/',
)
// biome-ignore lint/correctness/noUnusedVariables: Reserved for Epic 4 router integration.
let lastLocation = $state<string | null>(null)
let joinGuildName = $derived(
  (() => {
    if (typeof window === 'undefined') return null
    const params = new URLSearchParams(window.location.search)
    const guildName =
      params.get('guild_name')?.trim() || params.get('guild')?.trim() || ''
    return guildName || null
  })(),
)

// biome-ignore lint/correctness/noUnusedVariables: Used in Svelte markup; Biome doesn't detect template usage.
let showRecoveryNudge = $derived(
  Boolean(
    identityState.session &&
      view === 'home' &&
      !identityState.recoveryEmailLoading &&
      !identityState.recoveryNudgeDismissed &&
      !identityState.recoveryEmailStatus?.associated,
  ),
)
// biome-ignore lint/correctness/noUnusedVariables: Used in Svelte markup; Biome doesn't detect template usage.
let inviteContextInvalid = $derived(
  (() => {
    if (typeof window === 'undefined') return false
    const params = new URLSearchParams(window.location.search)
    return Boolean(params.get('invite')?.trim()) && !joinGuildName
  })(),
)
// biome-ignore lint/correctness/noUnusedVariables: Used in Svelte markup; Biome doesn't detect template usage.
let recoveryToken = $state(readRecoveryTokenFromLocation())

$effect(() => {
  if (typeof window === 'undefined') return
  const syncRecoveryToken = () => {
    recoveryToken = readRecoveryTokenFromLocation()
  }
  window.addEventListener('popstate', syncRecoveryToken)
  return () => window.removeEventListener('popstate', syncRecoveryToken)
})

$effect(() => {
  const adminUsername = status?.admin?.username
  const currentUsername = identityState.session?.user.username
  if (
    view === 'admin' &&
    adminUsername &&
    currentUsername &&
    adminUsername !== currentUsername
  ) {
    view = 'home'
  }
})

$effect(() => {
  if (!identityState.session) return
  if (
    currentPath === '/' ||
    currentPath.startsWith('/admin') ||
    currentPath.startsWith('/settings')
  )
    return
  saveLastLocation(currentPath)
})

$effect(() => {
  if (!identityState.session) {
    lastLocation = null
    return
  }

  const stored = getLastLocation()
  if (
    !stored ||
    stored === '/' ||
    stored.startsWith('/admin') ||
    stored.startsWith('/settings')
  ) {
    lastLocation = null
    return
  }

  lastLocation = stored
})

async function loadStatus() {
  loading = true
  errorMessage = null
  try {
    status = await getInstanceStatus()
    if (status.initialized) {
      await identityState.initialize()
    }
  } catch (err) {
    status = null
    if (err instanceof ApiError) {
      errorMessage = err.message
    } else if (err instanceof Error) {
      errorMessage = err.message
    } else {
      errorMessage = 'Could not connect to the server. Is it running?'
    }
  } finally {
    loading = false
  }
}

// biome-ignore lint/correctness/noUnusedVariables: Used in Svelte markup; Biome doesn't detect template usage.
async function handleRecoveryPromptRecover() {
  showIdentityRecovery = true
}

onMount(() => {
  void loadStatus()
})
</script>

{#if loading}
  <main class="min-h-screen bg-background p-8">
    <p class="text-center text-sm text-muted-foreground">Loading...</p>
  </main>
{:else if errorMessage}
  <main class="min-h-screen bg-background p-8">
    <div class="mx-auto w-full max-w-md space-y-4 rounded-lg border border-border bg-card p-6">
      <p class="text-sm text-destructive">{errorMessage}</p>
      <button
        type="button"
        class="inline-flex w-full items-center justify-center rounded-md bg-fire px-4 py-2 text-sm font-medium text-fire-foreground transition-opacity hover:opacity-90"
        onclick={() => void loadStatus()}
      >
        Retry
      </button>
    </div>
  </main>
{:else if status && !status.initialized}
  <SetupPage on:complete={() => void loadStatus()} />
{:else if status && status.initialized && identityState.session}
  <main class="min-h-screen bg-background">
    <div class="flex min-h-screen">
      <aside class="w-56 border-r border-border bg-sidebar p-4">
        <div class="mb-4 text-sm font-semibold text-sidebar-foreground">Discool</div>
        <nav class="space-y-1">
          <button
            type="button"
            class={`w-full rounded-md px-3 py-2 text-left text-sm transition-colors ${
              view === 'home'
                ? 'bg-sidebar-accent text-sidebar-accent-foreground'
                : 'text-muted-foreground hover:bg-muted'
            }`}
            onclick={() => (view = 'home')}
          >
            Home
          </button>
          {#if status.admin && status.admin.username === identityState.session.user.username}
            <button
              type="button"
              class={`w-full rounded-md px-3 py-2 text-left text-sm transition-colors ${
                view === 'admin'
                  ? 'bg-sidebar-accent text-sidebar-accent-foreground'
                  : 'text-muted-foreground hover:bg-muted'
              }`}
              onclick={() => (view = 'admin')}
              >
                Admin
              </button>
          {/if}
          <button
            type="button"
            class={`w-full rounded-md px-3 py-2 text-left text-sm transition-colors ${
              view === 'settings'
                ? 'bg-sidebar-accent text-sidebar-accent-foreground'
                : 'text-muted-foreground hover:bg-muted'
            }`}
            onclick={() => (view = 'settings')}
          >
            Settings
          </button>
        </nav>

        <div class="mt-6 border-t border-border pt-4">
          <button
            type="button"
            class="inline-flex w-full items-center justify-center rounded-md bg-destructive px-3 py-2 text-sm font-medium text-destructive-foreground transition-opacity hover:opacity-90"
            onclick={() => void identityState.logout()}
          >
            Log out
          </button>
        </div>
      </aside>

      <section class="flex-1 p-8">
        {#if view === 'admin'}
          <AdminPanel />
        {:else if view === 'settings'}
          <ProfileSettingsView />
        {:else}
          <div class="mx-auto flex w-full max-w-xl flex-col gap-6 rounded-lg border border-border bg-card p-8">
            <header class="space-y-2">
              <h1 class="text-4xl font-semibold tracking-tight">Discool</h1>
              <p class="text-sm text-muted-foreground">
                Signed in as {identityState.session.user.displayName}.
              </p>
              <p class="text-sm text-muted-foreground">
                Dual Core theme scaffold (Ice navigation, Fire actions, Zinc foundation).
              </p>
            </header>

            {#if showRecoveryNudge}
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
                    onclick={() => {
                      identityState.recoveryNudgeDismissed = false
                      view = 'settings'
                    }}
                  >
                    Set up recovery email
                  </button>
                  <button
                    type="button"
                    class="inline-flex items-center justify-center rounded-md bg-background px-3 py-2 text-sm font-medium text-foreground transition-opacity hover:opacity-90"
                    onclick={() => identityState.dismissRecoveryNudge()}
                  >
                    Not now
                  </button>
                </div>
              </section>
            {/if}

            <section class="flex flex-wrap gap-3">
              <button
                type="button"
                class="inline-flex items-center justify-center rounded-md bg-primary px-4 py-2 text-sm font-medium text-primary-foreground transition-colors hover:bg-accent hover:text-accent-foreground"
              >
                Ice action
              </button>

              <button
                type="button"
                class="inline-flex items-center justify-center rounded-md bg-fire px-4 py-2 text-sm font-medium text-fire-foreground transition-opacity hover:opacity-90"
              >
                Fire action
              </button>

              <button
                type="button"
                class="inline-flex items-center justify-center rounded-md bg-destructive px-4 py-2 text-sm font-medium text-destructive-foreground transition-opacity hover:opacity-90"
              >
                Destructive
              </button>
            </section>

            <section class="rounded-md bg-muted p-4 text-sm text-muted-foreground">
              Try inspecting CSS variables in <code class="text-foreground">src/app.css</code>.
            </section>
          </div>
        {/if}
      </section>
    </div>
  </main>
{:else if status && status.initialized && identityState.identityCorrupted}
  {#if showIdentityRecovery || recoveryToken}
    <IdentityRecoveryView
      token={recoveryToken}
      oncleartoken={() => {
        recoveryToken = readRecoveryTokenFromLocation()
      }}
      oncancel={async () => {
        showIdentityRecovery = false
        await identityState.clear()
      }}
    />
  {:else}
    <RecoveryPrompt
      onstartfresh={() => identityState.clear()}
      onrecover={handleRecoveryPromptRecover}
    />
  {/if}
{:else if status && status.initialized && identityState.identityNotRegistered && !reRegistering}
  <CrossInstanceJoinPrompt
    guildName={joinGuildName}
    inviteContextInvalid={inviteContextInvalid}
    username={identityState.identity?.username ?? ''}
    displayName={identityState.identity?.username ?? ''}
    avatarColor={identityState.identity?.avatarColor ?? null}
    joining={identityState.crossInstanceJoining}
    errorMessage={identityState.crossInstanceJoinError}
    onconfirm={() => void identityState.authenticateCrossInstance()}
    onusedifferentname={() => {
      identityState.identityNotRegistered = false
      identityState.crossInstanceJoinError = null
      reRegistering = true
    }}
  />
{:else if status && status.initialized && reRegistering}
  <LoginView
    mode="reregister"
    oncomplete={() => {
      reRegistering = false
      showIdentityRecovery = false
      view = 'home'
    }}
  />
{:else if status && status.initialized && !identityState.identity}
  {#if showIdentityRecovery || recoveryToken}
    <IdentityRecoveryView
      token={recoveryToken}
      oncleartoken={() => {
        recoveryToken = readRecoveryTokenFromLocation()
      }}
      oncancel={() => {
        showIdentityRecovery = false
      }}
    />
  {:else}
    <LoginView
      mode="create"
      onrecover={() => {
        showIdentityRecovery = true
      }}
      oncomplete={() => {
        showIdentityRecovery = false
        view = 'home'
      }}
    />
  {/if}
{:else if status && status.initialized && identityState.identity && identityState.authenticating}
  <main class="min-h-screen bg-background p-8">
    <div class="mx-auto w-full max-w-md space-y-4 rounded-lg border border-border bg-card p-6">
      <p class="text-center text-sm text-muted-foreground">Signing in...</p>
      <div class="h-2 w-full animate-pulse rounded bg-muted"></div>
      <div class="h-2 w-5/6 animate-pulse rounded bg-muted"></div>
      <div class="h-2 w-2/3 animate-pulse rounded bg-muted"></div>
    </div>
  </main>
{:else if status && status.initialized && identityState.identity && identityState.authError}
  <main class="min-h-screen bg-background p-8">
    <div class="mx-auto w-full max-w-md space-y-4 rounded-lg border border-border bg-card p-6">
      <p class="text-sm text-destructive">{identityState.authError}</p>
      <button
        type="button"
        class="inline-flex w-full items-center justify-center rounded-md bg-fire px-4 py-2 text-sm font-medium text-fire-foreground transition-opacity hover:opacity-90"
        onclick={() => void identityState.authenticate()}
      >
        {identityState.authError.startsWith('Signed out') ? 'Sign in' : 'Try again'}
      </button>
    </div>
  </main>
{:else}
  <main class="min-h-screen bg-background p-8">
    <p class="text-center text-sm text-muted-foreground">Signing in...</p>
  </main>
{/if}
