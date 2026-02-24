import { fireEvent, render, waitFor } from '@testing-library/svelte'
import { describe, expect, it, vi } from 'vitest'

vi.mock('$lib/api', async () => {
  const actual = await vi.importActual<typeof import('$lib/api')>('$lib/api')
  return {
    ...actual,
    submitSetup: vi.fn(),
  }
})

import { ApiError, submitSetup } from '$lib/api'
import SetupPage from './SetupPage.svelte'

describe('SetupPage', () => {
  it('validates required fields before submitting', async () => {
    const { getByRole, getByText } = render(SetupPage)

    await fireEvent.click(getByRole('button', { name: 'Complete Setup' }))

    expect(getByText('Username is required.')).toBeInTheDocument()
    expect(getByText('Instance name is required.')).toBeInTheDocument()
    expect(submitSetup).not.toHaveBeenCalled()
  })

  it('submits and dispatches complete on success', async () => {
    vi.mocked(submitSetup).mockResolvedValue({
      initialized: true,
      name: 'My Instance',
      admin: { username: 'alice' },
    })

    const onComplete = vi.fn()
    const { getByLabelText, getByRole, queryByRole } = render(SetupPage, {
      events: {
        complete: (event) => onComplete(event.detail),
      },
    })

    await fireEvent.input(getByLabelText('Admin username'), {
      target: { value: '  alice  ' },
    })
    await fireEvent.input(getByLabelText('Instance name'), {
      target: { value: '  My Instance  ' },
    })

    await fireEvent.click(getByRole('button', { name: 'Complete Setup' }))

    await waitFor(() =>
      expect(submitSetup).toHaveBeenCalledWith({
        adminUsername: 'alice',
        avatarColor: '#3b82f6',
        instanceName: 'My Instance',
        instanceDescription: undefined,
        discoveryEnabled: true,
      }),
    )
    await waitFor(() => expect(onComplete).toHaveBeenCalledTimes(1))
    expect(queryByRole('alert')).not.toBeInTheDocument()
  })

  it('shows ApiError message when submit fails', async () => {
    vi.mocked(submitSetup).mockRejectedValue(
      new ApiError('VALIDATION_ERROR', 'Missing required fields'),
    )

    const { getByLabelText, getByRole, findByRole } = render(SetupPage)

    await fireEvent.input(getByLabelText('Admin username'), {
      target: { value: 'alice' },
    })
    await fireEvent.input(getByLabelText('Instance name'), {
      target: { value: 'My Instance' },
    })

    await fireEvent.click(getByRole('button', { name: 'Complete Setup' }))

    expect(await findByRole('alert')).toHaveTextContent(
      'Missing required fields',
    )
  })
})
