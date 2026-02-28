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
      permissionsBitflag: 0,
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
      id: 'role-everyone',
      name: '@everyone',
      color: '#99aab5',
      position: 2147483647,
      permissionsBitflag: 0,
      isDefault: true,
      isSystem: true,
      canEdit: false,
      canDelete: false,
      createdAt: '2026-02-28T00:00:00.000Z',
    },
  ]
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
