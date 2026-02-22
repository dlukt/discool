import * as ed from '@noble/ed25519'

import type { StoredIdentity } from './types'

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
): Promise<StoredIdentity> {
  const record = await getIdentityRecord()
  if (!record) {
    throw new Error('No stored identity to finalize')
  }

  const next: IdentityRecord = {
    ...record,
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

export async function loadStoredIdentity(): Promise<StoredIdentity | null> {
  const record = await getIdentityRecord()
  if (!record?.registeredAt) return null

  return {
    publicKey: record.publicKey,
    didKey: record.didKey,
    username: record.username,
    avatarColor: record.avatarColor,
    registeredAt: record.registeredAt,
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
