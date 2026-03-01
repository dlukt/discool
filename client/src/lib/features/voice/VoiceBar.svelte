<script lang="ts">
// biome-ignore-all lint/correctness/noUnusedVariables: Svelte template usage isn't detected reliably.
import type { VoiceConnectionStatus } from './types'

type Props = {
  guildName: string
  channelName: string
  connectionState: VoiceConnectionStatus
  isMuted: boolean
  isDeafened: boolean
  onToggleMute: () => void
  onToggleDeafen: () => void
  onDisconnect: () => void
}

let {
  guildName,
  channelName,
  connectionState,
  isMuted,
  isDeafened,
  onToggleMute,
  onToggleDeafen,
  onDisconnect,
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

let voiceControlAnnouncement = $derived.by(() => {
  if (connectionState !== 'connected') return 'Voice disconnected.'
  if (isDeafened) return 'Voice controls updated. Deafened and muted.'
  if (isMuted) return 'Voice controls updated. Microphone muted.'
  return 'Voice controls updated. Microphone active.'
})
</script>

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
        class={`h-2.5 w-2.5 rounded-full ${qualityClass}`}
        data-testid="voice-bar-quality-dot"
        data-quality={quality}
      ></span>
      <span>{qualityLabel}</span>
    </div>
    <div class="flex items-center gap-1">
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
  <p class="sr-only" aria-live="polite" data-testid="voice-bar-live">
    {voiceControlAnnouncement}
  </p>
</section>
