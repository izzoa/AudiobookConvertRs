import { useState, useEffect } from "react";
import { Button } from "./ui/button";
import { Input } from "./ui/input";
import { Label } from "./ui/label";
import { useApp } from "@/context/AppContext";
import * as MetadataApi from "@/api/metadata";
import type { Metadata } from "@/types/models";

export function MetadataEditor() {
    const { activeAudiobook, setActiveAudiobook } = useApp();
    const [metadata, setMetadata] = useState<Metadata>({});
    const [isSaving, setIsSaving] = useState(false);

    useEffect(() => {
        if (activeAudiobook?.output_metadata) {
            setMetadata(activeAudiobook.output_metadata);
        }
    }, [activeAudiobook?.id]); // Only reset when book changes

    const handleChange = (field: keyof Metadata, value: string) => {
        setMetadata(prev => ({ ...prev, [field]: value }));
    };

    const handleSave = async () => {
        if (!activeAudiobook) return;
        setIsSaving(true);
        try {
            await MetadataApi.updateAudiobookMetadata(activeAudiobook.id, metadata);

            // Update local context
            setActiveAudiobook({
                ...activeAudiobook,
                output_metadata: metadata,
                title: metadata.title || activeAudiobook.title,
                author: metadata.artist || activeAudiobook.author,
            });
        } catch (error) {
            console.error("Failed to save metadata:", error);
        } finally {
            setIsSaving(false);
        }
    };

    if (!activeAudiobook) {
        return (
            <div className="p-8 text-center border rounded-lg bg-muted/10 text-muted-foreground">
                No audiobook selected. Load files to edit metadata.
            </div>
        );
    }

    return (
        <div className="space-y-4 p-4 border rounded-lg bg-card">
            <div className="grid gap-2">
                <Label htmlFor="title">Title</Label>
                <Input
                    id="title"
                    placeholder="Enter audiobook title"
                    value={metadata.title || ""}
                    onChange={(e) => handleChange("title", e.target.value)}
                />
            </div>

            <div className="grid grid-cols-2 gap-4">
                <div className="grid gap-2">
                    <Label htmlFor="artist">Author (Artist)</Label>
                    <Input
                        id="artist"
                        placeholder="Author name"
                        value={metadata.artist || ""}
                        onChange={(e) => handleChange("artist", e.target.value)}
                    />
                </div>
                <div className="grid gap-2">
                    <Label htmlFor="narrator">Narrator</Label>
                    <Input
                        id="narrator"
                        placeholder="Narrator name"
                        value={metadata.narrator || ""}
                        onChange={(e) => handleChange("narrator", e.target.value)}
                    />
                </div>
            </div>

            <div className="grid grid-cols-2 gap-4">
                <div className="grid gap-2">
                    <Label htmlFor="album">Album</Label>
                    <Input
                        id="album"
                        placeholder="Album"
                        value={metadata.album || ""}
                        onChange={(e) => handleChange("album", e.target.value)}
                    />
                </div>
                <div className="grid gap-2">
                    <Label htmlFor="year">Year</Label>
                    <Input
                        id="year"
                        placeholder="Year"
                        type="number"
                        value={metadata.year || ""}
                        onChange={(e) => setMetadata(prev => ({ ...prev, year: parseInt(e.target.value) || undefined }))}
                    />
                </div>
            </div>

            <div className="grid gap-2">
                <Label htmlFor="genre">Genre</Label>
                <Input
                    id="genre"
                    placeholder="Genre"
                    value={metadata.genre || ""}
                    onChange={(e) => handleChange("genre", e.target.value)}
                />
            </div>

            <div className="pt-2">
                <Button
                    className="w-full sm:w-auto"
                    onClick={handleSave}
                    disabled={isSaving}
                >
                    {isSaving ? "Saving..." : "Save Metadata"}
                </Button>
            </div>
        </div>
    );
}

