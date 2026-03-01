<script lang="ts">
// biome-ignore-all lint/correctness/noUnusedVariables: Svelte template usage isn't detected reliably.
import { ApiError } from '$lib/api'

import { guildState } from '$lib/features/guild/guildStore.svelte'
import {
  ALL_ROLE_PERMISSIONS_BITFLAG,
  GUILD_PERMISSION_CATALOG,
} from '$lib/features/guild/permissions'
import type {
  GuildMember,
  GuildRole,
  PresenceStatus,
} from '$lib/features/guild/types'
import { blockState } from '$lib/features/identity/blockStore.svelte'
import { identityState } from '$lib/features/identity/identityStore.svelte'
import { presenceState } from './presenceStore.svelte'

type Props = {
  activeGuild: string
}

type ModerationPermission =
  | 'MUTE_MEMBERS'
  | 'KICK_MEMBERS'
  | 'BAN_MEMBERS'
  | 'MANAGE_MESSAGES'

type MemberWithPresence = GuildMember & {
  presenceStatus: PresenceStatus
}

type MemberGroup = {
  id: string
  name: string
  color: string
  position: number
  members: MemberWithPresence[]
  onlineCount: number
  idleCount: number
  offlineCount: number
}

type VirtualRow = {
  id: string
  kind: 'group' | 'member'
  top: number
  height: number
  group: MemberGroup
  member?: MemberWithPresence
}

const GROUP_ROW_HEIGHT = 30
const MEMBER_ROW_HEIGHT = 56
const VIRTUAL_OVERSCAN_PX = 240
const OWNER_ROLE_COLOR = '#f59e0b'

const MODERATION_ACTIONS: Array<{
  permission: ModerationPermission
  label: string
}> = [
  { permission: 'MUTE_MEMBERS', label: 'Mute member' },
  { permission: 'KICK_MEMBERS', label: 'Kick member' },
  { permission: 'BAN_MEMBERS', label: 'Ban member' },
  { permission: 'MANAGE_MESSAGES', label: 'Moderate messages' },
]

let { activeGuild }: Props = $props()

let loading = $state(false)
let errorMessage = $state<string | null>(null)
let statusMessage = $state<string | null>(null)
let selectedMemberId = $state<string | null>(null)
let assignPanelMemberId = $state<string | null>(null)
let pendingMemberId = $state<string | null>(null)
let pendingBlockUserId = $state<string | null>(null)
let roleOverridesByMember = $state<Record<string, string[]>>({})
let scrollTop = $state(0)
let viewportHeight = $state(240)
let listViewport = $state<HTMLDivElement | null>(null)

let memberRoleData = $derived(guildState.memberRoleDataForGuild(activeGuild))
let members = $derived(memberRoleData.members)
let roles = $derived(memberRoleData.roles)
let assignableRoleIdSet = $derived(new Set(memberRoleData.assignableRoleIds))
let assignableRoles = $derived(
  roles.filter((role) => assignableRoleIdSet.has(role.id)),
)

let membersWithPresence = $derived.by(() => {
  const _presenceVersion = presenceState.version
  const _blockVersion = blockState.version
  void _presenceVersion
  void _blockVersion
  return members
    .map((member) => ({
      ...member,
      presenceStatus: presenceState.statusFor(
        member.userId,
        member.presenceStatus,
      ),
    }))
    .filter((member) => !blockState.isBlocked(member.userId))
})

let currentUserId = $derived(identityState.session?.user.id ?? null)
let currentViewer = $derived(
  currentUserId
    ? (membersWithPresence.find((member) => member.userId === currentUserId) ??
        null)
    : null,
)
let viewerPermissionBitflag = $derived(
  currentViewer ? effectivePermissionBitflag(currentViewer) : 0,
)
let moderationActions = $derived(
  MODERATION_ACTIONS.filter((action) => viewerHasPermission(action.permission)),
)

