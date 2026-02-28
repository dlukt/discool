import { fireEvent, render, waitFor } from '@testing-library/svelte'
import { beforeEach, describe, expect, it, vi } from 'vitest'

import ShellRoute from './ShellRoute.svelte'

type RenderProps = {
  mode: 'home' | 'channel' | 'settings' | 'admin'
  route: {
    result: {
      path: {
        condition: 'exact-match'
        original: string
        params?: Record<string, string>
      }
      querystring: {
        condition: 'exact-match'
        original: Record<
          string,
          string | number | boolean | string[] | number[] | boolean[]
        >
        params?: Record<
          string,
          string | number | boolean | string[] | number[] | boolean[]
        >
      }
      status: number
    }
  }
  isAdmin: boolean
  displayName: string
  showRecoveryNudge: boolean
  onOpenSettings: () => void
  onDismissRecoveryNudge: () => void
  onLogout: () => void
  onRouteResolved: (path: string) => void
}

function setViewport(width: number) {
  Object.defineProperty(window, 'innerWidth', {
    value: width,
    writable: true,
    configurable: true,
  })
  window.dispatchEvent(new Event('resize'))
}

function buildProps(overrides: Partial<RenderProps> = {}): RenderProps {
  return {
    mode: 'channel',
    route: {
      result: {
        path: {
          condition: 'exact-match',
          original: '/lobby/general',
          params: { guild: 'lobby', channel: 'general' },
        },
        querystring: {
          condition: 'exact-match',
          original: {},
          params: {},
        },
        status: 200,
      },
    },
    isAdmin: false,
    displayName: 'Darko',
    showRecoveryNudge: false,
    onOpenSettings: vi.fn(),
    onDismissRecoveryNudge: vi.fn(),
    onLogout: vi.fn(),
    onRouteResolved: vi.fn(),
    ...overrides,
  }
}

describe('ShellRoute', () => {
  beforeEach(() => {
    setViewport(1280)
  })

  it('renders skip link as the first focusable element', async () => {
    const props = buildProps()
    const { container } = render(ShellRoute, props)
    const skipLink = container.querySelector('a[href="#main-content"]')
    const firstFocusable = container.querySelector(
      'a[href],button,[tabindex]:not([tabindex="-1"])',
    )

    expect(skipLink).toBeInTheDocument()
    expect(firstFocusable).toBe(skipLink)

    await waitFor(() => {
      expect(container.querySelector('#main-content')).toHaveFocus()
    })
  })

  it('shows tablet member list only after toggle', async () => {
    setViewport(900)
    const props = buildProps()
    const { getByRole, queryByTestId, findByTestId } = render(ShellRoute, props)

    expect(queryByTestId('tablet-member-list')).not.toBeInTheDocument()
    await fireEvent.click(getByRole('button', { name: 'Toggle members' }))
    expect(await findByTestId('tablet-member-list')).toBeInTheDocument()
  })

  it('shows mobile drill-down with bottom navigation', async () => {
    setViewport(600)
    const props = buildProps()
    const { getByRole, queryByRole } = render(ShellRoute, props)

    expect(
      getByRole('navigation', { name: 'Mobile shell navigation' }),
    ).toBeInTheDocument()
    expect(getByRole('heading', { name: 'Messages' })).toBeInTheDocument()

    await fireEvent.click(getByRole('button', { name: 'Members' }))
    expect(queryByRole('heading', { name: 'Messages' })).not.toBeInTheDocument()
    expect(getByRole('heading', { name: 'Members' })).toBeInTheDocument()
  })

  it('shows invite action only in channel mode', async () => {
    const props = buildProps()
    const view = render(ShellRoute, props)
    expect(
      view.getByRole('button', { name: 'Invite people' }),
    ).toBeInTheDocument()

    await view.rerender(
      buildProps({
        mode: 'settings',
        route: {
          result: {
            path: {
              condition: 'exact-match',
              original: '/settings',
              params: {},
            },
            querystring: {
              condition: 'exact-match',
              original: {},
              params: {},
            },
            status: 200,
          },
        },
      }),
    )

    expect(
      view.queryByRole('button', { name: 'Invite people' }),
    ).not.toBeInTheDocument()
  })

  it('renders GuildRail home button in channel mode', () => {
    const props = buildProps()
    const view = render(ShellRoute, props)
    expect(view.getByRole('button', { name: 'Home' })).toBeInTheDocument()
  })

  it('emits route path changes for persistence integration', async () => {
    const onRouteResolved = vi.fn()
    const props = buildProps({ onRouteResolved })
    const view = render(ShellRoute, props)

    await waitFor(() => {
      expect(onRouteResolved).toHaveBeenCalledWith('/lobby/general')
    })

    await view.rerender(
      buildProps({
        onRouteResolved,
        route: {
          result: {
            path: {
              condition: 'exact-match',
              original: '/engineering/announcements',
              params: { guild: 'engineering', channel: 'announcements' },
            },
            querystring: {
              condition: 'exact-match',
              original: {},
              params: {},
            },
            status: 200,
          },
        },
      }),
    )

    await waitFor(() => {
      expect(onRouteResolved).toHaveBeenCalledWith('/engineering/announcements')
    })
  })
})
