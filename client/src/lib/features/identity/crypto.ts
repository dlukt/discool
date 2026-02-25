import * as ed from '@noble/ed25519'

import type { RecoveryIdentityPayload, StoredIdentity } from './types'

export type LoadStoredIdentityResult =
  | { status: 'none' }
  | { status: 'corrupted' }
  | { status: 'found'; identity: StoredIdentity }

type IdentityRecord = {
  wrappingKey: CryptoKey
  encryptedSecretKey: ArrayBuffer
  iv: Uint8Array<ArrayBuffer>
  publicKey: Uint8Array<ArrayBuffer>
  didKey: string
  username: string
  avatarColor: string | null
  registeredAt: string | null
}

const IDENTITY_DB_NAME = 'discool-identity'
const IDENTITY_DB_VERSION = 1
const IDENTITY_STORE = 'keys'
const IDENTITY_KEY = 'identity'

function isCryptoKeyLike(value: unknown): value is CryptoKey {
  if (!value || typeof value !== 'object') return false
  const key = value as CryptoKey
  const algo = key.algorithm as { name?: unknown } | undefined
  return (
    typeof key.type === 'string' &&
    typeof key.extractable === 'boolean' &&
    Array.isArray(key.usages) &&
    typeof algo?.name === 'string'
  )
}

export async function generateIdentity(): Promise<{
  secretKey: Uint8Array
  publicKey: Uint8Array
  didKey: string
}> {
  const { secretKey, publicKey } = await ed.keygenAsync()
  return { secretKey, publicKey, didKey: didKeyFromPublicKey(publicKey) }
}

export function didKeyFromPublicKey(publicKey: Uint8Array): string {
  const bytes = new Uint8Array(2 + publicKey.length)
  bytes[0] = 0xed
  bytes[1] = 0x01
  bytes.set(publicKey, 2)

  return `did:key:z${base58btcEncode(bytes)}`
}

export function base58btcEncode(bytes: Uint8Array): string {
  if (bytes.length === 0) return ''
  const alphabet = '123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz'

  let zeros = 0
  while (zeros < bytes.length && bytes[zeros] === 0) zeros++
  if (zeros === bytes.length) return '1'.repeat(zeros)

  const digits: number[] = [0]
  for (let i = zeros; i < bytes.length; i++) {
    let carry = bytes[i]
    for (let j = 0; j < digits.length; j++) {
      const x = digits[j] * 256 + carry
      digits[j] = x % 58
      carry = Math.floor(x / 58)
    }
    while (carry > 0) {
      digits.push(carry % 58)
      carry = Math.floor(carry / 58)
    }
  }

  let out = '1'.repeat(zeros)
  for (let i = digits.length - 1; i >= 0; i--) out += alphabet[digits[i]]
  return out
}

export async function encryptAndStoreKey(
  secretKey: Uint8Array,
  publicKey: Uint8Array,
  didKey: string,
  username: string,
  avatarColor: string | null,
): Promise<void> {
  const iv = crypto.getRandomValues(new Uint8Array(12))
  const wrappingKey = await crypto.subtle.generateKey(
    { name: 'AES-GCM', length: 256 },
    false,
    ['encrypt', 'decrypt'],
  )

  const secretKeyCopy = new Uint8Array(secretKey)
  try {
    const encryptedSecretKey = await crypto.subtle.encrypt(
      { name: 'AES-GCM', iv },
      wrappingKey,
      secretKeyCopy,
    )

    const record: IdentityRecord = {
      wrappingKey,
      encryptedSecretKey,
      iv: new Uint8Array(iv),
      publicKey: new Uint8Array(publicKey),
      didKey,
      username,
      avatarColor,
      registeredAt: null,
    }

    await putIdentityRecord(record)
  } finally {
    // Minimize memory exposure window.
    secretKey.fill(0)
    secretKeyCopy.fill(0)
  }
}

export async function finalizeIdentityRegistration(
  registeredAt: string,
  profile?: { username?: string; avatarColor?: string | null },
): Promise<StoredIdentity> {
  const record = await getIdentityRecord()
  if (!record) {
    throw new Error('No stored identity to finalize')
  }

  const next: IdentityRecord = {
    ...record,
    username: profile?.username ?? record.username,
    avatarColor: profile?.avatarColor ?? record.avatarColor,
    registeredAt,
  }
  await putIdentityRecord(next)

  return {
    publicKey: next.publicKey,
    didKey: next.didKey,
    username: next.username,
    avatarColor: next.avatarColor,
    registeredAt,
  }
}

