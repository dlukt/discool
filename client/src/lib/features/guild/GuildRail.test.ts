import { fireEvent, render, waitFor } from '@testing-library/svelte'
import { beforeEach, describe, expect, it, vi } from 'vitest'

const { goto, guildState } = vi.hoisted(() => {
  const goto = vi.fn()
  const guildState = {
    guilds: [] as Array<{
      id: string
      slug: string
      name: string
      defaultChannelSlug: string
      isOwner: boolean
      createdAt: string
      description?: string
      iconUrl?: string
    }>,
    loadGuilds: vi.fn(),
    createGuild: vi.fn(),
  }
  return { goto, guildState }
})

vi.mock('@mateothegreat/svelte5-router', () => ({
  goto,
  route: () => undefined,
}))

vi.mock('./guildStore.svelte', () => ({
  guildState,
}))

import GuildRail from './GuildRail.svelte'

describe('GuildRail', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    guildState.guilds = [
      {
        id: 'guild-1',
        slug: 'lobby',
        name: 'Lobby',
        defaultChannelSlug: 'general',
        isOwner: true,
        createdAt: '2026-02-28T00:00:00.000Z',
      },
    ]
    guildState.loadGuilds.mockResolvedValue(guildState.guilds)
    guildState.createGuild.mockResolvedValue({
      id: 'guild-2',
      slug: 'makers-hub',
      name: 'Makers Hub',
      defaultChannelSlug: 'general',
      isOwner: true,
      createdAt: '2026-02-28T00:00:00.000Z',
    })
  })

  it('opens create dialog and validates required name on blur', async () => {
    const { getByLabelText, findByText } = render(GuildRail, {
      activeGuild: 'lobby',
      activeChannel: 'general',
    })

    await fireEvent.click(getByLabelText('Create guild'))
    const input = getByLabelText('Guild name')
    await fireEvent.input(input, { target: { value: '   ' } })
    await fireEvent.blur(input)

    expect(await findByText('Guild name is required.')).toBeInTheDocument()
  })

  it('submits create flow and navigates to the new guild general channel', async () => {
    const { getByLabelText, getByRole } = render(GuildRail, {
      activeGuild: 'lobby',
      activeChannel: 'general',
    })

    await fireEvent.click(getByLabelText('Create guild'))
    await fireEvent.input(getByLabelText('Guild name'), {
      target: { value: 'Makers Hub' },
    })
    await fireEvent.click(getByRole('button', { name: 'Create Guild' }))

    await waitFor(() =>
      expect(guildState.createGuild).toHaveBeenCalledWith(
        { name: 'Makers Hub' },
        null,
      ),
    )
    await waitFor(() =>
      expect(goto).toHaveBeenCalledWith('/makers-hub/general'),
    )
  })
})
