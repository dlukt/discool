<script lang="ts">
// biome-ignore-all lint/correctness/noUnusedVariables: Svelte template usage isn't detected reliably.
import {
  normalizeParticipantVolumePercent,
  PARTICIPANT_VOLUME_DEFAULT_PERCENT,
} from './participantVolume'
import type { VoiceParticipant } from './types'

type Props = {
  participant: VoiceParticipant
  showKickPlaceholder?: boolean
  onVolumeChange?: (participantUserId: string, volumePercent: number) => void
  onKickFromVoice?: (participantUserId: string) => void
}

let {
  participant,
  showKickPlaceholder = false,
  onVolumeChange,
  onKickFromVoice,
}: Props = $props()

let controlsOpen = $state(false)

let participantName = $derived(
  participant.displayName?.trim() || participant.username,
)

let avatarInitial = $derived(
  participantName.trim().charAt(0).toUpperCase() || '?',
)

let currentVolumePercent = $derived(
  normalizeParticipantVolumePercent(
    participant.volumePercent ?? PARTICIPANT_VOLUME_DEFAULT_PERCENT,
  ),
)

let rowLabel = $derived.by(() => {
  const labels = [participantName, `volume ${currentVolumePercent}%`]
  if (participant.isMuted) labels.push('muted')
  if (participant.isDeafened) labels.push('deafened')
  if (participant.isSpeaking) labels.push('speaking')
  return labels.join(', ')
})

let controlContainerId = $derived(
  `voice-participant-controls-${participant.userId}`,
)

let sliderId = $derived(`voice-participant-volume-slider-${participant.userId}`)

function toggleControls(): void {
  controlsOpen = !controlsOpen
}

function handleVolumeInput(event: Event): void {
  const target = event.currentTarget as HTMLInputElement | null
  if (!target) return
  const nextVolumePercent = normalizeParticipantVolumePercent(
    Number(target.value),
  )
  onVolumeChange?.(participant.userId, nextVolumePercent)
}

function handleKickFromVoice(): void {
  onKickFromVoice?.(participant.userId)
}
</script>

<li
  class="rounded-md border border-border/60 bg-background/60"
  data-testid={`voice-participant-${participant.userId}`}
  aria-label={rowLabel}
>
  <button
    type="button"
    class="flex w-full items-center gap-2 px-2 py-1.5 text-left"
    data-testid={`voice-participant-toggle-${participant.userId}`}
    aria-expanded={controlsOpen}
    aria-controls={controlContainerId}
    onclick={toggleControls}
  >
    <span
      class={`inline-flex h-8 w-8 shrink-0 items-center justify-center rounded-full text-xs font-semibold text-black transition-[box-shadow,border-color] ${participant.isSpeaking ? 'border border-cyan-300 shadow-[0_0_0_2px_rgba(103,232,249,0.45)] motion-reduce:shadow-none' : 'border border-border/60'} motion-reduce:transition-none`}
      style={`background-color: ${participant.avatarColor ?? '#99aab5'};`}
      aria-hidden="true"
      data-testid={`voice-participant-avatar-${participant.userId}`}
    >
      {avatarInitial}
    </span>
    <div class="min-w-0 flex-1">
      <p
        class="truncate text-sm font-medium text-foreground"
        data-testid={`voice-participant-name-${participant.userId}`}
      >
        {participantName}
      </p>
      <p class="truncate text-xs text-muted-foreground">@{participant.username}</p>
    </div>
    <span class="inline-flex items-center gap-1 text-xs" aria-hidden="true">
      {#if participant.isMuted}
        <span
          class="inline-flex h-5 min-w-5 items-center justify-center rounded border border-amber-500/40 bg-amber-500/10 px-1 text-amber-200"
          data-testid={`voice-participant-muted-${participant.userId}`}
        >
          🔇
        </span>
      {/if}
      {#if participant.isDeafened}
        <span
          class="inline-flex h-5 min-w-5 items-center justify-center rounded border border-amber-500/40 bg-amber-500/10 px-1 text-amber-200"
          data-testid={`voice-participant-deafened-${participant.userId}`}
        >
          🎧✕
        </span>
      {/if}
      <span
        class="rounded border border-border/80 bg-muted px-1.5 py-0.5 text-[11px] text-muted-foreground"
        data-testid={`voice-participant-volume-label-${participant.userId}`}
      >
        {currentVolumePercent}%
      </span>
    </span>
  </button>

  {#if controlsOpen}
    <div
      id={controlContainerId}
      class="space-y-2 border-t border-border/50 px-2 pb-2 pt-1.5"
      data-testid={controlContainerId}
    >
      <div class="flex items-center gap-2">
        <label class="sr-only" for={sliderId}>
          {participantName} volume
        </label>
        <input
          id={sliderId}
          type="range"
          min="0"
          max="200"
          step="5"
          value={currentVolumePercent}
          class="h-2 w-full cursor-pointer accent-primary"
          aria-label={`${participantName} volume`}
          aria-valuemin={0}
          aria-valuemax={200}
          aria-valuenow={currentVolumePercent}
          aria-valuetext={`${currentVolumePercent}%`}
          data-testid={sliderId}
          oninput={handleVolumeInput}
        />
      </div>
      {#if showKickPlaceholder}
        <button
          type="button"
          class="rounded border border-destructive/70 bg-destructive/10 px-2 py-1 text-xs text-destructive hover:opacity-90"
          data-testid={`voice-participant-kick-${participant.userId}`}
          onclick={handleKickFromVoice}
        >
          Kick from voice
        </button>
      {/if}
    </div>
  {/if}
</li>
