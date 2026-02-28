import { fireEvent, render, waitFor } from '@testing-library/svelte'
import { beforeEach, describe, expect, it, vi } from 'vitest'

const { guildState, listInvites, createInvite, revokeInvite } = vi.hoisted(
  () => {
    const guildState = {
      bySlug: vi.fn(),
    }
    const listInvites = vi.fn()
    const createInvite = vi.fn()
    const revokeInvite = vi.fn()
    return { guildState, listInvites, createInvite, revokeInvite }
  },
)

vi.mock('./guildStore.svelte', () => ({
  guildState,
}))

vi.mock('./guildApi', () => ({
  listInvites,
  createInvite,
  revokeInvite,
}))

import InviteModal from './InviteModal.svelte'

function ownerGuild() {
  return {
    id: 'guild-1',
    slug: 'makers-hub',
    name: 'Makers Hub',
    defaultChannelSlug: 'general',
    isOwner: true,
    createdAt: '2026-02-28T00:00:00.000Z',
  }
}

describe('InviteModal', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    guildState.bySlug.mockImplementation((slug: string) =>
      slug === 'makers-hub' ? ownerGuild() : null,
    )
    listInvites.mockResolvedValue([])
    createInvite.mockResolvedValue({
      code: 'invite-code-1',
      type: 'single_use',
      usesRemaining: 1,
      createdBy: 'user-1',
      creatorUsername: 'owner',
      createdAt: '2026-02-28T00:00:00.000Z',
      revoked: false,
      inviteUrl: '/invite/invite-code-1',
    })
    revokeInvite.mockResolvedValue({
      code: 'invite-code-1',
      revoked: true,
    })
  })

  it('generates invite and copies link with required success message', async () => {
    const writeText = vi.fn().mockResolvedValue(undefined)
    Object.defineProperty(navigator, 'clipboard', {
      configurable: true,
      value: { writeText },
    })

    const { getByLabelText, getByRole, findByText } = render(InviteModal, {
      open: true,
      guildSlug: 'makers-hub',
    })

    await waitFor(() => expect(listInvites).toHaveBeenCalledWith('makers-hub'))
    await fireEvent.change(getByLabelText('Invite type'), {
      target: { value: 'single_use' },
    })
    await fireEvent.click(getByRole('button', { name: 'Generate invite' }))

    await waitFor(() =>
      expect(createInvite).toHaveBeenCalledWith('makers-hub', {
        type: 'single_use',
      }),
    )

    await fireEvent.click(getByRole('button', { name: 'Copy' }))
    await waitFor(() =>
      expect(writeText).toHaveBeenCalledWith(
        expect.stringContaining('/invite/invite-code-1'),
      ),
    )
    expect(await findByText('Invite link copied')).toBeInTheDocument()
  })

  it('revokes invite and removes card from list', async () => {
    listInvites.mockResolvedValue([
      {
        code: 'invite-code-1',
        type: 'reusable',
        usesRemaining: 0,
        createdBy: 'user-1',
        creatorUsername: 'owner',
        createdAt: '2026-02-28T00:00:00.000Z',
        revoked: false,
        inviteUrl: '/invite/invite-code-1',
      },
    ])

    const { getByRole, getByTestId, queryByTestId } = render(InviteModal, {
      open: true,
      guildSlug: 'makers-hub',
    })

    await waitFor(() =>
      expect(getByTestId('invite-card-invite-code-1')).toBeInTheDocument(),
    )
    await fireEvent.click(getByRole('button', { name: 'Revoke' }))

    await waitFor(() =>
      expect(revokeInvite).toHaveBeenCalledWith('makers-hub', 'invite-code-1'),
    )
    await waitFor(() =>
      expect(
        queryByTestId('invite-card-invite-code-1'),
      ).not.toBeInTheDocument(),
    )
  })
})
