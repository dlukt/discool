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
import { createVoiceKick, fetchModerationLog } from './moderationApi'

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
