export type VoiceConnectionStatus =
  | 'idle'
  | 'connecting'
  | 'connected'
  | 'disconnected'
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
  state?: VoiceConnectionStatus
}

export type VoiceParticipantWire = {
  user_id?: string
  username?: string
  display_name?: string | null
  avatar_color?: string | null
  audio_stream_id?: string | null
  is_muted?: boolean
  is_deafened?: boolean
  is_speaking?: boolean
}

export type VoiceStateUpdateWire = {
  guild_slug?: string
  channel_slug?: string
  participant_count?: number
  participants?: VoiceParticipantWire[]
}

export type VoiceJoinContext = {
  guildSlug: string
  channelSlug: string
}

export type VoiceParticipant = {
  userId: string
  username: string
  displayName: string | null
  avatarColor: string | null
  audioStreamId: string | null
  isMuted: boolean
  isDeafened: boolean
  isSpeaking: boolean
  volumePercent: number
  volumeScalar: number
}

export type VoiceParticipantAudioBinding = {
  userId: string
  audioStreamId: string | null
}

export type VoiceParticipantVolumePreference = {
  participantUserId: string
  volumePercent: number
  audioScalar: number
}
