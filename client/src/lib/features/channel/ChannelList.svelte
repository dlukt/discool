<script lang="ts">
// biome-ignore-all lint/correctness/noUnusedVariables: Svelte template usage isn't detected reliably.
// biome-ignore lint/correctness/noUnusedImports: Used in Svelte markup; Biome doesn't detect template usage.
import { goto, route as routerLink } from '@mateothegreat/svelte5-router'
import { onMount, tick } from 'svelte'

import { ApiError } from '$lib/api'
import { guildState } from '$lib/features/guild/guildStore.svelte'
import {
  GUILD_PERMISSION_CATALOG,
  hasGuildPermission,
} from '$lib/features/guild/permissions'
import { identityState } from '$lib/features/identity/identityStore.svelte'

import { channelState } from './channelStore.svelte'
import type {
  Channel,
  ChannelCategory,
  ChannelPermissionOverrideRole,
  ChannelType,
} from './types'

type Props = {
  activeGuild: string
  activeChannel: string
}

type PermissionOverrideState = 'allow' | 'deny' | 'inherit'

let { activeGuild, activeChannel }: Props = $props()

let guild = $derived(guildState.bySlug(activeGuild))
let guildLabel = $derived(guild?.name ?? activeGuild)
const manageChannelsPermission = GUILD_PERMISSION_CATALOG.find(
  (permission) => permission.key === 'MANAGE_CHANNELS',
)
let memberRoleData = $derived(guildState.memberRoleDataForGuild(activeGuild))
let currentUserId = $derived(identityState.session?.user.id ?? null)
let currentMember = $derived(
  currentUserId
    ? (memberRoleData.members.find(
        (member) => member.userId === currentUserId,
      ) ?? null)
    : null,
)
let rolePermissionMaskById = $derived(
  new Map(
    memberRoleData.roles.map((role) => [role.id, role.permissionsBitflag]),
  ),
)
let defaultRolePermissionsBitflag = $derived(
  memberRoleData.roles.find((role) => role.isDefault)?.permissionsBitflag ?? 0,
)
let currentMemberPermissionsBitflag = $derived(
  (currentMember?.roleIds ?? []).reduce(
    (mask, roleId) => mask | (rolePermissionMaskById.get(roleId) ?? 0),
    defaultRolePermissionsBitflag,
  ),
)
let canManageChannels = $derived(
  Boolean(guild?.isOwner) ||
    (manageChannelsPermission !== undefined &&
      hasGuildPermission(
        currentMemberPermissionsBitflag,
        manageChannelsPermission,
      )),
)
let channels = $derived(channelState.channels)
let categories = $derived(channelState.categories)
let loadingChannels = $derived(
  channelState.loading && channelState.activeGuild === activeGuild,
)

let loadError = $state<string | null>(null)
let reorderError = $state<string | null>(null)

let createDialogOpen = $state(false)
let createName = $state('')
let createType = $state<ChannelType>('text')
let createCategorySlug = $state<string | null>(null)
let createNameError = $state<string | null>(null)
let createError = $state<string | null>(null)
let createSubmitting = $state(false)
let createTrigger = $state<HTMLButtonElement | null>(null)
let createNameInput = $state<HTMLInputElement | null>(null)

let categoryCreateDialogOpen = $state(false)
let categoryCreateName = $state('')
let categoryCreateNameError = $state<string | null>(null)
let categoryCreateError = $state<string | null>(null)
let categoryCreateSubmitting = $state(false)
let categoryCreateTrigger = $state<HTMLButtonElement | null>(null)
let categoryCreateNameInput = $state<HTMLInputElement | null>(null)

let actionMenuSlug = $state<string | null>(null)
let categoryActionMenuSlug = $state<string | null>(null)

let renameDialogOpen = $state(false)
let renameTarget = $state<Channel | null>(null)
let renameName = $state('')
let renameNameError = $state<string | null>(null)
let renameError = $state<string | null>(null)
let renameSubmitting = $state(false)
let renameInput = $state<HTMLInputElement | null>(null)

let renameCategoryDialogOpen = $state(false)
let renameCategoryTarget = $state<ChannelCategory | null>(null)
let renameCategoryName = $state('')
let renameCategoryNameError = $state<string | null>(null)
let renameCategoryError = $state<string | null>(null)
let renameCategorySubmitting = $state(false)
let renameCategoryInput = $state<HTMLInputElement | null>(null)

let deleteDialogOpen = $state(false)
let deleteTarget = $state<Channel | null>(null)
let deleteError = $state<string | null>(null)
let deleteSubmitting = $state(false)

let deleteCategoryDialogOpen = $state(false)
let deleteCategoryTarget = $state<ChannelCategory | null>(null)
let deleteCategoryError = $state<string | null>(null)
let deleteCategorySubmitting = $state(false)

let permissionsDialogOpen = $state(false)
let permissionsTarget = $state<Channel | null>(null)
let permissionRoles = $state<ChannelPermissionOverrideRole[]>([])
let permissionOverridesByRoleId = $state<
  Record<string, { allowBitflag: number; denyBitflag: number }>
>({})
let selectedPermissionRoleId = $state<string | null>(null)
let permissionsLoading = $state(false)
let permissionsError = $state<string | null>(null)
let permissionSavingByKey = $state<Record<string, boolean>>({})

let draggedSlug = $state<string | null>(null)

let selectedPermissionRoleEntry = $derived(
  selectedPermissionRoleId
    ? (permissionRoles.find((item) => item.id === selectedPermissionRoleId) ??
        null)
    : null,
)

onMount(() => {
  void guildState.loadGuilds().catch(() => {
    // Shell can still render with fallback labels while guild metadata loads.
  })
})

$effect(() => {
  if (!activeGuild) return
  loadError = null
  void channelState.loadChannels(activeGuild).catch((err: unknown) => {
    loadError = messageFromError(err, 'Failed to load channels.')
  })
})

$effect(() => {
  if (!activeGuild || guild?.isOwner) return
  void guildState.loadMembers(activeGuild).catch(() => {
    // Member role data is loaded opportunistically for channel permission gating.
  })
})

$effect(() => {
  if (!activeGuild || channelState.activeGuild !== activeGuild) return
  if (channels.length === 0) return
  if (channels.some((item) => item.slug === activeChannel)) return

  const fallback =
    channels.find((item) => item.isDefault)?.slug ?? channels[0]?.slug ?? null
  if (!fallback || fallback === activeChannel) return

  void goto(`/${activeGuild}/${fallback}`)
})

$effect(() => {
  if (!actionMenuSlug) return
  if (channels.some((item) => item.slug === actionMenuSlug)) return
  actionMenuSlug = null
})

$effect(() => {
  if (!categoryActionMenuSlug) return
  if (categories.some((item) => item.slug === categoryActionMenuSlug)) return
  categoryActionMenuSlug = null
})

function messageFromError(err: unknown, fallback: string): string {
  if (err instanceof ApiError) return err.message
  if (err instanceof Error) return err.message
  return fallback
}

function validateName(value: string, label: string): string | null {
  const trimmed = value.trim()
  if (!trimmed) return `${label} is required.`
  if (trimmed.length > 64) return `${label} must be 64 characters or less.`
  return null
}

function sortedCategories(): ChannelCategory[] {
  return [...categories].sort((a, b) => a.position - b.position)
}

function channelsForCategory(categorySlug: string | null): Channel[] {
  return channels
    .filter((channel) => channel.categorySlug === categorySlug)
    .sort((a, b) => a.position - b.position)
}

function uncategorizedChannels(): Channel[] {
  return channelsForCategory(null)
}

