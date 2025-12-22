import { useState, useEffect } from "react";
import { Button } from "./ui/button";
import { Input } from "./ui/input";
import { Label } from "./ui/label";
import { useApp } from "@/context/AppContext";
import * as MetadataApi from "@/api/metadata";
import type { Metadata } from "@/types/models";
import { OpenLibraryFetcher } from "./OpenLibraryFetcher";
import { ArtworkPicker } from "./ArtworkPicker";

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
            const updatedAudiobook = await MetadataApi.updateAudiobookMetadata(activeAudiobook.id, metadata);
            // Update local context with returned audiobook
            setActiveAudiobook(updatedAudiobook);
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
        <div className="space-y-6">
            {/* Fetch from Open Library */}
            <OpenLibraryFetcher />

            {/* Cover Artwork */}
            <ArtworkPicker />

            {/* Manual Metadata Editing */}
            <div className="space-y-6 p-4 border rounded-lg bg-card">
                <Label className="text-base font-semibold">Manual Metadata</Label>

                {/* Core Section */}
                <div className="space-y-4">
                    <Label className="text-sm font-medium text-muted-foreground">Core Information</Label>
                    
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
                            <Label htmlFor="album_artist">Album Artist</Label>
                            <Input
                                id="album_artist"
                                placeholder="Album artist"
                                value={metadata.album_artist || ""}
                                onChange={(e) => handleChange("album_artist", e.target.value)}
                            />
                        </div>
                    </div>

                    <div className="grid grid-cols-2 gap-4">
                        <div className="grid gap-2">
                            <Label htmlFor="genre">Genre</Label>
                            <Input
                                id="genre"
                                placeholder="Genre"
                                value={metadata.genre || ""}
                                onChange={(e) => handleChange("genre", e.target.value)}
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

                    <div className="grid grid-cols-2 gap-4">
                        <div className="grid gap-2">
                            <Label htmlFor="language">Language</Label>
                            <Input
                                id="language"
                                placeholder="e.g. en, es, fr"
                                value={metadata.language || ""}
                                onChange={(e) => handleChange("language", e.target.value)}
                            />
                        </div>
                        <div className="grid gap-2">
                            <Label htmlFor="composer">Composer</Label>
                            <Input
                                id="composer"
                                placeholder="Composer"
                                value={metadata.composer || ""}
                                onChange={(e) => handleChange("composer", e.target.value)}
                            />
                        </div>
                    </div>
                </div>

                {/* Series Section */}
                <div className="space-y-4 pt-4 border-t">
                    <Label className="text-sm font-medium text-muted-foreground">Series</Label>
                    
                    <div className="grid grid-cols-3 gap-4">
                        <div className="grid gap-2 col-span-2">
                            <Label htmlFor="series">Series Name</Label>
                            <Input
                                id="series"
                                placeholder="Series name"
                                value={metadata.series || ""}
                                onChange={(e) => handleChange("series", e.target.value)}
                            />
                        </div>
                        <div className="grid gap-2">
                            <Label htmlFor="series_index">Book #</Label>
                            <Input
                                id="series_index"
                                placeholder="e.g. 1"
                                type="number"
                                value={metadata.series_index || ""}
                                onChange={(e) => setMetadata(prev => ({ ...prev, series_index: parseInt(e.target.value) || undefined }))}
                            />
                        </div>
                    </div>
                </div>

                {/* Publishing Section */}
                <div className="space-y-4 pt-4 border-t">
                    <Label className="text-sm font-medium text-muted-foreground">Publishing</Label>
                    
                    <div className="grid grid-cols-2 gap-4">
                        <div className="grid gap-2">
                            <Label htmlFor="publisher">Publisher</Label>
                            <Input
                                id="publisher"
                                placeholder="Publisher name"
                                value={metadata.publisher || ""}
                                onChange={(e) => handleChange("publisher", e.target.value)}
                            />
                        </div>
                        <div className="grid gap-2">
                            <Label htmlFor="copyright">Copyright</Label>
                            <Input
                                id="copyright"
                                placeholder="Copyright info"
                                value={metadata.copyright || ""}
                                onChange={(e) => handleChange("copyright", e.target.value)}
                            />
                        </div>
                    </div>
                </div>

                {/* Indexing Section */}
                <div className="space-y-4 pt-4 border-t">
                    <Label className="text-sm font-medium text-muted-foreground">Indexing</Label>
                    
                    <div className="grid grid-cols-4 gap-4">
                        <div className="grid gap-2">
                            <Label htmlFor="track_number">Track #</Label>
                            <Input
                                id="track_number"
                                placeholder="Track"
                                type="number"
                                value={metadata.track_number || ""}
                                onChange={(e) => setMetadata(prev => ({ ...prev, track_number: parseInt(e.target.value) || undefined }))}
                            />
                        </div>
                        <div className="grid gap-2">
                            <Label htmlFor="total_tracks">of</Label>
                            <Input
                                id="total_tracks"
                                placeholder="Total"
                                type="number"
                                value={metadata.total_tracks || ""}
                                onChange={(e) => setMetadata(prev => ({ ...prev, total_tracks: parseInt(e.target.value) || undefined }))}
                            />
                        </div>
                        <div className="grid gap-2">
                            <Label htmlFor="disc_number">Disc #</Label>
                            <Input
                                id="disc_number"
                                placeholder="Disc"
                                type="number"
                                value={metadata.disc_number || ""}
                                onChange={(e) => setMetadata(prev => ({ ...prev, disc_number: parseInt(e.target.value) || undefined }))}
                            />
                        </div>
                        <div className="grid gap-2">
                            <Label htmlFor="total_discs">of</Label>
                            <Input
                                id="total_discs"
                                placeholder="Total"
                                type="number"
                                value={metadata.total_discs || ""}
                                onChange={(e) => setMetadata(prev => ({ ...prev, total_discs: parseInt(e.target.value) || undefined }))}
                            />
                        </div>
                    </div>
                </div>

                {/* Notes Section */}
                <div className="space-y-4 pt-4 border-t">
                    <Label className="text-sm font-medium text-muted-foreground">Notes & Description</Label>
                    
                    <div className="grid gap-2">
                        <Label htmlFor="description">Description</Label>
                        <textarea
                            id="description"
                            placeholder="Book description or synopsis"
                            value={metadata.description || ""}
                            onChange={(e) => handleChange("description", e.target.value)}
                            className="flex min-h-[80px] w-full rounded-md border border-input bg-background px-3 py-2 text-sm ring-offset-background placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:cursor-not-allowed disabled:opacity-50"
                            rows={3}
                        />
                    </div>

                    <div className="grid gap-2">
                        <Label htmlFor="comments">Comments</Label>
                        <textarea
                            id="comments"
                            placeholder="Additional comments"
                            value={metadata.comments || ""}
                            onChange={(e) => handleChange("comments", e.target.value)}
                            className="flex min-h-[60px] w-full rounded-md border border-input bg-background px-3 py-2 text-sm ring-offset-background placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:cursor-not-allowed disabled:opacity-50"
                            rows={2}
                        />
                    </div>
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
        </div>
    );
}

