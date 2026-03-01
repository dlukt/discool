export type ToastVariant = 'success' | 'info' | 'error'

export type ToastInput = {
  variant: ToastVariant
  message: string
  actionLabel?: string
  onAction?: () => void
  durationMs?: number
}

export type ToastItem = {
  id: string
  variant: ToastVariant
  message: string
  actionLabel?: string
  onAction?: () => void
  paused: boolean
  remainingMs: number | null
  dismissAt: number | null
}

const DEFAULT_AUTO_DISMISS_MS = 4_000
const MAX_VISIBLE_TOASTS = 3

const dismissTimers = new Map<string, ReturnType<typeof setTimeout>>()

function generateToastId(): string {
  if (
    typeof crypto !== 'undefined' &&
    typeof crypto.randomUUID === 'function'
  ) {
    return crypto.randomUUID()
  }
  return `toast-${Date.now()}-${Math.random().toString(16).slice(2)}`
}

function clearDismissTimer(id: string): void {
  const timer = dismissTimers.get(id)
  if (!timer) return
  clearTimeout(timer)
  dismissTimers.delete(id)
}

function removeToast(id: string): void {
  const next = toastState.toasts.filter((toast) => toast.id !== id)
  if (next.length === toastState.toasts.length) return
  toastState.toasts = next
  toastState.version += 1
}

function scheduleDismiss(id: string, delayMs: number): void {
  clearDismissTimer(id)
  dismissTimers.set(
    id,
    setTimeout(() => {
      clearDismissTimer(id)
      removeToast(id)
    }, delayMs),
  )
}

function clampDuration(value: number | undefined): number {
  if (typeof value !== 'number' || !Number.isFinite(value)) {
    return DEFAULT_AUTO_DISMISS_MS
  }
  return Math.max(250, Math.round(value))
}

function updateToast(
  id: string,
  updater: (toast: ToastItem) => ToastItem,
): void {
  let changed = false
  toastState.toasts = toastState.toasts.map((toast) => {
    if (toast.id !== id) return toast
    changed = true
    return updater(toast)
  })
  if (changed) {
    toastState.version += 1
  }
  if (!changed) {
    clearDismissTimer(id)
  }
}

function trimToVisibleLimit(): void {
  while (toastState.toasts.length > MAX_VISIBLE_TOASTS) {
    const oldest = toastState.toasts[0]
    if (!oldest) return
    clearDismissTimer(oldest.id)
    toastState.toasts = toastState.toasts.slice(1)
    toastState.version += 1
  }
}

export const toastState = $state({
  version: 0,
  toasts: [] as ToastItem[],

  show: (input: ToastInput): string => {
    const message = input.message.trim()
    if (!message) return ''

    const variant = input.variant
    const autoDismiss = variant !== 'error'
    const durationMs = autoDismiss ? clampDuration(input.durationMs) : null
    const id = generateToastId()
    const dismissAt = durationMs !== null ? Date.now() + durationMs : null

    toastState.toasts = [
      ...toastState.toasts,
      {
        id,
        variant,
        message,
        actionLabel: input.actionLabel?.trim() || undefined,
        onAction: input.onAction,
        paused: false,
        remainingMs: durationMs,
        dismissAt,
      },
    ]
    toastState.version += 1

    trimToVisibleLimit()

    if (durationMs !== null) {
      scheduleDismiss(id, durationMs)
    }

    return id
  },

  dismiss: (id: string): void => {
    clearDismissTimer(id)
    removeToast(id)
  },

  clearAll: (): void => {
    for (const id of dismissTimers.keys()) {
      clearDismissTimer(id)
    }
    if (toastState.toasts.length === 0) return
    toastState.toasts = []
    toastState.version += 1
  },

  pause: (id: string): void => {
    updateToast(id, (toast) => {
      if (toast.remainingMs === null || toast.paused) return toast
      const remaining =
        toast.dismissAt === null
          ? toast.remainingMs
          : toast.dismissAt - Date.now()
      clearDismissTimer(id)
      return {
        ...toast,
        paused: true,
        dismissAt: null,
        remainingMs: Math.max(1, Math.round(remaining)),
      }
    })
  },

  resume: (id: string): void => {
    updateToast(id, (toast) => {
      if (toast.remainingMs === null || !toast.paused) return toast
      const delayMs = Math.max(1, Math.round(toast.remainingMs))
      scheduleDismiss(id, delayMs)
      return {
        ...toast,
        paused: false,
        dismissAt: Date.now() + delayMs,
      }
    })
  },

  runAction: (id: string): void => {
    const toast = toastState.toasts.find((entry) => entry.id === id)
    toast?.onAction?.()
    toastState.dismiss(id)
  },
})
