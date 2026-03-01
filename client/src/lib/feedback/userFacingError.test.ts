import { describe, expect, it } from 'vitest'

import { ApiError } from '$lib/api'
import {
  connectionStatusText,
  PERMISSION_DENIED_MESSAGE,
  parseWsErrorEnvelope,
  toUserFacingError,
} from '$lib/feedback/userFacingError'
import type { WsEnvelope } from '$lib/ws/protocol'

describe('userFacingError', () => {
  it('maps forbidden API errors to the standard permission message', () => {
    const error = new ApiError('FORBIDDEN', 'Admin access required')
    const mapped = toUserFacingError(error)
    expect(mapped.message).toBe(PERMISSION_DENIED_MESSAGE)
  })

  it('maps websocket forbidden error envelopes to the standard permission message', () => {
    const envelope: WsEnvelope = {
      op: 'error',
      d: {
        code: 'FORBIDDEN',
        message: 'Some server-specific permission text',
        details: {},
      },
    }
    const parsed = parseWsErrorEnvelope(envelope)
    expect(parsed).not.toBeNull()
    const mapped = toUserFacingError(parsed)
    expect(mapped.message).toBe(PERMISSION_DENIED_MESSAGE)
  })

  it('maps lifecycle states to plain-language status text', () => {
    expect(connectionStatusText('connecting')).toBe('Connecting...')
    expect(connectionStatusText('reconnecting')).toBe(
      'Connection lost. Reconnecting...',
    )
    expect(connectionStatusText('disconnected')).toBe(
      'Disconnected. Trying to reconnect...',
    )
    expect(connectionStatusText('connected')).toBeNull()
  })
})
