import { useEffect, useState } from "react";
import {
    BookOpenIcon,
    ChevronDownIcon,
    ArrowUturnLeftIcon,
    ArrowUturnRightIcon,
    DocumentPlusIcon,
    FolderOpenIcon,
    ArrowDownTrayIcon,
    DocumentTextIcon,
    XMarkIcon,
    PlusIcon,
    ClockIcon,
} from "@heroicons/react/24/outline";
import { Popover, MenuItem, MenuSeparator, MenuLabel } from "@/components/ui/popover";
import { QueueStatusPill } from "@/components/QueueStatusPill";
import { useApp } from "@/context/AppContext";
import { useProjectActions } from "@/hooks/useProjectActions";
import * as ProjectApi from "@/api/project";
import type { RecentProject } from "@/types/models";
import { formatDuration, MOD_KEY } from "@/lib/format";
import { cn } from "@/lib/utils";

/**
 * Identity + project commands + global state, in that reading order.
 *
 * The book chip doubles as the project menu: every command here acts on the
 * open book, so there is one place to look for them rather than a sidebar, a
 * ProjectManager card and a file dialog button.
 */
export function TopBar() {
    const { activeAudiobook, activeProject, undoState, handleUndo, handleRedo } = useApp();
    const { importFiles, openProject, saveProject, exportSummary, closeBook } = useProjectActions();
    const [recent, setRecent] = useState<RecentProject[]>([]);

    useEffect(() => {
        ProjectApi.listRecentProjects()
            .then((list) => setRecent(list.slice(0, 5)))
            .catch(() => setRecent([]));
    }, [activeProject?.saved_at]);

    const subtitle = activeAudiobook
        ? [
              `${activeAudiobook.files.length} ${activeAudiobook.files.length === 1 ? "file" : "files"}`,
              formatDuration(activeAudiobook.total_duration_ms),
              activeAudiobook.chapters.length > 0
                  ? `${activeAudiobook.chapters.length} chapters`
                  : "no chapters",
          ].join(" · ")
        : "Drop audio files anywhere to begin";

    return (
        <header className="h-[52px] shrink-0 bg-surface border-b flex items-center gap-2 pl-3 pr-3 no-select">
            <Popover
                align="start"
                trigger={(open) => (
                    <button
                        aria-haspopup="menu"
                        aria-expanded={open}
                        className={cn(
                            "flex items-center gap-2.5 h-10 pl-2 pr-2.5 rounded-lg transition-colors max-w-[460px]",
                            open ? "bg-accent" : "hover:bg-accent/70"
                        )}
                    >
                        <span className="w-8 h-8 rounded-md bg-primary/10 text-primary flex items-center justify-center shrink-0">
                            <BookOpenIcon className="w-[18px] h-[18px]" />
                        </span>
                        <span className="min-w-0 text-left">
                            <span className="block text-[13px] font-semibold leading-tight truncate">
                                {activeAudiobook?.title || "No book open"}
                                {activeAudiobook?.is_dirty && (
                                    <span
                                        className="inline-block ml-1.5 mb-px w-1.5 h-1.5 rounded-full bg-primary align-middle"
                                        title="Unsaved changes"
                                        aria-label="Unsaved changes"
                                    />
                                )}
                            </span>
                            <span className="block text-[11px] text-muted-foreground leading-tight truncate nums">
                                {subtitle}
                            </span>
                        </span>
                        <ChevronDownIcon className="w-3.5 h-3.5 text-muted-foreground shrink-0" />
                    </button>
                )}
            >
                {(close) => (
                    <>
                        <MenuItem
                            icon={<DocumentPlusIcon />}
                            onClick={() => {
                                close();
                                importFiles(false);
                            }}
                        >
                            Import audio files…
                        </MenuItem>
                        <MenuItem
                            icon={<PlusIcon />}
                            disabled={!activeAudiobook}
                            onClick={() => {
                                close();
                                importFiles(true);
                            }}
                        >
                            Add files to this book…
                        </MenuItem>
                        <MenuSeparator />
                        <MenuItem
                            icon={<FolderOpenIcon />}
                            shortcut={`${MOD_KEY}O`}
                            onClick={() => {
                                close();
                                openProject();
                            }}
                        >
                            Open project…
                        </MenuItem>
                        <MenuItem
                            icon={<ArrowDownTrayIcon />}
                            shortcut={`${MOD_KEY}S`}
                            disabled={!activeAudiobook}
                            onClick={() => {
                                close();
                                saveProject();
                            }}
                        >
                            Save project as…
                        </MenuItem>
                        <MenuItem
                            icon={<DocumentTextIcon />}
                            disabled={!activeAudiobook}
                            onClick={() => {
                                close();
                                exportSummary();
                            }}
                        >
                            Export chapter summary…
                        </MenuItem>
                        {recent.length > 0 && (
                            <>
                                <MenuLabel>Recent</MenuLabel>
                                {recent.map((project) => (
                                    <MenuItem
                                        key={project.path}
                                        icon={<ClockIcon />}
                                        disabled={!project.exists}
                                        onClick={() => {
                                            close();
                                            openProject(project.path);
                                        }}
                                    >
                                        {project.title}
                                    </MenuItem>
                                ))}
                            </>
                        )}
                        {activeAudiobook && (
                            <>
                                <MenuSeparator />
                                <MenuItem
                                    icon={<XMarkIcon />}
                                    danger
                                    onClick={() => {
                                        close();
                                        closeBook();
                                    }}
                                >
                                    Close book
                                </MenuItem>
                            </>
                        )}
                    </>
                )}
            </Popover>

            <div className="flex-1" />

            <div className="flex items-center gap-0.5">
                <IconButton
                    label={undoState.undoDescription ? `Undo ${undoState.undoDescription}` : "Undo"}
                    disabled={!undoState.canUndo}
                    onClick={handleUndo}
                >
                    <ArrowUturnLeftIcon className="w-[17px] h-[17px]" />
                </IconButton>
                <IconButton
                    label={undoState.redoDescription ? `Redo ${undoState.redoDescription}` : "Redo"}
                    disabled={!undoState.canRedo}
                    onClick={handleRedo}
                >
                    <ArrowUturnRightIcon className="w-[17px] h-[17px]" />
                </IconButton>
            </div>

            <div className="pl-1.5">
                <QueueStatusPill />
            </div>
        </header>
    );
}

function IconButton({
    children,
    label,
    onClick,
    disabled,
}: {
    children: React.ReactNode;
    label: string;
    onClick?: () => void;
    disabled?: boolean;
}) {
    return (
        <button
            title={label}
            aria-label={label}
            onClick={onClick}
            disabled={disabled}
            className={cn(
                "w-8 h-8 rounded-md flex items-center justify-center transition-colors",
                "text-muted-foreground hover:bg-accent hover:text-foreground",
                "disabled:opacity-30 disabled:pointer-events-none"
            )}
        >
            {children}
        </button>
    );
}