export async function loadStoredIdentity(): Promise<LoadStoredIdentityResult> {
  let record: IdentityRecord | null = null
  try {
    record = await getIdentityRecord()
  } catch {
    // IndexedDB can fail if storage was cleared or object stores are missing.
    return { status: 'corrupted' }
  }
  if (!record || !record.registeredAt) return { status: 'none' }

  if (
    !(record.iv instanceof Uint8Array) ||
    record.iv.length !== 12 ||
    !(record.encryptedSecretKey instanceof ArrayBuffer) ||
    record.encryptedSecretKey.byteLength === 0 ||
    !isCryptoKeyLike(record.wrappingKey) ||
    (record.wrappingKey.algorithm as { name?: unknown }).name !== 'AES-GCM' ||
    !record.wrappingKey.usages.includes('decrypt')
  ) {
    return { status: 'corrupted' }
  }

  if (
    !(record.publicKey instanceof Uint8Array) ||
    record.publicKey.length !== 32
  ) {
    return { status: 'corrupted' }
  }
  if (
    typeof record.didKey !== 'string' ||
    !record.didKey.startsWith('did:key:z6Mk')
  ) {
    return { status: 'corrupted' }
  }
  if (typeof record.username !== 'string' || !record.username.trim()) {
    return { status: 'corrupted' }
  }
  if (typeof record.registeredAt !== 'string' || !record.registeredAt.trim()) {
    return { status: 'corrupted' }
  }
  if (!Number.isFinite(Date.parse(record.registeredAt))) {
    return { status: 'corrupted' }
  }
  if (record.avatarColor !== null) {
    if (typeof record.avatarColor !== 'string') return { status: 'corrupted' }
    if (!/^#[0-9a-fA-F]{6}$/.test(record.avatarColor)) {
      return { status: 'corrupted' }
    }
  }

  return {
    status: 'found',
    identity: {
      publicKey: record.publicKey,
      didKey: record.didKey,
      username: record.username,
      avatarColor: record.avatarColor,
      registeredAt: record.registeredAt,
    },
  }
}

export async function decryptSecretKey(): Promise<Uint8Array> {
  const record = await getIdentityRecord()
  if (!record) {
    throw new Error('No stored identity')
  }

  const decrypted = await crypto.subtle.decrypt(
    { name: 'AES-GCM', iv: record.iv },
    record.wrappingKey,
    record.encryptedSecretKey,
  )

  return new Uint8Array(decrypted)
}

function fromHexChar(code: number): number | null {
  if (code >= 48 && code <= 57) return code - 48 // 0-9
  if (code >= 97 && code <= 102) return code - 97 + 10 // a-f
  if (code >= 65 && code <= 70) return code - 65 + 10 // A-F
  return null
}

function hexToBytes(hex: string): Uint8Array {
  const value = hex.trim()
  if (!value || value.length % 2 !== 0) {
    throw new Error('Invalid hex string')
  }

  const out = new Uint8Array(value.length / 2)
  for (let i = 0; i < out.length; i++) {
    const h1 = fromHexChar(value.charCodeAt(i * 2))
    const h2 = fromHexChar(value.charCodeAt(i * 2 + 1))
    if (h1 === null || h2 === null) {
      throw new Error('Invalid hex string')
    }
    out[i] = (h1 << 4) | h2
  }
  return out
}

function bytesToHex(bytes: Uint8Array): string {
  const alphabet = '0123456789abcdef'
  let out = ''
  for (let i = 0; i < bytes.length; i++) {
    const b = bytes[i]
    out += alphabet[(b >> 4) & 0xf]
    out += alphabet[b & 0xf]
  }
  return out
}

export async function signChallenge(challengeHex: string): Promise<string> {
  const challenge = challengeHex.trim()
  if (challenge.length !== 64) {
    throw new Error('Invalid challenge')
  }
  // Validate that we were given a hex challenge without changing what we sign.
  hexToBytes(challenge)

  const secretKey = await decryptSecretKey()
  try {
    const message = new TextEncoder().encode(challenge)
    const signature = await ed.signAsync(message, secretKey)
    return bytesToHex(signature)
  } finally {
    secretKey.fill(0)
  }
}

