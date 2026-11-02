import { createContext, useCallback, useContext, useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import { useApp } from "@/context/AppContext";
import { useConversionStore } from "@/context/ConversionStore";
import * as AudiobookApi from "@/api/audiobook";
import * as ConversionApi from "@/api/conversion";
import * as FilesApi from "@/api/files";
import * as MetadataApi from "@/api/metadata";
import type { AudioBook, BookGroup } from "@/types/models";
import { buildConversionSettings, outputPathFor } from "@/lib/conversionSettings";
import { basename } from "@/lib/format";

export type BatchStatus = "loading" | "ready" | "failed";

export interface BatchEntry {
    key: string;
    /** The folder name, shown until the book's own title is known. */
    name: string;
    files: string[];
    status: BatchStatus;
    book?: AudioBook;
    error?: string;
    include: boolean;
}

interface BatchState {
    entries: BatchEntry[];
    /** Where every book in the batch is written; settings' default until changed. */
    outputDir: string | null;
    setOutputDir: (dir: string) => void;
    /** Output path per entry key, made unique within the batch. */
    outputPaths: Map<string, string>;
    /** Output paths that already exist on disk. */
    existing: Set<string>;
    loading: boolean;
    /** Imports files or folders: one book opens in the workbench, several in the batch. */
    importPaths: (paths: string[]) => Promise<"single" | "batch">;
    openEntry: (key: string) => Promise<void>;
    removeEntry: (key: string) => void;
    splitEntry: (key: string) => void;
    setIncluded: (key: string, include: boolean) => void;
    setAllIncluded: (include: boolean) => void;
    combineIntoOneBook: () => Promise<void>;
    /** Rebuilding a book's files gives it a new id; keep the batch pointing at it. */
    replaceBook: (oldId: string, book: AudioBook) => void;
    clearBatch: () => void;
    /** Queues every included, ready book. Returns how many were queued. */
    convertAll: (options: { replaceExisting: boolean }) => Promise<number>;
}

const BatchContext = createContext<BatchState | undefined>(undefined);

let nextKey = 1;
const toEntry = (group: BookGroup): BatchEntry => ({
    key: `batch-${nextKey++}`,
    name: group.name,
    files: group.files,
    status: "loading",
    include: true,
});

export function bookTitle(entry: BatchEntry): string {
    return entry.book?.output_metadata?.title || entry.book?.title || entry.name;
}

/**
 * A batch is several books imported together and converted together.
 *
 * Books load one at a time in the background without becoming the active book,
 * so whatever is open in the workbench keeps its edits and its undo history
 * while a library loads.
 */
export function BatchProvider({ children }: { children: ReactNode }) {
    const {
        activeAudiobook,
        setActiveAudiobook,
        setView,
        settings,
        getEffectiveConversionDefaults,
        refreshUndoState,
    } = useApp();
    const { refreshJobs, refreshQueueState } = useConversionStore();
    const [entries, setEntries] = useState<BatchEntry[]>([]);
    const [outputDirChoice, setOutputDir] = useState<string | null>(null);
    const [existing, setExisting] = useState<Set<string>>(new Set());
    const entriesRef = useRef(entries);
    entriesRef.current = entries;

    const outputDir = outputDirChoice ?? settings?.default_output_directory ?? null;

    const update = (key: string, patch: Partial<BatchEntry>) =>
        setEntries((prev) => prev.map((e) => (e.key === key ? { ...e, ...patch } : e)));

    const loadEntries = useCallback(
        async (pending: BatchEntry[]) => {
            for (const entry of pending) {
                if (!entriesRef.current.some((e) => e.key === entry.key)) continue;
                try {
                    let book = await AudiobookApi.createAudiobookFromFiles(entry.files, false);
                    // The workbench claims a cover when a book is opened; a batch
                    // book may never be opened, so it claims one here instead.
                    if (settings?.auto_detect_artwork !== false && !book.output_metadata?.artwork_data) {
                        const found = await MetadataApi.discoverArtwork(book.id).catch(() => []);
                        if (found.length > 0) {
                            book = await MetadataApi.selectDiscoveredArtwork(book.id, found[0].source).catch(() => book);
                        }
                    }
                    update(entry.key, { status: "ready", book });
                } catch (error) {
                    update(entry.key, { status: "failed", error: String(error), include: false });
                }
            }
        },
        [settings?.auto_detect_artwork]
    );

    const importPaths = useCallback(
        async (paths: string[]): Promise<"single" | "batch"> => {
            const groups = await AudiobookApi.groupPathsIntoBooks(paths);

            // Nothing usable, or one book: load directly. With nothing usable the
            // backend's error names what was dropped.
            if (groups.length <= 1) {
                const book = await AudiobookApi.createAudiobookFromFiles(groups[0]?.files ?? paths);
                setActiveAudiobook(book);
                setView("book");
                return "single";
            }

            const fresh = groups.map(toEntry);
            setEntries((prev) => [...prev, ...fresh]);
            entriesRef.current = [...entriesRef.current, ...fresh];
            setView("batch");
            void loadEntries(fresh);
            return "batch";
        },
        [loadEntries, setActiveAudiobook, setView]
    );

    const openEntry = useCallback(
        async (key: string) => {
            const book = entriesRef.current.find((e) => e.key === key)?.book;
            if (!book) return;
            await AudiobookApi.setActiveAudiobookId(book.id);
            setActiveAudiobook(book);
            await refreshUndoState();
            setView("book");
        },
        [refreshUndoState, setActiveAudiobook, setView]
    );

    const removeEntry = useCallback((key: string) => {
        setEntries((prev) => prev.filter((e) => e.key !== key));
    }, []);

    // A folder of finished single-file books (Author/*.m4b) is one folder, so it
    // arrives as one book; splitting makes each file its own.
    const splitEntry = useCallback(
        (key: string) => {
            const entry = entriesRef.current.find((e) => e.key === key);
            if (!entry || entry.files.length < 2) return;
            const parts = entry.files.map((file) =>
                toEntry({ name: basename(file).replace(/\.[^.]+$/, ""), files: [file] })
            );
            const next = entriesRef.current.flatMap((e) => (e.key === key ? parts : [e]));
            entriesRef.current = next;
            setEntries(next);
            void loadEntries(parts);
        },
        [loadEntries]
    );

    const setIncluded = useCallback((key: string, include: boolean) => update(key, { include }), []);
    const setAllIncluded = useCallback(
        (include: boolean) =>
            setEntries((prev) => prev.map((e) => (e.status === "ready" ? { ...e, include } : e))),
        []
    );

    const clearBatch = useCallback(() => {
        entriesRef.current = [];
        setEntries([]);
    }, []);

    // When folder shape guesses wrong (a book split into "Part One", "Part Two"
    // folders), the whole drop can still become one book.
    const combineIntoOneBook = useCallback(async () => {
        const files = entriesRef.current.flatMap((e) => e.files);
        const book = await AudiobookApi.createAudiobookFromFiles(files);
        clearBatch();
        setActiveAudiobook(book);
        setView("book");
    }, [clearBatch, setActiveAudiobook, setView]);

    const replaceBook = useCallback((oldId: string, book: AudioBook) => {
        setEntries((prev) =>
            prev.map((e) => (e.book?.id === oldId ? { ...e, book, files: book.files.map((f) => f.path) } : e))
        );
    }, []);

    // Edits made in the workbench (tags, cover, chapters) land on the active
    // book; mirror them into the batch so its list and its jobs stay current.
    useEffect(() => {
        if (!activeAudiobook) return;
        setEntries((prev) =>
            prev.some((e) => e.book?.id === activeAudiobook.id && e.book !== activeAudiobook)
                ? prev.map((e) => (e.book?.id === activeAudiobook.id ? { ...e, book: activeAudiobook } : e))
                : prev
        );
    }, [activeAudiobook]);

    const outputPaths = useMemo(() => {
        const paths = new Map<string, string>();
        if (!outputDir) return paths;
        const used = new Set<string>();
        for (const entry of entries) {
            if (entry.status !== "ready") continue;
            let path = outputPathFor(outputDir, bookTitle(entry));
            // Two books with one title would otherwise overwrite each other.
            for (let n = 2; used.has(path.toLowerCase()); n++) {
                path = outputPathFor(outputDir, `${bookTitle(entry)} (${n})`);
            }
            used.add(path.toLowerCase());
            paths.set(entry.key, path);
        }
        return paths;
    }, [entries, outputDir]);

    const pathList = useMemo(() => [...outputPaths.values()].sort().join("\n"), [outputPaths]);
    useEffect(() => {
        if (!pathList) {
            setExisting(new Set());
            return;
        }
        let cancelled = false;
        FilesApi.existingPaths(pathList.split("\n"))
            .then((found) => !cancelled && setExisting(new Set(found)))
            .catch(() => !cancelled && setExisting(new Set()));
        return () => {
            cancelled = true;
        };
    }, [pathList]);

    const convertAll = useCallback(
        async ({ replaceExisting }: { replaceExisting: boolean }) => {
            const defaults = getEffectiveConversionDefaults();
            const queued: string[] = [];
            for (const entry of entriesRef.current) {
                const path = outputPaths.get(entry.key);
                if (!entry.include || entry.status !== "ready" || !entry.book || !path) continue;
                const exists = existing.has(path);
                if (exists && !replaceExisting) continue;
                const jobSettings = buildConversionSettings(defaults, settings, path);
                if (exists) jobSettings.overwrite_existing = true;
                await ConversionApi.enqueueConversion(entry.book.id, jobSettings);
                queued.push(entry.key);
            }
            if (queued.length > 0) {
                await ConversionApi.startConversionQueue(settings?.concurrent_jobs ?? 1);
                await Promise.all([refreshJobs(), refreshQueueState()]);
                const remaining = entriesRef.current.filter((e) => !queued.includes(e.key));
                entriesRef.current = remaining;
                setEntries(remaining);
            }
            return queued.length;
        },
        [existing, getEffectiveConversionDefaults, outputPaths, refreshJobs, refreshQueueState, settings]
    );

    const value: BatchState = {
        entries,
        outputDir,
        setOutputDir,
        outputPaths,
        existing,
        loading: entries.some((e) => e.status === "loading"),
        importPaths,
        openEntry,
        removeEntry,
        splitEntry,
        setIncluded,
        setAllIncluded,
        combineIntoOneBook,
        replaceBook,
        clearBatch,
        convertAll,
    };

    return <BatchContext.Provider value={value}>{children}</BatchContext.Provider>;
}

export function useBatch(): BatchState {
    const context = useContext(BatchContext);
    if (!context) throw new Error("useBatch must be used inside BatchProvider");
    return context;
}
