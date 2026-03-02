import { fireEvent, render } from '@testing-library/svelte'
import { describe, expect, it, vi } from 'vitest'
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
          audioStreamId: 'stream-1',
          isMuted: false,
          isDeafened: false,
          isSpeaking: false,
          volumePercent: 100,
          volumeScalar: 1,
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

  it('propagates participant volume changes and kick action callbacks', async () => {
    const onParticipantVolumeChange = vi.fn()
    const onKickFromVoice = vi.fn()
    const { getByTestId, queryByTestId } = render(VoicePanel, {
      channelName: 'general',
      canModerateVoiceParticipants: true,
      onParticipantVolumeChange,
      onKickFromVoice,
      participants: [
        {
          userId: 'user-1',
          username: 'alice',
          displayName: 'Alice',
          avatarColor: '#3366ff',
          audioStreamId: 'stream-1',
          isMuted: false,
          isDeafened: false,
          isSpeaking: false,
          volumePercent: 100,
          volumeScalar: 1,
        },
      ],
    })

    expect(
      queryByTestId('voice-participant-volume-slider-user-1'),
    ).not.toBeInTheDocument()

    await fireEvent.click(getByTestId('voice-participant-toggle-user-1'))

    const slider = getByTestId(
      'voice-participant-volume-slider-user-1',
    ) as HTMLInputElement
    expect(slider).toHaveAttribute('aria-valuetext', '100%')
    const kickButton = getByTestId('voice-participant-kick-user-1')
    expect(kickButton).toBeInTheDocument()

    await fireEvent.input(slider, { target: { value: '150' } })
    expect(onParticipantVolumeChange).toHaveBeenCalledWith('user-1', 150)
    await fireEvent.click(kickButton)
    expect(onKickFromVoice).toHaveBeenCalledWith('user-1')
  })

  it('supports mobile-sheet layout variant for bottom-sheet containers', () => {
    const { getByTestId } = render(VoicePanel, {
      channelName: 'general',
      variant: 'mobile-sheet',
      participants: [],
    })

    const panel = getByTestId('voice-panel')
    expect(panel).toHaveAttribute('data-variant', 'mobile-sheet')
    expect(panel).toHaveClass('max-h-[40vh]')
    expect(panel).toHaveClass('overflow-y-auto')
  })
})
