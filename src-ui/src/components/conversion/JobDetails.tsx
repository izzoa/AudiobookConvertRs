import { useConversionStore } from '@/context/ConversionStore';
import { JobStatus, ConversionStage, ConversionStageLabels } from '@/types/models';
import type { ConversionJob } from '@/types/models';
import { Progress } from '@/components/ui/progress';
import { Button } from '@/components/ui/button';
import { revealInFileManager } from '@/api/files';
import {
    PlayIcon,
    StopIcon,
    ArrowPathIcon,
    FolderOpenIcon,
    DocumentDuplicateIcon,
    ClockIcon,
    CheckCircleIcon,
    ExclamationCircleIcon,
    XCircleIcon,
} from '@heroicons/react/24/outline';

const STAGES: { key: typeof ConversionStage[keyof typeof ConversionStage]; label: string }[] = [
    { key: ConversionStage.Preparing, label: 'Preparing' },
    { key: ConversionStage.Encoding, label: 'Encoding' },
    { key: ConversionStage.Metadata, label: 'Metadata' },
    { key: ConversionStage.Chapters, label: 'Chapters' },
    { key: ConversionStage.Verifying, label: 'Verifying' },
    { key: ConversionStage.Finalizing, label: 'Finalizing' },
];

function formatTimeRemaining(seconds?: number): string {
    if (!seconds || seconds <= 0) return '';
    const mins = Math.floor(seconds / 60);
    const secs = seconds % 60;
    if (mins > 0) {
        return `~${mins}m ${secs}s remaining`;
    }
    return `~${secs}s remaining`;
}

function formatSpeed(speed?: number): string {
    if (!speed || speed <= 0) return '';
    return `${speed.toFixed(1)}x`;
}

function formatDuration(ms: number): string {
    const secs = Math.floor(ms / 1000);
    const mins = Math.floor(secs / 60);
    const hours = Math.floor(mins / 60);
    if (hours > 0) {
        return `${hours}h ${mins % 60}m ${secs % 60}s`;
    }
    if (mins > 0) {
        return `${mins}m ${secs % 60}s`;
    }
    return `${secs}s`;
}

function formatBytes(bytes: number): string {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
    return `${(bytes / (1024 * 1024 * 1024)).toFixed(2)} GB`;
}

function StageProgress({ currentStage }: { currentStage: string }) {
    const currentIndex = STAGES.findIndex((s) => s.key === currentStage);

    return (
        <div className="flex items-center gap-1 my-4">
            {STAGES.map((stage, idx) => {
                const isComplete = idx < currentIndex;
                const isCurrent = idx === currentIndex;
                return (
                    <div key={stage.key} className="flex items-center">
                        <div
                            className={`w-6 h-6 rounded-full flex items-center justify-center text-xs font-medium ${
                                isComplete
                                    ? 'bg-green-500 text-white'
                                    : isCurrent
                                    ? 'bg-primary text-primary-foreground'
                                    : 'bg-muted text-muted-foreground'
                            }`}
                        >
                            {isComplete ? '✓' : idx + 1}
                        </div>
                        {idx < STAGES.length - 1 && (
                            <div
                                className={`w-4 h-0.5 ${
                                    isComplete ? 'bg-green-500' : 'bg-muted'
                                }`}
                            />
                        )}
                    </div>
                );
            })}
        </div>
    );
}

