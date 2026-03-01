<script lang="ts">
// biome-ignore-all lint/correctness/noUnusedVariables: Svelte template usage isn't detected reliably.
type Props = {
  activeGuild: string
  activeChannel: string
  activeDm: string | null
  mode: 'home' | 'channel' | 'dm' | 'settings' | 'admin'
}

let { activeGuild }: Props = $props()
</script>

<aside data-testid="guild-rail">
  <button aria-label="Home" data-testid="guild-rail-home" type="button">
    Home
  </button>
  <span>{activeGuild}</span>
</aside>
