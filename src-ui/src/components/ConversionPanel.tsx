import React, { useState, useEffect } from "react";
import { Button } from "./ui/button";
import { Input } from "./ui/input";
import { Label } from "./ui/label";
import { Progress } from "./ui/progress";
import { useApp } from "@/context/AppContext";
import * as ConversionApi from "@/api/conversion";
import * as FilesApi from "@/api/files";
import type { ConversionSettings, JobStatus } from "@/types/models";
import { listen } from '@tauri-apps/api/event';
import { toast } from "@/components/ui/use-toast";

interface ProgressEvent {
    job_id: string;
    current_file: string;
    file_progress: number;
    total_progress: number;
    time_remaining_sec: number;
}

interface CompletionEvent {
    job_id: string;
    output_path: string;
    duration_ms: number;
}

interface ErrorEvent {
    job_id: string;
    error_message: string;
    recoverable: boolean;
}

export function ConversionPanel() {
    const { activeAudiobook } = useApp();
    const [isStarting, setIsStarting] = useState(false);
    const [status, setStatus] = useState<string>("");
    const [outputPath, setOutputPath] = useState("output.m4b");
    const [currentJobId, setCurrentJobId] = useState<string | null>(null);
    const [jobStatus, setJobStatus] = useState<JobStatus | null>(null);
    const [progress, setProgress] = useState<ProgressEvent | null>(null);

    // Track the job ID in a ref so event listeners always have the latest value
    const currentJobIdRef = React.useRef<string | null>(null);
    
    // Keep the ref in sync with state
    useEffect(() => {
        currentJobIdRef.current = currentJobId;
    }, [currentJobId]);

    // Listen to conversion events - set up ONCE on mount (not dependent on currentJobId)
    useEffect(() => {
        console.log('[ConversionPanel] Setting up event listeners');
        
        const unlistenProgress = listen<ProgressEvent>('conversion_progress', (event) => {
            const progressData = event.payload;
            console.log('[ConversionPanel] Received progress event:', progressData.job_id, 'current:', currentJobIdRef.current);
            if (progressData.job_id === currentJobIdRef.current) {
                setProgress(progressData);
                setStatus(`Converting: ${progressData.current_file}`);
            }
        });

        const unlistenComplete = listen<CompletionEvent>('conversion_complete', (event) => {
            const completionData = event.payload;
            console.log('[ConversionPanel] Received completion event:', completionData.job_id, 'current:', currentJobIdRef.current);
            if (completionData.job_id === currentJobIdRef.current) {
                setJobStatus('complete');
                setProgress(null);
                const durationSec = (completionData.duration_ms / 1000).toFixed(1);
                setStatus(`Completed: ${completionData.output_path} (${durationSec}s)`);
            }
        });

        const unlistenError = listen<ErrorEvent>('conversion_error', (event) => {
            const errorData = event.payload;
            console.log('[ConversionPanel] Received error event:', errorData.job_id, 'current:', currentJobIdRef.current, 'error:', errorData.error_message);
            if (errorData.job_id === currentJobIdRef.current) {
                setJobStatus('error');
                setProgress(null);
                setStatus(`Error: ${errorData.error_message}`);
            }
        });

        return () => {
            console.log('[ConversionPanel] Cleaning up event listeners');
            unlistenProgress.then(fn => fn());
            unlistenComplete.then(fn => fn());
            unlistenError.then(fn => fn());
        };
    }, []); // Empty dependency array - set up once on mount

    const handleStart = async () => {
        if (!activeAudiobook) return;
        setIsStarting(true);
        setStatus("Starting...");
        setProgress(null);

        try {
            const settings: ConversionSettings = {
                bitrate_kbps: 64,
                output_format: 'm4b',
                output_path: outputPath,
                concurrent_jobs: 1,
                overwrite_existing: false,
                embed_chapters: true,
                embed_artwork: true,
                copy_metadata: true,
            };

            const jobId = await ConversionApi.startConversion(activeAudiobook.id, settings);
            // Set ref immediately so event listeners can match the job ID
            currentJobIdRef.current = jobId;
            setCurrentJobId(jobId);
            setJobStatus('running');
            setStatus(`Started Job: ${jobId}`);
            console.log('[ConversionPanel] Started job:', jobId);
        } catch (e) {
            console.error(e);
            setStatus(`Failed to start: ${e}`);
        } finally {
            setIsStarting(false);
        }
    };

    // Fallback polling in case events don't reach the UI (e.g., running in browser)
    useEffect(() => {
        if (!currentJobId || jobStatus !== 'running') return;

        let isCancelled = false;
        const interval = setInterval(async () => {
            if (isCancelled) return;
            try {
                const status = await ConversionApi.getConversionStatus(currentJobId);
                // Update progress if available
                if (status.progress && typeof status.progress.total_progress === 'number') {
                    setProgress({
                        job_id: currentJobId,
                        current_file: status.progress.current_file ?? '',
                        file_progress: status.progress.file_progress ?? 0,
                        total_progress: status.progress.total_progress ?? 0,
                        time_remaining_sec: status.progress.time_remaining_sec ?? 0,
                    });
                    setStatus(`Converting: ${status.progress.current_file ?? ''}`);
                }
                setJobStatus(status.status);

                // Handle terminal states
                if (status.status === 'complete') {
                    setProgress(null);
                    setStatus(`Completed: ${status.output_path ?? ''}`);
                } else if (status.status === 'error') {
                    setProgress(null);
                    setStatus(`Error: ${status.status_message ?? 'Unknown error'}`);
                }
            } catch (e) {
                console.error(e);
                toast({
                    title: "Progress update failed",
                    description: `${e}`,
                    variant: "destructive",
                });
            }
        }, 1000);

        return () => {
            isCancelled = true;
            clearInterval(interval);
        };
    }, [currentJobId, jobStatus]);

    const handleCancel = async () => {
        if (!currentJobId) return;
        try {
            await ConversionApi.cancelConversion(currentJobId);
            setJobStatus('cancelled');
            setCurrentJobId(null);
            setProgress(null); // Clear progress UI
            setStatus("Cancelled");
        } catch (e) {
            console.error(e);
            setStatus(`Failed to cancel: ${e}`);
        }
    };

    const handleBrowseOutput = async () => {
        try {
            // Generate a default filename from the audiobook title if available
            let defaultName = 'output.m4b';
            if (activeAudiobook?.title) {
                // Sanitize title for filename
                const sanitized = activeAudiobook.title.replace(/[<>:"/\\|?*]/g, '_');
                defaultName = `${sanitized}.m4b`;
            }
            
            const selectedPath = await FilesApi.pickOutputFile(defaultName);
            if (selectedPath) {
                setOutputPath(selectedPath);
            }
        } catch (e) {
            console.error(e);
            setStatus(`Failed to open file dialog: ${e}`);
        }
    };

    if (!activeAudiobook) return null;

    const formatTime = (seconds: number) => {
        if (seconds < 60) return `${seconds}s`;
        const mins = Math.floor(seconds / 60);
        const secs = seconds % 60;
        return `${mins}m ${secs}s`;
    };

    return (
        <div className="p-4 border rounded-lg bg-card mt-4 space-y-4">
            <div className="flex justify-between items-center">
                <h3 className="font-semibold">Conversion</h3>
                <span className="text-sm text-muted-foreground">{status}</span>
            </div>

            <div className="flex gap-4 items-end">
                <div className="grid gap-2 flex-1">
                    <Label htmlFor="output-path">Output Path</Label>
                    <div className="flex gap-2">
                        <Input
                            id="output-path"
                            value={outputPath}
                            onChange={(e) => setOutputPath(e.target.value)}
                            className="flex-1"
                        />
                        <Button 
                            type="button" 
                            variant="outline" 
                            onClick={handleBrowseOutput}
                            disabled={isStarting || jobStatus === 'running'}
                        >
                            Browse
                        </Button>
                    </div>
                </div>
                <Button onClick={handleStart} disabled={isStarting || jobStatus === 'running'}>
                    {isStarting ? "Starting..." : "Start Conversion"}
                </Button>
            </div>

            {progress && (
                <div className="space-y-2">
                    <div className="flex justify-between text-sm">
                        <span className="text-muted-foreground">Progress: {progress.total_progress.toFixed(1)}%</span>
                        <span className="text-muted-foreground">ETA: {formatTime(progress.time_remaining_sec)}</span>
                    </div>
                    <Progress value={progress.total_progress} className="h-2" />
                    <div className="text-xs text-muted-foreground">
                        Current file: {progress.current_file} ({progress.file_progress.toFixed(1)}%)
                    </div>
                </div>
            )}

            {currentJobId && jobStatus === 'running' && (
                <div className="flex gap-2">
                    <Button variant="destructive" onClick={handleCancel}>
                        Cancel
                    </Button>
                </div>
            )}
        </div>
    );
}
