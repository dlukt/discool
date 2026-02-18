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

export async function apiFetch<T>(
  path: string,
  init: RequestInit = {},
): Promise<T> {
  const headers = new Headers(init.headers)
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
    if (isRecord(payload) && 'data' in payload) {
      return (payload as ApiSuccess<T>).data
    }
    throw new ApiError('INVALID_RESPONSE', 'Invalid server response')
  }

  if (isRecord(payload) && 'error' in payload) {
    const err = (payload as { error: unknown }).error
    if (
      isRecord(err) &&
      typeof err.code === 'string' &&
      typeof err.message === 'string'
    ) {
      const details =
        isRecord(err.details) && !Array.isArray(err.details) ? err.details : {}
      throw new ApiError(err.code, err.message, details)
    }
  }

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