let groupedMembers = $derived.by(() => {
  const grouped = new Map<string, MemberGroup>()

  for (const member of membersWithPresence) {
    const groupMeta = groupForMember(member)
    const existing = grouped.get(groupMeta.id)
    if (existing) {
      existing.members.push(member)
      incrementPresenceCount(existing, member.presenceStatus)
      continue
    }

    const created: MemberGroup = {
      ...groupMeta,
      members: [member],
      onlineCount: 0,
      idleCount: 0,
      offlineCount: 0,
    }
    incrementPresenceCount(created, member.presenceStatus)
    grouped.set(groupMeta.id, created)
  }

  const orderedGroups = [...grouped.values()].sort(
    (left, right) =>
      left.position - right.position || left.name.localeCompare(right.name),
  )

  for (const group of orderedGroups) {
    group.members.sort(compareMembers)
  }

  return orderedGroups
})

let virtualRows = $derived.by(() => {
  let top = 0
  const rows: VirtualRow[] = []

  for (const group of groupedMembers) {
    rows.push({
      id: `group-${group.id}`,
      kind: 'group',
      top,
      height: GROUP_ROW_HEIGHT,
      group,
    })
    top += GROUP_ROW_HEIGHT

    for (const member of group.members) {
      rows.push({
        id: `member-${member.userId}`,
        kind: 'member',
        top,
        height: MEMBER_ROW_HEIGHT,
        group,
        member,
      })
      top += MEMBER_ROW_HEIGHT
    }
  }

  return {
    rows,
    totalHeight: top,
  }
})

let visibleRows = $derived.by(() => {
  const start = Math.max(0, scrollTop - VIRTUAL_OVERSCAN_PX)
  const end = scrollTop + viewportHeight + VIRTUAL_OVERSCAN_PX
  return virtualRows.rows.filter(
    (row) => row.top + row.height >= start && row.top <= end,
  )
})

let selectedMember = $derived(
  selectedMemberId
    ? (membersWithPresence.find(
        (member) => member.userId === selectedMemberId,
      ) ?? null)
    : null,
)
let selectedMemberBlocked = $derived(
  selectedMember ? blockState.isBlocked(selectedMember.userId) : false,
)

function messageFromError(err: unknown, fallback: string): string {
  if (err instanceof ApiError) return err.message
  if (err instanceof Error) return err.message
  return fallback
}

function normalizeRoleIds(roleIds: string[]): string[] {
  const seen = new Set<string>()
  const normalized: string[] = []
  for (const roleId of roleIds) {
    const trimmed = roleId.trim()
    if (!trimmed || seen.has(trimmed)) continue
    seen.add(trimmed)
    normalized.push(trimmed)
  }
  return normalized
}

function roleIdsForMember(member: GuildMember): string[] {
  return roleOverridesByMember[member.userId] ?? member.roleIds
}

function roleName(roleId: string): string {
  const role = roles.find((item) => item.id === roleId)
  return role?.name ?? roleId
}

function roleSummary(member: GuildMember): string {
  const roleNames = roleIdsForMember(member).map(roleName)
  return roleNames.length > 0 ? roleNames.join(', ') : '@everyone'
}

function statusLabel(status: PresenceStatus): string {
  if (status === 'online') return 'Online'
  if (status === 'idle') return 'Idle'
  return 'Offline'
}

function statusDotClass(status: PresenceStatus): string {
  if (status === 'online') return 'bg-emerald-500'
  if (status === 'idle') return 'bg-amber-400'
  return 'bg-muted-foreground'
}

function statusRank(status: PresenceStatus): number {
  if (status === 'online') return 0
  if (status === 'idle') return 1
  return 2
}

function compareMembers(
  left: MemberWithPresence,
  right: MemberWithPresence,
): number {
  const byStatus =
    statusRank(left.presenceStatus) - statusRank(right.presenceStatus)
  if (byStatus !== 0) return byStatus

  const byUsername = left.username.localeCompare(right.username, undefined, {
    sensitivity: 'base',
  })
  if (byUsername !== 0) return byUsername

  return left.userId.localeCompare(right.userId)
}

function incrementPresenceCount(
  group: MemberGroup,
  status: PresenceStatus,
): void {
  if (status === 'online') {
    group.onlineCount += 1
    return
  }
  if (status === 'idle') {
    group.idleCount += 1
    return
  }
  group.offlineCount += 1
}

