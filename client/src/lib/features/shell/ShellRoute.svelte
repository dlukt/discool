<script lang="ts">
import {
  type RouteResult,
  route as routerLink,
} from '@mateothegreat/svelte5-router'
import { onMount, tick } from 'svelte'

// biome-ignore lint/correctness/noUnusedImports: Used in Svelte markup; Biome doesn't detect template usage.
import ChannelList from '$lib/features/channel/ChannelList.svelte'
// biome-ignore lint/correctness/noUnusedImports: Used in Svelte markup; Biome doesn't detect template usage.
import MessageArea from '$lib/features/chat/MessageArea.svelte'
// biome-ignore lint/correctness/noUnusedImports: Used in Svelte markup; Biome doesn't detect template usage.
import GuildRail from '$lib/features/guild/GuildRail.svelte'
// biome-ignore lint/correctness/noUnusedImports: Used in Svelte markup; Biome doesn't detect template usage.
import GuildSettings from '$lib/features/guild/GuildSettings.svelte'
// biome-ignore lint/correctness/noUnusedImports: Used in Svelte markup; Biome doesn't detect template usage.
import MemberList from '$lib/features/members/MemberList.svelte'

type ShellMode = 'home' | 'channel' | 'settings' | 'admin'
type ViewportMode = 'mobile' | 'tablet' | 'desktop'
type MobilePanel = 'guilds' | 'channels' | 'messages' | 'members'

type Props = {
  route?: RouteResult
  mode: ShellMode
  isAdmin: boolean
  displayName: string
  showRecoveryNudge: boolean
  onOpenSettings?: () => void | Promise<void>
  onDismissRecoveryNudge?: () => void | Promise<void>
  onLogout?: () => void | Promise<void>
  onRouteResolved?: (path: string) => void
}

let {
  route,
  mode,
  isAdmin,
  displayName,
  showRecoveryNudge,
  onOpenSettings,
  onDismissRecoveryNudge,
  onLogout,
  onRouteResolved,
}: Props = $props()

// biome-ignore lint/correctness/noUnusedVariables: Used in Svelte markup; Biome doesn't detect template usage.
const routerAction = routerLink
let shellMode = $derived(mode)
// biome-ignore lint/correctness/noUnusedVariables: Used in Svelte markup; Biome doesn't detect template usage.
let canAccessAdmin = $derived(isAdmin)
// biome-ignore lint/correctness/noUnusedVariables: Used in Svelte markup; Biome doesn't detect template usage.
let shellDisplayName = $derived(displayName)
// biome-ignore lint/correctness/noUnusedVariables: Used in Svelte markup; Biome doesn't detect template usage.
let shellRecoveryNudge = $derived(showRecoveryNudge)

let viewport = $state<ViewportMode>('desktop')
let tabletMembersVisible = $state(false)
let mobilePanel = $state<MobilePanel>('messages')
// biome-ignore lint/correctness/noUnusedVariables: Used in Svelte markup; Biome doesn't detect template usage.
let guildSettingsOpen = $state(false)
let mainContent = $state<HTMLElement | null>(null)
let lastFocusedPath = $state<string | null>(null)
// biome-ignore lint/correctness/noUnusedVariables: Used in Svelte markup; Biome doesn't detect template usage.
let isTabletMembersVisible = $derived(tabletMembersVisible)
// biome-ignore lint/correctness/noUnusedVariables: Used in Svelte markup; Biome doesn't detect template usage.
let activeMobilePanel = $derived(mobilePanel)

function viewportFromWidth(width: number): ViewportMode {
  if (width < 768) return 'mobile'
  if (width < 1024) return 'tablet'
  return 'desktop'
}

function syncViewport() {
  if (typeof window === 'undefined') return
  viewport = viewportFromWidth(window.innerWidth)
}

onMount(() => {
  syncViewport()
  if (typeof window === 'undefined') return undefined
  window.addEventListener('resize', syncViewport)
  return () => window.removeEventListener('resize', syncViewport)
})

let currentPath = $derived(
  route?.result?.path?.original ??
    (typeof window !== 'undefined' ? window.location.pathname : '/'),
)
let params = $derived(
  (route?.result?.path?.params ?? {}) as Record<string, string | undefined>,
)
// biome-ignore lint/correctness/noUnusedVariables: Used in Svelte markup; Biome doesn't detect template usage.
let activeGuild = $derived(params.guild ?? 'lobby')
// biome-ignore lint/correctness/noUnusedVariables: Used in Svelte markup; Biome doesn't detect template usage.
let activeChannel = $derived(params.channel ?? 'general')
// biome-ignore lint/correctness/noUnusedVariables: Used in Svelte markup; Biome doesn't detect template usage.
let canOpenGuildSettings = $derived(shellMode === 'channel')

