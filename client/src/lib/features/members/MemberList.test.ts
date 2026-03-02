import { fireEvent, render, waitFor } from '@testing-library/svelte'
import { beforeEach, describe, expect, it, vi } from 'vitest'

const { guildState, memberDataByGuild, presenceStatuses, identityState } =
  vi.hoisted(() => {
    const memberDataByGuild: Record<
      string,
      {
        members: Array<{
          userId: string
          username: string
          displayName: string
          avatarColor?: string
          presenceStatus: 'online' | 'idle' | 'offline'
          highestRoleColor: string
          roleIds: string[]
          isOwner: boolean
          canAssignRoles: boolean
        }>
        roles: Array<{
          id: string
          name: string
          color: string
          position: number
          permissionsBitflag: number
          isDefault: boolean
          isSystem: boolean
          canEdit: boolean
          canDelete: boolean
          createdAt: string
        }>
        assignableRoleIds: string[]
        canManageRoles: boolean
      }
    > = {}

    const presenceStatuses: Record<string, 'online' | 'idle' | 'offline'> = {}

    const emptyData = {
      members: [],
      roles: [],
      assignableRoleIds: [],
      canManageRoles: false,
    }

    const guildState = {
      loadMembers: vi.fn(
        async (guildSlug: string) => memberDataByGuild[guildSlug] ?? emptyData,
      ),
      memberRoleDataForGuild: vi.fn(
        (guildSlug: string) => memberDataByGuild[guildSlug] ?? emptyData,
      ),
      updateMemberRoles: vi.fn(
        async (
          guildSlug: string,
          memberUserId: string,
          input: { roleIds: string[] },
        ) => {
          const existing = memberDataByGuild[guildSlug]
          if (!existing) {
            throw new Error('Guild not found')
          }
          const updatedMember = existing.members.find(
            (member) => member.userId === memberUserId,
          )
          if (!updatedMember) {
            throw new Error('Member not found')
          }
          const nextMember = {
            ...updatedMember,
            roleIds: [...input.roleIds],
            highestRoleColor:
              input.roleIds.length > 0
                ? (existing.roles.find((role) => role.id === input.roleIds[0])
                    ?.color ?? '#99aab5')
                : '#99aab5',
          }
          memberDataByGuild[guildSlug] = {
            ...existing,
            members: existing.members.map((member) =>
              member.userId === memberUserId ? nextMember : member,
            ),
          }
          return nextMember
        },
      ),
    }

    const identityState = {
      session: {
        token: 'token-123',
        user: {
          id: 'user-viewer',
        },
      },
    }

    return { guildState, memberDataByGuild, presenceStatuses, identityState }
  })

const moderationApi = vi.hoisted(() => ({
  createMute: vi.fn(
    async (
      _guildSlug: string,
      _input: {
        targetUserId: string
        reason: string
        durationSeconds?: number | null
        isPermanent?: boolean
      },
    ) => ({
      id: 'mute-1',
      guildSlug: 'lobby',
      actorUserId: 'user-viewer',
      targetUserId: 'user-default',
      reason: 'cooldown',
      durationSeconds: 3600,
      expiresAt: '2026-03-02T00:00:00.000Z',
      isPermanent: false,
      createdAt: '2026-03-01T00:00:00.000Z',
      updatedAt: '2026-03-01T00:00:00.000Z',
    }),
  ),
}))

const presenceState = vi.hoisted(() => ({
  version: 0,
  seedFromMembers: vi.fn(
    (
      members: Array<{
        userId: string
        presenceStatus: 'online' | 'idle' | 'offline'
      }>,
    ) => {
      for (const member of members) {
        if (!presenceStatuses[member.userId]) {
          presenceStatuses[member.userId] = member.presenceStatus
        }
      }
    },
  ),
  statusFor: vi.fn(
    (userId: string, fallback: 'online' | 'idle' | 'offline' = 'offline') =>
      presenceStatuses[userId] ?? fallback,
  ),
  ensureConnected: vi.fn(),
}))

const { blockState, blockedUsers } = vi.hoisted(() => {
  const blockedUsers = new Set<string>()
  const blockState = {
    version: 0,
    isBlocked: vi.fn((userId: string) => blockedUsers.has(userId)),
    blockUser: vi.fn(async (userId: string) => {
      blockedUsers.add(userId)
      blockState.version += 1
      return { synced: true, syncError: null }
    }),
    unblockUser: vi.fn(async (userId: string) => {
      blockedUsers.delete(userId)
      blockState.version += 1
      return { synced: true, syncError: null }
    }),
  }
  return { blockState, blockedUsers }
})

vi.mock('$lib/features/guild/guildStore.svelte', () => ({
  guildState,
}))

vi.mock('$lib/features/identity/identityStore.svelte', () => ({
  identityState,
}))

vi.mock('$lib/features/identity/blockStore.svelte', () => ({
  blockState,
}))

vi.mock('./presenceStore.svelte', () => ({
  presenceState,
}))

