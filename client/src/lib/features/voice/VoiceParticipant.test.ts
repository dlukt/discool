import { render } from '@testing-library/svelte'
import { describe, expect, it } from 'vitest'
import VoiceParticipant from './VoiceParticipant.svelte'

describe('VoiceParticipant', () => {
  it('renders avatar, username metadata, and mute/deafen indicators', () => {
    const { getByTestId } = render(VoiceParticipant, {
      participant: {
        userId: 'user-1',
        username: 'alice',
        displayName: 'Alice',
        avatarColor: '#3366ff',
        isMuted: true,
        isDeafened: true,
        isSpeaking: true,
      },
    })

    expect(getByTestId('voice-participant-user-1')).toHaveAttribute(
      'aria-label',
      expect.stringContaining('speaking'),
    )
    expect(getByTestId('voice-participant-name-user-1')).toHaveTextContent(
      'Alice',
    )
    const avatar = getByTestId('voice-participant-avatar-user-1')
    expect(avatar).toHaveTextContent('A')
    expect(avatar.className).toContain('motion-reduce:shadow-none')
    expect(getByTestId('voice-participant-muted-user-1')).toBeInTheDocument()
    expect(getByTestId('voice-participant-deafened-user-1')).toBeInTheDocument()
  })
})