// biome-ignore lint/correctness/noUnusedVariables: Used in Svelte markup; Biome doesn't detect template usage.
function openMobilePanel(panel: MobilePanel) {
  mobilePanel = panel
}

// biome-ignore lint/correctness/noUnusedVariables: Used in Svelte markup; Biome doesn't detect template usage.
function toggleTabletMembers() {
  tabletMembersVisible = !tabletMembersVisible
}

// biome-ignore lint/correctness/noUnusedVariables: Used in Svelte markup; Biome doesn't detect template usage.
async function handleLogout() {
  await onLogout?.()
}

// biome-ignore lint/correctness/noUnusedVariables: Used in Svelte markup; Biome doesn't detect template usage.
async function handleOpenSettings() {
  await onOpenSettings?.()
}

// biome-ignore lint/correctness/noUnusedVariables: Used in Svelte markup; Biome doesn't detect template usage.
async function handleDismissRecoveryNudge() {
  await onDismissRecoveryNudge?.()
}

// biome-ignore lint/correctness/noUnusedVariables: Used in Svelte markup; Biome doesn't detect template usage.
function openGuildSettings() {
  guildSettingsOpen = true
}

// biome-ignore lint/correctness/noUnusedVariables: Used in Svelte markup; Biome doesn't detect template usage.
async function handleCloseGuildSettings() {
  guildSettingsOpen = false
}

$effect(() => {
  if (viewport !== 'tablet') tabletMembersVisible = false
})

$effect(() => {
  if (viewport !== 'mobile') mobilePanel = 'messages'
})

$effect(() => {
  if (!currentPath || currentPath === lastFocusedPath) return
  lastFocusedPath = currentPath
  onRouteResolved?.(currentPath)
  void tick().then(() => {
    mainContent?.focus()
  })
})
</script>

<a
  href="#main-content"
  class="sr-only focus:not-sr-only focus:fixed focus:left-4 focus:top-4 focus:z-50 focus:rounded-md focus:bg-background focus:px-3 focus:py-2 focus:text-sm focus:text-foreground focus:ring-2 focus:ring-primary"
>
  Skip to content
</a>

