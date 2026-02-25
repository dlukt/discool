import * as ed from '@noble/ed25519'
import { afterEach, beforeEach, describe, expect, it } from 'vitest'

import {
  base58btcEncode,
  didKeyFromPublicKey,
  encryptAndStoreKey,
  loadStoredIdentity,
  restoreIdentityFromRecovery,
  signChallenge,
} from './crypto'

type FakeOpenRequest = {
  result: unknown
  error: unknown | null
  onupgradeneeded: (() => void) | null
  onsuccess: (() => void) | null
  onerror: (() => void) | null
}

type FakeRequest<T> = {
  result: T
  error: unknown | null
  onsuccess: (() => void) | null
  onerror: (() => void) | null
}

type FakeTransaction = {
  oncomplete: (() => void) | null
  onabort: (() => void) | null
  onerror: (() => void) | null
  objectStore: (name: string) => {
    get: (key: string) => IDBRequest<unknown>
    put: (value: unknown, key: string) => IDBRequest<void>
    delete: (key: string) => IDBRequest<void>
  }
}

type FakeDb = {
  objectStoreNames: { contains: (name: string) => boolean }
  createObjectStore: (name: string) => unknown
  transaction: (name: string, mode: IDBTransactionMode) => IDBTransaction
  close: () => void
}

function installFakeIndexedDb(store: Map<string, unknown>) {
  const stores = new Set<string>()

  function makeOpenRequest(result: unknown): IDBOpenDBRequest {
    const req: FakeOpenRequest = {
      result,
      error: null,
      onupgradeneeded: null,
      onsuccess: null,
      onerror: null,
    }

    queueMicrotask(() => {
      req.onupgradeneeded?.()
      queueMicrotask(() => req.onsuccess?.())
    })

    return req as unknown as IDBOpenDBRequest
  }

  function makeRequest<T>(tx: FakeTransaction, result: T): IDBRequest<T> {
    const req: FakeRequest<T> = {
      result,
      error: null,
      onsuccess: null,
      onerror: null,
    }
    queueMicrotask(() => {
      req.onsuccess?.()
      // Fire completion after the awaited request resolves and `transactionDone()` installs handlers.
      queueMicrotask(() => tx.oncomplete?.())
    })
    return req as unknown as IDBRequest<T>
  }

  const db: FakeDb = {
    objectStoreNames: {
      contains: (name: string) => stores.has(name),
    },
    createObjectStore: (name: string) => {
      stores.add(name)
      return { name }
    },
    transaction: (_name: string, _mode: IDBTransactionMode) => {
      const tx: FakeTransaction = {
        oncomplete: null,
        onabort: null,
        onerror: null,
        objectStore: () => ({
          get: (key: string) =>
            makeRequest<unknown>(tx, store.get(key) ?? undefined),
          put: (value: unknown, key: string) => {
            store.set(key, value)
            return makeRequest<void>(tx, undefined)
          },
          delete: (key: string) => {
            store.delete(key)
            return makeRequest<void>(tx, undefined)
          },
        }),
      }

      return tx as unknown as IDBTransaction
    },
    close: () => {},
  }

  const original = globalThis.indexedDB
  globalThis.indexedDB = {
    open: () => makeOpenRequest(db),
  } as unknown as IDBFactory

  return () => {
    globalThis.indexedDB = original
  }
}

describe('crypto helpers', () => {
  it('base58btcEncode matches known vectors', () => {
    expect(base58btcEncode(new Uint8Array())).toBe('')
    expect(base58btcEncode(new Uint8Array([0]))).toBe('1')
    expect(base58btcEncode(new Uint8Array([0, 0]))).toBe('11')
    expect(base58btcEncode(new Uint8Array([1]))).toBe('2')
    expect(base58btcEncode(new TextEncoder().encode('Hello World'))).toBe(
      'JxF12TrwUP45BMd',
    )
  })

  it('didKeyFromPublicKey produces did:key:z6Mk... for Ed25519', () => {
    const publicKey = new Uint8Array(32).fill(1)
    expect(didKeyFromPublicKey(publicKey)).toBe(
      'did:key:z6MkeXBLjYiSvqnhFb6D7sHm8yKm4jV45wwBFRaatf1cfZ76',
    )
  })
})

