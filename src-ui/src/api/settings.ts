import { invoke } from '@tauri-apps/api/core';
import type { UserSettings, FFmpegStatus } from '../types/models';

export async function getSettings(): Promise<UserSettings> {
    return await invoke('get_settings');
}

export async function updateSettings(settings: UserSettings): Promise<void> {
    return await invoke('update_settings', { settings });
}

export async function getDefaultSettings(): Promise<UserSettings> {
    return await invoke('get_default_settings');
}

export async function resetSettings(): Promise<void> {
    return await invoke('reset_settings');
}

export async function checkFFmpegAvailable(): Promise<boolean> {
    return await invoke('check_ffmpeg_available');
}

export async function getFFmpegVersion(): Promise<string> {
    return await invoke('get_ffmpeg_version');
}

export async function getFFmpegInfo(): Promise<FFmpegStatus> {
    return await invoke('get_ffmpeg_info');
}
