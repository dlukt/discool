import { beforeEach, describe, expect, it } from 'vitest'

import {
  clearLastLocation,
  getLastLocation,
  saveLastLocation,
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
})
