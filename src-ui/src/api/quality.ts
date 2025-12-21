import { invoke } from '@tauri-apps/api/core';
import type { AudioLevels, NormalizationSuggestion, NormalizationSettings } from '../types/models';

export async function analyzeAudioLevels(filePath: string): Promise<AudioLevels> {
    return await invoke('analyze_audio_levels', { filePath });
}

export async function suggestNormalization(filePath: string): Promise<NormalizationSuggestion> {
    return await invoke('suggest_normalization', { filePath });
}

export async function applyAudioNormalization(
    input: string,
    output: string,
    settings: NormalizationSettings
): Promise<void> {
    return await invoke('apply_audio_normalization', { input, output, settings });
}
