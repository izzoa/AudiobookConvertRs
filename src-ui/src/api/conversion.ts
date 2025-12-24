import { invoke } from '@tauri-apps/api/core';
import type { ConversionSettings, ConversionJob } from '../types/models';

export async function startConversion(audiobookId: string, settings: ConversionSettings): Promise<string> {
    // Backend: start_conversion(audiobook_id, settings) -> AppResult<String> (job_id)
    return await invoke('start_conversion', { audiobookId, settings });
}

export async function cancelConversion(jobId: string): Promise<void> {
    return await invoke('cancel_conversion', { jobId });
}

export async function getConversionStatus(jobId: string): Promise<ConversionJob> {
    return await invoke('get_conversion_status', { jobId });
}
