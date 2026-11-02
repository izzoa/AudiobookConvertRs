/**
 * Dev-only stand-in for the Tauri backend, enabled with `?mock` on the Vite
 * dev server. A cold `cargo build` is minutes long, which is too slow a loop
 * for laying out panes; this makes the UI previewable in a plain browser.
 *
 * It is imported dynamically from main.tsx and never reaches a release build.
 */
import type { AudioBook, AudioFile, Chapter, UserSettings } from "@/types/models";

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
            series_index: 1,
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

const SETTINGS: UserSettings = {
    default_bitrate_kbps: 64,
    default_output_directory: "/Users/you/Audiobooks/out",
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
    let jobs: any[] = [];
    let callbackId = 1;

    const handlers: Record<string, (args: any) => unknown> = {
        get_settings: () => SETTINGS,
        get_default_settings: () => SETTINGS,
        update_settings: () => null,
        reset_settings: () => null,
        check_ffmpeg_available: () => true,
        get_ffmpeg_version: () => "ffmpeg version 7.1 (mock)",
        get_ffmpeg_info: () => ({
            available: true,
            path: "/opt/homebrew/bin/ffmpeg",
            version: "7.1",
            supports_libfdk_aac: true,
        }),

        can_undo: () => false,
        can_redo: () => false,
        get_undo_description: () => null,
        get_redo_description: () => null,

        get_active_audiobook: () => book,
        // Choose folder passes one directory; files arrive as a list.
        create_audiobook_from_files: ({ paths }: any) =>
            (book = makeBook(Array.isArray(paths) && paths.length === 1 && !/\.\w+$/.test(paths[0]))),
        clear_current_audiobook: () => ((book = null), null),
        load_audiobook_file: () => (book = makeBook()),

        update_audiobook_metadata: ({ metadata }: any) => {
            if (book) book = { ...book, output_metadata: metadata, title: metadata.title ?? book.title };
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
        enqueue_conversion: ({ settings }: any) => {
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
        pick_directory: () => "/Users/you/Audiobooks/Dune",
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
