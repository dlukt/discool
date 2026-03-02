<script lang="ts">
// biome-ignore-all lint/correctness/noUnusedVariables: Svelte template usage isn't detected reliably.
// biome-ignore-all lint/correctness/noUnusedImports: Svelte template usage isn't detected reliably.
import type { VoiceParticipant } from './types'
import VoiceParticipantRow from './VoiceParticipant.svelte'

type Props = {
  channelName: string
  participants: VoiceParticipant[]
  variant?: 'default' | 'mobile-sheet'
  canModerateVoiceParticipants?: boolean
  onParticipantVolumeChange?: (
    participantUserId: string,
    volumePercent: number,
  ) => void
}

let {
  channelName,
  participants,
  variant = 'default',
  canModerateVoiceParticipants = false,
  onParticipantVolumeChange,
}: Props = $props()

let occupancyAnnouncement = $derived(
  `${participants.length} ${participants.length === 1 ? 'user' : 'users'} in voice channel ${channelName}`,
)
let isMobileSheetVariant = $derived(variant === 'mobile-sheet')
</script>

<section
  class={`rounded-md border border-border bg-card/80 p-3 ${isMobileSheetVariant ? 'max-h-[40vh] overflow-y-auto' : ''}`}
  data-testid="voice-panel"
  data-variant={variant}
  aria-label={`Voice participants for ${channelName}`}
>
  <p
    class="sr-only"
    aria-live="polite"
    aria-atomic="true"
    data-testid="voice-panel-occupancy-announcement"
  >
    {occupancyAnnouncement}
  </p>
  <header class="mb-2 flex items-center justify-between gap-2">
    <h3 class="text-sm font-semibold text-foreground">Voice participants</h3>
    <span class="rounded-full bg-muted px-2 py-0.5 text-xs text-muted-foreground">
      {participants.length}
    </span>
  </header>

  {#if participants.length === 0}
    <p class="rounded-md bg-muted px-3 py-2 text-xs text-muted-foreground" data-testid="voice-panel-empty">
      No participants in voice.
    </p>
  {:else}
      <ul class="space-y-1" data-testid="voice-panel-list">
        {#each participants as participant (participant.userId)}
          <VoiceParticipantRow
            {participant}
            showKickPlaceholder={canModerateVoiceParticipants}
            onVolumeChange={onParticipantVolumeChange}
          />
        {/each}
      </ul>
    {/if}
</section>
