import { useState, useEffect } from "react";
import { Button } from "./ui/button";
import { Input } from "./ui/input";
import { Label } from "./ui/label";
import * as PresetsApi from "@/api/presets";
import type { ConversionPreset, PresetSettings } from "@/types/models";

interface PresetManagerProps {
    onLoadPreset?: (settings: PresetSettings) => void;
    currentSettings?: PresetSettings;
}

export function PresetManager({ onLoadPreset, currentSettings }: PresetManagerProps) {
    const [presets, setPresets] = useState<ConversionPreset[]>([]);
    const [selectedPreset, setSelectedPreset] = useState<ConversionPreset | null>(null);
    const [newPresetName, setNewPresetName] = useState("");
    const [isLoading, setIsLoading] = useState(false);
    const [isSaving, setIsSaving] = useState(false);
    const [deleteConfirm, setDeleteConfirm] = useState<string | null>(null);

    useEffect(() => {
        loadPresets();
    }, []);

    const loadPresets = async () => {
        setIsLoading(true);
        try {
            const list = await PresetsApi.listPresets();
            setPresets(list);
        } catch (error) {
            console.error("Failed to load presets:", error);
        } finally {
            setIsLoading(false);
        }
    };

    const handleSelectPreset = (preset: ConversionPreset) => {
        setSelectedPreset(preset);
        setDeleteConfirm(null);
    };

    const handleLoadPreset = () => {
        if (selectedPreset && onLoadPreset) {
            onLoadPreset(selectedPreset.settings);
        }
    };

    const handleSavePreset = async () => {
        if (!newPresetName.trim() || !currentSettings) return;
        setIsSaving(true);
        try {
            const now = new Date().toISOString();
            const preset: ConversionPreset = {
                name: newPresetName.trim(),
                settings: currentSettings,
                created_at: now,
                modified_at: now,
            };
            await PresetsApi.savePreset(preset);
            setNewPresetName("");
            await loadPresets();
        } catch (error) {
            console.error("Failed to save preset:", error);
        } finally {
            setIsSaving(false);
        }
    };

    const handleDeletePreset = async (name: string) => {
        if (deleteConfirm !== name) {
            setDeleteConfirm(name);
            return;
        }
        try {
            await PresetsApi.deletePreset(name);
            if (selectedPreset?.name === name) {
                setSelectedPreset(null);
            }
            setDeleteConfirm(null);
            await loadPresets();
        } catch (error) {
            console.error("Failed to delete preset:", error);
        }
    };

    return (
        <div className="space-y-4 p-4 border rounded-lg bg-card">
            <h3 className="font-semibold">Presets</h3>

            {/* Preset List */}
            <div className="space-y-2">
                <Label>Saved Presets</Label>
                {isLoading ? (
                    <div className="text-sm text-muted-foreground">Loading...</div>
                ) : presets.length === 0 ? (
                    <div className="text-sm text-muted-foreground">No presets saved</div>
                ) : (
                    <div className="space-y-1 max-h-40 overflow-y-auto">
                        {presets.map((preset) => (
                            <div
                                key={preset.name}
                                className={`flex items-center justify-between p-2 rounded cursor-pointer ${
                                    selectedPreset?.name === preset.name
                                        ? "bg-accent"
                                        : "hover:bg-muted"
                                }`}
                                onClick={() => handleSelectPreset(preset)}
                            >
                                <span className="text-sm">{preset.name}</span>
                                <Button
                                    variant={deleteConfirm === preset.name ? "destructive" : "ghost"}
                                    size="sm"
                                    onClick={(e) => {
                                        e.stopPropagation();
                                        handleDeletePreset(preset.name);
                                    }}
                                >
                                    {deleteConfirm === preset.name ? "Confirm" : "Delete"}
                                </Button>
                            </div>
                        ))}
                    </div>
                )}
            </div>

            {/* Preset Details */}
            {selectedPreset && (
                <div className="space-y-2 p-3 rounded bg-muted/50">
                    <div className="text-sm font-medium">{selectedPreset.name}</div>
                    {selectedPreset.description && (
                        <div className="text-sm text-muted-foreground">{selectedPreset.description}</div>
                    )}
                    <div className="grid grid-cols-2 gap-2 text-sm">
                        <div>
                            <span className="text-muted-foreground">Bitrate:</span>{" "}
                            {selectedPreset.settings.bitrate_kbps} kbps
                        </div>
                        {selectedPreset.settings.sample_rate && (
                            <div>
                                <span className="text-muted-foreground">Sample Rate:</span>{" "}
                                {selectedPreset.settings.sample_rate} Hz
                            </div>
                        )}
                        <div>
                            <span className="text-muted-foreground">Chapters:</span>{" "}
                            {selectedPreset.settings.embed_chapters ? "Yes" : "No"}
                        </div>
                        <div>
                            <span className="text-muted-foreground">Artwork:</span>{" "}
                            {selectedPreset.settings.embed_artwork ? "Yes" : "No"}
                        </div>
                        <div>
                            <span className="text-muted-foreground">Normalize:</span>{" "}
                            {selectedPreset.settings.normalize ? "Yes" : "No"}
                        </div>
                        {selectedPreset.settings.normalize && selectedPreset.settings.target_lufs && (
                            <div>
                                <span className="text-muted-foreground">Target LUFS:</span>{" "}
                                {selectedPreset.settings.target_lufs}
                            </div>
                        )}
                    </div>
                    <Button size="sm" onClick={handleLoadPreset} disabled={!onLoadPreset}>
                        Load Preset
                    </Button>
                </div>
            )}

            {/* Save New Preset */}
            <div className="space-y-2 pt-2 border-t">
                <Label htmlFor="preset-name">Save Current Settings</Label>
                <div className="flex gap-2">
                    <Input
                        id="preset-name"
                        placeholder="Preset name"
                        value={newPresetName}
                        onChange={(e) => setNewPresetName(e.target.value)}
                    />
                    <Button
                        onClick={handleSavePreset}
                        disabled={!newPresetName.trim() || !currentSettings || isSaving}
                    >
                        {isSaving ? "Saving..." : "Save"}
                    </Button>
                </div>
            </div>
        </div>
    );
}
