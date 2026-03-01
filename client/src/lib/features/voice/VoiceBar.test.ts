import { fireEvent, render } from '@testing-library/svelte'
import { describe, expect, it, vi } from 'vitest'
import VoiceBar from './VoiceBar.svelte'

describe('VoiceBar', () => {
  it('renders channel context and routes control actions', async () => {
    const onToggleMute = vi.fn()
    const onToggleDeafen = vi.fn()
    const onDisconnect = vi.fn()
    const { getByRole, getByTestId } = render(VoiceBar, {
      guildName: 'lobby',
      channelName: 'voice-room',
      connectionState: 'connected',
      isMuted: false,
      isDeafened: false,
      onToggleMute,
      onToggleDeafen,
      onDisconnect,
    })

    expect(getByTestId('voice-bar-channel')).toHaveTextContent('#voice-room')
    expect(getByTestId('voice-bar-guild')).toHaveTextContent('lobby')
    expect(getByTestId('voice-bar-quality-dot')).toHaveAttribute(
      'data-quality',
      'green',
    )

    await fireEvent.click(getByRole('button', { name: 'Mute microphone' }))
    await fireEvent.click(getByRole('button', { name: 'Deafen audio' }))
    await fireEvent.click(
      getByRole('button', { name: 'Disconnect from voice channel' }),
    )

    expect(onToggleMute).toHaveBeenCalledTimes(1)
    expect(onToggleDeafen).toHaveBeenCalledTimes(1)
    expect(onDisconnect).toHaveBeenCalledTimes(1)
  })

  it('maps connection states to quality indicator levels', async () => {
    const view = render(VoiceBar, {
      guildName: 'lobby',
      channelName: 'voice-room',
      connectionState: 'connecting',
      isMuted: false,
      isDeafened: false,
      onToggleMute: vi.fn(),
      onToggleDeafen: vi.fn(),
      onDisconnect: vi.fn(),
    })

    expect(view.getByTestId('voice-bar-quality-dot')).toHaveAttribute(
      'data-quality',
      'yellow',
    )

    await view.rerender({
      guildName: 'lobby',
      channelName: 'voice-room',
      connectionState: 'failed',
      isMuted: false,
      isDeafened: false,
      onToggleMute: vi.fn(),
      onToggleDeafen: vi.fn(),
      onDisconnect: vi.fn(),
    })

    expect(view.getByTestId('voice-bar-quality-dot')).toHaveAttribute(
      'data-quality',
      'red',
    )
  })

  it('announces control state changes via aria-live', async () => {
    const view = render(VoiceBar, {
      guildName: 'lobby',
      channelName: 'voice-room',
      connectionState: 'connected',
      isMuted: false,
      isDeafened: false,
      onToggleMute: vi.fn(),
      onToggleDeafen: vi.fn(),
      onDisconnect: vi.fn(),
    })
    expect(view.getByTestId('voice-bar-live')).toHaveTextContent(
      'Microphone active.',
    )

    await view.rerender({
      guildName: 'lobby',
      channelName: 'voice-room',
      connectionState: 'connected',
      isMuted: true,
      isDeafened: false,
      onToggleMute: vi.fn(),
      onToggleDeafen: vi.fn(),
      onDisconnect: vi.fn(),
    })
    expect(view.getByTestId('voice-bar-live')).toHaveTextContent(
      'Microphone muted.',
    )

    await view.rerender({
      guildName: 'lobby',
      channelName: 'voice-room',
      connectionState: 'connected',
      isMuted: true,
      isDeafened: true,
      onToggleMute: vi.fn(),
      onToggleDeafen: vi.fn(),
      onDisconnect: vi.fn(),
    })
    expect(view.getByTestId('voice-bar-live')).toHaveTextContent(
      'Deafened and muted.',
    )
  })
})
