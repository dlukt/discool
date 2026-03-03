import { ApiError, setSessionToken, setUnauthorizedHandler } from '$lib/api'

import {
  clearStoredIdentity,
  decryptSecretKey,
  finalizeIdentityRegistration,
  loadStoredIdentity,
  restoreIdentityFromRecovery,
  signChallenge,
} from './crypto'
import {
  deleteMyAccount as deleteMyAccountApi,
  getProfile as getProfileApi,
  getRecoveryEmailStatus as getRecoveryEmailStatusApi,
  logout as logoutApi,
  recoverIdentityByToken as recoverIdentityByTokenApi,
  register as registerApi,
  requestChallenge,
  requestPersonalDataExport as requestPersonalDataExportApi,
  startIdentityRecovery as startIdentityRecoveryApi,
  startRecoveryEmailAssociation as startRecoveryEmailAssociationApi,
  updateProfile as updateProfileApi,
  uploadAvatar as uploadAvatarApi,
  verifyChallenge,
} from './identityApi'
import { clearLastLocation } from './navigationState'
import type {
  AuthSession,
  IdentityRecoveryStartResponse,
  PersonalDataExport,
  RecoveryEmailStatus,
  StoredIdentity,
  UpdateProfileInput,
} from './types'

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

function bytesToBase64(bytes: Uint8Array): string {
  let binary = ''
  for (let i = 0; i < bytes.length; i++) {
    binary += String.fromCharCode(bytes[i])
  }
  return btoa(binary)
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
  crossInstanceJoining: false,
  crossInstanceJoinError: null as string | null,
  recoveryEmailStatus: null as RecoveryEmailStatus | null,
  recoveryEmailLoading: false,
  recoveryEmailError: null as string | null,
  recoveryNudgeDismissed: false,

  initialize: async () => {
    identityState.loading = true
    identityState.error = null
    identityState.identityCorrupted = false
    identityState.identityNotRegistered = false
    identityState.crossInstanceJoining = false
    identityState.crossInstanceJoinError = null
    identityState.recoveryEmailStatus = null
    identityState.recoveryEmailLoading = false
    identityState.recoveryEmailError = null

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
          identityState.crossInstanceJoining = false
          identityState.crossInstanceJoinError = null
          identityState.recoveryEmailStatus = null
          identityState.recoveryEmailLoading = false
          identityState.recoveryEmailError = null
          clearLastLocation()
          setSessionToken(null)
          return
        }

        identityState.authError = null
        identityState.identityNotRegistered = false
        identityState.crossInstanceJoining = false
        identityState.crossInstanceJoinError = null
        identityState.recoveryEmailStatus = null
        identityState.recoveryEmailLoading = false
        identityState.recoveryEmailError = null
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
        identityState.crossInstanceJoining = false
        identityState.crossInstanceJoinError = null
        identityState.recoveryEmailStatus = null
        identityState.recoveryEmailLoading = false
        identityState.recoveryEmailError = null
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
        identityState.crossInstanceJoining = false
        identityState.crossInstanceJoinError = null
        identityState.recoveryEmailStatus = null
        identityState.recoveryEmailLoading = false
        identityState.recoveryEmailError = null
        setSessionToken(null)
      }
    } catch (err) {
      identityState.identity = null
      identityState.identityCorrupted = false
      identityState.identityNotRegistered = false
      identityState.crossInstanceJoining = false
      identityState.crossInstanceJoinError = null
      identityState.recoveryEmailStatus = null
      identityState.recoveryEmailLoading = false
      identityState.recoveryEmailError = null
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
      identityState.crossInstanceJoining = false
      identityState.crossInstanceJoinError = null
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
      identityState.crossInstanceJoining = false
      identityState.crossInstanceJoinError = null
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
      identityState.crossInstanceJoining = false
      identityState.crossInstanceJoinError = null
      identityState.recoveryEmailStatus = null
      identityState.recoveryEmailLoading = false
      identityState.recoveryEmailError = null
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
    identityState.crossInstanceJoinError = null

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
          identityState.crossInstanceJoinError = null
          identityState.recoveryEmailStatus = null
          identityState.recoveryEmailLoading = false
          identityState.recoveryEmailError = null
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
      void identityState.loadRecoveryEmailStatus()
    } catch (err) {
      if (epoch !== authEpoch) return
      identityState.session = null
      identityState.recoveryEmailStatus = null
      identityState.recoveryEmailLoading = false
      identityState.recoveryEmailError = null
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

  authenticateCrossInstance: async () => {
    if (identityState.crossInstanceJoining || identityState.authenticating)
      return
    const epoch = authEpoch
    identityState.crossInstanceJoining = true
    identityState.crossInstanceJoinError = null
    identityState.authError = null

    try {
      const identity = identityState.identity
      if (!identity) {
        identityState.session = null
        identityState.crossInstanceJoinError = 'No identity found'
        return
      }

      const sessionProfile = identityState.session?.user
      const profileDidMatchesIdentity =
        sessionProfile && sessionProfile.didKey === identity.didKey
      const displayNameHint = profileDidMatchesIdentity
        ? sessionProfile.displayName
        : identity.username
      const avatarColorHint = profileDidMatchesIdentity
        ? sessionProfile.avatarColor
        : identity.avatarColor

      const { challenge } = await requestChallenge(identity.didKey, {
        username: identity.username,
        displayName: displayNameHint,
        avatarColor: avatarColorHint,
      })
      if (epoch !== authEpoch) return
      const signature = await signChallenge(challenge)
      if (epoch !== authEpoch) return
      const session = await verifyChallenge(
        identity.didKey,
        challenge,
        signature,
        true,
      )
      if (epoch !== authEpoch) return

      identityState.session = session
      identityState.identityNotRegistered = false
      identityState.crossInstanceJoinError = null
      safeWriteSessionToStorage(session)
      setSessionToken(session.token)
      void identityState.loadRecoveryEmailStatus()
    } catch (err) {
      if (epoch !== authEpoch) return
      identityState.session = null
      identityState.recoveryEmailStatus = null
      identityState.recoveryEmailLoading = false
      identityState.recoveryEmailError = null
      setSessionToken(null)
      if (err instanceof Error) {
        identityState.crossInstanceJoinError = err.message
      } else {
        identityState.crossInstanceJoinError = 'Failed to join this instance'
      }
    } finally {
      if (epoch === authEpoch) {
        identityState.crossInstanceJoining = false
      }
    }
  },

  setSessionUser: (user: AuthSession['user']) => {
    const current = identityState.session
    if (!current) return
    identityState.session = { ...current, user }
    safeWriteSessionToStorage(identityState.session)
  },

  refreshProfile: async () => {
    if (!identityState.session) {
      throw new Error('No active session')
    }
    const profile = await getProfileApi()
    identityState.setSessionUser(profile)
    return profile
  },

  saveProfile: async (
    input: UpdateProfileInput,
    avatarFile: File | null = null,
  ) => {
    if (!identityState.session) {
      throw new Error('No active session')
    }

    const hasProfileUpdate = 'displayName' in input || 'avatarColor' in input
    let nextUser = identityState.session.user
    if (hasProfileUpdate) {
      nextUser = await updateProfileApi(input)
      identityState.setSessionUser(nextUser)
    }
    if (avatarFile) {
      nextUser = await uploadAvatarApi(avatarFile)
      identityState.setSessionUser(nextUser)
    }
    return nextUser
  },

  requestPersonalDataExport: async (): Promise<PersonalDataExport> => {
    if (!identityState.session) {
      throw new Error('No active session')
    }
    return requestPersonalDataExportApi()
  },

  loadRecoveryEmailStatus: async (): Promise<RecoveryEmailStatus | null> => {
    if (!identityState.session) {
      identityState.recoveryEmailStatus = null
      identityState.recoveryEmailLoading = false
      identityState.recoveryEmailError = null
      return null
    }

    identityState.recoveryEmailLoading = true
    identityState.recoveryEmailError = null
    try {
      const status = await getRecoveryEmailStatusApi()
      identityState.recoveryEmailStatus = status
      return status
    } catch (err) {
      if (err instanceof Error) {
        identityState.recoveryEmailError = err.message
      } else {
        identityState.recoveryEmailError =
          'Failed to load recovery email status'
      }
      return null
    } finally {
      identityState.recoveryEmailLoading = false
    }
  },

  startRecoveryEmailAssociation: async (email: string) => {
    if (!identityState.session) {
      throw new Error('No active session')
    }
    const normalizedEmail = email.trim()
    if (!normalizedEmail) {
      throw new Error('Email is required')
    }

    identityState.recoveryEmailLoading = true
    identityState.recoveryEmailError = null
    let secretKey: Uint8Array | null = null
    try {
      secretKey = await decryptSecretKey()
      const status = await startRecoveryEmailAssociationApi({
        email: normalizedEmail,
        encryptedPrivateKey: bytesToBase64(secretKey),
        encryptionContext: {
          algorithm: 'aes-256-gcm',
          version: 1,
        },
      })
      identityState.recoveryEmailStatus = status
      identityState.recoveryNudgeDismissed = false
      return status
    } catch (err) {
      if (err instanceof Error) {
        identityState.recoveryEmailError = err.message
      } else {
        identityState.recoveryEmailError =
          'Failed to send recovery verification email'
      }
      throw err
    } finally {
      secretKey?.fill(0)
      identityState.recoveryEmailLoading = false
    }
  },

  startIdentityRecovery: async (
    email: string,
  ): Promise<IdentityRecoveryStartResponse> => {
    const normalizedEmail = email.trim()
    if (!normalizedEmail) {
      throw new Error('Email is required')
    }
    return startIdentityRecoveryApi(normalizedEmail)
  },

  recoverIdentityByToken: async (token: string): Promise<void> => {
    const normalizedToken = token.trim()
    if (!normalizedToken) {
      throw new Error('token is required')
    }

    const payload = await recoverIdentityByTokenApi(normalizedToken)
    identityState.identity = await restoreIdentityFromRecovery(payload)
    identityState.identityCorrupted = false
    identityState.identityNotRegistered = false
    identityState.crossInstanceJoining = false
    identityState.crossInstanceJoinError = null
    await identityState.authenticate()
  },

  deleteAccount: async (confirmUsername: string): Promise<void> => {
    const session = identityState.session
    if (!session) {
      throw new Error('No active session')
    }
    if (confirmUsername !== session.user.username) {
      throw new Error('Username confirmation must match your username exactly')
    }

    await deleteMyAccountApi({ confirmUsername })

    authEpoch++
    identityState.session = null
    identityState.authenticating = false
    identityState.authError = 'Account deleted.'
    identityState.identityNotRegistered = false
    identityState.crossInstanceJoining = false
    identityState.crossInstanceJoinError = null
    identityState.recoveryEmailStatus = null
    identityState.recoveryEmailLoading = false
    identityState.recoveryEmailError = null
    safeRemoveSessionFromStorage()
    clearLastLocation()
    setSessionToken(null)
  },

  dismissRecoveryNudge: () => {
    identityState.recoveryNudgeDismissed = true
  },

  logout: async () => {
    authEpoch++
    const token = identityState.session?.token
    identityState.session = null
    identityState.authenticating = false
    identityState.authError = 'Signed out.'
    identityState.identityNotRegistered = false
    identityState.crossInstanceJoining = false
    identityState.crossInstanceJoinError = null
    identityState.recoveryEmailStatus = null
    identityState.recoveryEmailLoading = false
    identityState.recoveryEmailError = null
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
      identityState.crossInstanceJoining = false
      identityState.crossInstanceJoinError = null
      identityState.recoveryEmailStatus = null
      identityState.recoveryEmailLoading = false
      identityState.recoveryEmailError = null
      setSessionToken(null)
      return false
    }
    if (!raw) {
      identityState.session = null
      identityState.authError = null
      identityState.identityNotRegistered = false
      identityState.crossInstanceJoining = false
      identityState.crossInstanceJoinError = null
      identityState.recoveryEmailStatus = null
      identityState.recoveryEmailLoading = false
      identityState.recoveryEmailError = null
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
      const rawDisplayName = userRecord.displayName
      let displayName = username
      if (rawDisplayName !== undefined && rawDisplayName !== null) {
        if (typeof rawDisplayName !== 'string' || !rawDisplayName.trim()) {
          throw new Error('invalid session')
        }
        displayName = rawDisplayName
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
      const rawAvatarUrl = userRecord.avatarUrl
      let avatarUrl: string | null = null
      if (rawAvatarUrl !== undefined && rawAvatarUrl !== null) {
        if (typeof rawAvatarUrl !== 'string' || !rawAvatarUrl.trim()) {
          throw new Error('invalid session')
        }
        avatarUrl = rawAvatarUrl
      }

      identityState.session = {
        token,
        expiresAt,
        user: {
          id,
          didKey,
          username,
          displayName,
          avatarColor,
          avatarUrl,
          createdAt,
        },
      }
      identityState.authError = null
      identityState.identityNotRegistered = false
      identityState.crossInstanceJoining = false
      identityState.crossInstanceJoinError = null
      setSessionToken(token)
      void identityState.loadRecoveryEmailStatus()
      return true
    } catch {
      identityState.session = null
      identityState.authError = null
      identityState.identityNotRegistered = false
      identityState.crossInstanceJoining = false
      identityState.crossInstanceJoinError = null
      identityState.recoveryEmailStatus = null
      identityState.recoveryEmailLoading = false
      identityState.recoveryEmailError = null
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
  identityState.crossInstanceJoining = false
  identityState.crossInstanceJoinError = null
  identityState.recoveryEmailStatus = null
  identityState.recoveryEmailLoading = false
  identityState.recoveryEmailError = null
  safeRemoveSessionFromStorage()
  setSessionToken(null)
  void identityState.authenticate()
})
