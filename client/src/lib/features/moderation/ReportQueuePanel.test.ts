import { fireEvent, render, waitFor } from '@testing-library/svelte'
import { beforeEach, describe, expect, it, vi } from 'vitest'

const moderationApi = vi.hoisted(() => ({
  fetchReportQueue: vi.fn(),
  reviewReport: vi.fn(),
  dismissReport: vi.fn(),
  actOnReport: vi.fn(),
}))

vi.mock('./moderationApi', () => moderationApi)

import ReportQueuePanel from './ReportQueuePanel.svelte'

function makeEntry(
  overrides: Partial<{
    id: string
    status: 'pending' | 'reviewed' | 'actioned' | 'dismissed'
    targetType: 'message' | 'user'
  }> = {},
) {
  return {
    id: overrides.id ?? 'report-1',
    guildSlug: 'lobby',
    reporterUserId: 'reporter-id',
    reporterUsername: 'reporter',
    reporterDisplayName: 'Reporter',
    reporterAvatarColor: '#3366ff',
    targetType: overrides.targetType ?? ('message' as const),
    targetMessageId: 'message-1',
    targetUserId: null,
    targetUsername: 'target-user',
    targetDisplayName: 'Target User',
    targetAvatarColor: '#99aab5',
    targetMessagePreview: 'message preview',
    reason: 'spam content',
    category: 'spam' as const,
    status: overrides.status ?? ('pending' as const),
    reviewedAt: null,
    actionedAt: null,
    dismissedAt: null,
    dismissalReason: null,
    moderationActionId: null,
    createdAt: '2026-03-02T00:00:00.000Z',
    updatedAt: '2026-03-02T00:00:00.000Z',
  }
}

describe('ReportQueuePanel', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('shows exact empty pending copy when queue has no pending items', async () => {
    moderationApi.fetchReportQueue.mockResolvedValueOnce({
      entries: [],
      cursor: null,
    })

    const { getByTestId } = render(ReportQueuePanel, { activeGuild: 'lobby' })

    await waitFor(() => {
      expect(moderationApi.fetchReportQueue).toHaveBeenCalledWith('lobby', {
        limit: 50,
        cursor: null,
        status: 'pending',
      })
      expect(getByTestId('report-queue-empty')).toHaveTextContent(
        'No pending reports.',
      )
    })
  })

  it('renders pending report items with highlighted card style', async () => {
    moderationApi.fetchReportQueue.mockResolvedValueOnce({
      entries: [makeEntry()],
      cursor: null,
    })

    const { getByTestId, getByText } = render(ReportQueuePanel, {
      activeGuild: 'lobby',
    })

    await waitFor(() => {
      expect(getByTestId('report-queue-item-report-1')).toBeInTheDocument()
    })
    expect(getByText('message preview')).toBeInTheDocument()
    expect(getByTestId('report-queue-item-report-1').className).toContain(
      'border-amber-400/40',
    )
  })

  it('removes reviewed entries from pending filter results', async () => {
    moderationApi.fetchReportQueue.mockResolvedValueOnce({
      entries: [makeEntry()],
      cursor: null,
    })
    moderationApi.reviewReport.mockResolvedValueOnce(
      makeEntry({ status: 'reviewed' }),
    )

    const { getByTestId, queryByTestId } = render(ReportQueuePanel, {
      activeGuild: 'lobby',
    })

    await waitFor(() => {
      expect(getByTestId('report-action-review-report-1')).toBeInTheDocument()
    })

    await fireEvent.click(getByTestId('report-action-review-report-1'))

    await waitFor(() => {
      expect(moderationApi.reviewReport).toHaveBeenCalledWith(
        'lobby',
        'report-1',
      )
      expect(
        queryByTestId('report-queue-item-report-1'),
      ).not.toBeInTheDocument()
      expect(getByTestId('report-queue-empty')).toHaveTextContent(
        'No pending reports.',
      )
    })
  })
})
