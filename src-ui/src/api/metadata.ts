import { invoke } from '@tauri-apps/api/core';
import type { Metadata, Chapter, ChapterUpdate, AudioBook } from '../types/models';

export async function updateAudiobookMetadata(audiobookId: string, metadata: Metadata): Promise<AudioBook> {
    return await invoke('update_audiobook_metadata', { audiobookId, metadata });
}

export async function addChapter(audiobookId: string, chapter: Chapter): Promise<AudioBook> {
    return await invoke('add_chapter', { audiobookId, chapter });
}

export async function updateChapter(audiobookId: string, index: number, update: ChapterUpdate): Promise<AudioBook> {
    return await invoke('update_chapter', { audiobookId, index, update });
}

export async function deleteChapter(audiobookId: string, index: number): Promise<AudioBook> {
    return await invoke('delete_chapter', { audiobookId, index });
}

export async function reorderChapters(audiobookId: string, oldIndex: number, newIndex: number): Promise<AudioBook> {
    return await invoke('reorder_chapters', { audiobookId, oldIndex, newIndex });
}

export async function autoCreateChapters(audiobookId: string, count: number): Promise<AudioBook> {
    return await invoke('auto_create_chapters', { audiobookId, count });
}

export async function uploadArtwork(audiobookId: string, imagePath: string): Promise<AudioBook> {
    return await invoke('upload_artwork', { audiobookId, imagePath });
}

export async function setArtworkBase64(audiobookId: string, artworkBase64: string): Promise<AudioBook> {
    return await invoke('set_artwork_base64', { audiobookId, artworkBase64 });
}

export async function clearArtwork(audiobookId: string): Promise<AudioBook> {
    return await invoke('clear_artwork', { audiobookId });
}
