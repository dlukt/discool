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

  it('keeps edit/delete disabled for non-owner messages', () => {
    const { getByRole } = render(MessageBubble, {
      message: makeMessage(),
      currentUserId: 'user-2',
    })

    expect(getByRole('button', { name: 'Edit message' })).toBeDisabled()
    expect(getByRole('button', { name: 'Delete message' })).toBeDisabled()
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
})
