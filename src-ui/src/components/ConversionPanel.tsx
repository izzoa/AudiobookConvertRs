import { useState } from "react";
import { Button } from "./ui/button";
import { Input } from "./ui/input";
import { Label } from "./ui/label";
import { useApp } from "@/context/AppContext";
import * as ConversionApi from "@/api/conversion";
import type { ConversionSettings } from "@/types/models";

export function ConversionPanel() {
    const { activeAudiobook } = useApp();
    const [isStarting, setIsStarting] = useState(false);
    const [status, setStatus] = useState<string>("");
    const [outputPath, setOutputPath] = useState("output.m4b");

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
            setStatus(`Started Job: ${jobId}`);
        } catch (e) {
            console.error(e);
            setStatus(`Failed to start: ${e}`);
        } finally {
            setIsStarting(false);
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
                <Button onClick={handleStart} disabled={isStarting}>
                    {isStarting ? "Starting..." : "Start Conversion"}
                </Button>
            </div>
        </div>
    );
}
