<script lang="ts">
type Props = {
  guildName?: string | null
  guildIconUrl?: string | null
  inviteContextInvalid?: boolean
  username: string
  displayName: string
  avatarColor: string | null
  joining?: boolean
  errorMessage?: string | null
  onconfirm?: () => void | Promise<void>
  onusedifferentname?: () => void
}

// biome-ignore lint/correctness/noUnusedVariables: Used in Svelte markup; Biome doesn't detect template usage.
function safeAvatarColor(value: string | null): string {
  if (typeof value === 'string' && /^#[0-9a-fA-F]{6}$/.test(value)) return value
  return '#3b82f6'
}

let props: Props = $props()
// biome-ignore lint/correctness/noUnusedVariables: Used in Svelte markup; Biome doesn't detect template usage.
let targetName = $derived(props.guildName?.trim() || 'this instance')
// biome-ignore lint/correctness/noUnusedVariables: Used in Svelte markup; Biome doesn't detect template usage.
let avatarInitial = $derived(
  (props.displayName ?? '').trim().slice(0, 1).toUpperCase() || '?',
)
</script>

<main class="min-h-screen bg-background p-8">
  <div class="mx-auto flex w-full max-w-md flex-col gap-6 rounded-lg border border-border bg-card p-8">
    <header class="space-y-2 text-center">
      <p class="text-sm font-medium text-muted-foreground">Discool</p>
      {#if props.guildIconUrl}
        <img
          src={props.guildIconUrl}
          alt="Guild icon"
          class="mx-auto h-12 w-12 rounded-full border border-border object-cover"
        />
      {/if}
      <h1 class="text-3xl font-semibold tracking-tight">
        Join {targetName} as {props.displayName}?
      </h1>
      <p class="text-sm text-muted-foreground">
        We'll verify your identity cryptographically and sign you in with one click.
      </p>
    </header>

    <div class="flex items-center gap-4 rounded-md bg-muted p-4">
      <div
        class="flex h-10 w-10 items-center justify-center rounded-full text-sm font-semibold text-white"
        style={`background-color: ${safeAvatarColor(props.avatarColor ?? null)}`}
        role="img"
        aria-label="Avatar preview"
      >
        {avatarInitial}
      </div>
      <div class="min-w-0">
        <p class="truncate text-sm font-semibold">{props.displayName}</p>
        <p class="truncate text-xs text-muted-foreground">@{props.username}</p>
      </div>
    </div>

    {#if props.inviteContextInvalid ?? false}
      <div class="rounded-md border border-border bg-muted p-3 text-sm text-muted-foreground">
        Invite details are unavailable, but you can still continue sign-in for this instance.
      </div>
    {/if}

    {#if props.errorMessage}
      <div
        class="rounded-md border border-destructive/30 bg-destructive/10 p-3 text-sm text-destructive"
        role="alert"
      >
        {props.errorMessage}
      </div>
    {/if}

    <div class="space-y-3">
      <button
        type="button"
        class="inline-flex w-full items-center justify-center gap-2 rounded-md bg-fire px-4 py-2 text-sm font-medium text-fire-foreground transition-opacity hover:opacity-90 disabled:cursor-not-allowed disabled:opacity-50"
        onclick={() => void props.onconfirm?.()}
        disabled={props.joining ?? false}
      >
        {#if props.joining ?? false}
          <span
            class="inline-block h-4 w-4 animate-spin rounded-full border-2 border-fire-foreground border-t-transparent"
            aria-hidden="true"
          ></span>
          Joining...
        {:else}
          Join as {props.displayName}
        {/if}
      </button>

      <button
        type="button"
        class="inline-flex w-full items-center justify-center rounded-md bg-muted px-4 py-2 text-sm font-medium text-foreground transition-opacity hover:opacity-90"
        onclick={props.onusedifferentname}
        disabled={props.joining ?? false}
      >
        Use a different name
      </button>
    </div>
  </div>
</main>
