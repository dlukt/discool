import { fireEvent, render } from '@testing-library/svelte'
import { beforeEach, describe, expect, it, vi } from 'vitest'

vi.mock('./identityStore.svelte', () => {
  const identityState = {
    identity: {
      publicKey: new Uint8Array(32).fill(1),
      didKey: 'did:key:z6Mk-test',
      username: 'alice',
      avatarColor: '#3b82f6',
      registeredAt: '2026-02-24T00:00:00.000Z',
    },
    identityNotRegistered: true,
    register: vi.fn(),
    authenticate: vi.fn(),
  }

  return { identityState }
})

import { identityState } from './identityStore.svelte'
import ReRegisterPrompt from './ReRegisterPrompt.svelte'

describe('ReRegisterPrompt', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    identityState.identity = {
      publicKey: new Uint8Array(32).fill(1),
      didKey: 'did:key:z6Mk-test',
      username: 'alice',
      avatarColor: '#3b82f6',
      registeredAt: '2026-02-24T00:00:00.000Z',
    }
  })

  it('renders stored username and calls register', async () => {
    vi.mocked(identityState.register).mockResolvedValue(undefined)

    const onusedifferentname = vi.fn()
    const { getByRole, getByText } = render(ReRegisterPrompt, {
      onusedifferentname,
    })

    expect(getByText('Welcome back!')).toBeInTheDocument()
    expect(getByText('alice')).toBeInTheDocument()

    const register = getByRole('button', { name: 'Register as alice' })
    await fireEvent.click(register)

    expect(identityState.register).toHaveBeenCalledWith(
      'did:key:z6Mk-test',
      'alice',
      '#3b82f6',
    )

    const differentName = getByRole('button', { name: 'Use a different name' })
    await fireEvent.click(differentName)
    expect(onusedifferentname).toHaveBeenCalledTimes(1)
  })
})