async function openCreateDialog(categorySlug: string | null = null) {
  createDialogOpen = true
  createName = ''
  createType = 'text'
  createCategorySlug = categorySlug
  createNameError = null
  createError = null
  await tick()
  createNameInput?.focus()
}

async function closeCreateDialog() {
  createDialogOpen = false
  createCategorySlug = null
  await tick()
  createTrigger?.focus()
}

function onCreateNameBlur() {
  createNameError = validateName(createName, 'Channel name')
}

async function handleCreateSubmit(event: SubmitEvent) {
  event.preventDefault()
  if (createSubmitting || !activeGuild) return

  createError = null
  createNameError = validateName(createName, 'Channel name')
  if (createNameError) return

  createSubmitting = true
  try {
    await channelState.createChannel(activeGuild, {
      name: createName.trim(),
      channelType: createType,
      categorySlug: createCategorySlug,
    })
    await closeCreateDialog()
  } catch (err) {
    createError = messageFromError(err, 'Failed to create channel.')
  } finally {
    createSubmitting = false
  }
}

async function openCategoryCreateDialog() {
  categoryCreateDialogOpen = true
  categoryCreateName = ''
  categoryCreateNameError = null
  categoryCreateError = null
  await tick()
  categoryCreateNameInput?.focus()
}

async function closeCategoryCreateDialog() {
  categoryCreateDialogOpen = false
  await tick()
  categoryCreateTrigger?.focus()
}

function onCategoryCreateNameBlur() {
  categoryCreateNameError = validateName(categoryCreateName, 'Category name')
}

async function handleCategoryCreateSubmit(event: SubmitEvent) {
  event.preventDefault()
  if (categoryCreateSubmitting || !activeGuild) return

  categoryCreateError = null
  categoryCreateNameError = validateName(categoryCreateName, 'Category name')
  if (categoryCreateNameError) return

  categoryCreateSubmitting = true
  try {
    await channelState.createCategory(activeGuild, {
      name: categoryCreateName.trim(),
    })
    await closeCategoryCreateDialog()
  } catch (err) {
    categoryCreateError = messageFromError(err, 'Failed to create category.')
  } finally {
    categoryCreateSubmitting = false
  }
}

function toggleActionMenu(channelSlug: string) {
  if (actionMenuSlug === channelSlug) {
    actionMenuSlug = null
    return
  }
  actionMenuSlug = channelSlug
  categoryActionMenuSlug = null
}

function toggleCategoryActionMenu(categorySlug: string) {
  if (categoryActionMenuSlug === categorySlug) {
    categoryActionMenuSlug = null
    return
  }
  categoryActionMenuSlug = categorySlug
  actionMenuSlug = null
}

function closeActionMenu() {
  actionMenuSlug = null
}

function closeCategoryActionMenu() {
  categoryActionMenuSlug = null
}

function handleChannelContextMenu(event: MouseEvent, channelSlug: string) {
  if (!canManageChannels) return
  event.preventDefault()
  actionMenuSlug = channelSlug
  categoryActionMenuSlug = null
}

function handleCategoryContextMenu(event: MouseEvent, categorySlug: string) {
  if (!canManageChannels) return
  event.preventDefault()
  categoryActionMenuSlug = categorySlug
  actionMenuSlug = null
}

async function openRenameDialog(channel: Channel) {
  closeActionMenu()
  renameTarget = channel
  renameName = channel.name
  renameNameError = null
  renameError = null
  renameDialogOpen = true
  await tick()
  renameInput?.focus()
}

function closeRenameDialog() {
  renameDialogOpen = false
  renameTarget = null
  renameName = ''
  renameNameError = null
  renameError = null
}

function onRenameNameBlur() {
  renameNameError = validateName(renameName, 'Channel name')
}

async function handleRenameSubmit(event: SubmitEvent) {
  event.preventDefault()
  if (renameSubmitting || !renameTarget || !activeGuild) return

  renameError = null
  renameNameError = validateName(renameName, 'Channel name')
  if (renameNameError) return

  renameSubmitting = true
  try {
    const previousSlug = renameTarget.slug
    const updated = await channelState.updateChannel(
      activeGuild,
      previousSlug,
      {
        name: renameName.trim(),
      },
    )
    closeRenameDialog()
    if (previousSlug === activeChannel && updated.slug !== previousSlug) {
      await goto(`/${activeGuild}/${updated.slug}`)
    }
  } catch (err) {
    renameError = messageFromError(err, 'Failed to rename channel.')
  } finally {
    renameSubmitting = false
  }
}

async function openCategoryRenameDialog(category: ChannelCategory) {
  closeCategoryActionMenu()
  renameCategoryTarget = category
  renameCategoryName = category.name
  renameCategoryNameError = null
  renameCategoryError = null
  renameCategoryDialogOpen = true
  await tick()
  renameCategoryInput?.focus()
}

function closeCategoryRenameDialog() {
  renameCategoryDialogOpen = false
  renameCategoryTarget = null
  renameCategoryName = ''
  renameCategoryNameError = null
  renameCategoryError = null
}

function onRenameCategoryNameBlur() {
  renameCategoryNameError = validateName(renameCategoryName, 'Category name')
}

async function handleCategoryRenameSubmit(event: SubmitEvent) {
  event.preventDefault()
  if (renameCategorySubmitting || !renameCategoryTarget || !activeGuild) return

  renameCategoryError = null
  renameCategoryNameError = validateName(renameCategoryName, 'Category name')
  if (renameCategoryNameError) return

  renameCategorySubmitting = true
  try {
    await channelState.updateCategory(activeGuild, renameCategoryTarget.slug, {
      name: renameCategoryName.trim(),
    })
    closeCategoryRenameDialog()
  } catch (err) {
    renameCategoryError = messageFromError(err, 'Failed to rename category.')
  } finally {
    renameCategorySubmitting = false
  }
}

function openDeleteDialog(channel: Channel) {
  closeActionMenu()
  deleteTarget = channel
  deleteError = null
  deleteDialogOpen = true
}

function closeDeleteDialog() {
  deleteDialogOpen = false
  deleteTarget = null
  deleteError = null
}

async function confirmDeleteChannel() {
  if (deleteSubmitting || !deleteTarget || !activeGuild) return

  deleteSubmitting = true
  deleteError = null
  try {
    const deletingSlug = deleteTarget.slug
    const deleted = await channelState.deleteChannel(activeGuild, deletingSlug)
    closeDeleteDialog()
    if (deletingSlug === activeChannel) {
      await goto(`/${activeGuild}/${deleted.fallbackChannelSlug}`)
    }
  } catch (err) {
    deleteError = messageFromError(err, 'Failed to delete channel.')
  } finally {
    deleteSubmitting = false
  }
}

function openDeleteCategoryDialog(category: ChannelCategory) {
  closeCategoryActionMenu()
  deleteCategoryTarget = category
  deleteCategoryError = null
  deleteCategoryDialogOpen = true
}

function closeDeleteCategoryDialog() {
  deleteCategoryDialogOpen = false
  deleteCategoryTarget = null
  deleteCategoryError = null
}

async function confirmDeleteCategory() {
  if (deleteCategorySubmitting || !deleteCategoryTarget || !activeGuild) return

  deleteCategorySubmitting = true
  deleteCategoryError = null
  try {
    await channelState.deleteCategory(activeGuild, deleteCategoryTarget.slug)
    closeDeleteCategoryDialog()
  } catch (err) {
    deleteCategoryError = messageFromError(err, 'Failed to delete category.')
  } finally {
    deleteCategorySubmitting = false
  }
}

function sortPermissionRoles(
  roles: ChannelPermissionOverrideRole[],
): ChannelPermissionOverrideRole[] {
  return [...roles].sort((left, right) => left.position - right.position)
}

