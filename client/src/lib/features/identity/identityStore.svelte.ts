import {
  clearStoredIdentity,
  finalizeIdentityRegistration,
  loadStoredIdentity,
} from './crypto'
import { register as registerApi } from './identityApi'
import type { StoredIdentity } from './types'

export const identityState = $state({
  identity: null as StoredIdentity | null,
  loading: false,
  error: null as string | null,

  initialize: async () => {
    identityState.loading = true
    identityState.error = null
    try {
      identityState.identity = await loadStoredIdentity()
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
    } catch (err) {
      identityState.error =
        err instanceof Error ? err.message : 'Failed to clear identity'
      throw err
    } finally {
      identityState.loading = false
    }
  },
})
