export type VoiceConnectionStatus =
  | 'idle'
  | 'connecting'
  | 'connected'
  | 'retrying'
  | 'failed'

export type VoiceOfferWire = {
  guild_slug?: string
  channel_slug?: string
  sdp?: string
  sdp_type?: string
}

export type VoiceIceCandidateWire = {
  guild_slug?: string
  channel_slug?: string
  candidate?: string
  sdp_mid?: string | null
  sdp_mline_index?: number | null
}

export type VoiceConnectionStateWire = {
  guild_slug?: string
  channel_slug?: string
  state?:
    | VoiceConnectionStatus
    | 'connecting'
    | 'connected'
    | 'retrying'
    | 'failed'
}

export type VoiceJoinContext = {
  guildSlug: string
  channelSlug: string
}
