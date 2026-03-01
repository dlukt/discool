<script lang="ts">
// biome-ignore-all lint/correctness/noUnusedVariables: Svelte template usage isn't detected reliably.
import {
  goto,
  type RouteResult,
  route as routerLink,
} from '@mateothegreat/svelte5-router'
import { onMount, tick } from 'svelte'

// biome-ignore lint/correctness/noUnusedImports: Used in Svelte markup; Biome doesn't detect template usage.
import ChannelList from '$lib/features/channel/ChannelList.svelte'
import { channelState } from '$lib/features/channel/channelStore.svelte'
// biome-ignore lint/correctness/noUnusedImports: Used in Svelte markup; Biome doesn't detect template usage.
import MessageArea from '$lib/features/chat/MessageArea.svelte'
import { messageState } from '$lib/features/chat/messageStore.svelte'
import { dmState } from '$lib/features/dm/dmStore.svelte'
// biome-ignore lint/correctness/noUnusedImports: Used in Svelte markup; Biome doesn't detect template usage.
import GuildRail from '$lib/features/guild/GuildRail.svelte'
// biome-ignore lint/correctness/noUnusedImports: Used in Svelte markup; Biome doesn't detect template usage.
import GuildSettings from '$lib/features/guild/GuildSettings.svelte'
import { guildState } from '$lib/features/guild/guildStore.svelte'
// biome-ignore lint/correctness/noUnusedImports: Used in Svelte markup; Biome doesn't detect template usage.
import InviteModal from '$lib/features/guild/InviteModal.svelte'
// biome-ignore lint/correctness/noUnusedImports: Used in Svelte markup; Biome doesn't detect template usage.
import MemberList from '$lib/features/members/MemberList.svelte'
import { presenceState } from '$lib/features/members/presenceStore.svelte'
import { wsClient } from '$lib/ws/client'

type ShellMode = 'home' | 'channel' | 'dm' | 'settings' | 'admin'
type ViewportMode = 'mobile' | 'tablet' | 'desktop'
type MobilePanel = 'guilds' | 'channels' | 'messages' | 'members'
type QuickSwitcherSection = 'DM' | 'Channel'
type QuickSwitcherResult = {
  id: string
  section: QuickSwitcherSection
  label: string
  description: string
  path: string
}

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

const routerAction = routerLink
let shellMode = $derived(mode)
let canAccessAdmin = $derived(isAdmin)
let shellDisplayName = $derived(displayName)
let shellRecoveryNudge = $derived(showRecoveryNudge)

let viewport = $state<ViewportMode>('desktop')
let tabletMembersVisible = $state(false)
let mobilePanel = $state<MobilePanel>('messages')
let guildSettingsOpen = $state(false)
let inviteModalOpen = $state(false)
let mainContent = $state<HTMLElement | null>(null)
let lastFocusedPath = $state<string | null>(null)
let quickSwitcherOpen = $state(false)
let quickSwitcherQuery = $state('')
let quickSwitcherOpenedPath = $state<string | null>(null)
let wsLifecycle = $state(wsClient.getLifecycleState())
let isTabletMembersVisible = $derived(tabletMembersVisible)
let activeMobilePanel = $derived(mobilePanel)
let showReconnectingStatus = $derived(wsLifecycle === 'reconnecting')

function viewportFromWidth(width: number): ViewportMode {
  if (width < 768) return 'mobile'
  if (width < 1024) return 'tablet'
  return 'desktop'
}

function syncViewport() {
  if (typeof window === 'undefined') return
  viewport = viewportFromWidth(window.innerWidth)
}

async function navigateUnreadChannel(direction: 'up' | 'down'): Promise<void> {
  const unreadSlugs = messageState.unreadChannelSlugsForGuild(activeGuild)
  if (unreadSlugs.length === 0) return
  const currentIndex = unreadSlugs.indexOf(activeChannel)
  let nextIndex = 0
  if (currentIndex >= 0) {
    const delta = direction === 'down' ? 1 : -1
    nextIndex = (currentIndex + delta + unreadSlugs.length) % unreadSlugs.length
  } else {
    nextIndex = direction === 'down' ? 0 : unreadSlugs.length - 1
  }
  const nextChannel = unreadSlugs[nextIndex]
  if (!nextChannel || nextChannel === activeChannel) return
  await goto(`/${activeGuild}/${nextChannel}`)
}

