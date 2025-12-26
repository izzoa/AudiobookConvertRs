import { invoke } from '@tauri-apps/api/core';
import type {
    AudioLevels,
    NormalizationSuggestion,
    NormalizationSettings,
    FileSizeEstimate,
    QualityPresetInfo
} from '../types/models';

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

export async function applyAudioNormalizationWithProgress(
    input: string,
    output: string,
    settings: NormalizationSettings,
    totalDurationMs: number
): Promise<void> {
    return await invoke('apply_audio_normalization_with_progress', {
        input,
        output,
        settings,
        totalDurationMs
    });
}

export async function estimateOutputSize(durationMs: number, bitrateKbps: number): Promise<FileSizeEstimate> {
    return await invoke('estimate_output_size', { durationMs, bitrateKbps });
}

export async function getQualityPresetInfo(): Promise<QualityPresetInfo[]> {
    return await invoke('get_quality_preset_info');
}
