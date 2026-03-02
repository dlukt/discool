<script lang="ts">
// biome-ignore-all lint/correctness/noUnusedVariables: Svelte template usage isn't detected reliably.
// biome-ignore-all lint/correctness/noUnusedImports: Svelte template usage isn't detected reliably.
import { onMount, tick } from 'svelte'
import AdminPanel from '$lib/components/AdminPanel.svelte'
import { guildState } from '$lib/features/guild/guildStore.svelte'
import {
  GUILD_PERMISSION_CATALOG,
  hasGuildPermission,
} from '$lib/features/guild/permissions'
import { blockState } from '$lib/features/identity/blockStore.svelte'
import { identityState } from '$lib/features/identity/identityStore.svelte'
import ProfileSettingsView from '$lib/features/identity/ProfileSettingsView.svelte'
import {
  createMessageDelete,
  createVoiceKick,
  type MuteStatus,
} from '$lib/features/moderation/moderationApi'
import { muteStatusState } from '$lib/features/moderation/muteStatusStore.svelte'
import type { VoiceParticipant } from '$lib/features/voice/types'
import VoiceBar from '$lib/features/voice/VoiceBar.svelte'
import VoicePanel from '$lib/features/voice/VoicePanel.svelte'
import {
  VOICE_CONNECTION_LOST_MESSAGE,
  voiceState,
} from '$lib/features/voice/voiceStore.svelte'
import { toastState } from '$lib/feedback/toastStore.svelte'
import {
  connectionStatusText,
  PERMISSION_DENIED_MESSAGE,
  parseWsErrorEnvelope,
  toUserFacingError,
} from '$lib/feedback/userFacingError'
import { wsClient } from '$lib/ws/client'
import type { WsLifecycleState } from '$lib/ws/protocol'
import MessageBubble from './MessageBubble.svelte'
import { type ChatAuthorInput, messageState } from './messageStore.svelte'
import type { ChatMessage } from './types'

type Props = {
  mode: 'home' | 'channel' | 'dm' | 'settings' | 'admin'
  activeGuild: string
  activeChannel: string
  activeDm?: string | null
  displayName: string
  isAdmin: boolean
  showRecoveryNudge: boolean
  showVoiceControls?: boolean
  onOpenSettings?: () => void | Promise<void>
  onDismissRecoveryNudge?: () => void | Promise<void>
}

type VirtualTimelineRow = {
  id: string
  top: number
  height: number
  compact: boolean
  message: ChatMessage
}

type ComposerEditState = {
  messageId: string
}

const MESSAGE_ROW_HEIGHT = 74
const COMPACT_MESSAGE_ROW_HEIGHT = 46
const SYSTEM_ROW_HEIGHT = 36
const IMAGE_ATTACHMENT_HEIGHT = 220
const FILE_ATTACHMENT_ROW_HEIGHT = 60
const EMBED_CARD_HEIGHT = 146
const ATTACHMENT_ROW_GAP = 8
const MARKDOWN_LINE_HEIGHT = 18
const VIRTUAL_OVERSCAN_PX = 320
const HISTORY_LOAD_TRIGGER_PX = 120
const JUMP_TO_PRESENT_THRESHOLD_PX = 320
const HISTORY_SKELETON_COUNT = 4
const TYPING_START_THROTTLE_MS = 1_500
const OPERATION_STATUS_AUTO_CLEAR_MS = 1_500
const MAX_TIMEOUT_MS = 2_147_483_647
const MAX_MODERATION_REASON_CHARS = 500

let {
  mode,
  activeGuild,
  activeChannel,
  activeDm,
  displayName,
  isAdmin,
  showRecoveryNudge,
  showVoiceControls = true,
  onOpenSettings,
  onDismissRecoveryNudge,
}: Props = $props()

let detailText = $derived(
  mode === 'channel'
    ? `#${activeChannel} in ${activeGuild}`
    : mode === 'dm' && activeDm
      ? `Direct message (${activeDm})`
      : `Signed in as ${displayName}.`,
)
let isChannelMode = $derived(mode === 'channel')
let isDmMode = $derived(mode === 'dm')
let isConversationMode = $derived(isChannelMode || isDmMode)
let canShowAdminPanel = $derived(mode === 'admin' && isAdmin)
let shouldShowRecoveryNudge = $derived(
  showRecoveryNudge && (mode === 'home' || mode === 'channel' || mode === 'dm'),
)
let wsLifecycleState = $state<WsLifecycleState>(wsClient.getLifecycleState())
let connectionStatusMessage = $derived(connectionStatusText(wsLifecycleState))
let voiceStatusMessage = $derived(
  isChannelMode
    ? voiceState.statusMessageForChannel(activeGuild, activeChannel)
    : null,
)
let voiceConnectionState = $derived(
  isChannelMode
    ? voiceState.statusForChannel(activeGuild, activeChannel)
    : 'idle',
)
let participantsOpen = $state(false)
let voiceControlsEnabled = $derived(showVoiceControls)
let showVoiceBar = $derived(
  voiceControlsEnabled &&
    isChannelMode &&
    (voiceConnectionState === 'connected' ||
      voiceConnectionState === 'retrying' ||
      (voiceConnectionState === 'failed' &&
        voiceStatusMessage === VOICE_CONNECTION_LOST_MESSAGE)),
)
let activeVoiceParticipants = $derived(
  isChannelMode ? voiceState.activeChannelParticipants() : [],
)
let showVoicePanel = $derived(showVoiceBar && participantsOpen)
let composerInput = $state<HTMLTextAreaElement | null>(null)
let composerValue = $state('')
let composerSelection = $state({ start: 0, end: 0 })
let composerEdit = $state<ComposerEditState | null>(null)
let timelineViewport = $state<HTMLDivElement | null>(null)
let scrollTop = $state(0)
let viewportHeight = $state(320)
let distanceFromBottomPx = $state(0)
let loadHistoryInFlight = $state(false)
let lastTailMessageId = $state<string | null>(null)
let skipNextTailChange = $state(false)
let pendingDeleteMessage = $state<ChatMessage | null>(null)
let pendingDeleteRequiresReason = $state(false)
let deleteReason = $state('')
let deleteError = $state<string | null>(null)
let deleteSubmitting = $state(false)
let attachmentInput = $state<HTMLInputElement | null>(null)
let selectedAttachment = $state<File | null>(null)
let attachmentUploadProgress = $state<number | null>(null)
let attachmentUploadInFlight = $state(false)
let attachmentError = $state<string | null>(null)
let dragDepth = $state(0)
let dragActive = $state(false)
let attachmentContextKey = $state<string | null>(null)
let lastTypingStartSentAt = $state(0)
let typingStartChannelKey = $state<string | null>(null)
let restoringTimelineKey = $state<string | null>(null)
let operationStatusText = $state<string | null>(null)
let operationStatusTone = $state<'default' | 'error'>('default')
let operationStatusResetTimer: ReturnType<typeof setTimeout> | null = null
let muteStatusRefreshTimer: ReturnType<typeof setTimeout> | null = null
let voiceKickDialogParticipant = $state<VoiceParticipant | null>(null)
let voiceKickReason = $state('')
let voiceKickSubmitting = $state(false)
let voiceKickError = $state<string | null>(null)

let channelKey = $derived(
  mode === 'channel'
    ? `${activeGuild}:${activeChannel}`
    : mode === 'dm' && activeDm
      ? `dm:${activeDm}`
      : null,
)
let activeDmSlug = $derived(mode === 'dm' ? activeDm?.trim() || null : null)

$effect(() => {
  if (showVoiceBar) return
  participantsOpen = false
})

$effect(() => {
  void voiceState.initializeParticipantVolumes(currentSessionUserId)
})

let timelineMessages = $derived.by(() => {
  const _messageVersion = messageState.version
  const _blockVersion = blockState.version
  void _messageVersion
  void _blockVersion
  if (isChannelMode) return messageState.timeline(activeGuild, activeChannel)
  if (isDmMode && activeDmSlug) return messageState.timelineForDm(activeDmSlug)
  return []
})

