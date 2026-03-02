import { beforeEach, describe, expect, it, vi } from 'vitest'

vi.mock('$lib/api', () => ({
  apiFetch: vi.fn(),
}))

import { apiFetch } from '$lib/api'
import { createVoiceKick } from './moderationApi'

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
