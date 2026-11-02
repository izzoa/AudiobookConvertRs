import { useMemo, useState } from "react";
import {
    DndContext,
    closestCenter,
    KeyboardSensor,
    PointerSensor,
    useSensor,
    useSensors,
    type DragEndEvent,
} from "@dnd-kit/core";
import {
    arrayMove,
    SortableContext,
    sortableKeyboardCoordinates,
    useSortable,
    verticalListSortingStrategy,
} from "@dnd-kit/sortable";
import { CSS } from "@dnd-kit/utilities";
import {
    Bars2Icon,
    PlusIcon,
    TrashIcon,
    ArrowsUpDownIcon,
    ExclamationTriangleIcon,
    FolderIcon,
} from "@heroicons/react/24/outline";
import { Button } from "@/components/ui/button";
import { Popover, MenuItem, MenuLabel } from "@/components/ui/popover";
import { PlayButton } from "@/components/AudioPlayer";
import { useToast } from "@/components/ui/use-toast";
import { useApp } from "@/context/useApp";
import { useRebuildBook } from "@/hooks/useRebuildBook";
import * as FilesApi from "@/api/files";
import type { AudioFile } from "@/types/models";
import { basename, formatBytes, formatClock } from "@/lib/format";
import { cn } from "@/lib/utils";

type SortKey = "name" | "track" | "duration";

/**
 * The file list is the book's spine: order here is playback order, so it is
 * directly draggable rather than hidden behind a sort dialog.
 */
export function FilesPanel() {
    const { activeAudiobook } = useApp();
    const { toast } = useToast();
    const [selected, setSelected] = useState<Set<string>>(new Set());
    const [busy, setBusy] = useState(false);

    const sensors = useSensors(
        useSensor(PointerSensor, { activationConstraint: { distance: 4 } }),
        useSensor(KeyboardSensor, { coordinateGetter: sortableKeyboardCoordinates })
    );

    const files = activeAudiobook?.files ?? [];

    const rebuildBook = useRebuildBook();
    const rebuild = async (paths: string[], action: string) => {
        setBusy(true);
        try {
            if (await rebuildBook(paths, action)) setSelected(new Set());
        } finally {
            setBusy(false);
        }
    };

    const handleDragEnd = (event: DragEndEvent) => {
        const { active, over } = event;
        if (!over || active.id === over.id) return;
        const from = files.findIndex((f) => f.path === active.id);
        const to = files.findIndex((f) => f.path === over.id);
        if (from < 0 || to < 0) return;
        rebuild(arrayMove(files, from, to).map((f) => f.path), "Reordering");
    };

    const sortBy = (key: SortKey) => {
        const sorted = [...files].sort((a, b) => {
            if (key === "duration") return a.duration_ms - b.duration_ms;
            if (key === "track") {
                const at = a.metadata.track_number ?? Number.MAX_SAFE_INTEGER;
                const bt = b.metadata.track_number ?? Number.MAX_SAFE_INTEGER;
                if (at !== bt) return at - bt;
            }
            return byFolderThenName(a.path, b.path);
        });
        rebuild(sorted.map((f) => f.path), "Sorting");
    };

    const removeSelected = () => {
        const remaining = files.filter((f) => !selected.has(f.path));
        if (remaining.length === 0) {
            toast({
                title: "Cannot remove every file",
                description: "Close the book instead, from the title menu.",
                variant: "destructive",
            });
            return;
        }
        rebuild(remaining.map((f) => f.path), "Removing files");
    };

    const addFiles = async () => {
        try {
            const picked = await FilesApi.pickFiles();
            // Added files join this book (keeping its typed tags); they are never
            // split into a batch the way a fresh import is.
            if (picked?.length) await rebuild([...files.map((f) => f.path), ...picked], "Adding files");
        } catch (error) {
            toast({ title: "Could not add files", description: String(error), variant: "destructive" });
        }
    };

    const toggle = (path: string, additive: boolean) => {
        setSelected((prev) => {
            const next = additive ? new Set(prev) : new Set<string>();
            if (additive && prev.has(path)) next.delete(path);
            else next.add(path);
            return next;
        });
    };

    const gaps = useMemo(() => detectOrderingDoubt(files), [files]);
    // Disc folders (CD1/, CD2/) explain the order, so name them when present.
    const multiFolder = useMemo(() => new Set(files.map((f) => dirname(f.path))).size > 1, [files]);

    return (
        <div className="h-full flex flex-col">
            <div className="shrink-0 h-11 px-3 border-b flex items-center gap-1.5 no-select">
                <Button variant="ghost" size="sm" onClick={addFiles} disabled={busy}>
                    <PlusIcon className="w-4 h-4 mr-1.5" />
                    Add files
                </Button>

                <Popover
                    align="start"
                    trigger={() => (
                        <Button variant="ghost" size="sm" disabled={busy}>
                            <ArrowsUpDownIcon className="w-4 h-4 mr-1.5" />
                            Sort
                        </Button>
                    )}
                >
                    {(close) => (
                        <>
                            <MenuLabel>Order files by</MenuLabel>
                            <MenuItem onClick={() => { close(); sortBy("name"); }}>
                                Filename (natural)
                            </MenuItem>
                            <MenuItem onClick={() => { close(); sortBy("track"); }}>
                                Track number
                            </MenuItem>
                            <MenuItem onClick={() => { close(); sortBy("duration"); }}>
                                Duration
                            </MenuItem>
                        </>
                    )}
                </Popover>

                <Button
                    variant="ghost"
                    size="sm"
                    onClick={removeSelected}
                    disabled={busy || selected.size === 0}
                    className="text-destructive hover:text-destructive hover:bg-destructive/10"
                >
                    <TrashIcon className="w-4 h-4 mr-1.5" />
                    Remove{selected.size > 0 ? ` (${selected.size})` : ""}
                </Button>

                <div className="flex-1" />
                <span className="text-xs text-muted-foreground nums pr-1">
                    {files.length} {files.length === 1 ? "file" : "files"}
                </span>
            </div>

            {gaps && (
                <div className="shrink-0 flex items-start gap-2 px-3 py-2 bg-warning/10 border-b text-xs text-foreground/80">
                    <ExclamationTriangleIcon className="w-4 h-4 text-warning shrink-0 mt-px" />
                    <span>
                        Filenames suggest a different order than the current one. Sort by filename, or drag
                        rows to fix it — this is the order listeners will hear.
                    </span>
                </div>
            )}

            <div className={cn("flex-1 overflow-y-auto", busy && "opacity-60 pointer-events-none")}>
                <DndContext sensors={sensors} collisionDetection={closestCenter} onDragEnd={handleDragEnd}>
                    <SortableContext items={files.map((f) => f.path)} strategy={verticalListSortingStrategy}>
                        {files.map((file, index) => {
                            const folder = dirname(file.path);
                            const startsFolder = multiFolder && (index === 0 || dirname(files[index - 1].path) !== folder);
                            return (
                                <div key={file.path}>
                                    {startsFolder && <FolderDivider path={folder} />}
                                    <FileRow
                                        file={file}
                                        index={index}
                                        selected={selected.has(file.path)}
                                        onToggle={toggle}
                                    />
                                </div>
                            );
                        })}
                    </SortableContext>
                </DndContext>
            </div>
        </div>
    );
}

