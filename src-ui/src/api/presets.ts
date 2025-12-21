import { invoke } from '@tauri-apps/api/core';
import type { ConversionPreset, DryRunResult, ConversionSettings } from '../types/models';

export async function listPresets(): Promise<ConversionPreset[]> {
    return await invoke('list_presets');
}

export async function getPreset(name: string): Promise<ConversionPreset> {
    return await invoke('get_preset', { name });
}

export async function savePreset(preset: ConversionPreset): Promise<void> {
    return await invoke('save_preset', { preset });
}

export async function deletePreset(name: string): Promise<boolean> {
    return await invoke('delete_preset', { name });
}

export async function dryRunConversion(settings: ConversionSettings): Promise<DryRunResult> {
    return await invoke('dry_run_conversion', { settings });
}
