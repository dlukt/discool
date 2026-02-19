<script lang="ts">
import { onDestroy, onMount } from 'svelte'
import {
  type AdminHealth,
  ApiError,
  downloadBackup,
  getAdminHealth,
} from '$lib/api'

let health: AdminHealth | null = null
// biome-ignore lint/correctness/noUnusedVariables: Used in Svelte markup; Biome doesn't detect template usage.
let errorMessage: string | null = null
let fetching = false

// biome-ignore lint/correctness/noUnusedVariables: Used in Svelte markup; Biome doesn't detect template usage.
let showSkeleton = false
// biome-ignore lint/correctness/noUnusedVariables: Used in Svelte markup; Biome doesn't detect template usage.
let showSlowLoadingText = false

// biome-ignore lint/correctness/noUnusedVariables: Used in Svelte markup; Biome doesn't detect template usage.
let lastUpdatedAtMs: number | null = null
let nowMs = Date.now()

let refreshInterval: ReturnType<typeof setInterval> | null = null
let clockInterval: ReturnType<typeof setInterval> | null = null
let skeletonTimer: ReturnType<typeof setTimeout> | null = null
let slowTimer: ReturnType<typeof setTimeout> | null = null
let backupSlowTimer: ReturnType<typeof setTimeout> | null = null
let backupSuccessTimer: ReturnType<typeof setTimeout> | null = null

function clearTimers() {
  if (skeletonTimer) clearTimeout(skeletonTimer)
  if (slowTimer) clearTimeout(slowTimer)
  skeletonTimer = null
  slowTimer = null
}

function clearBackupTimers() {
  if (backupSlowTimer) clearTimeout(backupSlowTimer)
  if (backupSuccessTimer) clearTimeout(backupSuccessTimer)
  backupSlowTimer = null
  backupSuccessTimer = null
}

let backingUp = false
// biome-ignore lint/correctness/noUnusedVariables: Used in Svelte markup; Biome doesn't detect template usage.
let backupErrorMessage: string | null = null
// biome-ignore lint/correctness/noUnusedVariables: Used in Svelte markup; Biome doesn't detect template usage.
let backupStatusMessage: string | null = null
// biome-ignore lint/correctness/noUnusedVariables: Used in Svelte markup; Biome doesn't detect template usage.
let showBackupSlowLoadingText = false

// biome-ignore lint/correctness/noUnusedVariables: Used in Svelte markup; Biome doesn't detect template usage.
async function runBackup() {
  if (backingUp) return
  backingUp = true
  backupErrorMessage = null
  backupStatusMessage = null

  showBackupSlowLoadingText = false
  clearBackupTimers()
  backupSlowTimer = setTimeout(() => {
    showBackupSlowLoadingText = true
  }, 2000)

  try {
    await downloadBackup()
    backupStatusMessage = 'Backup complete.'
    backupSuccessTimer = setTimeout(() => {
      backupStatusMessage = null
    }, 4000)
  } catch (err) {
    if (err instanceof ApiError) {
      backupErrorMessage = err.message
    } else if (err instanceof Error) {
      backupErrorMessage = err.message
    } else {
      backupErrorMessage = 'Could not create backup.'
    }
  } finally {
    backingUp = false
    if (backupSlowTimer) clearTimeout(backupSlowTimer)
    backupSlowTimer = null
    showBackupSlowLoadingText = false
  }
}

// biome-ignore lint/correctness/noUnusedVariables: Used in Svelte markup; Biome doesn't detect template usage.
function formatBytes(bytes: number): string {
  const units = ['B', 'KB', 'MB', 'GB', 'TB'] as const
  let value = bytes
  let i = 0
  while (value >= 1024 && i < units.length - 1) {
    value /= 1024
    i += 1
  }
  const digits = i === 0 ? 0 : value < 10 ? 1 : 0
  return `${value.toFixed(digits)} ${units[i]}`
}

