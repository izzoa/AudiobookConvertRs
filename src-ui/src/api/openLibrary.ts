import { invoke } from '@tauri-apps/api/core';
import type { AudioBook, OpenLibrarySearchResult, OpenLibrarySelection } from '../types/models';

export async function openLibrarySearch(
    query: string,
    limit?: number
): Promise<OpenLibrarySearchResult[]> {
    return await invoke('open_library_search', { query, limit });
}

export async function openLibraryApply(
    audiobookId: string,
    selection: OpenLibrarySelection,
    downloadCover: boolean = true
): Promise<AudioBook> {
    return await invoke('open_library_apply', { 
        audiobookId, 
        selection, 
        downloadCover 
    });
}

