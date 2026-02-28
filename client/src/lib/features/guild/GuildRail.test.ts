import { fireEvent, render, waitFor } from '@testing-library/svelte'
import { beforeEach, describe, expect, it, vi } from 'vitest'

const { getLastViewedChannel, goto, guildState } = vi.hoisted(() => {
  const getLastViewedChannel = vi.fn(
    (_guildSlug: string) => null as string | null,
  )
  const goto = vi.fn()
  const guildState = {
    guilds: [] as Array<{
      id: string
      slug: string
      name: string
      defaultChannelSlug: string
      lastViewedChannelSlug?: string
      hasUnreadActivity?: boolean
      isOwner: boolean
      createdAt: string
      description?: string
      iconUrl?: string
    }>,
    loadGuilds: vi.fn(),
    createGuild: vi.fn(),
    setGuildOrder: vi.fn(),
  }
  return { getLastViewedChannel, goto, guildState }
})

vi.mock('@mateothegreat/svelte5-router', () => ({
  goto,
  route: () => undefined,
}))

vi.mock('$lib/features/identity/navigationState', () => ({
  getLastViewedChannel,
}))

vi.mock('./guildStore.svelte', () => ({
  guildState,
}))

import GuildRail from './GuildRail.svelte'

describe('GuildRail', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    getLastViewedChannel.mockImplementation(() => null)
    guildState.guilds = [
      {
        id: 'guild-1',
        slug: 'lobby',
        name: 'Lobby',
        defaultChannelSlug: 'general',
        hasUnreadActivity: true,
        isOwner: true,
        createdAt: '2026-02-28T00:00:00.000Z',
      },
      {
        id: 'guild-2',
        slug: 'makers-hub',
        name: 'Makers Hub',
        defaultChannelSlug: 'general',
        isOwner: true,
        createdAt: '2026-02-28T00:00:00.000Z',
      },
    ]
    guildState.loadGuilds.mockResolvedValue(guildState.guilds)
    guildState.createGuild.mockResolvedValue({
      id: 'guild-3',
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

  it('renders home control, active indicator, tooltip, and unread badge states', () => {
    const { getByRole, getByTestId } = render(GuildRail, {
      activeGuild: 'lobby',
      activeChannel: 'general',
    })

    expect(getByRole('button', { name: 'Home' })).toBeInTheDocument()
    expect(getByRole('button', { name: 'Lobby' })).toHaveAttribute(
      'aria-current',
      'page',
    )
    expect(getByTestId('guild-active-indicator-lobby')).toBeInTheDocument()
    expect(getByTestId('guild-unread-badge-lobby')).toBeInTheDocument()
    expect(getByRole('tooltip', { name: 'Lobby' })).toBeInTheDocument()
  })

  it('prefers persisted last-viewed channel when switching guilds', async () => {
    getLastViewedChannel.mockImplementation((slug: string) =>
      slug === 'makers-hub' ? 'announcements' : null,
    )
    const { getByRole } = render(GuildRail, {
      activeGuild: 'lobby',
      activeChannel: 'general',
    })

    await fireEvent.click(getByRole('button', { name: 'Makers Hub' }))

    await waitFor(() =>
      expect(goto).toHaveBeenCalledWith('/makers-hub/announcements'),
    )
  })

  it('supports arrow-key navigation, enter activation, and drag-drop reordering', async () => {
    const { getByRole } = render(GuildRail, {
      activeGuild: 'lobby',
      activeChannel: 'general',
    })

    const lobbyButton = getByRole('button', { name: 'Lobby' })
    const makersButton = getByRole('button', { name: 'Makers Hub' })
    lobbyButton.focus()

    await fireEvent.keyDown(lobbyButton, { key: 'ArrowDown' })
    expect(makersButton).toHaveFocus()
    await fireEvent.keyDown(makersButton, { key: 'Enter' })
    await waitFor(() =>
      expect(goto).toHaveBeenCalledWith('/makers-hub/general'),
    )

    await fireEvent.dragStart(makersButton)
    await fireEvent.dragOver(lobbyButton)
    await fireEvent.drop(lobbyButton)
    expect(guildState.setGuildOrder).toHaveBeenCalledWith([
      'makers-hub',
      'lobby',
    ])
  })
})
