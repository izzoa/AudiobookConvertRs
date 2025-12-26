import { useState, useEffect } from 'react';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { useApp } from '@/context/AppContext';
import * as ConversionApi from '@/api/conversion';
import type { QueueState, ConversionProgress } from '@/types/models';
import { PlayIcon, ClockIcon } from '@heroicons/react/24/outline';

export function QueueStatusPill() {
    const { setView } = useApp();
    const [queueState, setQueueState] = useState<QueueState | null>(null);
    const [topJobProgress, setTopJobProgress] = useState<number | null>(null);

    useEffect(() => {
        const refreshState = async () => {
            try {
                const state = await ConversionApi.getConversionQueueState();
                setQueueState(state);
            } catch {
            }
        };

        refreshState();
        const interval = setInterval(refreshState, 5000);
        return () => clearInterval(interval);
    }, []);

    useEffect(() => {
        const unlisteners: UnlistenFn[] = [];

        const setup = async () => {
            unlisteners.push(
                await listen<ConversionProgress>('conversion_progress', (event) => {
                    setTopJobProgress(event.payload.total_progress);
                    setQueueState((prev) =>
                        prev ? { ...prev, active_jobs: Math.max(1, prev.active_jobs) } : null
                    );
                })
            );

            unlisteners.push(
                await listen('conversion_complete', () => {
                    setTopJobProgress(null);
                    ConversionApi.getConversionQueueState().then(setQueueState).catch(() => {});
                })
            );

            unlisteners.push(
                await listen('conversion_error', () => {
                    setTopJobProgress(null);
                    ConversionApi.getConversionQueueState().then(setQueueState).catch(() => {});
                })
            );
        };

        setup();

        return () => {
            unlisteners.forEach((fn) => fn());
        };
    }, []);

    const hasActiveJobs = queueState && (queueState.active_jobs > 0 || queueState.pending_jobs > 0);

    if (!hasActiveJobs) {
        return null;
    }

    const totalJobs = (queueState?.active_jobs || 0) + (queueState?.pending_jobs || 0);

    return (
        <button
            onClick={() => setView('jobs')}
            className="flex items-center gap-1.5 px-2.5 py-1 rounded-full bg-primary/10 text-primary text-xs font-medium hover:bg-primary/20 transition-colors"
        >
            {queueState?.active_jobs ? (
                <>
                    <PlayIcon className="w-3.5 h-3.5" />
                    <span>
                        {topJobProgress !== null ? `${Math.round(topJobProgress)}%` : 'Converting'}
                    </span>
                    {totalJobs > 1 && (
                        <span className="text-primary/70">+{totalJobs - 1}</span>
                    )}
                </>
            ) : (
                <>
                    <ClockIcon className="w-3.5 h-3.5" />
                    <span>{queueState?.pending_jobs} queued</span>
                </>
            )}
        </button>
    );
}