function applyPermissionOverrides(
  overrides: Array<{
    roleId: string
    allowBitflag: number
    denyBitflag: number
  }>,
) {
  const byRoleId: Record<
    string,
    { allowBitflag: number; denyBitflag: number }
  > = {}
  for (const item of overrides) {
    byRoleId[item.roleId] = {
      allowBitflag: item.allowBitflag,
      denyBitflag: item.denyBitflag,
    }
  }
  permissionOverridesByRoleId = byRoleId
}

async function openPermissionsDialog(channel: Channel) {
  closeActionMenu()
  permissionsDialogOpen = true
  permissionsTarget = channel
  permissionsLoading = true
  permissionsError = null
  permissionSavingByKey = {}
  permissionRoles = []
  permissionOverridesByRoleId = {}
  selectedPermissionRoleId = null
  try {
    const data = await channelState.loadChannelPermissionOverrides(
      activeGuild,
      channel.slug,
      true,
    )
    permissionRoles = sortPermissionRoles(data.roles)
    applyPermissionOverrides(data.overrides)
    selectedPermissionRoleId = permissionRoles[0]?.id ?? null
  } catch (err) {
    permissionsError = messageFromError(
      err,
      'Failed to load channel permission overrides.',
    )
  } finally {
    permissionsLoading = false
  }
}

function closePermissionsDialog() {
  permissionsDialogOpen = false
  permissionsTarget = null
  permissionsLoading = false
  permissionsError = null
  permissionSavingByKey = {}
  permissionRoles = []
  permissionOverridesByRoleId = {}
  selectedPermissionRoleId = null
}

function permissionCellKey(roleId: string, permissionMask: number): string {
  return `${roleId}:${permissionMask}`
}

function channelPermissionState(
  roleId: string,
  permissionMask: number,
): PermissionOverrideState {
  const existing = permissionOverridesByRoleId[roleId]
  if (existing && (existing.allowBitflag & permissionMask) === permissionMask) {
    return 'allow'
  }
  if (existing && (existing.denyBitflag & permissionMask) === permissionMask) {
    return 'deny'
  }
  return 'inherit'
}

function replaceRolePermissionOverride(
  roleId: string,
  allowBitflag: number,
  denyBitflag: number,
) {
  if (allowBitflag === 0 && denyBitflag === 0) {
    const { [roleId]: _removed, ...remaining } = permissionOverridesByRoleId
    permissionOverridesByRoleId = remaining
    return
  }
  permissionOverridesByRoleId = {
    ...permissionOverridesByRoleId,
    [roleId]: { allowBitflag, denyBitflag },
  }
}

async function setPermissionOverrideState(
  roleId: string,
  permissionMask: number,
  nextState: PermissionOverrideState,
) {
  if (!activeGuild || !permissionsTarget) return
  const previous = permissionOverridesByRoleId[roleId] ?? {
    allowBitflag: 0,
    denyBitflag: 0,
  }
  const nextAllow = previous.allowBitflag & ~permissionMask
  const nextDeny = previous.denyBitflag & ~permissionMask
  const allowBitflag =
    nextState === 'allow' ? nextAllow | permissionMask : nextAllow
  const denyBitflag =
    nextState === 'deny' ? nextDeny | permissionMask : nextDeny

  replaceRolePermissionOverride(roleId, allowBitflag, denyBitflag)
  permissionsError = null
  const key = permissionCellKey(roleId, permissionMask)
  permissionSavingByKey = { ...permissionSavingByKey, [key]: true }
  try {
    if (allowBitflag === 0 && denyBitflag === 0) {
      await channelState.deleteChannelPermissionOverride(
        activeGuild,
        permissionsTarget.slug,
        roleId,
      )
      replaceRolePermissionOverride(roleId, 0, 0)
      return
    }
    const saved = await channelState.upsertChannelPermissionOverride(
      activeGuild,
      permissionsTarget.slug,
      roleId,
      { allowBitflag, denyBitflag },
    )
    replaceRolePermissionOverride(roleId, saved.allowBitflag, saved.denyBitflag)
  } catch (err) {
    replaceRolePermissionOverride(
      roleId,
      previous.allowBitflag,
      previous.denyBitflag,
    )
    permissionsError = messageFromError(
      err,
      'Failed to save channel permission override.',
    )
  } finally {
    const { [key]: _ignored, ...rest } = permissionSavingByKey
    permissionSavingByKey = rest
  }
}

async function moveChannelWithinCategory(
  channelSlug: string,
  categorySlug: string | null,
  offset: number,
) {
  if (!canManageChannels || !activeGuild) return
  const bucket = channelsForCategory(categorySlug)
  const index = bucket.findIndex((item) => item.slug === channelSlug)
  if (index < 0) return
  const nextIndex = index + offset
  if (nextIndex < 0 || nextIndex >= bucket.length) return

  reorderError = null
  try {
    await channelState.moveChannel(
      activeGuild,
      channelSlug,
      categorySlug,
      nextIndex,
    )
  } catch (err) {
    reorderError = messageFromError(err, 'Failed to reorder channels.')
  }
}

async function moveCategoryByOffset(categorySlug: string, offset: number) {
  if (!canManageChannels || !activeGuild) return
  const ordered = sortedCategories()
  const index = ordered.findIndex((item) => item.slug === categorySlug)
  if (index < 0) return
  const nextIndex = index + offset
  if (nextIndex < 0 || nextIndex >= ordered.length) return

  const slugs = ordered.map((item) => item.slug)
  const [moved] = slugs.splice(index, 1)
  slugs.splice(nextIndex, 0, moved)

  reorderError = null
  try {
    await channelState.reorderCategories(activeGuild, slugs)
  } catch (err) {
    reorderError = messageFromError(err, 'Failed to reorder categories.')
  }
}

async function persistCategoryCollapsed(
  category: ChannelCategory,
  collapsed: boolean,
) {
  if (!activeGuild) return
  reorderError = null
  try {
    await channelState.setCategoryCollapsed(
      activeGuild,
      category.slug,
      collapsed,
    )
  } catch (err) {
    reorderError = messageFromError(err, 'Failed to persist category state.')
  }
}

async function toggleCategory(category: ChannelCategory) {
  await persistCategoryCollapsed(category, !category.collapsed)
}

function resolveDraggedSlug(event: DragEvent): string | null {
  return draggedSlug ?? event.dataTransfer?.getData('text/plain') ?? null
}

function handleDragStart(event: DragEvent, channelSlug: string) {
  if (!canManageChannels) return
  draggedSlug = channelSlug
  event.dataTransfer?.setData('text/plain', channelSlug)
  if (event.dataTransfer) {
    event.dataTransfer.effectAllowed = 'move'
  }
}

function handleDragOver(event: DragEvent) {
  if (!canManageChannels) return
  const sourceSlug = resolveDraggedSlug(event)
  if (!sourceSlug) return
  event.preventDefault()
  if (event.dataTransfer) {
    event.dataTransfer.dropEffect = 'move'
  }
}

async function handleDropOnChannel(event: DragEvent, target: Channel) {
  if (!canManageChannels || !activeGuild) return
  event.preventDefault()

  const sourceSlug = resolveDraggedSlug(event)
  draggedSlug = null
  if (!sourceSlug || sourceSlug === target.slug) return
  const targetBucket = channelsForCategory(target.categorySlug)
  const sourceIndex = targetBucket.findIndex((item) => item.slug === sourceSlug)
  const targetIndex = targetBucket.findIndex(
    (item) => item.slug === target.slug,
  )
  if (targetIndex < 0) return
  const targetPosition =
    sourceIndex >= 0 && sourceIndex < targetIndex
      ? targetIndex - 1
      : targetIndex

  reorderError = null
  try {
    await channelState.moveChannel(
      activeGuild,
      sourceSlug,
      target.categorySlug,
      targetPosition,
    )
  } catch (err) {
    reorderError = messageFromError(err, 'Failed to move channel.')
  }
}

