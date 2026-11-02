import { useCallback } from "react";
import { useToast } from "@/components/ui/use-toast";
import { useApp } from "@/context/useApp";
import { useBatch } from "@/context/useBatch";
import { useRebuildBook } from "@/hooks/useRebuildBook";
import * as FilesApi from "@/api/files";
import * as ProjectApi from "@/api/project";
import * as AudiobookApi from "@/api/audiobook";

/**
 * Book-level commands, shared by the title menu and the keyboard shortcuts so
 * both routes do exactly the same thing.
 */
export function useProjectActions() {
    const {
        activeAudiobook,
        setActiveAudiobook,
        setActiveProject,
        getEffectiveConversionDefaults,
    } = useApp();
    const { toast } = useToast();
    const { importPaths } = useBatch();
    const rebuildBook = useRebuildBook();

    const importFiles = useCallback(
        async (append = false) => {
            try {
                const picked = await FilesApi.pickFiles();
                if (!picked?.length) return;
                if (append && activeAudiobook) {
                    await rebuildBook([...activeAudiobook.files.map((f) => f.path), ...picked], "Adding files");
                } else {
                    await importPaths(picked);
                }
            } catch (error) {
                toast({ title: "Import failed", description: String(error), variant: "destructive" });
            }
        },
        [activeAudiobook, importPaths, rebuildBook, toast]
    );

    const openProject = useCallback(
        async (path?: string) => {
            try {
                const target = path ?? (await FilesApi.pickOpenProjectFile());
                if (!target) return;
                const project = await ProjectApi.loadProject(target);
                setActiveAudiobook(project.audiobook);
                setActiveProject(project);
            } catch (error) {
                toast({ title: "Could not open project", description: String(error), variant: "destructive" });
            }
        },
        [setActiveAudiobook, setActiveProject, toast]
    );

    const saveProject = useCallback(async () => {
        if (!activeAudiobook) return;
        try {
            const path = await FilesApi.pickSaveProjectFile();
            if (!path) return;
            await ProjectApi.saveProject(path, getEffectiveConversionDefaults());
            toast({ title: "Project saved", description: path });
        } catch (error) {
            toast({ title: "Save failed", description: String(error), variant: "destructive" });
        }
    }, [activeAudiobook, getEffectiveConversionDefaults, toast]);

    const exportSummary = useCallback(async () => {
        if (!activeAudiobook) return;
        try {
            const summary = await ProjectApi.exportProjectSummary();
            const path = await FilesApi.pickExportSummaryFile(`${summary.title || "audiobook"}.txt`);
            if (!path) return;
            const lines = [
                summary.title,
                summary.author ? `by ${summary.author}` : null,
                summary.narrator ? `read by ${summary.narrator}` : null,
                "",
                `${summary.duration_formatted} · ${summary.file_count} files · ${summary.chapter_count} chapters`,
                "",
                ...summary.chapters.map(
                    (c) =>
                        `${String(c.index + 1).padStart(3, " ")}. ${c.title}  ${c.start_time}–${c.end_time} (${c.duration})`
                ),
            ].filter((line): line is string => line !== null);
            await FilesApi.writeTextFile(path, lines.join("\n"));
            toast({ title: "Summary exported", description: path });
        } catch (error) {
            toast({ title: "Export failed", description: String(error), variant: "destructive" });
        }
    }, [activeAudiobook, toast]);

    const closeBook = useCallback(async () => {
        try {
            await AudiobookApi.clearCurrentAudiobook();
            await ProjectApi.clearProject();
        } catch (error) {
            console.error("Failed to clear book:", error);
        }
        setActiveAudiobook(null);
        setActiveProject(null);
    }, [setActiveAudiobook, setActiveProject]);

    return { importFiles, openProject, saveProject, exportSummary, closeBook };
}
