import { useConversionStore } from '@/context/ConversionStore';
import { JobStatus, ConversionStage, ConversionStageLabels } from '@/types/models';
import type { ConversionJob, ConversionStage as ConversionStageType } from '@/types/models';
import { Progress } from '@/components/ui/progress';
import { Button } from '@/components/ui/button';
import { useToast } from '@/components/ui/use-toast';
import { revealInFileManager } from '@/api/files';
import { formatBytes, formatDuration, formatEta } from '@/lib/format';
import { cn } from '@/lib/utils';
import { StatusIcon, jobTitle } from './JobList';
import {
    StopIcon,
    ArrowPathIcon,
    FolderOpenIcon,
    DocumentDuplicateIcon,
    CheckIcon,
    QueueListIcon,
} from '@heroicons/react/24/outline';

const STATUS_TEXT: Record<string, string> = {
    [JobStatus.Queued]: 'Waiting in the queue',
    [JobStatus.Paused]: 'Paused',
    [JobStatus.Complete]: 'Converted',
    [JobStatus.Error]: 'Conversion failed',
    [JobStatus.Cancelled]: 'Cancelled',
};

/** The pipeline's stages in order; normalizing only runs when it was asked for. */
function stagesFor(job: ConversionJob): ConversionStageType[] {
    return [
        ConversionStage.Preparing,
        ...(job.settings.normalize_audio ? [ConversionStage.Normalizing] : []),
        ConversionStage.Encoding,
        ConversionStage.Metadata,
        ConversionStage.Chapters,
        ConversionStage.Verifying,
        ConversionStage.Finalizing,
    ];
}

function StageSteps({ job, current }: { job: ConversionJob; current: ConversionStageType }) {
    const stages = stagesFor(job);
    const currentIndex = stages.indexOf(current);

    return (
        <ol className="flex flex-wrap gap-x-4 gap-y-1.5 text-xs" aria-label="Conversion stages">
            {stages.map((stage, idx) => {
                const done = idx < currentIndex;
                const active = idx === currentIndex;
                return (
                    <li
                        key={stage}
                        aria-current={active ? 'step' : undefined}
                        className={cn(
                            'flex items-center gap-1.5',
                            done ? 'text-muted-foreground' : active ? 'text-foreground font-medium' : 'text-muted-foreground/60'
                        )}
                    >
                        {done ? (
                            <CheckIcon className="w-3.5 h-3.5 text-success" />
                        ) : (
                            <span
                                className={cn(
                                    'w-1.5 h-1.5 mx-1 rounded-full',
                                    active ? 'bg-primary' : 'bg-muted-foreground/30'
                                )}
                            />
                        )}
                        {ConversionStageLabels[stage]}
                    </li>
                );
            })}
        </ol>
    );
}

function Detail({ label, children }: { label: string; children: React.ReactNode }) {
    return (
        <div className="min-w-0">
            <dt className="text-xs text-muted-foreground">{label}</dt>
            <dd className="mt-0.5 text-sm nums truncate">{children}</dd>
        </div>
    );
}