function groupForMember(member: GuildMember): {
  id: string
  name: string
  color: string
  position: number
} {
  if (member.isOwner) {
    return {
      id: `owner-${member.userId}`,
      name: 'Owner',
      color: OWNER_ROLE_COLOR,
      position: -1,
    }
  }

  const memberRoleIds = roleIdsForMember(member)
  for (const roleId of memberRoleIds) {
    const role = roles.find((candidate) => candidate.id === roleId)
    if (!role || role.isDefault) continue
    return {
      id: role.id,
      name: role.name,
      color: role.color,
      position: role.position,
    }
  }

  const defaultRole =
    roles.find((role) => role.isDefault) ??
    roles.find((role) => role.name === '@everyone')

  if (defaultRole) {
    return {
      id: defaultRole.id,
      name: defaultRole.name,
      color: defaultRole.color,
      position: defaultRole.position,
    }
  }

  return {
    id: 'everyone-fallback',
    name: '@everyone',
    color: '#99aab5',
    position: 2_147_483_647,
  }
}

function effectivePermissionBitflag(member: GuildMember): number {
  if (member.isOwner) return ALL_ROLE_PERMISSIONS_BITFLAG

  const defaultRole =
    roles.find((role) => role.isDefault) ??
    roles.find((role) => role.name === '@everyone')
  let effective = defaultRole?.permissionsBitflag ?? 0

  for (const roleId of member.roleIds) {
    const role = roles.find((candidate) => candidate.id === roleId)
    if (!role) continue
    effective |= role.permissionsBitflag
  }

  return effective
}

function permissionMask(permission: ModerationPermission): number {
  return (
    GUILD_PERMISSION_CATALOG.find((entry) => entry.key === permission)?.mask ??
    0
  )
}

function viewerHasPermission(permission: ModerationPermission): boolean {
  if (currentViewer?.isOwner) return true
  const mask = permissionMask(permission)
  return mask !== 0 && (viewerPermissionBitflag & mask) === mask
}

function canAssignRoles(member: GuildMember): boolean {
  return (
    memberRoleData.canManageRoles &&
    member.canAssignRoles &&
    assignableRoles.length > 0
  )
}

function openMemberPopover(member: GuildMember): void {
  selectedMemberId = member.userId
  assignPanelMemberId = null
  statusMessage = null
}

function closeMemberPopover(): void {
  selectedMemberId = null
  assignPanelMemberId = null
}

function toggleAssignPanel(member: GuildMember): void {
  if (!canAssignRoles(member)) return
  assignPanelMemberId =
    assignPanelMemberId === member.userId ? null : member.userId
}

function handleMemberContextMenu(event: MouseEvent, member: GuildMember): void {
  event.preventDefault()
  openMemberPopover(member)
}

function handleMemberKeydown(event: KeyboardEvent, member: GuildMember): void {
  if (
    event.key === 'Enter' ||
    event.key === 'ContextMenu' ||
    (event.shiftKey && event.key === 'F10')
  ) {
    event.preventDefault()
    openMemberPopover(member)
  }
}

function handleListScroll(event: Event): void {
  const target = event.currentTarget as HTMLDivElement | null
  if (!target) return
  scrollTop = target.scrollTop
  viewportHeight = target.clientHeight || 240
}

function nextRoleIds(
  currentRoleIds: string[],
  roleId: string,
  enabled: boolean,
): string[] {
  const mutableRoleIds = currentRoleIds.filter((id) =>
    assignableRoleIdSet.has(id),
  )
  const immutableRoleIds = currentRoleIds.filter(
    (id) => !assignableRoleIdSet.has(id),
  )
  const nextMutableRoleIds = enabled
    ? [...mutableRoleIds, roleId]
    : mutableRoleIds.filter((id) => id !== roleId)
  return normalizeRoleIds([...immutableRoleIds, ...nextMutableRoleIds])
}

