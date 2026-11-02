import { invoke } from '@tauri-apps/api/core';
import type { AudioFormat } from '../types/models';

export async function pickFiles(): Promise<string[]> {
    return await invoke('pick_files');
}

export async function pickDirectory(): Promise<string | null> {
    return await invoke('pick_directory');
}

export interface ValidationResult {
    valid: boolean;
    format?: AudioFormat;
    error?: string;
}

export async function validateFiles(paths: string[]): Promise<Record<string, ValidationResult>> {
    // Backend: validate_files(paths: Vec<PathBuf>) -> AppResult<HashMap<String, ValidationResult>>
    return await invoke('validate_files', { paths });
}

export async function getFileInfo(path: string): Promise<any> {
    return await invoke('get_file_info', { path });
}

export async function pickSaveProjectFile(): Promise<string | null> {
    return await invoke('pick_save_project_file');
}

export async function pickOpenProjectFile(): Promise<string | null> {
    return await invoke('pick_open_project_file');
}

export async function pickExportSummaryFile(defaultName: string): Promise<string | null> {
    return await invoke('pick_export_summary_file', { defaultName });
}

export async function writeTextFile(path: string, content: string): Promise<void> {
    return await invoke('write_text_file', { path, content });
}

export async function pickArtworkFile(): Promise<string | null> {
    return await invoke('pick_artwork_file');
}

export async function pickChaptersFile(): Promise<string | null> {
    return await invoke('pick_chapters_file');
}

export async function readTextFile(path: string): Promise<string> {
    return await invoke('read_text_file', { path });
}

export async function pickOutputFile(defaultName?: string): Promise<string | null> {
    return await invoke('pick_output_file', { defaultName });
}

export interface CueDetectionResult {
    cue_path: string;
    audio_files: string[];
    track_count: number;
    is_multi_file: boolean;
    album_title: string | null;
    album_artist: string | null;
}

export async function detectCueFile(paths: string[]): Promise<CueDetectionResult | null> {
    return await invoke('detect_cue_file', { paths });
}

export async function findCueForAudio(path: string): Promise<string | null> {
    return await invoke('find_cue_for_audio', { path });
}

export async function pickCueFile(): Promise<string | null> {
    return await invoke('pick_cue_file');
}

export async function pickNormalizedOutputFile(defaultName?: string): Promise<string | null> {
    return await invoke('pick_normalized_output_file', { defaultName });
}

export async function revealInFileManager(path: string): Promise<void> {
    return await invoke('reveal_in_file_manager', { path });
}

/** The given paths that already exist on disk. */
export async function existingPaths(paths: string[]): Promise<string[]> {
    return await invoke('existing_paths', { paths });
}
