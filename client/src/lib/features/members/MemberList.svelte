<script lang="ts">
// biome-ignore-all lint/correctness/noUnusedVariables: Svelte template usage isn't detected reliably.
import { ApiError } from '$lib/api'

import { guildState } from '$lib/features/guild/guildStore.svelte'
import type { GuildMember, GuildRole } from '$lib/features/guild/types'

type Props = {
  activeGuild: string
}

let { activeGuild }: Props = $props()

let loading = $state(false)
let errorMessage = $state<string | null>(null)
let statusMessage = $state<string | null>(null)
let selectedMemberId = $state<string | null>(null)
let assignPanelMemberId = $state<string | null>(null)
let pendingMemberId = $state<string | null>(null)
let roleOverridesByMember = $state<Record<string, string[]>>({})

let memberRoleData = $derived(guildState.memberRoleDataForGuild(activeGuild))
let members = $derived(memberRoleData.members)
let roles = $derived(memberRoleData.roles)
let assignableRoleIdSet = $derived(new Set(memberRoleData.assignableRoleIds))
let assignableRoles = $derived(
  roles.filter((role) => assignableRoleIdSet.has(role.id)),
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

$effect(() => {
  if (!activeGuild) return
  loading = true
  errorMessage = null
  statusMessage = null
  selectedMemberId = null
  assignPanelMemberId = null
  roleOverridesByMember = {}
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
</script>

<aside class="h-full bg-card p-4" data-testid="member-list" aria-label="Member list">
  <h2 class="mb-3 text-sm font-semibold text-foreground">Members</h2>

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

  {#if loading && members.length === 0}
    <p class="text-xs text-muted-foreground">Loading members…</p>
  {:else if members.length === 0}
    <p class="text-xs text-muted-foreground">No members found.</p>
  {:else}
    <ul class="space-y-2 text-sm">
      {#each members as member (member.userId)}
        {@const memberRoleIds = roleIdsForMember(member)}
        <li class="rounded-md border border-border bg-muted/30 p-2">
          <button
            type="button"
            class="w-full rounded-md px-2 py-1 text-left hover:bg-muted focus:outline-none focus:ring-2 focus:ring-ring"
            onclick={() => openMemberPopover(member)}
            oncontextmenu={(event) => handleMemberContextMenu(event, member)}
            onkeydown={(event) => handleMemberKeydown(event, member)}
            aria-label={`Open member profile for ${member.displayName}`}
            data-testid={`member-row-${member.userId}`}
          >
            <span
              class="block font-medium"
              style={`color: ${member.highestRoleColor}`}
              data-testid={`member-display-name-${member.userId}`}
            >
              {member.displayName}
            </span>
            <span class="block text-xs text-muted-foreground">@{member.username}</span>
          </button>

          <p class="mt-1 px-2 text-xs text-muted-foreground">
            Roles: {roleSummary(member)}
          </p>

          {#if selectedMemberId === member.userId}
            <div
              class="mt-2 rounded-md border border-border bg-card p-2 text-xs text-foreground"
              role="menu"
              aria-label={`Profile actions for ${member.displayName}`}
            >
              <div class="mb-2 flex items-center justify-between gap-2">
                <strong>{member.displayName}</strong>
                <button
                  type="button"
                  class="rounded-md bg-muted px-2 py-1 text-xs hover:opacity-90"
                  onclick={closeMemberPopover}
                >
                  Close
                </button>
              </div>

              {#if canAssignRoles(member)}
                <button
                  type="button"
                  class="mb-2 rounded-md bg-muted px-2 py-1 text-xs hover:opacity-90"
                  onclick={() => toggleAssignPanel(member)}
                  aria-expanded={assignPanelMemberId === member.userId}
                >
                  Assign role
                </button>

                {#if assignPanelMemberId === member.userId}
                  <fieldset class="space-y-1" data-testid={`assign-role-panel-${member.userId}`}>
                    <legend class="mb-1 text-xs text-muted-foreground">
                      Toggle member roles
                    </legend>
                    {#each assignableRoles as role (role.id)}
                      <label class="flex items-center gap-2 rounded px-1 py-1 hover:bg-muted">
                        <input
                          type="checkbox"
                          checked={memberRoleIds.includes(role.id)}
                          disabled={pendingMemberId === member.userId}
                          onchange={(event) => void handleRoleToggle(member, role, event)}
                          aria-label={`Toggle ${role.name} for ${member.displayName}`}
                        />
                        <span>{role.name}</span>
                      </label>
                    {/each}
                  </fieldset>
                {/if}
              {/if}
            </div>
          {/if}
        </li>
      {/each}
    </ul>
  {/if}
</aside>
