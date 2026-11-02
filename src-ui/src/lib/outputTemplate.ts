import type { AudioBook } from "@/types/models";

/**
 * Output naming: a template such as `{author}/{series}/{title}` decides the
 * folders and file name a book is written to, inside the output folder.
 *
 * Kept free of runtime imports so `npm test` can load it under plain Node.
 */

export const DEFAULT_OUTPUT_TEMPLATE = "{title}";

export const TEMPLATE_FIELDS = [
    { token: "author", label: "Author" },
    { token: "title", label: "Title" },
    { token: "series", label: "Series" },
    { token: "series_index", label: "Book number" },
    { token: "narrator", label: "Narrator" },
    { token: "year", label: "Year" },
] as const;

export type TemplateField = (typeof TEMPLATE_FIELDS)[number]["token"];
export type TemplateValues = Partial<Record<TemplateField, string>>;

export const TEMPLATE_PRESETS = [
    { label: "Title", template: "{title}" },
    { label: "Author / Title", template: "{author}/{title}" },
    // The folder layout Audiobookshelf scans: Author/Series/"2 - Title"/.
    { label: "Author / Series / Book", template: "{author}/{series}/{series_index} - {title}/{title}" },
] as const;

const TOKEN = /\{(\w+)\}/g;
const ILLEGAL = /[\\/:*?"<>|]/g;
// Separators a missing field leaves stranded at either end of a name.
const EDGE = /^[\s\-–—_.,;:]+|[\s\-–—_.,;:]+$/g;
const MAX_SEGMENT = 180;

export function templateValues(book: AudioBook): TemplateValues {
    const m = book.output_metadata ?? {};
    return {
        author: m.artist || m.album_artist || book.author,
        title: m.title || book.title,
        series: m.series,
        series_index: m.series_index != null ? String(m.series_index) : undefined,
        narrator: m.narrator,
        year: m.year ? String(m.year) : undefined,
    };
}

/**
 * `<folder>/<rendered template>.m4b`.
 *
 * A field with no value drops out together with the separators around it, so
 * one template works for books with and without a series: `{series_index} -
 * {title}` becomes just the title, and an empty `{series}/` folder vanishes.
 * Values can never add folders or escape the output folder.
 */
export function renderOutputPath(folder: string, template: string | undefined, values: TemplateValues): string {
    const fallback = cleanSegment((values.title ?? "").replace(ILLEGAL, "_")) || "audiobook";
    const source = (template ?? "").trim().replace(/\.m4b$/i, "") || DEFAULT_OUTPUT_TEMPLATE;

    const segments = source.split(/[\\/]+/).map((segment) =>
        cleanSegment(
            segment
                .replace(TOKEN, (raw, name: string) => {
                    const key = name.toLowerCase();
                    return isField(key) ? (values[key] ?? "").replace(ILLEGAL, "_") : raw;
                })
                .replace(ILLEGAL, "_")
        )
    );
    // The last segment names the file; when it renders empty the book would be
    // named after its folder, so fall back to the title instead.
    const file = segments.pop() || fallback;
    const folders = segments.filter(Boolean);

    return [folder.replace(/[\\/]+$/, ""), ...folders, `${file}.m4b`].join("/");
}

export function outputPathForBook(folder: string, template: string | undefined, book: AudioBook): string {
    return renderOutputPath(folder, template, templateValues(book));
}

/** `Book.m4b` → `Book (2).m4b`, for two books that would land on one path. */
export function numberedPath(path: string, n: number): string {
    return path.replace(/(\.m4b)?$/i, ` (${n})$1`);
}

/** `{author}/{title}` → `Author / Title`, for showing a template in a sentence. */
export function describeTemplate(template: string | undefined): string {
    const source = (template ?? "").trim() || DEFAULT_OUTPUT_TEMPLATE;
    return source
        .replace(TOKEN, (raw, name: string) => TEMPLATE_FIELDS.find((f) => f.token === name.toLowerCase())?.label ?? raw)
        .replace(/\s*[\\/]+\s*/g, " / ");
}

/** Tokens the template uses that are not fields, so Settings can flag typos. */
export function unknownFields(template: string | undefined): string[] {
    return [...(template ?? "").matchAll(TOKEN)].map((m) => m[1]).filter((name) => !isField(name.toLowerCase()));
}

function isField(name: string): name is TemplateField {
    return TEMPLATE_FIELDS.some((f) => f.token === name);
}

function cleanSegment(segment: string): string {
    return segment
        .replace(/\(\s*\)|\[\s*\]/g, "") // "({year})" with no year
        .replace(/\s+/g, " ")
        .replace(/( [-–—])+ /g, (run) => ` ${run.trim()[0]} `) // "Author -  - Title"
        .replace(EDGE, "")
        .slice(0, MAX_SEGMENT)
        .replace(EDGE, "");
}
