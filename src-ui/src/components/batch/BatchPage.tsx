import { useState } from "react";
import {
    AdjustmentsHorizontalIcon,
    ArrowPathIcon,
    DocumentTextIcon,
    ArrowRightIcon,
    EllipsisHorizontalIcon,
    ExclamationTriangleIcon,
    FolderIcon,
    PhotoIcon,
} from "@heroicons/react/24/outline";
import { Button } from "@/components/ui/button";
import { Popover, MenuItem, MenuSeparator } from "@/components/ui/popover";
import { useToast } from "@/components/ui/use-toast";
import { useApp } from "@/context/useApp";
import { useBatch, type BatchEntry } from "@/context/useBatch";
import * as FilesApi from "@/api/files";
import { formatDuration, shortenPath } from "@/lib/format";
import { describeTemplate } from "@/lib/outputTemplate";
import { cn } from "@/lib/utils";

/**
 * Every book from one import, reviewed together and converted together.
 *
 * A curator's library arrives as dozens of folders; the list makes the books
 * that need a look (no author, no cover, an existing file) stand out, and
 * opens any of them in the workbench for the full editor.
 */
export function BatchPage() {
    const { settings, setView, getEffectiveConversionDefaults } = useApp();
    const batch = useBatch();
    const { toast } = useToast();
    const [replaceExisting, setReplaceExisting] = useState(false);
    const [submitting, setSubmitting] = useState(false);

    const { entries, outputPaths, existing } = batch;
    const ready = entries.filter((e) => e.status === "ready");
    const failed = entries.filter((e) => e.status === "failed").length;
    const loadingCount = entries.filter((e) => e.status === "loading").length;
    const selected = ready.filter((e) => e.include);
    const exists = (e: BatchEntry) => {
        const path = outputPaths.get(e.key);
        return !!path && existing.has(path);
    };
    const selectedExisting = selected.filter(exists).length;
    const toConvert = replaceExisting ? selected.length : selected.length - selectedExisting;
    const allChecked = ready.length > 0 && ready.every((e) => e.include);

    const defaults = getEffectiveConversionDefaults();

    const chooseFolder = async () => {
        try {
            const dir = await FilesApi.pickDirectory();
            if (dir) batch.setOutputDir(dir);
        } catch (error) {
            toast({ title: "Could not open folder picker", description: String(error), variant: "destructive" });
        }
    };

    const convert = async () => {
        setSubmitting(true);
        try {
            const queued = await batch.convertAll({ replaceExisting });
            toast({
                title: `${queued} ${queued === 1 ? "book" : "books"} queued`,
                description: batch.outputDir ? `Writing to ${batch.outputDir}` : undefined,
            });
            setView("queue");
        } catch (error) {
            toast({ title: "Could not queue the batch", description: String(error), variant: "destructive" });
        } finally {
            setSubmitting(false);
        }
    };

    const combine = async () => {
        try {
            await batch.combineIntoOneBook();
        } catch (error) {
            toast({ title: "Could not combine", description: String(error), variant: "destructive" });
        }
    };

    const summary = [
        `${entries.length} ${entries.length === 1 ? "book" : "books"}`,
        loadingCount > 0 && `${loadingCount} loading`,
        failed > 0 && `${failed} could not be read`,
    ]
        .filter(Boolean)
        .join(" · ");

    return (
        <div className="flex-1 flex flex-col overflow-hidden">
            <div className="shrink-0 h-11 px-3 border-b flex items-center gap-2 no-select">
                <h1 className="text-sm font-semibold">Batch</h1>
                <span className="text-xs text-muted-foreground nums">{summary}</span>
                <div className="flex-1" />
                <Popover
                    align="end"
                    trigger={(open) => (
                        <Button
                            variant="ghost"
                            size="sm"
                            aria-label="Batch options"
                            aria-haspopup="menu"
                            aria-expanded={open}
                            className="w-9 px-0"
                        >
                            <EllipsisHorizontalIcon className="w-5 h-5" />
                        </Button>
                    )}
                >
                    {(close) => (
                        <>
                            <MenuItem
                                onClick={() => {
                                    close();
                                    combine();
                                }}
                            >
                                Combine into one book
                            </MenuItem>
                            <MenuSeparator />
                            <MenuItem
                                danger
                                onClick={() => {
                                    close();
                                    batch.clearBatch();
                                    setView("book");
                                }}
                            >
                                Clear batch
                            </MenuItem>
                        </>
                    )}
                </Popover>
            </div>

            <div className="shrink-0 h-8 px-3 border-b flex items-center gap-3 text-xs text-muted-foreground no-select">
                <input
                    type="checkbox"
                    aria-label="Include every book"
                    checked={allChecked}
                    disabled={ready.length === 0}
                    onChange={(e) => batch.setAllIncluded(e.target.checked)}
                    className="w-4 h-4 shrink-0"
                />
                <span className="w-10 shrink-0" />
                <span className="flex-1">Book</span>
                <span className="w-24 shrink-0 text-right">Length</span>
                <span className="w-40 shrink-0">Needs a look</span>
                <span className="w-[92px] shrink-0" />
            </div>

            <div className="flex-1 overflow-y-auto">
                {entries.map((entry) => (
                    <BatchRow
                        key={entry.key}
                        entry={entry}
                        exists={exists(entry)}
                        replaceExisting={replaceExisting}
                        outputPath={outputPaths.get(entry.key)}
                    />
                ))}
            </div>

            <footer className="shrink-0 border-t bg-surface-2">
                {selectedExisting > 0 && (
                    <label className="h-9 px-4 flex items-center gap-2 border-b border-border/60 text-xs cursor-pointer no-select">
                        <input
                            type="checkbox"
                            checked={replaceExisting}
                            onChange={(e) => setReplaceExisting(e.target.checked)}
                            className="w-3.5 h-3.5"
                        />
                        <span>
                            Replace {selectedExisting} existing {selectedExisting === 1 ? "file" : "files"}
                            <span className="text-muted-foreground">
                                {" "}
                                — otherwise {selectedExisting === 1 ? "that book is" : "those books are"} skipped
                            </span>
                        </span>
                    </label>
                )}
                <div className="h-[58px] px-3 flex items-center gap-2">
                    <button
                        onClick={chooseFolder}
                        className="flex items-center gap-2 h-9 px-2.5 rounded-md min-w-0 max-w-[45%] transition-colors hover:bg-accent"
                        title={batch.outputDir ?? "Choose where the books are written"}
                    >
                        <FolderIcon className="w-4 h-4 text-muted-foreground shrink-0" />
                        <span className="min-w-0 text-left">
                            <span className="block text-[10px] font-semibold uppercase tracking-wider text-muted-foreground leading-tight whitespace-nowrap">
                                Output folder
                            </span>
                            <span
                                className={cn(
                                    "block text-xs leading-tight truncate",
                                    batch.outputDir ? "text-foreground" : "text-warning"
                                )}
                            >
                                {batch.outputDir ? shortenPath(batch.outputDir, 46) : "Choose a folder…"}
                            </span>
                        </span>
                    </button>

                    <button
                        onClick={() => setView("settings")}
                        className="flex items-center gap-2 h-9 px-2.5 rounded-md min-w-0 max-w-[30%] shrink-[3] transition-colors hover:bg-accent"
                        title={`File and folder names come from Settings: ${settings?.output_template || "{title}"}`}
                    >
                        <DocumentTextIcon className="w-4 h-4 text-muted-foreground shrink-0" />
                        <span className="min-w-0 text-left">
                            <span className="block text-[10px] font-semibold uppercase tracking-wider text-muted-foreground leading-tight whitespace-nowrap">
                                Names
                            </span>
                            <span className="block text-xs leading-tight truncate">
                                {describeTemplate(settings?.output_template)}
                            </span>
                        </span>
                    </button>

                    <button
                        onClick={() => setView("settings")}
                        className="flex items-center gap-2 h-9 px-2.5 rounded-md shrink-0 transition-colors hover:bg-accent"
                        title="Every book uses the defaults from Settings"
                    >
                        <AdjustmentsHorizontalIcon className="w-4 h-4 text-muted-foreground" />
                        <span className="text-left">
                            <span className="block text-[10px] font-semibold uppercase tracking-wider text-muted-foreground leading-tight whitespace-nowrap">
                                Quality
                            </span>
                            <span className="block text-xs leading-tight nums">
                                {defaults.bitrate_kbps}k {defaults.encoding_mode.toUpperCase()} · from Settings
                            </span>
                        </span>
                    </button>

                    <div className="flex-1" />

                    {loadingCount > 0 && (
                        <span className="text-xs text-muted-foreground nums">Reading {loadingCount} more…</span>
                    )}
                    <Button
                        size="lg"
                        onClick={convert}
                        disabled={submitting || loadingCount > 0 || !batch.outputDir || toConvert === 0}
                        className="shrink-0"
                    >
                        {submitting
                            ? "Queueing…"
                            : `Convert ${toConvert} ${toConvert === 1 ? "book" : "books"}`}
                        <ArrowRightIcon className="w-4 h-4 ml-2" />
                    </Button>
                </div>
            </footer>
        </div>
    );
}

