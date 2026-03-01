<script lang="ts">
// biome-ignore lint/correctness/noUnusedImports: Used in Svelte markup; Biome doesn't detect template usage.
import { goto, Router } from '@mateothegreat/svelte5-router'
import { onMount } from 'svelte'
import { ApiError, getInstanceStatus, type InstanceStatus } from '$lib/api'

// biome-ignore lint/correctness/noUnusedImports: Used in Svelte markup; Biome doesn't detect template usage.
import SetupPage from '$lib/components/SetupPage.svelte'
import { channelState } from '$lib/features/channel/channelStore.svelte'
import { dmState } from '$lib/features/dm/dmStore.svelte'
import {
  getInviteMetadata,
  joinGuildByInvite,
} from '$lib/features/guild/guildApi'
import { guildState } from '$lib/features/guild/guildStore.svelte'
import type { InviteMetadata } from '$lib/features/guild/types'
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
  saveLastViewedChannel,
} from '$lib/features/identity/navigationState'
// biome-ignore lint/correctness/noUnusedImports: Used in Svelte markup; Biome doesn't detect template usage.
import RecoveryPrompt from '$lib/features/identity/RecoveryPrompt.svelte'
import { presenceState } from '$lib/features/members/presenceStore.svelte'
import {
  createAuthenticatedRoutes,
  isPersistableLocation,
  parsePersistedChannelLocation,
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

const INVALID_INVITE_MESSAGE = 'This invite link is invalid or has expired'
const UNREACHABLE_INVITE_MESSAGE =
  "This instance is currently unreachable. Your invite link will work when it's back online."

function readRecoveryTokenFromLocation(): string | null {
  if (typeof window === 'undefined') return null
  const token = new URLSearchParams(window.location.search)
    .get('recovery_token')
    ?.trim()
  return token || null
}

type InviteLocationContext = {
  inviteCode: string | null
  guildNameHint: string | null
}

function readInviteContextFromLocation(): InviteLocationContext {
  if (typeof window === 'undefined') {
    return { inviteCode: null, guildNameHint: null }
  }
  const params = new URLSearchParams(window.location.search)
  let inviteCode: string | null = null
  const inviteMatch = window.location.pathname.match(/^\/invite\/([^/]+)$/)
  if (inviteMatch?.[1]) {
    inviteCode = decodeURIComponent(inviteMatch[1]).trim() || null
  }
  if (!inviteCode) {
    inviteCode = params.get('invite')?.trim() || null
  }
  const guildNameHint =
    params.get('guild_name')?.trim() || params.get('guild')?.trim() || null
  return { inviteCode, guildNameHint }
}

function mapInviteErrorMessage(err: unknown): string {
  if (err instanceof ApiError) {
    if (
      err.message === INVALID_INVITE_MESSAGE ||
      err.code === 'VALIDATION_ERROR' ||
      err.code === 'NOT_FOUND'
    ) {
      return INVALID_INVITE_MESSAGE
    }
    return UNREACHABLE_INVITE_MESSAGE
  }
  return UNREACHABLE_INVITE_MESSAGE
}

const initialInviteContext = readInviteContextFromLocation()
let inviteCode = $state<string | null>(initialInviteContext.inviteCode)
// biome-ignore lint/correctness/noUnusedVariables: Used in Svelte markup; Biome doesn't detect template usage.
let inviteGuildNameHint = $state<string | null>(
  initialInviteContext.guildNameHint,
)
let inviteMetadata = $state<InviteMetadata | null>(null)
let inviteLoading = $state(false)
// biome-ignore lint/correctness/noUnusedVariables: Used in Svelte markup; Biome doesn't detect template usage.
let inviteErrorMessage = $state<string | null>(null)
let inviteJoining = $state(false)
let inviteJoinErrorMessage = $state<string | null>(null)
let inviteJoinAttemptedForCode = $state<string | null>(null)
let inviteWelcomeAcceptedForCode = $state<string | null>(null)
let shouldShowInviteWelcomeGate = $derived(
  Boolean(
    inviteCode &&
      inviteMetadata?.welcomeScreen.enabled &&
      inviteWelcomeAcceptedForCode !== inviteCode,
  ),
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
  const syncLocationState = () => {
    recoveryToken = readRecoveryTokenFromLocation()
    const nextInviteContext = readInviteContextFromLocation()
    const inviteChanged = inviteCode !== nextInviteContext.inviteCode
    inviteCode = nextInviteContext.inviteCode
    inviteGuildNameHint = nextInviteContext.guildNameHint
    if (inviteChanged) {
      inviteMetadata = null
      inviteLoading = false
      inviteErrorMessage = null
      inviteJoining = false
      inviteJoinErrorMessage = null
      inviteJoinAttemptedForCode = null
      inviteWelcomeAcceptedForCode = null
    }
  }
  syncLocationState()
  window.addEventListener('popstate', syncLocationState)
  return () => window.removeEventListener('popstate', syncLocationState)
})

