import { fireEvent, render, waitFor } from '@testing-library/svelte'
import { beforeEach, describe, expect, it, vi } from 'vitest'

import { ApiError } from '$lib/api'

type BlockedUserRecord = {
  userId: string
  displayName: string | null
  username: string | null
  avatarColor: string | null
  intervals: Array<{ blockedAt: string; unblockedAt: string | null }>
}

const { saveProfile, startRecoveryEmailAssociation, identityState } =
  vi.hoisted(() => {
    const saveProfile = vi.fn()
    const startRecoveryEmailAssociation = vi.fn()
    const identityState = {
      session: {
        token: 'token-1',
        expiresAt: '2026-03-01T00:00:00.000Z',
        user: {
          id: 'user-1',
          didKey: 'did:key:z6Mk-test',
          username: 'alice',
          displayName: 'Alice',
          avatarColor: '#3b82f6' as string | null,
          avatarUrl: null as string | null,
          createdAt: '2026-02-24T00:00:00.000Z',
        },
      },
      saveProfile,
      startRecoveryEmailAssociation,
      recoveryEmailStatus: null as {
        associated: boolean
        emailMasked: string | null
        verified: boolean
        verifiedAt: string | null
      } | null,
      recoveryEmailLoading: false,
    }
    return { saveProfile, startRecoveryEmailAssociation, identityState }
  })

const { blockState } = vi.hoisted(() => ({
  blockState: {
    version: 0,
    blockedUsers: vi.fn(() => [] as BlockedUserRecord[]),
    unblockUser: vi.fn(async () => ({ synced: true, syncError: null })),
  },
}))

vi.mock('./identityStore.svelte', () => ({ identityState }))
vi.mock('./blockStore.svelte', () => ({ blockState }))

import ProfileSettingsView from './ProfileSettingsView.svelte'

