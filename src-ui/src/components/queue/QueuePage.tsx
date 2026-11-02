import { PlayIcon, PauseIcon, TrashIcon, BookOpenIcon, QueueListIcon } from "@heroicons/react/24/outline";
import { Button } from "@/components/ui/button";
import { JobList } from "@/components/conversion/JobList";
import { JobDetails } from "@/components/conversion/JobDetails";
import { useConversionStore } from "@/context/useConversionStore";
import { useApp } from "@/context/useApp";
import { useToast } from "@/components/ui/use-toast";
import * as ConversionApi from "@/api/conversion";
import { JobStatus } from "@/types/models";

/**
 * Jobs are a destination now, not a panel stapled under the editor. The page
 * owns queue-level commands; per-job commands stay on the job itself.
 */
export function QueuePage() {
    const { setView, settings, activeAudiobook } = useApp();
    const { jobs, queueState, selectedJobId, isLoading, error, refreshJobs, refreshQueueState, clearCompleted } =
        useConversionStore();
    const { toast } = useToast();

    const selectedJob = jobs.find((j) => j.id === selectedJobId) ?? null;
    const running = queueState?.state === "running";
    // The job list refreshes on a poll, so right after pressing Convert it is
    // briefly empty while the queue already reports work. Don't claim idle then.
    const busy = (queueState?.active_jobs ?? 0) > 0 || (queueState?.pending_jobs ?? 0) > 0;
    const pendingCount = jobs.filter((j) => j.status === JobStatus.Queued).length;
    const completedCount = jobs.filter(
        (j) => j.status === JobStatus.Complete || j.status === JobStatus.Error || j.status === JobStatus.Cancelled
    ).length;

    const toggleQueue = async () => {
        try {
            if (running) await ConversionApi.stopConversionQueue();
            else await ConversionApi.startConversionQueue(settings?.concurrent_jobs ?? 1);
            await refreshQueueState();
            await refreshJobs();
        } catch (err) {
            toast({ title: "Queue command failed", description: String(err), variant: "destructive" });
        }
    };

    return (
        <div className="flex-1 flex flex-col overflow-hidden">
            <div className="shrink-0 h-11 px-3 border-b flex items-center gap-2 no-select">
                <h1 className="text-sm font-semibold">Queue</h1>
                {queueState && (
                    <span className="text-xs text-muted-foreground nums">
                        {queueState.active_jobs} active · {queueState.pending_jobs} waiting ·{" "}
                        {queueState.worker_count} {queueState.worker_count === 1 ? "worker" : "workers"}
                    </span>
                )}

                <div className="flex-1" />

                <Button
                    variant="ghost"
                    size="sm"
                    onClick={toggleQueue}
                    disabled={!running && pendingCount === 0 && (queueState?.pending_jobs ?? 0) === 0}
                    title={!running && pendingCount === 0 ? "Nothing is waiting to convert" : undefined}
                >
                    {running ? (
                        <>
                            <PauseIcon className="w-4 h-4 mr-1.5" />
                            Pause queue
                        </>
                    ) : (
                        <>
                            <PlayIcon className="w-4 h-4 mr-1.5" />
                            Start queue
                        </>
                    )}
                </Button>
                <Button variant="ghost" size="sm" onClick={clearCompleted} disabled={completedCount === 0}>
                    <TrashIcon className="w-4 h-4 mr-1.5" />
                    Clear finished
                </Button>
            </div>

            {isLoading && jobs.length === 0 ? (
                <Centered>Loading jobs…</Centered>
            ) : error ? (
                <Centered tone="danger">{error}</Centered>
            ) : jobs.length === 0 && !busy ? (
                <div className="flex-1 flex flex-col items-center justify-center gap-4 text-center px-8">
                    <span className="w-12 h-12 rounded-xl bg-muted flex items-center justify-center text-muted-foreground">
                        <QueueListIcon className="w-6 h-6" />
                    </span>
                    <div className="max-w-[340px]">
                        <p className="font-medium">Nothing in the queue</p>
                        <p className="text-sm text-muted-foreground mt-1">
                            {activeAudiobook
                                ? `Press Convert to M4B on ${activeAudiobook.title || "the open book"} to add it here. Books keep converting while you prepare the next one.`
                                : "Open a book and press Convert to M4B. Books keep converting while you prepare the next one."}
                        </p>
                    </div>
                    <Button variant="outline" onClick={() => setView("book")}>
                        <BookOpenIcon className="w-4 h-4 mr-2" />
                        {activeAudiobook ? "Back to the book" : "Open a book"}
                    </Button>
                </div>
            ) : (
                <div className="flex-1 flex overflow-hidden">
                    <div className="w-[340px] shrink-0 border-r overflow-hidden">
                        <JobList />
                    </div>
                    <div className="flex-1 overflow-hidden min-w-0">
                        <JobDetails job={selectedJob} />
                    </div>
                </div>
            )}
        </div>
    );
}

function Centered({ children, tone }: { children: React.ReactNode; tone?: "danger" }) {
    return (
        <div className="flex-1 flex items-center justify-center">
            <div className={tone === "danger" ? "text-destructive" : "text-muted-foreground"}>{children}</div>
        </div>
    );
}
