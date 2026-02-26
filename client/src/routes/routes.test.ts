import { describe, expect, it } from 'vitest'

import {
  createAuthenticatedRoutes,
  isPersistableLocation,
  resolveInitialLocation,
} from './routes'

describe('routes', () => {
  it('restores last persisted guild/channel path from root', () => {
    expect(resolveInitialLocation('/', '/lobby/general')).toBe('/lobby/general')
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
    expect(isPersistableLocation('/')).toBe(false)
    expect(isPersistableLocation('/settings')).toBe(false)
    expect(isPersistableLocation('/admin')).toBe(false)
  })

  it('includes admin route only for admin users', () => {
    const adminRoutes = createAuthenticatedRoutes(true)
    const memberRoutes = createAuthenticatedRoutes(false)

    expect(adminRoutes.some((route) => route.path === '/admin')).toBe(true)
    expect(memberRoutes.some((route) => route.path === '/admin')).toBe(false)
    expect(adminRoutes.some((route) => route.path === '/:guild/:channel')).toBe(
      true,
    )
  })
})
