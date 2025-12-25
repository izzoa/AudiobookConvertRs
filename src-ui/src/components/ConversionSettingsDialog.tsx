import React, { useState, useEffect } from 'react';
import { Button } from './ui/button';
import { Input } from './ui/input';
import { Label } from './ui/label';
import type { AudioBook, ConversionSettings } from '../types/models';
import { pickOutputFile } from '../api/files';
import { useApp } from '@/context/AppContext';

interface ConversionSettingsDialogProps {
    audiobook: AudioBook;
    onConfirm: (settings: ConversionSettings) => void;
    onCancel: () => void;
}

export const ConversionSettingsDialog: React.FC<ConversionSettingsDialogProps> = ({
    audiobook,
    onConfirm,
    onCancel,
}) => {
    const { getEffectiveConversionDefaults, settings } = useApp();
    const defaults = getEffectiveConversionDefaults();

    const [outputPath, setOutputPath] = useState('');
    const [bitrate, setBitrate] = useState(defaults.bitrate_kbps);
    const [matchBitrate, setMatchBitrate] = useState(false);
    const [sampleRate, setSampleRate] = useState<number | undefined>(defaults.sample_rate_hz);

    const sourceBitrate = React.useMemo(() => {
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

    // Initialize output path with default directory if set
    useEffect(() => {
        if (settings?.default_output_directory && audiobook.title) {
            const defaultPath = `${settings.default_output_directory}/${audiobook.title}.m4b`;
            setOutputPath(defaultPath);
        }
    }, [settings?.default_output_directory, audiobook.title]);

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
            <div className="bg-background border rounded-lg p-6 w-full max-w-md">
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
                        <Label htmlFor="bitrate">Bitrate (kbps)</Label>
                        <select
                            id="bitrate"
                            value={matchBitrate ? 'match' : bitrate}
                            onChange={(e) => {
                                if (e.target.value === 'match') {
                                    setMatchBitrate(true);
                                } else {
                                    setMatchBitrate(false);
                                    setBitrate(parseInt(e.target.value));
                                }
                            }}
                            className="w-full px-3 py-2 border rounded-md bg-background"
                        >
                            {sourceBitrate && (
                                <option value="match">Match source (~{sourceBitrate} kbps)</option>
                            )}
                            <option value="32">32 kbps</option>
                            <option value="48">48 kbps</option>
                            <option value="64">64 kbps</option>
                            <option value="96">96 kbps</option>
                            <option value="128">128 kbps</option>
                            <option value="192">192 kbps</option>
                            <option value="256">256 kbps</option>
                            <option value="320">320 kbps</option>
                        </select>
                    </div>

                    <div className="space-y-2">
                        <Label htmlFor="sample-rate">Sample Rate</Label>
                        <select
                            id="sample-rate"
                            value={sampleRate ?? 'match'}
                            onChange={(e) => setSampleRate(e.target.value === 'match' ? undefined : parseInt(e.target.value))}
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
