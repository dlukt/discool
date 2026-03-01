<script lang="ts">
// biome-ignore-all lint/correctness/noUnusedVariables: Svelte template usage isn't detected reliably.
type Props = {
  activeGuild: string
  activeChannel: string
}

let { activeGuild, activeChannel }: Props = $props()
</script>

<aside data-testid="channel-list">
  <h2>Channels</h2>
  <span>{activeGuild}/{activeChannel}</span>
</aside>
