import { beforeEach, describe, expect, it, vi } from 'vitest'

vi.mock('$lib/api', () => ({
  apiFetch: vi.fn(),
  apiFetchCursorList: vi.fn(),
  ApiError: class ApiError extends Error {
    code: string

    constructor(code: string, message: string) {
      super(message)
      this.code = code
      this.name = 'ApiError'
    }
  },
}))

import { apiFetch, apiFetchCursorList } from '$lib/api'
import {
  actOnReport,
  createMessageDelete,
  createMessageReport,
  createUserReport,
  createVoiceKick,
  dismissReport,
  fetchModerationLog,
  fetchReportQueue,
  fetchUserMessageHistory,
  reviewReport,
} from './moderationApi'

describe('moderationApi voice kick', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('posts voice-kick payload and maps response', async () => {
    vi.mocked(apiFetch).mockResolvedValue({
      id: 'voice-kick-1',
      guild_slug: 'lobby',
      channel_slug: 'voice-room',
      actor_user_id: 'mod-user-id',
      target_user_id: 'target-user-id',
      reason: 'disruptive behavior',
      created_at: '2026-03-02T00:00:00.000Z',
      updated_at: '2026-03-02T00:00:00.000Z',
    })

    await expect(
      createVoiceKick(' lobby ', {
        targetUserId: ' target-user-id ',
        channelSlug: ' voice-room ',
        reason: ' disruptive behavior ',
      }),
    ).resolves.toEqual({
      id: 'voice-kick-1',
      guildSlug: 'lobby',
      channelSlug: 'voice-room',
      actorUserId: 'mod-user-id',
      targetUserId: 'target-user-id',
      reason: 'disruptive behavior',
      createdAt: '2026-03-02T00:00:00.000Z',
      updatedAt: '2026-03-02T00:00:00.000Z',
    })

    expect(apiFetch).toHaveBeenCalledWith(
      '/api/v1/guilds/lobby/moderation/voice-kicks',
      {
        method: 'POST',
        body: JSON.stringify({
          target_user_id: 'target-user-id',
          channel_slug: 'voice-room',
          reason: 'disruptive behavior',
        }),
      },
    )
  })
})

describe('moderationApi message delete', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('posts moderated message-delete payload and maps response', async () => {
    vi.mocked(apiFetch).mockResolvedValue({
      id: 'mod-action-1',
      message_id: 'message-1',
      guild_slug: 'lobby',
      channel_slug: 'general',
      actor_user_id: 'mod-user-id',
      target_user_id: 'target-user-id',
      reason: 'policy violation',
      created_at: '2026-03-02T00:00:00.000Z',
      updated_at: '2026-03-02T00:00:00.000Z',
    })

    await expect(
      createMessageDelete(' lobby ', {
        messageId: ' message-1 ',
        channelSlug: ' general ',
        reason: ' policy violation ',
      }),
    ).resolves.toEqual({
      id: 'mod-action-1',
      messageId: 'message-1',
      guildSlug: 'lobby',
      channelSlug: 'general',
      actorUserId: 'mod-user-id',
      targetUserId: 'target-user-id',
      reason: 'policy violation',
      createdAt: '2026-03-02T00:00:00.000Z',
      updatedAt: '2026-03-02T00:00:00.000Z',
    })

    expect(apiFetch).toHaveBeenCalledWith(
      '/api/v1/guilds/lobby/moderation/messages/message-1/delete',
      {
        method: 'POST',
        body: JSON.stringify({
          channel_slug: 'general',
          reason: 'policy violation',
        }),
      },
    )
  })
})

