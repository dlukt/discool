import { fireEvent, render, waitFor } from '@testing-library/svelte'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import ToastViewport from '$lib/components/ToastViewport.svelte'
import { toastState } from '$lib/feedback/toastStore.svelte'

describe('ToastViewport', () => {
  beforeEach(() => {
    vi.useFakeTimers()
    toastState.clearAll()
  })

  afterEach(() => {
    toastState.clearAll()
    vi.useRealTimers()
    vi.restoreAllMocks()
  })

  it('renders bottom-right stack and keeps at most three visible toasts', async () => {
    render(ToastViewport)
    toastState.show({ variant: 'info', message: 'one' })
    toastState.show({ variant: 'info', message: 'two' })
    toastState.show({ variant: 'info', message: 'three' })
    toastState.show({ variant: 'info', message: 'four' })

    const viewport = document.querySelector(
      '[data-testid="toast-viewport"]',
    ) as HTMLElement | null
    expect(viewport).toBeInTheDocument()
    expect(viewport?.className).toContain('bottom-4')
    expect(viewport?.className).toContain('right-4')

    await waitFor(() => {
      const items = document.querySelectorAll('[data-testid="toast-item"]')
      expect(items).toHaveLength(3)
    })
    expect(document.body.textContent).not.toContain('one')
    expect(document.body.textContent).toContain('four')
  })

  it('auto-dismisses success toasts after four seconds', async () => {
    render(ToastViewport)
    toastState.show({ variant: 'success', message: 'Saved' })

    await waitFor(() => {
      expect(document.body.textContent).toContain('Saved')
    })
    vi.advanceTimersByTime(3_999)
    expect(document.body.textContent).toContain('Saved')

    vi.advanceTimersByTime(1)
    await waitFor(() => {
      expect(document.body.textContent).not.toContain('Saved')
    })
  })

  it('keeps error toasts visible until dismissed', async () => {
    render(ToastViewport)
    toastState.show({ variant: 'error', message: 'Send failed' })

    await waitFor(() => {
      expect(document.body.textContent).toContain('Send failed')
    })

    vi.advanceTimersByTime(30_000)
    expect(document.body.textContent).toContain('Send failed')

    const dismissButton = document.querySelector(
      '[data-testid="toast-dismiss"]',
    ) as HTMLButtonElement | null
    expect(dismissButton).toBeInTheDocument()
    await fireEvent.click(dismissButton as HTMLButtonElement)
    expect(document.body.textContent).not.toContain('Send failed')
  })

  it('pauses and resumes auto-dismiss on hover', async () => {
    render(ToastViewport)
    toastState.show({ variant: 'info', message: 'Connection restored' })

    await waitFor(() => {
      expect(
        document.querySelector('[data-testid="toast-item"]'),
      ).toBeInTheDocument()
    })
    const toast = document.querySelector(
      '[data-testid="toast-item"]',
    ) as HTMLElement | null
    expect(toast).toBeInTheDocument()

    vi.advanceTimersByTime(2_000)
    await fireEvent.mouseEnter(toast as HTMLElement)
    vi.advanceTimersByTime(5_000)
    expect(document.body.textContent).toContain('Connection restored')

    await fireEvent.mouseLeave(toast as HTMLElement)
    vi.advanceTimersByTime(1_999)
    expect(document.body.textContent).toContain('Connection restored')

    vi.advanceTimersByTime(1)
    await waitFor(() => {
      expect(document.body.textContent).not.toContain('Connection restored')
    })
  })

  it('pauses and resumes auto-dismiss on focus', async () => {
    render(ToastViewport)
    toastState.show({ variant: 'info', message: 'Focused toast' })

    await waitFor(() => {
      expect(
        document.querySelector('[data-testid="toast-item"]'),
      ).toBeInTheDocument()
    })
    const toast = document.querySelector(
      '[data-testid="toast-item"]',
    ) as HTMLElement | null
    expect(toast).toBeInTheDocument()

    vi.advanceTimersByTime(2_000)
    await fireEvent.focusIn(toast as HTMLElement)
    vi.advanceTimersByTime(5_000)
    expect(document.body.textContent).toContain('Focused toast')

    await fireEvent.focusOut(toast as HTMLElement, {
      relatedTarget: document.body,
    })
    vi.advanceTimersByTime(1_999)
    expect(document.body.textContent).toContain('Focused toast')

    vi.advanceTimersByTime(1)
    await waitFor(() => {
      expect(document.body.textContent).not.toContain('Focused toast')
    })
  })
})
