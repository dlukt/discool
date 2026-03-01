<script lang="ts">
// biome-ignore-all lint/correctness/noUnusedVariables: Svelte template usage isn't detected reliably.
import { goto } from '@mateothegreat/svelte5-router'
import { onMount } from 'svelte'
import { dmState } from './dmStore.svelte'

type Props = {
  activeDm: string | null
}

let { activeDm }: Props = $props()
let conversations = $derived(dmState.conversations)

onMount(() => {
  void dmState.ensureLoaded().catch(() => {})
})

async function goToDm(dmSlug: string): Promise<void> {
  await goto(`/dm/${dmSlug}`)
}
</script>

<section class="w-full px-2" data-testid="dm-list">
  <h3 class="mb-2 text-[11px] font-semibold uppercase tracking-wide text-muted-foreground">
    Direct Messages
  </h3>

  {#if conversations.length === 0}
    <p class="px-1 text-[11px] text-muted-foreground" data-testid="dm-list-empty">
      No active conversations
    </p>
  {:else}
    <ul class="space-y-1" role="list">
      {#each conversations as conversation}
        <li role="listitem">
          <button
            type="button"
            class={`flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-left text-xs transition-colors ${
              activeDm === conversation.dmSlug
                ? 'bg-primary/20 text-primary'
                : 'bg-muted text-foreground hover:bg-sidebar-accent'
            }`}
            onclick={() => void goToDm(conversation.dmSlug)}
            data-testid={`dm-list-item-${conversation.dmSlug}`}
            aria-current={activeDm === conversation.dmSlug ? 'page' : undefined}
          >
            <span class="min-w-0 flex-1">
              <span class="block truncate font-medium">
                {conversation.participant.displayName}
              </span>
              {#if conversation.lastMessagePreview}
                <span class="block truncate text-[11px] text-muted-foreground">
                  {conversation.lastMessagePreview}
                </span>
              {/if}
            </span>
            {#if conversation.hasUnreadActivity}
              <span
                class="h-2 w-2 shrink-0 rounded-full bg-fire"
                data-testid={`dm-unread-badge-${conversation.dmSlug}`}
              ></span>
            {/if}
          </button>
        </li>
      {/each}
    </ul>
  {/if}
</section>
