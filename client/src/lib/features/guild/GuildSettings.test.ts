import { fireEvent, render, waitFor } from '@testing-library/svelte'
import { beforeEach, describe, expect, it, vi } from 'vitest'

const { guildState } = vi.hoisted(() => {
  const guildState = {
    bySlug: vi.fn(),
    updateGuild: vi.fn(),
  }
  return { guildState }
})

vi.mock('./guildStore.svelte', () => ({
  guildState,
}))

import GuildSettings from './GuildSettings.svelte'

function ownerGuild() {
  return {
    id: 'guild-1',
    slug: 'makers-hub',
    name: 'Makers Hub',
    description: 'Build cool things',
    defaultChannelSlug: 'general',
    isOwner: true,
    createdAt: '2026-02-28T00:00:00.000Z',
  }
}

describe('GuildSettings', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    guildState.bySlug.mockImplementation((slug: string) =>
      slug === 'makers-hub' ? ownerGuild() : null,
    )
    guildState.updateGuild.mockResolvedValue(ownerGuild())
  })

  it('saves owner edits for guild name and description', async () => {
    const { getByLabelText, getByRole, getByText } = render(GuildSettings, {
      open: true,
      guildSlug: 'makers-hub',
    })

    await fireEvent.input(getByLabelText('Guild name'), {
      target: { value: 'Makers Hub Updated' },
    })
    await fireEvent.input(getByLabelText('Description'), {
      target: { value: 'Updated by owner' },
    })
    await fireEvent.click(getByRole('button', { name: 'Save Guild' }))

    await waitFor(() =>
      expect(guildState.updateGuild).toHaveBeenCalledWith(
        'makers-hub',
        {
          name: 'Makers Hub Updated',
          description: 'Updated by owner',
        },
        null,
      ),
    )
    expect(getByText('Guild settings saved.')).toBeInTheDocument()
  })

  it('shows owner-only guardrail for non-owners', () => {
    guildState.bySlug.mockReturnValue({
      ...ownerGuild(),
      isOwner: false,
    })
    const { getByText, queryByRole } = render(GuildSettings, {
      open: true,
      guildSlug: 'makers-hub',
    })

    expect(
      getByText('Only guild owners can edit guild settings.'),
    ).toBeInTheDocument()
    expect(
      queryByRole('button', { name: 'Save Guild' }),
    ).not.toBeInTheDocument()
  })
})
