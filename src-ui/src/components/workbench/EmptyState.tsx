import { useState } from "react";
import { ArrowDownTrayIcon, FolderOpenIcon, MusicalNoteIcon } from "@heroicons/react/24/outline";
import { Button } from "@/components/ui/button";
import { useToast } from "@/components/ui/use-toast";
import { useApp } from "@/context/AppContext";
import * as FilesApi from "@/api/files";
import * as ProjectApi from "@/api/project";
import { cn } from "@/lib/utils";
import { SUPPORTED_FORMAT_LABELS } from "@/lib/formats";

/**
 * First screen. It states the one thing to do and accepts a drop anywhere,
 * rather than opening on an empty five-section form.
 */
export function EmptyState({ isDraggingOver, onImport }: { isDraggingOver: boolean; onImport: (paths: string[]) => void }) {
    const { setActiveAudiobook, setActiveProject } = useApp();
    const { toast } = useToast();
    const [busy, setBusy] = useState(false);

    const chooseFiles = async () => {
        setBusy(true);
        try {
            const picked = await FilesApi.pickFiles();
            if (picked?.length) onImport(picked);
        } catch (error) {
            toast({ title: "Could not open file picker", description: String(error), variant: "destructive" });
        } finally {
            setBusy(false);
        }
    };

    const chooseFolder = async () => {
        setBusy(true);
        try {
            const dir = await FilesApi.pickDirectory();
            if (dir) onImport([dir]);
        } catch (error) {
            toast({ title: "Could not open folder picker", description: String(error), variant: "destructive" });
        } finally {
            setBusy(false);
        }
    };

    const openProject = async () => {
        setBusy(true);
        try {
            const path = await FilesApi.pickOpenProjectFile();
            if (!path) return;
            const project = await ProjectApi.loadProject(path);
            setActiveAudiobook(project.audiobook);
            setActiveProject(project);
        } catch (error) {
            toast({ title: "Could not open project", description: String(error), variant: "destructive" });
        } finally {
            setBusy(false);
        }
    };

    return (
        <div className="h-full flex items-center justify-center p-8">
            <div
                className={cn(
                    "w-full max-w-[560px] rounded-2xl border-2 border-dashed px-10 py-14 text-center transition-colors",
                    isDraggingOver
                        ? "border-primary bg-primary/[0.06]"
                        : "border-border bg-surface/60"
                )}
            >
                <div
                    className={cn(
                        "w-16 h-16 mx-auto rounded-2xl flex items-center justify-center transition-colors",
                        isDraggingOver ? "bg-primary text-primary-foreground" : "bg-primary/10 text-primary"
                    )}
                >
                    {isDraggingOver ? (
                        <ArrowDownTrayIcon className="w-8 h-8" />
                    ) : (
                        <MusicalNoteIcon className="w-8 h-8" />
                    )}
                </div>

                <h1 className="mt-6 text-[22px] font-semibold tracking-tight">
                    {isDraggingOver ? "Drop to import" : "Drop your audio files here"}
                </h1>
                <p className="mt-1.5 text-sm text-muted-foreground">
                    Files or a whole folder. They become one M4B audiobook.
                </p>

                <div className="mt-7 flex items-center justify-center gap-2">
                    <Button onClick={chooseFiles} disabled={busy}>
                        Choose files…
                    </Button>
                    <Button variant="outline" onClick={chooseFolder} disabled={busy}>
                        <FolderOpenIcon className="w-4 h-4 mr-2" />
                        Choose folder…
                    </Button>
                </div>

                <button
                    onClick={openProject}
                    disabled={busy}
                    className="mt-4 text-sm text-muted-foreground hover:text-foreground underline underline-offset-4 disabled:opacity-50"
                >
                    or open a saved project
                </button>

                <div className="mt-9 pt-6 border-t flex flex-wrap items-center justify-center gap-1.5">
                    {SUPPORTED_FORMAT_LABELS.map((format) => (
                        <span
                            key={format}
                            className="px-2 py-0.5 rounded text-[11px] font-medium text-muted-foreground bg-muted"
                        >
                            {format}
                        </span>
                    ))}
                </div>
            </div>
        </div>
    );
}
