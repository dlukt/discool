import { fireEvent, render } from '@testing-library/svelte'
import { describe, expect, it, vi } from 'vitest'

import RecoveryPrompt from './RecoveryPrompt.svelte'

describe('RecoveryPrompt', () => {
  it('renders copy and triggers actions', async () => {
    const onstartfresh = vi.fn()
    const onrecover = vi.fn()

    const { getByRole, getByText } = render(RecoveryPrompt, {
      onstartfresh,
      onrecover,
    })

    expect(
      getByText('Your stored identity appears to be damaged'),
    ).toBeInTheDocument()

    const startFresh = getByRole('button', { name: 'Create a new identity' })
    await fireEvent.click(startFresh)

    expect(onstartfresh).toHaveBeenCalledTimes(1)
    const recover = getByRole('button', { name: 'Recover existing identity' })
    await fireEvent.click(recover)
    expect(onrecover).toHaveBeenCalledTimes(1)
  })
})