let channelHistory = $derived.by(() => {
  const _messageVersion = messageState.version
  const _blockVersion = blockState.version
  void _messageVersion
  void _blockVersion
  if (!isConversationMode) {
    return {
      initialized: false,
      loadingHistory: false,
      hasMoreHistory: false,
      cursor: null,
      scrollTop: 0,
      pendingNewCount: 0,
    }
  }
  if (isChannelMode) {
    return messageState.historyStateForChannel(activeGuild, activeChannel)
  }
  if (isDmMode && activeDmSlug) {
    return messageState.historyStateForDm(activeDmSlug)
  }
  return {
    initialized: false,
    loadingHistory: false,
    hasMoreHistory: false,
    cursor: null,
    scrollTop: 0,
    pendingNewCount: 0,
  }
})

function estimateMessageRowHeight(
  message: ChatMessage,
  compact: boolean,
): number {
  if (message.isSystem) return SYSTEM_ROW_HEIGHT

  const base = compact ? COMPACT_MESSAGE_ROW_HEIGHT : MESSAGE_ROW_HEIGHT
  let extra = 0

  const contentLineEstimate = message.content
    .split('\n')
    .reduce(
      (lines, segment) => lines + Math.max(1, Math.ceil(segment.length / 90)),
      0,
    )
  if (contentLineEstimate > 2) {
    extra += (contentLineEstimate - 2) * MARKDOWN_LINE_HEIGHT
  }
  if (message.content.includes('```')) {
    extra += 96
  }
  if (message.content.includes('\n>') || message.content.startsWith('&gt;')) {
    extra += 20
  }
  if (message.embeds.length > 0) {
    extra += message.embeds.length * EMBED_CARD_HEIGHT
    extra += Math.max(0, message.embeds.length - 1) * ATTACHMENT_ROW_GAP
    extra += 12
  }

  const imageCount = message.attachments.filter(
    (attachment) => attachment.isImage,
  ).length
  const fileCount = message.attachments.length - imageCount

  if (imageCount > 0) {
    extra += imageCount * IMAGE_ATTACHMENT_HEIGHT
    extra += Math.max(0, imageCount - 1) * ATTACHMENT_ROW_GAP
    extra += 12
  }
  if (fileCount > 0) {
    extra += fileCount * FILE_ATTACHMENT_ROW_HEIGHT
    extra += Math.max(0, fileCount - 1) * ATTACHMENT_ROW_GAP
    extra += 12
  }

  return base + extra
}