vi.mock('$lib/features/moderation/moderationApi', () => moderationApi)

import MemberList from './MemberList.svelte'

function seedGuildData() {
  memberDataByGuild.lobby = {
    members: [
      {
        userId: 'user-viewer',
        username: 'viewer',
        displayName: 'Role Manager',
        avatarColor: '#3366ff',
        presenceStatus: 'online',
        highestRoleColor: '#3366ff',
        roleIds: ['role-manager'],
        isOwner: false,
        canAssignRoles: true,
      },
      {
        userId: 'user-helper-offline',
        username: 'zeta-helper',
        displayName: 'Zeta Helper',
        avatarColor: '#22aa88',
        presenceStatus: 'offline',
        highestRoleColor: '#22aa88',
        roleIds: ['role-helper'],
        isOwner: false,
        canAssignRoles: true,
      },
      {
        userId: 'user-helper-online',
        username: 'alpha-helper',
        displayName: 'Alpha Helper',
        avatarColor: '#22aa88',
        presenceStatus: 'online',
        highestRoleColor: '#22aa88',
        roleIds: ['role-helper'],
        isOwner: false,
        canAssignRoles: true,
      },
      {
        userId: 'user-default',
        username: 'general-user',
        displayName: 'General User',
        avatarColor: '#99aab5',
        presenceStatus: 'offline',
        highestRoleColor: '#99aab5',
        roleIds: [],
        isOwner: false,
        canAssignRoles: false,
      },
    ],
    roles: [
      {
        id: 'role-manager',
        name: 'Role Manager',
        color: '#3366ff',
        position: 1,
        permissionsBitflag: (1 << 2) | (1 << 4) | (1 << 7) | (1 << 11),
        isDefault: false,
        isSystem: false,
        canEdit: true,
        canDelete: true,
        createdAt: '2026-02-28T00:00:00.000Z',
      },
      {
        id: 'role-helper',
        name: 'Invite Helper',
        color: '#22aa88',
        position: 2,
        permissionsBitflag: 1 << 6,
        isDefault: false,
        isSystem: false,
        canEdit: true,
        canDelete: true,
        createdAt: '2026-02-28T00:00:00.000Z',
      },
      {
        id: 'role-everyone',
        name: '@everyone',
        color: '#99aab5',
        position: 2147483647,
        permissionsBitflag: 5633,
        isDefault: true,
        isSystem: true,
        canEdit: false,
        canDelete: false,
        createdAt: '2026-02-28T00:00:00.000Z',
      },
    ],
    assignableRoleIds: ['role-helper'],
    canManageRoles: true,
  }

  Object.assign(presenceStatuses, {
    'user-viewer': 'online',
    'user-helper-offline': 'offline',
    'user-helper-online': 'online',
    'user-default': 'offline',
  })
}