describe('loadStoredIdentity', () => {
  let store: Map<string, unknown>
  let restore: () => void

  beforeEach(() => {
    store = new Map()
    restore = installFakeIndexedDb(store)
  })

  afterEach(() => {
    restore()
  })

  it('returns none when no record exists', async () => {
    await expect(loadStoredIdentity()).resolves.toEqual({ status: 'none' })
  })

  it('returns corrupted for invalid stored record', async () => {
    const wrappingKey = await crypto.subtle.generateKey(
      { name: 'AES-GCM', length: 256 },
      false,
      ['encrypt', 'decrypt'],
    )

    store.set('identity', {
      wrappingKey,
      encryptedSecretKey: new Uint8Array([1]).buffer,
      iv: new Uint8Array(12),
      publicKey: new Uint8Array(31),
      didKey: 'did:key:z6Mk-invalid',
      username: 'alice',
      avatarColor: '#3b82f6',
      registeredAt: '2026-02-24T00:00:00.000Z',
    })

    await expect(loadStoredIdentity()).resolves.toEqual({ status: 'corrupted' })
  })

  it('returns found for a valid stored record', async () => {
    const wrappingKey = await crypto.subtle.generateKey(
      { name: 'AES-GCM', length: 256 },
      false,
      ['encrypt', 'decrypt'],
    )

    const publicKey = new Uint8Array(32).fill(7)
    store.set('identity', {
      wrappingKey,
      encryptedSecretKey: new Uint8Array([1]).buffer,
      iv: new Uint8Array(12).fill(2),
      publicKey,
      didKey: 'did:key:z6Mk-valid',
      username: 'alice',
      avatarColor: '#3b82f6',
      registeredAt: '2026-02-24T00:00:00.000Z',
    })

    const result = await loadStoredIdentity()
    expect(result.status).toBe('found')
    if (result.status !== 'found') return
    expect(result.identity).toEqual({
      publicKey,
      didKey: 'did:key:z6Mk-valid',
      username: 'alice',
      avatarColor: '#3b82f6',
      registeredAt: '2026-02-24T00:00:00.000Z',
    })
  })
})

function bytesToBase64(bytes: Uint8Array): string {
  let binary = ''
  for (let i = 0; i < bytes.length; i++) {
    binary += String.fromCharCode(bytes[i])
  }
  return btoa(binary)
}

describe('restoreIdentityFromRecovery', () => {
  let store: Map<string, unknown>
  let restore: () => void

  beforeEach(() => {
    store = new Map()
    restore = installFakeIndexedDb(store)
  })

  afterEach(() => {
    restore()
  })

  it('restores identity record and returns stored identity', async () => {
    const { secretKey, publicKey } = await ed.keygenAsync()
    const didKey = didKeyFromPublicKey(publicKey)

    const restored = await restoreIdentityFromRecovery({
      didKey,
      username: 'alice',
      avatarColor: '#3b82f6',
      registeredAt: '2026-02-24T00:00:00.000Z',
      encryptedPrivateKey: bytesToBase64(secretKey),
      encryptionContext: {
        algorithm: 'aes-256-gcm',
        version: 1,
      },
    })

    expect(restored.didKey).toBe(didKey)
    expect(restored.username).toBe('alice')
  })

  it('rejects payload when key material does not match did key', async () => {
    const { secretKey } = await ed.keygenAsync()

    await expect(
      restoreIdentityFromRecovery({
        didKey: 'did:key:z6MkeXBLjYiSvqnhFb6D7sHm8yKm4jV45wwBFRaatf1cfZ76',
        username: 'alice',
        avatarColor: null,
        registeredAt: '2026-02-24T00:00:00.000Z',
        encryptedPrivateKey: bytesToBase64(secretKey),
        encryptionContext: {
          algorithm: 'aes-256-gcm',
          version: 1,
        },
      }),
    ).rejects.toThrow('Recovery payload does not match identity')
  })
})

function hexToBytes(hex: string): Uint8Array {
  const value = hex.trim()
  if (!value || value.length % 2 !== 0) throw new Error('invalid hex')
  const out = new Uint8Array(value.length / 2)
  for (let i = 0; i < out.length; i++) {
    const pair = value.slice(i * 2, i * 2 + 2)
    const byte = Number.parseInt(pair, 16)
    if (Number.isNaN(byte)) throw new Error('invalid hex')
    out[i] = byte
  }
  return out
}

describe('signChallenge', () => {
  let store: Map<string, unknown>
  let restore: () => void

  beforeEach(() => {
    store = new Map()
    restore = installFakeIndexedDb(store)
  })

  afterEach(() => {
    restore()
  })

  it('returns a valid hex signature for a stored secret key', async () => {
    const { secretKey, publicKey } = await ed.keygenAsync()
    const secretKeyCopy = new Uint8Array(secretKey)

    await encryptAndStoreKey(
      secretKey,
      publicKey,
      'did:key:z6Mk-test',
      'alice',
      null,
    )

    const challenge = 'a1'.repeat(32)
    const signatureHex = await signChallenge(challenge)
    expect(signatureHex).toMatch(/^[0-9a-f]{128}$/)

    const ok = await ed.verifyAsync(
      hexToBytes(signatureHex),
      new TextEncoder().encode(challenge),
      publicKey,
    )
    expect(ok).toBe(true)

    // ensure the original secret key passed to encryptAndStoreKey was wiped
    expect(Array.from(secretKey)).toEqual(Array(32).fill(0))
    expect(Array.from(secretKeyCopy)).not.toEqual(Array(32).fill(0))
  })

  it('rejects invalid challenge inputs', async () => {
    await expect(signChallenge('abc')).rejects.toThrow('Invalid challenge')
    await expect(signChallenge('g'.repeat(64))).rejects.toThrow('Invalid hex')
  })
})
