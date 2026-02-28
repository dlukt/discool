<script lang="ts">
// biome-ignore-all lint/correctness/noUnusedVariables: Svelte template usage isn't detected reliably.
import { ApiError } from '$lib/api'

import { guildState } from './guildStore.svelte'
import {
  ALL_ROLE_PERMISSIONS_BITFLAG,
  GUILD_PERMISSION_CATALOG,
  type GuildPermissionDefinition,
  hasGuildPermission,
  toggleGuildPermission,
} from './permissions'
import type { GuildRole } from './types'

const MAX_ICON_BYTES = 2 * 1024 * 1024
const allowedIconTypes = new Set(['image/png', 'image/jpeg', 'image/webp'])

type Props = {
  open: boolean
  guildSlug: string
  onClose?: () => void | Promise<void>
}

let { open, guildSlug, onClose }: Props = $props()
let guild = $derived(guildState.bySlug(guildSlug))
let canEditGuild = $derived(Boolean(guild?.isOwner))
let roles = $derived(guildState.rolesForGuild(guildSlug))
let roleDragPreview = $state<GuildRole[] | null>(null)
let roleList = $derived(roleDragPreview ?? roles)

let initializedForSlug = $state<string | null>(null)
let rolesInitializedForSlug = $state<string | null>(null)
let name = $state('')
let description = $state('')
let selectedIcon = $state<File | null>(null)
let nameError = $state<string | null>(null)
let iconError = $state<string | null>(null)
let errorMessage = $state<string | null>(null)
let statusMessage = $state<string | null>(null)
let saving = $state(false)

let rolesErrorMessage = $state<string | null>(null)
let rolesStatusMessage = $state<string | null>(null)
let draggedRoleId = $state<string | null>(null)
let roleReorderSubmitting = $state(false)

let createRoleDialogOpen = $state(false)
let createRoleName = $state('')
let createRoleColor = $state('#3366ff')
let createRoleNameError = $state<string | null>(null)
let createRoleColorError = $state<string | null>(null)
let createRoleError = $state<string | null>(null)
let createRoleSubmitting = $state(false)

let editRoleDialogOpen = $state(false)
let editRoleTarget = $state<GuildRole | null>(null)
let editRoleName = $state('')
let editRoleColor = $state('#3366ff')
let editRoleNameError = $state<string | null>(null)
let editRoleColorError = $state<string | null>(null)
let editRoleError = $state<string | null>(null)
let editRoleSubmitting = $state(false)

let deleteRoleDialogOpen = $state(false)
let deleteRoleTarget = $state<GuildRole | null>(null)
let deleteRoleError = $state<string | null>(null)
let deleteRoleSubmitting = $state(false)

let permissionsDialogOpen = $state(false)
let permissionsRoleTarget = $state<GuildRole | null>(null)
let permissionsBitflag = $state(0)
let permissionsSavingKey = $state<string | null>(null)
let permissionsError = $state<string | null>(null)
const permissionCatalog = GUILD_PERMISSION_CATALOG

function validateName(value: string): string | null {
  const trimmed = value.trim()
  if (!trimmed) return 'Guild name is required.'
  if (trimmed.length > 64) return 'Guild name must be 64 characters or less.'
  return null
}

function validateRoleName(value: string): string | null {
  const trimmed = value.trim()
  if (!trimmed) return 'Role name is required.'
  if (trimmed.length > 64) return 'Role name must be 64 characters or less.'
  return null
}

function validateRoleColor(value: string): string | null {
  const normalized = value.trim()
  if (!/^#[0-9a-fA-F]{6}$/.test(normalized)) {
    return 'Role color must be a hex color like #3366ff.'
  }
  return null
}

function messageFromError(err: unknown, fallback: string): string {
  if (err instanceof ApiError) return err.message
  if (err instanceof Error) return err.message
  return fallback
}

function resetForm() {
  if (!guild) return
  name = guild.name
  description = guild.description ?? ''
  selectedIcon = null
  nameError = null
  iconError = null
  errorMessage = null
  statusMessage = null
}

