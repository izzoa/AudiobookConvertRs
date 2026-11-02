import { useCallback, useState } from "react";
import { AppShell } from "@/components/shell/AppShell";
import { Workbench, type WorkbenchTab } from "@/components/workbench/Workbench";
import { ConvertBar } from "@/components/workbench/ConvertBar";
import { EmptyState } from "@/components/workbench/EmptyState";
import { QueuePage } from "@/components/queue/QueuePage";
import { SettingsPage } from "@/components/SettingsPage";
import { HelpPage } from "@/components/HelpPage";
import { useToast } from "@/components/ui/use-toast";
import { useFileDrop } from "@/hooks/useFileDrop";
import { useKeyboardShortcuts } from "@/hooks/useKeyboardShortcuts";
import { useProjectActions } from "@/hooks/useProjectActions";
import { useApp } from "@/context/AppContext";
import * as AudiobookApi from "@/api/audiobook";

function App() {
    const { view, setView, activeAudiobook, setActiveAudiobook } = useApp();
    const { toast } = useToast();
    const [tab, setTab] = useState<WorkbenchTab>("files");
    const { openProject, saveProject } = useProjectActions();

    const importPaths = useCallback(
        async (paths: string[]) => {
            if (paths.length === 0) return;
            try {
                setActiveAudiobook(await AudiobookApi.createAudiobookFromFiles(paths));
                setView("book");
                setTab("files");
            } catch (error) {
                toast({
                    title: "Import failed",
                    description: String(error),
                    variant: "destructive",
                });
            }
        },
        [setActiveAudiobook, setView, toast]
    );

    // Dropping files is the primary way in, so it works from any view.
    const isDraggingOver = useFileDrop(importPaths);

    // ⌘O / ⌘S run the same commands as the title menu, not just a view switch.
    useKeyboardShortcuts({
        onOpen: () => {
            setView("book");
            openProject();
        },
        onSave: () => {
            setView("book");
            saveProject();
        },
    });

    return (
        <AppShell isDraggingOver={isDraggingOver}>
            {view === "queue" && <QueuePage />}
            {view === "settings" && (
                <div className="flex-1 overflow-y-auto">
                    <SettingsPage />
                </div>
            )}
            {view === "help" && (
                <div className="flex-1 overflow-y-auto">
                    <HelpPage />
                </div>
            )}
            {view === "book" &&
                (activeAudiobook ? (
                    <>
                        <Workbench tab={tab} onTabChange={setTab} onImport={importPaths} />
                        <ConvertBar onShowChapters={() => setTab("chapters")} />
                    </>
                ) : (
                    <EmptyState isDraggingOver={isDraggingOver} onImport={importPaths} />
                ))}
        </AppShell>
    );
}

export default App;
