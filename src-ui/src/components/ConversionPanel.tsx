import { useState } from "react";
import { Button } from "./ui/button";
import { Input } from "./ui/input";
import { Label } from "./ui/label";
import { useApp } from "@/context/AppContext";
import * as ConversionApi from "@/api/conversion";
import type { ConversionSettings, JobStatus } from "@/types/models";

export function ConversionPanel() {
    const { activeAudiobook } = useApp();
    const [isStarting, setIsStarting] = useState(false);
    const [status, setStatus] = useState<string>("");
    const [outputPath, setOutputPath] = useState("output.m4b");
    const [currentJobId, setCurrentJobId] = useState<string | null>(null);
    const [jobStatus, setJobStatus] = useState<JobStatus | null>(null);

    const handleStart = async () => {
        if (!activeAudiobook) return;
        setIsStarting(true);
        setStatus("Starting...");

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

            const jobId = await ConversionApi.startConversion(settings);
            setCurrentJobId(jobId);
            setJobStatus('running');
            setStatus(`Started Job: ${jobId}`);
        } catch (e) {
            console.error(e);
            setStatus(`Failed to start: ${e}`);
        } finally {
            setIsStarting(false);
        }
    };

    const handlePause = async () => {
        if (!currentJobId) return;
        try {
            const paused = await ConversionApi.pauseConversion(currentJobId);
            if (paused) {
                setJobStatus('paused');
                setStatus("Paused");
            } else {
                setStatus("Cannot pause - job not running");
            }
        } catch (e) {
            console.error(e);
            setStatus(`Failed to pause: ${e}`);
        }
    };

    const handleResume = async () => {
        if (!currentJobId) return;
        try {
            const resumed = await ConversionApi.resumeConversion(currentJobId);
            if (resumed) {
                setJobStatus('running');
                setStatus("Resumed");
            } else {
                setStatus("Cannot resume - job not paused");
            }
        } catch (e) {
            console.error(e);
            setStatus(`Failed to resume: ${e}`);
        }
    };

    const handleCancel = async () => {
        if (!currentJobId) return;
        try {
            await ConversionApi.cancelConversion(currentJobId);
            setJobStatus('cancelled');
            setCurrentJobId(null);
            setStatus("Cancelled");
        } catch (e) {
            console.error(e);
            setStatus(`Failed to cancel: ${e}`);
        }
    };

    if (!activeAudiobook) return null;

    return (
        <div className="p-4 border rounded-lg bg-card mt-4 space-y-4">
            <div className="flex justify-between items-center">
                <h3 className="font-semibold">Conversion</h3>
                <span className="text-sm text-muted-foreground">{status}</span>
            </div>

            <div className="flex gap-4 items-end">
                <div className="grid gap-2 flex-1">
                    <Label htmlFor="output-path">Output Path</Label>
                    <Input
                        id="output-path"
                        value={outputPath}
                        onChange={(e) => setOutputPath(e.target.value)}
                    />
                </div>
                <Button onClick={handleStart} disabled={isStarting || jobStatus === 'running' || jobStatus === 'paused'}>
                    {isStarting ? "Starting..." : "Start Conversion"}
                </Button>
            </div>

            {currentJobId && (
                <div className="flex gap-2">
                    {jobStatus === 'running' && (
                        <Button variant="secondary" onClick={handlePause}>
                            Pause
                        </Button>
                    )}
                    {jobStatus === 'paused' && (
                        <Button variant="secondary" onClick={handleResume}>
                            Resume
                        </Button>
                    )}
                    {(jobStatus === 'running' || jobStatus === 'paused') && (
                        <Button variant="destructive" onClick={handleCancel}>
                            Cancel
                        </Button>
                    )}
                </div>
            )}
        </div>
    );
}