function resetRoleDialogs() {
  roleDragPreview = null
  draggedRoleId = null
  roleReorderSubmitting = false

  createRoleDialogOpen = false
  createRoleName = ''
  createRoleColor = '#3366ff'
  createRoleNameError = null
  createRoleColorError = null
  createRoleError = null

  editRoleDialogOpen = false
  editRoleTarget = null
  editRoleName = ''
  editRoleColor = '#3366ff'
  editRoleNameError = null
  editRoleColorError = null
  editRoleError = null

  deleteRoleDialogOpen = false
  deleteRoleTarget = null
  deleteRoleError = null

  permissionsDialogOpen = false
  permissionsRoleTarget = null
  permissionsBitflag = 0
  permissionsSavingKey = null
  permissionsError = null
}

$effect(() => {
  if (!open || !guild) return
  if (initializedForSlug === guild.slug) return
  initializedForSlug = guild.slug
  resetForm()
})

$effect(() => {
  if (!open || !guild || !canEditGuild) return
  if (rolesInitializedForSlug === guild.slug) return
  rolesInitializedForSlug = guild.slug
  rolesErrorMessage = null
  rolesStatusMessage = null
  void guildState.loadRoles(guild.slug).catch((err: unknown) => {
    rolesErrorMessage = messageFromError(err, 'Failed to load roles.')
  })
})

$effect(() => {
  if (open) return
  initializedForSlug = null
  rolesInitializedForSlug = null
  resetRoleDialogs()
})

function onNameBlur() {
  nameError = validateName(name)
}

function onIconChange(event: Event) {
  iconError = null
  const input = event.currentTarget as HTMLInputElement | null
  const file = input?.files?.[0]
  if (!file) {
    selectedIcon = null
    return
  }

  if (!allowedIconTypes.has(file.type)) {
    selectedIcon = null
    iconError = 'Only PNG, JPEG, and WEBP images are supported.'
    return
  }
  if (file.size > MAX_ICON_BYTES) {
    selectedIcon = null
    iconError = 'Guild icon image must be 2 MB or smaller.'
    return
  }

  selectedIcon = file
}

async function handleSubmit(event: SubmitEvent) {
  event.preventDefault()
  if (saving || !guild || !canEditGuild) return

  errorMessage = null
  statusMessage = null
  nameError = validateName(name)
  if (nameError || iconError) return

  saving = true
  try {
    await guildState.updateGuild(
      guild.slug,
      {
        name: name.trim(),
        description: description.trim() ? description.trim() : null,
      },
      selectedIcon,
    )
    selectedIcon = null
    statusMessage = 'Guild settings saved.'
  } catch (err) {
    errorMessage = messageFromError(err, 'Failed to save guild settings.')
  } finally {
    saving = false
  }
}

function openCreateRoleDialog() {
  if (!canEditGuild) return
  createRoleDialogOpen = true
  createRoleName = ''
  createRoleColor = '#3366ff'
  createRoleNameError = null
  createRoleColorError = null
  createRoleError = null
}

function closeCreateRoleDialog() {
  createRoleDialogOpen = false
  createRoleName = ''
  createRoleColor = '#3366ff'
  createRoleNameError = null
  createRoleColorError = null
  createRoleError = null
}

function onCreateRoleNameBlur() {
  createRoleNameError = validateRoleName(createRoleName)
}

function onCreateRoleColorBlur() {
  createRoleColorError = validateRoleColor(createRoleColor)
}

async function handleCreateRoleSubmit(event: SubmitEvent) {
  event.preventDefault()
  if (createRoleSubmitting || !guild || !canEditGuild) return

  createRoleError = null
  createRoleNameError = validateRoleName(createRoleName)
  createRoleColorError = validateRoleColor(createRoleColor)
  if (createRoleNameError || createRoleColorError) return

  createRoleSubmitting = true
  try {
    await guildState.createRole(guild.slug, {
      name: createRoleName.trim(),
      color: createRoleColor.trim().toLowerCase(),
    })
    closeCreateRoleDialog()
    rolesStatusMessage = 'Role created.'
  } catch (err) {
    createRoleError = messageFromError(err, 'Failed to create role.')
  } finally {
    createRoleSubmitting = false
  }
}