export async function clearStoredIdentity(): Promise<void> {
  const db = await openIdentityDb()
  try {
    const tx = db.transaction(IDENTITY_STORE, 'readwrite')
    tx.objectStore(IDENTITY_STORE).delete(IDENTITY_KEY)
    await transactionDone(tx)
  } finally {
    db.close()
  }
}

function base64ToBytes(value: string): Uint8Array {
  const trimmed = value.trim()
  if (!trimmed) {
    throw new Error('Recovery payload is missing key material')
  }

  let binary: string
  try {
    binary = atob(trimmed)
  } catch {
    throw new Error('Recovery payload is invalid')
  }

  const out = new Uint8Array(binary.length)
  for (let i = 0; i < binary.length; i++) {
    out[i] = binary.charCodeAt(i)
  }
  return out
}

export async function restoreIdentityFromRecovery(
  payload: RecoveryIdentityPayload,
): Promise<StoredIdentity> {
  const didKey = payload.didKey.trim()
  if (!didKey) {
    throw new Error('Recovery payload is missing did key')
  }
  const username = payload.username.trim()
  if (!username) {
    throw new Error('Recovery payload is missing username')
  }
  const registeredAt = payload.registeredAt.trim()
  if (!registeredAt) {
    throw new Error('Recovery payload is missing registration timestamp')
  }

  const secretKey = base64ToBytes(payload.encryptedPrivateKey)
  if (secretKey.length !== 32) {
    secretKey.fill(0)
    throw new Error('Recovery payload has invalid key length')
  }

  const secretKeyCopy = new Uint8Array(secretKey)
  try {
    const publicKey = await ed.getPublicKeyAsync(secretKeyCopy)
    const expectedDidKey = didKeyFromPublicKey(publicKey)
    if (expectedDidKey !== didKey) {
      throw new Error('Recovery payload does not match identity')
    }

    await encryptAndStoreKey(
      secretKeyCopy,
      publicKey,
      didKey,
      username,
      payload.avatarColor,
    )
    return finalizeIdentityRegistration(registeredAt, {
      username,
      avatarColor: payload.avatarColor,
    })
  } finally {
    secretKey.fill(0)
    secretKeyCopy.fill(0)
  }
}

async function getIdentityRecord(): Promise<IdentityRecord | null> {
  const db = await openIdentityDb()
  try {
    const tx = db.transaction(IDENTITY_STORE, 'readonly')
    const record = await requestToPromise<IdentityRecord | undefined>(
      tx.objectStore(IDENTITY_STORE).get(IDENTITY_KEY),
    )
    await transactionDone(tx)
    return record ?? null
  } finally {
    db.close()
  }
}

async function putIdentityRecord(record: IdentityRecord): Promise<void> {
  const db = await openIdentityDb()
  try {
    const tx = db.transaction(IDENTITY_STORE, 'readwrite')
    tx.objectStore(IDENTITY_STORE).put(record, IDENTITY_KEY)
    await transactionDone(tx)
  } finally {
    db.close()
  }
}

function openIdentityDb(): Promise<IDBDatabase> {
  return new Promise((resolve, reject) => {
    const req = indexedDB.open(IDENTITY_DB_NAME, IDENTITY_DB_VERSION)

    req.onupgradeneeded = () => {
      const db = req.result
      if (!db.objectStoreNames.contains(IDENTITY_STORE)) {
        db.createObjectStore(IDENTITY_STORE)
      }
    }

    req.onerror = () => {
      reject(req.error ?? new Error('Failed to open identity database'))
    }
    req.onsuccess = () => resolve(req.result)
  })
}

function requestToPromise<T>(req: IDBRequest<T>): Promise<T> {
  return new Promise((resolve, reject) => {
    req.onsuccess = () => resolve(req.result)
    req.onerror = () =>
      reject(req.error ?? new Error('IndexedDB request failed'))
  })
}

function transactionDone(tx: IDBTransaction): Promise<void> {
  return new Promise((resolve, reject) => {
    tx.oncomplete = () => resolve()
    tx.onabort = () =>
      reject(tx.error ?? new Error('IndexedDB transaction aborted'))
    tx.onerror = () =>
      reject(tx.error ?? new Error('IndexedDB transaction failed'))
  })
}
