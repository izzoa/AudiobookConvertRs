import { useState, useMemo } from 'react';
import { useConversionStore } from '@/context/ConversionStore';
import { JobStatus, ConversionStageLabels } from '@/types/models';
import type { ConversionJob, JobStatus as JobStatusType } from '@/types/models';
import { Progress } from '@/components/ui/progress';
import { formatDuration } from '@/lib/format';
import { cn } from '@/lib/utils';
import {
    PlayIcon,
    PauseIcon,
    CheckCircleIcon,
    ExclamationCircleIcon,
    XCircleIcon,
    ClockIcon,
    MagnifyingGlassIcon,
} from '@heroicons/react/24/outline';

type FilterType = 'all' | JobStatusType;

const FILTERS: { label: string; value: FilterType }[] = [
    { label: 'All', value: 'all' },
    { label: 'Running', value: JobStatus.Running },
    { label: 'Queued', value: JobStatus.Queued },
    { label: 'Done', value: JobStatus.Complete },
    { label: 'Failed', value: JobStatus.Error },
    { label: 'Cancelled', value: JobStatus.Cancelled },
];

const STATUS_ORDER: Record<string, number> = {
    [JobStatus.Running]: 0,
    [JobStatus.Queued]: 1,
    [JobStatus.Error]: 2,
    [JobStatus.Complete]: 3,
    [JobStatus.Cancelled]: 4,
};

export function StatusIcon({ status, className = 'w-4 h-4' }: { status: JobStatusType; className?: string }) {
    switch (status) {
        case JobStatus.Running:
            return <PlayIcon className={cn(className, 'text-primary')} />;
        case JobStatus.Queued:
            return <ClockIcon className={cn(className, 'text-muted-foreground')} />;
        case JobStatus.Paused:
            return <PauseIcon className={cn(className, 'text-warning')} />;
        case JobStatus.Complete:
            return <CheckCircleIcon className={cn(className, 'text-success')} />;
        case JobStatus.Error:
            return <ExclamationCircleIcon className={cn(className, 'text-destructive')} />;
        case JobStatus.Cancelled:
            return <XCircleIcon className={cn(className, 'text-muted-foreground')} />;
        default:
            return null;
    }
}

export function jobTitle(job: ConversionJob): string {
    return job.audiobook?.output_metadata?.title || job.audiobook?.title || 'Untitled';
}

function JobRow({ job, isSelected, onClick }: { job: ConversionJob; isSelected: boolean; onClick: () => void }) {
    const isRunning = job.status === JobStatus.Running;
    const progress = job.progress;
    const totalProgress = progress?.total_progress ?? job.progress_percent ?? 0;
    const stageLabel = progress?.stage
        ? (ConversionStageLabels[progress.stage] || progress.stage)
        : (job.status_message || 'Preparing');

    return (
        <button
            onClick={onClick}
            aria-current={isSelected || undefined}
            className={cn(
                'w-full text-left px-3 py-2.5 border-b border-border/60 transition-colors',
                isSelected ? 'bg-primary/[0.08]' : 'hover:bg-accent/50'
            )}
        >
            <div className="flex items-start gap-2.5">
                <StatusIcon status={job.status} className="w-4 h-4 mt-0.5 shrink-0" />
                <div className="flex-1 min-w-0">
                    <div className="font-medium truncate text-sm">{jobTitle(job)}</div>
                    <div className="text-xs text-muted-foreground nums">
                        {job.total_files} {job.total_files === 1 ? 'file' : 'files'} ·{' '}
                        {formatDuration(job.audiobook?.total_duration_ms || 0)}
                    </div>
                    {isRunning && (
                        <div className="mt-2 space-y-1">
                            <div className="flex items-center justify-between text-xs">
                                <span className="text-muted-foreground truncate">{stageLabel}</span>
                                <span className="nums">{Math.round(totalProgress)}%</span>
                            </div>
                            <Progress value={totalProgress} className="h-1.5" />
                        </div>
                    )}
                    {job.status === JobStatus.Error && job.error_message && (
                        <div className="mt-1 text-xs text-destructive truncate" title={job.error_message}>
                            {job.error_message}
                        </div>
                    )}
                </div>
            </div>
        </button>
    );
}

/** Job index for the Queue page. Queue-wide commands live in the page header. */
export function JobList() {
    const { jobs, selectedJobId, selectJob } = useConversionStore();
    const [filter, setFilter] = useState<FilterType>('all');
    const [search, setSearch] = useState('');

    const counts = useMemo(() => {
        const byStatus: Record<string, number> = { all: jobs.length };
        for (const job of jobs) byStatus[job.status] = (byStatus[job.status] ?? 0) + 1;
        return byStatus;
    }, [jobs]);

    const filteredJobs = useMemo(() => {
        const query = search.trim().toLowerCase();
        return jobs
            .filter((job) => filter === 'all' || job.status === filter)
            .filter((job) => !query || jobTitle(job).toLowerCase().includes(query))
            .sort((a, b) => {
                const order = (STATUS_ORDER[a.status] ?? 5) - (STATUS_ORDER[b.status] ?? 5);
                if (order !== 0) return order;
                return new Date(b.created_at).getTime() - new Date(a.created_at).getTime();
            });
    }, [jobs, filter, search]);

    return (
        <div className="h-full flex flex-col">
            <div className="shrink-0 p-2.5 border-b space-y-2 no-select">
                <div className="relative">
                    <MagnifyingGlassIcon className="absolute left-2.5 top-1/2 -translate-y-1/2 w-4 h-4 text-muted-foreground pointer-events-none" />
                    <input
                        type="search"
                        placeholder="Search by title"
                        aria-label="Search jobs by title"
                        value={search}
                        onChange={(e) => setSearch(e.target.value)}
                        className="w-full h-8 rounded-md border border-input bg-background pl-8 pr-2.5 text-sm placeholder:text-muted-foreground/70 focus:outline-none focus-visible:ring-2 focus-visible:ring-ring"
                    />
                </div>

                <div className="flex flex-wrap gap-1" role="radiogroup" aria-label="Filter jobs by status">
                    {FILTERS.filter((f) => f.value === 'all' || counts[f.value]).map((f) => {
                        const active = filter === f.value;
                        return (
                            <button
                                key={f.value}
                                role="radio"
                                aria-checked={active}
                                onClick={() => setFilter(f.value)}
                                className={cn(
                                    'h-6 px-2 rounded-md text-xs font-medium transition-colors flex items-center gap-1',
                                    active
                                        ? 'bg-accent text-foreground'
                                        : 'text-muted-foreground hover:bg-accent/60 hover:text-foreground'
                                )}
                            >
                                {f.label}
                                <span className="nums text-muted-foreground">{counts[f.value] ?? 0}</span>
                            </button>
                        );
                    })}
                </div>
            </div>

            <div className="flex-1 overflow-y-auto">
                {filteredJobs.length === 0 ? (
                    <div className="h-full flex flex-col items-center justify-center gap-1 p-6 text-center">
                        <p className="text-sm font-medium">No matching jobs</p>
                        <button
                            onClick={() => {
                                setFilter('all');
                                setSearch('');
                            }}
                            className="text-xs text-muted-foreground underline underline-offset-4 hover:text-foreground"
                        >
                            Show all jobs
                        </button>
                    </div>
                ) : (
                    filteredJobs.map((job) => (
                        <JobRow
                            key={job.id}
                            job={job}
                            isSelected={job.id === selectedJobId}
                            onClick={() => selectJob(job.id)}
                        />
                    ))
                )}
            </div>
        </div>
    );
}
