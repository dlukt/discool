import { ApiError, apiFetchCursorList } from '$lib/api'
import { type ChatMessage, type ChatMessageWire, toChatMessage } from './types'

export type FetchChannelHistoryInput = {
  limit?: number
  before?: string | null
}

export type ChannelHistoryPage = {
  messages: ChatMessage[]
  cursor: string | null
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