function FileRow({
    file,
    index,
    selected,
    onToggle,
}: {
    file: AudioFile;
    index: number;
    selected: boolean;
    onToggle: (path: string, additive: boolean) => void;
}) {
    const { attributes, listeners, setNodeRef, transform, transition, isDragging } = useSortable({
        id: file.path,
    });

    return (
        <div
            ref={setNodeRef}
            style={{ transform: CSS.Transform.toString(transform), transition }}
            onClick={(e) => onToggle(file.path, e.metaKey || e.ctrlKey)}
            className={cn(
                "group flex items-center gap-2 h-[42px] px-2 border-b border-border/60 text-sm cursor-default",
                selected ? "bg-primary/[0.08]" : "hover:bg-accent/50",
                isDragging && "relative z-10 bg-surface shadow-lift rounded-md border-transparent"
            )}
        >
            <button
                {...attributes}
                {...listeners}
                aria-label={`Reorder ${basename(file.path)}`}
                className="w-5 h-7 shrink-0 flex items-center justify-center text-muted-foreground/40 group-hover:text-muted-foreground cursor-grab active:cursor-grabbing"
            >
                <Bars2Icon className="w-4 h-4" />
            </button>

            <span className="w-7 shrink-0 text-right text-xs text-muted-foreground nums">{index + 1}</span>

            <span className="flex-1 min-w-0 truncate" title={file.path}>
                {file.metadata.title || basename(file.path)}
            </span>

            <span className="hidden lg:inline shrink-0 px-1.5 py-0.5 rounded text-[10px] font-semibold uppercase tracking-wide text-muted-foreground bg-muted">
                {file.format}
            </span>

            <span className="w-16 shrink-0 text-right text-xs text-muted-foreground nums">
                {formatClock(file.duration_ms)}
            </span>
            <span className="w-16 shrink-0 text-right text-xs text-muted-foreground nums">
                {formatBytes(file.file_size_bytes)}
            </span>

            <PlayButton path={file.path} className="shrink-0 opacity-0 group-hover:opacity-100 focus:opacity-100" />
        </div>
    );
}

function FolderDivider({ path }: { path: string }) {
    return (
        <div
            className="sticky top-0 z-[1] flex items-center gap-2 h-7 px-3 border-b border-border/60 bg-surface-2 text-xs font-medium text-muted-foreground no-select"
            title={path}
        >
            <FolderIcon className="w-3.5 h-3.5 shrink-0" />
            <span className="truncate">{basename(path) || path}</span>
        </div>
    );
}

const collate = (a: string, b: string) =>
    a.localeCompare(b, undefined, { numeric: true, sensitivity: "base" });

function dirname(path: string): string {
    const cut = path.lastIndexOf("/");
    return cut < 0 ? "" : path.slice(0, cut);
}

/** Folder first, so `CD1/01` and `CD2/01` never interleave — the backend's order. */
function byFolderThenName(a: string, b: string): number {
    return collate(dirname(a), dirname(b)) || collate(basename(a), basename(b));
}

/**
 * Flags the common import mistake: a player sorted "10, 1, 2" so the files
 * arrived out of order.
 *
 * Compares the containing directory before the filename, matching how the
 * backend orders a folder import — otherwise a book split across CD1/CD2 looks
 * wrong to this check while actually being correct.
 */
function detectOrderingDoubt(files: AudioFile[]): boolean {
    if (files.length < 3) return false;
    const paths = files.map((f) => f.path);
    const natural = [...paths].sort(byFolderThenName);
    return paths.some((path, i) => path !== natural[i]);
}
