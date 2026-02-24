import { fireEvent, render, waitFor } from '@testing-library/svelte'
import { beforeEach, describe, expect, it, vi } from 'vitest'

import { ApiError } from '$lib/api'

vi.mock('./crypto', () => ({
  encryptAndStoreKey: vi.fn(),
  generateIdentity: vi.fn(),
}))

vi.mock('./identityStore.svelte', () => {
  const identityState = {
    identity: null as unknown,
    register: vi.fn(),
    reRegister: vi.fn(),
  }
  return { identityState }
})

import { encryptAndStoreKey, generateIdentity } from './crypto'
import { identityState } from './identityStore.svelte'
import LoginView from './LoginView.svelte'

describe('LoginView', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    identityState.identity = null
  })

  it('creates identity and registers in create mode', async () => {
    const oncomplete = vi.fn()
    vi.mocked(generateIdentity).mockResolvedValue({
      secretKey: new Uint8Array(32).fill(1),
      publicKey: new Uint8Array(32).fill(2),
      didKey: 'did:key:z6Mk-test',
    })
    vi.mocked(encryptAndStoreKey).mockResolvedValue(undefined)
    vi.mocked(identityState.register).mockResolvedValue(undefined)

    const { getByLabelText, getByRole } = render(LoginView, { oncomplete })

    await fireEvent.input(getByLabelText('Username'), {
      target: { value: 'alice' },
    })

    await fireEvent.click(getByRole('button', { name: 'Create' }))

    await waitFor(() => expect(generateIdentity).toHaveBeenCalledTimes(1))
    expect(encryptAndStoreKey).toHaveBeenCalledTimes(1)
    expect(identityState.register).toHaveBeenCalledWith(
      'did:key:z6Mk-test',
      'alice',
      '#3b82f6',
    )
    expect(oncomplete).toHaveBeenCalledTimes(1)
  })

  it('shows inline username error on conflict', async () => {
    vi.mocked(generateIdentity).mockResolvedValue({
      secretKey: new Uint8Array(32).fill(1),
      publicKey: new Uint8Array(32).fill(2),
      didKey: 'did:key:z6Mk-test',
    })
    vi.mocked(encryptAndStoreKey).mockResolvedValue(undefined)
    vi.mocked(identityState.register).mockRejectedValue(
      new ApiError('CONFLICT', 'Username already taken'),
    )

    const { getByLabelText, getByRole, getByText, queryByRole } = render(
      LoginView,
      {},
    )

    await fireEvent.input(getByLabelText('Username'), {
      target: { value: 'alice' },
    })

    await fireEvent.click(getByRole('button', { name: 'Create' }))

    expect(
      await waitFor(() => getByText('Username already taken')),
    ).toBeDefined()
    expect(queryByRole('alert')).not.toBeInTheDocument()
  })

  it('pre-fills identity and calls reRegister in reregister mode', async () => {
    const oncomplete = vi.fn()
    identityState.identity = {
      publicKey: new Uint8Array(32).fill(1),
      didKey: 'did:key:z6Mk-test',
      username: 'alice',
      avatarColor: '#3b82f6',
      registeredAt: '2026-02-24T00:00:00.000Z',
    }
    vi.mocked(identityState.reRegister).mockResolvedValue(undefined)

    const { getByLabelText, getByRole, findByRole } = render(LoginView, {
      oncomplete,
      mode: 'reregister',
    })

    const avatar = await findByRole('radio', { name: 'Select Blue' })
    expect(avatar).toBeDisabled()

    const username = getByLabelText('Username') as HTMLInputElement
    expect(username.value).toBe('alice')

    await fireEvent.input(username, { target: { value: 'bob' } })
    await fireEvent.click(getByRole('button', { name: 'Register' }))

    await waitFor(() =>
      expect(identityState.reRegister).toHaveBeenCalledWith('bob', '#3b82f6'),
    )
    expect(generateIdentity).not.toHaveBeenCalled()
    expect(encryptAndStoreKey).not.toHaveBeenCalled()
    expect(oncomplete).toHaveBeenCalledTimes(1)
  })
})