{#if viewport === 'mobile'}
  <div class="flex min-h-screen flex-col bg-background">
    <header class="border-b border-border px-3 py-2">
      <div class="flex items-center justify-between gap-2">
        <span class="text-sm font-semibold text-foreground">Discool</span>
        <div class="flex items-center gap-2">
          <a
            class="rounded-md bg-muted px-2 py-1 text-xs text-foreground"
            href="/"
            use:routerAction
          >
            Home
          </a>
          {#if canAccessAdmin}
            <a
              class="rounded-md bg-muted px-2 py-1 text-xs text-foreground"
              href="/admin"
              use:routerAction
            >
              Admin
            </a>
          {/if}
          <a
            class="rounded-md bg-muted px-2 py-1 text-xs text-foreground"
            href="/settings"
            use:routerAction
          >
            Settings
          </a>
          {#if canOpenGuildSettings}
            <button
              type="button"
              class="rounded-md bg-muted px-2 py-1 text-xs text-foreground"
              onclick={openGuildSettings}
            >
              Guild settings
            </button>
          {/if}
          <button
            type="button"
            class="rounded-md bg-destructive px-2 py-1 text-xs font-medium text-destructive-foreground"
            onclick={() => void handleLogout()}
          >
            Log out
          </button>
        </div>
      </div>
    </header>

    <main
      id="main-content"
      tabindex="-1"
      bind:this={mainContent}
      class="flex-1 overflow-auto outline-none"
    >
      {#if activeMobilePanel === 'guilds'}
        <GuildRail activeGuild={activeGuild} activeChannel={activeChannel} />
      {:else if activeMobilePanel === 'channels'}
        <ChannelList activeGuild={activeGuild} activeChannel={activeChannel} />
      {:else if activeMobilePanel === 'members'}
        <MemberList />
      {:else}
        <MessageArea
          mode={shellMode}
          activeGuild={activeGuild}
          activeChannel={activeChannel}
          displayName={shellDisplayName}
          isAdmin={canAccessAdmin}
          showRecoveryNudge={shellRecoveryNudge}
          onOpenSettings={handleOpenSettings}
          onDismissRecoveryNudge={handleDismissRecoveryNudge}
        />
      {/if}
    </main>

    <nav
      class="grid grid-cols-4 gap-1 border-t border-border bg-card p-2"
      aria-label="Mobile shell navigation"
    >
      <button
        type="button"
        class="rounded-md px-2 py-2 text-xs font-medium text-foreground hover:bg-muted"
        onclick={() => openMobilePanel('guilds')}
        aria-label="Guilds"
      >
        Guilds
      </button>
      <button
        type="button"
        class="rounded-md px-2 py-2 text-xs font-medium text-foreground hover:bg-muted"
        onclick={() => openMobilePanel('channels')}
        aria-label="Channels"
      >
        Channels
      </button>
      <button
        type="button"
        class="rounded-md px-2 py-2 text-xs font-medium text-foreground hover:bg-muted"
        onclick={() => openMobilePanel('messages')}
        aria-label="Messages"
      >
        Messages
      </button>
      <button
        type="button"
        class="rounded-md px-2 py-2 text-xs font-medium text-foreground hover:bg-muted"
        onclick={() => openMobilePanel('members')}
        aria-label="Members"
      >
        Members
      </button>
    </nav>
  </div>
{:else}
  <div class="relative flex min-h-screen bg-background">
    <aside class="w-[72px] shrink-0">
      <GuildRail activeGuild={activeGuild} activeChannel={activeChannel} />
    </aside>
    <aside class="w-[240px] shrink-0">
      <ChannelList activeGuild={activeGuild} activeChannel={activeChannel} />
    </aside>

    <main
      id="main-content"
      tabindex="-1"
      bind:this={mainContent}
      class="flex min-w-0 flex-1 flex-col outline-none"
    >
      <header class="flex flex-wrap items-center justify-between gap-2 border-b border-border px-4 py-3">
        <p class="text-sm text-muted-foreground">{activeGuild} / {activeChannel}</p>
        <div class="flex items-center gap-2">
          {#if viewport === 'tablet'}
            <button
              type="button"
              class="rounded-md bg-muted px-3 py-2 text-xs font-medium text-foreground"
              onclick={toggleTabletMembers}
              aria-label="Toggle members"
            >
              Members
            </button>
          {/if}
          <a
            class="rounded-md bg-muted px-3 py-2 text-xs font-medium text-foreground"
            href="/"
            use:routerAction
          >
            Home
          </a>
          {#if canAccessAdmin}
            <a
              class="rounded-md bg-muted px-3 py-2 text-xs font-medium text-foreground"
              href="/admin"
              use:routerAction
            >
              Admin
            </a>
          {/if}
          <a
            class="rounded-md bg-muted px-3 py-2 text-xs font-medium text-foreground"
            href="/settings"
            use:routerAction
          >
            Settings
          </a>
          {#if canOpenGuildSettings}
            <button
              type="button"
              class="rounded-md bg-muted px-3 py-2 text-xs font-medium text-foreground"
              onclick={openGuildSettings}
            >
              Guild settings
            </button>
          {/if}
          <button
            type="button"
            class="rounded-md bg-destructive px-3 py-2 text-xs font-medium text-destructive-foreground"
            onclick={() => void handleLogout()}
          >
            Log out
          </button>
        </div>
      </header>

      <div class="min-h-0 flex-1 overflow-auto">
        <MessageArea
          mode={shellMode}
          activeGuild={activeGuild}
          activeChannel={activeChannel}
          displayName={shellDisplayName}
          isAdmin={canAccessAdmin}
          showRecoveryNudge={shellRecoveryNudge}
          onOpenSettings={handleOpenSettings}
          onDismissRecoveryNudge={handleDismissRecoveryNudge}
        />
      </div>
    </main>

    {#if viewport === 'desktop'}
      <aside class="w-[240px] shrink-0 border-l border-border">
        <MemberList />
      </aside>
    {:else if isTabletMembersVisible}
      <aside
        class="absolute inset-y-0 right-0 z-10 w-[240px] border-l border-border bg-card shadow-2xl"
        data-testid="tablet-member-list"
      >
        <MemberList />
      </aside>
    {/if}
  </div>
{/if}

<GuildSettings
  open={guildSettingsOpen}
  guildSlug={activeGuild}
  onClose={handleCloseGuildSettings}
/>