// biome-ignore lint/correctness/noUnusedVariables: Used in Svelte markup; Biome doesn't detect template usage.
function formatUptime(seconds: number): string {
  const days = Math.floor(seconds / 86400)
  const hours = Math.floor((seconds % 86400) / 3600)
  const minutes = Math.floor((seconds % 3600) / 60)
  if (days > 0) return `${days}d ${hours}h ${minutes}m`
  if (hours > 0) return `${hours}h ${minutes}m`
  return `${minutes}m`
}

async function loadHealth() {
  if (fetching) return
  fetching = true
  errorMessage = null

  if (!health) {
    showSkeleton = false
    showSlowLoadingText = false
    clearTimers()

    skeletonTimer = setTimeout(() => {
      showSkeleton = true
    }, 200)
    slowTimer = setTimeout(() => {
      showSlowLoadingText = true
    }, 2000)
  }

  try {
    health = await getAdminHealth()
    lastUpdatedAtMs = Date.now()
  } catch (err) {
    if (err instanceof ApiError) {
      errorMessage = err.message
    } else if (err instanceof Error) {
      errorMessage = err.message
    } else {
      errorMessage = 'Could not load health data.'
    }
  } finally {
    fetching = false
    clearTimers()
    showSkeleton = false
    showSlowLoadingText = false
  }
}

onMount(() => {
  void loadHealth()
  refreshInterval = setInterval(() => void loadHealth(), 30_000)
  clockInterval = setInterval(() => (nowMs = Date.now()), 1000)
})

onDestroy(() => {
  if (refreshInterval) clearInterval(refreshInterval)
  if (clockInterval) clearInterval(clockInterval)
  clearTimers()
  clearBackupTimers()
})
</script>

