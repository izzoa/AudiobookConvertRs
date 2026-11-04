import { invoke } from '@tauri-apps/api/core';
import type { AudioBook, SpeechModelStatus } from '../types/models';

export async function getSpeechModelStatus(): Promise<SpeechModelStatus> {
    return await invoke('get_speech_model_status');
}

/** Download and install the speech model. Progress arrives as `speech_model_download_progress`. */
export async function downloadSpeechModel(): Promise<SpeechModelStatus> {
    return await invoke('download_speech_model');
}

export async function cancelSpeechModelDownload(): Promise<void> {
    return await invoke('cancel_speech_model_download');
}

export async function removeSpeechModel(): Promise<SpeechModelStatus> {
    return await invoke('remove_speech_model');
}

/** Replace the book's chapters with ones where a narrator announces a heading.
 * Progress arrives as `spoken_chapters_progress`; a cancelled run rejects with "cancelled". */
export async function detectSpokenChapters(audiobookId: string): Promise<AudioBook> {
    return await invoke('detect_spoken_chapters', { audiobookId });
}

export async function cancelSpokenChapters(): Promise<void> {
    return await invoke('cancel_spoken_chapters');
}
