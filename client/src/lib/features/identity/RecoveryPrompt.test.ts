import { fireEvent, render } from '@testing-library/svelte'
import { describe, expect, it, vi } from 'vitest'

import RecoveryPrompt from './RecoveryPrompt.svelte'

describe('RecoveryPrompt', () => {
  it('renders copy and triggers start-fresh action', async () => {
    const onstartfresh = vi.fn()

    const { getByRole, getByText } = render(RecoveryPrompt, { onstartfresh })

    expect(
      getByText('Your stored identity appears to be damaged'),
    ).toBeInTheDocument()

    const startFresh = getByRole('button', { name: 'Create a new identity' })
    await fireEvent.click(startFresh)

    expect(onstartfresh).toHaveBeenCalledTimes(1)
    expect(getByRole('button', { name: 'Recover via email' })).toBeDisabled()
  })
})
