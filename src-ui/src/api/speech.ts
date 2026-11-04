import { invoke } from '@tauri-apps/api/core';
import type { SpeechModelStatus } from '../types/models';

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