describe('ProfileSettingsView', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    identityState.session = {
      token: 'token-1',
      expiresAt: '2026-03-01T00:00:00.000Z',
      user: {
        id: 'user-1',
        didKey: 'did:key:z6Mk-test',
        username: 'alice',
        displayName: 'Alice',
        avatarColor: '#3b82f6',
        avatarUrl: null,
        createdAt: '2026-02-24T00:00:00.000Z',
      },
    }
    identityState.recoveryEmailStatus = null
    identityState.recoveryEmailLoading = false
    blockState.version = 0
    blockState.blockedUsers.mockReset()
    blockState.blockedUsers.mockReturnValue([])
    blockState.unblockUser.mockClear()
  })

  it('validates display name on blur', async () => {
    const { getByLabelText, findByText } = render(ProfileSettingsView)

    const input = getByLabelText('Display name')
    await fireEvent.input(input, { target: { value: '   ' } })
    await fireEvent.blur(input)

    expect(await findByText('Display name is required.')).toBeInTheDocument()
  })

  it('shows and removes selected avatar preview', async () => {
    const { getByLabelText, getByRole, queryByRole } =
      render(ProfileSettingsView)

    await fireEvent.click(getByLabelText('Upload image'))

    const file = new File(
      [new Uint8Array([0x89, 0x50, 0x4e, 0x47])],
      'avatar.png',
      { type: 'image/png' },
    )
    await fireEvent.change(getByLabelText('Avatar image'), {
      target: { files: [file] },
    })

    expect(getByRole('img', { name: 'Avatar preview' })).toBeInTheDocument()
    await fireEvent.click(
      getByRole('button', { name: 'Remove selected image' }),
    )
    expect(
      queryByRole('button', { name: 'Remove selected image' }),
    ).not.toBeInTheDocument()
  })

  it('saves profile changes and selected avatar', async () => {
    saveProfile.mockResolvedValue(undefined)
    const { getByLabelText, getByRole, getByText } = render(ProfileSettingsView)

    await fireEvent.input(getByLabelText('Display name'), {
      target: { value: 'Alice Cooper' },
    })
    await fireEvent.click(getByLabelText('Upload image'))

    const file = new File(
      [new Uint8Array([0x89, 0x50, 0x4e, 0x47])],
      'avatar.png',
      { type: 'image/png' },
    )
    await fireEvent.change(getByLabelText('Avatar image'), {
      target: { files: [file] },
    })

    await fireEvent.click(getByRole('button', { name: 'Save profile' }))

    await waitFor(() =>
      expect(saveProfile).toHaveBeenCalledWith(
        { displayName: 'Alice Cooper' },
        file,
      ),
    )
    expect(getByText('Profile saved.')).toBeInTheDocument()
  })

  it('switching from image avatar mode to color saves avatar color selection', async () => {
    saveProfile.mockResolvedValue(undefined)
    identityState.session = {
      token: 'token-1',
      expiresAt: '2026-03-01T00:00:00.000Z',
      user: {
        id: 'user-1',
        didKey: 'did:key:z6Mk-test',
        username: 'alice',
        displayName: 'Alice',
        avatarColor: '#3b82f6',
        avatarUrl: '/api/v1/users/me/avatar',
        createdAt: '2026-02-24T00:00:00.000Z',
      },
    }

    const { getByLabelText, getByRole } = render(ProfileSettingsView)
    await fireEvent.click(getByLabelText('Use avatar color'))
    await fireEvent.click(getByRole('button', { name: 'Save profile' }))

    await waitFor(() =>
      expect(saveProfile).toHaveBeenCalledWith(
        { displayName: 'Alice', avatarColor: '#3b82f6' },
        null,
      ),
    )
  })

  it('shows API validation errors', async () => {
    saveProfile.mockRejectedValue(
      new ApiError('VALIDATION_ERROR', 'Unsupported avatar image type'),
    )
    const { getByRole, findByRole } = render(ProfileSettingsView)

    await fireEvent.click(getByRole('button', { name: 'Save profile' }))

    const alert = await findByRole('alert')
    expect(alert).toHaveTextContent('Unsupported avatar image type')
  })

  it('renders recovery email status and sends verification action', async () => {
    startRecoveryEmailAssociation.mockResolvedValue({
      associated: true,
      emailMasked: 'a***@example.com',
      verified: false,
      verifiedAt: null,
    })

    const { getByText, getByPlaceholderText, getByRole } =
      render(ProfileSettingsView)
    expect(getByText('Status:')).toBeInTheDocument()
    expect(getByText('Not configured')).toBeInTheDocument()

    await fireEvent.input(getByPlaceholderText('name@example.com'), {
      target: { value: 'alice@example.com' },
    })
    await fireEvent.click(getByRole('button', { name: 'Send verification' }))

    await waitFor(() =>
      expect(startRecoveryEmailAssociation).toHaveBeenCalledWith(
        'alice@example.com',
      ),
    )
    expect(
      getByText(
        'Verification email sent. Check your inbox and click the link to verify.',
      ),
    ).toBeInTheDocument()
  })

  it('renders blocked users and unblocks entries from settings', async () => {
    blockState.blockedUsers.mockReturnValue([
      {
        userId: 'user-2',
        displayName: 'Bob',
        username: 'bob',
        avatarColor: '#22aa88',
        intervals: [
          { blockedAt: '2026-03-01T00:00:00.000Z', unblockedAt: null },
        ],
      },
    ])

    const { getByText, getByRole } = render(ProfileSettingsView)
    expect(getByText('Blocked users')).toBeInTheDocument()
    expect(getByText('Bob')).toBeInTheDocument()

    await fireEvent.click(getByRole('button', { name: 'Unblock' }))

    await waitFor(() => {
      expect(blockState.unblockUser).toHaveBeenCalledWith('user-2')
    })
    expect(getByText('Unblocked Bob.')).toBeInTheDocument()
  })
})
