import { invoke } from '@tauri-apps/api/core';
import type { AudioBook, BookGroup } from '../types/models';

/**
 * Loads files as one book. `activate: false` keeps the book that is open for
 * editing (and undo) where it is — batch imports load in the background.
 */
export async function createAudiobookFromFiles(files: string[], activate = true): Promise<AudioBook> {
    return await invoke('create_audiobook_from_files', { paths: files, activate });
}

/** Splits dropped files and folders into books, without reading any audio. */
export async function groupPathsIntoBooks(paths: string[]): Promise<BookGroup[]> {
    return await invoke('group_paths_into_books', { paths });
}

/** Makes a loaded book the one that edits and undo apply to. */
export async function setActiveAudiobookId(audiobookId: string): Promise<void> {
    return await invoke('set_active_audiobook', { audiobookId });
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

export async function getActiveAudiobook(): Promise<AudioBook | null> {
    return await invoke('get_active_audiobook');
}
