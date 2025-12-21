import { invoke } from '@tauri-apps/api/core';
import type { AudioBook } from '../types/models';

export async function createAudiobookFromFiles(files: string[]): Promise<AudioBook> {
    // Backend expects { paths } per command signature
    return await invoke('create_audiobook_from_files', { paths: files });
}

export async function loadAudiobookFile(path: string): Promise<AudioBook> {
    return await invoke('load_audiobook_file', { path });
}

export async function saveAudiobookProject(audiobook: AudioBook, path: string): Promise<void> {
    return await invoke('save_audiobook_project', { audiobook, path });
}

export async function clearCurrentAudiobook(): Promise<void> {
    return await invoke('clear_current_audiobook');
}
