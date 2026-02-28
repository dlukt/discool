<script lang="ts">
// biome-ignore lint/correctness/noUnusedImports: Used in Svelte markup; Biome doesn't detect template usage.
import { route as routerLink } from '@mateothegreat/svelte5-router'
import { onMount } from 'svelte'

import { guildState } from '$lib/features/guild/guildStore.svelte'

type Props = {
  activeGuild: string
  activeChannel: string
}

let { activeGuild, activeChannel }: Props = $props()

onMount(() => {
  void guildState.loadGuilds().catch(() => {
    // Shell can still render with fallback channel labels.
  })
})

let guild = $derived(guildState.bySlug(activeGuild))
// biome-ignore lint/correctness/noUnusedVariables: Used in Svelte markup; Biome doesn't detect template usage.
let guildLabel = $derived(guild?.name ?? activeGuild)
let defaultChannel = $derived(guild?.defaultChannelSlug ?? 'general')
// biome-ignore lint/correctness/noUnusedVariables: Used in Svelte markup; Biome doesn't detect template usage.
let channels = $derived(
  activeChannel === defaultChannel
    ? [defaultChannel]
    : [defaultChannel, activeChannel],
)
</script>

<aside
  class="h-full border-r border-border bg-card p-4"
  data-testid="channel-list"
  aria-label="Channel navigation"
>
  <h2 class="mb-3 text-sm font-semibold text-foreground">{guildLabel}</h2>
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
