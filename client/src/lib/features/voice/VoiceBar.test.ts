import { fireEvent, render } from '@testing-library/svelte'
import { describe, expect, it, vi } from 'vitest'
import VoiceBar from './VoiceBar.svelte'

describe('VoiceBar', () => {
  it('renders channel context and routes control actions', async () => {
    const onToggleParticipants = vi.fn()
    const onToggleMute = vi.fn()
    const onToggleDeafen = vi.fn()
    const onDisconnect = vi.fn()
    const { getByRole, getByTestId } = render(VoiceBar, {
      guildName: 'lobby',
      channelName: 'voice-room',
      connectionState: 'connected',
      isMuted: false,
      isDeafened: false,
      isParticipantsOpen: false,
      onToggleParticipants,
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

    await fireEvent.click(
      getByRole('button', { name: 'Show voice participants' }),
    )
    await fireEvent.click(getByRole('button', { name: 'Mute microphone' }))
    await fireEvent.click(getByRole('button', { name: 'Deafen audio' }))
    await fireEvent.click(
      getByRole('button', { name: 'Disconnect from voice channel' }),
    )

    expect(onToggleParticipants).toHaveBeenCalledTimes(1)
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
      isParticipantsOpen: false,
      onToggleParticipants: vi.fn(),
      onToggleMute: vi.fn(),
      onToggleDeafen: vi.fn(),
      onDisconnect: vi.fn(),
    })

    expect(view.getByTestId('voice-bar-quality-dot')).toHaveAttribute(
      'data-quality',
      'yellow',
    )
    expect(view.queryByTestId('voice-bar-status')).not.toBeInTheDocument()

    await view.rerender({
      guildName: 'lobby',
      channelName: 'voice-room',
      connectionState: 'retrying',
      isMuted: false,
      isDeafened: false,
      isParticipantsOpen: false,
      onToggleParticipants: vi.fn(),
      onToggleMute: vi.fn(),
      onToggleDeafen: vi.fn(),
      onDisconnect: vi.fn(),
    })

    expect(view.getByTestId('voice-bar-quality-dot')).toHaveAttribute(
      'data-quality',
      'yellow',
    )
    expect(view.getByTestId('voice-bar-quality-dot')).toHaveClass(
      'motion-safe:animate-pulse',
    )
    expect(view.getByTestId('voice-bar-status')).toHaveTextContent(
      'Reconnecting...',
    )

    await view.rerender({
      guildName: 'lobby',
      channelName: 'voice-room',
      connectionState: 'failed',
      isMuted: false,
      isDeafened: false,
      isParticipantsOpen: false,
      onToggleParticipants: vi.fn(),
      onToggleMute: vi.fn(),
      onToggleDeafen: vi.fn(),
      onDisconnect: vi.fn(),
    })

    expect(view.getByTestId('voice-bar-quality-dot')).toHaveAttribute(
      'data-quality',
      'red',
    )
    expect(view.getByTestId('voice-bar-status')).toHaveTextContent(
      'Connection lost',
    )
  })

  it('announces control state changes via aria-live', async () => {
    const view = render(VoiceBar, {
      guildName: 'lobby',
      channelName: 'voice-room',
      connectionState: 'connected',
      isMuted: false,
      isDeafened: false,
      isParticipantsOpen: false,
      onToggleParticipants: vi.fn(),
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
      isParticipantsOpen: true,
      onToggleParticipants: vi.fn(),
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
      isParticipantsOpen: true,
      onToggleParticipants: vi.fn(),
      onToggleMute: vi.fn(),
      onToggleDeafen: vi.fn(),
      onDisconnect: vi.fn(),
    })
    expect(view.getByTestId('voice-bar-live')).toHaveTextContent(
      'Deafened and muted.',
    )
  })

  it('supports compact mobile voice controls and sheet open action', async () => {
    const onToggleMute = vi.fn()
    const onOpenSheet = vi.fn()
    const view = render(VoiceBar, {
      guildName: 'lobby',
      channelName: 'voice-room',
      connectionState: 'connected',
      variant: 'mobile',
      isMuted: false,
      isDeafened: false,
      isParticipantsOpen: false,
      onToggleParticipants: vi.fn(),
      onToggleMute,
      onToggleDeafen: vi.fn(),
      onDisconnect: vi.fn(),
      onOpenSheet,
    })

    expect(view.getByTestId('voice-bar-channel')).toHaveTextContent(
      '#voice-room',
    )
    expect(view.queryByTestId('voice-bar-quality-dot')).not.toBeInTheDocument()
    expect(
      view.queryByTestId('voice-bar-toggle-participants'),
    ).not.toBeInTheDocument()
    expect(
      view.queryByTestId('voice-bar-toggle-deafen'),
    ).not.toBeInTheDocument()
    expect(view.queryByTestId('voice-bar-disconnect')).not.toBeInTheDocument()

    await fireEvent.click(view.getByTestId('voice-bar-open-sheet'))
    await fireEvent.click(view.getByTestId('voice-bar-toggle-mute'))
    expect(onOpenSheet).toHaveBeenCalledTimes(1)
    expect(onToggleMute).toHaveBeenCalledTimes(1)
  })
})
