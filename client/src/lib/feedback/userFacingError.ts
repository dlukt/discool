import { ApiError } from '$lib/api'
import type { WsEnvelope, WsLifecycleState } from '$lib/ws/protocol'

const DEFAULT_FALLBACK_MESSAGE = 'Something went wrong. Please try again.'
const VALIDATION_FALLBACK_MESSAGE = 'Please check your input and try again.'

export const PERMISSION_DENIED_MESSAGE = "You don't have permission to do this."

export type ParsedWsError = {
  code: string
  message: string
}

export type UserFacingError = {
  code: string
  message: string
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null
}

function sanitizeMessage(message: string, fallback: string): string {
  const trimmed = message.trim()
  if (!trimmed) return fallback
  const lower = trimmed.toLowerCase()
  if (
    lower.includes('stack trace') ||
    lower.includes('sql') ||
    lower.includes('panic') ||
    lower.includes('protocol') ||
    lower.includes('exception')
  ) {
    return fallback
  }
  return trimmed
}

function messageForCode(
  code: string,
  rawMessage: string,
  fallback: string,
): string {
  switch (code) {
    case 'FORBIDDEN':
      return PERMISSION_DENIED_MESSAGE
    case 'UNAUTHORIZED':
      return 'You need to sign in to continue.'
    case 'NOT_FOUND':
      return "We couldn't find what you were looking for."
    case 'RATE_LIMITED':
      return 'Too many requests. Please wait a moment and try again.'
    case 'VALIDATION_ERROR':
      return sanitizeMessage(rawMessage, VALIDATION_FALLBACK_MESSAGE)
    case 'INTERNAL_ERROR':
      return DEFAULT_FALLBACK_MESSAGE
    default:
      return sanitizeMessage(rawMessage, fallback)
  }
}

export function parseWsErrorEnvelope(
  envelope: WsEnvelope | unknown,
): ParsedWsError | null {
  if (!isRecord(envelope) || envelope.op !== 'error') return null
  if (!isRecord(envelope.d)) return null
  const rawCode = envelope.d.code
  const rawMessage = envelope.d.message
  const code =
    typeof rawCode === 'string' && rawCode.trim()
      ? rawCode.trim().toUpperCase()
      : 'WS_ERROR'
  const message =
    typeof rawMessage === 'string' ? rawMessage : DEFAULT_FALLBACK_MESSAGE
  return { code, message }
}

export function toUserFacingError(
  error: unknown,
  fallback = DEFAULT_FALLBACK_MESSAGE,
): UserFacingError {
  if (error instanceof ApiError) {
    const code = error.code.trim().toUpperCase()
    return {
      code,
      message: messageForCode(code, error.message, fallback),
    }
  }

  const parsedWsError = parseWsErrorEnvelope(error)
  if (parsedWsError) {
    return {
      code: parsedWsError.code,
      message: messageForCode(
        parsedWsError.code,
        parsedWsError.message,
        fallback,
      ),
    }
  }

  if (isRecord(error)) {
    const code = typeof error.code === 'string' ? error.code.trim() : ''
    const message = typeof error.message === 'string' ? error.message : ''
    if (code) {
      const normalizedCode = code.toUpperCase()
      return {
        code: normalizedCode,
        message: messageForCode(normalizedCode, message, fallback),
      }
    }
  }

  if (error instanceof Error) {
    return {
      code: 'ERROR',
      message: sanitizeMessage(error.message, fallback),
    }
  }

  return {
    code: 'ERROR',
    message: fallback,
  }
}

export function connectionStatusText(state: WsLifecycleState): string | null {
  switch (state) {
    case 'connecting':
      return 'Connecting...'
    case 'reconnecting':
      return 'Connection lost. Reconnecting...'
    case 'disconnected':
      return 'Disconnected. Trying to reconnect...'
    case 'connected':
      return null
  }
}
