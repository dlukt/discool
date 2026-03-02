<script lang="ts">
// biome-ignore-all lint/correctness/noUnusedVariables: Svelte template usage isn't detected reliably.
import type { VoiceConnectionStatus } from './types'

type Props = {
  guildName: string
  channelName: string
  connectionState: VoiceConnectionStatus
  variant?: 'default' | 'mobile'
  isMuted: boolean
  isDeafened: boolean
  isParticipantsOpen: boolean
  onToggleParticipants: () => void
  onToggleMute: () => void
  onToggleDeafen: () => void
  onDisconnect: () => void
  onOpenSheet?: () => void
}

let {
  guildName,
  channelName,
  connectionState,
  variant = 'default',
  isMuted,
  isDeafened,
  isParticipantsOpen,
  onToggleParticipants,
  onToggleMute,
  onToggleDeafen,
  onDisconnect,
  onOpenSheet,
}: Props = $props()

let quality = $derived.by(() => {
  if (connectionState === 'connected') return 'green'
  if (connectionState === 'connecting' || connectionState === 'retrying') {
    return 'yellow'
  }
  return 'red'
})

let qualityLabel = $derived.by(() => {
  if (quality === 'green') return 'Good'
  if (quality === 'yellow') return 'Limited'
  return 'Poor'
})

let qualityClass = $derived.by(() => {
  if (quality === 'green') return 'bg-emerald-500'
  if (quality === 'yellow') return 'bg-amber-400'
  return 'bg-destructive'
})

let reconnectAnimationClass = $derived(
  connectionState === 'retrying' ? 'motion-safe:animate-pulse' : '',
)

let reconnectStatusLabel = $derived.by(() => {
  if (connectionState === 'retrying') return 'Reconnecting...'
  if (connectionState === 'failed') return 'Connection lost'
  return null
})

let reconnectStatusTone = $derived(
  connectionState === 'failed' ? 'text-destructive' : 'text-muted-foreground',
)

let voiceControlAnnouncement = $derived.by(() => {
  if (connectionState !== 'connected') return 'Voice disconnected.'
  if (isDeafened) return 'Voice controls updated. Deafened and muted.'
  if (isMuted) return 'Voice controls updated. Microphone muted.'
  return 'Voice controls updated. Microphone active.'
})

const SWIPE_OPEN_THRESHOLD_PX = 40
let mobileBarPointerStartY = $state<number | null>(null)
let isMobileVariant = $derived(variant === 'mobile')

function openSheet(): void {
  onOpenSheet?.()
}

function handleMobileBarPointerDown(event: PointerEvent): void {
  if (!isMobileVariant || event.pointerType === 'mouse') return
  mobileBarPointerStartY = event.clientY
}

function handleMobileBarPointerCancel(): void {
  mobileBarPointerStartY = null
}

function handleMobileBarPointerUp(event: PointerEvent): void {
  if (!isMobileVariant || mobileBarPointerStartY === null) return
  const pointerEndY = Number.isFinite(event.clientY)
    ? event.clientY
    : mobileBarPointerStartY
  const swipeDelta = mobileBarPointerStartY - pointerEndY
  const eventTarget = event.target
  const targetElement = eventTarget instanceof HTMLElement ? eventTarget : null
  const touchedInteractiveControl = targetElement?.closest('button') !== null
  mobileBarPointerStartY = null
  if (
    swipeDelta >= SWIPE_OPEN_THRESHOLD_PX ||
    (!touchedInteractiveControl && Math.abs(swipeDelta) <= 8)
  ) {
    openSheet()
  }
}
</script>

