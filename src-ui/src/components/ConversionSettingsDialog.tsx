import React, { useState, useEffect, useMemo } from 'react';
import { Button } from './ui/button';
import { Input } from './ui/input';
import { Label } from './ui/label';
import { Slider } from './ui/slider';
import type { AudioBook, ConversionSettings, QualityPreset, EncodingMode, FileSizeEstimate } from '../types/models';
import { QualityPreset as QualityPresetValue, EncodingMode as EncodingModeValue } from '../types/models';
import { pickOutputFile } from '../api/files';
import { estimateOutputSize } from '../api/quality';
import { useApp } from '@/context/AppContext';

interface ConversionSettingsDialogProps {
    audiobook: AudioBook;
    onConfirm: (settings: ConversionSettings) => void;
    onCancel: () => void;
}

const PRESET_CONFIG: Record<QualityPreset, { bitrate: number; sampleRate: number; isVbr: boolean; description: string }> = {
    high: { bitrate: 256, sampleRate: 44100, isVbr: true, description: 'Best quality, larger file' },
    medium: { bitrate: 128, sampleRate: 44100, isVbr: true, description: 'Balanced quality and size' },
    low: { bitrate: 64, sampleRate: 22050, isVbr: false, description: 'Smaller file, lower quality' },
    custom: { bitrate: 128, sampleRate: 44100, isVbr: true, description: 'Custom settings' },
};


