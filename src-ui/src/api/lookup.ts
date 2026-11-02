import { invoke } from '@tauri-apps/api/core';
import type { AudioBook, LookupCandidate, LookupRecord, LookupSource } from '../types/models';
import type { ApplyPayload } from '../lib/lookupReview';

/** Searches the Audible catalog; the region defaults to the one in settings. */
export async function audibleSearch(title: string, author?: string, region?: string): Promise<LookupCandidate[]> {
    return await invoke('audible_search', { title, author: author || null, region: region || null });
}

export async function openLibrarySearch(query: string, limit = 10): Promise<LookupCandidate[]> {
    return await invoke('open_library_search', { query, limit });
}

/** One result's full record, with its chapters fitted to the given book. */
export async function lookupRecord(
    source: LookupSource,
    id: string,
    audiobookId: string,
    region?: string
): Promise<LookupRecord> {
    return await invoke('lookup_record', { source, id, region: region || null, audiobookId });
}

/** Applies the ticked review rows as one undo step. */
export async function applyLookup(audiobookId: string, source: LookupSource, payload: ApplyPayload): Promise<AudioBook> {
    return await invoke('apply_lookup', {
        audiobookId,
        source,
        fields: payload.fields,
        coverUrl: payload.coverUrl ?? null,
        chapters: payload.chapters ?? null,
    });
}
