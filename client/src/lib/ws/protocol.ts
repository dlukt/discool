export type WsEnvelope<T = Record<string, unknown>> = {
  op: string
  d: T
  s?: number
  t?: string
}

export type WsLifecycleState =
  | 'connecting'
  | 'connected'
  | 'reconnecting'
  | 'disconnected'

export type WsClientOp =
  | 'c_heartbeat'
  | 'c_subscribe'
  | 'c_unsubscribe'
  | 'c_message_create'
  | 'c_message_update'
  | 'c_message_delete'
  | 'c_message_reaction_toggle'
  | 'c_dm_subscribe'
  | 'c_dm_message_create'
  | 'c_typing_start'
  | 'c_resume'

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null
}

export function parseWsEnvelope(raw: unknown): WsEnvelope | null {
  if (!isRecord(raw)) return null
  const op = raw.op
  if (typeof op !== 'string' || op.length === 0) return null
  const data = isRecord(raw.d) ? raw.d : {}
  const sequence = typeof raw.s === 'number' ? raw.s : undefined
  const timestamp = typeof raw.t === 'string' ? raw.t : undefined
  return {
    op,
    d: data,
    s: sequence,
    t: timestamp,
  }
}
