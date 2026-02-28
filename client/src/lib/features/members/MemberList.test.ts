import { fireEvent, render, waitFor } from '@testing-library/svelte'
import { beforeEach, describe, expect, it, vi } from 'vitest'

const { guildState, memberDataByGuild } = vi.hoisted(() => {
  const memberDataByGuild: Record<
    string,
    {
      members: Array<{
        userId: string
        username: string
        displayName: string
        avatarColor?: string
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

  return { guildState, memberDataByGuild }
})

vi.mock('$lib/features/guild/guildStore.svelte', () => ({
  guildState,
}))

import MemberList from './MemberList.svelte'

function seedGuildData() {
  memberDataByGuild.lobby = {
    members: [
      {
        userId: 'user-manager',
        username: 'manager',
        displayName: 'Role Manager',
        highestRoleColor: '#3366ff',
        roleIds: ['role-manager'],
        isOwner: false,
        canAssignRoles: true,
      },
      {
        userId: 'user-target',
        username: 'target',
        displayName: 'Target User',
        highestRoleColor: '#99aab5',
        roleIds: [],
        isOwner: false,
        canAssignRoles: true,
      },
    ],
    roles: [
      {
        id: 'role-manager',
        name: 'Role Manager',
        color: '#3366ff',
        position: 1,
        permissionsBitflag: 80,
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
        permissionsBitflag: 64,
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
}

describe('MemberList', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    seedGuildData()
  })

  it('renders API-backed member rows and highest-role username colors', async () => {
    const { getByTestId, getByText } = render(MemberList, {
      activeGuild: 'lobby',
    })

    await waitFor(() => {
      expect(guildState.loadMembers).toHaveBeenCalledWith('lobby', true)
    })

    expect(getByText('Role Manager')).toBeInTheDocument()
    expect(getByText('Target User')).toBeInTheDocument()
    expect(getByTestId('member-display-name-user-manager')).toHaveStyle(
      'color: #3366ff',
    )
  })

  it('opens assign role controls from keyboard context-menu shortcut and updates roles', async () => {
    const { getByTestId, getByRole, getByLabelText } = render(MemberList, {
      activeGuild: 'lobby',
    })

    await waitFor(() => {
      expect(guildState.loadMembers).toHaveBeenCalledWith('lobby', true)
    })

    await fireEvent.keyDown(getByTestId('member-row-user-target'), {
      key: 'F10',
      shiftKey: true,
    })
    await fireEvent.click(getByRole('button', { name: 'Assign role' }))
    await fireEvent.click(
      getByLabelText('Toggle Invite Helper for Target User'),
    )

    await waitFor(() => {
      expect(guildState.updateMemberRoles).toHaveBeenCalledWith(
        'lobby',
        'user-target',
        {
          roleIds: ['role-helper'],
        },
      )
    })
  })

  it('restores role toggle state and shows actionable error text on failure', async () => {
    vi.mocked(guildState.updateMemberRoles).mockRejectedValueOnce(
      new Error('Role update failed'),
    )

    const { getByTestId, getByRole, getByLabelText, findByText } = render(
      MemberList,
      {
        activeGuild: 'lobby',
      },
    )

    await waitFor(() => {
      expect(guildState.loadMembers).toHaveBeenCalledWith('lobby', true)
    })

    await fireEvent.keyDown(getByTestId('member-row-user-target'), {
      key: 'ContextMenu',
    })
    await fireEvent.click(getByRole('button', { name: 'Assign role' }))
    const helperToggle = getByLabelText('Toggle Invite Helper for Target User')
    await fireEvent.click(helperToggle)

    expect(await findByText('Role update failed')).toBeInTheDocument()
    expect(helperToggle).not.toBeChecked()
  })
})
