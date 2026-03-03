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

const {
  deleteAccount,
  saveProfile,
  startRecoveryEmailAssociation,
  requestPersonalDataExport,
  identityState,
} = vi.hoisted(() => {
  const deleteAccount = vi.fn()
  const saveProfile = vi.fn()
  const startRecoveryEmailAssociation = vi.fn()
  const requestPersonalDataExport = vi.fn()
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
    deleteAccount,
    saveProfile,
    startRecoveryEmailAssociation,
    requestPersonalDataExport,
    recoveryEmailStatus: null as {
      associated: boolean
      emailMasked: string | null
      verified: boolean
      verifiedAt: string | null
    } | null,
    recoveryEmailLoading: false,
  }
  return {
    deleteAccount,
    saveProfile,
    startRecoveryEmailAssociation,
    requestPersonalDataExport,
    identityState,
  }
})

const { blockState } = vi.hoisted(() => ({
  blockState: {
    version: 0,
    blockedUsers: vi.fn(() => [] as BlockedUserRecord[]),
    unblockUser: vi.fn(async () => ({ synced: true, syncError: null })),
  },
}))

const { toastState } = vi.hoisted(() => ({
  toastState: {
    show: vi.fn(),
  },
}))

vi.mock('./identityStore.svelte', () => ({ identityState }))
vi.mock('./blockStore.svelte', () => ({ blockState }))
vi.mock('$lib/feedback/toastStore.svelte', () => ({ toastState }))

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
    deleteAccount.mockReset()
    requestPersonalDataExport.mockReset()
    blockState.version = 0
    blockState.blockedUsers.mockReset()
    blockState.blockedUsers.mockReturnValue([])
    blockState.unblockUser.mockClear()
    toastState.show.mockReset()
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

  it('shows progress after 2 seconds for slow personal data export requests', async () => {
    vi.useFakeTimers()
    const clickSpy = vi
      .spyOn(HTMLAnchorElement.prototype, 'click')
      .mockImplementation(() => {})
    try {
      const exportPromise = new Promise((resolve) => {
        setTimeout(
          () =>
            resolve({
              profile: {
                userId: 'user-1',
                didKey: 'did:key:z6Mk-test',
                username: 'alice',
                displayName: 'Alice',
                avatarColor: '#3b82f6',
                avatarUrl: null,
                email: null,
                emailVerifiedAt: null,
                createdAt: '2026-02-24T00:00:00.000Z',
                updatedAt: '2026-02-24T00:00:00.000Z',
              },
              guildMemberships: [],
              messages: [],
              dmMessages: [],
              reactions: [],
              uploadedFiles: [],
              blockList: [],
              exportedAt: '2026-03-02T00:00:00.000Z',
            }),
          2500,
        )
      })
      requestPersonalDataExport.mockReturnValue(exportPromise)

      const { getByRole, queryByText } = render(ProfileSettingsView)
      const button = getByRole('button', { name: 'Export my data' })

      await fireEvent.click(button)
      expect(requestPersonalDataExport).toHaveBeenCalledTimes(1)
      expect(button).toBeDisabled()
      expect(
        queryByText('Preparing your data export...'),
      ).not.toBeInTheDocument()

      await vi.advanceTimersByTimeAsync(2001)
      expect(queryByText('Preparing your data export...')).toBeInTheDocument()

      await fireEvent.click(button)
      expect(requestPersonalDataExport).toHaveBeenCalledTimes(1)

      await vi.advanceTimersByTimeAsync(600)
      await waitFor(() => {
        expect(button).not.toBeDisabled()
      })
    } finally {
      clickSpy.mockRestore()
      vi.useRealTimers()
    }
  })

  it('downloads personal data export and shows success toast copy', async () => {
    vi.useFakeTimers()
    const originalCreateObjectURL = URL.createObjectURL
    const originalRevokeObjectURL = URL.revokeObjectURL
    try {
      const createObjectURL = vi.fn(() => 'blob:personal-export')
      const revokeObjectURL = vi.fn()
      ;(
        URL as unknown as { createObjectURL: typeof URL.createObjectURL }
      ).createObjectURL = createObjectURL
      ;(
        URL as unknown as { revokeObjectURL: typeof URL.revokeObjectURL }
      ).revokeObjectURL = revokeObjectURL

      const clickSpy = vi
        .spyOn(HTMLAnchorElement.prototype, 'click')
        .mockImplementation(() => {})

      requestPersonalDataExport.mockResolvedValue({
        profile: {
          userId: 'user-1',
          didKey: 'did:key:z6Mk-test',
          username: 'alice',
          displayName: 'Alice',
          avatarColor: '#3b82f6',
          avatarUrl: null,
          email: null,
          emailVerifiedAt: null,
          createdAt: '2026-02-24T00:00:00.000Z',
          updatedAt: '2026-02-24T00:00:00.000Z',
        },
        guildMemberships: [],
        messages: [],
        dmMessages: [],
        reactions: [],
        uploadedFiles: [],
        blockList: [],
        exportedAt: '2026-03-02T00:00:00.000Z',
      })

      const { getByRole } = render(ProfileSettingsView)
      await fireEvent.click(getByRole('button', { name: 'Export my data' }))

      await waitFor(() => {
        expect(requestPersonalDataExport).toHaveBeenCalledTimes(1)
      })
      await vi.runAllTimersAsync()

      expect(createObjectURL).toHaveBeenCalledTimes(1)
      expect(clickSpy).toHaveBeenCalledTimes(1)
      expect(revokeObjectURL).toHaveBeenCalledWith('blob:personal-export')
      expect(toastState.show).toHaveBeenCalledWith({
        variant: 'success',
        message: 'Your data export is ready for download',
      })
    } finally {
      vi.useRealTimers()
      ;(
        URL as unknown as { createObjectURL: typeof URL.createObjectURL }
      ).createObjectURL = originalCreateObjectURL
      ;(
        URL as unknown as { revokeObjectURL: typeof URL.revokeObjectURL }
      ).revokeObjectURL = originalRevokeObjectURL
    }
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

  it('requires exact username confirmation before enabling account deletion', async () => {
    const { getByRole, getByText, getByTestId } = render(ProfileSettingsView)

    await fireEvent.click(getByRole('button', { name: 'Delete my account' }))
    expect(
      getByText(
        'This will permanently delete your identity and all associated data from this instance. This cannot be undone.',
      ),
    ).toBeInTheDocument()

    const confirmButton = getByTestId('delete-account-confirm-button')
    expect(confirmButton).toBeDisabled()

    await fireEvent.input(getByTestId('delete-account-confirm-input'), {
      target: { value: 'Alice' },
    })
    expect(confirmButton).toBeDisabled()

    await fireEvent.input(getByTestId('delete-account-confirm-input'), {
      target: { value: 'alice' },
    })
    expect(confirmButton).not.toBeDisabled()
  })

  it('deletes account and shows success toast', async () => {
    deleteAccount.mockResolvedValue(undefined)
    const { getByRole, getByTestId } = render(ProfileSettingsView)

    await fireEvent.click(getByRole('button', { name: 'Delete my account' }))
    await fireEvent.input(getByTestId('delete-account-confirm-input'), {
      target: { value: 'alice' },
    })
    await fireEvent.click(getByTestId('delete-account-confirm-button'))

    await waitFor(() => {
      expect(deleteAccount).toHaveBeenCalledWith('alice')
    })
    expect(toastState.show).toHaveBeenCalledWith({
      variant: 'success',
      message: 'Account deleted from this instance',
    })
  })

  it('shows deletion API errors in the confirmation dialog', async () => {
    deleteAccount.mockRejectedValue(
      new ApiError(
        'CONFLICT',
        'Transfer ownership or delete owned guilds first',
      ),
    )
    const { getByRole, getByTestId, findByRole } = render(ProfileSettingsView)

    await fireEvent.click(getByRole('button', { name: 'Delete my account' }))
    await fireEvent.input(getByTestId('delete-account-confirm-input'), {
      target: { value: 'alice' },
    })
    await fireEvent.click(getByTestId('delete-account-confirm-button'))

    const alert = await findByRole('alert')
    expect(alert).toHaveTextContent(
      'Transfer ownership or delete owned guilds first',
    )
  })
})
