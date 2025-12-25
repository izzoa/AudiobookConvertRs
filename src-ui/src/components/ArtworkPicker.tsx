import { useState, useEffect } from "react";
import { Button } from "./ui/button";
import { Label } from "./ui/label";
import { useApp } from "@/context/AppContext";
import * as FilesApi from "@/api/files";
import * as MetadataApi from "@/api/metadata";
import type { DiscoveredArtwork, ArtworkSource } from "@/api/metadata";
import { Search, Image, FileImage } from "lucide-react";

function formatBytes(bytes: number): string {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

export function ArtworkPicker() {
    const { activeAudiobook, setActiveAudiobook } = useApp();
    const [isDragging, setIsDragging] = useState(false);
    const [isProcessing, setIsProcessing] = useState(false);
    const [isDiscovering, setIsDiscovering] = useState(false);
    const [discoveredArtwork, setDiscoveredArtwork] = useState<DiscoveredArtwork[]>([]);
    const [showDiscovered, setShowDiscovered] = useState(false);

    useEffect(() => {
        if (activeAudiobook?.files?.length) {
            handleDiscover();
        } else {
            setDiscoveredArtwork([]);
        }
    }, [activeAudiobook?.id, activeAudiobook?.files?.length]);

    const handleDiscover = async () => {
        if (!activeAudiobook) return;
        setIsDiscovering(true);
        try {
            const artwork = await MetadataApi.discoverArtwork(activeAudiobook.id);
            setDiscoveredArtwork(artwork);
            if (artwork.length > 0) {
                setShowDiscovered(true);
            }
        } catch (error) {
            console.error("Failed to discover artwork:", error);
        } finally {
            setIsDiscovering(false);
        }
    };

    const handleSelectDiscovered = async (source: ArtworkSource) => {
        if (!activeAudiobook) return;
        setIsProcessing(true);
        try {
            const updated = await MetadataApi.selectDiscoveredArtwork(activeAudiobook.id, source);
            setActiveAudiobook(updated);
            setShowDiscovered(false);
        } catch (error) {
            console.error("Failed to select artwork:", error);
        } finally {
            setIsProcessing(false);
        }
    };

    const handleChooseImage = async () => {
        if (!activeAudiobook) return;
        setIsProcessing(true);
        try {
            const filePath = await FilesApi.pickArtworkFile();
            if (filePath) {
                const updated = await MetadataApi.uploadArtwork(activeAudiobook.id, filePath);
                setActiveAudiobook(updated);
            }
        } catch (error) {
            console.error("Failed to upload artwork:", error);
        } finally {
            setIsProcessing(false);
        }
    };

    const handleDrop = async (e: React.DragEvent) => {
        e.preventDefault();
        setIsDragging(false);

        if (!activeAudiobook) return;

        const files = Array.from(e.dataTransfer.files);
        const imageFile = files.find(f => f.type.startsWith('image/'));

        if (!imageFile) {
            console.error("No image file found in dropped items");
            return;
        }

        setIsProcessing(true);
        try {
            const arrayBuffer = await imageFile.arrayBuffer();
            const bytes = new Uint8Array(arrayBuffer);
            const base64 = btoa(String.fromCharCode(...bytes));

            const updated = await MetadataApi.setArtworkBase64(activeAudiobook.id, base64);
            setActiveAudiobook(updated);
        } catch (error) {
            console.error("Failed to set artwork from drop:", error);
        } finally {
            setIsProcessing(false);
        }
    };

    const handleRemove = async () => {
        if (!activeAudiobook) return;
        setIsProcessing(true);
        try {
            const updated = await MetadataApi.clearArtwork(activeAudiobook.id);
            setActiveAudiobook(updated);
        } catch (error) {
            console.error("Failed to clear artwork:", error);
        } finally {
            setIsProcessing(false);
        }
    };

    const handleDragOver = (e: React.DragEvent) => {
        e.preventDefault();
        setIsDragging(true);
    };

    const handleDragLeave = () => {
        setIsDragging(false);
    };

    if (!activeAudiobook) return null;

    const artworkPreview = activeAudiobook.output_metadata.artwork_data
        ? `data:image/jpeg;base64,${activeAudiobook.output_metadata.artwork_data}`
        : null;

    return (
        <div className="space-y-4 p-4 border rounded-lg bg-card">
            <Label className="text-base font-semibold">Cover Artwork</Label>

            <div
                className={`
                    border-2 border-dashed rounded-lg p-6
                    ${isDragging ? 'border-primary bg-accent/50' : 'border-muted-foreground/25'}
                    transition-colors
                `}
                onDrop={handleDrop}
                onDragOver={handleDragOver}
                onDragLeave={handleDragLeave}
            >
                {artworkPreview ? (
                    <div className="flex flex-col items-center gap-4">
                        <img
                            src={artworkPreview}
                            alt="Cover artwork"
                            className="max-w-xs max-h-64 rounded shadow-lg"
                        />
                        <p className="text-sm text-muted-foreground">
                            Drag a new image here to replace, or use the buttons below
                        </p>
                    </div>
                ) : (
                    <div className="text-center py-8">
                        <p className="text-sm text-muted-foreground">
                            No cover artwork. Drag an image here or click Choose Image.
                        </p>
                    </div>
                )}
            </div>

            {discoveredArtwork.length > 0 && showDiscovered && (
                <div className="border rounded-lg p-3 bg-blue-500/5 border-blue-500/30">
                    <div className="flex items-center justify-between mb-3">
                        <div className="flex items-center gap-2">
                            <Search className="h-4 w-4 text-blue-500" />
                            <span className="text-sm font-medium text-blue-600 dark:text-blue-400">
                                {discoveredArtwork.length} artwork option(s) found
                            </span>
                        </div>
                        <Button
                            variant="ghost"
                            size="sm"
                            onClick={() => setShowDiscovered(false)}
                            className="h-6 text-xs"
                        >
                            Hide
                        </Button>
                    </div>
                    <div className="grid gap-2 max-h-48 overflow-y-auto">
                        {discoveredArtwork.map((art, idx) => (
                            <button
                                key={idx}
                                onClick={() => handleSelectDiscovered(art.source)}
                                disabled={isProcessing}
                                className="flex items-center gap-3 p-2 rounded border hover:bg-accent text-left w-full transition-colors disabled:opacity-50"
                            >
                                {'embedded' in art.source ? (
                                    <Image className="h-5 w-5 text-muted-foreground flex-shrink-0" />
                                ) : (
                                    <FileImage className="h-5 w-5 text-muted-foreground flex-shrink-0" />
                                )}
                                <div className="flex-1 min-w-0">
                                    <div className="text-sm font-medium truncate">{art.name}</div>
                                    <div className="text-xs text-muted-foreground">
                                        {art.width && art.height && `${art.width}×${art.height} • `}
                                        {formatBytes(art.size_bytes)}
                                    </div>
                                </div>
                                {art.priority === 0 && (
                                    <span className="text-xs bg-green-500/20 text-green-600 dark:text-green-400 px-2 py-0.5 rounded">
                                        Embedded
                                    </span>
                                )}
                                {art.priority === 1 && (
                                    <span className="text-xs bg-blue-500/20 text-blue-600 dark:text-blue-400 px-2 py-0.5 rounded">
                                        Recommended
                                    </span>
                                )}
                            </button>
                        ))}
                    </div>
                </div>
            )}

            <div className="flex gap-2 flex-wrap">
                <Button
                    onClick={handleChooseImage}
                    disabled={isProcessing}
                    variant="secondary"
                >
                    {isProcessing ? "Processing..." : "Choose Image..."}
                </Button>
                {discoveredArtwork.length > 0 && !showDiscovered && (
                    <Button
                        onClick={() => setShowDiscovered(true)}
                        variant="outline"
                        disabled={isProcessing}
                    >
                        <Search className="h-4 w-4 mr-2" />
                        Show Discovered ({discoveredArtwork.length})
                    </Button>
                )}
                <Button
                    onClick={handleDiscover}
                    variant="outline"
                    disabled={isDiscovering || isProcessing}
                >
                    <Search className="h-4 w-4 mr-2" />
                    {isDiscovering ? "Scanning..." : "Scan for Artwork"}
                </Button>
                {artworkPreview && (
                    <Button
                        onClick={handleRemove}
                        disabled={isProcessing}
                        variant="outline"
                    >
                        Remove
                    </Button>
                )}
            </div>
        </div>
    );
}
