import { invoke } from '@tauri-apps/api/core';
import type { AudioBook, Chapter } from '../types/models';

export async function importChapters(
    audiobookId: string,
    format: string,
    content: string
): Promise<AudioBook> {
    return await invoke('import_chapters', { audiobookId, format, content });
}

export async function exportChapters(
    audiobookId: string,
    format: string
): Promise<string> {
    return await invoke('export_chapters', { audiobookId, format });
}

export async function setChapters(
    audiobookId: string,
    chapters: Chapter[]
): Promise<AudioBook> {
    return await invoke('set_chapters', { audiobookId, chapters });
}

export async function generateChaptersFromSource(
    audiobookId: string
): Promise<AudioBook> {
    return await invoke('generate_chapters_from_source', { audiobookId });
}

export async function renameChaptersWithRegex(
    audiobookId: string,
    pattern: string,
    replacement: string
): Promise<AudioBook> {
    return await invoke('rename_chapters_with_regex', { audiobookId, pattern, replacement });
}

export interface RegexPreset {
    name: string;
    description: string;
    pattern: string;
    replacement: string;
}

export async function getChapterRegexPresets(): Promise<RegexPreset[]> {
    return await invoke('get_chapter_regex_presets');
}

