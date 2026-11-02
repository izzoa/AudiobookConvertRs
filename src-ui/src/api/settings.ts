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

/** `refresh` searches again instead of returning the cached result. */
export async function getFFmpegInfo(refresh = false): Promise<FFmpegStatus> {
    return await invoke('get_ffmpeg_info', { refresh });
}

/** Choose the FFmpeg executable, or clear the choice with `null`. Rejects with the reason when it fails the check. */
export async function setFFmpegPath(path: string | null): Promise<FFmpegStatus> {
    return await invoke('set_ffmpeg_path', { path });
}

/** Download, verify and install this platform's FFmpeg build. Progress arrives as `ffmpeg_download_progress`. */
export async function downloadFFmpeg(): Promise<FFmpegStatus> {
    return await invoke('download_ffmpeg');
}

/** Stop a running download; `downloadFFmpeg` then rejects with "download cancelled". */
export async function cancelFFmpegDownload(): Promise<void> {
    return await invoke('cancel_ffmpeg_download');
}

/** Delete the downloaded FFmpeg and search again. */
export async function removeDownloadedFFmpeg(): Promise<FFmpegStatus> {
    return await invoke('remove_downloaded_ffmpeg');
}
