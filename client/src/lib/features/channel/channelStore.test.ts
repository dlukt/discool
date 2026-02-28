import { beforeEach, describe, expect, it, vi } from 'vitest'

const channelApi = vi.hoisted(() => ({
  listChannels: vi.fn(),
  listCategories: vi.fn(),
  createChannel: vi.fn(),
  updateChannel: vi.fn(),
  deleteChannel: vi.fn(),
  reorderChannels: vi.fn(),
  setCategoryCollapsed: vi.fn(),
}))

vi.mock('./channelApi', () => channelApi)

import { channelState } from './channelStore.svelte'

type TestChannel = {
  id: string
  slug: string
  name: string
  channelType: 'text' | 'voice'
  position: number
  isDefault: boolean
  categorySlug: string | null
  createdAt: string
}

type Deferred<T> = {
  promise: Promise<T>
  resolve: (value: T) => void
  reject: (reason?: unknown) => void
}

function deferred<T>(): Deferred<T> {
  let resolve = (_value: T) => {}
  let reject = (_reason?: unknown) => {}
  const promise = new Promise<T>((innerResolve, innerReject) => {
    resolve = innerResolve
    reject = innerReject
  })
  return { promise, resolve, reject }
}

describe('channelStore', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    channelState.clear()
  })

  it('ignores stale channel list responses after switching guilds', async () => {
    const guildA = deferred<TestChannel[]>()
    const guildB = deferred<TestChannel[]>()
    vi.mocked(channelApi.listCategories).mockResolvedValue([])

    vi.mocked(channelApi.listChannels)
      .mockReturnValueOnce(guildA.promise)
      .mockReturnValueOnce(guildB.promise)

    const firstLoad = channelState.loadChannels('guild-a')
    const secondLoad = channelState.loadChannels('guild-b')

    guildB.resolve([
      {
        id: 'b-1',
        slug: 'general',
        name: 'general',
        channelType: 'text',
        position: 0,
        isDefault: true,
        categorySlug: null,
        createdAt: '2026-02-28T00:00:00.000Z',
      },
    ])
    await secondLoad

    guildA.resolve([
      {
        id: 'a-1',
        slug: 'alpha',
        name: 'alpha',
        channelType: 'text',
        position: 0,
        isDefault: true,
        categorySlug: null,
        createdAt: '2026-02-28T00:00:00.000Z',
      },
    ])
    await firstLoad

    expect(channelState.activeGuild).toBe('guild-b')
    expect(channelState.channels.map((channel) => channel.slug)).toEqual([
      'general',
    ])
    expect(channelState.loadedByGuild['guild-b']).toBe(true)
    expect(channelState.loadedByGuild['guild-a']).toBeUndefined()
  })

  it('ignores stale channel load failures after switching guilds', async () => {
    const guildA = deferred<TestChannel[]>()
    const guildB = deferred<TestChannel[]>()
    vi.mocked(channelApi.listCategories).mockResolvedValue([])

    vi.mocked(channelApi.listChannels)
      .mockReturnValueOnce(guildA.promise)
      .mockReturnValueOnce(guildB.promise)

    const firstLoad = channelState.loadChannels('guild-a')
    const secondLoad = channelState.loadChannels('guild-b')

    guildB.resolve([
      {
        id: 'b-1',
        slug: 'general',
        name: 'general',
        channelType: 'text',
        position: 0,
        isDefault: true,
        categorySlug: null,
        createdAt: '2026-02-28T00:00:00.000Z',
      },
    ])
    await secondLoad

    guildA.reject(new Error('stale guild-a failure'))
    await expect(firstLoad).resolves.toEqual(channelState.channels)

    expect(channelState.activeGuild).toBe('guild-b')
    expect(channelState.channels.map((channel) => channel.slug)).toEqual([
      'general',
    ])
    expect(channelState.error).toBeNull()
  })

  it('reuses cached guild data when switching back to a loaded guild', async () => {
    vi.mocked(channelApi.listChannels)
      .mockResolvedValueOnce([
        {
          id: 'a-1',
          slug: 'alpha',
          name: 'alpha',
          channelType: 'text',
          position: 0,
          isDefault: true,
          categorySlug: null,
          createdAt: '2026-02-28T00:00:00.000Z',
        },
      ])
      .mockResolvedValueOnce([
        {
          id: 'b-1',
          slug: 'general',
          name: 'general',
          channelType: 'text',
          position: 0,
          isDefault: true,
          categorySlug: null,
          createdAt: '2026-02-28T00:00:00.000Z',
        },
      ])
    vi.mocked(channelApi.listCategories).mockResolvedValue([])

    await channelState.loadChannels('guild-a')
    await channelState.loadChannels('guild-b')
    vi.clearAllMocks()

    await channelState.loadChannels('guild-a')

    expect(channelState.activeGuild).toBe('guild-a')
    expect(channelState.channels.map((channel) => channel.slug)).toEqual([
      'alpha',
    ])
    expect(channelApi.listChannels).not.toHaveBeenCalled()
    expect(channelApi.listCategories).not.toHaveBeenCalled()
  })

  it('clears loading when switching from an in-flight guild to cached guild data', async () => {
    const guildC = deferred<TestChannel[]>()
    vi.mocked(channelApi.listChannels)
      .mockResolvedValueOnce([
        {
          id: 'a-1',
          slug: 'alpha',
          name: 'alpha',
          channelType: 'text',
          position: 0,
          isDefault: true,
          categorySlug: null,
          createdAt: '2026-02-28T00:00:00.000Z',
        },
      ])
      .mockReturnValueOnce(guildC.promise)
    vi.mocked(channelApi.listCategories).mockResolvedValue([])

    await channelState.loadChannels('guild-a')

    const inFlightLoad = channelState.loadChannels('guild-c')
    expect(channelState.loading).toBe(true)

    await channelState.loadChannels('guild-a')

    expect(channelState.activeGuild).toBe('guild-a')
    expect(channelState.loading).toBe(false)
    expect(channelState.channels.map((channel) => channel.slug)).toEqual([
      'alpha',
    ])

    guildC.resolve([
      {
        id: 'c-1',
        slug: 'gamma',
        name: 'gamma',
        channelType: 'text',
        position: 0,
        isDefault: true,
        categorySlug: null,
        createdAt: '2026-02-28T00:00:00.000Z',
      },
    ])
    await inFlightLoad

    expect(channelState.activeGuild).toBe('guild-a')
    expect(channelState.loading).toBe(false)
    expect(channelState.channels.map((channel) => channel.slug)).toEqual([
      'alpha',
    ])
  })

  it('updates category collapse state per guild', async () => {
    vi.mocked(channelApi.listChannels).mockResolvedValue([
      {
        id: 'b-1',
        slug: 'general',
        name: 'general',
        channelType: 'text',
        position: 0,
        isDefault: true,
        categorySlug: null,
        createdAt: '2026-02-28T00:00:00.000Z',
      },
    ])
    vi.mocked(channelApi.listCategories).mockResolvedValue([
      {
        id: 'c-1',
        slug: 'ops',
        name: 'Ops',
        position: 0,
        collapsed: false,
        createdAt: '2026-02-28T00:00:00.000Z',
      },
    ])
    vi.mocked(channelApi.setCategoryCollapsed).mockResolvedValue({
      id: 'c-1',
      slug: 'ops',
      name: 'Ops',
      position: 0,
      collapsed: true,
      createdAt: '2026-02-28T00:00:00.000Z',
    })

    await channelState.loadChannels('guild-b')
    await channelState.setCategoryCollapsed('guild-b', 'ops', true)

    expect(channelApi.setCategoryCollapsed).toHaveBeenCalledWith(
      'guild-b',
      'ops',
      true,
    )
    expect(channelState.categories[0]?.collapsed).toBe(true)
  })
})
