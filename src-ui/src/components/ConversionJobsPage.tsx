import { useState } from 'react';
import { ConversionStoreProvider, useConversionStore } from '@/context/ConversionStore';
import { JobList } from '@/components/conversion/JobList';
import { JobDetails } from '@/components/conversion/JobDetails';
import { NewJobDrawer } from '@/components/conversion/NewJobDrawer';

function JobsContent() {
    const { jobs, selectedJobId, selectJob, isLoading, error, refreshJobs } = useConversionStore();
    const [isNewJobOpen, setIsNewJobOpen] = useState(false);
    const selectedJob = jobs.find((j) => j.id === selectedJobId) ?? null;

    const handleJobCreated = async (jobId: string) => {
        await refreshJobs();
        selectJob(jobId);
    };

    if (isLoading) {
        return (
            <div className="flex items-center justify-center h-full">
                <div className="text-muted-foreground">Loading jobs...</div>
            </div>
        );
    }

    if (error) {
        return (
            <div className="flex items-center justify-center h-full">
                <div className="text-destructive">{error}</div>
            </div>
        );
    }

    return (
        <div className="flex h-full">
            <div className="w-[360px] border-r flex-shrink-0 overflow-hidden">
                <JobList onNewJob={() => setIsNewJobOpen(true)} />
            </div>
            <div className="flex-1 overflow-hidden">
                <JobDetails job={selectedJob} />
            </div>
            <NewJobDrawer
                isOpen={isNewJobOpen}
                onClose={() => setIsNewJobOpen(false)}
                onJobCreated={handleJobCreated}
            />
        </div>
    );
}

export function ConversionJobsPage() {
    return (
        <ConversionStoreProvider>
            <JobsContent />
        </ConversionStoreProvider>
    );
}