function openEditRoleDialog(role: GuildRole) {
  if (!canEditGuild || !role.canEdit) return
  editRoleTarget = role
  editRoleName = role.name
  editRoleColor = role.color
  editRoleNameError = null
  editRoleColorError = null
  editRoleError = null
  editRoleDialogOpen = true
}

function closeEditRoleDialog() {
  editRoleDialogOpen = false
  editRoleTarget = null
  editRoleName = ''
  editRoleColor = '#3366ff'
  editRoleNameError = null
  editRoleColorError = null
  editRoleError = null
}

function onEditRoleNameBlur() {
  editRoleNameError = validateRoleName(editRoleName)
}

function onEditRoleColorBlur() {
  editRoleColorError = validateRoleColor(editRoleColor)
}

async function handleEditRoleSubmit(event: SubmitEvent) {
  event.preventDefault()
  if (editRoleSubmitting || !guild || !editRoleTarget || !canEditGuild) return

  editRoleError = null
  editRoleNameError = validateRoleName(editRoleName)
  editRoleColorError = validateRoleColor(editRoleColor)
  if (editRoleNameError || editRoleColorError) return

  editRoleSubmitting = true
  try {
    await guildState.updateRole(guild.slug, editRoleTarget.id, {
      name: editRoleName.trim(),
      color: editRoleColor.trim().toLowerCase(),
    })
    closeEditRoleDialog()
    rolesStatusMessage = 'Role saved.'
  } catch (err) {
    editRoleError = messageFromError(err, 'Failed to save role.')
  } finally {
    editRoleSubmitting = false
  }
}

function isOwnerPseudoRole(role: GuildRole): boolean {
  return role.id.startsWith('owner:')
}

function openPermissionsDialog(role: GuildRole) {
  if (!canEditGuild) return
  permissionsRoleTarget = role
  permissionsBitflag = isOwnerPseudoRole(role)
    ? ALL_ROLE_PERMISSIONS_BITFLAG
    : role.permissionsBitflag
  permissionsSavingKey = null
  permissionsError = null
  permissionsDialogOpen = true
}

function closePermissionsDialog() {
  permissionsDialogOpen = false
  permissionsRoleTarget = null
  permissionsBitflag = 0
  permissionsSavingKey = null
  permissionsError = null
}

async function handlePermissionToggle(
  permission: GuildPermissionDefinition,
  event: Event,
) {
  if (!guild || !permissionsRoleTarget) return
  if (permissionsSavingKey || !permissionsRoleTarget.canEdit) return

  const input = event.currentTarget as HTMLInputElement | null
  if (!input) return

  const previousBitflag = permissionsBitflag
  const nextBitflag = toggleGuildPermission(
    previousBitflag,
    permission.mask,
    input.checked,
  )
  if (nextBitflag === previousBitflag) return

  permissionsBitflag = nextBitflag
  permissionsSavingKey = permission.key
  permissionsError = null

  try {
    const updated = await guildState.updateRole(
      guild.slug,
      permissionsRoleTarget.id,
      {
        permissionsBitflag: nextBitflag,
      },
    )
    permissionsRoleTarget = updated
    permissionsBitflag = updated.permissionsBitflag
    rolesStatusMessage = `Permissions saved for ${updated.name}.`
  } catch (err) {
    permissionsBitflag = previousBitflag
    permissionsError = messageFromError(err, 'Failed to save role permissions.')
  } finally {
    permissionsSavingKey = null
  }
}

function permissionIsEnabled(permission: GuildPermissionDefinition): boolean {
  return hasGuildPermission(permissionsBitflag, permission)
}

function openDeleteRoleDialog(role: GuildRole) {
  if (!canEditGuild || !role.canDelete) return
  deleteRoleTarget = role
  deleteRoleError = null
  deleteRoleDialogOpen = true
}

function closeDeleteRoleDialog() {
  deleteRoleDialogOpen = false
  deleteRoleTarget = null
  deleteRoleError = null
}

