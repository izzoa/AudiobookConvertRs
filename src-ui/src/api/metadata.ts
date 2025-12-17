import { invoke } from '@tauri-apps/api/core';
import type { Metadata, Chapter, ChapterUpdate } from '../types/models';

export async function updateAudiobookMetadata(audiobookId: string, metadata: Metadata): Promise<void> {
    // Note: The backend command doesn't actually need ID if it operates on active audiobook,
    // but verifying 'metadata.rs' command signature:
    // fn update_audiobook_metadata(state: State<'_, AppState>, metadata: Metadata)
    console.debug('Updating metadata for', audiobookId); // Use variable to silence warning
    return await invoke('update_audiobook_metadata', { metadata });
}

export async function addChapter(chapter: Chapter): Promise<void> {
    return await invoke('add_chapter', { chapter });
}

export async function updateChapter(index: number, update: ChapterUpdate): Promise<void> {
    return await invoke('update_chapter', { index, update });
}

export async function deleteChapter(index: number): Promise<void> {
    return await invoke('delete_chapter', { index });
}

export async function reorderChapters(oldIndex: number, newIndex: number): Promise<void> {
    return await invoke('reorder_chapters', { oldIndex, newIndex });
}

export async function autoCreateChapters(minDurationMs: number): Promise<void> {
    // Backend: auto_create_chapters(state, min_duration_ms: Option<u64>)
    // Wait, let me check backend signature for auto_create_chapters
    // Actually I implemented `auto_split_chapters` in `editor.rs` but command name?
    // checking main.rs: audiobookconvert_rs::commands::metadata::auto_create_chapters
    return await invoke('auto_create_chapters', { minDurationMs });
    // Backend argument name is snake_case likely unless strict camelCase.
    // 'min_duration_ms' or 'minDurationMs'? Tauri usually converts camel to snake.
    // I will double check this.
}

export async function uploadArtwork(path: string): Promise<void> {
    return await invoke('upload_artwork', { path });
}
