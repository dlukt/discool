<script lang="ts">
// biome-ignore-all lint/correctness/noUnusedVariables: Svelte template usage isn't detected reliably.
import type { VoiceParticipant } from './types'

type Props = {
  participant: VoiceParticipant
}

let { participant }: Props = $props()

let participantName = $derived(
  participant.displayName?.trim() || participant.username,
)

let avatarInitial = $derived(
  participantName.trim().charAt(0).toUpperCase() || '?',
)

let rowLabel = $derived.by(() => {
  const labels = [participantName]
  if (participant.isMuted) labels.push('muted')
  if (participant.isDeafened) labels.push('deafened')
  if (participant.isSpeaking) labels.push('speaking')
  return labels.join(', ')
})
</script>

<li
  class="flex items-center gap-2 rounded-md border border-border/60 bg-background/60 px-2 py-1.5"
  data-testid={`voice-participant-${participant.userId}`}
  aria-label={rowLabel}
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
  <span class="ml-auto inline-flex items-center gap-1 text-xs" aria-hidden="true">
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
  </span>
</li>
