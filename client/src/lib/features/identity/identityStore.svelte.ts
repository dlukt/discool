import { setSessionToken, setUnauthorizedHandler } from '$lib/api'

import {
  clearStoredIdentity,
  finalizeIdentityRegistration,
  loadStoredIdentity,
  signChallenge,
} from './crypto'
import {
  logout as logoutApi,
  register as registerApi,
  requestChallenge,
  verifyChallenge,
} from './identityApi'
import type { AuthSession, StoredIdentity } from './types'

export const identityState = $state({
  identity: null as StoredIdentity | null,
  loading: false,
  error: null as string | null,
  session: null as AuthSession | null,
  authenticating: false,
  authError: null as string | null,

  initialize: async () => {
    identityState.loading = true
    identityState.error = null
    try {
      identityState.identity = await loadStoredIdentity()
      if (identityState.identity) {
        const restored = await identityState.restoreSession()
        if (!restored) {
          void identityState.authenticate()
        }
      } else {
        identityState.session = null
        identityState.authError = null
        setSessionToken(null)
      }
    } catch (err) {
      identityState.identity = null
      identityState.error =
        err instanceof Error ? err.message : 'Failed to load identity'
      throw err
    } finally {
      identityState.loading = false
    }
  },

  register: async (
    didKey: string,
    username: string,
    avatarColor: string | null,
  ) => {
    identityState.loading = true
    identityState.error = null
    try {
      const user = await registerApi(didKey, username, avatarColor ?? undefined)
      identityState.identity = await finalizeIdentityRegistration(
        user.createdAt,
      )
      await identityState.authenticate()
    } catch (err) {
      identityState.identity = null
      identityState.error =
        err instanceof Error ? err.message : 'Failed to register identity'
      throw err
    } finally {
      identityState.loading = false
    }
  },

  clear: async () => {
    identityState.loading = true
    identityState.error = null
    try {
      await clearStoredIdentity()
      identityState.identity = null
      identityState.session = null
      identityState.authError = null
      sessionStorage.removeItem('discool-session')
      setSessionToken(null)
    } catch (err) {
      identityState.error =
        err instanceof Error ? err.message : 'Failed to clear identity'
      throw err
    } finally {
      identityState.loading = false
    }
  },

  authenticate: async () => {
    if (identityState.authenticating) return
    identityState.authenticating = true
    identityState.authError = null

    try {
      const identity = identityState.identity
      if (!identity) {
        identityState.session = null
        identityState.authError = 'No identity found'
        return
      }

      const { challenge } = await requestChallenge(identity.didKey)
      const signature = await signChallenge(challenge)
      const session = await verifyChallenge(
        identity.didKey,
        challenge,
        signature,
      )

      identityState.session = session
      sessionStorage.setItem('discool-session', JSON.stringify(session))
      setSessionToken(session.token)
    } catch (err) {
      identityState.session = null
      setSessionToken(null)
      if (err instanceof Error) {
        identityState.authError = err.message
      } else {
        identityState.authError = 'Failed to authenticate'
      }
    } finally {
      identityState.authenticating = false
    }
  },

  logout: async () => {
    const token = identityState.session?.token
    identityState.session = null
    identityState.authError = null
    sessionStorage.removeItem('discool-session')
    setSessionToken(null)

    if (!token) return
    await logoutApi(token)
  },

  restoreSession: async (): Promise<boolean> => {
    const raw = sessionStorage.getItem('discool-session')
    if (!raw) return false

    try {
      const parsed = JSON.parse(raw) as unknown
      if (!parsed || typeof parsed !== 'object') {
        throw new Error('invalid session')
      }

      const record = parsed as Record<string, unknown>
      const token = record.token
      const expiresAt = record.expiresAt
      const user = record.user
      if (
        typeof token !== 'string' ||
        !token.trim() ||
        typeof expiresAt !== 'string' ||
        !expiresAt.trim() ||
        !user ||
        typeof user !== 'object'
      ) {
        throw new Error('invalid session')
      }

      const expiresMs = Date.parse(expiresAt)
      if (!Number.isFinite(expiresMs) || expiresMs <= Date.now()) {
        throw new Error('session expired')
      }

      const userRecord = user as Record<string, unknown>
      const didKey = userRecord.didKey
      if (typeof didKey !== 'string' || !didKey.trim()) {
        throw new Error('invalid session')
      }
      const id = userRecord.id
      const username = userRecord.username
      const createdAt = userRecord.createdAt
      if (
        typeof id !== 'string' ||
        !id.trim() ||
        typeof username !== 'string' ||
        !username.trim() ||
        typeof createdAt !== 'string' ||
        !createdAt.trim()
      ) {
        throw new Error('invalid session')
      }

      if (identityState.identity && didKey !== identityState.identity.didKey) {
        throw new Error('session identity mismatch')
      }

      const avatarColor = userRecord.avatarColor
      if (typeof avatarColor !== 'string' && avatarColor !== null) {
        throw new Error('invalid session')
      }

      identityState.session = {
        token,
        expiresAt,
        user: {
          id,
          didKey,
          username,
          avatarColor: avatarColor ?? null,
          createdAt,
        },
      }
      identityState.authError = null
      setSessionToken(token)
      return true
    } catch {
      sessionStorage.removeItem('discool-session')
      setSessionToken(null)
      return false
    }
  },
})

setUnauthorizedHandler(() => {
  identityState.session = null
  identityState.authError = null
  sessionStorage.removeItem('discool-session')
  setSessionToken(null)
  void identityState.authenticate()
})
