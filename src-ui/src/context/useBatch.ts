import { createContext, useContext } from "react";
import type { AudioBook } from "@/types/models";

// Lives apart from BatchProvider so that editing the provider hot-reloads in place.

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

export interface BatchState {
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

export const BatchContext = createContext<BatchState | undefined>(undefined);

export function useBatch(): BatchState {
    const context = useContext(BatchContext);
    if (!context) throw new Error("useBatch must be used inside BatchProvider");
    return context;
}
