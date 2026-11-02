/**
 * Dev-only stand-in for the Tauri backend, enabled with `?mock` on the Vite
 * dev server. A cold `cargo build` is minutes long, which is too slow a loop
 * for laying out panes; this makes the UI previewable in a plain browser.
 *
 * It is imported dynamically from main.tsx and never reaches a release build.
 */
import type { AudioBook, AudioFile, Chapter, LookupCandidate, LookupRecord, Metadata, UserSettings } from "@/types/models";

const FIXTURE_FILES = [
    ["01 - Dune - Book One.mp3", 2_892_000],
    ["02 - Arrival on Arrakis.mp3", 3_063_000],
    ["03 - The Desert Planet.mp3", 2_695_000],
    ["04 - Sietch Tabr.mp3", 2_850_000],
    ["05 - Water of Life.mp3", 3_138_000],
    ["06 - The Gom Jabbar.mp3", 2_410_000],
    ["07 - Muad'Dib.mp3", 2_980_000],
    ["08 - The Fremen.mp3", 3_201_000],
    ["09 - Spice and Sand.mp3", 2_744_000],
    ["10 - The Reckoning.mp3", 3_410_000],
] as const;

function makeFile(name: string, durationMs: number, index: number, folder = ""): AudioFile {
    return {
        path: `/Users/you/Audiobooks/Dune/${folder}${name}`,
        format: "mp3",
        duration_ms: durationMs,
        bitrate_kbps: 128,
        sample_rate_hz: 44100,
        channels: 2,
        file_size_bytes: Math.round((durationMs / 1000) * 16000),
        metadata: { title: name.replace(/^\d+ - /, "").replace(/\.mp3$/, ""), track_number: index + 1 },
        has_artwork: index === 0,
    };
}

function makeChapters(files: AudioFile[]): Chapter[] {
    let cursor = 0;
    return files.map((file, index) => {
        const chapter: Chapter = {
            index,
            title: file.metadata.title ?? `Chapter ${index + 1}`,
            start_time_ms: cursor,
            end_time_ms: cursor + file.duration_ms,
            auto_detected: true,
        };
        cursor += file.duration_ms;
        return chapter;
    });
}

/** `discs` splits the fixture across CD1/ and CD2/, the shape a folder import often has. */
function makeBook(discs = false): AudioBook {
    const files = FIXTURE_FILES.map(([name, duration], i) =>
        makeFile(name, duration, i, discs ? (i < 5 ? "CD1/" : "CD2/") : "")
    );
    const chapters = makeChapters(files);
    return {
        id: "mock-book",
        title: "Dune",
        author: "Frank Herbert",
        files,
        chapters,
        output_metadata: {
            title: "Dune",
            artist: "Frank Herbert",
            narrator: "Simon Vance",
            series: "Dune Chronicles",
            series_index: "1",
            year: 1965,
            genre: "Science Fiction",
        },
        total_duration_ms: files.reduce((sum, f) => sum + f.duration_ms, 0),
        total_size_bytes: files.reduce((sum, f) => sum + f.file_size_bytes, 0),
        created_at: new Date().toISOString(),
        modified_at: new Date().toISOString(),
        is_dirty: false,
    };
}

/**
 * `?mock=library`: Choose folder picks a library of several books, so the
 * batch page can be laid out. Each entry is one book folder.
 */
const LIBRARY_ROOT = "/Users/you/Audiobooks/Library";
const LIBRARY: { folder: string; title: string; author?: string; narrator?: string; fails?: string }[] = [
    { folder: "Herbert/Dune", title: "Dune", author: "Frank Herbert", narrator: "Simon Vance" },
    { folder: "Herbert/Dune Messiah", title: "Dune Messiah", author: "Frank Herbert", narrator: "Scott Brick" },
    { folder: "Herbert/Children of Dune", title: "Children of Dune", author: "Frank Herbert", narrator: "Scott Brick" },
    { folder: "Le Guin/Earthsea", title: "A Wizard of Earthsea", author: "Ursula K. Le Guin", narrator: "Kobna Holdbrook-Smith" },
    { folder: "Unsorted/Tape rips", title: "Tape rips" },
    { folder: "Unsorted/Broken download", title: "Broken download", fails: "Could not parse audio format: 03.mp3" },
];

