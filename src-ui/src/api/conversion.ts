import { invoke } from '@tauri-apps/api/core';
import type { ConversionSettings, ConversionJob, CopyCheck, QueueState } from '../types/models';

export async function startConversion(audiobookId: string, settings: ConversionSettings): Promise<string> {
    return await invoke('start_conversion', { audiobookId, settings });
}

export async function enqueueConversion(audiobookId: string, settings: ConversionSettings): Promise<string> {
    // Backend: enqueue_conversion(audiobook_id, settings) -> AppResult<String> (job_id)
    return await invoke('enqueue_conversion', { audiobookId, settings });
}

export async function startConversionQueue(maxConcurrent?: number): Promise<void> {
    // Backend: start_conversion_queue(max_concurrent) -> AppResult<()>
    return await invoke('start_conversion_queue', { maxConcurrent });
}

export async function stopConversionQueue(): Promise<void> {
    // Backend: stop_conversion_queue() -> AppResult<()>
    return await invoke('stop_conversion_queue');
}

export async function listConversionJobs(): Promise<ConversionJob[]> {
    // Backend: list_conversion_jobs() -> AppResult<Vec<ConversionJob>>
    return await invoke('list_conversion_jobs');
}

export async function removeConversionJob(jobId: string): Promise<void> {
    // Backend: remove_conversion_job(job_id) -> AppResult<()>
    return await invoke('remove_conversion_job', { jobId });
}

export async function clearCompletedJobs(): Promise<void> {
    // Backend: clear_completed_jobs() -> AppResult<()>
    return await invoke('clear_completed_jobs');
}

export async function clearPendingJobs(): Promise<void> {
    // Backend: clear_pending_jobs() -> AppResult<()>
    return await invoke('clear_pending_jobs');
}

export async function cancelConversion(jobId: string): Promise<void> {
    return await invoke('cancel_conversion', { jobId });
}

export async function getConversionStatus(jobId: string): Promise<ConversionJob> {
    return await invoke('get_conversion_status', { jobId });
}

export async function getConversionQueueState(): Promise<QueueState> {
    return await invoke('get_conversion_queue_state');
}

export async function retryConversion(jobId: string): Promise<string> {
    return await invoke('retry_conversion', { jobId });
}

/** Whether the book's audio can go into the M4B as it is, without re-encoding. */
export async function checkAudioCopy(audiobookId: string): Promise<CopyCheck> {
    return await invoke('check_audio_copy', { audiobookId });
}
