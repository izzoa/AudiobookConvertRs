import { useState, useMemo } from 'react';
import { useConversionStore } from '@/context/ConversionStore';
import { JobStatus, ConversionStageLabels } from '@/types/models';
import type { ConversionJob, JobStatus as JobStatusType } from '@/types/models';
import { Progress } from '@/components/ui/progress';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import {
    PlayIcon,
    PauseIcon,
    CheckCircleIcon,
    ExclamationCircleIcon,
    XCircleIcon,
    ClockIcon,
    TrashIcon,
    MagnifyingGlassIcon,
    PlusIcon,
} from '@heroicons/react/24/outline';

interface JobListProps {
    onNewJob?: () => void;
}

type FilterType = 'all' | JobStatusType;

const FILTERS: { label: string; value: FilterType }[] = [
    { label: 'All', value: 'all' },
    { label: 'Running', value: JobStatus.Running },
    { label: 'Queued', value: JobStatus.Queued },
    { label: 'Complete', value: JobStatus.Complete },
    { label: 'Failed', value: JobStatus.Error },
    { label: 'Cancelled', value: JobStatus.Cancelled },
];

function getStatusIcon(status: JobStatusType) {
    switch (status) {
        case JobStatus.Running:
            return <PlayIcon className="w-4 h-4 text-blue-500" />;
        case JobStatus.Queued:
            return <ClockIcon className="w-4 h-4 text-muted-foreground" />;
        case JobStatus.Paused:
            return <PauseIcon className="w-4 h-4 text-yellow-500" />;
        case JobStatus.Complete:
            return <CheckCircleIcon className="w-4 h-4 text-green-500" />;
        case JobStatus.Error:
            return <ExclamationCircleIcon className="w-4 h-4 text-destructive" />;
        case JobStatus.Cancelled:
            return <XCircleIcon className="w-4 h-4 text-muted-foreground" />;
        default:
            return null;
    }
}

function formatDuration(ms: number): string {
    const secs = Math.floor(ms / 1000);
    const mins = Math.floor(secs / 60);
    const hours = Math.floor(mins / 60);
    if (hours > 0) {
        return `${hours}h ${mins % 60}m`;
    }
    return `${mins}m ${secs % 60}s`;
}

function JobRow({ job, isSelected, onClick }: { job: ConversionJob; isSelected: boolean; onClick: () => void }) {
    const title = job.audiobook?.output_metadata?.title || job.audiobook?.title || 'Untitled';
    const isRunning = job.status === JobStatus.Running;
    const progress = job.progress;

    return (
        <button
            onClick={onClick}
            className={`w-full text-left p-3 border-b transition-colors ${
                isSelected ? 'bg-primary/10 border-l-2 border-l-primary' : 'hover:bg-muted/50'
            }`}
        >
            <div className="flex items-start gap-2">
                <div className="mt-0.5">{getStatusIcon(job.status)}</div>
                <div className="flex-1 min-w-0">
                    <div className="font-medium truncate text-sm">{title}</div>
                    <div className="text-xs text-muted-foreground">
                        {job.total_files} file{job.total_files !== 1 ? 's' : ''} •{' '}
                        {formatDuration(job.audiobook?.total_duration_ms || 0)}
                    </div>
                    {isRunning && progress && (
                        <div className="mt-2 space-y-1">
                            <div className="flex items-center justify-between text-xs">
                                <span className="text-muted-foreground">
                                    {ConversionStageLabels[progress.stage] || progress.stage}
                                </span>
                                <span>{Math.round(progress.total_progress)}%</span>
                            </div>
                            <Progress value={progress.total_progress} className="h-1.5" />
                        </div>
                    )}
                    {job.status === JobStatus.Error && job.error_message && (
                        <div className="mt-1 text-xs text-destructive truncate">{job.error_message}</div>
                    )}
                </div>
            </div>
        </button>
    );
}

export function JobList({ onNewJob }: JobListProps) {
    const { jobs, selectedJobId, selectJob, clearCompleted, queueState } = useConversionStore();
    const [filter, setFilter] = useState<FilterType>('all');
    const [search, setSearch] = useState('');

    const filteredJobs = useMemo(() => {
        let result = jobs;

        if (filter !== 'all') {
            result = result.filter((job) => job.status === filter);
        }

        if (search.trim()) {
            const query = search.toLowerCase();
            result = result.filter((job) => {
                const title = job.audiobook?.output_metadata?.title || job.audiobook?.title || '';
                return title.toLowerCase().includes(query);
            });
        }

        return result.sort((a, b) => {
            const statusOrder: Record<string, number> = {
                [JobStatus.Running]: 0,
                [JobStatus.Queued]: 1,
                [JobStatus.Error]: 2,
                [JobStatus.Complete]: 3,
                [JobStatus.Cancelled]: 4,
            };
            const orderA = statusOrder[a.status] ?? 5;
            const orderB = statusOrder[b.status] ?? 5;
            if (orderA !== orderB) return orderA - orderB;
            return new Date(b.created_at).getTime() - new Date(a.created_at).getTime();
        });
    }, [jobs, filter, search]);

    const hasCompletedJobs = jobs.some((j) => j.status === JobStatus.Complete);

    return (
        <div className="h-full flex flex-col">
            <div className="p-3 border-b space-y-3">
                <div className="flex items-center justify-between">
                    <h2 className="font-semibold">Jobs</h2>
                    <div className="flex items-center gap-2">
                        {queueState && (
                            <span className="text-xs text-muted-foreground">
                                {queueState.active_jobs} running • {queueState.pending_jobs} queued
                            </span>
                        )}
                        {onNewJob && (
                            <Button variant="outline" size="sm" onClick={onNewJob} className="h-7 px-2">
                                <PlusIcon className="w-4 h-4" />
                            </Button>
                        )}
                    </div>
                </div>

                <div className="relative">
                    <MagnifyingGlassIcon className="absolute left-2.5 top-1/2 -translate-y-1/2 w-4 h-4 text-muted-foreground" />
                    <Input
                        type="text"
                        placeholder="Search jobs..."
                        value={search}
                        onChange={(e) => setSearch(e.target.value)}
                        className="pl-8 h-8 text-sm"
                    />
                </div>

                <div className="flex flex-wrap gap-1">
                    {FILTERS.map((f) => (
                        <button
                            key={f.value}
                            onClick={() => setFilter(f.value)}
                            className={`px-2 py-1 text-xs rounded-full transition-colors ${
                                filter === f.value
                                    ? 'bg-primary text-primary-foreground'
                                    : 'bg-muted text-muted-foreground hover:bg-muted/80'
                            }`}
                        >
                            {f.label}
                        </button>
                    ))}
                </div>
            </div>

            <div className="flex-1 overflow-y-auto">
                {filteredJobs.length === 0 ? (
                    <div className="flex flex-col items-center justify-center h-full text-muted-foreground p-4">
                        <ClockIcon className="w-12 h-12 mb-2 opacity-50" />
                        <div className="text-sm">No jobs found</div>
                        <div className="text-xs mt-1">Convert an audiobook to see it here</div>
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

            {hasCompletedJobs && (
                <div className="p-2 border-t">
                    <Button
                        variant="ghost"
                        size="sm"
                        className="w-full text-xs"
                        onClick={clearCompleted}
                    >
                        <TrashIcon className="w-3 h-3 mr-1" />
                        Clear completed
                    </Button>
                </div>
            )}
        </div>
    );
}
