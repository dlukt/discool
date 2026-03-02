import { fireEvent, render } from '@testing-library/svelte'
import { describe, expect, it, vi } from 'vitest'
import VoiceParticipant from './VoiceParticipant.svelte'

describe('VoiceParticipant', () => {
  it('renders avatar, username metadata, and mute/deafen indicators', () => {
    const { getByTestId } = render(VoiceParticipant, {
      participant: {
        userId: 'user-1',
        username: 'alice',
        displayName: 'Alice',
        avatarColor: '#3366ff',
        audioStreamId: 'stream-1',
        isMuted: true,
        isDeafened: true,
        isSpeaking: true,
        volumePercent: 100,
        volumeScalar: 1,
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

  it('shows keyboard-accessible slider with aria labeling when row is expanded', async () => {
    const onVolumeChange = vi.fn()
    const { getByTestId, queryByTestId } = render(VoiceParticipant, {
      participant: {
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
      onVolumeChange,
      showKickPlaceholder: true,
    })

    expect(
      queryByTestId('voice-participant-volume-slider-user-1'),
    ).not.toBeInTheDocument()
    await fireEvent.click(getByTestId('voice-participant-toggle-user-1'))

    const slider = getByTestId(
      'voice-participant-volume-slider-user-1',
    ) as HTMLInputElement
    expect(slider).toHaveAttribute('type', 'range')
    expect(slider).toHaveAttribute('min', '0')
    expect(slider).toHaveAttribute('max', '200')
    expect(slider).toHaveAttribute('step', '5')
    expect(slider).toHaveAttribute('aria-label', 'Alice volume')
    expect(slider).toHaveAttribute('aria-valuetext', '100%')
    expect(
      getByTestId('voice-participant-kick-placeholder-user-1'),
    ).toBeInTheDocument()

    await fireEvent.keyDown(slider, { key: 'ArrowRight' })
    await fireEvent.input(slider, { target: { value: '105' } })
    expect(onVolumeChange).toHaveBeenCalledWith('user-1', 105)
  })

  it('clamps slider input to supported 0..200 range', async () => {
    const onVolumeChange = vi.fn()
    const { getByTestId } = render(VoiceParticipant, {
      participant: {
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
      onVolumeChange,
    })

    await fireEvent.click(getByTestId('voice-participant-toggle-user-1'))
    const slider = getByTestId(
      'voice-participant-volume-slider-user-1',
    ) as HTMLInputElement

    await fireEvent.input(slider, { target: { value: '-20' } })
    await fireEvent.input(slider, { target: { value: '220' } })

    expect(onVolumeChange).toHaveBeenNthCalledWith(1, 'user-1', 0)
    expect(onVolumeChange).toHaveBeenNthCalledWith(2, 'user-1', 200)
  })
})