async function confirmDeleteRole() {
  if (deleteRoleSubmitting || !guild || !deleteRoleTarget || !canEditGuild)
    return

  deleteRoleSubmitting = true
  deleteRoleError = null
  try {
    await guildState.deleteRole(guild.slug, deleteRoleTarget.id)
    closeDeleteRoleDialog()
    rolesStatusMessage = 'Role deleted.'
  } catch (err) {
    deleteRoleError = messageFromError(err, 'Failed to delete role.')
  } finally {
    deleteRoleSubmitting = false
  }
}

function isFixedRole(role: GuildRole): boolean {
  return !role.canEdit || !role.canDelete
}

function isRoleReorderable(role: GuildRole): boolean {
  return canEditGuild && !isFixedRole(role) && !roleReorderSubmitting
}

function resolveDraggedRoleId(event: DragEvent): string | null {
  return draggedRoleId ?? event.dataTransfer?.getData('text/plain') ?? null
}

function reorderCustomRoles(
  currentRoles: GuildRole[],
  sourceRoleId: string,
  targetRoleId: string,
): GuildRole[] | null {
  const customRoles = currentRoles.filter((role) => !isFixedRole(role))
  const sourceIndex = customRoles.findIndex((role) => role.id === sourceRoleId)
  const targetIndex = customRoles.findIndex((role) => role.id === targetRoleId)
  if (sourceIndex < 0 || targetIndex < 0 || sourceIndex === targetIndex) {
    return null
  }

  const reorderedCustom = [...customRoles]
  const [moved] = reorderedCustom.splice(sourceIndex, 1)
  reorderedCustom.splice(targetIndex, 0, moved)

  let customIndex = 0
  return currentRoles.map((role) => {
    if (isFixedRole(role)) return role
    const next = reorderedCustom[customIndex]
    customIndex += 1
    return next
  })
}

async function persistRoleReorder(nextRoles: GuildRole[]) {
  if (!guild || roleReorderSubmitting) return
  roleReorderSubmitting = true
  rolesStatusMessage = null
  rolesErrorMessage = null
  roleDragPreview = nextRoles

  const roleIds = nextRoles
    .filter((role) => !isFixedRole(role))
    .map((role) => role.id)

  try {
    await guildState.reorderRoles(guild.slug, roleIds)
    roleDragPreview = null
    rolesStatusMessage = 'Role order updated.'
  } catch (err) {
    roleDragPreview = null
    rolesErrorMessage = messageFromError(err, 'Failed to reorder roles.')
  } finally {
    roleReorderSubmitting = false
  }
}

function handleRoleDragStart(event: DragEvent, role: GuildRole) {
  if (!isRoleReorderable(role)) return
  draggedRoleId = role.id
  event.dataTransfer?.setData('text/plain', role.id)
  if (event.dataTransfer) {
    event.dataTransfer.effectAllowed = 'move'
  }
}

function handleRoleDragOver(event: DragEvent, targetRole: GuildRole) {
  if (!isRoleReorderable(targetRole)) return
  const sourceRoleId = resolveDraggedRoleId(event)
  if (!sourceRoleId || sourceRoleId === targetRole.id) return
  event.preventDefault()
  if (event.dataTransfer) {
    event.dataTransfer.dropEffect = 'move'
  }
}

async function handleRoleDrop(event: DragEvent, targetRole: GuildRole) {
  if (!isRoleReorderable(targetRole)) return
  event.preventDefault()
  const sourceRoleId = resolveDraggedRoleId(event)
  draggedRoleId = null
  if (!sourceRoleId || sourceRoleId === targetRole.id) return

  const nextRoles = reorderCustomRoles(roleList, sourceRoleId, targetRole.id)
  if (!nextRoles) return
  await persistRoleReorder(nextRoles)
}

function handleRoleDragEnd() {
  draggedRoleId = null
}

async function handleClose() {
  await onClose?.()
}
</script>

