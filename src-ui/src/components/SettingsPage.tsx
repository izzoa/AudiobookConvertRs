import { useState, useEffect } from 'react';
import { useApp } from '@/context/AppContext';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { ArrowLeftIcon, FolderOpenIcon, CheckCircleIcon, XCircleIcon } from '@heroicons/react/24/outline';
import * as SettingsApi from '@/api/settings';
import * as FilesApi from '@/api/files';
import type { UserSettings, FFmpegStatus, ConversionDefaults } from '@/types/models';

export function SettingsPage() {
    const { settings, updateSettings, refreshSettings, setView, activeProject, setActiveProject } = useApp();
    const [draftSettings, setDraftSettings] = useState<UserSettings | null>(null);
    const [draftProjectDefaults, setDraftProjectDefaults] = useState<ConversionDefaults | null>(null);
    const [ffmpegStatus, setFfmpegStatus] = useState<FFmpegStatus | null>(null);
    const [isSaving, setIsSaving] = useState(false);
    const [error, setError] = useState<string | null>(null);
    const [success, setSuccess] = useState<string | null>(null);

    // Load initial values
    useEffect(() => {
        if (settings) {
            setDraftSettings({ ...settings });
        }
    }, [settings]);

    // Load project defaults when project changes
    useEffect(() => {
        if (activeProject?.conversion_defaults) {
            setDraftProjectDefaults({ ...activeProject.conversion_defaults });
        } else {
            setDraftProjectDefaults(null);
        }
    }, [activeProject]);

    // Load FFmpeg info
    useEffect(() => {
        async function loadFFmpegInfo() {
            try {
                const info = await SettingsApi.getFFmpegInfo();
                setFfmpegStatus(info);
            } catch (e) {
                console.error("Failed to load FFmpeg info:", e);
            }
        }
        loadFFmpegInfo();
    }, []);

    const handleBackToApp = () => {
        setView('workflow');
    };

    const handleSave = async () => {
        if (!draftSettings) return;

        setIsSaving(true);
        setError(null);
        setSuccess(null);

        try {
            // Validate
            if (draftSettings.default_bitrate_kbps < 8 || draftSettings.default_bitrate_kbps > 320) {
                throw new Error("Bitrate must be between 8 and 320 kbps");
            }
            if (draftSettings.concurrent_jobs < 1 || draftSettings.concurrent_jobs > 16) {
                throw new Error("Concurrent jobs must be between 1 and 16");
            }

            await updateSettings(draftSettings);
            setSuccess("Settings saved successfully");
        } catch (e) {
            setError(e instanceof Error ? e.message : "Failed to save settings");
        } finally {
            setIsSaving(false);
        }
    };

    const handleReset = async () => {
        if (!confirm("Reset all settings to defaults? This cannot be undone.")) {
            return;
        }

        setIsSaving(true);
        setError(null);

        try {
            await SettingsApi.resetSettings();
            await refreshSettings();
            setSuccess("Settings reset to defaults");
        } catch (e) {
            setError(e instanceof Error ? e.message : "Failed to reset settings");
        } finally {
            setIsSaving(false);
        }
    };

    const handleCancel = () => {
        if (settings) {
            setDraftSettings({ ...settings });
        }
        setError(null);
        setSuccess(null);
    };

    const handlePickOutputDirectory = async () => {
        try {
            const path = await FilesApi.pickDirectory();
            if (path && draftSettings) {
                setDraftSettings({ ...draftSettings, default_output_directory: path });
            }
        } catch (e) {
            console.error("Failed to pick directory:", e);
        }
    };

    const handleRefreshFFmpeg = async () => {
        try {
            const info = await SettingsApi.getFFmpegInfo();
            setFfmpegStatus(info);
        } catch (e) {
            console.error("Failed to refresh FFmpeg info:", e);
        }
    };

    const handleApplyProjectDefaults = () => {
        if (!activeProject || !draftProjectDefaults) return;

        setActiveProject({
            ...activeProject,
            conversion_defaults: draftProjectDefaults,
        });
        setSuccess("Project defaults updated. Save the project to persist changes.");
    };

    const handleClearProjectDefaults = () => {
        if (!activeProject) return;

        setActiveProject({
            ...activeProject,
            conversion_defaults: undefined,
        });
        setDraftProjectDefaults(null);
        setSuccess("Project defaults cleared. Will use global defaults.");
    };

    const updateDraft = <K extends keyof UserSettings>(key: K, value: UserSettings[K]) => {
        if (draftSettings) {
            setDraftSettings({ ...draftSettings, [key]: value });
        }
    };

    const updateProjectDraft = <K extends keyof ConversionDefaults>(key: K, value: ConversionDefaults[K]) => {
        const defaults = draftProjectDefaults ?? {
            bitrate_kbps: settings?.default_bitrate_kbps ?? 128,
            sample_rate_hz: settings?.default_sample_rate,
            overwrite_existing: settings?.overwrite_existing_by_default ?? false,
            embed_chapters: settings?.embed_chapters_by_default ?? true,
            embed_artwork: settings?.embed_artwork_by_default ?? true,
            copy_metadata: settings?.copy_metadata_by_default ?? true,
        };
        setDraftProjectDefaults({ ...defaults, [key]: value });
    };

    if (!draftSettings) {
        return (
            <div className="p-6 flex items-center justify-center">
                <p className="text-muted-foreground">Loading settings...</p>
            </div>
        );
    }

    return (
        <div className="p-6 space-y-6 max-w-3xl mx-auto">
            {/* Header */}
            <div className="flex items-center gap-4">
                <Button variant="ghost" size="sm" onClick={handleBackToApp}>
                    <ArrowLeftIcon className="w-4 h-4 mr-2" />
                    Back to App
                </Button>
                <h1 className="text-2xl font-semibold">Settings</h1>
            </div>

            {/* Feedback messages */}
            {error && (
                <div className="bg-destructive/10 text-destructive px-4 py-3 rounded-md text-sm">
                    {error}
                </div>
            )}
            {success && (
                <div className="bg-green-500/10 text-green-600 dark:text-green-400 px-4 py-3 rounded-md text-sm">
                    {success}
                </div>
            )}

            {/* Global Defaults Section */}
            <section className="space-y-4 border rounded-lg p-4">
                <h2 className="text-lg font-medium">Global Defaults</h2>

                {/* Conversion Defaults */}
                <div className="space-y-4">
                    <h3 className="text-sm font-medium text-muted-foreground">Conversion</h3>
                    <div className="grid grid-cols-2 gap-4">
                        <div className="space-y-2">
                            <Label htmlFor="bitrate">Default Bitrate (kbps)</Label>
                            <select
                                id="bitrate"
                                className="w-full h-10 px-3 rounded-md border border-input bg-background"
                                value={draftSettings.default_bitrate_kbps}
                                onChange={(e) => updateDraft('default_bitrate_kbps', parseInt(e.target.value))}
                            >
                                <option value="320">320 kbps</option>
                                <option value="256">256 kbps</option>
                                <option value="192">192 kbps</option>
                                <option value="160">160 kbps</option>
                                <option value="128">128 kbps</option>
                                <option value="96">96 kbps</option>
                                <option value="64">64 kbps</option>
                                <option value="48">48 kbps</option>
                                <option value="32">32 kbps</option>
                            </select>
                        </div>
                        <div className="space-y-2">
                            <Label htmlFor="sample-rate">Sample Rate (Hz)</Label>
                            <select
                                id="sample-rate"
                                className="w-full h-10 px-3 rounded-md border border-input bg-background"
                                value={draftSettings.default_sample_rate ?? 'match'}
                                onChange={(e) => updateDraft('default_sample_rate', e.target.value === 'match' ? undefined : parseInt(e.target.value))}
                            >
                                <option value="match">Match source</option>
                                <option value="44100">44100 Hz</option>
                                <option value="48000">48000 Hz</option>
                                <option value="22050">22050 Hz</option>
                            </select>
                        </div>
                    </div>

                    <div className="grid grid-cols-2 gap-4">
                        <label className="flex items-center gap-2">
                            <input
                                type="checkbox"
                                checked={draftSettings.embed_chapters_by_default}
                                onChange={(e) => updateDraft('embed_chapters_by_default', e.target.checked)}
                            />
                            <span className="text-sm">Embed chapters by default</span>
                        </label>
                        <label className="flex items-center gap-2">
                            <input
                                type="checkbox"
                                checked={draftSettings.embed_artwork_by_default}
                                onChange={(e) => updateDraft('embed_artwork_by_default', e.target.checked)}
                            />
                            <span className="text-sm">Embed artwork by default</span>
                        </label>
                        <label className="flex items-center gap-2">
                            <input
                                type="checkbox"
                                checked={draftSettings.copy_metadata_by_default}
                                onChange={(e) => updateDraft('copy_metadata_by_default', e.target.checked)}
                            />
                            <span className="text-sm">Copy metadata by default</span>
                        </label>
                        <label className="flex items-center gap-2">
                            <input
                                type="checkbox"
                                checked={draftSettings.overwrite_existing_by_default}
                                onChange={(e) => updateDraft('overwrite_existing_by_default', e.target.checked)}
                            />
                            <span className="text-sm">Overwrite existing by default</span>
                        </label>
                    </div>
                </div>

                {/* Output Defaults */}
                <div className="space-y-4 pt-4 border-t">
                    <h3 className="text-sm font-medium text-muted-foreground">Output</h3>
                    <div className="space-y-2">
                        <Label htmlFor="output-dir">Default Output Directory</Label>
                        <div className="flex gap-2">
                            <Input
                                id="output-dir"
                                value={draftSettings.default_output_directory ?? ''}
                                onChange={(e) => updateDraft('default_output_directory', e.target.value || undefined)}
                                placeholder="Same as input files"
                            />
                            <Button variant="outline" size="icon" onClick={handlePickOutputDirectory}>
                                <FolderOpenIcon className="w-4 h-4" />
                            </Button>
                        </div>
                        <p className="text-xs text-muted-foreground">Leave empty to save output files in the same directory as input files</p>
                    </div>
                </div>

                {/* Queue Defaults */}
                <div className="space-y-4 pt-4 border-t">
                    <h3 className="text-sm font-medium text-muted-foreground">Queue</h3>
                    <div className="space-y-2">
                        <Label htmlFor="concurrent">Concurrent Jobs</Label>
                        <Input
                            id="concurrent"
                            type="number"
                            min={1}
                            max={16}
                            value={draftSettings.concurrent_jobs}
                            onChange={(e) => updateDraft('concurrent_jobs', parseInt(e.target.value) || 1)}
                        />
                        <p className="text-xs text-muted-foreground">Number of conversions to run in parallel</p>
                    </div>
                </div>

                {/* Artwork Behavior */}
                <div className="space-y-4 pt-4 border-t">
                    <h3 className="text-sm font-medium text-muted-foreground">Artwork</h3>
                    <div className="grid grid-cols-2 gap-4">
                        <label className="flex items-center gap-2">
                            <input
                                type="checkbox"
                                checked={draftSettings.auto_detect_artwork}
                                onChange={(e) => updateDraft('auto_detect_artwork', e.target.checked)}
                            />
                            <span className="text-sm">Auto-detect artwork from audio files</span>
                        </label>
                        <label className="flex items-center gap-2">
                            <input
                                type="checkbox"
                                checked={draftSettings.scan_for_cover_images}
                                onChange={(e) => updateDraft('scan_for_cover_images', e.target.checked)}
                            />
                            <span className="text-sm">Scan folder for cover images</span>
                        </label>
                    </div>
                </div>

                {/* UI Preferences */}
                <div className="space-y-4 pt-4 border-t">
                    <h3 className="text-sm font-medium text-muted-foreground">Interface</h3>
                    <div className="grid grid-cols-2 gap-4">
                        <div className="space-y-2">
                            <Label htmlFor="theme">Theme</Label>
                            <select
                                id="theme"
                                className="w-full h-10 px-3 rounded-md border border-input bg-background"
                                value={draftSettings.theme}
                                onChange={(e) => updateDraft('theme', e.target.value as 'light' | 'dark' | 'system')}
                            >
                                <option value="system">System</option>
                                <option value="light">Light</option>
                                <option value="dark">Dark</option>
                            </select>
                        </div>
                        <label className="flex items-center gap-2 pt-6">
                            <input
                                type="checkbox"
                                checked={draftSettings.show_advanced_options}
                                onChange={(e) => updateDraft('show_advanced_options', e.target.checked)}
                            />
                            <span className="text-sm">Show advanced options</span>
                        </label>
                    </div>
                </div>

                {/* System/Debug */}
                <div className="space-y-4 pt-4 border-t">
                    <h3 className="text-sm font-medium text-muted-foreground">System</h3>
                    <div className="grid grid-cols-2 gap-4">
                        <div className="space-y-2">
                            <Label htmlFor="log-level">Log Level</Label>
                            <select
                                id="log-level"
                                className="w-full h-10 px-3 rounded-md border border-input bg-background"
                                value={draftSettings.log_level}
                                onChange={(e) => updateDraft('log_level', e.target.value as UserSettings['log_level'])}
                            >
                                <option value="error">Error</option>
                                <option value="warn">Warn</option>
                                <option value="info">Info</option>
                                <option value="debug">Debug</option>
                                <option value="trace">Trace</option>
                            </select>
                            <p className="text-xs text-muted-foreground">Takes effect on next launch</p>
                        </div>
                    </div>

                    {/* FFmpeg Status */}
                    <div className="p-3 bg-muted/50 rounded-md space-y-2">
                        <div className="flex items-center justify-between">
                            <span className="text-sm font-medium">FFmpeg Status</span>
                            <Button variant="ghost" size="sm" onClick={handleRefreshFFmpeg}>
                                Refresh
                            </Button>
                        </div>
                        {ffmpegStatus ? (
                            <div className="space-y-1 text-sm">
                                <div className="flex items-center gap-2">
                                    {ffmpegStatus.available ? (
                                        <CheckCircleIcon className="w-4 h-4 text-green-500" />
                                    ) : (
                                        <XCircleIcon className="w-4 h-4 text-red-500" />
                                    )}
                                    <span>{ffmpegStatus.available ? 'Detected' : 'Not found'}</span>
                                </div>
                                {ffmpegStatus.version && (
                                    <p className="text-muted-foreground">Version: {ffmpegStatus.version}</p>
                                )}
                                {ffmpegStatus.path && (
                                    <p className="text-muted-foreground text-xs truncate" title={ffmpegStatus.path}>
                                        Path: {ffmpegStatus.path}
                                    </p>
                                )}
                                {ffmpegStatus.supports_libfdk_aac && (
                                    <p className="text-green-600 dark:text-green-400 text-xs">libfdk_aac encoder available (better quality)</p>
                                )}
                            </div>
                        ) : (
                            <p className="text-sm text-muted-foreground">Checking...</p>
                        )}
                    </div>
                </div>

                {/* Action Buttons */}
                <div className="flex gap-3 pt-4 border-t">
                    <Button onClick={handleSave} disabled={isSaving}>
                        {isSaving ? 'Saving...' : 'Save'}
                    </Button>
                    <Button variant="outline" onClick={handleReset} disabled={isSaving}>
                        Reset to Defaults
                    </Button>
                    <Button variant="ghost" onClick={handleCancel} disabled={isSaving}>
                        Cancel
                    </Button>
                </div>
            </section>

            {/* Project Overrides Section */}
            {activeProject && (
                <section className="space-y-4 border rounded-lg p-4">
                    <h2 className="text-lg font-medium">Project Overrides</h2>
                    <p className="text-sm text-muted-foreground">
                        These settings override global defaults for the current project only.
                        Save the project to persist these changes.
                    </p>

                    <div className="space-y-4">
                        <div className="grid grid-cols-2 gap-4">
                            <div className="space-y-2">
                                <Label htmlFor="project-bitrate">Bitrate (kbps)</Label>
                                <select
                                    id="project-bitrate"
                                    className="w-full h-10 px-3 rounded-md border border-input bg-background"
                                    value={draftProjectDefaults?.bitrate_kbps ?? settings?.default_bitrate_kbps ?? 128}
                                    onChange={(e) => updateProjectDraft('bitrate_kbps', parseInt(e.target.value))}
                                >
                                    <option value="320">320 kbps</option>
                                    <option value="256">256 kbps</option>
                                    <option value="192">192 kbps</option>
                                    <option value="160">160 kbps</option>
                                    <option value="128">128 kbps</option>
                                    <option value="96">96 kbps</option>
                                    <option value="64">64 kbps</option>
                                    <option value="48">48 kbps</option>
                                    <option value="32">32 kbps</option>
                                </select>
                            </div>
                            <div className="space-y-2">
                                <Label htmlFor="project-sample-rate">Sample Rate (Hz)</Label>
                                <select
                                    id="project-sample-rate"
                                    className="w-full h-10 px-3 rounded-md border border-input bg-background"
                                    value={draftProjectDefaults?.sample_rate_hz ?? 'match'}
                                    onChange={(e) => updateProjectDraft('sample_rate_hz', e.target.value === 'match' ? undefined : parseInt(e.target.value))}
                                >
                                    <option value="match">Match source</option>
                                    <option value="44100">44100 Hz</option>
                                    <option value="48000">48000 Hz</option>
                                    <option value="22050">22050 Hz</option>
                                </select>
                            </div>
                        </div>

                        <div className="grid grid-cols-2 gap-4">
                            <label className="flex items-center gap-2">
                                <input
                                    type="checkbox"
                                    checked={draftProjectDefaults?.embed_chapters ?? settings?.embed_chapters_by_default ?? true}
                                    onChange={(e) => updateProjectDraft('embed_chapters', e.target.checked)}
                                />
                                <span className="text-sm">Embed chapters</span>
                            </label>
                            <label className="flex items-center gap-2">
                                <input
                                    type="checkbox"
                                    checked={draftProjectDefaults?.embed_artwork ?? settings?.embed_artwork_by_default ?? true}
                                    onChange={(e) => updateProjectDraft('embed_artwork', e.target.checked)}
                                />
                                <span className="text-sm">Embed artwork</span>
                            </label>
                            <label className="flex items-center gap-2">
                                <input
                                    type="checkbox"
                                    checked={draftProjectDefaults?.copy_metadata ?? settings?.copy_metadata_by_default ?? true}
                                    onChange={(e) => updateProjectDraft('copy_metadata', e.target.checked)}
                                />
                                <span className="text-sm">Copy metadata</span>
                            </label>
                            <label className="flex items-center gap-2">
                                <input
                                    type="checkbox"
                                    checked={draftProjectDefaults?.overwrite_existing ?? settings?.overwrite_existing_by_default ?? false}
                                    onChange={(e) => updateProjectDraft('overwrite_existing', e.target.checked)}
                                />
                                <span className="text-sm">Overwrite existing</span>
                            </label>
                        </div>
                    </div>

                    <div className="flex gap-3 pt-4 border-t">
                        <Button onClick={handleApplyProjectDefaults}>
                            Apply to Project
                        </Button>
                        <Button variant="outline" onClick={handleClearProjectDefaults}>
                            Use Global Defaults
                        </Button>
                    </div>
                </section>
            )}
        </div>
    );
}