$effect(() => {
  if (!status?.initialized || !inviteCode) {
    inviteLoading = false
    if (!inviteCode) {
      inviteMetadata = null
      inviteErrorMessage = null
      inviteJoining = false
      inviteJoinErrorMessage = null
      inviteJoinAttemptedForCode = null
    }
    return
  }

  let active = true
  const code = inviteCode
  inviteLoading = true
  inviteErrorMessage = null
  void getInviteMetadata(code)
    .then((metadata) => {
      if (!active || inviteCode !== code) return
      inviteMetadata = metadata
      inviteGuildNameHint = metadata.guildName
      inviteErrorMessage = null
    })
    .catch((err) => {
      if (!active || inviteCode !== code) return
      inviteMetadata = null
      inviteErrorMessage = mapInviteErrorMessage(err)
    })
    .finally(() => {
      if (!active || inviteCode !== code) return
      inviteLoading = false
    })

  return () => {
    active = false
  }
})

$effect(() => {
  if (
    !status?.initialized ||
    !identityState.session ||
    !inviteCode ||
    !inviteMetadata ||
    shouldShowInviteWelcomeGate ||
    inviteLoading ||
    inviteJoining
  ) {
    return
  }
  if (inviteJoinAttemptedForCode === inviteCode && inviteJoinErrorMessage) {
    return
  }

  let active = true
  const code = inviteCode
  inviteJoining = true
  inviteJoinErrorMessage = null
  inviteJoinAttemptedForCode = code

  void joinGuildByInvite(code)
    .then(async (joined) => {
      if (!active || inviteCode !== code) return
      await guildState.loadGuilds(true)
      if (!active || inviteCode !== code) return
      inviteCode = null
      inviteMetadata = null
      inviteGuildNameHint = null
      inviteErrorMessage = null
      inviteJoining = false
      inviteJoinErrorMessage = null
      inviteJoinAttemptedForCode = null
      inviteWelcomeAcceptedForCode = null
      await goto(`/${joined.guildSlug}/${joined.defaultChannelSlug}`)
    })
    .catch((err) => {
      if (!active || inviteCode !== code) return
      inviteJoinErrorMessage = mapInviteErrorMessage(err)
    })
    .finally(() => {
      if (!active || inviteCode !== code) return
      inviteJoining = false
    })

  return () => {
    active = false
  }
})

$effect(() => {
  if (identityState.session) return
  guildState.clear()
  channelState.clear()
  dmState.clearAll()
  initialRouteResolved = false
  shellBootstrapping = true
})

