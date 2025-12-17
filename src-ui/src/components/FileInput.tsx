import { DocumentArrowUpIcon } from '@heroicons/react/24/outline';
import { cn } from "@/lib/utils";

export function FileInput() {
    const handleDragOver = (e: React.DragEvent) => {
        e.preventDefault();
        // Add hover state
    };

    const handleDrop = (e: React.DragEvent) => {
        e.preventDefault();
        // Handle files
        console.log("Files dropped", e.dataTransfer.files);
    };

    return (
        <div
            className={cn(
                "border-2 border-dashed border-muted-foreground/25",
                "rounded-lg p-12 text-center",
                "hover:bg-accent/50 hover:border-primary/50",
                "transition-colors cursor-pointer",
                "flex flex-col items-center justify-center"
            )}
            onDragOver={handleDragOver}
            onDrop={handleDrop}
            onClick={() => document.getElementById('file-upload')?.click()}
        >
            <div className="bg-primary/10 p-4 rounded-full mb-4">
                <DocumentArrowUpIcon className="w-8 h-8 text-primary" />
            </div>
            <p className="text-lg font-semibold">Drop audiobooks here</p>
            <p className="text-sm text-muted-foreground">
                Supports MP3, FLAC, M4A, WAV, OGG
            </p>
            <p className="text-xs text-muted-foreground/75 mt-2">
                or click to browse
            </p>

            <input
                id="file-upload"
                type="file"
                multiple
                accept=".mp3,.flac,.m4a,.wav,.ogg"
                className="hidden"
                onChange={(e) => console.log(e.target.files)}
            />
        </div>
    );
}
