import { ApiError, setSessionToken, setUnauthorizedHandler } from '$lib/api'

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
import { clearLastLocation } from './navigationState'
import type { AuthSession, StoredIdentity } from './types'

const SESSION_KEY = 'discool-session'
let storageListenerInstalled = false
let authEpoch = 0

function safeRemoveSessionFromStorage(): void {
  try {
    localStorage.removeItem(SESSION_KEY)
  } catch {
    // best-effort
    return
  }
}

function safeWriteSessionToStorage(session: AuthSession): void {
  try {
    localStorage.setItem(SESSION_KEY, JSON.stringify(session))
  } catch {
    // best-effort
    return
  }
}

export const identityState = $state({
  identity: null as StoredIdentity | null,
  identityCorrupted: false,
  identityNotRegistered: false,
  loading: false,
  error: null as string | null,
  session: null as AuthSession | null,
  authenticating: false,
  authError: null as string | null,

  initialize: async () => {
    identityState.loading = true
    identityState.error = null
    identityState.identityCorrupted = false
    identityState.identityNotRegistered = false

    if (typeof window !== 'undefined' && !storageListenerInstalled) {
      storageListenerInstalled = true
      window.addEventListener('storage', async (event) => {
        if (event.key !== SESSION_KEY) return

        // Another tab changed the session; invalidate any in-flight auth attempt.
        authEpoch++
        const epoch = authEpoch

        if (event.newValue === null) {
          identityState.session = null
          identityState.authenticating = false
          identityState.authError = 'Signed out in another tab.'
          identityState.identityNotRegistered = false
          clearLastLocation()
          setSessionToken(null)
          return
        }

        identityState.authError = null
        identityState.identityNotRegistered = false
        identityState.authenticating = false
        const restored = await identityState.restoreSession()
        if (epoch !== authEpoch) return
        if (!restored && identityState.identity) {
          void identityState.authenticate()
        }
      })
    }

    try {
      const loaded = await loadStoredIdentity()
      if (loaded.status === 'found') {
        identityState.identity = loaded.identity
      } else if (loaded.status === 'corrupted') {
        authEpoch++
        identityState.identity = null
        identityState.identityCorrupted = true
        identityState.session = null
        identityState.authenticating = false
        identityState.authError = null
        safeRemoveSessionFromStorage()
        clearLastLocation()
        setSessionToken(null)
        return
      } else {
        identityState.identity = null
      }

      const restored = await identityState.restoreSession()
      if (!restored && identityState.identity) {
        void identityState.authenticate()
      }
      if (!restored && !identityState.identity) {
        identityState.session = null
        identityState.authError = null
        setSessionToken(null)
      }
    } catch (err) {
      identityState.identity = null
      identityState.identityCorrupted = false
      identityState.identityNotRegistered = false
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
      identityState.identityCorrupted = false
      identityState.identityNotRegistered = false
      await identityState.authenticate()
    } catch (err) {
      identityState.error =
        err instanceof Error ? err.message : 'Failed to register identity'
      throw err
    } finally {
      identityState.loading = false
    }
  },

  reRegister: async (username: string, avatarColor: string | null) => {
    identityState.loading = true
    identityState.error = null
    try {
      const identity = identityState.identity
      if (!identity) {
        throw new Error('No identity found')
      }

      const user = await registerApi(
        identity.didKey,
        username,
        avatarColor ?? undefined,
      )
      identityState.identity = await finalizeIdentityRegistration(
        user.createdAt,
        { username, avatarColor },
      )
      identityState.identityCorrupted = false
      identityState.identityNotRegistered = false
      await identityState.authenticate()
    } catch (err) {
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
      authEpoch++
      await clearStoredIdentity()
      identityState.identity = null
      identityState.identityCorrupted = false
      identityState.identityNotRegistered = false
      identityState.session = null
      identityState.authenticating = false
      identityState.authError = null
      safeRemoveSessionFromStorage()
      clearLastLocation()
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
    const epoch = authEpoch
    identityState.authenticating = true
    identityState.authError = null
    identityState.identityNotRegistered = false

    try {
      const identity = identityState.identity
      if (!identity) {
        identityState.session = null
        identityState.authError = 'No identity found'
        return
      }

      let challenge: string
      try {
        ;({ challenge } = await requestChallenge(identity.didKey))
      } catch (err) {
        if (epoch !== authEpoch) return
        if (err instanceof ApiError && err.code === 'NOT_FOUND') {
          identityState.session = null
          identityState.authError = null
          identityState.identityNotRegistered = true
          setSessionToken(null)
          return
        }
        throw err
      }
      if (epoch !== authEpoch) return
      const signature = await signChallenge(challenge)
      if (epoch !== authEpoch) return
      const session = await verifyChallenge(
        identity.didKey,
        challenge,
        signature,
      )
      if (epoch !== authEpoch) return

      identityState.session = session
      safeWriteSessionToStorage(session)
      setSessionToken(session.token)
    } catch (err) {
      if (epoch !== authEpoch) return
      identityState.session = null
      setSessionToken(null)
      if (err instanceof Error) {
        identityState.authError = err.message
      } else {
        identityState.authError = 'Failed to authenticate'
      }
    } finally {
      if (epoch === authEpoch) {
        identityState.authenticating = false
      }
    }
  },

  logout: async () => {
    authEpoch++
    const token = identityState.session?.token
    identityState.session = null
    identityState.authenticating = false
    identityState.authError = 'Signed out.'
    identityState.identityNotRegistered = false
    safeRemoveSessionFromStorage()
    clearLastLocation()
    setSessionToken(null)

    if (!token) return
    try {
      await logoutApi(token)
    } catch {
      identityState.authError =
        'Signed out, but the server could not be reached.'
    }
  },

  restoreSession: async (): Promise<boolean> => {
    let raw: string | null
    try {
      raw = localStorage.getItem(SESSION_KEY)
    } catch {
      identityState.session = null
      identityState.authError = null
      identityState.identityNotRegistered = false
      setSessionToken(null)
      return false
    }
    if (!raw) {
      identityState.session = null
      identityState.authError = null
      identityState.identityNotRegistered = false
      setSessionToken(null)
      return false
    }

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

      const rawAvatarColor = userRecord.avatarColor
      let avatarColor: string | null = null
      if (rawAvatarColor !== undefined && rawAvatarColor !== null) {
        if (typeof rawAvatarColor !== 'string') {
          throw new Error('invalid session')
        }
        if (!/^#[0-9a-fA-F]{6}$/.test(rawAvatarColor)) {
          throw new Error('invalid session')
        }
        avatarColor = rawAvatarColor
      }

      identityState.session = {
        token,
        expiresAt,
        user: {
          id,
          didKey,
          username,
          avatarColor,
          createdAt,
        },
      }
      identityState.authError = null
      identityState.identityNotRegistered = false
      setSessionToken(token)
      return true
    } catch {
      identityState.session = null
      identityState.authError = null
      identityState.identityNotRegistered = false
      safeRemoveSessionFromStorage()
      setSessionToken(null)
      return false
    }
  },
})

setUnauthorizedHandler(() => {
  authEpoch++
  identityState.session = null
  identityState.authenticating = false
  identityState.authError = null
  identityState.identityNotRegistered = false
  safeRemoveSessionFromStorage()
  setSessionToken(null)
  void identityState.authenticate()
})
