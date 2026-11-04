import { invoke } from '@tauri-apps/api/core';
import type { Waveform } from '../types/models';

export type PlaybackState = 'stopped' | 'playing' | 'paused';

export interface PlaybackStatus {
    state: PlaybackState;
    current_path: string | null;
    position_ms: number;
    duration_ms: number;
    volume: number;
}

export async function playAudio(path: string, startMs?: number): Promise<void> {
    return await invoke('play_audio', { path, startMs });
}

export async function pauseAudio(): Promise<void> {
    return await invoke('pause_audio');
}

export async function resumeAudio(): Promise<void> {
    return await invoke('resume_audio');
}

export async function stopAudio(): Promise<void> {
    return await invoke('stop_audio');
}

export async function seekAudio(positionMs: number): Promise<void> {
    return await invoke('seek_audio', { positionMs });
}

export async function setAudioVolume(volume: number): Promise<void> {
    return await invoke('set_audio_volume', { volume });
}

export async function getPlaybackStatus(): Promise<PlaybackStatus> {
    return await invoke('get_playback_status');
}

/** The book's audio `spanMs` either side of `atMs` (book time), drawn as a waveform with its pauses. */
export async function getWaveform(audiobookId: string, atMs: number, spanMs: number): Promise<Waveform> {
    return await invoke('get_waveform', { audiobookId, atMs, spanMs });
}
