import { DocumentArrowUpIcon } from '@heroicons/react/24/outline';
import { cn } from "@/lib/utils";
import { useApp } from "@/context/AppContext";
import * as FilesApi from "@/api/files";
import * as AudiobookApi from "@/api/audiobook";
import { useState } from 'react';

export function FileInput() {
    const { setActiveAudiobook } = useApp();
    const [isProcessing, setIsProcessing] = useState(false);

    const handleFiles = async (files: string[]) => {
        if (files.length === 0) return;
        setIsProcessing(true);
        try {
            console.log("Creating audiobook from files:", files);
            const audiobook = await AudiobookApi.createAudiobookFromFiles(files);
            console.log("Audiobook created:", audiobook);
            setActiveAudiobook(audiobook);
        } catch (error) {
            console.error("Failed to create audiobook:", error);
            // TODO: Error handling
        } finally {
            setIsProcessing(false);
        }
    };

    const handleClick = async () => {
        if (isProcessing) return;
        try {
            const files = await FilesApi.pickFiles();
            if (files && files.length > 0) {
                await handleFiles(files);
            }
        } catch (error) {
            console.error("Failed to pick files:", error);
        }
    };

    const handleDragOver = (e: React.DragEvent) => {
        e.preventDefault();
        // Add hover state visual if needed
    };

    const handleDrop = (e: React.DragEvent) => {
        e.preventDefault();
        // NOTE: Web drag-drop file paths are not easily accessible in Tauri
        // without enabling specific permissions or using 'tauri://file-drop'.
        // For now, we rely on the Click handler and system dialog.
        console.warn("Drag and drop not fully implemented yet - use click to browse");
    };

    return (
        <div
            className={cn(
                "border-2 border-dashed border-muted-foreground/25",
                "rounded-lg p-12 text-center",
                "hover:bg-accent/50 hover:border-primary/50",
                "transition-colors cursor-pointer",
                "flex flex-col items-center justify-center",
                isProcessing && "opacity-50 cursor-wait"
            )}
            onDragOver={handleDragOver}
            onDrop={handleDrop}
            onClick={handleClick}
        >
            <div className="bg-primary/10 p-4 rounded-full mb-4">
                <DocumentArrowUpIcon className={cn("w-8 h-8 text-primary", isProcessing && "animate-pulse")} />
            </div>
            <p className="text-lg font-semibold">
                {isProcessing ? "Processing..." : "Drop audiobooks here"}
            </p>
            <p className="text-sm text-muted-foreground">
                Supports MP3, FLAC, M4A, WAV, OGG
            </p>
            <p className="text-xs text-muted-foreground/75 mt-2">
                or click to browse
            </p>
        </div>
    );
}