async function handleDropOnCategory(
  event: DragEvent,
  categorySlug: string | null,
) {
  if (!canManageChannels || !activeGuild) return
  event.preventDefault()

  const sourceSlug = resolveDraggedSlug(event)
  draggedSlug = null
  if (!sourceSlug) return

  const targetPosition = channelsForCategory(categorySlug).length
  reorderError = null
  try {
    await channelState.moveChannel(
      activeGuild,
      sourceSlug,
      categorySlug,
      targetPosition,
    )
  } catch (err) {
    reorderError = messageFromError(err, 'Failed to move channel.')
  }
}

function handleDragEnd() {
  draggedSlug = null
}

function focusByDelta(source: HTMLElement, delta: number) {
  if (typeof document === 'undefined') return
  const nodes = Array.from(
    document.querySelectorAll<HTMLElement>('[data-channel-nav="true"]'),
  ).filter((node) => !node.hasAttribute('disabled'))
  const index = nodes.indexOf(source)
  if (index < 0) return
  const next = nodes[index + delta]
  next?.focus()
}

function handleArrowNavigation(event: KeyboardEvent) {
  if (event.key !== 'ArrowDown' && event.key !== 'ArrowUp') return
  event.preventDefault()
  const source = event.currentTarget
  if (!(source instanceof HTMLElement)) return
  focusByDelta(source, event.key === 'ArrowDown' ? 1 : -1)
}

function handleChannelKeydown(event: KeyboardEvent, channelSlug: string) {
  if (event.key === 'ContextMenu' || (event.shiftKey && event.key === 'F10')) {
    if (!canManageChannels) return
    event.preventDefault()
    actionMenuSlug = channelSlug
    categoryActionMenuSlug = null
    return
  }
  handleArrowNavigation(event)
}

function handleCategoryHeaderKeydown(
  event: KeyboardEvent,
  category: ChannelCategory,
) {
  if (event.key === 'ContextMenu' || (event.shiftKey && event.key === 'F10')) {
    if (!canManageChannels) return
    event.preventDefault()
    categoryActionMenuSlug = category.slug
    actionMenuSlug = null
    return
  }
  if (event.key === 'Enter' || event.key === ' ') {
    event.preventDefault()
    void toggleCategory(category)
    return
  }
  if (event.key === 'ArrowRight' && category.collapsed) {
    event.preventDefault()
    void persistCategoryCollapsed(category, false)
    return
  }
  if (event.key === 'ArrowLeft' && !category.collapsed) {
    event.preventDefault()
    void persistCategoryCollapsed(category, true)
    return
  }
  handleArrowNavigation(event)
}
</script>

<aside
  class="flex h-full min-h-0 flex-col border-r border-border bg-card p-4"
  data-testid="channel-list"
  aria-label="Channel navigation"
