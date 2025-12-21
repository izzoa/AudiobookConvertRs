import { useState } from "react";
import { Button } from "./ui/button";
import { Label } from "./ui/label";
import { useApp } from "@/context/AppContext";
import * as FilesApi from "@/api/files";
import * as MetadataApi from "@/api/metadata";

export function ArtworkPicker() {
    const { activeAudiobook, setActiveAudiobook } = useApp();
    const [isDragging, setIsDragging] = useState(false);
    const [isProcessing, setIsProcessing] = useState(false);

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
            // Read file as ArrayBuffer and convert to base64
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

    // Display preview if artwork_data exists
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

            <div className="flex gap-2">
                <Button 
                    onClick={handleChooseImage} 
                    disabled={isProcessing}
                    variant="secondary"
                >
                    {isProcessing ? "Processing..." : "Choose Image..."}
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

