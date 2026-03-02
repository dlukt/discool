import { beforeEach, describe, expect, it, vi } from 'vitest'

const { envelopeListeners, listGuildsApi } = vi.hoisted(() => {
  const envelopeListeners = new Set<(envelope: unknown) => void>()
  const listGuildsApi = vi.fn(
    async (): Promise<Array<Record<string, unknown>>> => [],
  )
  return {
    envelopeListeners,
    listGuildsApi,
  }
})

vi.mock('$lib/ws/client', () => ({
  wsClient: {
    subscribe: vi.fn((listener: (envelope: unknown) => void) => {
      envelopeListeners.add(listener)
      return () => envelopeListeners.delete(listener)
    }),
  },
}))

vi.mock('$lib/features/identity/navigationState', () => ({
  getGuildOrder: vi.fn(() => []),
  saveGuildOrder: vi.fn(),
}))

vi.mock('./guildApi', () => ({
  createGuild: vi.fn(),
  createRole: vi.fn(),
  deleteRole: vi.fn(),
  listBans: vi.fn(async () => []),
  listGuilds: listGuildsApi,
  listMembers: vi.fn(),
  listRoles: vi.fn(),
  reorderRoles: vi.fn(),
  unban: vi.fn(),
  updateGuild: vi.fn(),
  updateMemberRoles: vi.fn(),
  updateRole: vi.fn(),
  uploadGuildIcon: vi.fn(),
}))

import { guildState } from './guildStore.svelte'

function emitEnvelope(envelope: unknown): void {
  for (const listener of envelopeListeners) {
    listener(envelope)
  }
}

async function flushMicrotasks(iterations = 4): Promise<void> {
  for (let index = 0; index < iterations; index += 1) {
    await Promise.resolve()
  }
}

describe('guildState websocket updates', () => {
  beforeEach(() => {
    guildState.clear()
    listGuildsApi.mockReset()
    listGuildsApi.mockResolvedValue([
      {
        id: 'guild-1',
        slug: 'engineering',
        name: 'Engineering',
        defaultChannelSlug: 'announcements',
        hasUnreadActivity: false,
        lastViewedChannelSlug: 'announcements',
      },
    ])
  })

  it('reloads guild list and clears cached member data on guild_update', async () => {
    guildState.memberRoleDataByGuild.lobby = {
      members: [],
      roles: [],
      assignableRoleIds: [],
      canManageRoles: false,
    }
    guildState.bansByGuild.lobby = [
      {
        id: 'ban-1',
        targetUserId: 'user-target',
        targetUsername: 'target',
        actorUserId: 'owner',
        actorUsername: 'owner',
        reason: 'reason',
        createdAt: '2026-03-01T00:00:00.000Z',
      },
    ]

    emitEnvelope({
      op: 'guild_update',
      d: { guild_slug: 'lobby', action_type: 'kick' },
    })
    await flushMicrotasks()

    expect(listGuildsApi).toHaveBeenCalledTimes(1)
    expect(guildState.memberRoleDataByGuild.lobby).toBeUndefined()
    expect(guildState.bansByGuild.lobby).toBeUndefined()
  })

  it('ignores non-guild-update websocket envelopes', async () => {
    emitEnvelope({ op: 'typing_start', d: { guild_slug: 'lobby' } })
    await flushMicrotasks()
    expect(listGuildsApi).not.toHaveBeenCalled()
  })
})
