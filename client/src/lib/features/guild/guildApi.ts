import { apiFetch } from '$lib/api'

import {
  type CreateGuildInput,
  type Guild,
  type GuildWire,
  toCreateGuildInputWire,
  toGuild,
  toUpdateGuildInputWire,
  type UpdateGuildInput,
} from './types'

export function listGuilds(): Promise<Guild[]> {
  return apiFetch<GuildWire[]>('/api/v1/guilds').then((wireGuilds) =>
    wireGuilds.map(toGuild),
  )
}

export function createGuild(input: CreateGuildInput): Promise<Guild> {
  return apiFetch<GuildWire>('/api/v1/guilds', {
    method: 'POST',
    body: JSON.stringify(toCreateGuildInputWire(input)),
  }).then(toGuild)
}

export function updateGuild(
  guildSlug: string,
  input: UpdateGuildInput,
): Promise<Guild> {
  return apiFetch<GuildWire>(
    `/api/v1/guilds/${encodeURIComponent(guildSlug)}`,
    {
      method: 'PATCH',
      body: JSON.stringify(toUpdateGuildInputWire(input)),
    },
  ).then(toGuild)
}

export function uploadGuildIcon(guildSlug: string, file: File): Promise<Guild> {
  const formData = new FormData()
  formData.append('icon', file)
  return apiFetch<GuildWire>(
    `/api/v1/guilds/${encodeURIComponent(guildSlug)}/icon`,
    {
      method: 'POST',
      body: formData,
    },
  ).then(toGuild)
}
