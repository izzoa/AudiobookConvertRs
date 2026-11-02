// Run with `npm test`: Node strips the types, so no test framework is needed.
import { test } from "node:test";
import assert from "node:assert/strict";
import {
    describeTemplate,
    numberedPath,
    outputPathForBook,
    renderOutputPath,
    unknownFields,
    type TemplateValues,
} from "../src/lib/outputTemplate.ts";
import type { AudioBook } from "../src/types/models.ts";

const OUT = "/Users/you/Audiobooks";
const LIBRARY = "{author}/{series}/{series_index} - {title}/{title}";
const messiah: TemplateValues = {
    author: "Frank Herbert",
    title: "Dune Messiah",
    series: "Dune",
    series_index: "2",
};

test("the default template names the file after the title", () => {
    assert.equal(renderOutputPath(OUT, undefined, messiah), `${OUT}/Dune Messiah.m4b`);
    assert.equal(renderOutputPath(`${OUT}/`, "  ", messiah), `${OUT}/Dune Messiah.m4b`);
});

test("slashes make folders", () => {
    assert.equal(
        renderOutputPath(OUT, LIBRARY, messiah),
        `${OUT}/Frank Herbert/Dune/2 - Dune Messiah/Dune Messiah.m4b`
    );
});

test("a decimal book number is kept as written", () => {
    const novella = {
        title: "The Churn",
        output_metadata: { artist: "James S. A. Corey", series: "The Expanse", series_index: "1.5" },
    } as unknown as AudioBook;
    assert.equal(
        outputPathForBook(OUT, LIBRARY, novella),
        `${OUT}/James S. A. Corey/The Expanse/1.5 - The Churn/The Churn.m4b`
    );
});

test("a missing field drops out with its separators and its folder", () => {
    const standalone = { author: "Andy Weir", title: "Project Hail Mary" };
    assert.equal(
        renderOutputPath(OUT, LIBRARY, standalone),
        `${OUT}/Andy Weir/Project Hail Mary/Project Hail Mary.m4b`
    );
    assert.equal(
        renderOutputPath(OUT, "{author} - {series} - {title} ({year})", standalone),
        `${OUT}/Andy Weir - Project Hail Mary.m4b`
    );
});

test("an empty file name falls back to the title, not the folder", () => {
    assert.equal(
        renderOutputPath(OUT, "{author}/{narrator}", { author: "Andy Weir", title: "Artemis" }),
        `${OUT}/Andy Weir/Artemis.m4b`
    );
    assert.equal(renderOutputPath(OUT, "{narrator}", {}), `${OUT}/audiobook.m4b`);
});

test("values cannot add folders or escape the output folder", () => {
    assert.equal(
        renderOutputPath(OUT, "{author}/{title}", { author: "../..", title: "AC/DC: Live" }),
        `${OUT}/AC_DC_ Live.m4b`
    );
    assert.equal(renderOutputPath(OUT, "/../{title}", { title: "Dune" }), `${OUT}/Dune.m4b`);
});

test("a typed .m4b extension is not doubled", () => {
    assert.equal(renderOutputPath(OUT, "{title}.m4b", messiah), `${OUT}/Dune Messiah.m4b`);
});

test("fields are matched case-insensitively and unknown ones stay visible", () => {
    assert.equal(renderOutputPath(OUT, "{Author}/{titel}", messiah), `${OUT}/Frank Herbert/{titel}.m4b`);
    assert.deepEqual(unknownFields("{Author}/{titel}"), ["titel"]);
});

test("numbered paths and descriptions", () => {
    assert.equal(numberedPath(`${OUT}/Dune.m4b`, 2), `${OUT}/Dune (2).m4b`);
    assert.equal(describeTemplate(LIBRARY), "Author / Series / Book number - Title / Title");
});