describe('moderationApi message report', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('posts message-report payload and maps response', async () => {
    vi.mocked(apiFetch).mockResolvedValue({
      id: 'report-1',
      guild_slug: 'lobby',
      reporter_user_id: 'reporter-id',
      target_type: 'message',
      target_message_id: 'message-1',
      reason: 'harmful content',
      category: 'harassment',
      status: 'pending',
      created_at: '2026-03-02T00:00:00.000Z',
      updated_at: '2026-03-02T00:00:00.000Z',
    })

    await expect(
      createMessageReport(' lobby ', {
        messageId: ' message-1 ',
        reason: ' harmful content ',
        category: 'harassment',
      }),
    ).resolves.toEqual({
      id: 'report-1',
      guildSlug: 'lobby',
      reporterUserId: 'reporter-id',
      targetType: 'message',
      targetMessageId: 'message-1',
      targetUserId: null,
      reason: 'harmful content',
      category: 'harassment',
      status: 'pending',
      createdAt: '2026-03-02T00:00:00.000Z',
      updatedAt: '2026-03-02T00:00:00.000Z',
    })

    expect(apiFetch).toHaveBeenCalledWith(
      '/api/v1/guilds/lobby/moderation/reports/messages/message-1',
      {
        method: 'POST',
        body: JSON.stringify({
          reason: 'harmful content',
          category: 'harassment',
        }),
      },
    )
  })

  it('rejects invalid category before issuing request', async () => {
    await expect(
      createMessageReport('lobby', {
        messageId: 'message-1',
        reason: 'content',
        category: 'invalid' as never,
      }),
    ).rejects.toMatchObject({
      code: 'VALIDATION_ERROR',
    })
    expect(apiFetch).not.toHaveBeenCalled()
  })
})

describe('moderationApi user report', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('posts user-report payload and maps response', async () => {
    vi.mocked(apiFetch).mockResolvedValue({
      id: 'report-2',
      guild_slug: 'lobby',
      reporter_user_id: 'reporter-id',
      target_type: 'user',
      target_user_id: 'target-user-id',
      reason: 'impersonation',
      status: 'pending',
      created_at: '2026-03-02T00:00:00.000Z',
      updated_at: '2026-03-02T00:00:00.000Z',
    })

    await expect(
      createUserReport(' lobby ', {
        targetUserId: ' target-user-id ',
        reason: ' impersonation ',
      }),
    ).resolves.toEqual({
      id: 'report-2',
      guildSlug: 'lobby',
      reporterUserId: 'reporter-id',
      targetType: 'user',
      targetMessageId: null,
      targetUserId: 'target-user-id',
      reason: 'impersonation',
      category: null,
      status: 'pending',
      createdAt: '2026-03-02T00:00:00.000Z',
      updatedAt: '2026-03-02T00:00:00.000Z',
    })

    expect(apiFetch).toHaveBeenCalledWith(
      '/api/v1/guilds/lobby/moderation/reports/users/target-user-id',
      {
        method: 'POST',
        body: JSON.stringify({
          reason: 'impersonation',
        }),
      },
    )
  })
})