>
  <div class="mb-3 flex items-center justify-between gap-2">
    <h2 class="truncate text-sm font-semibold text-foreground">{guildLabel}</h2>
    {#if canManageChannels}
      <div class="flex items-center gap-1">
        <button
          bind:this={categoryCreateTrigger}
          type="button"
          class="inline-flex h-8 items-center justify-center rounded-md border border-border bg-muted px-2 text-xs font-semibold text-foreground transition-colors hover:bg-sidebar-accent hover:text-sidebar-accent-foreground"
          aria-label="Create category"
          onclick={() => void openCategoryCreateDialog()}
        >
          Create Category
        </button>
        <button
          bind:this={createTrigger}
          type="button"
          class="inline-flex h-8 w-8 items-center justify-center rounded-md border border-border bg-muted text-lg font-semibold text-foreground transition-colors hover:bg-sidebar-accent hover:text-sidebar-accent-foreground"
          aria-label="Create channel"
          onclick={() => void openCreateDialog(null)}
        >
          +
        </button>
      </div>
    {/if}
  </div>

  {#if loadError}
    <p
      class="mb-3 rounded-md border border-destructive/30 bg-destructive/10 p-2 text-xs text-destructive"
      role="alert"
    >
      {loadError}
    </p>
  {/if}
  {#if reorderError}
    <p
      class="mb-3 rounded-md border border-destructive/30 bg-destructive/10 p-2 text-xs text-destructive"
      role="alert"
    >
      {reorderError}
    </p>
  {/if}

  <div class="min-h-0 flex-1 overflow-y-auto pr-1" data-testid="channel-list-scroll-area">
    {#if loadingChannels && channels.length === 0}
      <p class="rounded-md bg-muted px-3 py-2 text-sm text-muted-foreground">Loading channels...</p>
    {:else if channels.length === 0 && categories.length === 0}
      <p class="rounded-md bg-muted px-3 py-2 text-sm text-muted-foreground">No channels yet.</p>
    {:else}
      <div class="space-y-3" aria-label="Guild channels">
        {#each sortedCategories() as category, categoryIndex}
          <section
            class="group rounded-md"
            role="group"
            data-testid={`category-section-${category.slug}`}
            ondragover={handleDragOver}
            ondrop={(event) => void handleDropOnCategory(event, category.slug)}
            oncontextmenu={(event) => handleCategoryContextMenu(event, category.slug)}
          >
            <div class="flex items-center gap-1 px-1 py-1">
              <button
                type="button"
                class="flex min-w-0 flex-1 items-center gap-2 rounded-md px-2 py-1 text-left text-muted-foreground transition-colors hover:bg-muted hover:text-foreground"
                aria-label={`Toggle category ${category.name.toUpperCase()}`}
                aria-expanded={!category.collapsed}
                data-channel-nav="true"
                onkeydown={(event) => handleCategoryHeaderKeydown(event, category)}
                onclick={() => void toggleCategory(category)}
              >
                <span class="inline-flex w-4 shrink-0 items-center justify-center text-xs" aria-hidden="true">
                  {category.collapsed ? '▸' : '▾'}
                </span>
                <span class="truncate text-[11px] font-semibold uppercase tracking-wide text-muted-foreground">
                  {category.name}
                </span>
              </button>

              {#if canManageChannels}
                <button
                  type="button"
                  class="inline-flex h-7 w-7 items-center justify-center rounded-md text-sm text-muted-foreground transition-colors hover:bg-muted hover:text-foreground"
                  aria-label={`Create channel in ${category.name}`}
                  onclick={() => void openCreateDialog(category.slug)}
                >
                  +
                </button>
                <button
                  type="button"
                  class="inline-flex h-7 w-7 items-center justify-center rounded-md text-base text-muted-foreground transition-colors hover:bg-muted hover:text-foreground"
                  aria-label={`Open category actions for ${category.name}`}
                  onclick={() => toggleCategoryActionMenu(category.slug)}
                >
                  ⋯
                </button>
              {/if}
            </div>

            {#if canManageChannels && categoryActionMenuSlug === category.slug}
              <div
                class="mb-2 grid gap-1 rounded-md border border-border bg-card p-2"
                role="menu"
                aria-label={`Category actions for ${category.name}`}
              >
                <button
                  type="button"
                  class="rounded-md px-2 py-1 text-left text-sm text-foreground hover:bg-muted"
                  role="menuitem"
                  onclick={() => void openCategoryRenameDialog(category)}
                >
                  Rename category
                </button>
                <button
                  type="button"
                  class="rounded-md px-2 py-1 text-left text-sm text-destructive hover:bg-destructive/10"
                  role="menuitem"
                  onclick={() => openDeleteCategoryDialog(category)}
                >
                  Delete category
                </button>
                <button
                  type="button"
                  class="rounded-md px-2 py-1 text-left text-sm text-foreground hover:bg-muted disabled:cursor-not-allowed disabled:opacity-50"
                  role="menuitem"
                  onclick={() => void moveCategoryByOffset(category.slug, -1)}
                  disabled={categoryIndex === 0}
                >
                  Move up
                </button>
                <button
                  type="button"
                  class="rounded-md px-2 py-1 text-left text-sm text-foreground hover:bg-muted disabled:cursor-not-allowed disabled:opacity-50"
                  role="menuitem"
                  onclick={() => void moveCategoryByOffset(category.slug, 1)}
                  disabled={categoryIndex === sortedCategories().length - 1}
                >
                  Move down
                </button>
              </div>
            {/if}

            {#if !category.collapsed}
              <ul class="space-y-1 pl-2" aria-label={`Channels in ${category.name}`}>
                {#each channelsForCategory(category.slug) as channel}
                  <li
                    class="rounded-md"
                    draggable={canManageChannels}
                    ondragstart={(event) => handleDragStart(event, channel.slug)}
                    ondragover={handleDragOver}
                    ondrop={(event) => void handleDropOnChannel(event, channel)}
                    ondragend={handleDragEnd}
                    oncontextmenu={(event) => handleChannelContextMenu(event, channel.slug)}
                    data-testid={`channel-item-${channel.slug}`}
                  >
                    <div class="flex items-center gap-1">
                      <a
                        class={`flex min-w-0 flex-1 items-center gap-2 rounded-md px-3 py-2 text-sm transition-colors ${
                          channel.slug === activeChannel
                            ? 'bg-sidebar-accent text-sidebar-accent-foreground'
                            : channel.hasUnreadActivity
                              ? 'text-foreground hover:bg-muted hover:text-foreground'
                            : 'text-muted-foreground hover:bg-muted hover:text-foreground'
                        }`}
                        href={`/${activeGuild}/${channel.slug}`}
                        use:routerLink
                        aria-label={`${channel.hasUnreadActivity ? 'Unread. ' : ''}Open channel ${channel.name}`}
                        aria-current={channel.slug === activeChannel ? 'page' : undefined}
                        data-has-unread-activity={channel.hasUnreadActivity
                          ? 'true'
                          : 'false'}
                        data-channel-nav="true"
                        onkeydown={(event) => handleChannelKeydown(event, channel.slug)}
                        data-testid={`channel-link-${channel.slug}`}
                      >
                        <span
                          class="inline-flex w-4 shrink-0 items-center justify-center text-xs"
                          aria-hidden="true"
                          data-testid={`channel-icon-${channel.slug}`}
                        >
                          {channel.channelType === 'voice' ? '🔊' : '#'}
                        </span>
                        <span
                          class={`truncate ${channel.hasUnreadActivity ? 'font-semibold text-foreground' : ''}`}
                        >
                          {channel.name}
                        </span>
                        {#if channel.hasUnreadActivity}
                          <span
                            class="ml-auto inline-flex h-2 w-2 shrink-0 rounded-full bg-sky-300"
                            aria-hidden="true"
                            data-testid={`channel-unread-dot-${channel.slug}`}
                          ></span>
                        {/if}
                      </a>

                      {#if canManageChannels}
                        <button
                          type="button"
                          class="inline-flex h-8 w-8 items-center justify-center rounded-md text-base text-muted-foreground transition-colors hover:bg-muted hover:text-foreground"
                          aria-label={`Open channel actions for ${channel.name}`}
                          onclick={() => toggleActionMenu(channel.slug)}
                        >
                          ⋯
                        </button>
                      {/if}
                    </div>

                    {#if canManageChannels && actionMenuSlug === channel.slug}
                      <div
                        class="mt-1 grid gap-1 rounded-md border border-border bg-card p-2"
                        role="menu"
                        aria-label={`Actions for ${channel.name}`}
                      >
                        <button
                          type="button"
                          class="rounded-md px-2 py-1 text-left text-sm text-foreground hover:bg-muted"
                          role="menuitem"
                          onclick={() => void openPermissionsDialog(channel)}
                        >
                          Permission overrides
                        </button>
                        <button
                          type="button"
                          class="rounded-md px-2 py-1 text-left text-sm text-foreground hover:bg-muted"
                          role="menuitem"
                          onclick={() => void openRenameDialog(channel)}
                        >
                          Edit channel
                        </button>
                        <button
                          type="button"
                          class="rounded-md px-2 py-1 text-left text-sm text-destructive hover:bg-destructive/10"
                          role="menuitem"
                          onclick={() => openDeleteDialog(channel)}
                        >
                          Delete channel
                        </button>
                        <button
                          type="button"
                          class="rounded-md px-2 py-1 text-left text-sm text-foreground hover:bg-muted disabled:cursor-not-allowed disabled:opacity-50"
                          role="menuitem"
                          onclick={() => void moveChannelWithinCategory(channel.slug, channel.categorySlug, -1)}
                          disabled={channel.position === 0}
                        >
                          Move up
                        </button>
                        <button
                          type="button"
                          class="rounded-md px-2 py-1 text-left text-sm text-foreground hover:bg-muted disabled:cursor-not-allowed disabled:opacity-50"
                          role="menuitem"
                          onclick={() => void moveChannelWithinCategory(channel.slug, channel.categorySlug, 1)}
                          disabled={
                            channel.position >=
                            channelsForCategory(channel.categorySlug).length - 1
                          }
                        >
                          Move down
                        </button>
                      </div>
                    {/if}
                  </li>
                {/each}
              </ul>
            {/if}
          </section>
        {/each}

        <section
          class="group rounded-md"
          role="group"
          data-testid="uncategorized-section"
          ondragover={handleDragOver}
          ondrop={(event) => void handleDropOnCategory(event, null)}
        >
          <div class="mb-1 flex items-center gap-2 px-1">
            <span class="text-[11px] font-semibold uppercase tracking-wide text-muted-foreground">
              Uncategorized
            </span>
            {#if canManageChannels}
              <button
                type="button"
                class="inline-flex h-7 w-7 items-center justify-center rounded-md text-sm text-muted-foreground transition-colors hover:bg-muted hover:text-foreground"
                aria-label="Create channel in uncategorized"
                onclick={() => void openCreateDialog(null)}
              >
                +
              </button>
            {/if}
          </div>

          {#if uncategorizedChannels().length === 0}
            <p class="rounded-md bg-muted px-3 py-2 text-sm text-muted-foreground">
              No uncategorized channels.
            </p>
          {:else}
            <ul class="space-y-1" aria-label="Uncategorized channels">
              {#each uncategorizedChannels() as channel}
                <li
                  class="rounded-md"
                  draggable={canManageChannels}
                  ondragstart={(event) => handleDragStart(event, channel.slug)}
                  ondragover={handleDragOver}
                  ondrop={(event) => void handleDropOnChannel(event, channel)}
                  ondragend={handleDragEnd}
                  oncontextmenu={(event) => handleChannelContextMenu(event, channel.slug)}
                  data-testid={`channel-item-${channel.slug}`}
                >
                  <div class="flex items-center gap-1">
                    <a
                      class={`flex min-w-0 flex-1 items-center gap-2 rounded-md px-3 py-2 text-sm transition-colors ${
                        channel.slug === activeChannel
                          ? 'bg-sidebar-accent text-sidebar-accent-foreground'
                          : channel.hasUnreadActivity
                            ? 'text-foreground hover:bg-muted hover:text-foreground'
                          : 'text-muted-foreground hover:bg-muted hover:text-foreground'
                      }`}
                      href={`/${activeGuild}/${channel.slug}`}
                      use:routerLink
                      aria-label={`${channel.hasUnreadActivity ? 'Unread. ' : ''}Open channel ${channel.name}`}
                      aria-current={channel.slug === activeChannel ? 'page' : undefined}
                      data-has-unread-activity={channel.hasUnreadActivity
                        ? 'true'
                        : 'false'}
                      data-channel-nav="true"
                      onkeydown={(event) => handleChannelKeydown(event, channel.slug)}
                      data-testid={`channel-link-${channel.slug}`}
                    >
                      <span
                        class="inline-flex w-4 shrink-0 items-center justify-center text-xs"
                        aria-hidden="true"
                        data-testid={`channel-icon-${channel.slug}`}
                      >
                        {channel.channelType === 'voice' ? '🔊' : '#'}
                      </span>
                      <span
                        class={`truncate ${channel.hasUnreadActivity ? 'font-semibold text-foreground' : ''}`}
                      >
                        {channel.name}
                      </span>
                      {#if channel.hasUnreadActivity}
                        <span
                          class="ml-auto inline-flex h-2 w-2 shrink-0 rounded-full bg-sky-300"
                          aria-hidden="true"
                          data-testid={`channel-unread-dot-${channel.slug}`}
                        ></span>
                      {/if}
                    </a>

                    {#if canManageChannels}
                      <button
                        type="button"
                        class="inline-flex h-8 w-8 items-center justify-center rounded-md text-base text-muted-foreground transition-colors hover:bg-muted hover:text-foreground"
                        aria-label={`Open channel actions for ${channel.name}`}
                        onclick={() => toggleActionMenu(channel.slug)}
                      >
                        ⋯
                      </button>
                    {/if}
                  </div>

                  {#if canManageChannels && actionMenuSlug === channel.slug}
                    <div
                      class="mt-1 grid gap-1 rounded-md border border-border bg-card p-2"
                      role="menu"
                      aria-label={`Actions for ${channel.name}`}
                    >
                      <button
                        type="button"
                        class="rounded-md px-2 py-1 text-left text-sm text-foreground hover:bg-muted"
                        role="menuitem"
                        onclick={() => void openPermissionsDialog(channel)}
                      >
                        Permission overrides
                      </button>
                      <button
                        type="button"
                        class="rounded-md px-2 py-1 text-left text-sm text-foreground hover:bg-muted"
                        role="menuitem"
                        onclick={() => void openRenameDialog(channel)}
                      >
                        Edit channel
                      </button>
                      <button
                        type="button"
                        class="rounded-md px-2 py-1 text-left text-sm text-destructive hover:bg-destructive/10"
                        role="menuitem"
                        onclick={() => openDeleteDialog(channel)}
                      >
                        Delete channel
                      </button>
                      <button
                        type="button"
                        class="rounded-md px-2 py-1 text-left text-sm text-foreground hover:bg-muted disabled:cursor-not-allowed disabled:opacity-50"
                        role="menuitem"
                        onclick={() => void moveChannelWithinCategory(channel.slug, null, -1)}
                        disabled={channel.position === 0}
                      >
                        Move up
                      </button>
                      <button
                        type="button"
                        class="rounded-md px-2 py-1 text-left text-sm text-foreground hover:bg-muted disabled:cursor-not-allowed disabled:opacity-50"
                        role="menuitem"
                        onclick={() => void moveChannelWithinCategory(channel.slug, null, 1)}
                        disabled={channel.position >= uncategorizedChannels().length - 1}
                      >
                        Move down
                      </button>
                    </div>
                  {/if}
                </li>
              {/each}
            </ul>
          {/if}
        </section>
      </div>
    {/if}
  </div>
</aside>

{#if createDialogOpen}
  <div
    class="fixed inset-0 z-50 flex items-center justify-center bg-black/50 p-4"
    role="presentation"
    onclick={(event) => {
      if (event.target !== event.currentTarget) return
      void closeCreateDialog()
    }}
  >
    <div
      class="w-full max-w-md rounded-lg border border-border bg-card p-6 shadow-2xl"
      role="dialog"
      aria-modal="true"
      aria-label="Create channel"
    >
      <header class="mb-4 flex items-center justify-between">
        <h2 class="text-lg font-semibold">Create channel</h2>
        <button
          type="button"
          class="rounded-md bg-muted px-3 py-1 text-sm text-foreground hover:opacity-90"
          onclick={() => void closeCreateDialog()}
          aria-label="Close create channel dialog"
        >
          Close
        </button>
      </header>

      {#if createCategorySlug}
        <p class="mb-3 text-sm text-muted-foreground">
          New channel will be created in <strong>{createCategorySlug.toUpperCase()}</strong>.
        </p>
      {/if}

      {#if createError}
        <p
          class="mb-4 rounded-md border border-destructive/30 bg-destructive/10 p-3 text-sm text-destructive"
          role="alert"
        >
          {createError}
        </p>
      {/if}

      <form
        class="space-y-4"
        onsubmit={handleCreateSubmit}
        novalidate
        data-testid="create-channel-form"
      >
        <div class="space-y-1">
          <label for="channel-create-name" class="text-sm font-medium"
            >Channel name</label
          >
          <input
            bind:this={createNameInput}
            id="channel-create-name"
            type="text"
            class={`w-full rounded-md border bg-background px-3 py-2 text-base focus:outline-none focus:ring-2 ${
              createNameError
                ? 'border-destructive focus:ring-destructive'
                : 'border-input focus:ring-ring'
            }`}
            bind:value={createName}
            onblur={onCreateNameBlur}
            maxlength={64}
            required
          />
          {#if createNameError}
            <p class="text-sm text-destructive">{createNameError}</p>
          {/if}
        </div>

        <div class="space-y-1">
          <label for="channel-create-type" class="text-sm font-medium"
            >Channel type</label
          >
          <select
            id="channel-create-type"
            class="w-full rounded-md border border-input bg-background px-3 py-2 text-base focus:outline-none focus:ring-2 focus:ring-ring"
            bind:value={createType}
          >
            <option value="text">Text</option>
            <option value="voice">Voice</option>
          </select>
        </div>

        <button
          type="submit"
          class="inline-flex w-full items-center justify-center rounded-md bg-fire px-4 py-2 text-sm font-medium text-fire-foreground transition-opacity hover:opacity-90 disabled:cursor-not-allowed disabled:opacity-60"
          disabled={createSubmitting}
        >
          {createSubmitting ? 'Creating...' : 'Create channel'}
        </button>
      </form>
    </div>
  </div>
{/if}

{#if categoryCreateDialogOpen}
  <div
    class="fixed inset-0 z-50 flex items-center justify-center bg-black/50 p-4"
    role="presentation"
    onclick={(event) => {
      if (event.target !== event.currentTarget) return
      void closeCategoryCreateDialog()
    }}
  >
    <div
      class="w-full max-w-md rounded-lg border border-border bg-card p-6 shadow-2xl"
      role="dialog"
      aria-modal="true"
      aria-label="Create category"
    >
      <header class="mb-4 flex items-center justify-between">
        <h2 class="text-lg font-semibold">Create category</h2>
        <button
          type="button"
          class="rounded-md bg-muted px-3 py-1 text-sm text-foreground hover:opacity-90"
          onclick={() => void closeCategoryCreateDialog()}
          aria-label="Close create category dialog"
        >
          Close
        </button>
      </header>

      {#if categoryCreateError}
        <p
          class="mb-4 rounded-md border border-destructive/30 bg-destructive/10 p-3 text-sm text-destructive"
          role="alert"
        >
          {categoryCreateError}
        </p>
      {/if}

      <form class="space-y-4" onsubmit={handleCategoryCreateSubmit} novalidate>
        <div class="space-y-1">
          <label for="category-create-name" class="text-sm font-medium"
            >Category name</label
          >
          <input
            bind:this={categoryCreateNameInput}
            id="category-create-name"
            type="text"
            class={`w-full rounded-md border bg-background px-3 py-2 text-base focus:outline-none focus:ring-2 ${
              categoryCreateNameError
                ? 'border-destructive focus:ring-destructive'
                : 'border-input focus:ring-ring'
            }`}
            bind:value={categoryCreateName}
            onblur={onCategoryCreateNameBlur}
            maxlength={64}
            required
          />
          {#if categoryCreateNameError}
            <p class="text-sm text-destructive">{categoryCreateNameError}</p>
          {/if}
        </div>

        <button
          type="submit"
          class="inline-flex w-full items-center justify-center rounded-md bg-fire px-4 py-2 text-sm font-medium text-fire-foreground transition-opacity hover:opacity-90 disabled:cursor-not-allowed disabled:opacity-60"
          disabled={categoryCreateSubmitting}
        >
          {categoryCreateSubmitting ? 'Creating...' : 'Create category'}
        </button>
      </form>
    </div>
  </div>
{/if}

{#if renameDialogOpen && renameTarget}
  <div
    class="fixed inset-0 z-50 flex items-center justify-center bg-black/50 p-4"
    role="presentation"
    onclick={(event) => {
      if (event.target !== event.currentTarget) return
      closeRenameDialog()
    }}
  >
    <div
      class="w-full max-w-md rounded-lg border border-border bg-card p-6 shadow-2xl"
      role="dialog"
      aria-modal="true"
      aria-label="Edit channel"
    >
      <header class="mb-4 flex items-center justify-between">
        <h2 class="text-lg font-semibold">Edit channel</h2>
        <button
          type="button"
          class="rounded-md bg-muted px-3 py-1 text-sm text-foreground hover:opacity-90"
          onclick={closeRenameDialog}
          aria-label="Close edit channel dialog"
        >
          Close
        </button>
      </header>

      {#if renameError}
        <p
          class="mb-4 rounded-md border border-destructive/30 bg-destructive/10 p-3 text-sm text-destructive"
          role="alert"
        >
          {renameError}
        </p>
      {/if}

      <form
        class="space-y-4"
        onsubmit={handleRenameSubmit}
        novalidate
        data-testid="rename-channel-form"
      >
        <div class="space-y-1">
          <label for="channel-rename-name" class="text-sm font-medium"
            >Channel name</label
          >
          <input
            bind:this={renameInput}
            id="channel-rename-name"
            type="text"
            class={`w-full rounded-md border bg-background px-3 py-2 text-base focus:outline-none focus:ring-2 ${
              renameNameError
                ? 'border-destructive focus:ring-destructive'
                : 'border-input focus:ring-ring'
            }`}
            bind:value={renameName}
            onblur={onRenameNameBlur}
            maxlength={64}
            required
          />
          {#if renameNameError}
            <p class="text-sm text-destructive">{renameNameError}</p>
          {/if}
        </div>

        <button
          type="submit"
          class="inline-flex w-full items-center justify-center rounded-md bg-fire px-4 py-2 text-sm font-medium text-fire-foreground transition-opacity hover:opacity-90 disabled:cursor-not-allowed disabled:opacity-60"
          disabled={renameSubmitting}
        >
          {renameSubmitting ? 'Saving...' : 'Save channel'}
        </button>
      </form>
    </div>
  </div>
{/if}

{#if renameCategoryDialogOpen && renameCategoryTarget}
  <div
    class="fixed inset-0 z-50 flex items-center justify-center bg-black/50 p-4"
    role="presentation"
    onclick={(event) => {
      if (event.target !== event.currentTarget) return
      closeCategoryRenameDialog()
    }}
  >
    <div
      class="w-full max-w-md rounded-lg border border-border bg-card p-6 shadow-2xl"
      role="dialog"
      aria-modal="true"
      aria-label="Edit category"
    >
      <header class="mb-4 flex items-center justify-between">
        <h2 class="text-lg font-semibold">Edit category</h2>
        <button
          type="button"
          class="rounded-md bg-muted px-3 py-1 text-sm text-foreground hover:opacity-90"
          onclick={closeCategoryRenameDialog}
          aria-label="Close edit category dialog"
        >
          Close
        </button>
      </header>

      {#if renameCategoryError}
        <p
          class="mb-4 rounded-md border border-destructive/30 bg-destructive/10 p-3 text-sm text-destructive"
          role="alert"
        >
          {renameCategoryError}
        </p>
      {/if}

      <form class="space-y-4" onsubmit={handleCategoryRenameSubmit} novalidate>
        <div class="space-y-1">
          <label for="category-rename-name" class="text-sm font-medium"
            >Category name</label
          >
          <input
            bind:this={renameCategoryInput}
            id="category-rename-name"
            type="text"
            class={`w-full rounded-md border bg-background px-3 py-2 text-base focus:outline-none focus:ring-2 ${
              renameCategoryNameError
                ? 'border-destructive focus:ring-destructive'
                : 'border-input focus:ring-ring'
            }`}
            bind:value={renameCategoryName}
            onblur={onRenameCategoryNameBlur}
            maxlength={64}
            required
          />
          {#if renameCategoryNameError}
            <p class="text-sm text-destructive">{renameCategoryNameError}</p>
          {/if}
        </div>

        <button
          type="submit"
          class="inline-flex w-full items-center justify-center rounded-md bg-fire px-4 py-2 text-sm font-medium text-fire-foreground transition-opacity hover:opacity-90 disabled:cursor-not-allowed disabled:opacity-60"
          disabled={renameCategorySubmitting}
        >
          {renameCategorySubmitting ? 'Saving...' : 'Save category'}
        </button>
      </form>
    </div>
  </div>
{/if}

{#if deleteDialogOpen && deleteTarget}
  <div
    class="fixed inset-0 z-50 flex items-center justify-center bg-black/50 p-4"
    role="presentation"
    onclick={(event) => {
      if (event.target !== event.currentTarget) return
      closeDeleteDialog()
    }}
  >
    <div
      class="w-full max-w-md rounded-lg border border-border bg-card p-6 shadow-2xl"
      role="dialog"
      aria-modal="true"
      aria-label="Delete channel"
    >
      <header class="mb-3 flex items-center justify-between">
        <h2 class="text-lg font-semibold">Delete channel</h2>
        <button
          type="button"
          class="rounded-md bg-muted px-3 py-1 text-sm text-foreground hover:opacity-90"
          onclick={closeDeleteDialog}
          aria-label="Close delete channel dialog"
        >
          Close
        </button>
      </header>

      <p class="mb-3 text-sm text-foreground">
        You are about to delete <strong>{deleteTarget.name}</strong>.
      </p>
      <p class="mb-4 rounded-md border border-destructive/30 bg-destructive/10 p-3 text-sm text-destructive">
        This will permanently delete all messages in this channel
      </p>

      {#if deleteError}
        <p
          class="mb-4 rounded-md border border-destructive/30 bg-destructive/10 p-3 text-sm text-destructive"
          role="alert"
        >
          {deleteError}
        </p>
      {/if}

      <div class="grid gap-2 sm:grid-cols-2">
        <button
          type="button"
          class="inline-flex items-center justify-center rounded-md bg-muted px-4 py-2 text-sm font-medium text-foreground transition-opacity hover:opacity-90"
          onclick={closeDeleteDialog}
          disabled={deleteSubmitting}
        >
          Cancel
        </button>
        <button
          type="button"
          class="inline-flex items-center justify-center rounded-md bg-destructive px-4 py-2 text-sm font-medium text-destructive-foreground transition-opacity hover:opacity-90 disabled:cursor-not-allowed disabled:opacity-60"
          onclick={() => void confirmDeleteChannel()}
          disabled={deleteSubmitting}
        >
          {deleteSubmitting ? 'Deleting...' : 'Delete channel'}
        </button>
      </div>
    </div>
  </div>
{/if}

{#if deleteCategoryDialogOpen && deleteCategoryTarget}
  <div
    class="fixed inset-0 z-50 flex items-center justify-center bg-black/50 p-4"
    role="presentation"
    onclick={(event) => {
      if (event.target !== event.currentTarget) return
      closeDeleteCategoryDialog()
    }}
  >
    <div
      class="w-full max-w-md rounded-lg border border-border bg-card p-6 shadow-2xl"
      role="dialog"
      aria-modal="true"
      aria-label="Delete category"
    >
      <header class="mb-3 flex items-center justify-between">
        <h2 class="text-lg font-semibold">Delete category</h2>
        <button
          type="button"
          class="rounded-md bg-muted px-3 py-1 text-sm text-foreground hover:opacity-90"
          onclick={closeDeleteCategoryDialog}
          aria-label="Close delete category dialog"
        >
          Close
        </button>
      </header>

      <p class="mb-3 text-sm text-foreground">
        You are about to delete <strong>{deleteCategoryTarget.name}</strong>.
      </p>
      <p class="mb-4 rounded-md border border-warning/30 bg-warning/10 p-3 text-sm text-foreground">
        Channels in this category will be moved to Uncategorized. No channels or messages are deleted.
      </p>

      {#if deleteCategoryError}
        <p
          class="mb-4 rounded-md border border-destructive/30 bg-destructive/10 p-3 text-sm text-destructive"
          role="alert"
        >
          {deleteCategoryError}
        </p>
      {/if}

      <div class="grid gap-2 sm:grid-cols-2">
        <button
          type="button"
          class="inline-flex items-center justify-center rounded-md bg-muted px-4 py-2 text-sm font-medium text-foreground transition-opacity hover:opacity-90"
          onclick={closeDeleteCategoryDialog}
          disabled={deleteCategorySubmitting}
        >
          Cancel
        </button>
        <button
          type="button"
          class="inline-flex items-center justify-center rounded-md bg-destructive px-4 py-2 text-sm font-medium text-destructive-foreground transition-opacity hover:opacity-90 disabled:cursor-not-allowed disabled:opacity-60"
          onclick={() => void confirmDeleteCategory()}
          disabled={deleteCategorySubmitting}
        >
          {deleteCategorySubmitting ? 'Deleting...' : 'Delete category'}
        </button>
      </div>
    </div>
  </div>
{/if}

{#if permissionsDialogOpen && permissionsTarget}
  <div
    class="fixed inset-0 z-50 flex items-center justify-center bg-black/50 p-4"
    role="presentation"
    onclick={(event) => {
      if (event.target !== event.currentTarget) return
      closePermissionsDialog()
    }}
  >
    <div
      class="max-h-[85vh] w-full max-w-4xl overflow-y-auto rounded-lg border border-border bg-card p-6 shadow-2xl"
      role="dialog"
      aria-modal="true"
      aria-label="Channel permissions"
    >
      <header class="mb-4 flex items-center justify-between gap-2">
        <div>
          <h2 class="text-lg font-semibold">Permission Overrides</h2>
          <p class="text-sm text-muted-foreground">
            Channel: <strong>{permissionsTarget.name}</strong>
          </p>
        </div>
        <button
          type="button"
          class="rounded-md bg-muted px-3 py-1 text-sm text-foreground hover:opacity-90"
          onclick={closePermissionsDialog}
          aria-label="Close channel permissions dialog"
        >
          Close
        </button>
      </header>

      {#if permissionsError}
        <p
          class="mb-4 rounded-md border border-destructive/30 bg-destructive/10 p-3 text-sm text-destructive"
          role="alert"
        >
          {permissionsError}
        </p>
      {/if}

      {#if permissionsLoading}
        <p class="rounded-md bg-muted px-3 py-2 text-sm text-muted-foreground">
          Loading permission overrides...
        </p>
      {:else if permissionRoles.length === 0}
        <p class="rounded-md bg-muted px-3 py-2 text-sm text-muted-foreground">
          No roles available for channel permission overrides.
        </p>
      {:else}
        <div class="grid gap-4 md:grid-cols-[220px,1fr]">
          <section aria-label="Roles" class="rounded-md border border-border p-3">
            <h3 class="mb-2 text-sm font-semibold">Roles</h3>
            <div class="grid gap-1">
              {#each permissionRoles as role}
                <button
                  type="button"
                  class={`rounded-md px-2 py-1 text-left text-sm transition-colors ${
                    selectedPermissionRoleId === role.id
                      ? 'bg-sidebar-accent text-sidebar-accent-foreground'
                      : 'text-foreground hover:bg-muted'
                  }`}
                  aria-pressed={selectedPermissionRoleId === role.id}
                  onclick={() => {
                    selectedPermissionRoleId = role.id
                  }}
                >
                  {role.name}
                </button>
              {/each}
            </div>
          </section>

          <section
            aria-label="Permission overrides"
            class="rounded-md border border-border p-3"
          >
            {#if selectedPermissionRoleEntry}
              {@const selectedRole = selectedPermissionRoleEntry}
              <h3 class="mb-3 text-sm font-semibold">
                Permissions for {selectedRole.name}
              </h3>
              <div class="space-y-3">
                {#each GUILD_PERMISSION_CATALOG as permission}
                  {@const currentState = channelPermissionState(
                    selectedRole.id,
                    permission.mask,
                  )}
                  {@const saving = Boolean(
                    permissionSavingByKey[
                      permissionCellKey(selectedRole.id, permission.mask)
                    ],
                  )}
                  <section class="rounded-md border border-border p-2">
                    <p class="text-sm font-medium text-foreground">{permission.label}</p>
                    <p class="mb-2 text-xs text-muted-foreground">
                      {permission.description}
                    </p>
                    <fieldset class="flex flex-wrap gap-3">
                      <legend class="sr-only">
                        Override state for {permission.label}
                      </legend>
                      <label class="inline-flex items-center gap-1 text-sm">
                        <input
                          type="radio"
                          name={`permission-${selectedRole.id}-${permission.key}`}
                          checked={currentState === 'allow'}
                          onchange={() =>
                            void setPermissionOverrideState(
                              selectedRole.id,
                              permission.mask,
                              'allow',
                            )}
                          disabled={saving}
                          aria-label={`Allow ${permission.label} for ${selectedRole.name}`}
                        />
                        Allow
                      </label>
                      <label class="inline-flex items-center gap-1 text-sm">
                        <input
                          type="radio"
                          name={`permission-${selectedRole.id}-${permission.key}`}
                          checked={currentState === 'deny'}
                          onchange={() =>
                            void setPermissionOverrideState(
                              selectedRole.id,
                              permission.mask,
                              'deny',
                            )}
                          disabled={saving}
                          aria-label={`Deny ${permission.label} for ${selectedRole.name}`}
                        />
                        Deny
                      </label>
                      <label class="inline-flex items-center gap-1 text-sm">
                        <input
                          type="radio"
                          name={`permission-${selectedRole.id}-${permission.key}`}
                          checked={currentState === 'inherit'}
                          onchange={() =>
                            void setPermissionOverrideState(
                              selectedRole.id,
                              permission.mask,
                              'inherit',
                            )}
                          disabled={saving}
                          aria-label={`Inherit ${permission.label} for ${selectedRole.name}`}
                        />
                        Inherit
                      </label>
                    </fieldset>
                  </section>
                {/each}
              </div>
            {/if}
          </section>
        </div>
      {/if}
    </div>
  </div>
{/if}