<div class="space-y-4">
  <header class="space-y-1">
    <h2 class="text-lg font-semibold tracking-tight">Admin</h2>
    <p class="text-sm text-muted-foreground">Instance health and resource usage.</p>
  </header>

  {#if errorMessage && !health}
    <div class="rounded-lg border border-border bg-card p-6">
      <p class="text-sm text-destructive">{errorMessage}</p>
      <button
        type="button"
        class="mt-4 inline-flex items-center justify-center rounded-md bg-fire px-4 py-2 text-sm font-medium text-fire-foreground transition-opacity hover:opacity-90"
        on:click={() => void loadHealth()}
      >
        Retry
      </button>
    </div>
  {:else if showSkeleton}
    <div class="grid grid-cols-1 gap-4 md:grid-cols-3">
      {#each ['Server', 'Database', 'Connections'] as title}
        <div class="rounded-lg border border-border bg-card p-4">
          <div class="animate-pulse space-y-3">
            <div class="h-4 w-24 rounded bg-muted" aria-hidden="true"></div>
            <div class="h-7 w-32 rounded bg-muted" aria-hidden="true"></div>
            <div class="h-4 w-40 rounded bg-muted" aria-hidden="true"></div>
          </div>
        </div>
      {/each}
    </div>
    {#if showSlowLoadingText}
      <p class="text-sm text-muted-foreground">Loading health data...</p>
    {/if}
  {:else if health}
    {#if errorMessage}
      <div
        class="rounded-md border border-destructive/30 bg-destructive/10 p-3 text-sm text-destructive"
        role="alert"
      >
        <div class="flex items-start justify-between gap-3">
          <p class="flex-1">{errorMessage}</p>
          <button
            type="button"
            class="inline-flex items-center justify-center rounded-md bg-fire px-3 py-1.5 text-xs font-medium text-fire-foreground transition-opacity hover:opacity-90 disabled:opacity-50"
            on:click={() => void loadHealth()}
            disabled={fetching}
          >
            Retry
          </button>
        </div>
      </div>
    {/if}

    <div class="grid grid-cols-1 gap-4 md:grid-cols-3">
      <div class="rounded-lg border border-border bg-card p-4">
        <h3 class="text-sm font-medium text-primary">Server</h3>
        <div class="mt-3 space-y-3">
          <div>
            <p class="text-xs text-muted-foreground">CPU</p>
            <div class="mt-1 flex items-center gap-3">
              <progress
                class="h-2 flex-1 overflow-hidden rounded bg-muted [&::-webkit-progress-bar]:bg-muted [&::-webkit-progress-value]:bg-primary [&::-moz-progress-bar]:bg-primary"
                max="100"
                value={Math.min(Math.max(health.cpuUsagePercent, 0), 100)}
              ></progress>
              <p class="w-16 text-right text-sm tabular-nums">
                {health.cpuUsagePercent.toFixed(1)}%
              </p>
            </div>
          </div>

          <div>
            <p class="text-xs text-muted-foreground">Memory (RSS)</p>
            <p class="mt-1 text-sm tabular-nums">{formatBytes(health.memoryRssBytes)}</p>
          </div>

          <div>
            <p class="text-xs text-muted-foreground">Uptime</p>
            <p class="mt-1 text-sm tabular-nums">{formatUptime(health.uptimeSeconds)}</p>
          </div>
        </div>
      </div>

      <div class="rounded-lg border border-border bg-card p-4">
        <h3 class="text-sm font-medium text-primary">Database</h3>
        <div class="mt-3 space-y-3">
          <div>
            <p class="text-xs text-muted-foreground">Size</p>
            <p class="mt-1 text-sm tabular-nums">{formatBytes(health.dbSizeBytes)}</p>
          </div>

          <div>
            <p class="text-xs text-muted-foreground">Pool</p>
            <p class="mt-1 text-sm tabular-nums">
              {health.dbPoolActive}/{health.dbPoolIdle}/{health.dbPoolMax}
              <span class="text-muted-foreground"> (active/idle/max)</span>
            </p>
          </div>
        </div>
      </div>

      <div class="rounded-lg border border-border bg-card p-4">
        <h3 class="text-sm font-medium text-primary">Connections</h3>
        <div class="mt-3 space-y-3">
          <div>
            <p class="text-xs text-muted-foreground">WebSocket</p>
            <p class="mt-1 text-sm tabular-nums">{health.websocketConnections}</p>
          </div>

          {#if lastUpdatedAtMs}
            <p class="text-xs text-muted-foreground">
              Last updated: {Math.max(0, Math.floor((nowMs - lastUpdatedAtMs) / 1000))}s
              ago
            </p>
          {/if}
        </div>
      </div>
    </div>

    <div class="rounded-lg border border-border bg-card p-6">
      <div class="flex flex-col gap-4 md:flex-row md:items-start md:justify-between">
        <div class="space-y-1">
          <h3 class="text-sm font-medium text-primary">Backup</h3>
          <p class="text-sm text-muted-foreground">
            Download a full database backup for restore or migration.
          </p>
        </div>

        <div class="flex flex-col items-start gap-2 md:items-end">
          <button
            type="button"
            class="inline-flex items-center justify-center rounded-md bg-fire px-4 py-2 text-sm font-medium text-fire-foreground transition-opacity hover:opacity-90 disabled:opacity-50"
            on:click={() => void runBackup()}
            disabled={backingUp}
          >
            Download Backup
          </button>

          {#if showBackupSlowLoadingText && backingUp}
            <p class="text-sm text-muted-foreground">Creating backup...</p>
          {:else if backupStatusMessage}
            <p class="text-sm text-muted-foreground">{backupStatusMessage}</p>
          {/if}
        </div>
      </div>

      {#if backupErrorMessage}
        <div
          class="mt-4 rounded-md border border-destructive/30 bg-destructive/10 p-3 text-sm text-destructive"
          role="alert"
        >
          <div class="flex items-start justify-between gap-3">
            <p class="flex-1">{backupErrorMessage}</p>
            <button
              type="button"
              class="inline-flex items-center justify-center rounded-md bg-fire px-3 py-1.5 text-xs font-medium text-fire-foreground transition-opacity hover:opacity-90 disabled:opacity-50"
              on:click={() => void runBackup()}
              disabled={backingUp}
            >
              Retry
            </button>
          </div>
        </div>
      {/if}
    </div>
  {/if}
</div>
