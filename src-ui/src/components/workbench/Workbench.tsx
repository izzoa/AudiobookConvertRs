import { FilesPanel } from "./FilesPanel";
import { BookInspector } from "./BookInspector";
import { ChapterEditor } from "@/components/ChapterEditor";
import { useApp } from "@/context/useApp";
import { cn } from "@/lib/utils";

export type WorkbenchTab = "files" | "chapters";

/**
 * Center = the material (files, chapters). Right = the product (cover, tags).
 * Tabs switch only the material, so the book's identity never leaves the screen.
 */
export function Workbench({
    tab,
    onTabChange,
}: {
    tab: WorkbenchTab;
    onTabChange: (tab: WorkbenchTab) => void;
}) {
    const { activeAudiobook } = useApp();

    return (
        <div className="flex-1 flex overflow-hidden">
            <div className="flex-1 flex flex-col overflow-hidden min-w-0">
                <div className="shrink-0 h-10 px-2 border-b flex items-end gap-0.5 no-select">
                    <Tab
                        active={tab === "files"}
                        onClick={() => onTabChange("files")}
                        count={activeAudiobook?.files.length}
                    >
                        Files
                    </Tab>
                    <Tab
                        active={tab === "chapters"}
                        onClick={() => onTabChange("chapters")}
                        count={activeAudiobook?.chapters.length}
                    >
                        Chapters
                    </Tab>
                </div>

                <div className="flex-1 overflow-hidden">
                    {tab === "files" ? (
                        <FilesPanel />
                    ) : (
                        <ChapterEditor />
                    )}
                </div>
            </div>

            <BookInspector />
        </div>
    );
}

function Tab({
    active,
    onClick,
    count,
    children,
}: {
    active: boolean;
    onClick: () => void;
    count?: number;
    children: React.ReactNode;
}) {
    return (
        <button
            onClick={onClick}
            className={cn(
                "relative h-9 px-3 flex items-center gap-1.5 text-sm font-medium rounded-t-md transition-colors",
                active ? "text-foreground" : "text-muted-foreground hover:text-foreground hover:bg-accent/50"
            )}
        >
            {children}
            {count !== undefined && (
                <span
                    className={cn(
                        "px-1.5 h-[17px] rounded-full text-[10px] font-semibold leading-[17px] nums",
                        active ? "bg-primary/15 text-primary" : "bg-muted text-muted-foreground"
                    )}
                >
                    {count}
                </span>
            )}
            {active && <span className="absolute left-2 right-2 -bottom-px h-[2px] bg-primary rounded-full" />}
        </button>
    );
}
