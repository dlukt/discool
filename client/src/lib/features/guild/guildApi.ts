import { apiFetch } from '$lib/api'

import {
  type CreateGuildInput,
  type CreateGuildInviteInput,
  type Guild,
  type GuildInvite,
  type GuildInviteWire,
  type GuildWire,
  type InviteMetadata,
  type InviteMetadataWire,
  type JoinGuildByInviteResult,
  type JoinGuildByInviteResultWire,
  type RevokeGuildInviteResult,
  type RevokeGuildInviteResultWire,
  toCreateGuildInputWire,
  toCreateGuildInviteInputWire,
  toGuild,
  toGuildInvite,
  toInviteMetadata,
  toJoinGuildByInviteResult,
  toRevokeGuildInviteResult,
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

export function listInvites(guildSlug: string): Promise<GuildInvite[]> {
  return apiFetch<GuildInviteWire[]>(
    `/api/v1/guilds/${encodeURIComponent(guildSlug)}/invites`,
  ).then((invites) => invites.map(toGuildInvite))
}

export function createInvite(
  guildSlug: string,
  input: CreateGuildInviteInput,
): Promise<GuildInvite> {
  return apiFetch<GuildInviteWire>(
    `/api/v1/guilds/${encodeURIComponent(guildSlug)}/invites`,
    {
      method: 'POST',
      body: JSON.stringify(toCreateGuildInviteInputWire(input)),
    },
  ).then(toGuildInvite)
}

export function revokeInvite(
  guildSlug: string,
  inviteCode: string,
): Promise<RevokeGuildInviteResult> {
  return apiFetch<RevokeGuildInviteResultWire>(
    `/api/v1/guilds/${encodeURIComponent(guildSlug)}/invites/${encodeURIComponent(inviteCode)}`,
    {
      method: 'DELETE',
    },
  ).then(toRevokeGuildInviteResult)
}

export function getInviteMetadata(inviteCode: string): Promise<InviteMetadata> {
  return apiFetch<InviteMetadataWire>(
    `/api/v1/invites/${encodeURIComponent(inviteCode)}`,
  ).then(toInviteMetadata)
}

export function joinGuildByInvite(
  inviteCode: string,
): Promise<JoinGuildByInviteResult> {
  return apiFetch<JoinGuildByInviteResultWire>(
    `/api/v1/invites/${encodeURIComponent(inviteCode)}/join`,
    {
      method: 'POST',
      body: '{}',
    },
  ).then(toJoinGuildByInviteResult)
}
