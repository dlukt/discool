export type ApiErrorDetails = Record<string, unknown>

export type AdminInfo = {
  username: string
  avatarColor?: string
}

export type InstanceStatus = {
  initialized: boolean
  name?: string
  description?: string
  discoveryEnabled?: boolean
  admin?: AdminInfo
}

export type SetupRequest = {
  adminUsername: string
  avatarColor?: string
  instanceName: string
  instanceDescription?: string
  discoveryEnabled?: boolean
}

export type AdminHealth = {
  cpuUsagePercent: number
  memoryRssBytes: number
  uptimeSeconds: number
  dbSizeBytes: number
  dbPoolActive: number
  dbPoolIdle: number
  dbPoolMax: number
  websocketConnections: number
  p2pDiscoveryEnabled: boolean
  p2pDiscoveryLabel: string
}

type AdminInfoWire = {
  username: string
  avatar_color?: string
}

type InstanceStatusWire = {
  initialized: boolean
  name?: string
  description?: string
  discovery_enabled?: boolean
  admin?: AdminInfoWire
}

type SetupRequestWire = {
  admin_username: string
  avatar_color?: string
  instance_name: string
  instance_description?: string
  discovery_enabled?: boolean
}

type AdminHealthWire = {
  cpu_usage_percent: number
  memory_rss_bytes: number
  uptime_seconds: number
  db_size_bytes: number
  db_pool_active: number
  db_pool_idle: number
  db_pool_max: number
  websocket_connections: number
  p2p_discovery_enabled: boolean
  p2p_discovery_label: string
}

function toInstanceStatus(wire: InstanceStatusWire): InstanceStatus {
  return {
    initialized: wire.initialized,
    name: wire.name,
    description: wire.description,
    discoveryEnabled: wire.discovery_enabled,
    admin: wire.admin
      ? { username: wire.admin.username, avatarColor: wire.admin.avatar_color }
      : undefined,
  }
}

function toAdminHealth(wire: AdminHealthWire): AdminHealth {
  return {
    cpuUsagePercent: wire.cpu_usage_percent,
    memoryRssBytes: wire.memory_rss_bytes,
    uptimeSeconds: wire.uptime_seconds,
    dbSizeBytes: wire.db_size_bytes,
    dbPoolActive: wire.db_pool_active,
    dbPoolIdle: wire.db_pool_idle,
    dbPoolMax: wire.db_pool_max,
    websocketConnections: wire.websocket_connections,
    p2pDiscoveryEnabled: wire.p2p_discovery_enabled,
    p2pDiscoveryLabel: wire.p2p_discovery_label,
  }
}

function toSetupRequestWire(req: SetupRequest): SetupRequestWire {
  return {
    admin_username: req.adminUsername,
    avatar_color: req.avatarColor,
    instance_name: req.instanceName,
    instance_description: req.instanceDescription,
    discovery_enabled: req.discoveryEnabled,
  }
}

export class ApiError extends Error {
  code: string
  details: ApiErrorDetails

  constructor(code: string, message: string, details: ApiErrorDetails = {}) {
    super(message)
    this.name = 'ApiError'
    this.code = code
    this.details = details
  }
}

type ApiSuccess<T> = { data: T }

let sessionToken: string | null = null
let unauthorizedHandler: (() => void) | null = null

export function setSessionToken(token: string | null) {
  sessionToken = token
}

export function setUnauthorizedHandler(handler: (() => void) | null) {
  unauthorizedHandler = handler
}

function handleUnauthorized() {
  // Only trigger auto-re-auth when we actually had a session token in play.
  if (!sessionToken) return
  sessionToken = null
  unauthorizedHandler?.()
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null
}

function parseJson(text: string): unknown {
  if (!text) return undefined
  try {
    return JSON.parse(text) as unknown
  } catch {
    return undefined
  }
}

function apiErrorFromPayload(payload: unknown): ApiError | undefined {
  if (isRecord(payload) && 'error' in payload) {
    const err = (payload as { error: unknown }).error
    if (
      isRecord(err) &&
      typeof err.code === 'string' &&
      typeof err.message === 'string'
    ) {
      const details =
        isRecord(err.details) && !Array.isArray(err.details) ? err.details : {}
      return new ApiError(err.code, err.message, details)
    }
  }
  return undefined
}

