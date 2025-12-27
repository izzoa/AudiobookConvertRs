import React, { useState, useEffect } from 'react';
import { Button } from './ui/button';
import { Progress } from './ui/progress';
import { Input } from './ui/input';
import { Label } from './ui/label';
import { useToast } from './ui/use-toast';
import {
    Play,
    Square,
    Trash2,
    Plus,
    RefreshCw,
    CheckCircle,
    XCircle,
    Clock,
    Loader2,
    ArrowLeft
} from 'lucide-react';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { JobStatus } from '../types/models';
import type { ConversionJob, ConversionSettings } from '../types/models';
import {
    enqueueConversion,
    startConversionQueue,
    stopConversionQueue,
    listConversionJobs,
    removeConversionJob,
    clearCompletedJobs,
    clearPendingJobs,
    cancelConversion
} from '../api/conversion';
import { useApp } from '@/context/AppContext';
import { ConversionSettingsDialog } from './ConversionSettingsDialog';

interface ConversionProgressEvent {
    job_id: string;
    current_file: string;
    file_progress: number;
    total_progress: number;
    time_remaining_sec?: number;
}

interface ConversionCompleteEvent {
    job_id: string;
    output_path: string;
    duration_ms: number;
}

interface ConversionErrorEvent {
    job_id: string;
    error_message: string;
    recoverable: boolean;
}

