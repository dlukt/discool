import { fireEvent, render, waitFor } from '@testing-library/svelte'
import { beforeEach, describe, expect, it, vi } from 'vitest'

vi.mock('./identityStore.svelte', () => ({
  identityState: {
    startIdentityRecovery: vi.fn(),
    recoverIdentityByToken: vi.fn(),
  },
}))

import IdentityRecoveryView from './IdentityRecoveryView.svelte'
import { identityState } from './identityStore.svelte'

describe('IdentityRecoveryView', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('starts recovery flow and renders helper message', async () => {
    vi.mocked(identityState.startIdentityRecovery).mockResolvedValue({
      message: 'Recovery email sent. Check your inbox for a recovery link.',
      helpMessage: "Didn't receive the email? Check spam, or try again.",
    })

    const { getByLabelText, getByRole, getByText } =
      render(IdentityRecoveryView)

    await fireEvent.input(getByLabelText('Recovery email'), {
      target: { value: 'liam@example.com' },
    })
    await fireEvent.click(getByRole('button', { name: 'Send recovery email' }))

    await waitFor(() =>
      expect(identityState.startIdentityRecovery).toHaveBeenCalledWith(
        'liam@example.com',
      ),
    )
    expect(
      getByText("Didn't receive the email? Check spam, or try again."),
    ).toBeInTheDocument()
  })

  it('redeems token from URL context automatically', async () => {
    vi.mocked(identityState.recoverIdentityByToken).mockResolvedValue(undefined)
    const oncleartoken = vi.fn()

    render(IdentityRecoveryView, { token: 'token-123', oncleartoken })

    await waitFor(() =>
      expect(identityState.recoverIdentityByToken).toHaveBeenCalledWith(
        'token-123',
      ),
    )
    await waitFor(() => expect(oncleartoken).toHaveBeenCalledTimes(1))
  })

  it('clears token state when cancelling', async () => {
    const oncancel = vi.fn()
    const oncleartoken = vi.fn()
    const { getByRole } = render(IdentityRecoveryView, {
      oncancel,
      oncleartoken,
    })

    await fireEvent.click(getByRole('button', { name: 'Create new identity' }))

    expect(oncleartoken).toHaveBeenCalledTimes(1)
    expect(oncancel).toHaveBeenCalledTimes(1)
  })
})