async function applyRoleToggle(
  member: GuildMember,
  roleId: string,
  enabled: boolean,
): Promise<void> {
  if (pendingMemberId || !canAssignRoles(member)) return
  const previousRoleIds = roleIdsForMember(member)
  const updatedRoleIds = nextRoleIds(previousRoleIds, roleId, enabled)
  if (
    previousRoleIds.length === updatedRoleIds.length &&
    previousRoleIds.every((role, index) => role === updatedRoleIds[index])
  ) {
    return
  }

  pendingMemberId = member.userId
  errorMessage = null
  roleOverridesByMember = {
    ...roleOverridesByMember,
    [member.userId]: updatedRoleIds,
  }

  try {
    await guildState.updateMemberRoles(activeGuild, member.userId, {
      roleIds: updatedRoleIds,
    })
    const nextOverrides = { ...roleOverridesByMember }
    delete nextOverrides[member.userId]
    roleOverridesByMember = nextOverrides
    statusMessage = `Roles updated for ${member.displayName}.`
  } catch (err) {
    const nextOverrides = { ...roleOverridesByMember }
    delete nextOverrides[member.userId]
    roleOverridesByMember = nextOverrides
    errorMessage = messageFromError(err, 'Failed to update member roles.')
  } finally {
    pendingMemberId = null
  }
}

async function handleRoleToggle(
  member: GuildMember,
  role: GuildRole,
  event: Event,
): Promise<void> {
  const input = event.currentTarget as HTMLInputElement | null
  if (!input) return
  await applyRoleToggle(member, role.id, input.checked)
}

function triggerSendDm(member: GuildMember): void {
  if (typeof window !== 'undefined') {
    window.dispatchEvent(
      new CustomEvent('discool:open-dm-intent', {
        detail: {
          guildSlug: activeGuild,
          userId: member.userId,
        },
      }),
    )
  }
  statusMessage = `DM intent opened for ${member.displayName}.`
}

function triggerModerationAction(label: string, member: GuildMember): void {
  statusMessage = `${label} for ${member.displayName} will be available in Epic 8.`
}

async function toggleBlockForMember(member: GuildMember): Promise<void> {
  if (pendingBlockUserId || !currentUserId || member.userId === currentUserId) {
    return
  }

  const currentlyBlocked = blockState.isBlocked(member.userId)
  if (!currentlyBlocked && typeof window !== 'undefined') {
    const confirmed = window.confirm(
      `Block ${member.displayName}? Their messages, reactions, typing, and member presence will be erased from your view.`,
    )
    if (!confirmed) return
  }

  pendingBlockUserId = member.userId
  errorMessage = null
  statusMessage = null
  try {
    if (currentlyBlocked) {
      const result = await blockState.unblockUser(member.userId)
      statusMessage = result.synced
        ? `Unblocked ${member.displayName}.`
        : `Unblocked ${member.displayName}. Local change saved, but sync failed: ${result.syncError}`
      return
    }

    const result = await blockState.blockUser(member.userId, {
      displayName: member.displayName,
      username: member.username,
      avatarColor: member.avatarColor ?? null,
    })
    statusMessage = result.synced
      ? `Blocked ${member.displayName}.`
      : `Blocked ${member.displayName}. Local change saved, but sync failed: ${result.syncError}`
  } catch (err) {
    errorMessage = messageFromError(err, 'Failed to update blocked-user state.')
  } finally {
    pendingBlockUserId = null
  }
}

$effect(() => {
  if (!activeGuild) return
  loading = true
  errorMessage = null
  statusMessage = null
  selectedMemberId = null
  assignPanelMemberId = null
  roleOverridesByMember = {}
  scrollTop = 0
  const guildSlug = activeGuild

  void guildState
    .loadMembers(guildSlug, true)
    .catch((err: unknown) => {
      if (activeGuild !== guildSlug) return
      errorMessage = messageFromError(err, 'Failed to load members.')
    })
    .finally(() => {
      if (activeGuild !== guildSlug) return
      loading = false
    })
})

$effect(() => {
  presenceState.seedFromMembers(members)
})

$effect(() => {
  const token = identityState.session?.token ?? null
  presenceState.ensureConnected(token)
})

$effect(() => {
  viewportHeight = listViewport?.clientHeight || 240
})
</script>

