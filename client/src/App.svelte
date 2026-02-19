<script lang="ts">
import { onMount } from 'svelte'
import { ApiError, getInstanceStatus, type InstanceStatus } from '$lib/api'

// biome-ignore lint/correctness/noUnusedImports: Used in Svelte markup; Biome doesn't detect template usage.
import AdminPanel from '$lib/components/AdminPanel.svelte'
// biome-ignore lint/correctness/noUnusedImports: Used in Svelte markup; Biome doesn't detect template usage.
import SetupPage from '$lib/components/SetupPage.svelte'

// biome-ignore lint/correctness/noUnusedVariables: Used in Svelte markup; Biome doesn't detect template usage.
let loading = true
// biome-ignore lint/correctness/noUnusedVariables: Used in Svelte markup; Biome doesn't detect template usage.
let errorMessage: string | null = null
// biome-ignore lint/correctness/noUnusedVariables: Used in Svelte markup; Biome doesn't detect template usage.
let status: InstanceStatus | null = null
// biome-ignore lint/correctness/noUnusedVariables: Used in Svelte markup; Biome doesn't detect template usage.
let view: 'home' | 'admin' = 'home'

async function loadStatus() {
  loading = true
  errorMessage = null
  try {
    status = await getInstanceStatus()
  } catch (err) {
    status = null
    if (err instanceof ApiError) {
      errorMessage = err.message
    } else {
      errorMessage = 'Could not connect to the server. Is it running?'
    }
  } finally {
    loading = false
  }
}

onMount(() => {
  void loadStatus()
})
</script>

{#if loading}
  <main class="min-h-screen bg-background p-8">
    <p class="text-center text-sm text-muted-foreground">Loading...</p>
  </main>
{:else if errorMessage}
  <main class="min-h-screen bg-background p-8">
    <div class="mx-auto w-full max-w-md space-y-4 rounded-lg border border-border bg-card p-6">
      <p class="text-sm text-destructive">{errorMessage}</p>
      <button
        type="button"
        class="inline-flex w-full items-center justify-center rounded-md bg-fire px-4 py-2 text-sm font-medium text-fire-foreground transition-opacity hover:opacity-90"
        on:click={() => void loadStatus()}
      >
        Retry
      </button>
    </div>
  </main>
{:else if status && !status.initialized}
  <SetupPage on:complete={() => void loadStatus()} />
{:else}
  <main class="min-h-screen bg-background">
    <div class="flex min-h-screen">
      <aside class="w-56 border-r border-border bg-sidebar p-4">
        <div class="mb-4 text-sm font-semibold text-sidebar-foreground">Discool</div>
        <nav class="space-y-1">
          <button
            type="button"
            class={`w-full rounded-md px-3 py-2 text-left text-sm transition-colors ${
              view === 'home'
                ? 'bg-sidebar-accent text-sidebar-accent-foreground'
                : 'text-muted-foreground hover:bg-muted'
            }`}
            on:click={() => (view = 'home')}
          >
            Home
          </button>
          <button
            type="button"
            class={`w-full rounded-md px-3 py-2 text-left text-sm transition-colors ${
              view === 'admin'
                ? 'bg-sidebar-accent text-sidebar-accent-foreground'
                : 'text-muted-foreground hover:bg-muted'
            }`}
            on:click={() => (view = 'admin')}
          >
            Admin
          </button>
        </nav>
      </aside>

      <section class="flex-1 p-8">
        {#if view === 'admin'}
          <AdminPanel />
        {:else}
          <div
            class="mx-auto flex w-full max-w-xl flex-col gap-6 rounded-lg border border-border bg-card p-8"
          >
            <header class="space-y-2">
              <h1 class="text-4xl font-semibold tracking-tight">Discool</h1>
              <p class="text-sm text-muted-foreground">
                Dual Core theme scaffold (Ice navigation, Fire actions, Zinc foundation).
              </p>
            </header>

            <section class="flex flex-wrap gap-3">
              <button
                type="button"
                class="inline-flex items-center justify-center rounded-md bg-primary px-4 py-2 text-sm font-medium text-primary-foreground transition-colors hover:bg-accent hover:text-accent-foreground"
              >
                Ice action
              </button>

              <button
                type="button"
                class="inline-flex items-center justify-center rounded-md bg-fire px-4 py-2 text-sm font-medium text-fire-foreground transition-opacity hover:opacity-90"
              >
                Fire action
              </button>

              <button
                type="button"
                class="inline-flex items-center justify-center rounded-md bg-destructive px-4 py-2 text-sm font-medium text-destructive-foreground transition-opacity hover:opacity-90"
              >
                Destructive
              </button>
            </section>

            <section class="rounded-md bg-muted p-4 text-sm text-muted-foreground">
              Try inspecting CSS variables in <code class="text-foreground">src/app.css</code>.
            </section>
          </div>
        {/if}
      </section>
    </div>
  </main>
{/if}
