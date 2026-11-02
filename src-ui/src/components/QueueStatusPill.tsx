import { useState, useEffect } from 'react';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { useApp } from '@/context/useApp';
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
                /* the pill simply stays hidden until the backend answers */
            }
        };

        refreshState();
        const interval = setInterval(refreshState, 5000);
        return () => clearInterval(interval);
    }, []);

    useEffect(() => {
        const refresh = () => ConversionApi.getConversionQueueState().then(setQueueState).catch(() => {});
        const pending: Promise<UnlistenFn>[] = [
            listen<ConversionProgress>('conversion_progress', (event) => {
                setTopJobProgress(event.payload.total_progress);
                setQueueState((prev) => (prev ? { ...prev, active_jobs: Math.max(1, prev.active_jobs) } : null));
            }),
            listen('conversion_complete', () => {
                setTopJobProgress(null);
                refresh();
            }),
            listen('conversion_error', () => {
                setTopJobProgress(null);
                refresh();
            }),
        ];

        // Listeners resolve asynchronously, so detach them whenever they land —
        // including after an unmount that happened first.
        return () => {
            for (const p of pending) p.then((unlisten) => unlisten() as unknown).catch(() => {});
        };
    }, []);

    const hasActiveJobs = queueState && (queueState.active_jobs > 0 || queueState.pending_jobs > 0);

    if (!hasActiveJobs) {
        return null;
    }

    const totalJobs = (queueState?.active_jobs || 0) + (queueState?.pending_jobs || 0);

    return (
        <button
            onClick={() => setView('queue')}
            className="flex items-center gap-1.5 h-7 px-2.5 rounded-full bg-primary/10 text-primary text-xs font-medium nums hover:bg-primary/20 transition-colors"
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
