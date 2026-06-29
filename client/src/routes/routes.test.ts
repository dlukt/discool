import { describe, expect, it } from 'vitest'

import {
  createAuthenticatedRoutes,
  isPersistableLocation,
  resolveInitialLocation,
} from './routes'

describe('routes', () => {
  it('restores last persisted guild/channel path from root', () => {
    expect(resolveInitialLocation('/', '/lobby/general')).toBe('/lobby/general')
    expect(resolveInitialLocation('/', '/dm/abc123')).toBe('/dm/abc123')
  })

  it('ignores non-persistable targets for restore', () => {
    expect(resolveInitialLocation('/', '/settings')).toBe('/')
    expect(resolveInitialLocation('/', '/admin')).toBe('/')
    expect(resolveInitialLocation('/', '/')).toBe('/')
  })

  it('keeps current deep-link path when it is not root', () => {
    expect(
      resolveInitialLocation('/engineering/announcements', '/lobby/general'),
    ).toBe('/engineering/announcements')
  })

  it('only persists guild/channel locations', () => {
    expect(isPersistableLocation('/lobby/general')).toBe(true)
    expect(isPersistableLocation('/dm/abc123')).toBe(true)
    expect(isPersistableLocation('/')).toBe(false)
    expect(isPersistableLocation('/settings')).toBe(false)
    expect(isPersistableLocation('/admin')).toBe(false)
  })

  it('includes admin route only for admin users', () => {
    const adminRoutes = createAuthenticatedRoutes(true)
    const memberRoutes = createAuthenticatedRoutes(false)

    expect(adminRoutes.some((route) => route.path === '/admin')).toBe(true)
    expect(memberRoutes.some((route) => route.path === '/admin')).toBe(false)

    // guild/channel and dm routes use regex named groups: this router does not
    // treat ":param" in string paths as parameters, so verify the routes match
    // real paths and capture the params ShellRoute reads (guild/channel/dm).
    const byMode = (mode: string) =>
      adminRoutes.find(
        (route) =>
          route.path instanceof RegExp &&
          (route as { props?: { mode?: string } }).props?.mode === mode,
      ) as { path: RegExp } | undefined

    expect(byMode('channel')?.path.exec('/lobby/general')?.groups).toEqual({
      guild: 'lobby',
      channel: 'general',
    })
    expect(byMode('dm')?.path.exec('/dm/abc123')?.groups).toEqual({
      dm: 'abc123',
    })
  })
})