$effect(() => {
  if (
    !identityState.session ||
    typeof window === 'undefined' ||
    initialRouteResolved ||
    inviteCode
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

$effect(() => {
  const token = identityState.session?.token ?? null
  presenceState.ensureConnected(token)
})

// biome-ignore lint/correctness/noUnusedVariables: Used in Svelte markup; Biome doesn't detect template usage.
function handleShellRouteResolved(path: string): void {
  if (!identityState.session) return
  if (!isPersistableLocation(path)) return
  saveLastLocation(path)
  const channelLocation = parsePersistedChannelLocation(path)
  if (!channelLocation) return
  saveLastViewedChannel(channelLocation.guild, channelLocation.channel)
}

// biome-ignore lint/correctness/noUnusedVariables: Used in Svelte markup; Biome doesn't detect template usage.
function handleOpenSettings(): void {
  goto('/settings')
}

// biome-ignore lint/correctness/noUnusedVariables: Used in Svelte markup; Biome doesn't detect template usage.
function handleRetryInviteJoin(): void {
  inviteJoinErrorMessage = null
  inviteJoinAttemptedForCode = null
}

// biome-ignore lint/correctness/noUnusedVariables: Used in Svelte markup; Biome doesn't detect template usage.
function handleDismissInviteFlow(): void {
  inviteCode = null
  inviteMetadata = null
  inviteGuildNameHint = null
  inviteLoading = false
  inviteErrorMessage = null
  inviteJoining = false
  inviteJoinErrorMessage = null
  inviteJoinAttemptedForCode = null
  inviteWelcomeAcceptedForCode = null
  goto('/')
}

// biome-ignore lint/correctness/noUnusedVariables: Used in Svelte markup; Biome doesn't detect template usage.
function handleAcceptInviteWelcome(): void {
  if (!inviteCode) return
  inviteWelcomeAcceptedForCode = inviteCode
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
{:else if status && status.initialized && identityState.session && inviteCode}
  <main class="min-h-screen bg-background p-8">
    <div class="mx-auto w-full max-w-md space-y-4 rounded-lg border border-border bg-card p-6">
      {#if inviteLoading}
        <p class="text-sm text-muted-foreground">Checking invite…</p>
      {:else if inviteErrorMessage}
        <p class="text-sm text-destructive">{inviteErrorMessage}</p>
        <button
          type="button"
          class="inline-flex w-full items-center justify-center rounded-md bg-muted px-4 py-2 text-sm font-medium text-foreground transition-opacity hover:opacity-90"
          onclick={handleDismissInviteFlow}
        >
          Continue to app
        </button>
      {:else if inviteJoinErrorMessage}
        <p class="text-sm text-destructive">{inviteJoinErrorMessage}</p>
        <div class="flex gap-2">
          <button
            type="button"
            class="inline-flex flex-1 items-center justify-center rounded-md bg-fire px-4 py-2 text-sm font-medium text-fire-foreground transition-opacity hover:opacity-90"
            onclick={handleRetryInviteJoin}
          >
            Retry join
          </button>
          <button
            type="button"
            class="inline-flex flex-1 items-center justify-center rounded-md bg-muted px-4 py-2 text-sm font-medium text-foreground transition-opacity hover:opacity-90"
            onclick={handleDismissInviteFlow}
          >
            Cancel
          </button>
        </div>
      {:else if shouldShowInviteWelcomeGate}
        <h1 class="text-lg font-semibold text-foreground">
          {inviteMetadata?.welcomeScreen.title ?? 'Welcome'}
        </h1>
        {#if inviteMetadata?.welcomeScreen.rules}
          <p class="whitespace-pre-wrap text-sm text-muted-foreground">
            {inviteMetadata.welcomeScreen.rules}
          </p>
        {:else}
          <p class="text-sm text-muted-foreground">
            Please accept and continue to enter this guild.
          </p>
        {/if}
        <div class="flex gap-2">
          <button
            type="button"
            class="inline-flex flex-1 items-center justify-center rounded-md bg-fire px-4 py-2 text-sm font-medium text-fire-foreground transition-opacity hover:opacity-90"
            onclick={handleAcceptInviteWelcome}
          >
            {inviteMetadata?.welcomeScreen.acceptLabel ?? 'Accept & Continue'}
          </button>
          <button
            type="button"
            class="inline-flex flex-1 items-center justify-center rounded-md bg-muted px-4 py-2 text-sm font-medium text-foreground transition-opacity hover:opacity-90"
            onclick={handleDismissInviteFlow}
          >
            Cancel
          </button>
        </div>
      {:else}
        <div class="flex items-center gap-3">
          {#if inviteMetadata?.guildIconUrl}
            <img
              src={inviteMetadata.guildIconUrl}
              alt="Guild icon"
              class="h-10 w-10 rounded-full border border-border object-cover"
            />
          {/if}
          <div class="space-y-1">
            <p class="text-sm font-medium text-foreground">
              Joining {inviteMetadata?.guildName ?? inviteGuildNameHint ?? 'guild'}…
            </p>
            <p class="text-xs text-muted-foreground">
              Preparing your channel view.
            </p>
          </div>
        </div>
        <div class="h-2 w-full animate-pulse rounded bg-muted"></div>
      {/if}
    </div>
  </main>
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
    guildName={inviteMetadata?.guildName ?? inviteGuildNameHint}
    guildIconUrl={inviteMetadata?.guildIconUrl ?? null}
    inviteContextInvalid={Boolean(inviteCode) && Boolean(inviteErrorMessage)}
    username={identityState.identity?.username ?? ''}
    displayName={identityState.identity?.username ?? ''}
    avatarColor={identityState.identity?.avatarColor ?? null}
    joining={identityState.crossInstanceJoining}
    errorMessage={identityState.crossInstanceJoinError ?? inviteErrorMessage}
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
    inviteGuildName={inviteMetadata?.guildName ?? inviteGuildNameHint}
    inviteGuildIconUrl={inviteMetadata?.guildIconUrl ?? null}
    inviteErrorMessage={inviteErrorMessage}
    oncomplete={() => {
      reRegistering = false
      showIdentityRecovery = false
      if (!inviteCode) goto('/')
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
      inviteGuildName={inviteMetadata?.guildName ?? inviteGuildNameHint}
      inviteGuildIconUrl={inviteMetadata?.guildIconUrl ?? null}
      inviteErrorMessage={inviteErrorMessage}
      onrecover={() => {
        showIdentityRecovery = true
      }}
      oncomplete={() => {
        showIdentityRecovery = false
        if (!inviteCode) goto('/')
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