describe('moderationApi report queue', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('maps report queue query params and response entries', async () => {
    vi.mocked(apiFetchCursorList).mockResolvedValue({
      data: [
        {
          id: 'report-1',
          guild_slug: 'lobby',
          reporter_user_id: 'reporter-id',
          reporter_username: 'reporter',
          reporter_display_name: 'Reporter',
          reporter_avatar_color: '#3366ff',
          target_type: 'message',
          target_message_id: 'message-1',
          target_username: 'target-user',
          target_display_name: 'Target User',
          target_avatar_color: '#99aab5',
          target_message_preview: 'message preview',
          reason: 'spam',
          category: 'spam',
          status: 'pending',
          created_at: '2026-03-02T00:00:00.000Z',
          updated_at: '2026-03-02T00:00:00.000Z',
        },
      ],
      cursor: 'cursor-next',
    })

    await expect(
      fetchReportQueue(' lobby ', {
        limit: 20.4,
        cursor: ' cursor-1 ',
        status: 'pending',
      }),
    ).resolves.toEqual({
      entries: [
        {
          id: 'report-1',
          guildSlug: 'lobby',
          reporterUserId: 'reporter-id',
          reporterUsername: 'reporter',
          reporterDisplayName: 'Reporter',
          reporterAvatarColor: '#3366ff',
          targetType: 'message',
          targetMessageId: 'message-1',
          targetUserId: null,
          targetUsername: 'target-user',
          targetDisplayName: 'Target User',
          targetAvatarColor: '#99aab5',
          targetMessagePreview: 'message preview',
          reason: 'spam',
          category: 'spam',
          status: 'pending',
          reviewedAt: null,
          actionedAt: null,
          dismissedAt: null,
          dismissalReason: null,
          moderationActionId: null,
          createdAt: '2026-03-02T00:00:00.000Z',
          updatedAt: '2026-03-02T00:00:00.000Z',
        },
      ],
      cursor: 'cursor-next',
    })

    expect(apiFetchCursorList).toHaveBeenCalledWith(
      '/api/v1/guilds/lobby/moderation/reports?limit=20&cursor=cursor-1&status=pending',
    )
  })

  it('reviews a report and maps response', async () => {
    vi.mocked(apiFetch).mockResolvedValue({
      id: 'report-1',
      guild_slug: 'lobby',
      reporter_user_id: 'reporter-id',
      reporter_username: 'reporter',
      reporter_display_name: 'Reporter',
      target_type: 'user',
      target_user_id: 'target-user-id',
      reason: 'abuse',
      status: 'reviewed',
      reviewed_at: '2026-03-02T00:01:00.000Z',
      created_at: '2026-03-02T00:00:00.000Z',
      updated_at: '2026-03-02T00:01:00.000Z',
    })

    await expect(reviewReport(' lobby ', ' report-1 ')).resolves.toMatchObject({
      id: 'report-1',
      guildSlug: 'lobby',
      status: 'reviewed',
      reviewedAt: '2026-03-02T00:01:00.000Z',
    })

    expect(apiFetch).toHaveBeenCalledWith(
      '/api/v1/guilds/lobby/moderation/reports/report-1/review',
      {
        method: 'POST',
      },
    )
  })

  it('dismisses a report with optional reason', async () => {
    vi.mocked(apiFetch).mockResolvedValue({
      id: 'report-1',
      guild_slug: 'lobby',
      reporter_user_id: 'reporter-id',
      reporter_username: 'reporter',
      reporter_display_name: 'Reporter',
      target_type: 'user',
      target_user_id: 'target-user-id',
      reason: 'abuse',
      status: 'dismissed',
      dismissed_at: '2026-03-02T00:01:00.000Z',
      dismissal_reason: 'not actionable',
      created_at: '2026-03-02T00:00:00.000Z',
      updated_at: '2026-03-02T00:01:00.000Z',
    })

    await expect(
      dismissReport(' lobby ', ' report-1 ', {
        dismissalReason: ' not actionable ',
      }),
    ).resolves.toMatchObject({
      id: 'report-1',
      status: 'dismissed',
      dismissalReason: 'not actionable',
    })

    expect(apiFetch).toHaveBeenCalledWith(
      '/api/v1/guilds/lobby/moderation/reports/report-1/dismiss',
      {
        method: 'POST',
        body: JSON.stringify({
          dismissal_reason: 'not actionable',
        }),
      },
    )
  })

  it('acts on report and validates action type', async () => {
    vi.mocked(apiFetch).mockResolvedValue({
      id: 'report-1',
      guild_slug: 'lobby',
      reporter_user_id: 'reporter-id',
      reporter_username: 'reporter',
      reporter_display_name: 'Reporter',
      target_type: 'user',
      target_user_id: 'target-user-id',
      reason: 'abuse',
      status: 'actioned',
      actioned_at: '2026-03-02T00:01:00.000Z',
      moderation_action_id: 'mod-action-1',
      created_at: '2026-03-02T00:00:00.000Z',
      updated_at: '2026-03-02T00:01:00.000Z',
    })

    await expect(
      actOnReport(' lobby ', ' report-1 ', {
        actionType: 'ban',
        reason: ' repeat abuse ',
        deleteMessageWindow: '24h',
      }),
    ).resolves.toMatchObject({
      id: 'report-1',
      status: 'actioned',
      moderationActionId: 'mod-action-1',
    })

    expect(apiFetch).toHaveBeenCalledWith(
      '/api/v1/guilds/lobby/moderation/reports/report-1/actions',
      {
        method: 'POST',
        body: JSON.stringify({
          action_type: 'ban',
          reason: 'repeat abuse',
          delete_message_window: '24h',
        }),
      },
    )

    await expect(
      actOnReport('lobby', 'report-1', {
        actionType: 'invalid' as never,
      }),
    ).rejects.toMatchObject({
      code: 'VALIDATION_ERROR',
    })
  })
})