export async function apiFetch<T>(
  path: string,
  init: RequestInit = {},
): Promise<T> {
  const headers = new Headers(init.headers)
  if (sessionToken && !headers.has('authorization')) {
    headers.set('authorization', `Bearer ${sessionToken}`)
  }
  if (typeof init.body === 'string' && !headers.has('content-type')) {
    headers.set('content-type', 'application/json')
  }

  const res = await fetch(path, {
    ...init,
    headers,
  })

  const text = await res.text()
  const payload = parseJson(text)

  if (res.ok) {
    if (res.status === 204) {
      return undefined as T
    }
    if (isRecord(payload) && 'data' in payload) {
      return (payload as ApiSuccess<T>).data
    }
    throw new ApiError('INVALID_RESPONSE', 'Invalid server response')
  }

  const apiErr = apiErrorFromPayload(payload)
  if (res.status === 401) {
    handleUnauthorized()
  }
  if (apiErr) throw apiErr

  throw new ApiError('HTTP_ERROR', res.statusText || `HTTP ${res.status}`, {
    status: res.status,
  })
}

export function getInstanceStatus(): Promise<InstanceStatus> {
  return apiFetch<InstanceStatusWire>('/api/v1/instance').then(toInstanceStatus)
}

export function submitSetup(data: SetupRequest): Promise<InstanceStatus> {
  return apiFetch<InstanceStatusWire>('/api/v1/instance/setup', {
    method: 'POST',
    body: JSON.stringify(toSetupRequestWire(data)),
  }).then(toInstanceStatus)
}

export function getAdminHealth(): Promise<AdminHealth> {
  return apiFetch<AdminHealthWire>('/api/v1/admin/health').then(toAdminHealth)
}

function filenameFromContentDisposition(
  header: string | null,
): string | undefined {
  if (!header) return undefined
  const filenameStar = header.match(/filename\*=([^;]+)/i)
  if (filenameStar?.[1]) {
    let value = filenameStar[1].trim()
    if (
      (value.startsWith('"') && value.endsWith('"')) ||
      (value.startsWith("'") && value.endsWith("'"))
    ) {
      value = value.slice(1, -1)
    }

    // RFC 5987 / RFC 6266: filename*=charset'lang'percent-encoded
    const rfc5987 = value.match(/^([^']*)'[^']*'(.*)$/)
    const encoded = rfc5987 ? rfc5987[2] : value
    try {
      return decodeURIComponent(encoded)
    } catch {
      return encoded
    }
  }
  const quoted = header.match(/filename="([^"]+)"/)
  if (quoted?.[1]) return quoted[1]
  const unquoted = header.match(/filename=([^;]+)/)
  return unquoted?.[1]?.trim()
}

function sanitizeDownloadFilename(name: string): string {
  let file = name.split(/[/\\]/).pop() ?? name
  file = file.trim()
  file = file
    .split('')
    .filter((ch) => {
      const code = ch.charCodeAt(0)
      return !(code < 32 || code === 127)
    })
    .join('')
  if (!file || file === '.' || file === '..') return 'discool-backup.bin'
  return file
}

export async function downloadBackup(): Promise<void> {
  const init: RequestInit = { method: 'POST' }
  if (sessionToken) {
    init.headers = { authorization: `Bearer ${sessionToken}` }
  }
  const res = await fetch('/api/v1/admin/backup', init)
  if (res.status === 401) {
    handleUnauthorized()
  }

  if (!res.ok) {
    const text = await res.text()
    const payload = parseJson(text)

    const apiErr = apiErrorFromPayload(payload)
    if (apiErr) throw apiErr

    throw new ApiError('HTTP_ERROR', res.statusText || `HTTP ${res.status}`, {
      status: res.status,
    })
  }

  const blob = await res.blob()
  const filename = sanitizeDownloadFilename(
    filenameFromContentDisposition(res.headers.get('content-disposition')) ??
      'discool-backup.bin',
  )

  const url = URL.createObjectURL(blob)
  try {
    const a = document.createElement('a')
    a.href = url
    a.download = filename
    a.hidden = true
    document.body.appendChild(a)
    a.click()
    a.remove()
  } finally {
    // Some browsers can cancel the download if the object URL is revoked immediately.
    setTimeout(() => URL.revokeObjectURL(url), 0)
  }
}
