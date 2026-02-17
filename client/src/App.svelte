<main class="min-h-screen p-8">
  <div class="mx-auto flex w-full max-w-xl flex-col gap-6 rounded-lg border border-border bg-card p-8">
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
</main>
