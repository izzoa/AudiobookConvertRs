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
    /** Set while the value is still being worked out; the row cannot be ticked meanwhile. */
    pending?: string;
    /** Fitted chapters that did not land on a track start, for the user to check. */
    flagged?: FlaggedChapter[];
}

export interface FlaggedChapter {
    title: string;
    start_ms: number;
    kind: "pause" | "estimate";
    /** Placed start minus estimated start. */
    moved_ms: number;
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
    return row.state !== "unchanged" && !row.unavailable && !row.pending;
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
    return {
        fields,
        coverUrl: selected.has("cover") ? (record.cover_url ?? undefined) : undefined,
        chapters: selected.has("chapters") ? offeredChapters(record.chapters) : undefined,
    };
}

/** The chapters an offer would apply; none while unavailable or still fitting. */
function offeredChapters(offer: ChapterOffer): Chapter[] | undefined {
    return offer.kind === "unavailable" || offer.kind === "fitting" ? undefined : offer.chapters;
}

function chapterRow(current: Chapter[], offer: ChapterOffer): ReviewRow {
    const base = { key: "chapters" as const, label: "Chapters", current: countChapters(current.length) };
    if (offer.kind === "unavailable") {
        return { ...base, found: "", state: "unselected", unavailable: offer.reason };
    }
    if (offer.kind === "fitting") {
        return { ...base, found: "", state: "unselected", pending: "Fitting chapters to your tracks…" };
    }
    const n = offer.chapters.length;
    const found =
        offer.kind === "exact"
            ? `${countChapters(n)}, length matches`
            : offer.kind === "shifted"
              ? `${countChapters(n)}, moved ${(offer.shift_ms / 1000).toFixed(2)} s earlier for the removed intro`
              : offer.kind === "fitted"
                ? fittedSummary(n, offer.counts)
                : `${n} chapter titles on your current chapters`;
    // Generated chapters (one per file, auto-split, CUE) are placeholders, so
    // found chapters replace them by default; hand-edited ones start unticked.
    const generated = current.every((c) => c.auto_detected);
    const row: ReviewRow = { ...base, found, state: generated ? "selected" : "unselected" };
    if (offer.kind === "fitted") row.flagged = flaggedChapters(offer.chapters, offer.placements);
    return row;
}

/** "48 chapters fitted: 45 on a track start, 2 at a pause, 1 estimated" */
function fittedSummary(n: number, counts: { track: number; pause: number; estimate: number }): string {
    if (counts.pause === 0 && counts.estimate === 0) return `${countChapters(n)} fitted, all on a track start`;
    const parts = [
        counts.track > 0 && `${counts.track} on a track start`,
        counts.pause > 0 && `${counts.pause} at a pause`,
        counts.estimate > 0 && `${counts.estimate} estimated`,
    ].filter(Boolean);
    return `${countChapters(n)} fitted: ${parts.join(", ")}`;
}

function flaggedChapters(
    chapters: Chapter[],
    placements: { index: number; kind: "track" | "pause" | "estimate"; moved_ms: number }[]
): FlaggedChapter[] {
    return placements.flatMap((p) => {
        const chapter = chapters[p.index];
        if (p.kind === "track" || !chapter) return [];
        return [{ title: chapter.title, start_ms: chapter.start_time_ms, kind: p.kind, moved_ms: p.moved_ms }];
    });
}

function countChapters(n: number): string {
    return n === 1 ? "1 chapter" : `${n} chapters`;
}

function text(value: unknown): string {
    if (value == null) return "";
    return String(value).trim();
}
