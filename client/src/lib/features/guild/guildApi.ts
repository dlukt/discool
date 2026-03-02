import { apiFetch } from '$lib/api'

import {
  type CreateGuildInput,
  type CreateGuildInviteInput,
  type CreateGuildRoleInput,
  type DeleteGuildRoleResult,
  type DeleteGuildRoleResultWire,
  type Guild,
  type GuildBan,
  type GuildBanWire,
  type GuildInvite,
  type GuildInviteWire,
  type GuildMember,
  type GuildMemberRoleData,
  type GuildMemberRoleDataWire,
  type GuildMemberWire,
  type GuildRole,
  type GuildRoleWire,
  type GuildWire,
  type InviteMetadata,
  type InviteMetadataWire,
  type JoinGuildByInviteResult,
  type JoinGuildByInviteResultWire,
  type ReorderGuildRolesInput,
  type RevokeGuildInviteResult,
  type RevokeGuildInviteResultWire,
  toCreateGuildInputWire,
  toCreateGuildInviteInputWire,
  toCreateGuildRoleInputWire,
  toDeleteGuildRoleResult,
  toGuild,
  toGuildBan,
  toGuildInvite,
  toGuildMember,
  toGuildMemberRoleData,
  toGuildRole,
  toInviteMetadata,
  toJoinGuildByInviteResult,
  toReorderGuildRolesInputWire,
  toRevokeGuildInviteResult,
  toUnbanGuildBanResult,
  toUpdateGuildInputWire,
  toUpdateGuildMemberRolesInputWire,
  toUpdateGuildRoleInputWire,
  type UnbanGuildBanResult,
  type UnbanGuildBanResultWire,
  type UpdateGuildInput,
  type UpdateGuildMemberRolesInput,
  type UpdateGuildRoleInput,
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

export function listRoles(guildSlug: string): Promise<GuildRole[]> {
  return apiFetch<GuildRoleWire[]>(
    `/api/v1/guilds/${encodeURIComponent(guildSlug)}/roles`,
  ).then((roles) => roles.map(toGuildRole))
}

export function createRole(
  guildSlug: string,
  input: CreateGuildRoleInput,
): Promise<GuildRole> {
  return apiFetch<GuildRoleWire>(
    `/api/v1/guilds/${encodeURIComponent(guildSlug)}/roles`,
    {
      method: 'POST',
      body: JSON.stringify(toCreateGuildRoleInputWire(input)),
    },
  ).then(toGuildRole)
}

export function updateRole(
  guildSlug: string,
  roleId: string,
  input: UpdateGuildRoleInput,
): Promise<GuildRole> {
  return apiFetch<GuildRoleWire>(
    `/api/v1/guilds/${encodeURIComponent(guildSlug)}/roles/${encodeURIComponent(roleId)}`,
    {
      method: 'PATCH',
      body: JSON.stringify(toUpdateGuildRoleInputWire(input)),
    },
  ).then(toGuildRole)
}

export function deleteRole(
  guildSlug: string,
  roleId: string,
): Promise<DeleteGuildRoleResult> {
  return apiFetch<DeleteGuildRoleResultWire>(
    `/api/v1/guilds/${encodeURIComponent(guildSlug)}/roles/${encodeURIComponent(roleId)}`,
    {
      method: 'DELETE',
    },
  ).then(toDeleteGuildRoleResult)
}

export function reorderRoles(
  guildSlug: string,
  input: ReorderGuildRolesInput,
): Promise<GuildRole[]> {
  return apiFetch<GuildRoleWire[]>(
    `/api/v1/guilds/${encodeURIComponent(guildSlug)}/roles/reorder`,
    {
      method: 'PATCH',
      body: JSON.stringify(toReorderGuildRolesInputWire(input)),
    },
  ).then((roles) => roles.map(toGuildRole))
}

export function listMembers(guildSlug: string): Promise<GuildMemberRoleData> {
  return apiFetch<GuildMemberRoleDataWire>(
    `/api/v1/guilds/${encodeURIComponent(guildSlug)}/members`,
  ).then(toGuildMemberRoleData)
}

export function updateMemberRoles(
  guildSlug: string,
  memberUserId: string,
  input: UpdateGuildMemberRolesInput,
): Promise<GuildMember> {
  return apiFetch<GuildMemberWire>(
    `/api/v1/guilds/${encodeURIComponent(guildSlug)}/members/${encodeURIComponent(memberUserId)}/roles`,
    {
      method: 'PATCH',
      body: JSON.stringify(toUpdateGuildMemberRolesInputWire(input)),
    },
  ).then(toGuildMember)
}

export function listBans(guildSlug: string): Promise<GuildBan[]> {
  return apiFetch<GuildBanWire[]>(
    `/api/v1/guilds/${encodeURIComponent(guildSlug)}/moderation/bans`,
  ).then((bans) => bans.map(toGuildBan))
}

export function unban(
  guildSlug: string,
  banId: string,
): Promise<UnbanGuildBanResult> {
  return apiFetch<UnbanGuildBanResultWire>(
    `/api/v1/guilds/${encodeURIComponent(guildSlug)}/moderation/bans/${encodeURIComponent(banId)}`,
    {
      method: 'DELETE',
    },
  ).then(toUnbanGuildBanResult)
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
