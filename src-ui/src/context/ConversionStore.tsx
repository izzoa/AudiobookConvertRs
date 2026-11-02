import { createContext, useContext, useState, useEffect, useCallback, type ReactNode } from 'react';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import type { ConversionJob, ConversionProgress, QueueState } from '../types/models';
import { JobStatus } from '../types/models';
import * as ConversionApi from '../api/conversion';

interface ConversionStoreState {
    jobs: ConversionJob[];
    queueState: QueueState | null;
    selectedJobId: string | null;
    isLoading: boolean;
    error: string | null;
    selectJob: (jobId: string | null) => void;
    refreshJobs: () => Promise<void>;
    refreshQueueState: () => Promise<void>;
    cancelJob: (jobId: string) => Promise<void>;
    retryJob: (jobId: string) => Promise<string>;
    removeJob: (jobId: string) => Promise<void>;
    clearCompleted: () => Promise<void>;
}

const ConversionStoreContext = createContext<ConversionStoreState | undefined>(undefined);

const FALLBACK_REFRESH_INTERVAL = 5000;

export function ConversionStoreProvider({ children }: { children: ReactNode }) {
    const [jobs, setJobs] = useState<ConversionJob[]>([]);
    const [queueState, setQueueState] = useState<QueueState | null>(null);
    const [selectedJobId, setSelectedJobId] = useState<string | null>(null);
    const [isLoading, setIsLoading] = useState(true);
    const [error, setError] = useState<string | null>(null);

    const refreshJobs = useCallback(async () => {
        try {
            const jobList = await ConversionApi.listConversionJobs();
            setJobs((prev) =>
                jobList.map((serverJob) => {
                    const existing = prev.find((j) => j.id === serverJob.id);
                    if (serverJob.status === JobStatus.Running && !serverJob.progress && existing?.progress) {
                        return {
                            ...serverJob,
                            progress: existing.progress,
                            progress_percent: serverJob.progress_percent || existing.progress_percent,
                            status_message: serverJob.status_message || existing.status_message,
                        };
                    }
                    return serverJob;
                })
            );
            setError(null);
        } catch (err) {
            console.error('Failed to refresh jobs:', err);
            setError('Failed to load conversion jobs');
        }
    }, []);

    const refreshQueueState = useCallback(async () => {
        try {
            const state = await ConversionApi.getConversionQueueState();
            setQueueState(state);
        } catch (err) {
            console.error('Failed to refresh queue state:', err);
        }
    }, []);

    const selectJob = useCallback((jobId: string | null) => {
        setSelectedJobId(jobId);
    }, []);

    const cancelJob = useCallback(async (jobId: string) => {
        await ConversionApi.cancelConversion(jobId);
        await refreshJobs();
    }, [refreshJobs]);

    const retryJob = useCallback(async (jobId: string) => {
        const newJobId = await ConversionApi.retryConversion(jobId);
        await refreshJobs();
        return newJobId;
    }, [refreshJobs]);

    const removeJob = useCallback(async (jobId: string) => {
        await ConversionApi.removeConversionJob(jobId);
        if (selectedJobId === jobId) {
            setSelectedJobId(null);
        }
        await refreshJobs();
    }, [refreshJobs, selectedJobId]);

    const clearCompleted = useCallback(async () => {
        await ConversionApi.clearCompletedJobs();
        await refreshJobs();
    }, [refreshJobs]);

    useEffect(() => {
        const loadInitial = async () => {
            setIsLoading(true);
            await Promise.all([refreshJobs(), refreshQueueState()]);
            setIsLoading(false);
        };
        loadInitial();
    }, [refreshJobs, refreshQueueState]);

    useEffect(() => {
        const unlisteners: UnlistenFn[] = [];

        const setupListeners = async () => {
            unlisteners.push(
                await listen<ConversionProgress>('conversion_progress', (event) => {
                    const progress = event.payload;
                    setJobs((prev) =>
                        prev.map((job) =>
                            job.id === progress.job_id
                                ? {
                                      ...job,
                                      progress_percent: progress.total_progress,
                                      current_file_index: progress.file_index,
                                      progress: progress,
                                      status: JobStatus.Running,
                                  }
                                : job
                        )
                    );
                })
            );

            unlisteners.push(
                await listen<{ job_id: string; output_path: string; duration_ms: number }>(
                    'conversion_complete',
                    (event) => {
                        const { job_id, output_path } = event.payload;
                        setJobs((prev) =>
                            prev.map((job) =>
                                job.id === job_id
                                    ? {
                                          ...job,
                                          status: JobStatus.Complete,
                                          progress_percent: 100,
                                          output_path,
                                          completed_at: new Date().toISOString(),
                                      }
                                    : job
                            )
                        );
                        refreshQueueState();
                    }
                )
            );

            unlisteners.push(
                await listen<{ job_id: string; error_message: string; recoverable: boolean }>(
                    'conversion_error',
                    (event) => {
                        const { job_id, error_message } = event.payload;
                        setJobs((prev) =>
                            prev.map((job) =>
                                job.id === job_id
                                    ? {
                                          ...job,
                                          status: JobStatus.Error,
                                          error_message,
                                          completed_at: new Date().toISOString(),
                                      }
                                    : job
                            )
                        );
                        refreshQueueState();
                    }
                )
            );
        };

        setupListeners();

        return () => {
            unlisteners.forEach((unlisten) => unlisten());
        };
    }, [refreshQueueState]);

    useEffect(() => {
        const interval = setInterval(() => {
            refreshJobs();
            refreshQueueState();
        }, FALLBACK_REFRESH_INTERVAL);

        return () => clearInterval(interval);
    }, [refreshJobs, refreshQueueState]);

    return (
        <ConversionStoreContext.Provider
            value={{
                jobs,
                queueState,
                selectedJobId,
                isLoading,
                error,
                selectJob,
                refreshJobs,
                refreshQueueState,
                cancelJob,
                retryJob,
                removeJob,
                clearCompleted,
            }}
        >
            {children}
        </ConversionStoreContext.Provider>
    );
}

export function useConversionStore() {
    const context = useContext(ConversionStoreContext);
    if (context === undefined) {
        throw new Error('useConversionStore must be used within a ConversionStoreProvider');
    }
    return context;
}