<aside class="flex h-full min-h-0 flex-col bg-card p-4" data-testid="member-list" aria-label="Member list">
  <h2 class="mb-3 text-sm font-semibold text-foreground">
    Members
    <span class="text-xs text-muted-foreground" aria-hidden="true">
      ({membersWithPresence.length})
    </span>
  </h2>

  {#if statusMessage}
    <p class="mb-3 rounded-md border border-emerald-500/30 bg-emerald-500/10 p-2 text-xs text-emerald-300">
      {statusMessage}
    </p>
  {/if}

  {#if errorMessage}
    <p
      class="mb-3 rounded-md border border-destructive/30 bg-destructive/10 p-2 text-xs text-destructive"
      role="alert"
    >
      {errorMessage}
    </p>
  {/if}

  {#if loading && membersWithPresence.length === 0}
    <p class="text-xs text-muted-foreground">Loading members…</p>
  {:else if membersWithPresence.length === 0}
    <p class="text-xs text-muted-foreground">No members found.</p>
  {:else}
    <div
      class="relative min-h-0 flex-1 overflow-y-auto"
      bind:this={listViewport}
      onscroll={handleListScroll}
      data-testid="member-list-scroll"
    >
      <div class="relative" style={`height: ${virtualRows.totalHeight}px;`}>
        {#each visibleRows as row (row.id)}
          {#if row.kind === 'group'}
            <div
              class="absolute inset-x-0 flex items-center justify-between px-1 text-[11px] font-semibold uppercase tracking-wide"
              style={`top: ${row.top}px; height: ${row.height}px;`}
              role="heading"
              aria-level="3"
              aria-label={`${row.group.name}: ${row.group.members.length} members, ${row.group.onlineCount} online, ${row.group.idleCount} idle, ${row.group.offlineCount} offline`}
              data-testid={`role-group-${row.group.id}`}
            >
              <span style={`color: ${row.group.color};`}>
                {row.group.name} ({row.group.members.length})
              </span>
              <span class="text-muted-foreground">
                {row.group.onlineCount} online
              </span>
            </div>
          {:else if row.member}
            <div
              class="absolute inset-x-0 px-1"
              style={`top: ${row.top}px; height: ${row.height}px;`}
            >
              <button
                type="button"
                class="flex w-full items-center gap-2 rounded-md px-2 py-2 text-left hover:bg-muted focus:outline-none focus:ring-2 focus:ring-ring"
                onclick={() => openMemberPopover(row.member!)}
                oncontextmenu={(event) => handleMemberContextMenu(event, row.member!)}
                onkeydown={(event) => handleMemberKeydown(event, row.member!)}
                aria-label={`Open member profile for ${row.member.displayName}, ${statusLabel(row.member.presenceStatus)}`}
                data-testid={`member-row-${row.member.userId}`}
              >
                <span class="relative inline-flex h-8 w-8 shrink-0 items-center justify-center rounded-full text-xs font-semibold text-black"
                  style={`background-color: ${row.member.avatarColor ?? row.member.highestRoleColor};`}
                >
                  {row.member.displayName.slice(0, 1).toUpperCase()}
                  <span
                    class={`absolute -bottom-0.5 -right-0.5 h-2.5 w-2.5 rounded-full border border-card ${statusDotClass(row.member.presenceStatus)}`}
                    data-testid={`member-status-dot-${row.member.userId}`}
                  ></span>
                </span>
                <span class="min-w-0 flex-1">
                  <span
                    class="block truncate font-medium"
                    style={`color: ${row.member.highestRoleColor}`}
                    data-testid={`member-display-name-${row.member.userId}`}
                  >
                    {row.member.displayName}
                  </span>
                  <span class="block truncate text-xs text-muted-foreground">
                    @{row.member.username} · {statusLabel(row.member.presenceStatus)}
                  </span>
                </span>
              </button>
            </div>
          {/if}
        {/each}
      </div>
    </div>

    {#if selectedMember}
      <div
        class="mt-3 rounded-md border border-border bg-muted/30 p-3 text-xs text-foreground"
        role="menu"
        aria-label={`Profile actions for ${selectedMember.displayName}`}
      >
        <div class="mb-3 flex items-center justify-between gap-2">
          <div class="flex min-w-0 items-center gap-2">
            <span
              class="inline-flex h-9 w-9 items-center justify-center rounded-full text-sm font-semibold text-black"
              style={`background-color: ${selectedMember.avatarColor ?? selectedMember.highestRoleColor};`}
            >
              {selectedMember.displayName.slice(0, 1).toUpperCase()}
            </span>
            <div class="min-w-0">
              <p class="truncate text-sm font-semibold">{selectedMember.displayName}</p>
              <p class="truncate text-muted-foreground">@{selectedMember.username}</p>
            </div>
          </div>
          <button
            type="button"
            class="rounded-md bg-muted px-2 py-1 text-xs hover:opacity-90"
            onclick={closeMemberPopover}
          >
            Close
          </button>
        </div>

        <p class="mb-2 text-muted-foreground">
          Roles: {roleSummary(selectedMember)}
        </p>

        <div class="mb-3 flex flex-wrap gap-1">
          {#each roleIdsForMember(selectedMember) as roleId (roleId)}
            <span class="rounded-full bg-card px-2 py-0.5 text-[11px]">
              {roleName(roleId)}
            </span>
          {/each}
          {#if roleIdsForMember(selectedMember).length === 0}
            <span class="rounded-full bg-card px-2 py-0.5 text-[11px]">@everyone</span>
          {/if}
        </div>

        <div class="mb-3 flex flex-wrap gap-2">
          <button
            type="button"
            class="rounded-md bg-muted px-2 py-1 text-xs hover:opacity-90"
            onclick={() => triggerSendDm(selectedMember)}
          >
            Send DM
          </button>

          {#if selectedMember.userId !== currentUserId}
            <button
              type="button"
              class="rounded-md border border-border px-2 py-1 text-xs hover:bg-muted disabled:cursor-not-allowed disabled:opacity-60"
              onclick={() => void toggleBlockForMember(selectedMember)}
              disabled={pendingBlockUserId === selectedMember.userId}
            >
              {#if pendingBlockUserId === selectedMember.userId}
                {selectedMemberBlocked ? 'Unblocking...' : 'Blocking...'}
              {:else}
                {selectedMemberBlocked ? 'Unblock user' : 'Block user'}
              {/if}
            </button>
          {/if}

          {#if moderationActions.length === 0}
            <span class="rounded-md border border-border px-2 py-1 text-[11px] text-muted-foreground">
              Moderation actions unavailable
            </span>
          {:else}
            {#each moderationActions as action (action.permission)}
              <button
                type="button"
                class="rounded-md border border-border px-2 py-1 text-xs hover:bg-muted"
                onclick={() => triggerModerationAction(action.label, selectedMember)}
              >
                {action.label} (coming soon)
              </button>
            {/each}
          {/if}
        </div>

        {#if canAssignRoles(selectedMember)}
          <button
            type="button"
            class="mb-2 rounded-md bg-muted px-2 py-1 text-xs hover:opacity-90"
            onclick={() => toggleAssignPanel(selectedMember)}
            aria-expanded={assignPanelMemberId === selectedMember.userId}
          >
            Assign role
          </button>

          {#if assignPanelMemberId === selectedMember.userId}
            <fieldset class="space-y-1" data-testid={`assign-role-panel-${selectedMember.userId}`}>
              <legend class="mb-1 text-xs text-muted-foreground">
                Toggle member roles
              </legend>
              {#each assignableRoles as role (role.id)}
                <label class="flex items-center gap-2 rounded px-1 py-1 hover:bg-muted">
                  <input
                    type="checkbox"
                    checked={roleIdsForMember(selectedMember).includes(role.id)}
                    disabled={pendingMemberId === selectedMember.userId}
                    onchange={(event) =>
                      void handleRoleToggle(selectedMember, role, event)}
                    aria-label={`Toggle ${role.name} for ${selectedMember.displayName}`}
                  />
                  <span>{role.name}</span>
                </label>
              {/each}
            </fieldset>
          {/if}
        {/if}
      </div>
    {/if}
  {/if}
</aside>