{#if isMobileVariant}
  <section
    class="touch-manipulation rounded-md border border-border bg-card/80 p-2"
    aria-label="Voice controls"
    data-testid="voice-bar"
    onpointerdown={handleMobileBarPointerDown}
    onpointerup={handleMobileBarPointerUp}
    onpointercancel={handleMobileBarPointerCancel}
  >
    <div class="flex items-center gap-2">
      <button
        type="button"
        class={`inline-flex h-12 w-12 shrink-0 items-center justify-center rounded-md border text-lg hover:bg-muted ${isMuted ? 'border-amber-500/40 bg-amber-500/10 text-amber-200' : 'border-border bg-background text-foreground'}`}
        onclick={onToggleMute}
        aria-label={isMuted ? 'Unmute microphone' : 'Mute microphone'}
        data-testid="voice-bar-toggle-mute"
      >
        {isMuted ? '🎤✕' : '🎤'}
      </button>
      <button
        type="button"
        class="min-w-0 flex-1 rounded-md px-2 py-2 text-left hover:bg-muted"
        onclick={openSheet}
        aria-label="Open voice controls"
        data-testid="voice-bar-open-sheet"
      >
        <p
          class="truncate text-sm font-medium text-foreground"
          data-testid="voice-bar-channel"
        >
          #{channelName}
        </p>
        <p
          class="truncate text-xs text-muted-foreground"
          data-testid="voice-bar-guild"
        >
          {guildName}
        </p>
      </button>
    </div>
    {#if reconnectStatusLabel}
      <p
        class={`mt-2 text-xs ${reconnectStatusTone}`}
        data-testid="voice-bar-status"
        aria-live={connectionState === 'failed' ? 'assertive' : 'polite'}
      >
        {reconnectStatusLabel}
      </p>
    {/if}
    <p class="sr-only" aria-live="polite" data-testid="voice-bar-live">
      {voiceControlAnnouncement}
    </p>
  </section>
{:else}
  <section
    class="rounded-md border border-border bg-card/80 px-3 py-2"
    aria-label="Voice controls"
    data-testid="voice-bar"
  >
    <div class="flex flex-wrap items-center gap-2">
      <div class="min-w-0">
        <p
          class="truncate text-sm font-medium text-foreground"
          data-testid="voice-bar-channel"
        >
          #{channelName}
        </p>
        <p class="truncate text-xs text-muted-foreground" data-testid="voice-bar-guild">
          {guildName}
        </p>
      </div>
      <div class="ml-auto flex items-center gap-2 text-xs text-muted-foreground">
        <span
          class={`h-2.5 w-2.5 rounded-full ${qualityClass} ${reconnectAnimationClass}`}
          data-testid="voice-bar-quality-dot"
          data-quality={quality}
        ></span>
        <span>{qualityLabel}</span>
      </div>
      <div class="flex items-center gap-1">
        <button
          type="button"
          class={`rounded-md border px-2 py-1 text-xs font-medium hover:bg-muted ${isParticipantsOpen ? 'border-fire/40 bg-fire/10 text-fire-foreground' : 'border-border bg-background text-foreground'}`}
          onclick={onToggleParticipants}
          aria-label={isParticipantsOpen ? 'Hide voice participants' : 'Show voice participants'}
          data-testid="voice-bar-toggle-participants"
        >
          👥
        </button>
        <button
          type="button"
          class={`rounded-md border px-2 py-1 text-xs font-medium hover:bg-muted ${isMuted ? 'border-amber-500/40 bg-amber-500/10 text-amber-200' : 'border-border bg-background text-foreground'}`}
          onclick={onToggleMute}
          aria-label={isMuted ? 'Unmute microphone' : 'Mute microphone'}
          data-testid="voice-bar-toggle-mute"
        >
          {isMuted ? '🎤✕' : '🎤'}
        </button>
        <button
          type="button"
          class={`rounded-md border px-2 py-1 text-xs font-medium hover:bg-muted ${isDeafened ? 'border-amber-500/40 bg-amber-500/10 text-amber-200' : 'border-border bg-background text-foreground'}`}
          onclick={onToggleDeafen}
          aria-label={isDeafened ? 'Undeafen audio' : 'Deafen audio'}
          data-testid="voice-bar-toggle-deafen"
        >
          {isDeafened ? '🎧✕' : '🎧'}
        </button>
        <button
          type="button"
          class="rounded-md border border-fire/40 bg-fire px-2 py-1 text-xs font-medium text-fire-foreground hover:opacity-90"
          onclick={onDisconnect}
          aria-label="Disconnect from voice channel"
          data-testid="voice-bar-disconnect"
        >
          📞
        </button>
      </div>
    </div>
    {#if reconnectStatusLabel}
      <p
        class={`mt-2 text-xs ${reconnectStatusTone}`}
        data-testid="voice-bar-status"
        aria-live={connectionState === 'failed' ? 'assertive' : 'polite'}
      >
        {reconnectStatusLabel}
      </p>
    {/if}
    <p class="sr-only" aria-live="polite" data-testid="voice-bar-live">
      {voiceControlAnnouncement}
    </p>
  </section>
{/if}
