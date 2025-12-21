import { useState } from "react";
import { Button } from "./ui/button";
import { useApp } from "@/context/AppContext";
import { dryRunConversion } from "@/api/presets";
import type { DryRunResult, ConversionSettings } from "@/types/models";

function formatBytes(bytes: number): string {
    if (bytes === 0) return "0 B";
    const units = ["B", "KB", "MB", "GB", "TB"];
    const i = Math.floor(Math.log(bytes) / Math.log(1024));
    const value = bytes / Math.pow(1024, i);
    return `${value.toFixed(1)} ${units[i]}`;
}

function formatDuration(ms: number): string {
    const totalSeconds = Math.floor(ms / 1000);
    const hours = Math.floor(totalSeconds / 3600);
    const minutes = Math.floor((totalSeconds % 3600) / 60);
    const seconds = totalSeconds % 60;

    if (hours > 0) {
        return `${hours}h ${minutes}m ${seconds}s`;
    }
    if (minutes > 0) {
        return `${minutes}m ${seconds}s`;
    }
    return `${seconds}s`;
}

function formatTime(seconds: number): string {
    const hours = Math.floor(seconds / 3600);
    const minutes = Math.floor((seconds % 3600) / 60);
    const secs = Math.floor(seconds % 60);

    if (hours > 0) {
        return `${hours}h ${minutes}m ${secs}s`;
    }
    if (minutes > 0) {
        return `${minutes}m ${secs}s`;
    }
    return `${secs}s`;
}

export function DryRunPreview() {
    const { activeAudiobook } = useApp();
    const [result, setResult] = useState<DryRunResult | null>(null);
    const [loading, setLoading] = useState(false);
    const [error, setError] = useState<string | null>(null);

    const handlePreview = async () => {
        if (!activeAudiobook) return;

        setLoading(true);
        setError(null);
        setResult(null);

        try {
            const settings: ConversionSettings = {
                bitrate_kbps: 64,
                output_format: "m4b",
                output_path: "output.m4b",
                concurrent_jobs: 1,
                overwrite_existing: false,
                embed_chapters: true,
                embed_artwork: true,
                copy_metadata: true,
            };

            const dryRunResult = await dryRunConversion(settings);
            setResult(dryRunResult);
        } catch (e) {
            console.error(e);
            setError(e instanceof Error ? e.message : String(e));
        } finally {
            setLoading(false);
        }
    };

    if (!activeAudiobook) return null;

    return (
        <div className="p-4 border rounded-lg bg-card mt-4 space-y-4">
            <div className="flex justify-between items-center">
                <h3 className="font-semibold">Conversion Preview</h3>
                <Button onClick={handlePreview} disabled={loading} variant="secondary">
                    {loading ? "Analyzing..." : "Preview Conversion"}
                </Button>
            </div>

            {loading && (
                <div className="text-sm text-muted-foreground">
                    Analyzing files...
                </div>
            )}

            {error && (
                <div className="text-sm text-destructive">
                    Error: {error}
                </div>
            )}

            {result && (
                <div className="space-y-3 text-sm">
                    <div className="grid grid-cols-2 gap-4">
                        <div>
                            <span className="text-muted-foreground">Input:</span>{" "}
                            {result.input_file_count} file{result.input_file_count !== 1 ? "s" : ""},{" "}
                            {formatDuration(result.input_duration_ms)},{" "}
                            {formatBytes(result.input_size_bytes)}
                        </div>
                        <div>
                            <span className="text-muted-foreground">Output:</span>{" "}
                            ~{formatBytes(result.estimated_output_size_bytes)},{" "}
                            ~{formatTime(result.estimated_time_seconds)}
                        </div>
                    </div>

                    <div className="grid grid-cols-2 gap-4">
                        <div>
                            <span className="text-muted-foreground">Chapters:</span>{" "}
                            {result.chapter_count}
                        </div>
                        <div>
                            <span className="text-muted-foreground">Artwork:</span>{" "}
                            {result.has_artwork ? "Yes" : "No"}
                        </div>
                    </div>

                    {result.output_exists && (
                        <div className="text-amber-600">
                            ⚠ Output file already exists: {result.output_path}
                        </div>
                    )}

                    {result.warnings.length > 0 && (
                        <div className="space-y-1">
                            {result.warnings.map((warning, index) => (
                                <div key={index} className="text-amber-600">
                                    ⚠ {warning}
                                </div>
                            ))}
                        </div>
                    )}
                </div>
            )}
        </div>
    );
}
