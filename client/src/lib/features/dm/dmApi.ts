import { ApiError, apiFetch, apiFetchCursorList } from '$lib/api'
import {
  type ChatMessage,
  type ChatMessageWire,
  toChatMessage,
} from '../chat/types'
import type { DmConversationWire } from './types'

export type FetchDmHistoryInput = {
  limit?: number
  before?: string | null
}

export type DmHistoryPage = {
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

function buildDmHistoryPath(
  dmSlug: string,
  input: FetchDmHistoryInput,
): string {
  const dm = normalizePathPart(dmSlug, 'dmSlug')
  const params = new URLSearchParams()
  if (typeof input.limit === 'number' && Number.isFinite(input.limit)) {
    params.set('limit', String(Math.trunc(input.limit)))
  }
  if (input.before) {
    params.set('before', input.before)
  }
  const query = params.toString()
  const base = `/api/v1/dms/${dm}/messages`
  return query ? `${base}?${query}` : base
}

export async function listDms(): Promise<DmConversationWire[]> {
  const response = await apiFetch<DmConversationWire[]>('/api/v1/dms')
  if (!Array.isArray(response)) {
    throw new ApiError('INVALID_RESPONSE', 'Invalid server response')
  }
  return response
}

export async function openDm(userId: string): Promise<DmConversationWire> {
  const normalizedUserId = userId.trim()
  if (!normalizedUserId) {
    throw new ApiError('VALIDATION_ERROR', 'userId is required')
  }
  return apiFetch<DmConversationWire>('/api/v1/dms', {
    method: 'POST',
    body: JSON.stringify({ user_id: normalizedUserId }),
  })
}

export async function fetchDmHistory(
  dmSlug: string,
  input: FetchDmHistoryInput = {},
): Promise<DmHistoryPage> {
  const path = buildDmHistoryPath(dmSlug, input)
  const response = await apiFetchCursorList<ChatMessageWire[]>(path)
  if (!Array.isArray(response.data)) {
    throw new ApiError('INVALID_RESPONSE', 'Invalid server response')
  }
  return {
    messages: response.data.map(toChatMessage),
    cursor: response.cursor,
  }
}