describe('moderationApi moderation log', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('maps query params and converts moderation log entries', async () => {
    vi.mocked(apiFetchCursorList).mockResolvedValue({
      data: [
        {
          id: 'log-1',
          action_type: 'kick',
          reason: 'serious breach',
          created_at: '2026-03-02T00:00:00.000Z',
          actor_user_id: 'mod-user-id',
          actor_username: 'mod-user',
          actor_display_name: 'Moderator',
          actor_avatar_color: '#3366ff',
          target_user_id: 'target-user-id',
          target_username: 'target-user',
          target_display_name: 'Target User',
          target_avatar_color: '#99aab5',
        },
      ],
      cursor: 'cursor-next',
    })

    await expect(
      fetchModerationLog(' lobby ', {
        limit: 25.7,
        cursor: ' cursor-1 ',
        order: 'desc',
        actionType: 'kick',
      }),
    ).resolves.toEqual({
      entries: [
        {
          id: 'log-1',
          actionType: 'kick',
          reason: 'serious breach',
          createdAt: '2026-03-02T00:00:00.000Z',
          actorUserId: 'mod-user-id',
          actorUsername: 'mod-user',
          actorDisplayName: 'Moderator',
          actorAvatarColor: '#3366ff',
          targetUserId: 'target-user-id',
          targetUsername: 'target-user',
          targetDisplayName: 'Target User',
          targetAvatarColor: '#99aab5',
        },
      ],
      cursor: 'cursor-next',
    })

    expect(apiFetchCursorList).toHaveBeenCalledWith(
      '/api/v1/guilds/lobby/moderation/log?limit=25&cursor=cursor-1&order=desc&action_type=kick',
    )
  })

  it('uses base path when no query options are passed', async () => {
    vi.mocked(apiFetchCursorList).mockResolvedValue({
      data: [],
      cursor: null,
    })

    await expect(fetchModerationLog('lobby')).resolves.toEqual({
      entries: [],
      cursor: null,
    })

    expect(apiFetchCursorList).toHaveBeenCalledWith(
      '/api/v1/guilds/lobby/moderation/log',
    )
  })

  it('rejects invalid limit values before issuing request', async () => {
    await expect(
      fetchModerationLog('lobby', { limit: Number.NaN }),
    ).rejects.toMatchObject({
      code: 'VALIDATION_ERROR',
    })
    expect(apiFetchCursorList).not.toHaveBeenCalled()
  })
})

describe('moderationApi user message history', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('maps query params and converts history entries', async () => {
    vi.mocked(apiFetchCursorList).mockResolvedValue({
      data: [
        {
          id: 'history-1',
          channel_slug: 'general',
          channel_name: 'general',
          content: 'hello',
          created_at: '2026-03-02T00:00:00.000Z',
        },
      ],
      cursor: 'history-cursor-2',
    })

    await expect(
      fetchUserMessageHistory(' lobby ', ' user-123 ', {
        limit: 25.2,
        cursor: ' cursor-1 ',
        channelSlug: ' general ',
        from: ' 2026-03-01T00:00:00.000Z ',
        to: ' 2026-03-02T00:00:00.000Z ',
      }),
    ).resolves.toEqual({
      entries: [
        {
          id: 'history-1',
          channelSlug: 'general',
          channelName: 'general',
          content: 'hello',
          createdAt: '2026-03-02T00:00:00.000Z',
        },
      ],
      cursor: 'history-cursor-2',
    })

    expect(apiFetchCursorList).toHaveBeenCalledWith(
      '/api/v1/guilds/lobby/moderation/users/user-123/messages?limit=25&cursor=cursor-1&channel_slug=general&from=2026-03-01T00%3A00%3A00.000Z&to=2026-03-02T00%3A00%3A00.000Z',
    )
  })

  it('uses base path when no query options are passed', async () => {
    vi.mocked(apiFetchCursorList).mockResolvedValue({
      data: [],
      cursor: null,
    })

    await expect(fetchUserMessageHistory('lobby', 'user-123')).resolves.toEqual(
      {
        entries: [],
        cursor: null,
      },
    )

    expect(apiFetchCursorList).toHaveBeenCalledWith(
      '/api/v1/guilds/lobby/moderation/users/user-123/messages',
    )
  })

  it('rejects invalid limit values before issuing request', async () => {
    await expect(
      fetchUserMessageHistory('lobby', 'user-123', { limit: Number.NaN }),
    ).rejects.toMatchObject({
      code: 'VALIDATION_ERROR',
    })
    expect(apiFetchCursorList).not.toHaveBeenCalled()
  })
})