let virtualRows = $derived.by(() => {
  let top = 0
  const rows: VirtualTimelineRow[] = []

  for (const [index, message] of timelineMessages.entries()) {
    const previous = index > 0 ? timelineMessages[index - 1] : null
    const compact =
      Boolean(
        previous &&
          !previous.isSystem &&
          !message.isSystem &&
          previous.authorUserId === message.authorUserId &&
          message.updatedAt === message.createdAt,
      ) && !message.isSystem
    const height = estimateMessageRowHeight(message, compact)

    rows.push({
      id: message.id,
      top,
      height,
      compact,
      message,
    })
    top += height
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

let currentSessionUser = $derived(identityState.session?.user ?? null)
let currentSessionUserId = $derived(currentSessionUser?.id ?? null)
let activeGuildRecord = $derived(
  isChannelMode ? guildState.bySlug(activeGuild) : null,
)
const attachFilesPermission = GUILD_PERMISSION_CATALOG.find(
  (permission) => permission.key === 'ATTACH_FILES',
)
const muteMembersPermission = GUILD_PERMISSION_CATALOG.find(
  (permission) => permission.key === 'MUTE_MEMBERS',
)
const manageMessagesPermission = GUILD_PERMISSION_CATALOG.find(
  (permission) => permission.key === 'MANAGE_MESSAGES',
)
let memberRoleData = $derived(
  isChannelMode
    ? guildState.memberRoleDataForGuild(activeGuild)
    : {
        members: [],
        roles: [],
        assignableRoleIds: [],
        canManageRoles: false,
      },
)
let currentMember = $derived(
  currentSessionUser
    ? (memberRoleData.members.find(
        (member) => member.userId === currentSessionUser.id,
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
let activeMuteStatus = $derived(
  isChannelMode ? muteStatusState.statusForGuild(activeGuild) : null,
)
let isMuted = $derived(Boolean(activeMuteStatus?.active))
let muteStatusMessage = $derived(
  isChannelMode ? buildMuteStatusMessage(activeMuteStatus) : null,
)
let canAttachFiles = $derived(
  isChannelMode &&
    !isMuted &&
    (Boolean(activeGuildRecord?.isOwner) ||
      (attachFilesPermission !== undefined &&
        hasGuildPermission(
          currentMemberPermissionsBitflag,
          attachFilesPermission,
        ))),
)
let canModerateVoiceParticipants = $derived(
  isChannelMode &&
    (Boolean(activeGuildRecord?.isOwner) ||
      (muteMembersPermission !== undefined &&
        hasGuildPermission(
          currentMemberPermissionsBitflag,
          muteMembersPermission,
        ))),
)
let canModerateMessages = $derived(
  isChannelMode &&
    (Boolean(activeGuildRecord?.isOwner) ||
      (manageMessagesPermission !== undefined &&
        hasGuildPermission(
          currentMemberPermissionsBitflag,
          manageMessagesPermission,
        ))),
)
let currentRoleColor = $derived(
  isChannelMode
    ? (currentMember?.highestRoleColor ??
        currentSessionUser?.avatarColor ??
        '#99aab5')
    : (currentSessionUser?.avatarColor ?? '#99aab5'),
)
let canSubmitComposer = $derived(
  Boolean(currentSessionUser) &&
    !attachmentUploadInFlight &&
    !isMuted &&
    (composerEdit
      ? composerValue.trim().length > 0
      : composerValue.trim().length > 0 || selectedAttachment !== null),
)
let hasComposerSelection = $derived(
  composerSelection.end > composerSelection.start,
)
let emptyStateCopy = $derived(
  isChannelMode
    ? `This is the beginning of #${activeChannel}. Say something!`
    : 'This is the beginning of your direct conversation. Say something!',
)

let typingUserDisplayNames = $derived.by(() => {
  const _messageVersion = messageState.version
  const _blockVersion = blockState.version
  void _messageVersion
  void _blockVersion
  if (!isChannelMode) return []
  const typingUserIds = messageState.typingUserIdsForChannel(
    activeGuild,
    activeChannel,
  )
  if (typingUserIds.length === 0) return []
  const memberNameById = new Map(
    memberRoleData.members.map((member) => [
      member.userId,
      member.displayName || member.username,
    ]),
  )
  return typingUserIds.map((userId) => memberNameById.get(userId) ?? 'Someone')
})

let typingIndicatorText = $derived.by(() => {
  if (typingUserDisplayNames.length === 0) return null
  if (typingUserDisplayNames.length === 1) {
    return `${typingUserDisplayNames[0]} is typing...`
  }
  if (typingUserDisplayNames.length === 2) {
    return `${typingUserDisplayNames[0]} and ${typingUserDisplayNames[1]} are typing...`
  }
  if (typingUserDisplayNames.length === 3) {
    return `${typingUserDisplayNames[0]}, ${typingUserDisplayNames[1]}, and ${typingUserDisplayNames[2]} are typing...`
  }
  return 'Several people are typing...'
})

let showJumpToPresent = $derived(
  isConversationMode &&
    (distanceFromBottomPx > JUMP_TO_PRESENT_THRESHOLD_PX ||
      channelHistory.pendingNewCount > 0),
)

async function handleOpenSettings() {
  await onOpenSettings?.()
}

async function handleDismissRecoveryNudge() {
  await onDismissRecoveryNudge?.()
}

function buildCurrentAuthor(): ChatAuthorInput | null {
  if (!currentSessionUser) return null
  return {
    userId: currentSessionUser.id,
    username: currentSessionUser.username,
    displayName: currentSessionUser.displayName,
    avatarColor: currentSessionUser.avatarColor,
    roleColor: currentRoleColor,
  }
}

function clearOperationStatusTimer(): void {
  if (operationStatusResetTimer === null) return
  clearTimeout(operationStatusResetTimer)
  operationStatusResetTimer = null
}

function clearMuteStatusRefreshTimer(): void {
  if (muteStatusRefreshTimer === null) return
  clearTimeout(muteStatusRefreshTimer)
  muteStatusRefreshTimer = null
}

function setOperationStatus(
  text: string,
  tone: 'default' | 'error' = 'default',
  autoClearMs?: number,
): void {
  operationStatusText = text
  operationStatusTone = tone
  clearOperationStatusTimer()
  if (typeof autoClearMs === 'number' && autoClearMs > 0) {
    operationStatusResetTimer = setTimeout(() => {
      operationStatusText = null
      operationStatusTone = 'default'
      operationStatusResetTimer = null
    }, autoClearMs)
  }
}

function messageFromError(error: unknown, fallback: string): string {
  return toUserFacingError(error, fallback).message
}

function formatMuteExpiry(value: string | null): string {
  if (!value) return 'an unknown time'
  const parsed = Date.parse(value)
  if (!Number.isFinite(parsed)) return value
  return new Date(parsed).toLocaleString()
}

function buildMuteStatusMessage(status: MuteStatus | null): string | null {
  if (!status?.active) return null
  const reason = status.reason?.trim()
  if (status.isPermanent) {
    return reason
      ? `You are muted in this guild permanently. Reason: ${reason}`
      : 'You are muted in this guild permanently.'
  }
  const expiry = formatMuteExpiry(status.expiresAt)
  return reason
    ? `You are muted in this guild until ${expiry}. Reason: ${reason}`
    : `You are muted in this guild until ${expiry}.`
}

function formatFileSize(sizeBytes: number): string {
  if (!Number.isFinite(sizeBytes) || sizeBytes <= 0) return '0 B'
  if (sizeBytes < 1024) return `${Math.round(sizeBytes)} B`
  if (sizeBytes < 1024 * 1024) return `${(sizeBytes / 1024).toFixed(1)} KB`
  return `${(sizeBytes / (1024 * 1024)).toFixed(1)} MB`
}

function clearSelectedAttachment(clearInput = true): void {
  selectedAttachment = null
  attachmentUploadProgress = null
  dragDepth = 0
  dragActive = false
  if (clearInput && attachmentInput) {
    attachmentInput.value = ''
  }
}

function selectAttachment(file: File): void {
  selectedAttachment = file
  attachmentError = null
}

function updateComposerSelection(target: HTMLTextAreaElement | null): void {
  if (!target) {
    composerSelection = { start: 0, end: 0 }
    return
  }
  const start = Math.max(0, target.selectionStart ?? 0)
  const end = Math.max(0, target.selectionEnd ?? start)
  composerSelection = { start, end }
}

function resetTypingStartThrottle(): void {
  lastTypingStartSentAt = 0
  typingStartChannelKey = null
}

function maybeEmitTypingStart(rawValue: string): void {
  if (mode !== 'channel' || composerEdit) return
  if (isMuted) return
  if (!rawValue.trim()) return
  const nextChannelKey = `${activeGuild}:${activeChannel}`
  if (typingStartChannelKey !== nextChannelKey) {
    typingStartChannelKey = nextChannelKey
    lastTypingStartSentAt = 0
  }
  const now = Date.now()
  if (now - lastTypingStartSentAt < TYPING_START_THROTTLE_MS) return
  const sent = messageState.sendTypingStart(activeGuild, activeChannel)
  if (!sent) return
  typingStartChannelKey = nextChannelKey
  lastTypingStartSentAt = now
}

function handleComposerInput(event: Event): void {
  const target = event.currentTarget as HTMLTextAreaElement | null
  updateComposerSelection(target)
  maybeEmitTypingStart(target?.value ?? composerValue)
}

function applyMarkdownWrap(prefix: string, suffix = prefix): void {
  const target = composerInput
  if (!target) return
  const start = target.selectionStart ?? 0
  const end = target.selectionEnd ?? start
  if (end <= start) return

  const selected = composerValue.slice(start, end)
  composerValue = `${composerValue.slice(0, start)}${prefix}${selected}${suffix}${composerValue.slice(end)}`

  const nextStart = start + prefix.length
  const nextEnd = nextStart + selected.length
  void tick().then(() => {
    if (!composerInput) return
    composerInput.focus()
    composerInput.selectionStart = nextStart
    composerInput.selectionEnd = nextEnd
    updateComposerSelection(composerInput)
  })
}

function formatSelectionBold(): void {
  applyMarkdownWrap('**')
}

function formatSelectionItalic(): void {
  applyMarkdownWrap('*')
}

function formatSelectionCode(): void {
  applyMarkdownWrap('`')
}

function openAttachmentPicker(): void {
  if (!canAttachFiles || attachmentUploadInFlight || isMuted) return
  attachmentInput?.click()
}

function handleAttachmentInputChange(event: Event): void {
  if (isMuted) return
  const target = event.currentTarget as HTMLInputElement | null
  const nextFile = target?.files?.[0] ?? null
  if (!nextFile) return
  selectAttachment(nextFile)
}

function eventHasFiles(event: DragEvent): boolean {
  const types = event.dataTransfer?.types
  if (!types) return false
  return Array.from(types).includes('Files')
}

function handleTimelineDragEnter(event: DragEvent): void {
  if (mode !== 'channel' || !canAttachFiles || isMuted || !eventHasFiles(event))
    return
  event.preventDefault()
  dragDepth += 1
  dragActive = true
}

function handleTimelineDragOver(event: DragEvent): void {
  if (mode !== 'channel' || !canAttachFiles || isMuted || !eventHasFiles(event))
    return
  event.preventDefault()
  if (event.dataTransfer) {
    event.dataTransfer.dropEffect = 'copy'
  }
  dragActive = true
}

function handleTimelineDragLeave(event: DragEvent): void {
  if (mode !== 'channel' || !canAttachFiles || isMuted || !eventHasFiles(event))
    return
  event.preventDefault()
  dragDepth = Math.max(0, dragDepth - 1)
  if (dragDepth === 0) {
    dragActive = false
  }
}

function handleTimelineDrop(event: DragEvent): void {
  if (mode !== 'channel' || !canAttachFiles || isMuted) return
  event.preventDefault()
  dragDepth = 0
  dragActive = false
  const dropped = event.dataTransfer?.files?.[0] ?? null
  if (!dropped) return
  selectAttachment(dropped)
}

async function sendComposerMessage() {
  const author = buildCurrentAuthor()
  if (!author || !isConversationMode) return
  attachmentError = null
  if (isChannelMode && isMuted && muteStatusMessage) {
    setOperationStatus(muteStatusMessage, 'error')
    return
  }

  if (composerEdit && isChannelMode) {
    setOperationStatus('Saving...')
    const sent = messageState.sendMessageUpdate(
      activeGuild,
      activeChannel,
      composerEdit.messageId,
      composerValue,
    )
    if (sent) {
      composerValue = ''
      composerEdit = null
      composerSelection = { start: 0, end: 0 }
      resetTypingStartThrottle()
      setOperationStatus('Saved.', 'default', OPERATION_STATUS_AUTO_CLEAR_MS)
    } else {
      const message =
        connectionStatusText(wsLifecycleState) ?? 'Could not save message.'
      setOperationStatus(message, 'error')
      toastState.show({ variant: 'error', message })
    }
    return
  }

  if (selectedAttachment && isChannelMode) {
    if (!canAttachFiles) {
      attachmentError = PERMISSION_DENIED_MESSAGE
      setOperationStatus(PERMISSION_DENIED_MESSAGE, 'error')
      return
    }
    setOperationStatus('Uploading attachment...')
    attachmentUploadInFlight = true
    attachmentUploadProgress = 0
    try {
      await messageState.uploadAttachment(activeGuild, activeChannel, {
        file: selectedAttachment,
        content: composerValue,
        onProgress: (percentage) => {
          attachmentUploadProgress = Math.max(0, Math.min(100, percentage))
        },
      })
      composerValue = ''
      composerSelection = { start: 0, end: 0 }
      clearSelectedAttachment()
      resetTypingStartThrottle()
      setOperationStatus(
        'Upload complete.',
        'default',
        OPERATION_STATUS_AUTO_CLEAR_MS,
      )
    } catch (error) {
      attachmentError = messageFromError(error, 'Failed to upload attachment')
      setOperationStatus(attachmentError, 'error')
      toastState.show({ variant: 'error', message: attachmentError })
    } finally {
      attachmentUploadInFlight = false
      attachmentUploadProgress = null
    }
    return
  }

  setOperationStatus(composerEdit ? 'Saving...' : 'Sending...')
  const contentToSend = composerValue
  const sendChannel = activeGuild
  const sendConversation = activeChannel
  const sendDm = activeDmSlug

  const sent = isChannelMode
    ? messageState.sendMessage(
        sendChannel,
        sendConversation,
        contentToSend,
        author,
      )
    : sendDm
      ? messageState.sendDmMessage(sendDm, contentToSend, author)
      : false

  if (sent) {
    if (composerValue === contentToSend) {
      composerValue = ''
    }
    composerSelection = { start: 0, end: 0 }
    resetTypingStartThrottle()
    setOperationStatus('Sent.', 'default', OPERATION_STATUS_AUTO_CLEAR_MS)
    return
  }

  const sendFailureMessage =
    connectionStatusText(wsLifecycleState) ?? 'Message failed to send.'
  setOperationStatus(sendFailureMessage, 'error')
  toastState.show({
    variant: 'error',
    message: sendFailureMessage,
    actionLabel: 'Retry?',
    onAction: () => {
      const retried = isChannelMode
        ? messageState.sendMessage(
            sendChannel,
            sendConversation,
            contentToSend,
            author,
          )
        : sendDm
          ? messageState.sendDmMessage(sendDm, contentToSend, author)
          : false
      if (!retried) return
      if (composerValue === contentToSend) {
        composerValue = ''
      }
      composerSelection = { start: 0, end: 0 }
      resetTypingStartThrottle()
      setOperationStatus('Sent.', 'default', OPERATION_STATUS_AUTO_CLEAR_MS)
    },
  })
}

function findLatestOwnMessage(): ChatMessage | null {
  const currentUserId = currentSessionUser?.id
  if (!currentUserId || mode !== 'channel') return null
  for (let index = timelineMessages.length - 1; index >= 0; index -= 1) {
    const message = timelineMessages[index]
    if (!message || message.isSystem) continue
    if (message.authorUserId !== currentUserId) continue
    return message
  }
  return null
}

function startEditingMessage(message: ChatMessage): void {
  const currentUserId = currentSessionUser?.id
  if (!currentUserId || mode !== 'channel') return
  if (message.isSystem || message.authorUserId !== currentUserId) return
  if (
    message.guildSlug !== activeGuild ||
    message.channelSlug !== activeChannel
  )
    return
  clearSelectedAttachment()
  attachmentError = null
  composerEdit = { messageId: message.id }
  composerValue = message.content
  void tick().then(() => {
    if (!composerInput) return
    composerInput.focus()
    const end = composerValue.length
    composerInput.selectionStart = end
    composerInput.selectionEnd = end
    updateComposerSelection(composerInput)
  })
}

function cancelComposerEdit(): void {
  composerEdit = null
  composerValue = ''
  composerSelection = { start: 0, end: 0 }
  resetTypingStartThrottle()
  composerInput?.focus()
}

function requestDeleteMessage(message: ChatMessage): void {
  const currentUserId = currentSessionUser?.id
  if (!currentUserId || mode !== 'channel') return
  if (message.isSystem) return
  const isOwnMessage = message.authorUserId === currentUserId
  if (!isOwnMessage && !canModerateMessages) return
  pendingDeleteMessage = message
  pendingDeleteRequiresReason = !isOwnMessage
  deleteReason = ''
  deleteError = null
}

function closeDeleteDialog(): void {
  if (deleteSubmitting) return
  pendingDeleteMessage = null
  pendingDeleteRequiresReason = false
  deleteReason = ''
  deleteError = null
}

function openVoiceKickDialog(participant: VoiceParticipant): void {
  if (!isChannelMode || !canModerateVoiceParticipants || voiceKickSubmitting)
    return
  voiceKickDialogParticipant = participant
  voiceKickReason = ''
  voiceKickError = null
}

function closeVoiceKickDialog(): void {
  if (voiceKickSubmitting) return
  voiceKickDialogParticipant = null
  voiceKickReason = ''
  voiceKickError = null
}

async function submitVoiceKick(): Promise<void> {
  if (!isChannelMode || !voiceKickDialogParticipant || voiceKickSubmitting)
    return
  const participant = voiceKickDialogParticipant
  const trimmedReason = voiceKickReason.trim()
  if (!trimmedReason) {
    voiceKickError = 'Reason is required.'
    return
  }
  if (trimmedReason.length > MAX_MODERATION_REASON_CHARS) {
    voiceKickError = `Reason must be ${MAX_MODERATION_REASON_CHARS} characters or less.`
    return
  }

  voiceKickSubmitting = true
  voiceKickError = null
  try {
    await createVoiceKick(activeGuild, {
      targetUserId: participant.userId,
      channelSlug: activeChannel,
      reason: trimmedReason,
    })
    const successMessage = 'User kicked from voice'
    setOperationStatus(
      successMessage,
      'default',
      OPERATION_STATUS_AUTO_CLEAR_MS,
    )
    toastState.show({ variant: 'success', message: successMessage })
    voiceKickDialogParticipant = null
    voiceKickReason = ''
  } catch (error) {
    voiceKickError = messageFromError(error, 'Failed to kick user from voice.')
  } finally {
    voiceKickSubmitting = false
  }
}

async function confirmDeleteMessage(): Promise<void> {
  if (!pendingDeleteMessage || mode !== 'channel') return
  if (pendingDeleteRequiresReason) {
    const trimmedReason = deleteReason.trim()
    if (!trimmedReason) {
      deleteError = 'Reason is required.'
      return
    }
    if (trimmedReason.length > MAX_MODERATION_REASON_CHARS) {
      deleteError = `Reason must be ${MAX_MODERATION_REASON_CHARS} characters or less.`
      return
    }

    deleteSubmitting = true
    deleteError = null
    setOperationStatus('Deleting...')
    try {
      await createMessageDelete(activeGuild, {
        messageId: pendingDeleteMessage.id,
        channelSlug: pendingDeleteMessage.channelSlug,
        reason: trimmedReason,
      })
      const successMessage = 'Message deleted'
      setOperationStatus(
        successMessage,
        'default',
        OPERATION_STATUS_AUTO_CLEAR_MS,
      )
      toastState.show({ variant: 'success', message: successMessage })
      pendingDeleteMessage = null
      pendingDeleteRequiresReason = false
      deleteReason = ''
      deleteError = null
    } catch (error) {
      const message = messageFromError(error, 'Could not delete message.')
      deleteError = message
      setOperationStatus(message, 'error')
      toastState.show({ variant: 'error', message })
    } finally {
      deleteSubmitting = false
    }
    return
  }

  setOperationStatus('Deleting...')
  const sent = messageState.sendMessageDelete(
    pendingDeleteMessage.guildSlug,
    pendingDeleteMessage.channelSlug,
    pendingDeleteMessage.id,
  )
  if (!sent) {
    const message =
      connectionStatusText(wsLifecycleState) ?? 'Could not delete message.'
    setOperationStatus(message, 'error')
    toastState.show({ variant: 'error', message })
    return
  }
  setOperationStatus('Deleted.', 'default', OPERATION_STATUS_AUTO_CLEAR_MS)
  closeDeleteDialog()
}

function handleReactionRequest(message: ChatMessage, emoji: string): void {
  if (mode !== 'channel') return
  const normalizedEmoji = emoji.trim()
  if (!normalizedEmoji) return
  if (
    message.guildSlug !== activeGuild ||
    message.channelSlug !== activeChannel
  ) {
    return
  }
  messageState.sendMessageReactionToggle(
    message.guildSlug,
    message.channelSlug,
    message.id,
    normalizedEmoji,
  )
}

function handleComposerSelectionEvent(event: Event): void {
  const target = event.currentTarget as HTMLTextAreaElement | null
  updateComposerSelection(target)
}

function handleComposerKeydown(event: KeyboardEvent) {
  const isShortcutModifier = event.ctrlKey || event.metaKey
  if (isShortcutModifier && !event.shiftKey && !event.altKey) {
    const key = event.key.toLowerCase()
    if (key === 'b') {
      event.preventDefault()
      formatSelectionBold()
      return
    }
    if (key === 'i') {
      event.preventDefault()
      formatSelectionItalic()
      return
    }
    if (key === 'e') {
      event.preventDefault()
      formatSelectionCode()
      return
    }
  }

  if (
    event.key === 'ArrowUp' &&
    !event.altKey &&
    !event.ctrlKey &&
    !event.metaKey &&
    !event.shiftKey
  ) {
    const target = event.currentTarget as HTMLTextAreaElement | null
    const selectionStart = target?.selectionStart ?? 0
    const selectionEnd = target?.selectionEnd ?? 0
    if (
      !selectedAttachment &&
      composerValue.trim().length === 0 &&
      selectionStart === 0 &&
      selectionEnd === 0
    ) {
      const latestOwn = findLatestOwnMessage()
      if (latestOwn) {
        event.preventDefault()
        startEditingMessage(latestOwn)
      }
    }
    return
  }

  if (event.key === 'Escape' && composerEdit) {
    event.preventDefault()
    cancelComposerEdit()
    return
  }

  if (event.key !== 'Enter') return

  if (event.shiftKey) {
    event.preventDefault()
    const target = event.currentTarget as HTMLTextAreaElement | null
    const start = target?.selectionStart ?? composerValue.length
    const end = target?.selectionEnd ?? composerValue.length
    composerValue = `${composerValue.slice(0, start)}\n${composerValue.slice(end)}`
    if (target) {
      void tick().then(() => {
        target.selectionStart = start + 1
        target.selectionEnd = start + 1
      })
    }
    return
  }

  event.preventDefault()
  void sendComposerMessage()
}

function updateViewportMetrics(
  target: HTMLDivElement | null,
  guildSlug = activeGuild,
  channelSlug = activeChannel,
): void {
  if (!target) {
    scrollTop = 0
    viewportHeight = 320
    distanceFromBottomPx = 0
    return
  }

  scrollTop = target.scrollTop
  viewportHeight = target.clientHeight || 320
  distanceFromBottomPx = Math.max(
    0,
    target.scrollHeight - (target.scrollTop + target.clientHeight),
  )

  if (isChannelMode) {
    messageState.setScrollTop(guildSlug, channelSlug, target.scrollTop)
    if (distanceFromBottomPx <= 24) {
      messageState.clearPendingNew(guildSlug, channelSlug)
    }
  } else if (isDmMode && activeDmSlug) {
    messageState.setScrollTopForDm(activeDmSlug, target.scrollTop)
    if (distanceFromBottomPx <= 24) {
      messageState.clearPendingNewForDm(activeDmSlug)
    }
  }
}

function isNearBottom(threshold = 24): boolean {
  return distanceFromBottomPx <= threshold
}

async function loadOlderHistoryIfNeeded(): Promise<void> {
  if (!isConversationMode || loadHistoryInFlight) return
  if (!channelHistory.hasMoreHistory || channelHistory.loadingHistory) return
  if (scrollTop > HISTORY_LOAD_TRIGGER_PX) return

  const viewport = timelineViewport
  if (!viewport) return
  const loadGuild = activeGuild
  const loadChannel = activeChannel
  const loadDm = activeDmSlug
  const loadKey = isChannelMode
    ? `${loadGuild}:${loadChannel}`
    : loadDm
      ? `dm:${loadDm}`
      : null
  if (!loadKey) return

  loadHistoryInFlight = true
  const beforeHeight = viewport.scrollHeight
  const beforeTop = viewport.scrollTop

  try {
    if (isChannelMode) {
      await messageState.loadOlderHistory(loadGuild, loadChannel)
    } else if (loadDm) {
      await messageState.loadOlderDmHistory(loadDm)
    }
  } finally {
    await tick()
    const nextViewport = timelineViewport
    const currentKey = isChannelMode
      ? `${activeGuild}:${activeChannel}`
      : activeDmSlug
        ? `dm:${activeDmSlug}`
        : null
    if (nextViewport && currentKey === loadKey) {
      const delta = Math.max(0, nextViewport.scrollHeight - beforeHeight)
      nextViewport.scrollTop = beforeTop + delta
      updateViewportMetrics(nextViewport, loadGuild, loadChannel)
    }
    loadHistoryInFlight = false
  }
}

function handleTimelineScroll(event: Event): void {
  const target = event.currentTarget as HTMLDivElement | null
  if (!target) return
  updateViewportMetrics(target, activeGuild, activeChannel)
  if (target.scrollTop <= HISTORY_LOAD_TRIGGER_PX) {
    void loadOlderHistoryIfNeeded()
  }
}

function jumpToPresent(): void {
  const target = timelineViewport
  if (!target) return
  target.scrollTop = target.scrollHeight
  updateViewportMetrics(target, activeGuild, activeChannel)
  if (isChannelMode) {
    messageState.clearPendingNew(activeGuild, activeChannel)
  } else if (isDmMode && activeDmSlug) {
    messageState.clearPendingNewForDm(activeDmSlug)
  }
}

$effect(() => {
  if (!isConversationMode) return
  activeGuild
  activeChannel
  activeDmSlug
  void tick().then(() => {
    composerInput?.focus()
  })
})

$effect(() => {
  messageState.setCurrentUser(currentSessionUser?.id ?? null)
})

$effect(() => {
  if (!isChannelMode || !activeGuild || activeGuildRecord?.isOwner) return
  void guildState.loadMembers(activeGuild).catch(() => {
    // Member role data is loaded opportunistically for attachment permission gating.
  })
})

$effect(() => {
  if (!isChannelMode || !activeGuild) return
  void muteStatusState.refresh(activeGuild).catch(() => {})
})

$effect(() => {
  clearMuteStatusRefreshTimer()
  if (!isChannelMode || !activeGuild) return
  if (
    !activeMuteStatus?.active ||
    activeMuteStatus.isPermanent ||
    !activeMuteStatus.expiresAt
  ) {
    return
  }
  const expiresAtMs = Date.parse(activeMuteStatus.expiresAt)
  if (!Number.isFinite(expiresAtMs)) return
  const delayMs = Math.max(
    250,
    Math.min(MAX_TIMEOUT_MS, Math.round(expiresAtMs - Date.now() + 250)),
  )
  muteStatusRefreshTimer = setTimeout(() => {
    void muteStatusState.refresh(activeGuild).catch(() => {})
  }, delayMs)
})

$effect(() => {
  if (!isConversationMode) {
    composerEdit = null
    composerSelection = { start: 0, end: 0 }
    pendingDeleteMessage = null
    pendingDeleteRequiresReason = false
    deleteReason = ''
    deleteError = null
    deleteSubmitting = false
    voiceKickDialogParticipant = null
    voiceKickReason = ''
    voiceKickError = null
    voiceKickSubmitting = false
    attachmentContextKey = null
    resetTypingStartThrottle()
    clearSelectedAttachment()
    attachmentError = null
    attachmentUploadInFlight = false
    return
  }
  if (!isChannelMode) {
    composerEdit = null
    pendingDeleteMessage = null
    pendingDeleteRequiresReason = false
    deleteReason = ''
    deleteError = null
    deleteSubmitting = false
    voiceKickDialogParticipant = null
    voiceKickReason = ''
    voiceKickError = null
    voiceKickSubmitting = false
    resetTypingStartThrottle()
    clearSelectedAttachment()
    attachmentError = null
    attachmentUploadInFlight = false
    return
  }
  if (
    pendingDeleteMessage &&
    (pendingDeleteMessage.guildSlug !== activeGuild ||
      pendingDeleteMessage.channelSlug !== activeChannel)
  ) {
    pendingDeleteMessage = null
    pendingDeleteRequiresReason = false
    deleteReason = ''
    deleteError = null
    deleteSubmitting = false
  }
  if (
    voiceKickDialogParticipant &&
    !activeVoiceParticipants.some(
      (participant) =>
        participant.userId === voiceKickDialogParticipant?.userId,
    )
  ) {
    voiceKickDialogParticipant = null
    voiceKickReason = ''
    voiceKickError = null
  }
  if (!composerEdit) return
  const exists = timelineMessages.some(
    (message) => message.id === composerEdit?.messageId,
  )
  if (!exists) {
    composerEdit = null
  }
})

$effect(() => {
  if (!isChannelMode || !isMuted) return
  clearSelectedAttachment()
})

$effect(() => {
  if (!isChannelMode) return
  const key = `${activeGuild}:${activeChannel}`
  if (attachmentContextKey === key) return
  attachmentContextKey = key
  attachmentError = null
  attachmentUploadInFlight = false
  resetTypingStartThrottle()
  clearSelectedAttachment()
})

$effect(() => {
  viewportHeight = timelineViewport?.clientHeight || 320
})

$effect(() => {
  if (!isConversationMode) {
    restoringTimelineKey = null
    lastTailMessageId = null
    skipNextTailChange = false
    messageState.setActiveDm(null)
    wsClient.setDmSubscription?.(null)
    return
  }

  const currentKey =
    isChannelMode && activeGuild && activeChannel
      ? `${activeGuild}:${activeChannel}`
      : isDmMode && activeDmSlug
        ? `dm:${activeDmSlug}`
        : null
  if (!currentKey) return

  let currentHistory = channelHistory
  let restoreGuild = activeGuild
  let restoreChannel = activeChannel
  let restoreDm = activeDmSlug
  if (isChannelMode) {
    currentHistory = messageState.historyStateForChannel(
      activeGuild,
      activeChannel,
    )
    messageState.setActiveChannel(activeGuild, activeChannel)
    wsClient.setDmSubscription?.(null)
    void messageState
      .ensureHistoryLoaded(activeGuild, activeChannel)
      .catch(() => {})
  } else if (isDmMode && activeDmSlug) {
    currentHistory = messageState.historyStateForDm(activeDmSlug)
    messageState.setActiveDm(activeDmSlug)
    wsClient.setDmSubscription?.(activeDmSlug)
    void messageState.ensureDmHistoryLoaded(activeDmSlug).catch(() => {})
  }

  if (restoringTimelineKey === currentKey) return

  restoringTimelineKey = currentKey
  skipNextTailChange = !currentHistory.initialized
  lastTailMessageId = timelineMessages.at(-1)?.id ?? null

  void tick().then(() => {
    const liveKey =
      isChannelMode && activeGuild && activeChannel
        ? `${activeGuild}:${activeChannel}`
        : isDmMode && activeDmSlug
          ? `dm:${activeDmSlug}`
          : null
    if (liveKey !== currentKey) {
      return
    }
    const viewport = timelineViewport
    if (!viewport) return

    const savedTop = isChannelMode
      ? messageState.scrollTopForChannel(restoreGuild, restoreChannel)
      : restoreDm
        ? messageState.scrollTopForDm(restoreDm)
        : 0
    if (savedTop > 0) {
      viewport.scrollTop = savedTop
    } else {
      viewport.scrollTop = viewport.scrollHeight
      if (isChannelMode) {
        messageState.clearPendingNew(restoreGuild, restoreChannel)
      } else if (restoreDm) {
        messageState.clearPendingNewForDm(restoreDm)
      }
    }
    updateViewportMetrics(viewport, restoreGuild, restoreChannel)

    if (viewport.scrollTop <= HISTORY_LOAD_TRIGGER_PX) {
      void loadOlderHistoryIfNeeded()
    }
  })
})

$effect(() => {
  if (!isConversationMode) return

  const _messageVersion = messageState.version
  const _blockVersion = blockState.version
  void _messageVersion
  void _blockVersion
  const contextKey = isChannelMode
    ? `${activeGuild}:${activeChannel}`
    : activeDmSlug
      ? `dm:${activeDmSlug}`
      : null
  if (!contextKey) return
  const currentTail = timelineMessages.at(-1)?.id ?? null
  if (!currentTail || currentTail === lastTailMessageId) return

  if (skipNextTailChange) {
    skipNextTailChange = false
    lastTailMessageId = currentTail
    return
  }

  lastTailMessageId = currentTail

  if (isNearBottom(JUMP_TO_PRESENT_THRESHOLD_PX)) {
    const tailGuild = activeGuild
    const tailChannel = activeChannel
    const tailDm = activeDmSlug
    const tailKey = contextKey
    void tick().then(() => {
      const liveKey = isChannelMode
        ? `${activeGuild}:${activeChannel}`
        : activeDmSlug
          ? `dm:${activeDmSlug}`
          : null
      if (liveKey !== tailKey) {
        return
      }
      const viewport = timelineViewport
      if (!viewport) return
      viewport.scrollTop = viewport.scrollHeight
      updateViewportMetrics(viewport, tailGuild, tailChannel)
      if (isChannelMode) {
        messageState.clearPendingNew(tailGuild, tailChannel)
      } else if (tailDm) {
        messageState.clearPendingNewForDm(tailDm)
      }
    })
    return
  }

  if (isChannelMode) {
    messageState.addPendingNew(activeGuild, activeChannel, 1)
  } else if (activeDmSlug) {
    messageState.addPendingNewForDm(activeDmSlug, 1)
  }
})

onMount(() => {
  const unsubscribeLifecycle = wsClient.subscribeLifecycle((state) => {
    wsLifecycleState = state
  })
  const unsubscribeEnvelopes = wsClient.subscribe((envelope) => {
    const wsError = parseWsErrorEnvelope(envelope)
    if (!wsError) return
    const userFacing = toUserFacingError(wsError, 'Message operation failed.')
    setOperationStatus(userFacing.message, 'error')
    toastState.show({ variant: 'error', message: userFacing.message })
  })
  return () => {
    clearOperationStatusTimer()
    clearMuteStatusRefreshTimer()
    unsubscribeLifecycle()
    unsubscribeEnvelopes()
  }
})
</script>

{#if mode === 'admin'}
  {#if canShowAdminPanel}
    <AdminPanel />
  {:else}
    <section class="p-6">
      <p class="rounded-md border border-destructive/30 bg-destructive/10 p-3 text-sm text-destructive">
        Admin access is only available to the instance administrator.
      </p>
    </section>
  {/if}
{:else if mode === 'settings'}
  <ProfileSettingsView />
{:else}
  <section class="flex h-full flex-col gap-4 p-4 md:p-6">
    <header class="space-y-1">
      <h1 class="text-2xl font-semibold tracking-tight">Messages</h1>
      <p class="text-sm text-muted-foreground">{detailText}</p>
    </header>

    {#if connectionStatusMessage}
      <p
        class="rounded-md border border-amber-500/30 bg-amber-500/10 px-3 py-2 text-sm text-amber-200"
        data-testid="connection-status"
        aria-live="polite"
      >
        {connectionStatusMessage}
      </p>
    {/if}

    {#if voiceStatusMessage}
      <p
        class="rounded-md border border-destructive/40 bg-destructive/10 px-3 py-2 text-sm text-destructive"
        data-testid="voice-connection-status"
        aria-live={voiceConnectionState === 'failed' ? 'assertive' : 'polite'}
      >
        {voiceStatusMessage}
      </p>
    {/if}

    {#if shouldShowRecoveryNudge}
      <section class="rounded-md border border-border bg-muted p-4">
        <p class="text-sm font-medium text-foreground">
          Add a recovery email to protect this identity.
        </p>
        <p class="mt-1 text-sm text-muted-foreground">
          Optional, and only shown after your first successful session.
        </p>
        <div class="mt-3 flex flex-wrap gap-2">
          <button
            type="button"
            class="inline-flex items-center justify-center rounded-md bg-fire px-3 py-2 text-sm font-medium text-fire-foreground transition-opacity hover:opacity-90"
            onclick={() => void handleOpenSettings()}
          >
            Set up recovery email
          </button>
          <button
            type="button"
            class="inline-flex items-center justify-center rounded-md bg-background px-3 py-2 text-sm font-medium text-foreground transition-opacity hover:opacity-90"
            onclick={() => void handleDismissRecoveryNudge()}
          >
            Not now
          </button>
        </div>
      </section>
    {/if}

    <section
      class="relative min-h-0 flex-1 rounded-md border border-border bg-card p-4"
      ondragenter={handleTimelineDragEnter}
      ondragover={handleTimelineDragOver}
      ondragleave={handleTimelineDragLeave}
      ondrop={handleTimelineDrop}
      aria-label={isChannelMode ? 'Channel timeline' : 'Direct message timeline'}
    >
      <h2 class="text-sm font-medium text-foreground">
        {isChannelMode ? 'Channel Timeline' : 'Direct Message Timeline'}
      </h2>
      <div class="mt-3 min-h-0 flex-1" data-testid="channel-timeline">
        <div
          class="h-full overflow-y-auto rounded-md border border-border/40 bg-background/20 px-2 py-2"
          bind:this={timelineViewport}
          onscroll={handleTimelineScroll}
          role="log"
          aria-live="polite"
          aria-relevant="additions text"
          data-testid="channel-timeline-scroll"
        >
          {#if channelHistory.loadingHistory}
            <div class="space-y-2 pb-2" data-testid="history-loading-skeletons">
              {#each Array.from({ length: HISTORY_SKELETON_COUNT }) as _, index (`skeleton-${index}`)}
                <div class="flex animate-pulse gap-3 rounded-md px-2 py-1">
                  <span class="h-8 w-8 shrink-0 rounded-full bg-muted"></span>
                  <span class="min-w-0 flex-1 space-y-1 pt-1">
                    <span class="block h-3 w-24 rounded bg-muted"></span>
                    <span class="block h-3 w-4/5 rounded bg-muted"></span>
                  </span>
                </div>
              {/each}
            </div>
          {/if}

          {#if timelineMessages.length === 0 && !channelHistory.loadingHistory}
            <p
              class="rounded-md bg-muted px-3 py-2 text-sm text-muted-foreground"
              data-testid="message-empty-state"
            >
              {emptyStateCopy}
            </p>
          {:else if timelineMessages.length > 0}
            <div class="relative" style={`height: ${virtualRows.totalHeight}px;`}>
              {#each visibleRows as row (row.id)}
                <div
                  class="absolute inset-x-0"
                  style={`top: ${row.top}px; height: ${row.height}px;`}
                  data-testid={`message-window-row-${row.message.id}`}
                >
                  <MessageBubble
                    message={row.message}
                    compact={row.compact}
                    currentUserId={currentSessionUser?.id ?? null}
                    hasManageMessagesPermission={canModerateMessages}
                    onEditRequest={startEditingMessage}
                    onDeleteRequest={requestDeleteMessage}
                    onReactRequest={handleReactionRequest}
                  />
                </div>
              {/each}
            </div>
          {/if}
        </div>
      </div>

      {#if dragActive}
        <div
          class="pointer-events-none absolute inset-0 z-20 flex items-center justify-center rounded-md border-2 border-dashed border-fire bg-fire/10 text-sm font-medium text-fire-foreground"
          data-testid="message-attachment-drag-overlay"
        >
          Drop file to attach
        </div>
      {/if}

      {#if isConversationMode && showJumpToPresent}
        <div class="mt-2 flex justify-end">
          <button
            type="button"
            class="rounded-md border border-border bg-background px-3 py-1.5 text-xs font-medium text-foreground hover:bg-muted"
            onclick={jumpToPresent}
            data-testid="jump-to-present"
          >
            {#if channelHistory.pendingNewCount > 0}
              Jump to present ({channelHistory.pendingNewCount} new)
            {:else}
              Jump to present
            {/if}
          </button>
        </div>
      {/if}
    </section>

    {#if showVoiceBar}
      <VoiceBar
        guildName={activeGuild}
        channelName={activeChannel}
        connectionState={voiceConnectionState}
        isMuted={voiceState.isMuted}
        isDeafened={voiceState.isDeafened}
        isParticipantsOpen={participantsOpen}
        onToggleParticipants={() => {
          participantsOpen = !participantsOpen
        }}
        onToggleMute={() => voiceState.toggleMute()}
        onToggleDeafen={() => voiceState.toggleDeafen()}
        onDisconnect={() => voiceState.disconnect()}
      />
    {/if}

    {#if showVoicePanel}
      <VoicePanel
        channelName={activeChannel}
        participants={activeVoiceParticipants}
        canModerateVoiceParticipants={canModerateVoiceParticipants}
        onParticipantVolumeChange={(participantUserId, volumePercent) => {
          voiceState.setParticipantVolume(participantUserId, volumePercent)
        }}
        onKickFromVoice={(participantUserId) => {
          const participant = activeVoiceParticipants.find(
            (entry) => entry.userId === participantUserId,
          )
          if (!participant) return
          openVoiceKickDialog(participant)
        }}
      />
    {/if}

    {#if isConversationMode}
      <section class="rounded-md border border-border bg-card p-4">
        <label
          for="message-composer"
          class="mb-2 block text-sm font-medium text-foreground"
        >
          Message
        </label>
        {#if isChannelMode && muteStatusMessage}
          <p
            class="mb-2 rounded-md border border-destructive/40 bg-destructive/10 px-2 py-1 text-xs text-destructive"
            data-testid="mute-status-banner"
          >
            {muteStatusMessage}
          </p>
        {/if}
        {#if isChannelMode}
          <input
            type="file"
            class="hidden"
            bind:this={attachmentInput}
            onchange={handleAttachmentInputChange}
            data-testid="message-attachment-input"
          />
          {#if selectedAttachment}
            <div
              class="mb-2 flex items-center gap-2 rounded-md border border-border bg-muted px-2 py-1.5 text-xs text-foreground"
              data-testid="attachment-preview-chip"
            >
              <span class="truncate">{selectedAttachment.name}</span>
              <span class="shrink-0 text-muted-foreground">
                {formatFileSize(selectedAttachment.size)}
              </span>
              <button
                type="button"
                class="ml-auto rounded px-2 py-0.5 text-xs text-foreground hover:bg-background/70"
                onclick={() => clearSelectedAttachment()}
                disabled={attachmentUploadInFlight}
                data-testid="attachment-preview-remove"
              >
                Remove
              </button>
            </div>
          {/if}
          {#if attachmentError}
            <p
              class="mb-2 rounded-md border border-destructive/40 bg-destructive/10 px-2 py-1 text-xs text-destructive"
              data-testid="attachment-error"
            >
              {attachmentError}
            </p>
          {/if}
          {#if attachmentUploadInFlight && attachmentUploadProgress !== null}
            <div class="mb-2" data-testid="attachment-upload-progress">
              <div class="h-1.5 w-full rounded bg-muted">
                <div
                  class="h-full rounded bg-fire transition-all"
                  style={`width: ${Math.max(0, Math.min(100, attachmentUploadProgress))}%`}
                ></div>
              </div>
              <p class="mt-1 text-xs text-muted-foreground">
                Uploading {Math.round(attachmentUploadProgress)}%
              </p>
            </div>
          {/if}
        {/if}
        {#if hasComposerSelection}
          <div
            class="mb-2 flex flex-wrap items-center gap-1 rounded-md border border-border/70 bg-background/70 px-2 py-1"
            data-testid="message-markdown-toolbar"
          >
            <button
              type="button"
              class="rounded px-2 py-1 text-xs text-foreground hover:bg-muted"
              onmousedown={(event) => event.preventDefault()}
              onclick={formatSelectionBold}
              data-testid="message-format-bold"
            >
              Bold
            </button>
            <button
              type="button"
              class="rounded px-2 py-1 text-xs text-foreground hover:bg-muted"
              onmousedown={(event) => event.preventDefault()}
              onclick={formatSelectionItalic}
              data-testid="message-format-italic"
            >
              Italic
            </button>
            <button
              type="button"
              class="rounded px-2 py-1 text-xs text-foreground hover:bg-muted"
              onmousedown={(event) => event.preventDefault()}
              onclick={formatSelectionCode}
              data-testid="message-format-code"
            >
              Code
            </button>
          </div>
        {/if}
        <div class="flex items-end gap-2">
          {#if isChannelMode}
            <button
              type="button"
              class="inline-flex h-[44px] shrink-0 items-center justify-center rounded-md border border-border bg-background px-3 text-lg text-foreground hover:bg-muted disabled:cursor-not-allowed disabled:opacity-50"
              onclick={openAttachmentPicker}
              disabled={!canAttachFiles || attachmentUploadInFlight}
              title={isMuted && muteStatusMessage
                ? muteStatusMessage
                : canAttachFiles
                  ? 'Attach file'
                  : PERMISSION_DENIED_MESSAGE}
              aria-label="Attach file"
              data-testid="message-attachment-button"
            >
              📎
            </button>
          {/if}
          <textarea
            id="message-composer"
            data-testid="message-composer-input"
            class="min-h-[44px] w-full resize-y rounded-md border border-input bg-background px-3 py-2 text-sm text-foreground placeholder:text-muted-foreground focus:outline-none focus:ring-2 focus:ring-ring"
            placeholder={isChannelMode && isMuted
              ? 'You are muted in this channel'
              : isChannelMode
                ? `Message #${activeChannel}`
                : 'Message'}
            bind:this={composerInput}
            bind:value={composerValue}
            onkeydown={handleComposerKeydown}
            oninput={handleComposerInput}
            onfocus={handleComposerSelectionEvent}
            onclick={handleComposerSelectionEvent}
            onkeyup={handleComposerSelectionEvent}
            onselect={handleComposerSelectionEvent}
            disabled={isChannelMode && (attachmentUploadInFlight || isMuted)}
          ></textarea>
          <button
            type="button"
            class="inline-flex h-[44px] items-center justify-center rounded-md bg-fire px-4 text-sm font-medium text-fire-foreground transition-opacity hover:opacity-90 disabled:cursor-not-allowed disabled:opacity-50"
            onclick={() => void sendComposerMessage()}
            disabled={!canSubmitComposer}
            data-testid="message-composer-submit"
          >
            {composerEdit ? 'Save' : 'Send'}
          </button>
        </div>
        {#if isChannelMode && typingIndicatorText}
          <p
            class="mt-2 text-xs text-muted-foreground"
            aria-live="polite"
            data-testid="typing-indicator"
          >
            {typingIndicatorText}
          </p>
        {/if}
        {#if operationStatusText}
          <p
            class={`mt-2 text-xs ${operationStatusTone === 'error' ? 'text-destructive' : 'text-muted-foreground'}`}
            aria-live={operationStatusTone === 'error' ? 'assertive' : 'polite'}
            data-testid="composer-operation-status"
          >
            {operationStatusText}
          </p>
        {/if}
        <p class="mt-2 text-xs text-muted-foreground">
          {#if composerEdit}
            Editing message · Enter to save · Escape to cancel
          {:else}
            {#if isChannelMode}
              Enter to send · Shift+Enter for newline · Ctrl+B/Ctrl+I/Ctrl+E format selected text · Up to edit latest own message · Drag/drop or paperclip to attach files
            {:else}
              Enter to send · Shift+Enter for newline · Ctrl+B/Ctrl+I/Ctrl+E format selected text
            {/if}
          {/if}
        </p>
      </section>
    {/if}
  </section>
{/if}

{#if voiceKickDialogParticipant}
  <div class="fixed inset-0 z-30 flex items-center justify-center bg-black/60 p-4">
    <div
      class="w-full max-w-md rounded-md border border-border bg-card p-4 shadow-lg"
      role="dialog"
      aria-modal="true"
      aria-label={`Kick ${voiceKickDialogParticipant.displayName ?? voiceKickDialogParticipant.username} from voice`}
      data-testid="voice-kick-dialog"
    >
      <h3 class="text-base font-semibold text-foreground">
        Kick {voiceKickDialogParticipant.displayName ?? voiceKickDialogParticipant.username}
      </h3>
      <p class="mt-2 text-sm text-muted-foreground">
        Provide a reason before disconnecting this user from the voice channel.
      </p>
      {#if voiceKickError}
        <p
          class="mt-3 rounded-md border border-destructive/40 bg-destructive/10 px-2 py-1 text-xs text-destructive"
          data-testid="voice-kick-error"
        >
          {voiceKickError}
        </p>
      {/if}
      <label class="mt-3 block space-y-1 text-xs text-muted-foreground">
        <span class="font-medium text-foreground">Reason</span>
        <textarea
          class="min-h-[96px] w-full rounded-md border border-input bg-background px-3 py-2 text-sm text-foreground focus:outline-none focus:ring-2 focus:ring-ring"
          bind:value={voiceKickReason}
          data-testid="voice-kick-reason-input"
          maxlength={MAX_MODERATION_REASON_CHARS}
          placeholder="Required moderation reason"
        ></textarea>
      </label>
      <div class="mt-4 flex justify-end gap-2">
        <button
          type="button"
          class="rounded-md border border-border bg-background px-3 py-2 text-sm font-medium text-foreground hover:bg-muted disabled:cursor-not-allowed disabled:opacity-60"
          onclick={closeVoiceKickDialog}
          disabled={voiceKickSubmitting}
        >
          Cancel
        </button>
        <button
          type="button"
          class="rounded-md bg-destructive px-3 py-2 text-sm font-medium text-destructive-foreground hover:opacity-90 disabled:cursor-not-allowed disabled:opacity-60"
          onclick={() => void submitVoiceKick()}
          disabled={voiceKickSubmitting}
          data-testid="voice-kick-submit-button"
        >
          {voiceKickSubmitting ? 'Kicking...' : 'Kick from voice'}
        </button>
      </div>
    </div>
  </div>
{/if}

{#if pendingDeleteMessage}
  <div class="fixed inset-0 z-30 flex items-center justify-center bg-black/60 p-4">
    <div
      class="w-full max-w-md rounded-md border border-border bg-card p-4 shadow-lg"
      role="dialog"
      aria-modal="true"
      aria-label="Delete message"
    >
      <h3 class="text-base font-semibold text-foreground">Delete message</h3>
      <p class="mt-2 text-sm text-muted-foreground">
        {#if pendingDeleteRequiresReason}
          Provide a moderation reason before deleting this message for everyone in the channel.
        {:else}
          This message will be removed for everyone in this channel.
        {/if}
      </p>
      {#if pendingDeleteRequiresReason}
        <label class="mt-3 block space-y-1 text-xs text-muted-foreground">
          <span class="font-medium text-foreground">Reason</span>
          <textarea
            class="min-h-[96px] w-full rounded-md border border-input bg-background px-3 py-2 text-sm text-foreground focus:outline-none focus:ring-2 focus:ring-ring"
            bind:value={deleteReason}
            data-testid="message-delete-reason-input"
            maxlength={MAX_MODERATION_REASON_CHARS}
            placeholder="Required moderation reason"
            oninput={() => {
              if (deleteError) deleteError = null
            }}
          ></textarea>
        </label>
      {/if}
      {#if deleteError}
        <p
          class="mt-3 rounded-md border border-destructive/40 bg-destructive/10 px-2 py-1 text-xs text-destructive"
          data-testid="message-delete-error"
        >
          {deleteError}
        </p>
      {/if}
      <div class="mt-4 flex justify-end gap-2">
        <button
          type="button"
          class="rounded-md border border-border bg-background px-3 py-2 text-sm font-medium text-foreground hover:bg-muted"
          onclick={closeDeleteDialog}
          disabled={deleteSubmitting}
        >
          Cancel
        </button>
        <button
          type="button"
          class="rounded-md bg-destructive px-3 py-2 text-sm font-medium text-destructive-foreground hover:opacity-90"
          onclick={() => void confirmDeleteMessage()}
          disabled={deleteSubmitting}
        >
          {deleteSubmitting ? 'Deleting...' : 'Delete message'}
        </button>
      </div>
    </div>
  </div>
{/if}
