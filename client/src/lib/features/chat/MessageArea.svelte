<script lang="ts">
// biome-ignore-all lint/correctness/noUnusedVariables: Svelte template usage isn't detected reliably.
// biome-ignore-all lint/correctness/noUnusedImports: Svelte template usage isn't detected reliably.
import { onMount, tick } from 'svelte'
import AdminPanel from '$lib/components/AdminPanel.svelte'
import { guildState } from '$lib/features/guild/guildStore.svelte'
import { identityState } from '$lib/features/identity/identityStore.svelte'
import ProfileSettingsView from '$lib/features/identity/ProfileSettingsView.svelte'
import { wsClient } from '$lib/ws/client'
import type { WsLifecycleState } from '$lib/ws/protocol'
import MessageBubble from './MessageBubble.svelte'
import { type ChatAuthorInput, messageState } from './messageStore.svelte'

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

let detailText = $derived(
  mode === 'channel'
    ? `#${activeChannel} in ${activeGuild}`
    : `Signed in as ${displayName}.`,
)
let canShowAdminPanel = $derived(mode === 'admin' && isAdmin)
let shouldShowRecoveryNudge = $derived(
  showRecoveryNudge && (mode === 'home' || mode === 'channel'),
)
let wsLifecycleState = $state<WsLifecycleState>(wsClient.getLifecycleState())
let showReconnectingBanner = $derived(wsLifecycleState === 'reconnecting')
let composerInput = $state<HTMLTextAreaElement | null>(null)
let composerValue = $state('')

let timelineMessages = $derived.by(() => {
  const _messageVersion = messageState.version
  if (mode !== 'channel') return []
  return messageState.timeline(activeGuild, activeChannel)
})
let currentSessionUser = $derived(identityState.session?.user ?? null)
let currentMember = $derived(
  currentSessionUser
    ? guildState.memberByUserId(activeGuild, currentSessionUser.id)
    : null,
)
let currentRoleColor = $derived(
  currentMember?.highestRoleColor ??
    currentSessionUser?.avatarColor ??
    '#99aab5',
)
let emptyStateCopy = $derived(
  `This is the beginning of #${activeChannel}. Say something!`,
)

async function handleOpenSettings() {
  await onOpenSettings?.()
}

async function handleDismissRecoveryNudge() {
  await onDismissRecoveryNudge?.()
}

function buildCurrentAuthor(): ChatAuthorInput | null {
  if (!currentSessionUser) return null
  return {
    userId: currentSessionUser.id,
    username: currentSessionUser.username,
    displayName: currentSessionUser.displayName,
    avatarColor: currentSessionUser.avatarColor,
    roleColor: currentRoleColor,
  }
}

function sendComposerMessage() {
  const author = buildCurrentAuthor()
  if (!author || mode !== 'channel') return
  const sent = messageState.sendMessage(
    activeGuild,
    activeChannel,
    composerValue,
    author,
  )
  if (sent) {
    composerValue = ''
  }
}

function handleComposerKeydown(event: KeyboardEvent) {
  if (event.key !== 'Enter') return

  if (event.shiftKey) {
    event.preventDefault()
    const target = event.currentTarget as HTMLTextAreaElement | null
    const start = target?.selectionStart ?? composerValue.length
    const end = target?.selectionEnd ?? composerValue.length
    composerValue = `${composerValue.slice(0, start)}\n${composerValue.slice(end)}`
    if (target) {
      void tick().then(() => {
        target.selectionStart = start + 1
        target.selectionEnd = start + 1
      })
    }
    return
  }

  event.preventDefault()
  sendComposerMessage()
}

$effect(() => {
  if (mode !== 'channel') return
  activeGuild
  activeChannel
  void tick().then(() => {
    composerInput?.focus()
  })
})

onMount(() => {
  return wsClient.subscribeLifecycle((state) => {
    wsLifecycleState = state
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

    {#if showReconnectingBanner}
      <p
        class="rounded-md border border-amber-500/30 bg-amber-500/10 px-3 py-2 text-sm text-amber-200"
        data-testid="reconnecting-status"
      >
        Reconnecting...
      </p>
    {/if}

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

    <section class="min-h-0 flex-1 rounded-md border border-border bg-card p-4">
      <h2 class="text-sm font-medium text-foreground">Channel Timeline</h2>
      <div class="mt-3 space-y-2" data-testid="channel-timeline">
        {#if timelineMessages.length === 0}
          <p
            class="rounded-md bg-muted px-3 py-2 text-sm text-muted-foreground"
            data-testid="message-empty-state"
          >
            {emptyStateCopy}
          </p>
        {:else}
          {#each timelineMessages as message, index (message.id)}
            {@const previous = index > 0 ? timelineMessages[index - 1] : null}
            <MessageBubble
              {message}
              compact={Boolean(
                previous &&
                  !previous.isSystem &&
                  !message.isSystem &&
                  previous.authorUserId === message.authorUserId
              )}
            />
          {/each}
        {/if}
      </div>
    </section>

    {#if mode === 'channel'}
      <section class="rounded-md border border-border bg-card p-4">
        <label
          for="message-composer"
          class="mb-2 block text-sm font-medium text-foreground"
        >
          Message
        </label>
        <div class="flex items-end gap-2">
          <textarea
            id="message-composer"
            data-testid="message-composer-input"
            class="min-h-[44px] w-full resize-y rounded-md border border-input bg-background px-3 py-2 text-sm text-foreground placeholder:text-muted-foreground focus:outline-none focus:ring-2 focus:ring-ring"
            placeholder={`Message #${activeChannel}`}
            bind:this={composerInput}
            bind:value={composerValue}
            onkeydown={handleComposerKeydown}
          ></textarea>
          <button
            type="button"
            class="inline-flex h-[44px] items-center justify-center rounded-md bg-fire px-4 text-sm font-medium text-fire-foreground transition-opacity hover:opacity-90 disabled:cursor-not-allowed disabled:opacity-50"
            onclick={sendComposerMessage}
            disabled={!currentSessionUser || composerValue.trim().length === 0}
          >
            Send
          </button>
        </div>
        <p class="mt-2 text-xs text-muted-foreground">
          Enter to send · Shift+Enter for newline
        </p>
      </section>
    {/if}
  </section>
{/if}
