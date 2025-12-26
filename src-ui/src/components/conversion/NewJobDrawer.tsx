import { useState, useEffect, useCallback } from 'react';
import { useApp } from '@/context/AppContext';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import type { ConversionPreset, ConversionSettings, DryRunResult } from '@/types/models';
import * as PresetsApi from '@/api/presets';
import * as ConversionApi from '@/api/conversion';
import * as FilesApi from '@/api/files';
import {
    XMarkIcon,
    FolderOpenIcon,
    PlayIcon,
    ExclamationTriangleIcon,
    CheckCircleIcon,
} from '@heroicons/react/24/outline';

interface NewJobDrawerProps {
    isOpen: boolean;
    onClose: () => void;
    onJobCreated?: (jobId: string) => void;
}

export function NewJobDrawer({ isOpen, onClose, onJobCreated }: NewJobDrawerProps) {
    const { activeAudiobook, settings, getEffectiveConversionDefaults } = useApp();
    const [presets, setPresets] = useState<ConversionPreset[]>([]);
    const [selectedPreset, setSelectedPreset] = useState<string>('');
    const [outputPath, setOutputPath] = useState('');
    const [dryRunResult, setDryRunResult] = useState<DryRunResult | null>(null);
    const [isLoadingDryRun, setIsLoadingDryRun] = useState(false);
    const [isEnqueuing, setIsEnqueuing] = useState(false);
    const [error, setError] = useState<string | null>(null);

    const defaults = getEffectiveConversionDefaults();

    useEffect(() => {
        if (isOpen) {
            PresetsApi.listPresets()
                .then(setPresets)
                .catch(console.error);

            if (activeAudiobook && settings?.default_output_directory) {
                const title = activeAudiobook.output_metadata?.title || activeAudiobook.title || 'audiobook';
                const safeName = title.replace(/[/\\?%*:|"<>]/g, '-');
                setOutputPath(`${settings.default_output_directory}/${safeName}.m4b`);
            }
        }
    }, [isOpen, activeAudiobook, settings?.default_output_directory]);

    const buildSettings = useCallback((): ConversionSettings | null => {
        if (!outputPath.trim()) return null;

        const preset = presets.find((p) => p.name === selectedPreset);
        const presetSettings = preset?.settings;

        return {
            bitrate_kbps: presetSettings?.bitrate_kbps ?? defaults.bitrate_kbps,
            sample_rate_hz: presetSettings?.sample_rate ?? defaults.sample_rate_hz,
            encoding_mode: defaults.encoding_mode,
            output_format: 'm4b',
            output_path: outputPath,
            concurrent_jobs: presetSettings?.concurrent_jobs ?? settings?.concurrent_jobs ?? 1,
            overwrite_existing: presetSettings?.overwrite_existing ?? defaults.overwrite_existing,
            embed_chapters: presetSettings?.embed_chapters ?? defaults.embed_chapters,
            embed_artwork: presetSettings?.embed_artwork ?? defaults.embed_artwork,
            copy_metadata: presetSettings?.copy_metadata ?? defaults.copy_metadata,
            normalize_audio: presetSettings?.normalize ?? false,
        };
    }, [outputPath, presets, selectedPreset, defaults, settings]);

    const runDryRun = useCallback(async () => {
        const convSettings = buildSettings();
        if (!convSettings || !activeAudiobook) return;

        setIsLoadingDryRun(true);
        setError(null);

        try {
            const result = await PresetsApi.dryRunConversion(convSettings);
            setDryRunResult(result);
        } catch (err) {
            setError(err instanceof Error ? err.message : 'Dry run failed');
        } finally {
            setIsLoadingDryRun(false);
        }
    }, [buildSettings, activeAudiobook]);

    useEffect(() => {
        if (outputPath && activeAudiobook) {
            const debounce = setTimeout(runDryRun, 500);
            return () => clearTimeout(debounce);
        }
    }, [outputPath, selectedPreset, runDryRun, activeAudiobook]);

    const handlePickOutput = async () => {
        const title = activeAudiobook?.output_metadata?.title || activeAudiobook?.title || 'audiobook';
        const safeName = title.replace(/[/\\?%*:|"<>]/g, '-');
        const path = await FilesApi.pickOutputFile(`${safeName}.m4b`);
        if (path) {
            setOutputPath(path);
        }
    };

    const handleEnqueue = async () => {
        const convSettings = buildSettings();
        if (!convSettings || !activeAudiobook) return;

        setIsEnqueuing(true);
        setError(null);

        try {
            const jobId = await ConversionApi.startConversion(activeAudiobook.id, convSettings);
            onJobCreated?.(jobId);
            onClose();
        } catch (err) {
            setError(err instanceof Error ? err.message : 'Failed to start conversion');
        } finally {
            setIsEnqueuing(false);
        }
    };

    if (!isOpen) return null;

    const canEnqueue = activeAudiobook && outputPath.trim() && !isEnqueuing && !dryRunResult?.output_exists;

    return (
        <div className="fixed inset-0 z-50 flex">
            <div className="absolute inset-0 bg-black/50" onClick={onClose} />
            <div className="absolute right-0 top-0 bottom-0 w-[480px] bg-background border-l shadow-xl flex flex-col">
                <div className="flex items-center justify-between p-4 border-b">
                    <h2 className="text-lg font-semibold">New Conversion Job</h2>
                    <Button variant="ghost" size="sm" onClick={onClose}>
                        <XMarkIcon className="w-5 h-5" />
                    </Button>
                </div>

                <div className="flex-1 overflow-y-auto p-4 space-y-6">
                    {!activeAudiobook ? (
                        <div className="text-center text-muted-foreground py-8">
                            <p>No audiobook loaded.</p>
                            <p className="text-sm mt-1">Import files first to create a conversion job.</p>
                        </div>
                    ) : (
                        <>
                            <div className="space-y-2">
                                <Label>Audiobook</Label>
                                <div className="p-3 bg-muted rounded-md">
                                    <div className="font-medium">
                                        {activeAudiobook.output_metadata?.title || activeAudiobook.title}
                                    </div>
                                    <div className="text-sm text-muted-foreground">
                                        {activeAudiobook.files.length} files •{' '}
                                        {activeAudiobook.chapters?.length || 0} chapters
                                    </div>
                                </div>
                            </div>

                            <div className="space-y-2">
                                <Label>Preset (optional)</Label>
                                <select
                                    value={selectedPreset}
                                    onChange={(e) => setSelectedPreset(e.target.value)}
                                    className="w-full px-3 py-2 border rounded-md bg-background text-sm"
                                >
                                    <option value="">Use defaults</option>
                                    {presets.map((p) => (
                                        <option key={p.name} value={p.name}>
                                            {p.name}
                                        </option>
                                    ))}
                                </select>
                            </div>

                            <div className="space-y-2">
                                <Label>Output Path</Label>
                                <div className="flex gap-2">
                                    <Input
                                        value={outputPath}
                                        onChange={(e) => setOutputPath(e.target.value)}
                                        placeholder="/path/to/output.m4b"
                                        className="flex-1"
                                    />
                                    <Button variant="outline" size="sm" onClick={handlePickOutput}>
                                        <FolderOpenIcon className="w-4 h-4" />
                                    </Button>
                                </div>
                            </div>

                            {isLoadingDryRun && (
                                <div className="flex items-center gap-2 text-sm text-muted-foreground">
                                    <div className="w-4 h-4 border-2 border-t-primary rounded-full animate-spin" />
                                    Calculating preview...
                                </div>
                            )}

                            {dryRunResult && (
                                <div className="border rounded-md divide-y">
                                    <div className="p-3">
                                        <div className="text-sm font-medium mb-2">Preview</div>
                                        <div className="grid grid-cols-2 gap-2 text-sm">
                                            <div>
                                                <span className="text-muted-foreground">Est. Size: </span>
                                                {formatBytes(dryRunResult.estimated_output_size_bytes)}
                                            </div>
                                            <div>
                                                <span className="text-muted-foreground">Est. Time: </span>
                                                {formatTime(dryRunResult.estimated_time_seconds)}
                                            </div>
                                        </div>
                                    </div>

                                    {dryRunResult.output_exists && (
                                        <div className="p-3 bg-yellow-500/10 flex items-start gap-2">
                                            <ExclamationTriangleIcon className="w-5 h-5 text-yellow-500 flex-shrink-0" />
                                            <div className="text-sm">
                                                <div className="font-medium text-yellow-600">Output exists</div>
                                                <div className="text-muted-foreground">
                                                    A file already exists at this location.
                                                </div>
                                            </div>
                                        </div>
                                    )}

                                    {dryRunResult.warnings.length > 0 && (
                                        <div className="p-3 space-y-1">
                                            {dryRunResult.warnings.map((w, i) => (
                                                <div key={i} className="text-sm text-yellow-600 flex items-start gap-1">
                                                    <ExclamationTriangleIcon className="w-4 h-4 flex-shrink-0" />
                                                    {w}
                                                </div>
                                            ))}
                                        </div>
                                    )}

                                    {!dryRunResult.output_exists && dryRunResult.warnings.length === 0 && (
                                        <div className="p-3 bg-green-500/10 flex items-center gap-2">
                                            <CheckCircleIcon className="w-5 h-5 text-green-500" />
                                            <span className="text-sm text-green-600">Ready to convert</span>
                                        </div>
                                    )}
                                </div>
                            )}

                            {error && (
                                <div className="p-3 bg-destructive/10 rounded-md text-sm text-destructive">
                                    {error}
                                </div>
                            )}
                        </>
                    )}
                </div>

                <div className="p-4 border-t flex justify-end gap-2">
                    <Button variant="outline" onClick={onClose}>
                        Cancel
                    </Button>
                    <Button onClick={handleEnqueue} disabled={!canEnqueue}>
                        {isEnqueuing ? (
                            <>
                                <div className="w-4 h-4 border-2 border-t-transparent rounded-full animate-spin mr-2" />
                                Starting...
                            </>
                        ) : (
                            <>
                                <PlayIcon className="w-4 h-4 mr-2" />
                                Start Conversion
                            </>
                        )}
                    </Button>
                </div>
            </div>
        </div>
    );
}

function formatBytes(bytes: number): string {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
    return `${(bytes / (1024 * 1024 * 1024)).toFixed(2)} GB`;
}

function formatTime(seconds: number): string {
    if (seconds < 60) return `${Math.round(seconds)}s`;
    const mins = Math.floor(seconds / 60);
    const secs = Math.round(seconds % 60);
    if (mins < 60) return `${mins}m ${secs}s`;
    const hours = Math.floor(mins / 60);
    return `${hours}h ${mins % 60}m`;
}
