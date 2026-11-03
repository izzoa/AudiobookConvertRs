import { test } from "node:test";
import assert from "node:assert/strict";
import { formatChapterTime, moveChapterStart, parseChapterTime } from "../src/lib/chapterTime.ts";
import type { Chapter } from "../src/types/models.ts";

const chapter = (index: number, start: number, end: number, placement?: Chapter["placement"]): Chapter => ({
    index,
    title: `Chapter ${index + 1}`,
    start_time_ms: start,
    end_time_ms: end,
    auto_detected: false,
    ...(placement ? { placement } : {}),
});

// 0:00, 1:09:57, 2:19:55, 3:29:52 (at a pause), ending at 4:39:50
const chapters = [
    chapter(0, 0, 4_197_000),
    chapter(1, 4_197_000, 8_395_000),
    chapter(2, 8_395_000, 12_592_000),
    chapter(3, 12_592_000, 16_790_000, "pause"),
];

test("typed times are read in every accepted form", () => {
    assert.equal(parseChapterTime("1:02:21.5"), 3_741_500);
    assert.equal(parseChapterTime("62:21"), 3_741_000);
    assert.equal(parseChapterTime("95.25"), 95_250);
    assert.equal(parseChapterTime(" 0:05 "), 5_000);
});

test("text that is not a time is not read", () => {
    for (const text of ["soon", "", "1:2:3:4", "1:60", "1:02:75", "-5", "1:02:"]) {
        assert.equal(parseChapterTime(text), null, text);
    }
});

test("start times are shown the way they are typed", () => {
    assert.equal(formatChapterTime(0), "0:00");
    assert.equal(formatChapterTime(95_250), "1:35.2");
    assert.equal(formatChapterTime(12_592_000), "3:29:52");
    assert.equal(parseChapterTime(formatChapterTime(3_741_500)), 3_741_500);
});

test("moving a start ends the chapter before there and clears its mark", () => {
    const result = moveChapterStart(chapters, 3, parseChapterTime("3:30:05")!);
    assert.ok("chapters" in result);
    assert.equal(result.chapters[3].start_time_ms, 12_605_000);
    assert.equal(result.chapters[3].placement, undefined);
    assert.equal(result.chapters[2].end_time_ms, 12_605_000);
    assert.equal(result.chapters[2].start_time_ms, 8_395_000);
    assert.equal(result.chapters[3].end_time_ms, 16_790_000);
    // The original list is untouched
    assert.equal(chapters[3].placement, "pause");
});

test("a start that does not fit is refused with a reason", () => {
    assert.deepEqual(moveChapterStart(chapters, 3, parseChapterTime("2:10:00")!), {
        error: "It has to come after 2:19:55, where the chapter before starts.",
    });
    assert.deepEqual(moveChapterStart(chapters, 3, parseChapterTime("4:40:00")!), {
        error: "It has to come before 4:39:50, where this chapter ends.",
    });
    assert.deepEqual(moveChapterStart(chapters, 0, 1_000), { error: "The first chapter always starts at 0:00." });
});

test("typing the same start changes nothing", () => {
    const result = moveChapterStart(chapters, 3, 12_592_000);
    assert.ok("chapters" in result);
    assert.equal(result.chapters, chapters);
});
