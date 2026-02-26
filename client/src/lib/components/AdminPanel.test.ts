import { fireEvent, render } from '@testing-library/svelte'
import { tick } from 'svelte'
import { beforeEach, describe, expect, it, vi } from 'vitest'

vi.mock('$lib/api', async () => {
  const actual = await vi.importActual<typeof import('$lib/api')>('$lib/api')
  return {
    ...actual,
    downloadBackup: vi.fn(),
    getAdminHealth: vi.fn(),
  }
})

import { ApiError, downloadBackup, getAdminHealth } from '$lib/api'
import AdminPanel from './AdminPanel.svelte'

const sampleHealth = {
  cpuUsagePercent: 12.3,
  memoryRssBytes: 42 * 1024 * 1024,
  uptimeSeconds: 60,
  dbSizeBytes: 1024,
  dbPoolActive: 1,
  dbPoolIdle: 0,
  dbPoolMax: 5,
  websocketConnections: 0,
  p2pDiscoveryEnabled: true,
  p2pDiscoveryLabel: 'Enabled',
}

describe('AdminPanel', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('renders health metrics after load', async () => {
    vi.mocked(getAdminHealth).mockResolvedValue(sampleHealth)

    const { findByText } = render(AdminPanel)

    expect(await findByText('CPU')).toBeInTheDocument()
    expect(await findByText('12.3%')).toBeInTheDocument()
    expect(await findByText('Enabled')).toBeInTheDocument()
    expect(getAdminHealth).toHaveBeenCalledTimes(1)
  })

  it('shows initial error and retries', async () => {
    vi.mocked(getAdminHealth)
      .mockRejectedValueOnce(
        new ApiError('HTTP_ERROR', 'Could not load health data.'),
      )
      .mockResolvedValueOnce(sampleHealth)

    const { findByText, getByRole } = render(AdminPanel)

    expect(await findByText('Could not load health data.')).toBeInTheDocument()

    await fireEvent.click(getByRole('button', { name: 'Retry' }))

    expect(await findByText('CPU')).toBeInTheDocument()
    expect(getAdminHealth).toHaveBeenCalledTimes(2)
  })

  it('shows backup slow-loading copy and success state', async () => {
    vi.useFakeTimers()
    try {
      vi.mocked(getAdminHealth).mockResolvedValue(sampleHealth)

      let resolveBackup = () => {}
      vi.mocked(downloadBackup).mockImplementation(
        () =>
          new Promise<void>((resolve) => {
            resolveBackup = () => resolve()
          }),
      )

      const { findByRole, queryByText, getByText } = render(AdminPanel)
      const download = await findByRole('button', { name: 'Download Backup' })

      await fireEvent.click(download)
      expect(downloadBackup).toHaveBeenCalledTimes(1)

      await vi.advanceTimersByTimeAsync(1999)
      expect(queryByText('Creating backup...')).not.toBeInTheDocument()

      await vi.advanceTimersByTimeAsync(1)
      expect(getByText('Creating backup...')).toBeInTheDocument()

      resolveBackup()
      await tick()
      await tick()

      expect(getByText('Backup complete.')).toBeInTheDocument()

      await vi.advanceTimersByTimeAsync(4000)
      await tick()
      expect(queryByText('Backup complete.')).not.toBeInTheDocument()
    } finally {
      vi.useRealTimers()
    }
  })

  it('shows disabled discovery label from health payload', async () => {
    vi.mocked(getAdminHealth).mockResolvedValue({
      ...sampleHealth,
      p2pDiscoveryEnabled: false,
      p2pDiscoveryLabel: 'Disabled (Unlisted)',
    })

    const { findByText } = render(AdminPanel)
    expect(await findByText('Disabled (Unlisted)')).toBeInTheDocument()
  })

  it('shows backup error and allows retry', async () => {
    vi.mocked(getAdminHealth).mockResolvedValue(sampleHealth)
    vi.mocked(downloadBackup)
      .mockRejectedValueOnce(new ApiError('HTTP_ERROR', 'Backup failed'))
      .mockResolvedValueOnce(undefined)

    const { findByRole, findByText, getByRole } = render(AdminPanel)

    const download = await findByRole('button', { name: 'Download Backup' })
    await fireEvent.click(download)

    expect(await findByText('Backup failed')).toBeInTheDocument()

    await fireEvent.click(getByRole('button', { name: 'Retry' }))
    expect(await findByText('Backup complete.')).toBeInTheDocument()
    expect(downloadBackup).toHaveBeenCalledTimes(2)
  })
})