let mockBookId = 1;
function makeLibraryBook(entry: (typeof LIBRARY)[number]): AudioBook {
    const base = makeBook();
    const files = base.files.slice(0, 6).map((f) => ({ ...f, path: `${LIBRARY_ROOT}/${entry.folder}/${f.path.split("/").pop()}` }));
    return {
        ...base,
        id: `mock-book-${mockBookId++}`,
        title: entry.title,
        author: entry.author,
        files,
        chapters: makeChapters(files),
        total_duration_ms: files.reduce((sum, f) => sum + f.duration_ms, 0),
        total_size_bytes: files.reduce((sum, f) => sum + f.file_size_bytes, 0),
        output_metadata: { title: entry.title, artist: entry.author, narrator: entry.narrator },
    };
}

/**
 * Find details fixtures, shaped like real Audible and Audnexus answers.
 * "Dune" offers its chapter titles on the book's own chapters; "Dune Messiah"
 * is a different edition, so its chapters are unavailable.
 */
const COVER = "https://m.media-amazon.com/images/I/41dcXp9bIoL._SL500_.jpg";
const AUDIBLE_RESULTS: LookupCandidate[] = [
    { source: "audible", id: "B002V1OF70", title: "Dune", subtitle: "Book One in the Dune Chronicles", authors: ["Frank Herbert"], narrators: ["Scott Brick", "Orlagh Cassidy", "Euan Morton", "Simon Vance"], series: "Dune", series_index: "1", runtime_ms: 1_265 * 60_000, year: 2007, cover_url: COVER },
    { source: "audible", id: "B002V8H660", title: "Dune Messiah", subtitle: "Book Two in the Dune Chronicles", authors: ["Frank Herbert"], narrators: ["Scott Brick", "Euan Morton", "Simon Vance", "Katherine Kellgren"], series: "Dune", series_index: "2", runtime_ms: 537 * 60_000, year: 2007, cover_url: COVER },
    { source: "audible", id: "B002V5B5JA", title: "Children of Dune", subtitle: null, authors: ["Frank Herbert"], narrators: ["Scott Brick", "Ilyana Kadushin"], series: "Dune", series_index: "3", runtime_ms: 1_074 * 60_000, year: 2007, cover_url: COVER },
];
const OPEN_LIBRARY_RESULTS: LookupCandidate[] = [
    { source: "open_library", id: "/works/OL893415W", title: "Dune", subtitle: null, authors: ["Frank Herbert"], narrators: [], series: null, series_index: null, runtime_ms: null, year: 1965, cover_url: null },
];

function mockRecord(id: string, target: AudioBook | null | undefined): LookupRecord {
    const found = AUDIBLE_RESULTS.find((c) => c.id === id) ?? OPEN_LIBRARY_RESULTS.find((c) => c.id === id) ?? AUDIBLE_RESULTS[0];
    const fields: Metadata = found.source === "open_library"
        ? { title: found.title, artist: found.authors.join(", "), year: found.year ?? undefined, publisher: "Chilton Books", genre: "Science fiction" }
        : {
              title: found.title,
              artist: found.authors.join(", "),
              narrator: found.narrators.join(", "),
              series: found.series ?? undefined,
              series_index: found.series_index ?? undefined,
              year: found.year ?? undefined,
              genre: "Literature & Fiction",
              publisher: "Macmillan Audio",
              language: "English",
              copyright: "© 1965",
              description: "A sample summary.\n\nSecond paragraph.",
              asin: found.id,
          };
    const current = target?.chapters ?? [];
    const chapters: LookupRecord["chapters"] =
        found.source === "open_library"
            ? { kind: "unavailable", reason: "Open Library has no chapter lists." }
            : found.id === "B002V1OF70" && current.length > 0
              ? { kind: "titles_only", chapters: current.map((c, i) => ({ ...c, title: i === 0 ? "Opening Credits" : `Book One: Part ${i}`, auto_detected: false })) }
              : { kind: "unavailable", reason: `The found edition runs 8h 57m with 28 chapters; this book runs 8h 9m with ${current.length}.` };
    return { source: found.source, id: found.id, fields, cover_url: found.cover_url, runtime_ms: found.runtime_ms, chapters };
}