describe('MemberList', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    seedGuildData()
    presenceState.version = 0
    blockedUsers.clear()
    blockState.version = 0
    identityState.session = {
      token: 'token-123',
      user: { id: 'user-viewer' },
    }
    moderationApi.createMute.mockClear()
  })

  it('groups by highest role, sorts online first, and renders status dots', async () => {
    const { getByTestId, getByText, container } = render(MemberList, {
      activeGuild: 'lobby',
    })

    await waitFor(() => {
      expect(guildState.loadMembers).toHaveBeenCalledWith('lobby', true)
    })

    expect(getByTestId('role-group-role-manager')).toBeInTheDocument()
    expect(getByTestId('role-group-role-helper')).toBeInTheDocument()
    expect(getByText('Role Manager (1)')).toBeInTheDocument()
    expect(getByText('Invite Helper (2)')).toBeInTheDocument()

    const helperOnline = getByTestId('member-row-user-helper-online')
    const helperOffline = getByTestId('member-row-user-helper-offline')
    const memberRows = [
      ...container.querySelectorAll('[data-testid^="member-row-"]'),
    ]

    expect(memberRows.indexOf(helperOnline)).toBeLessThan(
      memberRows.indexOf(helperOffline),
    )

    expect(
      getByTestId('member-status-dot-user-helper-online').className,
    ).toContain('bg-emerald-500')
    expect(
      getByTestId('member-status-dot-user-helper-offline').className,
    ).toContain('bg-muted-foreground')
    expect(getByText('@alpha-helper · Online')).toBeInTheDocument()
  })

  it('opens popover from keyboard shortcut and preserves delegated role assignment flow', async () => {
    const { getByTestId, getByRole, getByLabelText } = render(MemberList, {
      activeGuild: 'lobby',
    })

    await waitFor(() => {
      expect(guildState.loadMembers).toHaveBeenCalledWith('lobby', true)
    })

    await fireEvent.keyDown(getByTestId('member-row-user-helper-offline'), {
      key: 'F10',
      shiftKey: true,
    })

    expect(getByRole('button', { name: 'Send DM' })).toBeInTheDocument()
    await fireEvent.click(getByRole('button', { name: 'Assign role' }))
    await fireEvent.click(
      getByLabelText('Toggle Invite Helper for Zeta Helper'),
    )

    await waitFor(() => {
      expect(guildState.updateMemberRoles).toHaveBeenCalledWith(
        'lobby',
        'user-helper-offline',
        {
          roleIds: [],
        },
      )
    })
  })

  it('shows DM intent and moderation actions only for available permissions', async () => {
    const dmIntentHandler = vi.fn()
    window.addEventListener('discool:open-dm-intent', dmIntentHandler)

    const { getByTestId, getByRole, queryByRole } = render(MemberList, {
      activeGuild: 'lobby',
    })

    await waitFor(() => {
      expect(guildState.loadMembers).toHaveBeenCalledWith('lobby', true)
    })

    await fireEvent.keyDown(getByTestId('member-row-user-default'), {
      key: 'ContextMenu',
    })

    expect(getByRole('button', { name: 'Mute member' })).toBeInTheDocument()
    expect(
      getByRole('button', { name: 'Kick member (coming soon)' }),
    ).toBeInTheDocument()
    expect(
      getByRole('button', { name: 'Moderate messages (coming soon)' }),
    ).toBeInTheDocument()
    expect(
      queryByRole('button', { name: 'Ban member (coming soon)' }),
    ).not.toBeInTheDocument()

    await fireEvent.click(getByRole('button', { name: 'Mute member' }))
    expect(
      getByRole('dialog', { name: 'Mute General User' }),
    ).toBeInTheDocument()
    await fireEvent.input(getByTestId('mute-reason-input'), {
      target: { value: 'cooldown' },
    })
    await fireEvent.click(getByTestId('mute-submit-button'))

    await waitFor(() => {
      expect(moderationApi.createMute).toHaveBeenCalledWith('lobby', {
        targetUserId: 'user-default',
        reason: 'cooldown',
        isPermanent: false,
        durationSeconds: 24 * 60 * 60,
      })
    })

    await fireEvent.click(getByRole('button', { name: 'Send DM' }))

    expect(dmIntentHandler).toHaveBeenCalledTimes(1)
    expect(dmIntentHandler.mock.calls[0][0]).toMatchObject({
      detail: {
        guildSlug: 'lobby',
        userId: 'user-default',
      },
    })

    window.removeEventListener('discool:open-dm-intent', dmIntentHandler)
  })

  it('hides blocked members and supports blocking from context actions', async () => {
    const confirmSpy = vi.spyOn(window, 'confirm').mockReturnValue(true)
    blockedUsers.add('user-helper-online')

    const { getByTestId, getByRole, queryByTestId } = render(MemberList, {
      activeGuild: 'lobby',
    })

    await waitFor(() => {
      expect(guildState.loadMembers).toHaveBeenCalledWith('lobby', true)
    })

    expect(
      queryByTestId('member-row-user-helper-online'),
    ).not.toBeInTheDocument()

    await fireEvent.keyDown(getByTestId('member-row-user-default'), {
      key: 'ContextMenu',
    })

    await fireEvent.click(getByRole('button', { name: 'Block user' }))

    await waitFor(() => {
      expect(blockState.blockUser).toHaveBeenCalledWith('user-default', {
        displayName: 'General User',
        username: 'general-user',
        avatarColor: '#99aab5',
      })
    })
    confirmSpy.mockRestore()
  })

  it('virtualizes long member lists and renders only a windowed subset', async () => {
    memberDataByGuild.lobby = {
      ...memberDataByGuild.lobby,
      members: [
        memberDataByGuild.lobby.members[0],
        ...Array.from({ length: 120 }, (_, index) => ({
          userId: `bulk-${index}`,
          username: `bulk-${index.toString().padStart(3, '0')}`,
          displayName: `Bulk ${index}`,
          avatarColor: '#99aab5',
          presenceStatus: 'offline' as const,
          highestRoleColor: '#99aab5',
          roleIds: [],
          isOwner: false,
          canAssignRoles: false,
        })),
      ],
    }

    for (let index = 0; index < 120; index += 1) {
      presenceStatuses[`bulk-${index}`] = 'offline'
    }

    const { container, getByTestId, queryByTestId } = render(MemberList, {
      activeGuild: 'lobby',
    })

    await waitFor(() => {
      expect(guildState.loadMembers).toHaveBeenCalledWith('lobby', true)
    })

    const initiallyRenderedRows = container.querySelectorAll(
      '[data-testid^="member-row-"]',
    )
    expect(initiallyRenderedRows.length).toBeLessThan(60)

    const scroll = getByTestId('member-list-scroll')
    Object.defineProperty(scroll, 'clientHeight', {
      value: 240,
      configurable: true,
    })
    scroll.scrollTop = 2200
    await fireEvent.scroll(scroll)

    expect(queryByTestId('member-row-bulk-0')).not.toBeInTheDocument()
    expect(getByTestId('member-row-bulk-35')).toBeInTheDocument()
  })
})
