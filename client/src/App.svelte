<script lang="ts">
// biome-ignore lint/correctness/noUnusedImports: Used in Svelte markup; Biome doesn't detect template usage.
import { goto, Router } from '@mateothegreat/svelte5-router'
import { onMount } from 'svelte'
import { ApiError, getInstanceStatus, type InstanceStatus } from '$lib/api'

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
import RecoveryPrompt from '$lib/features/identity/RecoveryPrompt.svelte'
import {
  createAuthenticatedRoutes,
  isPersistableLocation,
  resolveInitialLocation,
} from './routes/routes'

// biome-ignore lint/correctness/noUnusedVariables: Used in Svelte markup; Biome doesn't detect template usage.
let loading = $state(true)
// biome-ignore lint/correctness/noUnusedVariables: Used in Svelte markup; Biome doesn't detect template usage.
let errorMessage = $state<string | null>(null)
let status = $state<InstanceStatus | null>(null)
// biome-ignore lint/correctness/noUnusedVariables: Used in Svelte markup; Biome doesn't detect template usage.
let reRegistering = $state(false)
// biome-ignore lint/correctness/noUnusedVariables: Used in Svelte markup; Biome doesn't detect template usage.
let showIdentityRecovery = $state(false)
let initialRouteResolved = $state(false)
// biome-ignore lint/correctness/noUnusedVariables: Used in Svelte markup; Biome doesn't detect template usage.
let shellBootstrapping = $state(true)

function readRecoveryTokenFromLocation(): string | null {
  if (typeof window === 'undefined') return null
  const token = new URLSearchParams(window.location.search)
    .get('recovery_token')
    ?.trim()
  return token || null
}

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

let isAdminUser = $derived(
  Boolean(
    status?.admin &&
      identityState.session &&
      status.admin.username === identityState.session.user.username,
  ),
)
// biome-ignore lint/correctness/noUnusedVariables: Used in Svelte markup; Biome doesn't detect template usage.
let authenticatedRoutes = $derived(createAuthenticatedRoutes(isAdminUser))

$effect(() => {
  if (typeof window === 'undefined') return
  const syncRecoveryToken = () => {
    recoveryToken = readRecoveryTokenFromLocation()
  }
  window.addEventListener('popstate', syncRecoveryToken)
  return () => window.removeEventListener('popstate', syncRecoveryToken)
})

$effect(() => {
  if (identityState.session) return
  initialRouteResolved = false
  shellBootstrapping = true
})

$effect(() => {
  if (
    !identityState.session ||
    typeof window === 'undefined' ||
    initialRouteResolved
  )
    return

  shellBootstrapping = true
  const targetPath = resolveInitialLocation(
    window.location.pathname,
    getLastLocation(),
  )
  if (targetPath !== window.location.pathname) {
    goto(targetPath)
  }

  initialRouteResolved = true
  const timer = window.setTimeout(() => {
    shellBootstrapping = false
  }, 120)
  return () => window.clearTimeout(timer)
})

$effect(() => {
  if (!identityState.session || typeof window === 'undefined' || isAdminUser)
    return
  if (window.location.pathname.startsWith('/admin')) {
    goto('/')
  }
})

// biome-ignore lint/correctness/noUnusedVariables: Used in Svelte markup; Biome doesn't detect template usage.
function handleShellRouteResolved(path: string): void {
  if (!identityState.session) return
  if (!isPersistableLocation(path)) return
  saveLastLocation(path)
}

// biome-ignore lint/correctness/noUnusedVariables: Used in Svelte markup; Biome doesn't detect template usage.
function handleOpenSettings(): void {
  goto('/settings')
}

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
  {#if shellBootstrapping}
    <main class="min-h-screen bg-background p-8">
      <div class="mx-auto w-full max-w-2xl space-y-4 rounded-lg border border-border bg-card p-6">
        <p class="text-sm text-muted-foreground">Loading workspace...</p>
        <div class="h-2 w-full animate-pulse rounded bg-muted"></div>
        <div class="h-2 w-11/12 animate-pulse rounded bg-muted"></div>
        <div class="h-2 w-4/5 animate-pulse rounded bg-muted"></div>
      </div>
    </main>
  {:else}
    <Router
      routes={authenticatedRoutes}
      isAdmin={isAdminUser}
      displayName={identityState.session.user.displayName}
      showRecoveryNudge={showRecoveryNudge}
      onOpenSettings={handleOpenSettings}
      onDismissRecoveryNudge={() => identityState.dismissRecoveryNudge()}
      onLogout={() => identityState.logout()}
      onRouteResolved={handleShellRouteResolved}
    />
  {/if}
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
      goto('/')
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
        goto('/')
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
