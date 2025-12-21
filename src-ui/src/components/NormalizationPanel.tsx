import { useState } from "react";
import { Button } from "./ui/button";
import { Input } from "./ui/input";
import { Label } from "./ui/label";
import { useApp } from "@/context/AppContext";
import * as QualityApi from "@/api/quality";
import type { AudioLevels, NormalizationSuggestion, NormalizationSettings } from "@/types/models";

export function NormalizationPanel() {
    const { activeAudiobook } = useApp();
    const [isAnalyzing, setIsAnalyzing] = useState(false);
    const [isApplying, setIsApplying] = useState(false);
    const [error, setError] = useState<string | null>(null);
    const [levels, setLevels] = useState<AudioLevels | null>(null);
    const [suggestion, setSuggestion] = useState<NormalizationSuggestion | null>(null);
    const [settings, setSettings] = useState<NormalizationSettings>({
        target_lufs: -16,
        target_true_peak: -1.5,
        target_lra: 7,
        two_pass: true,
    });

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

        setIsApplying(true);
        setError(null);

        try {
            const inputPath = activeAudiobook.files[0].path;
            const outputPath = inputPath.replace(/(\.[^.]+)$/, "_normalized$1");

            await QualityApi.applyAudioNormalization(inputPath, outputPath, settings);
        } catch (e) {
            console.error("Failed to apply normalization:", e);
            setError(`Normalization failed: ${e}`);
        } finally {
            setIsApplying(false);
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
                                onChange={(e) => setSettings(prev => ({
                                    ...prev,
                                    target_lufs: parseFloat(e.target.value) || -16
                                }))}
                            />
                        </div>
                        <div className="grid gap-2">
                            <Label htmlFor="target-peak">Target True Peak (dBTP)</Label>
                            <Input
                                id="target-peak"
                                type="number"
                                step="0.1"
                                value={settings.target_true_peak}
                                onChange={(e) => setSettings(prev => ({
                                    ...prev,
                                    target_true_peak: parseFloat(e.target.value) || -1.5
                                }))}
                            />
                        </div>
                        <div className="grid gap-2">
                            <Label htmlFor="target-lra">Target LRA (LU)</Label>
                            <Input
                                id="target-lra"
                                type="number"
                                step="0.1"
                                value={settings.target_lra}
                                onChange={(e) => setSettings(prev => ({
                                    ...prev,
                                    target_lra: parseFloat(e.target.value) || 7
                                }))}
                            />
                        </div>
                    </div>

                    <Button onClick={handleApplyNormalization} disabled={isApplying}>
                        {isApplying ? "Applying..." : "Apply Normalization"}
                    </Button>
                </>
            )}
        </div>
    );
}
