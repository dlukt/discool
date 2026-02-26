<script lang="ts">
// biome-ignore lint/correctness/noUnusedImports: Used in Svelte markup; Biome doesn't detect template usage.
import { route as routerLink } from '@mateothegreat/svelte5-router'

type Props = {
  activeGuild: string
  activeChannel: string
}

// biome-ignore lint/correctness/noUnusedVariables: Used in Svelte markup; Biome doesn't detect template usage.
let { activeGuild, activeChannel }: Props = $props()

// biome-ignore lint/correctness/noUnusedVariables: Used in Svelte markup; Biome doesn't detect template usage.
const guilds = [
  { slug: 'lobby', short: 'L', label: 'Lobby' },
  { slug: 'engineering', short: 'E', label: 'Engineering' },
  { slug: 'support', short: 'S', label: 'Support' },
]
</script>

<div
  class="flex h-full w-full flex-col items-center gap-3 border-r border-border bg-sidebar py-3"
  data-testid="guild-rail"
>
  <a
    class="inline-flex h-10 w-10 items-center justify-center rounded-xl bg-fire text-sm font-semibold text-fire-foreground"
    href="/"
    use:routerLink
    aria-label="Go to home"
  >
    D
  </a>

  <nav class="flex w-full flex-col items-center gap-2" aria-label="Guild navigation">
    {#each guilds as guild}
      <a
        class={`inline-flex h-10 w-10 items-center justify-center rounded-xl border text-xs font-semibold transition-colors ${
          guild.slug === activeGuild
            ? 'border-primary bg-primary/20 text-primary'
            : 'border-border bg-muted text-muted-foreground hover:bg-sidebar-accent hover:text-sidebar-accent-foreground'
        }`}
        href={`/${guild.slug}/${activeChannel}`}
        use:routerLink
        aria-label={`Open ${guild.label} guild`}
        aria-current={guild.slug === activeGuild ? 'page' : undefined}
      >
        {guild.short}
      </a>
    {/each}
  </nav>
</div>
