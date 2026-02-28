import { beforeEach, describe, expect, it } from 'vitest'

import {
  clearLastLocation,
  getGuildOrder,
  getLastLocation,
  getLastViewedChannel,
  saveGuildOrder,
  saveLastLocation,
  saveLastViewedChannel,
} from './navigationState'

describe('navigationState', () => {
  beforeEach(() => {
    localStorage.clear()
  })

  it('round-trips last location in localStorage', () => {
    expect(getLastLocation()).toBeNull()
    saveLastLocation('/guild/abc/channel/def')
    expect(getLastLocation()).toBe('/guild/abc/channel/def')
    clearLastLocation()
    expect(getLastLocation()).toBeNull()
  })

  it('persists per-guild channel history and guild order', () => {
    expect(getLastViewedChannel('lobby')).toBeNull()
    saveLastViewedChannel('lobby', 'general')
    saveLastViewedChannel('makers', 'announcements')
    expect(getLastViewedChannel('lobby')).toBe('general')
    expect(getLastViewedChannel('makers')).toBe('announcements')

    saveGuildOrder(['makers', 'lobby', 'makers'])
    expect(getGuildOrder()).toEqual(['makers', 'lobby'])

    clearLastLocation()
    expect(getLastViewedChannel('lobby')).toBeNull()
    expect(getGuildOrder()).toEqual([])
  })
})