onMount(() => {
  syncViewport()
  void dmState.ensureLoaded().catch(() => {})
  const unsubscribeLifecycle = wsClient.subscribeLifecycle((state) => {
    wsLifecycle = state
  })
  if (typeof window === 'undefined') return undefined
  const handleUnreadHotkey = (event: KeyboardEvent) => {
    if (!event.altKey || !event.shiftKey || event.ctrlKey || event.metaKey)
      return
    if (event.key === 'ArrowDown') {
      event.preventDefault()
      void navigateUnreadChannel('down')
      return
    }
    if (event.key === 'ArrowUp') {
      event.preventDefault()
      void navigateUnreadChannel('up')
    }
  }
  const handleQuickSwitcherHotkey = (event: KeyboardEvent) => {
    if (
      (event.ctrlKey || event.metaKey) &&
      !event.altKey &&
      event.key.toLowerCase() === 'k'
    ) {
      event.preventDefault()
      quickSwitcherOpen = !quickSwitcherOpen
      if (!quickSwitcherOpen) {
        quickSwitcherQuery = ''
      }
    }
  }
  const handleOpenDmIntent = (
    event: Event & { detail?: { userId?: unknown } },
  ) => {
    const detail = event.detail
    if (!detail || typeof detail.userId !== 'string') return
    void dmState
      .openOrCreateDm(detail.userId)
      .then((conversation) => goto(`/dm/${conversation.dmSlug}`))
      .catch(() => {})
  }
  window.addEventListener('resize', syncViewport)
  window.addEventListener('keydown', handleUnreadHotkey)
  window.addEventListener('keydown', handleQuickSwitcherHotkey)
  window.addEventListener(
    'discool:open-dm-intent',
    handleOpenDmIntent as EventListener,
  )
  return () => {
    unsubscribeLifecycle()
    window.removeEventListener('resize', syncViewport)
    window.removeEventListener('keydown', handleUnreadHotkey)
    window.removeEventListener('keydown', handleQuickSwitcherHotkey)
    window.removeEventListener(
      'discool:open-dm-intent',
      handleOpenDmIntent as EventListener,
    )
  }
})

let currentPath = $derived(
  route?.result?.path?.original ??
    (typeof window !== 'undefined' ? window.location.pathname : '/'),
)
let params = $derived(
  (route?.result?.path?.params ?? {}) as Record<string, string | undefined>,
)
let activeGuild = $derived(params.guild ?? 'lobby')
let activeChannel = $derived(params.channel ?? 'general')
let activeDm = $derived(params.dm ?? null)
let canOpenGuildSettings = $derived(shellMode === 'channel')
let canOpenInvites = $derived(shellMode === 'channel')
let quickQueryNormalized = $derived(quickSwitcherQuery.trim().toLowerCase())
let quickDmResults = $derived.by(() => {
  const conversations = dmState.conversations
  const matches = conversations
    .map((conversation) => ({
      id: `dm:${conversation.dmSlug}`,
      section: 'DM' as const,
      label: conversation.participant.displayName,
      description: `@${conversation.participant.username}`,
      path: `/dm/${conversation.dmSlug}`,
    }))
    .filter((result) => {
      if (!quickQueryNormalized) return true
      const haystack = `${result.label} ${result.description}`.toLowerCase()
      return haystack.includes(quickQueryNormalized)
    })
  return matches
})
let quickChannelResults = $derived.by(() => {
  const guilds = guildState.guilds
  const guildEntries = guilds.map((guild) => {
    const channelSlug =
      guild.lastViewedChannelSlug ??
      channelState.orderedChannelsForGuild(guild.slug)[0]?.slug ??
      guild.defaultChannelSlug
    return {
      id: `channel:${guild.slug}:${channelSlug}`,
      section: 'Channel' as const,
      label: `${guild.name} #${channelSlug}`,
      description: guild.slug,
      path: `/${guild.slug}/${channelSlug}`,
    }
  })
  return guildEntries.filter((result) => {
    if (!quickQueryNormalized) return true
    const haystack = `${result.label} ${result.description}`.toLowerCase()
    return haystack.includes(quickQueryNormalized)
  })
})
let quickSwitcherResults = $derived([
  ...quickDmResults,
  ...quickChannelResults,
] as QuickSwitcherResult[])

function openMobilePanel(panel: MobilePanel) {
  mobilePanel = panel
}

function toggleTabletMembers() {
  tabletMembersVisible = !tabletMembersVisible
}

async function handleLogout() {
  await onLogout?.()
}

async function handleOpenSettings() {
  await onOpenSettings?.()
}

async function handleDismissRecoveryNudge() {
  await onDismissRecoveryNudge?.()
}

function closeQuickSwitcher(): void {
  quickSwitcherOpen = false
  quickSwitcherQuery = ''
}

async function handleQuickSwitcherPick(path: string): Promise<void> {
  closeQuickSwitcher()
  await goto(path)
}

function openGuildSettings() {
  guildSettingsOpen = true
}

function openInviteModal() {
  inviteModalOpen = true
}

async function handleCloseGuildSettings() {
  guildSettingsOpen = false
}

async function handleCloseInviteModal() {
  inviteModalOpen = false
}

$effect(() => {
  if (viewport !== 'tablet') tabletMembersVisible = false
})

$effect(() => {
  if (viewport !== 'mobile') mobilePanel = 'messages'
})

$effect(() => {
  if (shellMode === 'channel') {
    presenceState.setRouting(activeGuild, activeChannel)
    return
  }
  presenceState.clearRouting()
})

$effect(() => {
  if (!currentPath || currentPath === lastFocusedPath) return
  lastFocusedPath = currentPath
  onRouteResolved?.(currentPath)
  void tick().then(() => {
    mainContent?.focus()
  })
})