function bookTitle(entry: BatchEntry): string {
    return entry.book?.output_metadata?.title || entry.book?.title || entry.name;
}

function BatchRow({
    entry,
    exists,
    replaceExisting,
    outputPath,
}: {
    entry: BatchEntry;
    exists: boolean;
    replaceExisting: boolean;
    outputPath?: string;
}) {
    const batch = useBatch();
    const book = entry.book;
    const meta = book?.output_metadata;
    const title = bookTitle(entry);
    const byline = [meta?.artist, meta?.narrator && `read by ${meta.narrator}`].filter(Boolean).join(" · ");
    const cover = meta?.artwork_data ? `data:image/jpeg;base64,${meta.artwork_data}` : null;
    const muted = entry.status !== "ready" || !entry.include;

    return (
        <div
            className={cn(
                "group flex items-center gap-3 h-[60px] px-3 border-b border-border/60 text-sm",
                entry.status === "ready" && "hover:bg-accent/40"
            )}
        >
            <input
                type="checkbox"
                aria-label={`Include ${title}`}
                checked={entry.include}
                disabled={entry.status !== "ready"}
                onChange={(e) => batch.setIncluded(entry.key, e.target.checked)}
                className="w-4 h-4 shrink-0"
            />

            <span
                className={cn(
                    "w-10 h-10 shrink-0 rounded-md overflow-hidden bg-muted flex items-center justify-center",
                    muted && "opacity-60"
                )}
            >
                {cover ? (
                    <img src={cover} alt="" className="w-full h-full object-cover" />
                ) : (
                    <PhotoIcon className="w-4 h-4 text-muted-foreground" />
                )}
            </span>

            <span className={cn("flex-1 min-w-0", muted && "text-muted-foreground")}>
                <span className="block font-medium truncate" title={outputPath ?? title}>
                    {title}
                </span>
                <span className="block text-xs text-muted-foreground truncate">
                    {entry.status === "loading" ? entry.files.length > 0 && `${entry.files.length} files` : byline || " "}
                </span>
            </span>

            <span className="w-24 shrink-0 text-right text-xs text-muted-foreground nums leading-tight">
                {book ? (
                    <>
                        <span className="block">{formatDuration(book.total_duration_ms)}</span>
                        <span className="block">
                            {book.files.length} {book.files.length === 1 ? "file" : "files"}
                        </span>
                    </>
                ) : null}
            </span>

            <span className="w-40 shrink-0 flex flex-wrap gap-x-2 gap-y-0.5 text-xs">
                <Notes entry={entry} exists={exists} replaceExisting={replaceExisting} />
            </span>

            <span className="w-[92px] shrink-0 flex items-center justify-end gap-0.5">
                {entry.status === "ready" && (
                    <Button variant="ghost" size="sm" className="h-8 px-2.5" onClick={() => batch.openEntry(entry.key)}>
                        Open
                    </Button>
                )}
                <Popover
                    align="end"
                    trigger={(open) => (
                        <button
                            aria-label={`More for ${title}`}
                            aria-haspopup="menu"
                            aria-expanded={open}
                            className="w-8 h-8 rounded-md flex items-center justify-center text-muted-foreground hover:bg-accent hover:text-foreground"
                        >
                            <EllipsisHorizontalIcon className="w-4 h-4" />
                        </button>
                    )}
                >
                    {(close) => (
                        <>
                            <MenuItem
                                disabled={entry.files.length < 2 || entry.status === "loading"}
                                onClick={() => {
                                    close();
                                    batch.splitEntry(entry.key);
                                }}
                            >
                                Split into one book per file
                            </MenuItem>
                            <MenuSeparator />
                            <MenuItem
                                danger
                                onClick={() => {
                                    close();
                                    batch.removeEntry(entry.key);
                                }}
                            >
                                Remove from batch
                            </MenuItem>
                        </>
                    )}
                </Popover>
            </span>
        </div>
    );
}

