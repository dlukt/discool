import { fireEvent, render, waitFor, within } from '@testing-library/svelte'
import { beforeEach, describe, expect, it, vi } from 'vitest'

const { guildState } = vi.hoisted(() => {
  const guildState = {
    bySlug: vi.fn(),
    updateGuild: vi.fn(),
    loadRoles: vi.fn(),
    rolesForGuild: vi.fn(),
    createRole: vi.fn(),
    updateRole: vi.fn(),
    deleteRole: vi.fn(),
    reorderRoles: vi.fn(),
  }
  return { guildState }
})

vi.mock('./guildStore.svelte', () => ({
  guildState,
}))

import GuildSettings from './GuildSettings.svelte'

function ownerGuild() {
  return {
    id: 'guild-1',
    slug: 'makers-hub',
    name: 'Makers Hub',
    description: 'Build cool things',
    defaultChannelSlug: 'general',
    isOwner: true,
    createdAt: '2026-02-28T00:00:00.000Z',
  }
}

function ownerRoles() {
  return [
    {
      id: 'owner:user-1',
      name: 'Owner',
      color: '#f59e0b',
      position: -1,
      permissionsBitflag: 8191,
      isDefault: false,
      isSystem: true,
      canEdit: false,
      canDelete: false,
      createdAt: '2026-02-28T00:00:00.000Z',
    },
    {
      id: 'role-moderators',
      name: 'Moderators',
      color: '#3366ff',
      position: 0,
      permissionsBitflag: 0,
      isDefault: false,
      isSystem: false,
      canEdit: true,
      canDelete: true,
      createdAt: '2026-02-28T00:00:00.000Z',
    },
    {
      id: 'role-helpers',
      name: 'Helpers',
      color: '#22aa88',
      position: 1,
      permissionsBitflag: 0,
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
  ]
}

function buildDragDataTransfer(sourceId: string) {
  return {
    setData: vi.fn(),
    getData: vi.fn(() => sourceId),
    effectAllowed: 'move',
    dropEffect: 'move',
  }
}

function roleOrder(container: HTMLElement): string[] {
  return Array.from(
    container.querySelectorAll('[data-testid="guild-role-name"]'),
  )
    .map((node) => node.textContent?.trim() ?? '')
    .filter(Boolean)
}

describe('GuildSettings', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    guildState.bySlug.mockImplementation((slug: string) =>
      slug === 'makers-hub' ? ownerGuild() : null,
    )
    guildState.updateGuild.mockResolvedValue(ownerGuild())
    guildState.loadRoles.mockResolvedValue(ownerRoles())
    guildState.rolesForGuild.mockImplementation((slug: string) =>
      slug === 'makers-hub' ? ownerRoles() : [],
    )
    guildState.createRole.mockResolvedValue(ownerRoles()[1])
    guildState.updateRole.mockResolvedValue({
      ...ownerRoles()[1],
      name: 'Moderation Team',
      color: '#7744aa',
    })
    guildState.deleteRole.mockResolvedValue({
      deletedId: 'role-moderators',
      removedAssignmentCount: 1,
    })
    guildState.reorderRoles.mockResolvedValue(ownerRoles())
  })

  it('saves owner edits for guild name and description', async () => {
    const { getByLabelText, getByRole, getByText } = render(GuildSettings, {
      open: true,
      guildSlug: 'makers-hub',
    })

    await fireEvent.input(getByLabelText('Guild name'), {
      target: { value: 'Makers Hub Updated' },
    })
    await fireEvent.input(getByLabelText('Description'), {
      target: { value: 'Updated by owner' },
    })
    await fireEvent.click(getByRole('button', { name: 'Save Guild' }))

    await waitFor(() =>
      expect(guildState.updateGuild).toHaveBeenCalledWith(
        'makers-hub',
        {
          name: 'Makers Hub Updated',
          description: 'Updated by owner',
        },
        null,
      ),
    )
    expect(getByText('Guild settings saved.')).toBeInTheDocument()
  })

  it('renders role hierarchy and supports create/edit/delete flows', async () => {
    const { getByRole, getByText, getByLabelText, findByText } = render(
      GuildSettings,
      {
        open: true,
        guildSlug: 'makers-hub',
      },
    )

    await waitFor(() => {
      expect(guildState.loadRoles).toHaveBeenCalledWith('makers-hub')
    })

    expect(getByText('Owner')).toBeInTheDocument()
    expect(getByText('@everyone')).toBeInTheDocument()

    await fireEvent.click(getByRole('button', { name: 'Create role' }))
    const createDialog = getByRole('dialog', { name: 'Create role' })
    await fireEvent.input(within(createDialog).getByLabelText('Role name'), {
      target: { value: 'Support Team' },
    })
    await fireEvent.input(within(createDialog).getByLabelText('Role color'), {
      target: { value: '#6633ff' },
    })
    await fireEvent.submit(within(createDialog).getByTestId('create-role-form'))

    await waitFor(() => {
      expect(guildState.createRole).toHaveBeenCalledWith('makers-hub', {
        name: 'Support Team',
        color: '#6633ff',
      })
    })

    await fireEvent.click(getByLabelText('Edit role Moderators'))
    const editDialog = getByRole('dialog', { name: 'Edit role' })
    await fireEvent.input(within(editDialog).getByLabelText('Role name'), {
      target: { value: 'Moderation Team' },
    })
    await fireEvent.input(within(editDialog).getByLabelText('Role color'), {
      target: { value: '#7744aa' },
    })
    await fireEvent.submit(within(editDialog).getByTestId('edit-role-form'))

    await waitFor(() => {
      expect(guildState.updateRole).toHaveBeenCalledWith(
        'makers-hub',
        'role-moderators',
        {
          name: 'Moderation Team',
          color: '#7744aa',
        },
      )
    })

    await fireEvent.click(getByLabelText('Delete role Moderators'))
    const deleteDialog = getByRole('dialog', { name: 'Delete role' })
    expect(
      await findByText(
        'This action is irreversible and removes this role from all assigned members.',
      ),
    ).toBeInTheDocument()
    await fireEvent.click(
      within(deleteDialog).getByRole('button', { name: 'Delete role' }),
    )
    await waitFor(() => {
      expect(guildState.deleteRole).toHaveBeenCalledWith(
        'makers-hub',
        'role-moderators',
      )
    })
  })

  it('lists canonical permissions and autosaves toggles with failure feedback', async () => {
    const { getByLabelText, getByRole, getByText } = render(GuildSettings, {
      open: true,
      guildSlug: 'makers-hub',
    })

    guildState.updateRole
      .mockResolvedValueOnce({
        ...ownerRoles()[1],
        permissionsBitflag: 2,
      })
      .mockRejectedValueOnce(new Error('Permission save failed'))

    await fireEvent.click(getByLabelText('Edit permissions for Moderators'))
    const dialog = getByRole('dialog', { name: 'Role permissions' })

    expect(within(dialog).getByLabelText('Send messages')).toBeInTheDocument()
    expect(within(dialog).getByLabelText('Manage channels')).toBeInTheDocument()
    expect(within(dialog).getByLabelText('Kick members')).toBeInTheDocument()
    expect(within(dialog).getByLabelText('Ban members')).toBeInTheDocument()
    expect(within(dialog).getByLabelText('Manage roles')).toBeInTheDocument()
    expect(within(dialog).getByLabelText('Manage guild')).toBeInTheDocument()
    expect(within(dialog).getByLabelText('Manage invites')).toBeInTheDocument()
    expect(within(dialog).getByLabelText('Mute members')).toBeInTheDocument()
    expect(within(dialog).getByLabelText('View mod log')).toBeInTheDocument()
    expect(within(dialog).getByLabelText('Attach files')).toBeInTheDocument()
    expect(within(dialog).getByLabelText('Add reactions')).toBeInTheDocument()
    expect(within(dialog).getByLabelText('Manage messages')).toBeInTheDocument()

    await fireEvent.click(within(dialog).getByLabelText('Manage channels'))
    await waitFor(() => {
      expect(guildState.updateRole).toHaveBeenNthCalledWith(
        1,
        'makers-hub',
        'role-moderators',
        { permissionsBitflag: 2 },
      )
    })
    expect(getByText('Permissions saved for Moderators.')).toBeInTheDocument()

    await fireEvent.click(within(dialog).getByLabelText('Ban members'))
    await waitFor(() => {
      expect(guildState.updateRole).toHaveBeenNthCalledWith(
        2,
        'makers-hub',
        'role-moderators',
        { permissionsBitflag: 10 },
      )
    })
    expect(
      await within(dialog).findByText('Permission save failed'),
    ).toBeInTheDocument()
  })

  it('shows owner permissions as read-only implicit all', async () => {
    const { getByLabelText, getByRole } = render(GuildSettings, {
      open: true,
      guildSlug: 'makers-hub',
    })

    await fireEvent.click(getByLabelText('Edit permissions for Owner'))
    const dialog = getByRole('dialog', { name: 'Role permissions' })

    expect(
      within(dialog).getByText(
        'The Owner role always has all permissions implicitly and cannot be modified.',
      ),
    ).toBeInTheDocument()
    expect(within(dialog).getByLabelText('Manage guild')).toBeDisabled()
  })

  it('supports drag reorder for custom roles and keeps system roles fixed', async () => {
    const { getByTestId, getByText } = render(GuildSettings, {
      open: true,
      guildSlug: 'makers-hub',
    })

    expect(getByTestId('guild-role-item-owner:user-1')).toHaveAttribute(
      'draggable',
      'false',
    )
    expect(getByTestId('guild-role-item-role-everyone')).toHaveAttribute(
      'draggable',
      'false',
    )
    expect(getByTestId('guild-role-item-role-moderators')).toHaveAttribute(
      'draggable',
      'true',
    )

    const dataTransfer = buildDragDataTransfer('role-moderators')
    await fireEvent.dragStart(getByTestId('guild-role-item-role-moderators'), {
      dataTransfer,
    })
    await fireEvent.dragOver(getByTestId('guild-role-item-role-helpers'), {
      dataTransfer,
    })
    await fireEvent.drop(getByTestId('guild-role-item-role-helpers'), {
      dataTransfer,
    })

    await waitFor(() => {
      expect(guildState.reorderRoles).toHaveBeenCalledWith('makers-hub', [
        'role-helpers',
        'role-moderators',
      ])
    })
    expect(getByText('Role order updated.')).toBeInTheDocument()
  })

  it('rolls back optimistic role order when reorder fails', async () => {
    guildState.reorderRoles.mockRejectedValueOnce(
      new Error('Role reorder failed'),
    )
    const { container, getByTestId, findByText } = render(GuildSettings, {
      open: true,
      guildSlug: 'makers-hub',
    })
    const initialOrder = roleOrder(container)

    const dataTransfer = buildDragDataTransfer('role-moderators')
    await fireEvent.dragStart(getByTestId('guild-role-item-role-moderators'), {
      dataTransfer,
    })
    await fireEvent.dragOver(getByTestId('guild-role-item-role-helpers'), {
      dataTransfer,
    })
    await fireEvent.drop(getByTestId('guild-role-item-role-helpers'), {
      dataTransfer,
    })

    expect(await findByText('Role reorder failed')).toBeInTheDocument()
    expect(roleOrder(container)).toEqual(initialOrder)
  })

  it('shows owner-only guardrail for non-owners', () => {
    guildState.bySlug.mockReturnValue({
      ...ownerGuild(),
      isOwner: false,
    })
    const { getByText, queryByRole } = render(GuildSettings, {
      open: true,
      guildSlug: 'makers-hub',
    })

    expect(
      getByText('Only guild owners can edit guild settings.'),
    ).toBeInTheDocument()
    expect(
      queryByRole('button', { name: 'Save Guild' }),
    ).not.toBeInTheDocument()
    expect(
      queryByRole('button', { name: 'Create role' }),
    ).not.toBeInTheDocument()
  })
})