$effect(() => {
  if (!quickSwitcherOpen) {
    quickSwitcherOpenedPath = null
    return
  }
  if (quickSwitcherOpenedPath === null) {
    quickSwitcherOpenedPath = currentPath
    return
  }
  if (currentPath === quickSwitcherOpenedPath) return
  quickSwitcherOpen = false
  quickSwitcherQuery = ''
  quickSwitcherOpenedPath = currentPath
})
</script>

<a
  href="#main-content"
  class="sr-only focus:not-sr-only focus:fixed focus:left-4 focus:top-4 focus:z-50 focus:rounded-md focus:bg-background focus:px-3 focus:py-2 focus:text-sm focus:text-foreground focus:ring-2 focus:ring-primary"
>
  Skip to content
</a>

{#if showReconnectingStatus}
  <div
    class="fixed right-3 top-3 z-50 rounded-md border border-border bg-card px-3 py-2 text-xs text-muted-foreground shadow"
    data-testid="reconnecting-status"
  >
    Reconnecting...
  </div>
{/if}

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
          {#if canOpenInvites}
            <button
              type="button"
              class="rounded-md bg-muted px-2 py-1 text-xs text-foreground"
              onclick={openInviteModal}
            >
              Invite people
            </button>
          {/if}
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
        <GuildRail
          activeGuild={activeGuild}
          activeChannel={activeChannel}
          activeDm={activeDm}
          mode={shellMode}
        />
      {:else if activeMobilePanel === 'channels'}
        <ChannelList activeGuild={activeGuild} activeChannel={activeChannel} />
      {:else if activeMobilePanel === 'members'}
        <MemberList activeGuild={activeGuild} />
      {:else}
        <MessageArea
          mode={shellMode}
          activeGuild={activeGuild}
          activeChannel={activeChannel}
          activeDm={activeDm}
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
      <GuildRail
        activeGuild={activeGuild}
        activeChannel={activeChannel}
        activeDm={activeDm}
        mode={shellMode}
      />
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
        <p class="text-sm text-muted-foreground">
          {#if shellMode === 'dm' && activeDm}
            DM / {activeDm}
          {:else}
            {activeGuild} / {activeChannel}
          {/if}
        </p>
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
          {#if canOpenInvites}
            <button
              type="button"
              class="rounded-md bg-muted px-3 py-2 text-xs font-medium text-foreground"
              onclick={openInviteModal}
            >
              Invite people
            </button>
          {/if}
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
          activeDm={activeDm}
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
        <MemberList activeGuild={activeGuild} />
      </aside>
    {:else if isTabletMembersVisible}
      <aside
        class="absolute inset-y-0 right-0 z-10 w-[240px] border-l border-border bg-card shadow-2xl"
        data-testid="tablet-member-list"
      >
        <MemberList activeGuild={activeGuild} />
      </aside>
    {/if}
  </div>
{/if}

{#if quickSwitcherOpen}
  <div
    class="fixed inset-0 z-40 flex items-start justify-center bg-black/50 p-4 pt-24"
    role="presentation"
    data-testid="quick-switcher-overlay"
  >
    <div
      class="w-full max-w-xl rounded-lg border border-border bg-card p-3 shadow-2xl"
      role="dialog"
      aria-modal="true"
      aria-label="Quick switcher"
      data-testid="quick-switcher"
    >
      <div class="mb-2 flex items-center gap-2">
        <input
          type="text"
          class="w-full rounded-md border border-input bg-background px-3 py-2 text-sm text-foreground focus:outline-none focus:ring-2 focus:ring-ring"
          placeholder="Jump to channels or DMs…"
          bind:value={quickSwitcherQuery}
          data-testid="quick-switcher-input"
        />
        <button
          type="button"
          class="rounded-md bg-muted px-2 py-2 text-xs text-foreground"
          onclick={closeQuickSwitcher}
        >
          Close
        </button>
      </div>
      <ul class="max-h-72 overflow-y-auto" role="list" data-testid="quick-switcher-results">
        {#if quickSwitcherResults.length === 0}
          <li class="px-2 py-2 text-sm text-muted-foreground">No matches</li>
        {:else}
          {#each quickSwitcherResults as result (result.id)}
            <li role="listitem">
              <button
                type="button"
                class="flex w-full items-center justify-between rounded-md px-2 py-2 text-left text-sm text-foreground hover:bg-muted"
                onclick={() => void handleQuickSwitcherPick(result.path)}
                data-testid={`quick-switcher-result-${result.id}`}
              >
                <span class="min-w-0 flex-1 truncate">{result.label}</span>
                <span class="ml-2 shrink-0 text-[11px] uppercase tracking-wide text-muted-foreground">
                  {result.section}
                </span>
              </button>
            </li>
          {/each}
        {/if}
      </ul>
    </div>
  </div>
{/if}

<GuildSettings
  open={guildSettingsOpen}
  guildSlug={activeGuild}
  onClose={handleCloseGuildSettings}
/>
<InviteModal
  open={inviteModalOpen}
  guildSlug={activeGuild}
  onClose={handleCloseInviteModal}
/>
