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

export async function pauseConversion(jobId: string): Promise<boolean> {
    return await invoke('pause_conversion', { jobId });
}

export async function resumeConversion(jobId: string): Promise<boolean> {
    return await invoke('resume_conversion', { jobId });
}
