import type { Chapter, ChapterOffer, LookupRecord, Metadata } from "@/types/models";

/**
 * The review step of Find details: one row per value a lookup found, beside
 * the book's current value, and the apply payload built from the rows the
 * user leaves ticked.
 *
 * Kept free of runtime imports so `npm test` can load it under plain Node.
 */

type FieldKey =
    | "title"
    | "artist"
    | "narrator"
    | "series"
    | "series_index"
    | "year"
    | "genre"
    | "publisher"
    | "language"
    | "description"
    | "copyright"
    | "asin";

export type ReviewKey = FieldKey | "cover" | "chapters";
export type RowState = "selected" | "unselected" | "unchanged";

const FIELDS: { key: FieldKey; label: string }[] = [
    { key: "title", label: "Title" },
    { key: "artist", label: "Author" },
    { key: "narrator", label: "Narrator" },
    { key: "series", label: "Series" },
    { key: "series_index", label: "Book number" },
    { key: "year", label: "Year" },
    { key: "genre", label: "Genre" },
    { key: "publisher", label: "Publisher" },
    { key: "language", label: "Language" },
    { key: "description", label: "Description" },
    { key: "copyright", label: "Copyright" },
    { key: "asin", label: "ASIN" },
];

export interface ReviewRow {
    key: ReviewKey;
    label: string;
    /** The book's value as text; empty when it has none. */
    current: string;
    /** The found value as text (the cover row carries the image address). */
    found: string;
    state: RowState;
    /** Set when the row cannot be applied, saying why. */
    unavailable?: string;
}

export interface BookSnapshot {
    metadata: Metadata;
    hasCover: boolean;
    chapters: Chapter[];
}

export interface ApplyPayload {
    fields: Metadata;
    coverUrl?: string;
    chapters?: Chapter[];
}

/**
 * Rows for every value the source provides. An empty current value starts
 * selected, a different one unselected, an identical one is unchanged.
 */
export function reviewRows(book: BookSnapshot, record: LookupRecord): ReviewRow[] {
    const rows: ReviewRow[] = [];
    for (const { key, label } of FIELDS) {
        const found = text(record.fields[key]);
        if (!found) continue;
        const current = text(book.metadata[key]);
        const state: RowState = !current ? "selected" : current === found ? "unchanged" : "unselected";
        rows.push({ key, label, current, found, state });
    }
    if (record.cover_url) {
        rows.push({
            key: "cover",
            label: "Cover",
            current: book.hasCover ? "Current cover" : "",
            found: record.cover_url,
            state: book.hasCover ? "unselected" : "selected",
        });
    }
    rows.push(chapterRow(book.chapters, record.chapters));
    return rows;
}

/** The keys that start ticked. */
export function initialSelection(rows: ReviewRow[]): Set<ReviewKey> {
    return new Set(rows.filter((r) => r.state === "selected").map((r) => r.key));
}

export function isSelectable(row: ReviewRow): boolean {
    return row.state !== "unchanged" && !row.unavailable;
}

/** What `apply_lookup` receives: only the ticked values. */
export function applyPayload(record: LookupRecord, selected: ReadonlySet<ReviewKey>): ApplyPayload {
    const fields: Metadata = {};
    for (const { key } of FIELDS) {
        const value = record.fields[key];
        if (selected.has(key) && value != null && value !== "") {
            (fields as Record<FieldKey, unknown>)[key] = value;
        }
    }
    const chapters = record.chapters;
    return {
        fields,
        coverUrl: selected.has("cover") ? (record.cover_url ?? undefined) : undefined,
        chapters: selected.has("chapters") && chapters.kind !== "unavailable" ? chapters.chapters : undefined,
    };
}

function chapterRow(current: Chapter[], offer: ChapterOffer): ReviewRow {
    const base = { key: "chapters" as const, label: "Chapters", current: countChapters(current.length) };
    if (offer.kind === "unavailable") {
        return { ...base, found: "", state: "unselected", unavailable: offer.reason };
    }
    const n = offer.chapters.length;
    const found =
        offer.kind === "exact"
            ? `${countChapters(n)}, length matches`
            : offer.kind === "shifted"
              ? `${countChapters(n)}, moved ${(offer.shift_ms / 1000).toFixed(2)} s earlier for the removed intro`
              : `${n} chapter titles on your current chapters`;
    // Generated chapters (one per file, auto-split, CUE) are placeholders, so
    // found chapters replace them by default; hand-edited ones start unticked.
    const generated = current.every((c) => c.auto_detected);
    return { ...base, found, state: generated ? "selected" : "unselected" };
}

function countChapters(n: number): string {
    return n === 1 ? "1 chapter" : `${n} chapters`;
}

function text(value: unknown): string {
    if (value == null) return "";
    return String(value).trim();
}
