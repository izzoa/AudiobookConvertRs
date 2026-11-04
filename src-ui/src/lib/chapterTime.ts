import type { Chapter } from "@/types/models";

/**
 * Typed chapter start times: reading what the user typed, and moving one
 * chapter's start in a chapter list so the chapter before it ends there.
 *
 * Kept free of runtime imports so `npm test` can load it under plain Node.
 */

export const TIME_FORMATS = "h:mm:ss, m:ss or seconds";

/** "1:02:21.5" → 3,741,500; "62:21" → 3,741,000; "95.25" → 95,250; `null` when unreadable. */
export function parseChapterTime(text: string): number | null {
    const trimmed = text.trim();
    if (!/^\d+(:\d{1,2}){0,2}(\.\d+)?$/.test(trimmed)) return null;
    const [whole, fraction = ""] = trimmed.split(".");
    const parts = whole.split(":").map(Number);
    // Only the leading part may run past 59 ("62:21" is 62 minutes)
    if (parts.slice(1).some((part) => part >= 60)) return null;
    const seconds = parts.reduce((total, part) => total * 60 + part, 0);
    return seconds * 1000 + Math.round(Number(`0.${fraction || "0"}`) * 1000);
}

/** 12,592,400 → "3:29:52.4"; whole seconds drop the fraction. */
export function formatChapterTime(ms: number): string {
    const total = Math.max(0, Math.floor(ms / 1000));
    const h = Math.floor(total / 3600);
    const m = Math.floor((total % 3600) / 60);
    const s = String(total % 60).padStart(2, "0");
    const tenths = Math.floor((ms % 1000) / 100);
    const clock = h > 0 ? `${h}:${String(m).padStart(2, "0")}:${s}` : `${m}:${s}`;
    return tenths > 0 ? `${clock}.${tenths}` : clock;
}

export type MoveResult = { chapters: Chapter[] } | { error: string };

/**
 * Moves chapter `index` to start at `startMs`; the chapter before it ends
 * there. Refused for the first chapter, and for a start that is not after
 * the previous chapter's start or not before this chapter's end.
 */
export function moveChapterStart(chapters: Chapter[], index: number, startMs: number): MoveResult {
    if (index === 0) return { error: "The first chapter always starts at 0:00." };
    const chapter = chapters[index];
    const previous = chapters[index - 1];
    if (!chapter || !previous) return { error: "There is no such chapter." };
    if (startMs <= previous.start_time_ms) {
        return { error: `It has to come after ${formatChapterTime(previous.start_time_ms)}, where the chapter before starts.` };
    }
    if (startMs >= chapter.end_time_ms) {
        return { error: `It has to come before ${formatChapterTime(chapter.end_time_ms)}, where this chapter ends.` };
    }
    if (startMs === chapter.start_time_ms) return { chapters };
    return {
        chapters: chapters.map((c, i) =>
            i === index - 1
                ? { ...c, end_time_ms: startMs }
                : // A moved start is the user's own, so its placement mark goes
                  i === index
                  ? { ...c, start_time_ms: startMs, placement: undefined }
                  : c
        ),
    };
}

/** A pause in the book, as the waveform view reports it. */
export interface PauseSpan {
    start_ms: number;
    end_ms: number;
}

/** How long before speech resumes a chapter starts in a pause (`LEAD_IN_MS` in `ffmpeg/silence.rs`). */
export const PAUSE_LEAD_IN_MS = 200;

/**
 * Where a chapter would start in the pause nearest `atMs`: just before the
 * speech after it resumes. `null` when no pause starts a chapter within
 * `reachMs`, or none lies between `minMs` and `maxMs` (exclusive).
 */
export function snapToPause(
    pauses: PauseSpan[],
    atMs: number,
    minMs: number,
    maxMs: number,
    reachMs = 5_000
): number | null {
    let best: number | null = null;
    for (const pause of pauses) {
        const start = Math.max(pause.start_ms, pause.end_ms - PAUSE_LEAD_IN_MS);
        if (start <= minMs || start >= maxMs || Math.abs(start - atMs) > reachMs) continue;
        if (best === null || Math.abs(start - atMs) < Math.abs(best - atMs)) best = start;
    }
    return best;
}

/** The file that book time `ms` falls in, and how far into it; the last file's end counts as in it. */
export function locateInBook(
    files: { path: string; duration_ms: number }[],
    ms: number
): { path: string; offsetMs: number; fileStartMs: number } | null {
    let start = 0;
    for (const [i, file] of files.entries()) {
        const end = start + file.duration_ms;
        if (ms < end || i === files.length - 1) {
            return { path: file.path, offsetMs: Math.max(0, Math.min(ms - start, file.duration_ms)), fileStartMs: start };
        }
        start = end;
    }
    return null;
}
