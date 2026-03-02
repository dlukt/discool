<script lang="ts">
// biome-ignore-all lint/correctness/noUnusedVariables: Svelte template usage isn't detected reliably.
type Props = {
  mode: 'home' | 'channel' | 'dm' | 'settings' | 'admin'
  activeGuild: string
  activeChannel: string
  activeDm?: string | null
  displayName: string
  isAdmin: boolean
  showRecoveryNudge: boolean
  showVoiceControls?: boolean
}

let { mode }: Props = $props()
</script>

<section data-testid="message-area">
  <h2>Messages</h2>
  <span>{mode}</span>
</section>