{#if open}
  <div
    class="fixed inset-0 z-50 flex items-center justify-center bg-black/50 p-4"
    role="presentation"
  >
    <div
      class="max-h-[90vh] w-full max-w-lg overflow-y-auto rounded-lg border border-border bg-card p-6 shadow-2xl"
      role="dialog"
      aria-modal="true"
      aria-label="Guild settings"
    >
      <header class="mb-4 flex items-center justify-between">
        <h2 class="text-lg font-semibold">Guild settings</h2>
        <button
          type="button"
          class="rounded-md bg-muted px-3 py-1 text-sm text-foreground hover:opacity-90"
          onclick={() => void handleClose()}
          aria-label="Close guild settings"
        >
          Close
        </button>
      </header>

      {#if !guild}
        <p class="rounded-md border border-destructive/30 bg-destructive/10 p-3 text-sm text-destructive">
          Guild not found.
        </p>
      {:else if !canEditGuild}
        <p class="rounded-md border border-destructive/30 bg-destructive/10 p-3 text-sm text-destructive">
          Only guild owners can edit guild settings.
        </p>
      {:else}
        {#if statusMessage}
          <p class="mb-4 rounded-md border border-emerald-500/30 bg-emerald-500/10 p-3 text-sm text-emerald-300">
            {statusMessage}
          </p>
        {/if}
        {#if errorMessage}
          <p class="mb-4 rounded-md border border-destructive/30 bg-destructive/10 p-3 text-sm text-destructive" role="alert">
            {errorMessage}
          </p>
        {/if}

        <form class="space-y-4" onsubmit={handleSubmit} novalidate>
          <div class="space-y-1">
            <label for="guild-settings-name" class="text-sm font-medium">Guild name</label>
            <input
              id="guild-settings-name"
              type="text"
              class={`w-full rounded-md border bg-background px-3 py-2 text-base focus:outline-none focus:ring-2 ${
                nameError
                  ? 'border-destructive focus:ring-destructive'
                  : 'border-input focus:ring-ring'
              }`}
              bind:value={name}
              onblur={onNameBlur}
              maxlength={64}
              required
            />
            {#if nameError}
              <p class="text-sm text-destructive">{nameError}</p>
            {/if}
          </div>

          <div class="space-y-1">
            <label for="guild-settings-description" class="text-sm font-medium">Description</label>
            <textarea
              id="guild-settings-description"
              class="h-24 w-full rounded-md border border-input bg-background px-3 py-2 text-base focus:outline-none focus:ring-2 focus:ring-ring"
              bind:value={description}
              maxlength={512}
            ></textarea>
          </div>

          <div class="space-y-1">
            <label for="guild-settings-icon" class="text-sm font-medium">Guild icon (optional)</label>
            <input
              id="guild-settings-icon"
              type="file"
              accept="image/png,image/jpeg,image/webp"
              class="block w-full text-sm text-muted-foreground file:mr-4 file:rounded-md file:border-0 file:bg-muted file:px-3 file:py-2 file:text-sm file:font-medium"
              onchange={onIconChange}
            />
            {#if iconError}
              <p class="text-sm text-destructive">{iconError}</p>
            {/if}
          </div>

          <button
            type="submit"
            class="inline-flex w-full items-center justify-center rounded-md bg-fire px-4 py-2 text-sm font-medium text-fire-foreground transition-opacity hover:opacity-90 disabled:cursor-not-allowed disabled:opacity-60"
            disabled={saving}
          >
            {saving ? 'Saving...' : 'Save Guild'}
          </button>
        </form>

        <section class="mt-6 border-t border-border pt-5" aria-labelledby="guild-roles-heading">
          <div class="mb-3 flex items-center justify-between gap-2">
            <div>
              <h3 id="guild-roles-heading" class="text-base font-semibold">Roles</h3>
              <p class="text-sm text-muted-foreground">
                Owner role appears first, custom roles in hierarchy order, and @everyone remains last.
              </p>
            </div>
            <button
              type="button"
              class="rounded-md bg-muted px-3 py-2 text-sm font-medium text-foreground hover:opacity-90"
              onclick={openCreateRoleDialog}
            >
              Create role
            </button>
          </div>

          {#if rolesStatusMessage}
            <p class="mb-3 rounded-md border border-emerald-500/30 bg-emerald-500/10 p-3 text-sm text-emerald-300">
              {rolesStatusMessage}
            </p>
          {/if}
          {#if rolesErrorMessage}
            <p class="mb-3 rounded-md border border-destructive/30 bg-destructive/10 p-3 text-sm text-destructive" role="alert">
              {rolesErrorMessage}
            </p>
          {/if}

          {#if roleList.length === 0}
            <p class="rounded-md border border-border bg-background p-3 text-sm text-muted-foreground">
              No roles available.
            </p>
          {:else}
            <ul class="space-y-2">
              {#each roleList as role (role.id)}
                <li
                  class={`flex items-center justify-between gap-3 rounded-md border p-3 ${
                    role.isSystem
                      ? 'border-fire/40 bg-fire/5'
                      : 'border-border bg-background'
                  }`}
                  data-testid={`guild-role-item-${role.id}`}
                  draggable={isRoleReorderable(role)}
                  ondragstart={(event) => handleRoleDragStart(event, role)}
                  ondragover={(event) => handleRoleDragOver(event, role)}
                  ondrop={(event) => void handleRoleDrop(event, role)}
                  ondragend={handleRoleDragEnd}
                >
                  <div class="min-w-0">
                    <div class="flex items-center gap-2">
                      <span
                        class="inline-block h-3 w-3 shrink-0 rounded-full border border-border"
                        style={`background-color: ${role.color}`}
                        aria-hidden="true"
                      ></span>
                      <p
                        class="truncate text-sm font-medium text-foreground"
                        data-testid="guild-role-name"
                      >
                        {role.name}
                      </p>
                    </div>
                    <div class="mt-1 flex items-center gap-2 text-xs text-muted-foreground">
                      {#if role.isSystem}
                        <span class="rounded bg-muted px-2 py-0.5">System role</span>
                      {/if}
                      {#if isFixedRole(role)}
                        <span class="rounded bg-muted px-2 py-0.5">Fixed position</span>
                      {:else}
                        <span class="rounded bg-muted px-2 py-0.5">Drag to reorder</span>
                      {/if}
                      <span>Color: {role.color}</span>
                    </div>
                  </div>

                  <div class="flex shrink-0 items-center gap-2">
                    <button
                      type="button"
                      class="rounded-md bg-muted px-2 py-1 text-xs text-foreground hover:opacity-90"
                      onclick={() => openPermissionsDialog(role)}
                      aria-label={`Edit permissions for ${role.name}`}
                    >
                      Permissions
                    </button>
                    {#if role.canEdit}
                      <button
                        type="button"
                        class="rounded-md bg-muted px-2 py-1 text-xs text-foreground hover:opacity-90"
                        onclick={() => openEditRoleDialog(role)}
                        aria-label={`Edit role ${role.name}`}
                      >
                        Edit
                      </button>
                    {/if}
                    {#if role.canDelete}
                      <button
                        type="button"
                        class="rounded-md bg-destructive px-2 py-1 text-xs font-medium text-destructive-foreground hover:opacity-90"
                        onclick={() => openDeleteRoleDialog(role)}
                        aria-label={`Delete role ${role.name}`}
                      >
                        Delete
                      </button>
                    {/if}
                  </div>
                </li>
              {/each}
            </ul>
          {/if}
        </section>
      {/if}
    </div>
  </div>

  {#if createRoleDialogOpen}
    <div class="fixed inset-0 z-[60] flex items-center justify-center bg-black/60 p-4" role="presentation">
      <div
        class="w-full max-w-md rounded-lg border border-border bg-card p-6 shadow-2xl"
        role="dialog"
        aria-modal="true"
        aria-label="Create role"
      >
        <header class="mb-4 flex items-center justify-between">
          <h3 class="text-lg font-semibold">Create role</h3>
          <button
            type="button"
            class="rounded-md bg-muted px-3 py-1 text-sm text-foreground hover:opacity-90"
            onclick={closeCreateRoleDialog}
          >
            Cancel
          </button>
        </header>

        <form class="space-y-4" onsubmit={handleCreateRoleSubmit} data-testid="create-role-form">
          <div class="space-y-1">
            <label for="guild-role-create-name" class="text-sm font-medium">Role name</label>
            <input
              id="guild-role-create-name"
              type="text"
              class={`w-full rounded-md border bg-background px-3 py-2 text-base focus:outline-none focus:ring-2 ${
                createRoleNameError
                  ? 'border-destructive focus:ring-destructive'
                  : 'border-input focus:ring-ring'
              }`}
              bind:value={createRoleName}
              onblur={onCreateRoleNameBlur}
              maxlength={64}
              required
            />
            {#if createRoleNameError}
              <p class="text-sm text-destructive">{createRoleNameError}</p>
            {/if}
          </div>

          <div class="space-y-1">
            <label for="guild-role-create-color" class="text-sm font-medium">Role color</label>
            <input
              id="guild-role-create-color"
              type="color"
              class="h-10 w-20 rounded-md border border-input bg-background p-1"
              bind:value={createRoleColor}
              onblur={onCreateRoleColorBlur}
            />
            {#if createRoleColorError}
              <p class="text-sm text-destructive">{createRoleColorError}</p>
            {/if}
          </div>

          {#if createRoleError}
            <p class="rounded-md border border-destructive/30 bg-destructive/10 p-3 text-sm text-destructive" role="alert">
              {createRoleError}
            </p>
          {/if}

          <button
            type="submit"
            class="inline-flex w-full items-center justify-center rounded-md bg-fire px-4 py-2 text-sm font-medium text-fire-foreground transition-opacity hover:opacity-90 disabled:cursor-not-allowed disabled:opacity-60"
            disabled={createRoleSubmitting}
          >
            {createRoleSubmitting ? 'Creating...' : 'Create role'}
          </button>
        </form>
      </div>
    </div>
  {/if}

  {#if editRoleDialogOpen && editRoleTarget}
    <div class="fixed inset-0 z-[60] flex items-center justify-center bg-black/60 p-4" role="presentation">
      <div
        class="w-full max-w-md rounded-lg border border-border bg-card p-6 shadow-2xl"
        role="dialog"
        aria-modal="true"
        aria-label="Edit role"
      >
        <header class="mb-4 flex items-center justify-between">
          <h3 class="text-lg font-semibold">Edit role</h3>
          <button
            type="button"
            class="rounded-md bg-muted px-3 py-1 text-sm text-foreground hover:opacity-90"
            onclick={closeEditRoleDialog}
          >
            Cancel
          </button>
        </header>

        <form class="space-y-4" onsubmit={handleEditRoleSubmit} data-testid="edit-role-form">
          <div class="space-y-1">
            <label for="guild-role-edit-name" class="text-sm font-medium">Role name</label>
            <input
              id="guild-role-edit-name"
              type="text"
              class={`w-full rounded-md border bg-background px-3 py-2 text-base focus:outline-none focus:ring-2 ${
                editRoleNameError
                  ? 'border-destructive focus:ring-destructive'
                  : 'border-input focus:ring-ring'
              }`}
              bind:value={editRoleName}
              onblur={onEditRoleNameBlur}
              maxlength={64}
              required
            />
            {#if editRoleNameError}
              <p class="text-sm text-destructive">{editRoleNameError}</p>
            {/if}
          </div>

          <div class="space-y-1">
            <label for="guild-role-edit-color" class="text-sm font-medium">Role color</label>
            <input
              id="guild-role-edit-color"
              type="color"
              class="h-10 w-20 rounded-md border border-input bg-background p-1"
              bind:value={editRoleColor}
              onblur={onEditRoleColorBlur}
            />
            {#if editRoleColorError}
              <p class="text-sm text-destructive">{editRoleColorError}</p>
            {/if}
          </div>

          {#if editRoleError}
            <p class="rounded-md border border-destructive/30 bg-destructive/10 p-3 text-sm text-destructive" role="alert">
              {editRoleError}
            </p>
          {/if}

          <button
            type="submit"
            class="inline-flex w-full items-center justify-center rounded-md bg-fire px-4 py-2 text-sm font-medium text-fire-foreground transition-opacity hover:opacity-90 disabled:cursor-not-allowed disabled:opacity-60"
            disabled={editRoleSubmitting}
          >
            {editRoleSubmitting ? 'Saving...' : 'Save role'}
          </button>
        </form>
      </div>
    </div>
  {/if}

  {#if permissionsDialogOpen && permissionsRoleTarget}
    <div class="fixed inset-0 z-[60] flex items-center justify-center bg-black/60 p-4" role="presentation">
      <div
        class="max-h-[90vh] w-full max-w-xl overflow-y-auto rounded-lg border border-border bg-card p-6 shadow-2xl"
        role="dialog"
        aria-modal="true"
        aria-label="Role permissions"
      >
        <header class="mb-4 flex items-center justify-between">
          <h3 class="text-lg font-semibold">Role permissions</h3>
          <button
            type="button"
            class="rounded-md bg-muted px-3 py-1 text-sm text-foreground hover:opacity-90"
            onclick={closePermissionsDialog}
          >
            Close
          </button>
        </header>

        <p class="mb-3 text-sm text-foreground">
          Managing permissions for <strong>{permissionsRoleTarget.name}</strong>.
        </p>

        {#if isOwnerPseudoRole(permissionsRoleTarget)}
          <p class="mb-3 rounded-md border border-fire/40 bg-fire/10 p-3 text-sm text-fire">
            The Owner role always has all permissions implicitly and cannot be modified.
          </p>
        {/if}

        {#if permissionsError}
          <p class="mb-3 rounded-md border border-destructive/30 bg-destructive/10 p-3 text-sm text-destructive" role="alert">
            {permissionsError}
          </p>
        {/if}

        <ul class="space-y-2">
          {#each permissionCatalog as permission (permission.key)}
            <li class="flex items-start justify-between gap-3 rounded-md border border-border bg-background p-3">
              <div class="min-w-0">
                <p class="text-sm font-medium text-foreground">{permission.label}</p>
                <p class="text-xs text-muted-foreground">{permission.description}</p>
              </div>
              <label class="flex shrink-0 items-center gap-2">
                <input
                  type="checkbox"
                  class="h-4 w-4 accent-fire"
                  checked={permissionIsEnabled(permission)}
                  onchange={(event) => void handlePermissionToggle(permission, event)}
                  disabled={!permissionsRoleTarget.canEdit || permissionsSavingKey !== null}
                  aria-label={permission.label}
                />
                <span class="text-xs text-muted-foreground">
                  {permissionIsEnabled(permission) ? 'On' : 'Off'}
                </span>
              </label>
            </li>
          {/each}
        </ul>

        {#if permissionsSavingKey}
          <p class="mt-3 text-xs text-muted-foreground" aria-live="polite">
            Saving permission change...
          </p>
        {/if}
      </div>
    </div>
  {/if}

  {#if deleteRoleDialogOpen && deleteRoleTarget}
    <div class="fixed inset-0 z-[60] flex items-center justify-center bg-black/60 p-4" role="presentation">
      <div
        class="w-full max-w-md rounded-lg border border-border bg-card p-6 shadow-2xl"
        role="dialog"
        aria-modal="true"
        aria-label="Delete role"
      >
        <header class="mb-4">
          <h3 class="text-lg font-semibold">Delete role</h3>
        </header>

        <p class="mb-3 text-sm text-foreground">
          You are about to delete <strong>{deleteRoleTarget.name}</strong>.
        </p>
        <p class="mb-4 rounded-md border border-destructive/30 bg-destructive/10 p-3 text-sm text-destructive">
          This action is irreversible and removes this role from all assigned members.
        </p>

        {#if deleteRoleError}
          <p class="mb-4 rounded-md border border-destructive/30 bg-destructive/10 p-3 text-sm text-destructive" role="alert">
            {deleteRoleError}
          </p>
        {/if}

        <div class="flex items-center justify-end gap-2">
          <button
            type="button"
            class="rounded-md bg-muted px-3 py-2 text-sm text-foreground hover:opacity-90"
            onclick={closeDeleteRoleDialog}
            disabled={deleteRoleSubmitting}
          >
            Cancel
          </button>
          <button
            type="button"
            class="rounded-md bg-destructive px-3 py-2 text-sm font-medium text-destructive-foreground hover:opacity-90 disabled:cursor-not-allowed disabled:opacity-60"
            onclick={() => void confirmDeleteRole()}
            disabled={deleteRoleSubmitting}
          >
            {deleteRoleSubmitting ? 'Deleting...' : 'Delete role'}
          </button>
        </div>
      </div>
    </div>
  {/if}
{/if}
