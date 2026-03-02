import { fireEvent, render, waitFor } from '@testing-library/svelte'
import { beforeEach, describe, expect, it, vi } from 'vitest'

const moderationApi = vi.hoisted(() => ({
  fetchModerationLog: vi.fn(),
}))

vi.mock('./moderationApi', () => moderationApi)

import ModLogPanel from './ModLogPanel.svelte'

function makeEntry(index: number) {
  return {
    id: `log-${index}`,
    actionType: index % 2 === 0 ? ('kick' as const) : ('mute' as const),
    reason: `reason-${index}`,
    createdAt: `2026-03-02T00:00:${index.toString().padStart(2, '0')}.000Z`,
    actorUserId: 'mod-user-id',
    actorUsername: 'mod-user',
    actorDisplayName: 'Moderator',
    actorAvatarColor: '#3366ff',
    targetUserId: `target-${index}`,
    targetUsername: `target-${index}`,
    targetDisplayName: `Target ${index}`,
    targetAvatarColor: '#99aab5',
  }
}

describe('ModLogPanel', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('renders empty state when no moderation entries exist', async () => {
    moderationApi.fetchModerationLog.mockResolvedValueOnce({
      entries: [],
      cursor: null,
    })

    const { getByTestId } = render(ModLogPanel, { activeGuild: 'lobby' })

    await waitFor(() => {
      expect(moderationApi.fetchModerationLog).toHaveBeenCalledWith('lobby', {
        limit: 50,
        cursor: null,
        order: 'desc',
        actionType: null,
      })
      expect(getByTestId('mod-log-empty')).toBeInTheDocument()
    })
  })

  it('updates query params when changing filter and sort controls', async () => {
    moderationApi.fetchModerationLog.mockResolvedValue({
      entries: [makeEntry(1)],
      cursor: null,
    })

    const { getByTestId, getByText } = render(ModLogPanel, {
      activeGuild: 'lobby',
    })

    await waitFor(() => {
      expect(getByText('Target: Target 1')).toBeInTheDocument()
    })

    await fireEvent.change(getByTestId('mod-log-filter-select'), {
      target: { value: 'kick' },
    })
    await waitFor(() => {
      expect(moderationApi.fetchModerationLog).toHaveBeenLastCalledWith(
        'lobby',
        expect.objectContaining({
          actionType: 'kick',
          order: 'desc',
        }),
      )
    })

    await fireEvent.change(getByTestId('mod-log-order-select'), {
      target: { value: 'asc' },
    })
    await waitFor(() => {
      expect(moderationApi.fetchModerationLog).toHaveBeenLastCalledWith(
        'lobby',
        expect.objectContaining({
          actionType: 'kick',
          order: 'asc',
        }),
      )
    })
  })

  it('virtualizes long moderation logs', async () => {
    moderationApi.fetchModerationLog.mockResolvedValueOnce({
      entries: Array.from({ length: 120 }, (_, index) => makeEntry(index)),
      cursor: null,
    })

    const { getByTestId, queryAllByTestId, queryByTestId } = render(
      ModLogPanel,
      {
        activeGuild: 'lobby',
      },
    )

    await waitFor(() => {
      expect(queryAllByTestId(/^mod-log-entry-log-/).length).toBeGreaterThan(0)
    })
    expect(queryAllByTestId(/^mod-log-entry-log-/).length).toBeLessThan(120)

    const scroll = getByTestId('mod-log-scroll')
    Object.defineProperty(scroll, 'clientHeight', {
      value: 240,
      configurable: true,
    })
    Object.defineProperty(scroll, 'scrollHeight', {
      value: 120 * 132,
      configurable: true,
    })
    scroll.scrollTop = 2200
    await fireEvent.scroll(scroll)

    expect(queryByTestId('mod-log-entry-log-0')).not.toBeInTheDocument()
  })
})
