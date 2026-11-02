import { useCallback, useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import { useApp } from "@/context/useApp";
import { useConversionStore } from "@/context/useConversionStore";
import { BatchContext, type BatchEntry, type BatchState } from "@/context/useBatch";
import * as AudiobookApi from "@/api/audiobook";
import * as ConversionApi from "@/api/conversion";
import * as FilesApi from "@/api/files";
import * as MetadataApi from "@/api/metadata";
import type { AudioBook, BookGroup } from "@/types/models";
import { buildConversionSettings } from "@/lib/conversionSettings";
import { numberedPath, outputPathForBook } from "@/lib/outputTemplate";
import { basename } from "@/lib/format";

const NO_PATHS = new Set<string>();

let nextKey = 1;
const toEntry = (group: BookGroup): BatchEntry => ({
    key: `batch-${nextKey++}`,
    name: group.name,
    files: group.files,
    status: "loading",
    include: true,
});

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
    const [onDisk, setOnDisk] = useState<Set<string>>(NO_PATHS);

    // Callbacks read entries from the ref, so a book loading in the background
    // sees a removal made a moment ago. changeEntries is the only writer, which
    // keeps the ref and the rendered state in step.
    const entriesRef = useRef<BatchEntry[]>([]);
    const changeEntries = useCallback((change: (prev: BatchEntry[]) => BatchEntry[]) => {
        entriesRef.current = change(entriesRef.current);
        setEntries(entriesRef.current);
    }, []);

    const outputDir = outputDirChoice ?? settings?.default_output_directory ?? null;

    const update = useCallback(
        (key: string, patch: Partial<BatchEntry>) =>
            changeEntries((prev) => prev.map((e) => (e.key === key ? { ...e, ...patch } : e))),
        [changeEntries]
    );

    const autoDetectArtwork = settings?.auto_detect_artwork !== false;
    const loadEntries = useCallback(
        async (pending: BatchEntry[]) => {
            for (const entry of pending) {
                if (!entriesRef.current.some((e) => e.key === entry.key)) continue;
                try {
                    let book = await AudiobookApi.createAudiobookFromFiles(entry.files, false);
                    // The workbench claims a cover when a book is opened; a batch
                    // book may never be opened, so it claims one here instead.
                    if (autoDetectArtwork && !book.output_metadata?.artwork_data) {
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
        [autoDetectArtwork, update]
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
            changeEntries((prev) => [...prev, ...fresh]);
            setView("batch");
            void loadEntries(fresh);
            return "batch";
        },
        [changeEntries, loadEntries, setActiveAudiobook, setView]
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

    const removeEntry = useCallback(
        (key: string) => changeEntries((prev) => prev.filter((e) => e.key !== key)),
        [changeEntries]
    );

    // A folder of finished single-file books (Author/*.m4b) is one folder, so it
    // arrives as one book; splitting makes each file its own.
    const splitEntry = useCallback(
        (key: string) => {
            const entry = entriesRef.current.find((e) => e.key === key);
            if (!entry || entry.files.length < 2) return;
            const parts = entry.files.map((file) =>
                toEntry({ name: basename(file).replace(/\.[^.]+$/, ""), files: [file] })
            );
            changeEntries((prev) => prev.flatMap((e) => (e.key === key ? parts : [e])));
            void loadEntries(parts);
        },
        [changeEntries, loadEntries]
    );

    const setIncluded = useCallback((key: string, include: boolean) => update(key, { include }), [update]);
    const setAllIncluded = useCallback(
        (include: boolean) =>
            changeEntries((prev) => prev.map((e) => (e.status === "ready" ? { ...e, include } : e))),
        [changeEntries]
    );

    const clearBatch = useCallback(() => changeEntries(() => []), [changeEntries]);

    // When folder shape guesses wrong (a book split into "Part One", "Part Two"
    // folders), the whole drop can still become one book.
    const combineIntoOneBook = useCallback(async () => {
        const files = entriesRef.current.flatMap((e) => e.files);
        const book = await AudiobookApi.createAudiobookFromFiles(files);
        clearBatch();
        setActiveAudiobook(book);
        setView("book");
    }, [clearBatch, setActiveAudiobook, setView]);

    const replaceBook = useCallback(
        (oldId: string, book: AudioBook) =>
            changeEntries((prev) =>
                prev.map((e) => (e.book?.id === oldId ? { ...e, book, files: book.files.map((f) => f.path) } : e))
            ),
        [changeEntries]
    );

    // Edits made in the workbench (tags, cover, chapters) land on the active
    // book; mirror them into the batch so its list and its jobs stay current.
    useEffect(() => {
        if (!activeAudiobook) return;
        changeEntries((prev) =>
            prev.some((e) => e.book?.id === activeAudiobook.id && e.book !== activeAudiobook)
                ? prev.map((e) => (e.book?.id === activeAudiobook.id ? { ...e, book: activeAudiobook } : e))
                : prev
        );
    }, [activeAudiobook, changeEntries]);

    const outputPaths = useMemo(() => {
        const paths = new Map<string, string>();
        if (!outputDir) return paths;
        const used = new Set<string>();
        for (const entry of entries) {
            if (entry.status !== "ready" || !entry.book) continue;
            const base = outputPathForBook(outputDir, settings?.output_template, entry.book);
            let path = base;
            // Two books with one name would otherwise overwrite each other.
            for (let n = 2; used.has(path.toLowerCase()); n++) path = numberedPath(base, n);
            used.add(path.toLowerCase());
            paths.set(entry.key, path);
        }
        return paths;
    }, [entries, outputDir, settings?.output_template]);

    const pathList = useMemo(() => [...outputPaths.values()].sort().join("\n"), [outputPaths]);
    useEffect(() => {
        if (!pathList) return;
        let cancelled = false;
        FilesApi.existingPaths(pathList.split("\n"))
            .then((found) => !cancelled && setOnDisk(new Set(found)))
            .catch(() => !cancelled && setOnDisk(NO_PATHS));
        return () => {
            cancelled = true;
        };
    }, [pathList]);
    // With no paths there is nothing to check, so a stale answer never shows.
    const existing = pathList ? onDisk : NO_PATHS;

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
                changeEntries((prev) => prev.filter((e) => !queued.includes(e.key)));
            }
            return queued.length;
        },
        [changeEntries, existing, getEffectiveConversionDefaults, outputPaths, refreshJobs, refreshQueueState, settings]
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
