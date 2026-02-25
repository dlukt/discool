import { fireEvent, render } from '@testing-library/svelte'
import { describe, expect, it, vi } from 'vitest'

import CrossInstanceJoinPrompt from './CrossInstanceJoinPrompt.svelte'

describe('CrossInstanceJoinPrompt', () => {
  it('renders join copy and triggers one-click confirmation', async () => {
    const onconfirm = vi.fn()
    const onusedifferentname = vi.fn()
    const { getByRole, getByText } = render(CrossInstanceJoinPrompt, {
      guildName: 'Guild Alpha',
      username: 'alice',
      displayName: 'Alice',
      avatarColor: '#3b82f6',
      onconfirm,
      onusedifferentname,
    })

    expect(getByText('Join Guild Alpha as Alice?')).toBeInTheDocument()
    await fireEvent.click(getByRole('button', { name: 'Join as Alice' }))
    expect(onconfirm).toHaveBeenCalledTimes(1)

    await fireEvent.click(getByRole('button', { name: 'Use a different name' }))
    expect(onusedifferentname).toHaveBeenCalledTimes(1)
  })
})
