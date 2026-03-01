import { render } from '@testing-library/svelte'
import { describe, expect, it } from 'vitest'
import VoicePanel from './VoicePanel.svelte'

describe('VoicePanel', () => {
  it('shows empty state when no voice participants are present', () => {
    const { getByTestId } = render(VoicePanel, {
      channelName: 'general',
      participants: [],
    })

    expect(getByTestId('voice-panel')).toBeInTheDocument()
    expect(getByTestId('voice-panel-occupancy-announcement')).toHaveTextContent(
      '0 users in voice channel general',
    )
    expect(getByTestId('voice-panel-empty')).toHaveTextContent(
      'No participants in voice.',
    )
  })

  it('renders participant rows and count when data is available', () => {
    const { getByTestId, queryByTestId } = render(VoicePanel, {
      channelName: 'general',
      participants: [
        {
          userId: 'user-1',
          username: 'alice',
          displayName: 'Alice',
          avatarColor: '#3366ff',
          isMuted: false,
          isDeafened: false,
          isSpeaking: false,
        },
      ],
    })

    expect(getByTestId('voice-panel-list')).toBeInTheDocument()
    expect(getByTestId('voice-panel-occupancy-announcement')).toHaveTextContent(
      '1 user in voice channel general',
    )
    expect(getByTestId('voice-participant-user-1')).toBeInTheDocument()
    expect(queryByTestId('voice-panel-empty')).not.toBeInTheDocument()
  })
})