const SETTINGS: UserSettings = {
    default_bitrate_kbps: 64,
    default_output_directory: "/Users/you/Audiobooks/out",
    output_template: "{title}",
    audible_region: "us",
    embed_chapters_by_default: true,
    copy_metadata_by_default: true,
    embed_artwork_by_default: true,
    overwrite_existing_by_default: false,
    auto_detect_artwork: true,
    scan_for_cover_images: true,
    concurrent_jobs: 2,
    theme: "dark",
    window_width: 1200,
    window_height: 800,
    show_advanced_options: false,
    confirm_overwrite: true,
    notification_sound: false,
    recent_files: [],
    max_recent_files: 10,
    log_level: "info",
    default_quality_preset: "medium",
    default_encoding_mode: "vbr",
    normalization_target_lufs: -18,
    normalization_target_true_peak: -2,
    normalization_target_lra: 7,
    normalization_two_pass: true,
};

export function installMockBackend() {
    let book: AudioBook | null = null;
    const books = new Map<string, AudioBook>();
    let jobs: any[] = [];
    let callbackId = 1;
    // Only Find details applies are undoable here, so its review can be checked end to end.
    const undoStack: { before: AudioBook; description: string }[] = [];

    const handlers: Record<string, (args: any) => unknown> = {
        get_settings: () => SETTINGS,
        get_default_settings: () => SETTINGS,
        // Kept, so a saved setting (like the naming template) shows up in the book flow.
        update_settings: ({ settings }: any) => {
            Object.assign(SETTINGS, settings);
            return null;
        },
        reset_settings: () => null,
        check_ffmpeg_available: () => true,
        get_ffmpeg_version: () => "ffmpeg version 7.1 (mock)",
        get_ffmpeg_info: () => ({
            available: true,
            path: "/opt/homebrew/bin/ffmpeg",
            version: "7.1",
            supports_libfdk_aac: true,
        }),

        can_undo: () => undoStack.length > 0,
        can_redo: () => false,
        get_undo_description: () => undoStack.at(-1)?.description ?? null,
        get_redo_description: () => null,
        undo: () => {
            const last = undoStack.pop();
            if (!last) return null;
            books.set(last.before.id, last.before);
            if (book?.id === last.before.id) book = last.before;
            return last.description;
        },

        audible_search: ({ title }: { title?: string }) =>
            AUDIBLE_RESULTS.filter((c) => !title || c.title.toLowerCase().includes(String(title).toLowerCase().split(" ")[0])),
        open_library_search: () => OPEN_LIBRARY_RESULTS,
        lookup_record: ({ id, audiobookId }: { id: string; audiobookId: string }) =>
            mockRecord(id, books.get(audiobookId) ?? book),
        apply_lookup: ({ audiobookId, source, fields, chapters }: { audiobookId: string; source: string; fields: Metadata; chapters: Chapter[] | null }) => {
            const target = books.get(audiobookId) ?? book;
            if (!target) return book;
            const defined = Object.fromEntries(Object.entries(fields).filter(([, v]) => v != null));
            // The real backend downloads the cover; the mock keeps the current one.
            const updated: AudioBook = {
                ...target,
                output_metadata: { ...target.output_metadata, ...defined },
                chapters: chapters ?? target.chapters,
            };
            undoStack.push({ before: target, description: `Apply details from ${source === "audible" ? "Audible" : "Open Library"}` });
            books.set(updated.id, updated);
            if (book?.id === updated.id) book = updated;
            return updated;
        },

        get_active_audiobook: () => book,
        group_paths_into_books: ({ paths }: any) =>
            paths.includes(LIBRARY_ROOT)
                ? LIBRARY.map((e) => ({ name: e.folder.split("/").pop(), folder: `${LIBRARY_ROOT}/${e.folder}`, files: [`${LIBRARY_ROOT}/${e.folder}`] }))
                : [{ name: "Dune", folder: null, files: paths }],
        // Choose folder passes one directory; files arrive as a list.
        create_audiobook_from_files: ({ paths, activate }: any) => {
            const first: string = paths?.[0] ?? "";
            const entry = LIBRARY.find((e) => {
                const folder = `${LIBRARY_ROOT}/${e.folder}`;
                return first === folder || first.startsWith(`${folder}/`);
            });
            if (entry?.fails) throw entry.fails;
            const made = entry
                ? makeLibraryBook(entry)
                : makeBook(Array.isArray(paths) && paths.length === 1 && !/\.\w+$/.test(paths[0]));
            books.set(made.id, made);
            if (activate !== false) book = made;
            return made;
        },
        set_active_audiobook: ({ audiobookId }: any) => {
            book = books.get(audiobookId) ?? book;
            return null;
        },
        existing_paths: ({ paths }: any) => paths.filter((p: string) => p.includes("Children of Dune")),
        // The fixture books have no CUE sheet beside them.
        detect_cue_file: () => null,
        clear_current_audiobook: () => ((book = null), null),
        load_audiobook_file: () => (book = makeBook()),

        update_audiobook_metadata: ({ audiobookId, metadata }: any) => {
            const target = books.get(audiobookId) ?? book;
            if (target) {
                const updated = { ...target, output_metadata: metadata, title: metadata.title ?? target.title };
                books.set(updated.id, updated);
                if (book?.id === updated.id) book = updated;
                return updated;
            }
            return book;
        },
        set_chapters: ({ chapters }: any) => {
            if (book) book = { ...book, chapters };
            return book;
        },
        generate_chapters_from_source: () => {
            if (book) book = { ...book, chapters: makeChapters(book.files) };
            return book;
        },
        auto_create_chapters: ({ count }: any) => {
            if (!book) return null;
            const span = Math.floor(book.total_duration_ms / count);
            book = {
                ...book,
                chapters: Array.from({ length: count }, (_, i) => ({
                    index: i,
                    title: `Chapter ${i + 1}`,
                    start_time_ms: i * span,
                    end_time_ms: (i + 1) * span,
                    auto_detected: true,
                })),
            };
            return book;
        },
        discover_artwork: () => [],
        clear_artwork: () => {
            if (book) {
                const { artwork_data: _omit, ...rest } = book.output_metadata;
                book = { ...book, output_metadata: rest };
            }
            return book;
        },
        get_chapter_regex_presets: () => [
            { name: "Strip track numbers", description: "01 - Title → Title", pattern: "^\\d+\\s*[-.]\\s*", replacement: "" },
        ],
        export_chapters: () => "",

        list_recent_projects: () => [
            { path: "/Users/you/Projects/neuromancer.abcp", title: "Neuromancer", last_opened: new Date().toISOString(), exists: true },
            { path: "/Users/you/Projects/hyperion.abcp", title: "Hyperion", last_opened: new Date().toISOString(), exists: true },
        ],
        save_project: () => null,
        clear_project: () => null,

        // Converting seeds a running job beside finished and failed history, so
        // every job state can be laid out after one press of Convert.
        list_conversion_jobs: () => jobs,
        get_conversion_queue_state: () => ({
            state: jobs.some((j) => j.status === "running") ? "running" : "stopped",
            worker_count: 2,
            active_jobs: jobs.filter((j) => j.status === "running").length,
            pending_jobs: jobs.filter((j) => j.status === "queued").length,
        }),
        enqueue_conversion: ({ audiobookId, settings }: any) => {
            const libraryBook = audiobookId?.startsWith("mock-book-") ? books.get(audiobookId) : undefined;
            if (libraryBook) {
                const running = jobs.some((j) => j.status === "running");
                jobs = [
                    ...jobs,
                    {
                        id: `job-${audiobookId}`,
                        audiobook: libraryBook,
                        settings,
                        status: running ? "queued" : "running",
                        progress_percent: running ? 0 : 12,
                        current_file_index: 0,
                        total_files: libraryBook.files.length,
                        created_at: new Date().toISOString(),
                    },
                ];
                return `job-${audiobookId}`;
            }
            if (!book) return null;
            const now = Date.now();
            const job = (id: string, status: string, extra: Record<string, unknown>) => ({
                id,
                audiobook: book,
                settings,
                status,
                progress_percent: 0,
                current_file_index: 0,
                total_files: book!.files.length,
                created_at: new Date(now - 60_000).toISOString(),
                ...extra,
            });
            jobs = [
                job("mock-running", "running", {
                    started_at: new Date(now - 30_000).toISOString(),
                    progress_percent: 42,
                    progress: {
                        job_id: "mock-running",
                        file_index: 4,
                        current_file: "05 - Water of Life.mp3",
                        file_progress: 60,
                        total_progress: 42,
                        time_remaining_sec: 754,
                        speed: 18.4,
                        stage: "encoding",
                    },
                }),
                job("mock-queued", "queued", { audiobook: { ...book, title: "Dune Messiah", output_metadata: { ...book.output_metadata, title: "Dune Messiah" } } }),
                job("mock-done", "complete", {
                    audiobook: { ...book, title: "Children of Dune", output_metadata: { ...book.output_metadata, title: "Children of Dune" } },
                    progress_percent: 100,
                    output_path: "/Users/you/Audiobooks/out/Children of Dune.m4b",
                    started_at: new Date(now - 3_600_000).toISOString(),
                    completed_at: new Date(now - 3_000_000).toISOString(),
                }),
                job("mock-failed", "error", {
                    audiobook: { ...book, title: "God Emperor of Dune", output_metadata: { ...book.output_metadata, title: "God Emperor of Dune" } },
                    error_message: "FFmpeg exited with status 1: Permission denied writing /Volumes/Archive/God Emperor of Dune.m4b",
                }),
            ];
            return "mock-running";
        },
        cancel_conversion: () => null,
        retry_conversion: () => null,
        remove_conversion_job: ({ jobId }: any) => {
            jobs = jobs.filter((j) => j.id !== jobId);
            return null;
        },
        start_conversion_queue: () => null,
        stop_conversion_queue: () => null,
        clear_completed_jobs: () => {
            jobs = jobs.filter((j) => j.status === "running" || j.status === "queued");
            return null;
        },

        dry_run_conversion: ({ settings }: any) => ({
            title: book?.title ?? "Audiobook",
            input_duration_ms: book?.total_duration_ms ?? 0,
            input_file_count: book?.files.length ?? 0,
            input_size_bytes: book?.total_size_bytes ?? 0,
            estimated_output_size_bytes: Math.round(
                ((book?.total_duration_ms ?? 0) / 1000) * ((settings?.bitrate_kbps ?? 64) * 125)
            ),
            estimated_time_seconds: Math.round((book?.total_duration_ms ?? 0) / 1000 / 18),
            output_path: settings?.output_path ?? "",
            output_exists: false,
            chapter_count: book?.chapters.length ?? 0,
            has_artwork: !!book?.output_metadata.artwork_data,
            warnings: book?.output_metadata.artwork_data ? [] : ["No artwork available to embed"],
        }),
        get_quality_preset_info: () => [
            { preset: "high", name: "High", description: "Music-grade", bitrate_kbps: 128, sample_rate_hz: 44100, is_vbr: true },
            { preset: "medium", name: "Medium", description: "Spoken word", bitrate_kbps: 64, sample_rate_hz: 44100, is_vbr: true },
            { preset: "low", name: "Low", description: "Smallest", bitrate_kbps: 32, sample_rate_hz: 22050, is_vbr: true },
        ],
        list_presets: () => [],

        get_playback_status: () => ({ state: "stopped", current_path: null, position_ms: 0, duration_ms: 0, volume: 1 }),

        pick_files: () => FIXTURE_FILES.map(([name]) => `/Users/you/Audiobooks/Dune/${name}`),
        pick_directory: () =>
            new URLSearchParams(window.location.search).get("mock") === "library" ? LIBRARY_ROOT : "/Users/you/Audiobooks/Dune",
        pick_output_file: () => "/Users/you/Audiobooks/out/Dune.m4b",
        pick_artwork_file: () => null,
        pick_open_project_file: () => null,
        pick_save_project_file: () => null,
    };

    (window as any).__TAURI_INTERNALS__ = {
        transformCallback(callback: (payload: unknown) => void) {
            const id = callbackId++;
            (window as any)[`_${id}`] = callback;
            return id;
        },
        async invoke(cmd: string, args: any) {
            if (cmd.startsWith("plugin:event|")) return 0;
            const handler = handlers[cmd];
            if (!handler) {
                console.warn(`[mock] unhandled command: ${cmd}`, args);
                return null;
            }
            return handler(args ?? {});
        },
        metadata: {
            currentWindow: { label: "main" },
            currentWebview: { label: "main", windowLabel: "main" },
        },
        convertFileSrc: (path: string) => path,
    };

    // The event plugin reads its own global, separate from __TAURI_INTERNALS__.
    (window as any).__TAURI_EVENT_PLUGIN_INTERNALS__ = {
        unregisterListener() {
            /* nothing is ever emitted, so there is nothing to detach */
        },
    };

    console.info("[mock] Tauri backend mocked — UI preview only.");
}
