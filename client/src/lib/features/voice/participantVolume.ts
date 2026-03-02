export const PARTICIPANT_VOLUME_MIN_PERCENT = 0
export const PARTICIPANT_VOLUME_MAX_PERCENT = 200
export const PARTICIPANT_VOLUME_DEFAULT_PERCENT = 100

export function normalizeParticipantVolumePercent(value: number): number {
  if (!Number.isFinite(value)) {
    return PARTICIPANT_VOLUME_DEFAULT_PERCENT
  }
  const rounded = Math.round(value)
  if (rounded < PARTICIPANT_VOLUME_MIN_PERCENT) {
    return PARTICIPANT_VOLUME_MIN_PERCENT
  }
  if (rounded > PARTICIPANT_VOLUME_MAX_PERCENT) {
    return PARTICIPANT_VOLUME_MAX_PERCENT
  }
  return rounded
}

export function participantVolumePercentToAudioScalar(value: number): number {
  return normalizeParticipantVolumePercent(value) / 100
}
