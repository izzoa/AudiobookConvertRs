import { test } from "node:test";
import assert from "node:assert/strict";
import {
    applyPayload,
    initialSelection,
    isSelectable,
    reviewRows,
    type BookSnapshot,
} from "../src/lib/lookupReview.ts";
import type { Chapter, LookupRecord } from "../src/types/models.ts";

const chapter = (index: number, title: string, auto_detected = true): Chapter => ({
    index,
    title,
    start_time_ms: index * 1000,
    end_time_ms: (index + 1) * 1000,
    auto_detected,
});

const book: BookSnapshot = {
    metadata: { title: "Dune Messiah", year: 1969 },
    hasCover: false,
    chapters: [chapter(0, "Track 01"), chapter(1, "Track 02")],
};

const record: LookupRecord = {
    source: "audible",
    id: "B002V8H660",
    fields: {
        title: "Dune Messiah",
        artist: "Frank Herbert",
        narrator: "Scott Brick, Euan Morton",
        series: "Dune",
        series_index: "2",
        year: 2007,
        asin: "B002V8H660",
        description: null as unknown as undefined,
    },
    cover_url: "https://m.media-amazon.com/images/I/81VCyFRJT2L.jpg",
    runtime_ms: 32_269_161,
    chapters: { kind: "exact", chapters: [chapter(0, "Opening Credits", false), chapter(1, "Book One", false)] },
};

const row = (key: string) => reviewRows(book, record).find((r) => r.key === key);

test("an empty field starts selected", () => {
    assert.equal(row("narrator")?.state, "selected");
    assert.equal(row("narrator")?.current, "");
});

test("a value the book already has starts unselected", () => {
    assert.deepEqual(
        { state: row("year")?.state, current: row("year")?.current, found: row("year")?.found },
        { state: "unselected", current: "1969", found: "2007" }
    );
});

test("an identical value is unchanged and cannot be ticked", () => {
    const title = row("title")!;
    assert.equal(title.state, "unchanged");
    assert.equal(isSelectable(title), false);
    assert.equal(initialSelection(reviewRows(book, record)).has("title"), false);
});

test("fields the source does not provide get no row", () => {
    assert.equal(row("description"), undefined);
    assert.equal(row("genre"), undefined);
});

test("found chapters replace generated ones by default, not hand-edited ones", () => {
    assert.equal(row("chapters")?.state, "selected");
    assert.equal(row("chapters")?.found, "2 chapters, length matches");
    const edited = { ...book, chapters: [chapter(0, "Prologue", false), chapter(1, "Part One")] };
    assert.equal(reviewRows(edited, record).find((r) => r.key === "chapters")?.state, "unselected");
});

test("unavailable chapters show the reason and cannot be ticked", () => {
    const reason = "The found edition runs 8h 57m with 28 chapters; this book runs 9h 12m with 6.";
    const noFit: LookupRecord = { ...record, chapters: { kind: "unavailable", reason } };
    const chapters = reviewRows(book, noFit).find((r) => r.key === "chapters")!;
    assert.equal(chapters.unavailable, reason);
    assert.equal(isSelectable(chapters), false);
});

test("the cover starts selected only when the book has none", () => {
    assert.equal(row("cover")?.state, "selected");
    assert.equal(reviewRows({ ...book, hasCover: true }, record).find((r) => r.key === "cover")?.state, "unselected");
});

test("the payload carries only the ticked values", () => {
    const payload = applyPayload(record, new Set(["narrator", "series", "series_index", "chapters"]));
    assert.deepEqual(payload.fields, { narrator: "Scott Brick, Euan Morton", series: "Dune", series_index: "2" });
    assert.equal(payload.coverUrl, undefined);
    assert.equal(payload.chapters?.[0].title, "Opening Credits");

    const defaults = applyPayload(record, initialSelection(reviewRows(book, record)));
    assert.deepEqual(Object.keys(defaults.fields).sort(), ["artist", "asin", "narrator", "series", "series_index"]);
    assert.equal(defaults.coverUrl, record.cover_url);
});
