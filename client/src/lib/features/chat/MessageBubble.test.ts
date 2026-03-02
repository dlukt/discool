import { fireEvent, render, within } from '@testing-library/svelte'
import { describe, expect, it, vi } from 'vitest'
import MessageBubble from './MessageBubble.svelte'
import type { ChatMessage } from './types'

function makeMessage(overrides: Partial<ChatMessage> = {}): ChatMessage {
  return {
    id: 'message-1',
    guildSlug: 'lobby',
    channelSlug: 'general',
    authorUserId: 'user-1',
    authorUsername: 'alice',
    authorDisplayName: 'Alice',
    authorAvatarColor: '#3366ff',
    authorRoleColor: '#3366ff',
    content: 'hello',
    isSystem: false,
    createdAt: '2026-02-28T00:00:00Z',
    updatedAt: '2026-02-28T00:00:00Z',
    optimistic: false,
    attachments: [],
    reactions: [],
    embeds: [],
    ...overrides,
  }
}

describe('MessageBubble', () => {
  it('renders edited label when updated_at differs from created_at', () => {
    const { getByTestId } = render(MessageBubble, {
      message: makeMessage({ updatedAt: '2026-02-28T00:00:05Z' }),
      currentUserId: 'user-1',
    })

    expect(getByTestId('message-edited-message-1')).toBeInTheDocument()
  })

  it('supports context-menu keyboard flow for edit/delete own message', async () => {
    const onEditRequest = vi.fn()
    const onDeleteRequest = vi.fn()
    const { getByTestId } = render(MessageBubble, {
      message: makeMessage(),
      currentUserId: 'user-1',
      onEditRequest,
      onDeleteRequest,
    })

    const row = getByTestId('message-row-message-1')
    await fireEvent.keyDown(row, { key: 'ContextMenu' })
    const editMenu = getByTestId('message-context-menu-message-1')
    await fireEvent.click(
      within(editMenu).getByRole('menuitem', { name: 'Edit' }),
    )
    expect(onEditRequest).toHaveBeenCalledTimes(1)

    await fireEvent.keyDown(row, { key: 'ContextMenu' })
    const deleteMenu = getByTestId('message-context-menu-message-1')
    await fireEvent.click(
      within(deleteMenu).getByRole('menuitem', { name: 'Delete message' }),
    )
    expect(onDeleteRequest).toHaveBeenCalledTimes(1)
  })

  it('opens emoji picker and emits selected emoji reaction', async () => {
    const onReactRequest = vi.fn()
    const { getByTestId } = render(MessageBubble, {
      message: makeMessage(),
      currentUserId: 'user-1',
      onReactRequest,
    })

    await fireEvent.click(getByTestId('message-react-button-message-1'))
    expect(getByTestId('message-reaction-picker-message-1')).toBeInTheDocument()

    await fireEvent.click(
      getByTestId('message-reaction-picker-option-message-1-0'),
    )
    expect(onReactRequest).toHaveBeenCalledWith(
      expect.objectContaining({ id: 'message-1' }),
      '😀',
    )
  })

  it('clicking a reaction badge toggles that emoji', async () => {
    const onReactRequest = vi.fn()
    const { getByTestId } = render(MessageBubble, {
      message: makeMessage({
        reactions: [
          { emoji: '🎉', count: 3, reacted: true },
          { emoji: '👍', count: 1, reacted: false },
        ],
      }),
      currentUserId: 'user-1',
      onReactRequest,
    })

    await fireEvent.click(getByTestId('message-reaction-badge-message-1-0'))
    expect(onReactRequest).toHaveBeenCalledWith(
      expect.objectContaining({ id: 'message-1' }),
      '🎉',
    )
  })

  it('keeps edit/delete disabled for non-owner messages and allows reporting', () => {
    const { getByRole } = render(MessageBubble, {
      message: makeMessage(),
      currentUserId: 'user-2',
    })

    expect(getByRole('button', { name: 'Edit message' })).toBeDisabled()
    expect(getByRole('button', { name: 'Delete message' })).toBeDisabled()
    expect(getByRole('button', { name: 'Report message' })).toBeEnabled()
  })

  it('enables delete for moderators on non-owned messages', async () => {
    const onDeleteRequest = vi.fn()
    const { getByRole } = render(MessageBubble, {
      message: makeMessage(),
      currentUserId: 'user-2',
      hasManageMessagesPermission: true,
      onDeleteRequest,
    })

    const deleteButton = getByRole('button', { name: 'Delete message' })
    expect(deleteButton).toBeEnabled()
    await fireEvent.click(deleteButton)
    expect(onDeleteRequest).toHaveBeenCalledWith(
      expect.objectContaining({ id: 'message-1' }),
    )
  })

  it('emits report callbacks for non-owned messages', async () => {
    const onReportRequest = vi.fn()
    const { getByRole, getByTestId } = render(MessageBubble, {
      message: makeMessage(),
      currentUserId: 'user-2',
      onReportRequest,
    })

    await fireEvent.click(getByRole('button', { name: 'Report message' }))
    expect(onReportRequest).toHaveBeenCalledWith(
      expect.objectContaining({ id: 'message-1' }),
    )

    const row = getByTestId('message-row-message-1')
    await fireEvent.keyDown(row, { key: 'ContextMenu' })
    await fireEvent.click(
      within(getByTestId('message-context-menu-message-1')).getByRole(
        'menuitem',
        {
          name: 'Report',
        },
      ),
    )
    expect(onReportRequest).toHaveBeenCalledTimes(2)
  })

  it('renders image and file attachments with fullscreen preview', async () => {
    const { getByTestId } = render(MessageBubble, {
      message: makeMessage({
        content: '',
        attachments: [
          {
            id: 'att-image',
            storageKey: 'attachment-image.png',
            originalFilename: 'image.png',
            mimeType: 'image/png',
            sizeBytes: 2048,
            isImage: true,
            url: '/image.png',
          },
          {
            id: 'att-file',
            storageKey: 'attachment-file.pdf',
            originalFilename: 'doc.pdf',
            mimeType: 'application/pdf',
            sizeBytes: 4096,
            isImage: false,
            url: '/doc.pdf',
          },
        ],
      }),
      currentUserId: 'user-1',
    })

    expect(
      getByTestId('message-attachment-image-message-1-0'),
    ).toBeInTheDocument()
    expect(
      getByTestId('message-attachment-file-message-1-1'),
    ).toBeInTheDocument()

    await fireEvent.click(getByTestId('message-attachment-image-message-1-0'))
    expect(getByTestId('message-image-preview-message-1')).toBeInTheDocument()
  })

  it('renders markdown syntax and safe autolinks', () => {
    const { getByTestId } = render(MessageBubble, {
      message: makeMessage({
        content:
          '**bold** *italic* ~~strike~~ `inline`\n> quote\n\n```ts\nconst answer = 42\n```\nhttps://example.com',
      }),
      currentUserId: 'user-1',
    })

    const content = getByTestId('message-content-message-1')
    expect(content.querySelector('strong')?.textContent).toBe('bold')
    expect(content.querySelector('em')?.textContent).toBe('italic')
    expect(content.querySelector('del')?.textContent).toBe('strike')
    expect(content.querySelector('blockquote')).toBeInTheDocument()
    expect(content.querySelector('pre code.hljs')).toBeInTheDocument()
    const link = content.querySelector('a') as HTMLAnchorElement | null
    expect(link).not.toBeNull()
    expect(link?.getAttribute('href')).toBe('https://example.com')
    expect(link?.getAttribute('rel')).toContain('noopener')
  })

  it('ignores unsafe code fence language attributes', () => {
    const { getByTestId } = render(MessageBubble, {
      message: makeMessage({
        content:
          '```" onmouseover="alert(1)\nconst answer = 42\n```\nhttps://example.com',
      }),
      currentUserId: 'user-1',
    })

    const content = getByTestId('message-content-message-1')
    const codeBlock = content.querySelector('pre code')
    expect(codeBlock).toBeInTheDocument()
    expect(codeBlock?.getAttribute('class')).toBe('hljs')
    expect(content.querySelector('[onmouseover]')).toBeNull()
  })

  it('does not create clickable links or thumbnails for local/private URLs', () => {
    const { getByTestId } = render(MessageBubble, {
      message: makeMessage({
        content: 'http://127.0.0.1/admin',
        embeds: [
          {
            id: 'embed-local',
            url: 'http://127.0.0.1/admin',
            domain: '127.0.0.1',
            title: 'Local admin',
            description: null,
            thumbnailUrl: 'http://[::1]/thumb.png',
          },
        ],
      }),
      currentUserId: 'user-1',
    })

    const content = getByTestId('message-content-message-1')
    expect(content.querySelector('a')).toBeNull()

    const card = getByTestId('message-embed-card-message-1-0')
    expect(card.querySelector('a')).toBeNull()
    expect(card.querySelector('img')).toBeNull()
    expect(card).toHaveTextContent('Local admin')
  })

  it('renders compact embed cards and tolerates missing fields', () => {
    const { getByTestId, queryByTestId } = render(MessageBubble, {
      message: makeMessage({
        embeds: [
          {
            id: 'embed-1',
            url: 'https://example.com/post',
            domain: 'example.com',
            title: 'Example Post',
            description: 'Description',
            thumbnailUrl: 'https://example.com/thumb.png',
          },
          {
            id: 'embed-2',
            url: 'https://example.com/empty',
            domain: 'example.com',
            title: null,
            description: null,
            thumbnailUrl: null,
          },
        ],
      }),
      currentUserId: 'user-1',
    })

    const firstCard = getByTestId('message-embed-card-message-1-0')
    expect(firstCard).toBeInTheDocument()
    const firstLink = firstCard.querySelector('a') as HTMLAnchorElement | null
    expect(firstLink?.getAttribute('href')).toBe('https://example.com/post')
    expect(firstLink?.getAttribute('rel')).toContain('noopener')
    expect(queryByTestId('message-embed-card-message-1-1')).toBeInTheDocument()
  })
})