export function JobDetails({ job }: { job: ConversionJob | null }) {
    const { cancelJob, retryJob, removeJob } = useConversionStore();
    const { toast } = useToast();

    if (!job) {
        return (
            <div className="h-full flex flex-col items-center justify-center gap-2 p-8 text-center text-muted-foreground">
                <QueueListIcon className="w-8 h-8" />
                <p className="text-sm">Select a job to see its progress and output.</p>
            </div>
        );
    }

    const author = job.audiobook?.output_metadata?.artist || job.audiobook?.author || '';
    const progress = job.progress;
    const currentStage = progress?.stage ?? ConversionStage.Preparing;
    const totalProgress = progress?.total_progress ?? job.progress_percent ?? 0;
    const outputPath = job.output_path || job.settings.output_path;
    const isRunning = job.status === JobStatus.Running;
    const isQueued = job.status === JobStatus.Queued;
    const isComplete = job.status === JobStatus.Complete;
    const isError = job.status === JobStatus.Error;
    const isCancelled = job.status === JobStatus.Cancelled;
    const isTerminal = isComplete || isError || isCancelled;

    const copy = async (text: string, what: string) => {
        try {
            await navigator.clipboard.writeText(text);
            toast({ title: `${what} copied` });
        } catch (err) {
            toast({ title: `Could not copy ${what.toLowerCase()}`, description: String(err), variant: 'destructive' });
        }
    };

    const reveal = async () => {
        try {
            await revealInFileManager(outputPath);
        } catch (err) {
            toast({ title: 'Could not show the file', description: String(err), variant: 'destructive' });
        }
    };

    return (
        <div className="h-full overflow-y-auto">
            <div className="max-w-2xl mx-auto px-6 py-6 space-y-6">
                <header className="flex items-start justify-between gap-4">
                    <div className="min-w-0">
                        <h1 className="text-xl font-semibold tracking-tight truncate">{jobTitle(job)}</h1>
                        {author && <p className="text-sm text-muted-foreground mt-0.5 truncate">{author}</p>}
                    </div>
                    <div className="flex items-center gap-1.5 shrink-0">
                        {(isRunning || isQueued) && (
                            <Button variant="outline" size="sm" onClick={() => cancelJob(job.id)}>
                                <StopIcon className="w-4 h-4 mr-1.5" />
                                Cancel
                            </Button>
                        )}
                        {isComplete && (
                            <Button variant="outline" size="sm" onClick={reveal}>
                                <FolderOpenIcon className="w-4 h-4 mr-1.5" />
                                Show in folder
                            </Button>
                        )}
                        {(isError || isCancelled) && (
                            <Button size="sm" onClick={() => retryJob(job.id)}>
                                <ArrowPathIcon className="w-4 h-4 mr-1.5" />
                                Retry
                            </Button>
                        )}
                        {isTerminal && (
                            <Button variant="ghost" size="sm" onClick={() => removeJob(job.id)}>
                                Remove
                            </Button>
                        )}
                    </div>
                </header>

                {isRunning ? (
                    <section className="space-y-3">
                        <div className="flex items-baseline justify-between gap-4">
                            <span className="flex items-center gap-2 text-sm font-medium">
                                <StatusIcon status={job.status} />
                                {ConversionStageLabels[currentStage] || currentStage}
                            </span>
                            <span className="text-sm text-muted-foreground nums">
                                <span className="text-foreground font-medium">{Math.round(totalProgress)}%</span>
                                {!!progress?.time_remaining_sec && ` · ${formatEta(progress.time_remaining_sec)} left`}
                                {!!progress?.speed && ` · ${progress.speed.toFixed(1)}×`}
                            </span>
                        </div>
                        <Progress value={totalProgress} className="h-2" />
                        <StageSteps job={job} current={currentStage} />
                        {progress?.current_file && (
                            <p className="text-xs text-muted-foreground truncate" title={progress.current_file}>
                                File {progress.file_index + 1} of {job.total_files}: {progress.current_file}
                            </p>
                        )}
                    </section>
                ) : (
                    <section className="space-y-2">
                        <p
                            className={cn(
                                'flex items-center gap-2 text-sm font-medium',
                                isComplete && 'text-success',
                                isError && 'text-destructive'
                            )}
                        >
                            <StatusIcon status={job.status} />
                            {STATUS_TEXT[job.status] ?? job.status}
                        </p>
                        {isError && job.error_message && (
                            <div className="rounded-md border border-destructive/30 bg-destructive/[0.06] p-3">
                                <p className="text-sm text-foreground break-words font-mono">{job.error_message}</p>
                                <button
                                    onClick={() => copy(job.error_message!, 'Error')}
                                    className="mt-2 inline-flex items-center gap-1 text-xs text-muted-foreground hover:text-foreground"
                                >
                                    <DocumentDuplicateIcon className="w-3.5 h-3.5" />
                                    Copy error
                                </button>
                            </div>
                        )}
                    </section>
                )}

                <section className="border-t pt-5">
                    <h2 className="text-sm font-semibold mb-3">Output</h2>
                    <div className="flex items-center gap-2 rounded-md bg-muted px-3 py-2">
                        <span className="flex-1 min-w-0 truncate text-sm font-mono" title={outputPath}>
                            {outputPath}
                        </span>
                        <button
                            onClick={() => copy(outputPath, 'Path')}
                            aria-label="Copy output path"
                            title="Copy output path"
                            className="shrink-0 w-7 h-7 rounded-md flex items-center justify-center text-muted-foreground hover:bg-accent hover:text-foreground"
                        >
                            <DocumentDuplicateIcon className="w-4 h-4" />
                        </button>
                    </div>
                    <dl className="mt-4 grid grid-cols-3 gap-x-6 gap-y-3">
                        <Detail label="Bitrate">
                            {job.settings.bitrate_kbps} kbps {job.settings.encoding_mode.toUpperCase()}
                        </Detail>
                        <Detail label="Sample rate">
                            {job.settings.sample_rate_hz ? `${job.settings.sample_rate_hz} Hz` : 'Match source'}
                        </Detail>
                        <Detail label="Loudness">{job.settings.normalize_audio ? 'Normalized' : 'As recorded'}</Detail>
                    </dl>
                </section>

                <section className="border-t pt-5">
                    <h2 className="text-sm font-semibold mb-3">Source</h2>
                    <dl className="grid grid-cols-3 gap-x-6 gap-y-3">
                        <Detail label="Files">{job.total_files}</Detail>
                        <Detail label="Duration">{formatDuration(job.audiobook?.total_duration_ms || 0)}</Detail>
                        <Detail label="Size">{formatBytes(job.audiobook?.total_size_bytes || 0)}</Detail>
                        <Detail label="Chapters">{job.audiobook?.chapters?.length || 0}</Detail>
                    </dl>
                </section>

                <section className="border-t pt-5">
                    <h2 className="text-sm font-semibold mb-3">Timeline</h2>
                    <dl className="grid grid-cols-3 gap-x-6 gap-y-3">
                        <Detail label="Added">{new Date(job.created_at).toLocaleString()}</Detail>
                        {job.started_at && <Detail label="Started">{new Date(job.started_at).toLocaleString()}</Detail>}
                        {job.completed_at && (
                            <Detail label="Finished">{new Date(job.completed_at).toLocaleString()}</Detail>
                        )}
                    </dl>
                </section>
            </div>
        </div>
    );
}
