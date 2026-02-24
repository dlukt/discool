import { beforeEach, describe, expect, it, vi } from 'vitest'

import { ApiError, downloadBackup, setSessionToken } from '$lib/api'

describe('downloadBackup', () => {
  const originalFetch = globalThis.fetch
  const originalCreateObjectURL = URL.createObjectURL
  const originalRevokeObjectURL = URL.revokeObjectURL

  beforeEach(() => {
    vi.restoreAllMocks()
    setSessionToken(null)
    document.body.innerHTML = ''
  })

  it('downloads a blob with a sanitized filename', async () => {
    vi.useFakeTimers()
    try {
      setSessionToken('token-123')

      const createObjectURL = vi.fn(() => 'blob:mock')
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

      const originalCreate = document.createElement.bind(document)
      let createdAnchor: { download: string } = { download: '' }
      vi.spyOn(document, 'createElement').mockImplementation(
        (tagName: string) => {
          const el = originalCreate(tagName) as HTMLElement
          if (tagName.toLowerCase() === 'a') {
            createdAnchor = el as unknown as { download: string }
          }
          return el
        },
      )

      globalThis.fetch = vi.fn(async (_url: string, init?: RequestInit) => {
        expect(init?.method).toBe('POST')
        expect(init?.headers).toEqual({ authorization: 'Bearer token-123' })

        const blob = new Blob(['backup-bytes'], {
          type: 'application/octet-stream',
        })
        return new Response(blob, {
          status: 200,
          headers: {
            'content-disposition':
              "attachment; filename*=UTF-8''..%2F..%2Fevil%0Aname.sql",
          },
        })
      }) as typeof fetch

      await downloadBackup()
      await vi.runAllTimersAsync()

      expect(createObjectURL).toHaveBeenCalledTimes(1)
      expect(clickSpy).toHaveBeenCalledTimes(1)
      expect(createdAnchor.download).toBe('evilname.sql')
      expect(revokeObjectURL).toHaveBeenCalledWith('blob:mock')
    } finally {
      vi.useRealTimers()
      globalThis.fetch = originalFetch
      ;(
        URL as unknown as { createObjectURL: typeof URL.createObjectURL }
      ).createObjectURL = originalCreateObjectURL
      ;(
        URL as unknown as { revokeObjectURL: typeof URL.revokeObjectURL }
      ).revokeObjectURL = originalRevokeObjectURL
    }
  })

  it('throws ApiError when server returns JSON error envelope', async () => {
    globalThis.fetch = vi.fn(async () => {
      return new Response(
        JSON.stringify({
          error: { code: 'FORBIDDEN', message: 'Nope', details: {} },
        }),
        { status: 403, headers: { 'content-type': 'application/json' } },
      )
    }) as typeof fetch

    try {
      await expect(downloadBackup()).rejects.toBeInstanceOf(ApiError)
      await expect(downloadBackup()).rejects.toMatchObject({
        code: 'FORBIDDEN',
        message: 'Nope',
      })
    } finally {
      globalThis.fetch = originalFetch
    }
  })
})