export function JobDetails({ job }: { job: ConversionJob | null }) {
    const { cancelJob, retryJob, removeJob } = useConversionStore();

    if (!job) {
        return (
            <div className="flex flex-col items-center justify-center h-full text-muted-foreground">
                <ClockIcon className="w-16 h-16 mb-4 opacity-50" />
                <div className="text-lg font-medium">No job selected</div>
                <div className="text-sm mt-1">Select a job from the list to view details</div>
            </div>
        );
    }

    const title = job.audiobook?.output_metadata?.title || job.audiobook?.title || 'Untitled';
    const author = job.audiobook?.output_metadata?.artist || job.audiobook?.author || '';
    const progress = job.progress;
    const isRunning = job.status === JobStatus.Running;
    const isQueued = job.status === JobStatus.Queued;
    const isComplete = job.status === JobStatus.Complete;
    const isError = job.status === JobStatus.Error;
    const isCancelled = job.status === JobStatus.Cancelled;
    const isTerminal = isComplete || isError || isCancelled;

    const handleRevealOutput = async () => {
        if (job.output_path || job.settings.output_path) {
            try {
                await revealInFileManager(job.output_path || job.settings.output_path);
            } catch (err) {
                console.error('Failed to reveal file:', err);
            }
        }
    };

    const handleCopyPath = () => {
        const path = job.output_path || job.settings.output_path;
        if (path) {
            navigator.clipboard.writeText(path);
        }
    };

    const handleCopyError = () => {
        if (job.error_message) {
            navigator.clipboard.writeText(job.error_message);
        }
    };

    const handleCancel = async () => {
        await cancelJob(job.id);
    };

    const handleRetry = async () => {
        await retryJob(job.id);
    };

    const handleRemove = async () => {
        await removeJob(job.id);
    };

    return (
        <div className="h-full overflow-y-auto p-6">
            <div className="max-w-2xl mx-auto">
                <div className="flex items-start justify-between mb-6">
                    <div>
                        <h1 className="text-2xl font-semibold">{title}</h1>
                        {author && <div className="text-muted-foreground mt-1">{author}</div>}
                    </div>
                    <div className="flex items-center gap-2">
                        {isRunning || isQueued ? (
                            <>
                                <Button variant="outline" size="sm" onClick={handleCancel}>
                                    <StopIcon className="w-4 h-4 mr-1" />
                                    Cancel
                                </Button>
                            </>
                        ) : isComplete ? (
                            <>
                                <Button variant="outline" size="sm" onClick={handleRevealOutput}>
                                    <FolderOpenIcon className="w-4 h-4 mr-1" />
                                    Reveal
                                </Button>
                                <Button variant="outline" size="sm" onClick={handleCopyPath}>
                                    <DocumentDuplicateIcon className="w-4 h-4 mr-1" />
                                    Copy path
                                </Button>
                            </>
                        ) : isError ? (
                            <>
                                <Button variant="default" size="sm" onClick={handleRetry}>
                                    <ArrowPathIcon className="w-4 h-4 mr-1" />
                                    Retry
                                </Button>
                                <Button variant="outline" size="sm" onClick={handleCopyError}>
                                    <DocumentDuplicateIcon className="w-4 h-4 mr-1" />
                                    Copy error
                                </Button>
                            </>
                        ) : isCancelled ? (
                            <Button variant="default" size="sm" onClick={handleRetry}>
                                <ArrowPathIcon className="w-4 h-4 mr-1" />
                                Retry
                            </Button>
                        ) : null}

                        {isTerminal && (
                            <Button variant="ghost" size="sm" onClick={handleRemove}>
                                Remove
                            </Button>
                        )}
                    </div>
                </div>

                <div className="mb-6">
                    {isRunning && progress ? (
                        <div className="space-y-4">
                            <div className="flex items-center justify-between">
                                <div className="flex items-center gap-2">
                                    <PlayIcon className="w-5 h-5 text-blue-500" />
                                    <span className="font-medium">
                                        {ConversionStageLabels[progress.stage] || progress.stage}
                                    </span>
                                </div>
                                <div className="text-sm text-muted-foreground">
                                    {formatTimeRemaining(progress.time_remaining_sec)}
                                    {progress.speed && ` • ${formatSpeed(progress.speed)}`}
                                </div>
                            </div>

                            <StageProgress currentStage={progress.stage} />

                            <div className="space-y-2">
                                <div className="flex items-center justify-between text-sm">
                                    <span>Overall progress</span>
                                    <span className="font-medium">{Math.round(progress.total_progress)}%</span>
                                </div>
                                <Progress value={progress.total_progress} className="h-2" />
                            </div>

                            {progress.current_file && (
                                <div className="text-sm text-muted-foreground">
                                    File {progress.file_index + 1} of {job.total_files}:{' '}
                                    <span className="font-mono">{progress.current_file}</span>
                                </div>
                            )}
                        </div>
                    ) : isQueued ? (
                        <div className="flex items-center gap-2 text-muted-foreground">
                            <ClockIcon className="w-5 h-5" />
                            <span>Waiting in queue...</span>
                        </div>
                    ) : isComplete ? (
                        <div className="flex items-center gap-2 text-green-600">
                            <CheckCircleIcon className="w-5 h-5" />
                            <span>Conversion complete</span>
                        </div>
                    ) : isError ? (
                        <div className="space-y-2">
                            <div className="flex items-center gap-2 text-destructive">
                                <ExclamationCircleIcon className="w-5 h-5" />
                                <span>Conversion failed</span>
                            </div>
                            {job.error_message && (
                                <div className="p-3 bg-destructive/10 rounded-md text-sm text-destructive font-mono">
                                    {job.error_message}
                                </div>
                            )}
                        </div>
                    ) : isCancelled ? (
                        <div className="flex items-center gap-2 text-muted-foreground">
                            <XCircleIcon className="w-5 h-5" />
                            <span>Conversion cancelled</span>
                        </div>
                    ) : null}
                </div>

                <div className="border rounded-lg divide-y">
                    <div className="p-4">
                        <h3 className="font-medium mb-3">Input</h3>
                        <div className="grid grid-cols-2 gap-4 text-sm">
                            <div>
                                <div className="text-muted-foreground">Files</div>
                                <div>{job.total_files}</div>
                            </div>
                            <div>
                                <div className="text-muted-foreground">Duration</div>
                                <div>{formatDuration(job.audiobook?.total_duration_ms || 0)}</div>
                            </div>
                            <div>
                                <div className="text-muted-foreground">Size</div>
                                <div>{formatBytes(job.audiobook?.total_size_bytes || 0)}</div>
                            </div>
                            <div>
                                <div className="text-muted-foreground">Chapters</div>
                                <div>{job.audiobook?.chapters?.length || 0}</div>
                            </div>
                        </div>
                    </div>

                    <div className="p-4">
                        <h3 className="font-medium mb-3">Settings</h3>
                        <div className="grid grid-cols-2 gap-4 text-sm">
                            <div>
                                <div className="text-muted-foreground">Bitrate</div>
                                <div>{job.settings.bitrate_kbps} kbps</div>
                            </div>
                            <div>
                                <div className="text-muted-foreground">Sample Rate</div>
                                <div>{job.settings.sample_rate_hz ? `${job.settings.sample_rate_hz} Hz` : 'Auto'}</div>
                            </div>
                            <div>
                                <div className="text-muted-foreground">Mode</div>
                                <div className="uppercase">{job.settings.encoding_mode}</div>
                            </div>
                            <div>
                                <div className="text-muted-foreground">Format</div>
                                <div className="uppercase">{job.settings.output_format}</div>
                            </div>
                        </div>
                    </div>

                    <div className="p-4">
                        <h3 className="font-medium mb-3">Output</h3>
                        <div className="text-sm font-mono bg-muted p-2 rounded truncate">
                            {job.output_path || job.settings.output_path}
                        </div>
                    </div>

                    <div className="p-4">
                        <h3 className="font-medium mb-3">Timeline</h3>
                        <div className="text-sm space-y-1">
                            <div className="flex justify-between">
                                <span className="text-muted-foreground">Created</span>
                                <span>{new Date(job.created_at).toLocaleString()}</span>
                            </div>
                            {job.started_at && (
                                <div className="flex justify-between">
                                    <span className="text-muted-foreground">Started</span>
                                    <span>{new Date(job.started_at).toLocaleString()}</span>
                                </div>
                            )}
                            {job.completed_at && (
                                <div className="flex justify-between">
                                    <span className="text-muted-foreground">Completed</span>
                                    <span>{new Date(job.completed_at).toLocaleString()}</span>
                                </div>
                            )}
                        </div>
                    </div>
                </div>
            </div>
        </div>
    );
}
