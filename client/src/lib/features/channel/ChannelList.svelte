<script lang="ts">
// biome-ignore lint/correctness/noUnusedImports: Used in Svelte markup; Biome doesn't detect template usage.
import { route as routerLink } from '@mateothegreat/svelte5-router'

type Props = {
  activeGuild: string
  activeChannel: string
}

// biome-ignore lint/correctness/noUnusedVariables: Used in Svelte markup; Biome doesn't detect template usage.
let { activeGuild, activeChannel }: Props = $props()

const defaultChannels = ['general', 'announcements', 'random']
const channelsByGuild: Record<string, string[]> = {
  lobby: defaultChannels,
  engineering: ['general', 'builds', 'on-call'],
  support: ['general', 'incidents', 'feedback'],
}

// biome-ignore lint/correctness/noUnusedVariables: Used in Svelte markup; Biome doesn't detect template usage.
let channels = $derived(channelsByGuild[activeGuild] ?? defaultChannels)
</script>

<aside
  class="h-full border-r border-border bg-card p-4"
  data-testid="channel-list"
  aria-label="Channel navigation"
>
  <h2 class="mb-3 text-sm font-semibold text-foreground">{activeGuild}</h2>
  <nav class="space-y-1">
    {#each channels as channel}
      <a
        class={`block rounded-md px-3 py-2 text-sm transition-colors ${
          channel === activeChannel
            ? 'bg-sidebar-accent text-sidebar-accent-foreground'
            : 'text-muted-foreground hover:bg-muted hover:text-foreground'
        }`}
        href={`/${activeGuild}/${channel}`}
        use:routerLink
        aria-label={`Open channel ${channel}`}
        aria-current={channel === activeChannel ? 'page' : undefined}
      >
        # {channel}
      </a>
    {/each}
  </nav>
</aside>
