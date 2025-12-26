import { useState, useEffect, useCallback } from "react";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { Button } from "./ui/button";
import { Input } from "./ui/input";
import { Label } from "./ui/label";
import { Progress } from "./ui/progress";
import { useApp } from "@/context/AppContext";
import { toast } from "@/components/ui/use-toast";
import * as QualityApi from "@/api/quality";
import * as SettingsApi from "@/api/settings";
import { pickNormalizedOutputFile } from "@/api/files";
import type { AudioLevels, NormalizationSuggestion, NormalizationSettings, NormalizationProgress } from "@/types/models";

export function NormalizationPanel() {
    const { activeAudiobook } = useApp();
    const [isAnalyzing, setIsAnalyzing] = useState(false);
    const [isApplying, setIsApplying] = useState(false);
    const [error, setError] = useState<string | null>(null);
    const [levels, setLevels] = useState<AudioLevels | null>(null);
    const [suggestion, setSuggestion] = useState<NormalizationSuggestion | null>(null);
    const [progress, setProgress] = useState<NormalizationProgress | null>(null);
    const [settings, setSettings] = useState<NormalizationSettings>({
        target_lufs: -16,
        target_true_peak: -1.5,
        target_lra: 11,
        two_pass: true,
    });

    useEffect(() => {
        SettingsApi.getSettings().then(userSettings => {
            setSettings({
                target_lufs: userSettings.normalization_target_lufs,
                target_true_peak: userSettings.normalization_target_true_peak,
                target_lra: userSettings.normalization_target_lra,
                two_pass: userSettings.normalization_two_pass,
            });
        }).catch(console.error);
    }, []);

    useEffect(() => {
        const unlisteners: UnlistenFn[] = [];

        const setup = async () => {
            unlisteners.push(
                await listen<NormalizationProgress>("normalization_progress", (event) => {
                    setProgress(event.payload);
                })
            );
        };
        setup();

        return () => {
            unlisteners.forEach((fn) => fn());
        };
    }, []);

    const saveSettingsDebounced = useCallback(
        (() => {
            let timeout: ReturnType<typeof setTimeout>;
            return (newSettings: NormalizationSettings) => {
                clearTimeout(timeout);
                timeout = setTimeout(async () => {
                    try {
                        const currentSettings = await SettingsApi.getSettings();
                        await SettingsApi.updateSettings({
                            ...currentSettings,
                            normalization_target_lufs: newSettings.target_lufs,
                            normalization_target_true_peak: newSettings.target_true_peak,
                            normalization_target_lra: newSettings.target_lra,
                            normalization_two_pass: newSettings.two_pass,
                        });
                    } catch (e) {
                        console.error("Failed to save normalization settings:", e);
                    }
                }, 500);
            };
        })(),
        []
    );

    const updateSettings = (newSettings: NormalizationSettings) => {
        setSettings(newSettings);
        saveSettingsDebounced(newSettings);
    };

    const handleAnalyze = async () => {
        if (!activeAudiobook || activeAudiobook.files.length === 0) return;

        setIsAnalyzing(true);
        setError(null);

        try {
            const filePath = activeAudiobook.files[0].path;
            const [audioLevels, normSuggestion] = await Promise.all([
                QualityApi.analyzeAudioLevels(filePath),
                QualityApi.suggestNormalization(filePath),
            ]);

            setLevels(audioLevels);
            setSuggestion(normSuggestion);

            if (normSuggestion) {
                setSettings({
                    target_lufs: normSuggestion.target_lufs,
                    target_true_peak: normSuggestion.target_true_peak,
                    target_lra: normSuggestion.target_lra,
                    two_pass: true,
                });
            }
        } catch (e) {
            console.error("Failed to analyze audio:", e);
            setError(`Analysis failed: ${e}`);
        } finally {
            setIsAnalyzing(false);
        }
    };

    const handleApplyNormalization = async () => {
        if (!activeAudiobook || activeAudiobook.files.length === 0) return;

        setError(null);
        setProgress(null);

        try {
            const inputPath = activeAudiobook.files[0].path;
            const defaultName = inputPath.split('/').pop()?.replace(/(\.[^.]+)$/, "_normalized$1") || "normalized_output";

            const outputPath = await pickNormalizedOutputFile(defaultName);
            if (!outputPath) return;

            setIsApplying(true);

            const totalDurationMs = activeAudiobook.files[0].duration_ms;
            await QualityApi.applyAudioNormalizationWithProgress(inputPath, outputPath, settings, totalDurationMs);
            toast({
                title: "Normalization Complete",
                description: `Output saved to: ${outputPath}`,
            });
        } catch (e) {
            console.error("Failed to apply normalization:", e);
            toast({
                title: "Normalization Failed",
                description: `${e}`,
                variant: "destructive",
            });
            setError(`Normalization failed: ${e}`);
        } finally {
            setIsApplying(false);
            setProgress(null);
        }
    };

    if (!activeAudiobook || activeAudiobook.files.length === 0) {
        return null;
    }

    return (
        <div className="p-4 border rounded-lg bg-card mt-4 space-y-4">
            <div className="flex justify-between items-center">
                <h3 className="font-semibold">Audio Normalization</h3>
                {error && <span className="text-sm text-destructive">{error}</span>}
            </div>

            <Button onClick={handleAnalyze} disabled={isAnalyzing}>
                {isAnalyzing ? "Analyzing..." : "Analyze Audio"}
            </Button>

            {levels && (
                <div className="grid grid-cols-3 gap-4 p-4 border rounded-lg bg-muted/50">
                    <div className="text-center">
                        <div className="text-2xl font-mono font-semibold">
                            {levels.integrated_loudness_lufs.toFixed(1)}
                        </div>
                        <div className="text-sm text-muted-foreground">LUFS</div>
                        <div className="text-xs text-muted-foreground">Integrated Loudness</div>
                    </div>
                    <div className="text-center">
                        <div className="text-2xl font-mono font-semibold">
                            {levels.loudness_range_lu.toFixed(1)}
                        </div>
                        <div className="text-sm text-muted-foreground">LU</div>
                        <div className="text-xs text-muted-foreground">Loudness Range</div>
                    </div>
                    <div className="text-center">
                        <div className="text-2xl font-mono font-semibold">
                            {levels.true_peak_dbtp.toFixed(1)}
                        </div>
                        <div className="text-sm text-muted-foreground">dBTP</div>
                        <div className="text-xs text-muted-foreground">True Peak</div>
                    </div>
                </div>
            )}

            {suggestion && (
                <div className={`p-3 border rounded-lg ${suggestion.recommended ? "bg-yellow-500/10 border-yellow-500/50" : "bg-green-500/10 border-green-500/50"}`}>
                    <div className="flex items-center gap-2">
                        <span className={`text-sm font-medium ${suggestion.recommended ? "text-yellow-600 dark:text-yellow-400" : "text-green-600 dark:text-green-400"}`}>
                            {suggestion.recommended ? "Normalization Recommended" : "No Normalization Needed"}
                        </span>
                    </div>
                    <p className="text-sm text-muted-foreground mt-1">{suggestion.reason}</p>
                </div>
            )}

            {suggestion?.recommended && (
                <>
                    <div className="grid grid-cols-3 gap-4">
                        <div className="grid gap-2">
                            <Label htmlFor="target-lufs">Target LUFS</Label>
                            <Input
                                id="target-lufs"
                                type="number"
                                step="0.1"
                                value={settings.target_lufs}
                                onChange={(e) => updateSettings({
                                    ...settings,
                                    target_lufs: parseFloat(e.target.value) || -16
                                })}
                            />
                        </div>
                        <div className="grid gap-2">
                            <Label htmlFor="target-peak">Target True Peak (dBTP)</Label>
                            <Input
                                id="target-peak"
                                type="number"
                                step="0.1"
                                value={settings.target_true_peak}
                                onChange={(e) => updateSettings({
                                    ...settings,
                                    target_true_peak: parseFloat(e.target.value) || -1.5
                                })}
                            />
                        </div>
                        <div className="grid gap-2">
                            <Label htmlFor="target-lra">Target LRA (LU)</Label>
                            <Input
                                id="target-lra"
                                type="number"
                                step="0.1"
                                value={settings.target_lra}
                                onChange={(e) => updateSettings({
                                    ...settings,
                                    target_lra: parseFloat(e.target.value) || 11
                                })}
                            />
                        </div>
                    </div>

                    <Button onClick={handleApplyNormalization} disabled={isApplying}>
                        {isApplying ? "Applying..." : "Apply Normalization"}
                    </Button>

                    {isApplying && progress && (
                        <div className="space-y-2">
                            <div className="flex justify-between text-sm text-muted-foreground">
                                <span>{progress.stage}</span>
                                <span>{progress.percent.toFixed(0)}%</span>
                            </div>
                            <Progress value={progress.percent} />
                        </div>
                    )}
                </>
            )}
        </div>
    );
}