export const ConversionSettingsDialog: React.FC<ConversionSettingsDialogProps> = ({
    audiobook,
    onConfirm,
    onCancel,
}) => {
    const { getEffectiveConversionDefaults, settings } = useApp();
    const defaults = getEffectiveConversionDefaults();

    const [outputPath, setOutputPath] = useState('');
    const [qualityPreset, setQualityPreset] = useState<QualityPreset>(settings?.default_quality_preset ?? QualityPresetValue.Medium as QualityPreset);
    const [encodingMode, setEncodingMode] = useState<EncodingMode>(settings?.default_encoding_mode ?? EncodingModeValue.Vbr as EncodingMode);
    const [bitrate, setBitrate] = useState(defaults.bitrate_kbps);
    const [matchBitrate, setMatchBitrate] = useState(false);
    const [sampleRate, setSampleRate] = useState<number | undefined>(defaults.sample_rate_hz);
    const [estimatedSize, setEstimatedSize] = useState<FileSizeEstimate | null>(null);

    const sourceBitrate = useMemo(() => {
        const bitrates = audiobook.files
            .map(f => f.bitrate_kbps)
            .filter((b): b is number => b !== undefined && b > 0);
        if (bitrates.length === 0) return undefined;
        const avg = Math.round(bitrates.reduce((a, b) => a + b, 0) / bitrates.length);
        return avg;
    }, [audiobook.files]);

    const [embedChapters, setEmbedChapters] = useState(defaults.embed_chapters);
    const [embedArtwork, setEmbedArtwork] = useState(defaults.embed_artwork);
    const [copyMetadata, setCopyMetadata] = useState(defaults.copy_metadata);
    const [overwrite, setOverwrite] = useState(defaults.overwrite_existing);

    useEffect(() => {
        if (settings?.default_output_directory && audiobook.title) {
            const defaultPath = `${settings.default_output_directory}/${audiobook.title}.m4b`;
            setOutputPath(defaultPath);
        }
    }, [settings?.default_output_directory, audiobook.title]);

    useEffect(() => {
        if (qualityPreset !== QualityPresetValue.Custom) {
            const config = PRESET_CONFIG[qualityPreset];
            setBitrate(config.bitrate);
            setSampleRate(config.sampleRate);
            setEncodingMode(config.isVbr ? EncodingModeValue.Vbr as EncodingMode : EncodingModeValue.Cbr as EncodingMode);
            setMatchBitrate(false);
        }
    }, [qualityPreset]);

    useEffect(() => {
        const effectiveBitrate = matchBitrate && sourceBitrate ? sourceBitrate : bitrate;
        estimateOutputSize(audiobook.total_duration_ms, effectiveBitrate)
            .then(setEstimatedSize)
            .catch(console.error);
    }, [audiobook.total_duration_ms, bitrate, matchBitrate, sourceBitrate]);

    const handlePresetChange = (preset: QualityPreset) => {
        setQualityPreset(preset);
    };

    const handleBitrateChange = (value: number) => {
        setBitrate(value);
        setQualityPreset(QualityPresetValue.Custom as QualityPreset);
        setMatchBitrate(false);
    };

    const handleEncodingModeChange = (mode: EncodingMode) => {
        setEncodingMode(mode);
        if (qualityPreset !== QualityPresetValue.Custom) {
            setQualityPreset(QualityPresetValue.Custom as QualityPreset);
        }
    };

    const handlePickOutputFile = async () => {
        try {
            const path = await pickOutputFile(audiobook.title);
            if (path) {
                setOutputPath(path);
            }
        } catch (error) {
            console.error('Failed to pick output file:', error);
        }
    };

    const handleConfirm = () => {
        if (!outputPath) {
            alert('Please select an output path');
            return;
        }

        const effectiveBitrate = matchBitrate && sourceBitrate ? sourceBitrate : bitrate;
        const conversionSettings: ConversionSettings = {
            output_path: outputPath,
            bitrate_kbps: effectiveBitrate,
            sample_rate_hz: sampleRate,
            encoding_mode: encodingMode,
            output_format: 'm4b',
            concurrent_jobs: settings?.concurrent_jobs ?? 1,
            overwrite_existing: overwrite,
            embed_chapters: embedChapters,
            embed_artwork: embedArtwork,
            copy_metadata: copyMetadata,
        };

        onConfirm(conversionSettings);
    };

    return (
        <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
            <div className="bg-background border rounded-lg p-6 w-full max-w-lg max-h-[90vh] overflow-y-auto">
                <div className="mb-4">
                    <h2 className="text-lg font-semibold">Conversion Settings</h2>
                    <p className="text-sm text-muted-foreground">
                        Configure settings for converting "{audiobook.title}"
                    </p>
                </div>

                <div className="space-y-4">
                    <div className="space-y-2">
                        <Label htmlFor="output">Output File</Label>
                        <div className="flex gap-2">
                            <Input
                                id="output"
                                value={outputPath}
                                onChange={(e) => setOutputPath(e.target.value)}
                                placeholder="Select output file..."
                                className="flex-1"
                            />
                            <Button onClick={handlePickOutputFile} size="sm">
                                Browse
                            </Button>
                        </div>
                    </div>

                    <div className="space-y-2">
                        <Label>Quality Preset</Label>
                        <div className="grid grid-cols-4 gap-2">
                            {(['high', 'medium', 'low', 'custom'] as QualityPreset[]).map((preset) => (
                                <button
                                    key={preset}
                                    onClick={() => handlePresetChange(preset)}
                                    className={`px-3 py-2 rounded-md text-sm font-medium transition-colors ${
                                        qualityPreset === preset
                                            ? 'bg-primary text-primary-foreground'
                                            : 'bg-secondary hover:bg-secondary/80'
                                    }`}
                                >
                                    {preset.charAt(0).toUpperCase() + preset.slice(1)}
                                </button>
                            ))}
                        </div>
                        <p className="text-xs text-muted-foreground">
                            {PRESET_CONFIG[qualityPreset].description}
                        </p>
                    </div>

                    <div className="space-y-2">
                        <Label>Encoding Mode</Label>
                        <div className="flex gap-2">
                            <button
                                onClick={() => handleEncodingModeChange(EncodingModeValue.Vbr as EncodingMode)}
                                className={`flex-1 px-3 py-2 rounded-md text-sm font-medium transition-colors ${
                                    encodingMode === EncodingModeValue.Vbr
                                        ? 'bg-primary text-primary-foreground'
                                        : 'bg-secondary hover:bg-secondary/80'
                                }`}
                            >
                                VBR (Variable)
                            </button>
                            <button
                                onClick={() => handleEncodingModeChange(EncodingModeValue.Cbr as EncodingMode)}
                                className={`flex-1 px-3 py-2 rounded-md text-sm font-medium transition-colors ${
                                    encodingMode === EncodingModeValue.Cbr
                                        ? 'bg-primary text-primary-foreground'
                                        : 'bg-secondary hover:bg-secondary/80'
                                }`}
                            >
                                CBR (Constant)
                            </button>
                        </div>
                        <p className="text-xs text-muted-foreground">
                            {encodingMode === EncodingModeValue.Vbr
                                ? 'Variable bitrate: Better quality-to-size ratio'
                                : 'Constant bitrate: More predictable file size'}
                        </p>
                    </div>

                    <div className="space-y-2">
                        <div className="flex justify-between items-center">
                            <Label>Bitrate: {matchBitrate && sourceBitrate ? `~${sourceBitrate}` : bitrate} kbps</Label>
                            {estimatedSize && (
                                <span className="text-sm text-muted-foreground">
                                    Est. size: {estimatedSize.formatted}
                                </span>
                            )}
                        </div>
                        <Slider
                            value={[matchBitrate && sourceBitrate ? sourceBitrate : bitrate]}
                            onValueChange={([value]) => handleBitrateChange(value)}
                            min={32}
                            max={320}
                            step={8}
                            disabled={matchBitrate}
                        />
                        <div className="flex justify-between text-xs text-muted-foreground">
                            <span>32 kbps</span>
                            <span>320 kbps</span>
                        </div>
                        {sourceBitrate && (
                            <label className="flex items-center gap-2 text-sm">
                                <input
                                    type="checkbox"
                                    checked={matchBitrate}
                                    onChange={(e) => {
                                        setMatchBitrate(e.target.checked);
                                        if (e.target.checked) {
                                            setQualityPreset(QualityPresetValue.Custom as QualityPreset);
                                        }
                                    }}
                                />
                                Match source bitrate (~{sourceBitrate} kbps)
                            </label>
                        )}
                    </div>

                    <div className="space-y-2">
                        <Label htmlFor="sample-rate">Sample Rate</Label>
                        <select
                            id="sample-rate"
                            value={sampleRate ?? 'match'}
                            onChange={(e) => {
                                setSampleRate(e.target.value === 'match' ? undefined : parseInt(e.target.value));
                                if (qualityPreset !== QualityPresetValue.Custom) {
                                    setQualityPreset(QualityPresetValue.Custom as QualityPreset);
                                }
                            }}
                            className="w-full px-3 py-2 border rounded-md bg-background"
                        >
                            <option value="match">Match source</option>
                            <option value="22050">22050 Hz</option>
                            <option value="44100">44100 Hz</option>
                            <option value="48000">48000 Hz</option>
                        </select>
                    </div>

                    <div className="space-y-2">
                        <Label>Options</Label>
                        <div className="space-y-2">
                            <label className="flex items-center gap-2">
                                <input
                                    type="checkbox"
                                    checked={embedChapters}
                                    onChange={(e) => setEmbedChapters(e.target.checked)}
                                />
                                <span className="text-sm">Embed chapters</span>
                            </label>
                            <label className="flex items-center gap-2">
                                <input
                                    type="checkbox"
                                    checked={embedArtwork}
                                    onChange={(e) => setEmbedArtwork(e.target.checked)}
                                />
                                <span className="text-sm">Embed artwork</span>
                            </label>
                            <label className="flex items-center gap-2">
                                <input
                                    type="checkbox"
                                    checked={copyMetadata}
                                    onChange={(e) => setCopyMetadata(e.target.checked)}
                                />
                                <span className="text-sm">Copy metadata</span>
                            </label>
                            <label className="flex items-center gap-2">
                                <input
                                    type="checkbox"
                                    checked={overwrite}
                                    onChange={(e) => setOverwrite(e.target.checked)}
                                />
                                <span className="text-sm">Overwrite existing file</span>
                            </label>
                        </div>
                    </div>
                </div>

                <div className="flex justify-end gap-2 mt-6">
                    <Button variant="outline" onClick={onCancel}>
                        Cancel
                    </Button>
                    <Button onClick={handleConfirm}>Confirm</Button>
                </div>
            </div>
        </div>
    );
};
