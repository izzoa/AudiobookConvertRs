import { createContext, useContext } from 'react';
import type { ConversionJob, QueueState } from '../types/models';

// Lives apart from ConversionStoreProvider so that editing the provider hot-reloads in place.

export interface ConversionStoreState {
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

export const ConversionStoreContext = createContext<ConversionStoreState | undefined>(undefined);

export function useConversionStore() {
    const context = useContext(ConversionStoreContext);
    if (context === undefined) {
        throw new Error('useConversionStore must be used within a ConversionStoreProvider');
    }
    return context;
}
