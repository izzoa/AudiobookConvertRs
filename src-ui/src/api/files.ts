import { invoke } from '@tauri-apps/api/core';
import type { AudioFormat } from '../types/models';

export async function pickFiles(): Promise<string[]> {
    return await invoke('pick_files');
}

export async function pickDirectory(): Promise<string[]> {
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