export const ConversionQueuePanel: React.FC = () => {
    const { activeAudiobook, settings, setPendingConversionSettings, setView } = useApp();
    const audiobook = activeAudiobook;
    const { toast } = useToast();

    const [jobs, setJobs] = useState<ConversionJob[]>([]);
    const [isQueueRunning, setIsQueueRunning] = useState(false);
    const [maxConcurrent, setMaxConcurrent] = useState(settings?.concurrent_jobs ?? 2);
    const [isLoading, setIsLoading] = useState(false);
    const [showSettings, setShowSettings] = useState(false);
    const [conversionSettings, setConversionSettings] = useState<ConversionSettings | null>(null);
    const [jobTimeRemaining, setJobTimeRemaining] = useState<Record<string, number>>({});

    // Update maxConcurrent when settings change
    React.useEffect(() => {
        if (settings?.concurrent_jobs) {
            setMaxConcurrent(settings.concurrent_jobs);
        }
    }, [settings?.concurrent_jobs]);

    // Load jobs on mount and set up event listeners + polling
    useEffect(() => {
        loadJobs();

        // Set up event listeners
        let unlisteners: UnlistenFn[] = [];

        const setupListeners = async () => {
            // Progress events
            unlisteners.push(await listen<ConversionProgressEvent>('conversion_progress', (event) => {
                console.log('Progress event received:', event.payload);
                updateJobProgress(event.payload);
            }));

            // Completion events
            unlisteners.push(await listen<ConversionCompleteEvent>('conversion_complete', (event) => {
                console.log('Completion event received:', event.payload);
                handleJobComplete(event.payload);
            }));

            // Error events
            unlisteners.push(await listen<ConversionErrorEvent>('conversion_error', (event) => {
                console.log('Error event received:', event.payload);
                handleJobError(event.payload);
            }));
        };

        setupListeners();

        // Poll for updates every second as fallback
        const pollInterval = setInterval(() => {
            loadJobs();
        }, 1000);

        // Cleanup
        return () => {
            console.log('Cleaning up event listeners');
            unlisteners.forEach(unlisten => unlisten());
            clearInterval(pollInterval);
        };
    }, []);

    const loadJobs = async () => {
        try {
            const jobList = await listConversionJobs();
            setJobs(jobList);

            // Check if any jobs are running
            const hasRunning = jobList.some(job => job.status === JobStatus.Running);
            setIsQueueRunning(hasRunning);
        } catch (error) {
            console.error('Failed to load jobs:', error);
            toast({
                title: 'Error',
                description: 'Failed to load conversion jobs',
                variant: 'destructive',
            });
        }
    };

    const updateJobProgress = (progress: ConversionProgressEvent) => {
        setJobs(prevJobs => {
            return prevJobs.map(job => {
                if (job.id === progress.job_id) {
                    return {
                        ...job,
                        progress_percent: progress.total_progress,
                        status_message: `Processing: ${progress.current_file}`,
                    };
                }
                return job;
            });
        });
        // Update time remaining separately
        console.log('Progress time_remaining_sec:', progress.time_remaining_sec);
        if (progress.time_remaining_sec !== undefined) {
            console.log('Setting time remaining for', progress.job_id, ':', progress.time_remaining_sec);
            setJobTimeRemaining(prev => ({
                ...prev,
                [progress.job_id]: progress.time_remaining_sec!
            }));
        } else {
            console.log('time_remaining_sec is undefined');
        }
    };

    const handleJobComplete = (complete: ConversionCompleteEvent) => {
        setJobs(prevJobs =>
            prevJobs.map(job =>
                job.id === complete.job_id
                    ? {
                        ...job,
                        status: JobStatus.Complete,
                        progress_percent: 100,
                        output_path: complete.output_path,
                    }
                    : job
            )
        );

        toast({
            title: 'Conversion Complete',
            description: `Output saved to: ${complete.output_path}`,
        });

        // Reload jobs to get updated state
        loadJobs();
    };

    const handleJobError = (error: ConversionErrorEvent) => {
        setJobs(prevJobs =>
            prevJobs.map(job =>
                job.id === error.job_id
                    ? {
                        ...job,
                        status: JobStatus.Error,
                        error_message: error.error_message,
                    }
                    : job
            )
        );

        toast({
            title: 'Conversion Failed',
            description: error.error_message,
            variant: 'destructive',
        });

        // Reload jobs to get updated state
        loadJobs();
    };

    const handleEnqueue = async () => {
        if (!audiobook || !conversionSettings) {
            toast({
                title: 'Error',
                description: 'Please load an audiobook and configure settings first',
                variant: 'destructive',
            });
            return;
        }

        try {
            setIsLoading(true);
            const jobId = await enqueueConversion(audiobook.id, conversionSettings);
            toast({
                title: 'Job Enqueued',
                description: `Job ${jobId} added to queue`,
            });
            await loadJobs();
        } catch (error) {
            console.error('Failed to enqueue job:', error);
            toast({
                title: 'Error',
                description: `Failed to enqueue job: ${error}`,
                variant: 'destructive',
            });
        } finally {
            setIsLoading(false);
        }
    };

    const handleStartQueue = async () => {
        try {
            setIsLoading(true);
            await startConversionQueue(maxConcurrent);
            setIsQueueRunning(true);
            toast({
                title: 'Queue Started',
                description: `Processing with ${maxConcurrent} concurrent workers`,
            });
        } catch (error) {
            console.error('Failed to start queue:', error);
            toast({
                title: 'Error',
                description: `Failed to start queue: ${error}`,
                variant: 'destructive',
            });
        } finally {
            setIsLoading(false);
        }
    };

    const handleStopQueue = async () => {
        try {
            setIsLoading(true);
            await stopConversionQueue();
            setIsQueueRunning(false);
            toast({
                title: 'Queue Stopped',
                description: 'Conversion queue has been stopped',
            });
        } catch (error) {
            console.error('Failed to stop queue:', error);
            toast({
                title: 'Error',
                description: `Failed to stop queue: ${error}`,
                variant: 'destructive',
            });
        } finally {
            setIsLoading(false);
        }
    };

    const handleCancelJob = async (jobId: string) => {
        try {
            await cancelConversion(jobId);
            toast({
                title: 'Job Cancelled',
                description: `Job ${jobId} has been cancelled`,
            });
            await loadJobs();
        } catch (error) {
            console.error('Failed to cancel job:', error);
            toast({
                title: 'Error',
                description: `Failed to cancel job: ${error}`,
                variant: 'destructive',
            });
        }
    };

    const handleRemoveJob = async (jobId: string) => {
        try {
            await removeConversionJob(jobId);
            await loadJobs();
        } catch (error) {
            console.error('Failed to remove job:', error);
            toast({
                title: 'Error',
                description: `Failed to remove job: ${error}`,
                variant: 'destructive',
            });
        }
    };

    const handleClearCompleted = async () => {
        try {
            await clearCompletedJobs();
            toast({
                title: 'Completed Jobs Cleared',
                description: 'All completed jobs have been removed',
            });
            await loadJobs();
        } catch (error) {
            console.error('Failed to clear completed jobs:', error);
            toast({
                title: 'Error',
                description: `Failed to clear completed jobs: ${error}`,
                variant: 'destructive',
            });
        }
    };

    const handleClearQueue = async () => {
        try {
            await clearPendingJobs();
            toast({
                title: 'Queue Cleared',
                description: 'All pending jobs have been removed',
            });
            await loadJobs();
        } catch (error) {
            console.error('Failed to clear queue:', error);
            toast({
                title: 'Error',
                description: `Failed to clear queue: ${error}`,
                variant: 'destructive',
            });
        }
    };

    const getStatusIcon = (status: string) => {
        switch (status) {
            case JobStatus.Queued:
                return <Clock className="w-4 h-4" />;
            case JobStatus.Running:
                return <Loader2 className="w-4 h-4 animate-spin" />;
            case JobStatus.Complete:
                return <CheckCircle className="w-4 h-4 text-green-500" />;
            case JobStatus.Error:
                return <XCircle className="w-4 h-4 text-red-500" />;
            case JobStatus.Cancelled:
                return <XCircle className="w-4 h-4 text-orange-500" />;
            default:
                return null;
        }
    };

    const pendingJobs = jobs.filter(job => job.status === JobStatus.Queued);
    const activeJobs = jobs.filter(job => job.status === JobStatus.Running);
    const completedJobs = jobs.filter(job =>
        job.status === JobStatus.Complete || job.status === JobStatus.Error || job.status === JobStatus.Cancelled
    );

    const formatTimeRemaining = (seconds: number): string => {
        if (seconds < 60) {
            return `${Math.round(seconds)}s remaining`;
        } else if (seconds < 3600) {
            const mins = Math.floor(seconds / 60);
            const secs = Math.round(seconds % 60);
            return `${mins}m ${secs}s remaining`;
        } else {
            const hours = Math.floor(seconds / 3600);
            const mins = Math.floor((seconds % 3600) / 60);
            return `${hours}h ${mins}m remaining`;
        }
    };

    return (
        <div className="space-y-4 p-4">
            <div className="flex items-center gap-4 mb-4">
                <Button variant="ghost" size="sm" onClick={() => setView('workflow')}>
                    <ArrowLeft className="w-4 h-4 mr-2" />
                    Back to App
                </Button>
                <h1 className="text-2xl font-semibold">Conversion Queue</h1>
            </div>
            <div className="flex justify-between items-center border rounded-lg p-4">
                <div className="flex gap-2">
                    {audiobook && (
                        <>
                            <Button
                                size="sm"
                                variant="outline"
                                onClick={() => setShowSettings(true)}
                                disabled={isLoading}
                            >
                                <Plus className="w-4 h-4 mr-2" />
                                Configure
                            </Button>
                            <Button
                                size="sm"
                                onClick={handleEnqueue}
                                disabled={isLoading || !conversionSettings}
                            >
                                <Plus className="w-4 h-4 mr-2" />
                                Add to Queue
                            </Button>
                        </>
                    )}
                    <Button
                        size="sm"
                        variant="outline"
                        onClick={loadJobs}
                        disabled={isLoading}
                    >
                        <RefreshCw className="w-4 h-4" />
                    </Button>
                </div>
            </div>

            {/* Queue Controls */}
            <div className="flex items-center gap-4 p-4 bg-muted/50 rounded-lg">
                <div className="flex items-center gap-2 flex-1">
                    <Label htmlFor="concurrency">Concurrency:</Label>
                    <Input
                        id="concurrency"
                        type="number"
                        min={1}
                        max={16}
                        value={maxConcurrent}
                        onChange={(e) => setMaxConcurrent(parseInt(e.target.value) || 1)}
                        disabled={isQueueRunning}
                        className="w-20"
                    />
                </div>
                <div className="flex gap-2">
                    {!isQueueRunning ? (
                        <Button
                            onClick={handleStartQueue}
                            disabled={isLoading || pendingJobs.length === 0}
                        >
                            <Play className="w-4 h-4 mr-2" />
                            Start Queue
                        </Button>
                    ) : (
                        <Button
                            onClick={handleStopQueue}
                            disabled={isLoading}
                            variant="secondary"
                        >
                            <Square className="w-4 h-4 mr-2" />
                            Stop Queue
                        </Button>
                    )}
                    <Button
                        variant="outline"
                        onClick={handleClearQueue}
                        disabled={isLoading || isQueueRunning || pendingJobs.length === 0}
                    >
                        Clear Queue
                    </Button>
                    <Button
                        variant="outline"
                        onClick={handleClearCompleted}
                        disabled={isLoading || completedJobs.length === 0}
                    >
                        Clear Completed
                    </Button>
                </div>
            </div>

            {/* Queue Stats */}
            <div className="grid grid-cols-3 gap-4">
                <div className="p-4 border rounded-lg text-center">
                    <div className="text-2xl font-bold">{pendingJobs.length}</div>
                    <p className="text-xs text-muted-foreground">Pending</p>
                </div>
                <div className="p-4 border rounded-lg text-center">
                    <div className="text-2xl font-bold">{activeJobs.length}</div>
                    <p className="text-xs text-muted-foreground">Active</p>
                </div>
                <div className="p-4 border rounded-lg text-center">
                    <div className="text-2xl font-bold">{completedJobs.length}</div>
                    <p className="text-xs text-muted-foreground">Completed</p>
                </div>
            </div>

            {/* Jobs List */}
            <div className="space-y-2 max-h-96 overflow-y-auto">
                {jobs.length === 0 ? (
                    <div className="text-center text-muted-foreground py-8">
                        No jobs in queue
                    </div>
                ) : (
                    jobs.map((job) => (
                        <div key={job.id} className="p-4 border rounded-lg space-y-2">
                            <div className="flex items-center justify-between">
                                <div className="flex items-center gap-2">
                                    {getStatusIcon(job.status)}
                                    <span className="font-semibold">
                                        {job.audiobook.title}
                                    </span>
                                    <span className="text-sm text-muted-foreground">
                                        ({job.status})
                                    </span>
                                </div>
                                <div className="flex gap-2">
                                    {(job.status === JobStatus.Queued || job.status === JobStatus.Running) && (
                                        <Button
                                            size="sm"
                                            variant="ghost"
                                            onClick={() => handleCancelJob(job.id)}
                                        >
                                            <XCircle className="w-4 h-4" />
                                        </Button>
                                    )}
                                    {(job.status === JobStatus.Complete || job.status === JobStatus.Error || job.status === JobStatus.Cancelled) && (
                                        <Button
                                            size="sm"
                                            variant="ghost"
                                            onClick={() => handleRemoveJob(job.id)}
                                        >
                                            <Trash2 className="w-4 h-4" />
                                        </Button>
                                    )}
                                </div>
                            </div>

                            {job.status === JobStatus.Running && (
                                <>
                                    <div className="flex items-center gap-2">
                                        <Progress value={job.progress_percent || 0} className="flex-1" />
                                        <span className="text-sm text-muted-foreground whitespace-nowrap">
                                            {Math.round(job.progress_percent || 0)}%
                                        </span>
                                    </div>
                                    <div className="flex justify-between text-sm text-muted-foreground">
                                        <span>{job.status_message || 'Processing...'}</span>
                                        <span>
                                            {jobTimeRemaining[job.id] !== undefined && jobTimeRemaining[job.id] > 0
                                                ? formatTimeRemaining(jobTimeRemaining[job.id])
                                                : 'Calculating ETA...'}
                                        </span>
                                    </div>
                                </>
                            )}

                            {job.error_message && (
                                <p className="text-sm text-red-500">
                                    Error: {job.error_message}
                                </p>
                            )}

                            {job.output_path && (
                                <p className="text-sm text-muted-foreground">
                                    Output: {job.output_path}
                                </p>
                            )}
                        </div>
                    ))
                )}
            </div>

            {/* Settings Dialog */}
            {showSettings && audiobook && (
                <ConversionSettingsDialog
                    audiobook={audiobook}
                    onConfirm={(settings) => {
                        setConversionSettings(settings);
                        setPendingConversionSettings(settings);
                        setShowSettings(false);
                        toast({
                            title: 'Settings Configured',
                            description: 'Ready to add to queue',
                        });
                    }}
                    onCancel={() => setShowSettings(false)}
                />
            )}
        </div>
    );
};