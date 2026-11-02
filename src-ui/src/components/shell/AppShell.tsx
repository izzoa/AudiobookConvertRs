import { TopBar } from "./TopBar";
import { Rail } from "./Rail";
import { AudioPlayer } from "@/components/AudioPlayer";
import { Toaster } from "@/components/ui/toaster";
import { cn } from "@/lib/utils";

/**
 * Fixed chrome, one scrolling region per pane.
 *
 * The old shell let the whole workflow scroll as a single document, so the
 * Convert action could be several screens away from the files it applied to.
 * Here nothing that acts on the book can scroll out of reach.
 */
export function AppShell({
    children,
    isDraggingOver,
}: {
    children: React.ReactNode;
    isDraggingOver: boolean;
}) {
    return (
        <div className="h-full flex flex-col overflow-hidden bg-background text-foreground">
            <TopBar />

            <div className="flex-1 flex overflow-hidden">
                <Rail />
                <div className="flex-1 flex flex-col overflow-hidden min-w-0">{children}</div>
            </div>

            <AudioPlayer />
            <Toaster />

            {/* Drop target feedback has to read over whichever pane is open. */}
            <div
                className={cn(
                    "pointer-events-none fixed inset-0 z-40 transition-opacity",
                    isDraggingOver ? "opacity-100" : "opacity-0"
                )}
            >
                <div className="absolute inset-2 rounded-xl border-2 border-dashed border-primary bg-primary/[0.04]" />
            </div>
        </div>
    );
}