/** What a curator should check before converting; nothing shown means ready. */
function Notes({ entry, exists, replaceExisting }: { entry: BatchEntry; exists: boolean; replaceExisting: boolean }) {
    if (entry.status === "loading") {
        return (
            <span className="flex items-center gap-1 text-muted-foreground">
                <ArrowPathIcon className="w-3.5 h-3.5 animate-spin" />
                Reading tags…
            </span>
        );
    }
    if (entry.status === "failed") {
        return (
            <span className="flex items-center gap-1 text-destructive min-w-0" title={entry.error}>
                <ExclamationTriangleIcon className="w-3.5 h-3.5 shrink-0" />
                <span className="truncate">Could not read</span>
            </span>
        );
    }

    const meta = entry.book?.output_metadata;
    const warnings = [
        exists && (replaceExisting ? "Replaces file" : "File exists"),
        !meta?.artist && "No author",
    ].filter(Boolean) as string[];

    return (
        <>
            {warnings.map((text) => (
                <span key={text} className="flex items-center gap-1 text-warning">
                    <ExclamationTriangleIcon className="w-3.5 h-3.5 shrink-0" />
                    {text}
                </span>
            ))}
            {!meta?.artwork_data && <span className="text-muted-foreground">No cover</span>}
        </>
    );
}
