import { ApiError, apiFetch, apiFetchCursorList } from '$lib/api'
import { type ChatMessage, type ChatMessageWire, toChatMessage } from './types'

export type FetchChannelHistoryInput = {
  limit?: number
  before?: string | null
}

export type ChannelHistoryPage = {
  messages: ChatMessage[]
  cursor: string | null
}

export type UploadMessageAttachmentInput = {
  file: File
  content?: string
  clientNonce?: string
  onProgress?: (percentage: number) => void
}

function normalizePathPart(value: string, field: string): string {
  const trimmed = value.trim()
  if (!trimmed) {
    throw new ApiError('VALIDATION_ERROR', `${field} is required`)
  }
  return encodeURIComponent(trimmed)
}

function buildHistoryPath(
  guildSlug: string,
  channelSlug: string,
  input: FetchChannelHistoryInput,
): string {
  const guild = normalizePathPart(guildSlug, 'guildSlug')
  const channel = normalizePathPart(channelSlug, 'channelSlug')
  const params = new URLSearchParams()
  if (typeof input.limit === 'number' && Number.isFinite(input.limit)) {
    params.set('limit', String(Math.trunc(input.limit)))
  }
  if (input.before) {
    params.set('before', input.before)
  }

  const query = params.toString()
  const base = `/api/v1/guilds/${guild}/channels/${channel}/messages`
  return query ? `${base}?${query}` : base
}

function buildAttachmentUploadPath(
  guildSlug: string,
  channelSlug: string,
): string {
  const guild = normalizePathPart(guildSlug, 'guildSlug')
  const channel = normalizePathPart(channelSlug, 'channelSlug')
  return `/api/v1/guilds/${guild}/channels/${channel}/messages/attachments`
}

export async function fetchChannelHistory(
  guildSlug: string,
  channelSlug: string,
  input: FetchChannelHistoryInput = {},
): Promise<ChannelHistoryPage> {
  const path = buildHistoryPath(guildSlug, channelSlug, input)
  const response = await apiFetchCursorList<ChatMessageWire[]>(path)
  if (!Array.isArray(response.data)) {
    throw new ApiError('INVALID_RESPONSE', 'Invalid server response')
  }

  return {
    messages: response.data.map(toChatMessage),
    cursor: response.cursor,
  }
}

export async function uploadMessageAttachment(
  guildSlug: string,
  channelSlug: string,
  input: UploadMessageAttachmentInput,
): Promise<ChatMessage> {
  const path = buildAttachmentUploadPath(guildSlug, channelSlug)
  const formData = new FormData()
  formData.append('file', input.file)
  if (typeof input.content === 'string') {
    formData.append('content', input.content)
  }
  if (typeof input.clientNonce === 'string' && input.clientNonce.trim()) {
    formData.append('client_nonce', input.clientNonce.trim())
  }

  const reportProgress = input.onProgress
  let progress = 0
  reportProgress?.(progress)
  const timer = globalThis.setInterval(() => {
    progress = Math.min(progress + 10, 90)
    reportProgress?.(progress)
  }, 120)

  try {
    const response = await apiFetch<ChatMessageWire>(path, {
      method: 'POST',
      body: formData,
    })
    reportProgress?.(100)
    return toChatMessage(response)
  } finally {
    globalThis.clearInterval(timer)
  }
}
