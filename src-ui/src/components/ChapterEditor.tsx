import { useState, useEffect } from "react";
import { Button } from "./ui/button";
import { Input } from "./ui/input";
import { useApp } from "@/context/useApp";
import * as ChaptersApi from "@/api/chapters";
import * as FilesApi from "@/api/files";
import * as PlaybackApi from "@/api/playback";
import * as SpeechApi from "@/api/speech";
import { listen } from "@tauri-apps/api/event";
import { Progress } from "./ui/progress";
import { SpeechModelDownload } from "./settings/SpeechModelPanel";
import { formatBytes } from "@/lib/format";
import type { Chapter, ChapterPlacement, SpokenChaptersProgress } from "@/types/models";
import type { CueDetectionResult } from "@/api/files";
import { PlayIcon, StopIcon } from "@heroicons/react/20/solid";
import { Popover, MenuItem, MenuLabel, MenuSeparator } from "./ui/popover";
import { cn } from "@/lib/utils";
import { formatChapterTime, moveChapterStart, parseChapterTime, TIME_FORMATS } from "@/lib/chapterTime";
import { ChapterStartCheck } from "./ChapterStartCheck";
import { Bars2Icon, AdjustmentsHorizontalIcon, SparklesIcon, ArrowUpTrayIcon, WrenchScrewdriverIcon, ExclamationTriangleIcon } from "@heroicons/react/24/outline";
import {
    DndContext,
    closestCenter,
    KeyboardSensor,
    PointerSensor,
    useSensor,
    useSensors,
    type DragEndEvent,
} from "@dnd-kit/core";
import {
    arrayMove,
    SortableContext,
    sortableKeyboardCoordinates,
    useSortable,
    verticalListSortingStrategy,
} from "@dnd-kit/sortable";
import { CSS } from "@dnd-kit/utilities";

type ChapterFormat = "audacity" | "ffmetadata" | "csv";

/** Why a chapter is marked, for its tooltip. */
const PLACEMENT_HINT: Record<ChapterPlacement, string> = {
    pause: "Find details placed this chapter in a pause near where it expected it, as no track starts there. Play it to check.",
    estimate: "Find details placed this chapter where it expected it; no track start or pause was found nearby. Play it to check.",
};

interface SortableChapterProps {
    chapter: Chapter;
    displayIndex: number;
    formatDuration: (ms: number) => string;
    onPlay: (chapter: Chapter) => void;
    isPlaying: boolean;
    onRename: (chapter: Chapter, title: string) => void;
    /** Moves this chapter's start; resolves to why it was refused, or null. Absent for the first chapter. */
    onMoveStart?: (startMs: number) => Promise<string | null>;
    /** Opens or closes the waveform check of this chapter's start */
    onToggleCheck?: () => void;
    /** The open waveform check, shown under the row */
    check?: React.ReactNode;
}

function SortableChapter({
    chapter,
    displayIndex,
    formatDuration,
    onPlay,
    isPlaying,
    onRename,
    onMoveStart,
    onToggleCheck,
    check,
}: SortableChapterProps) {
    const [draft, setDraft] = useState<string | null>(null);
    const [timeDraft, setTimeDraft] = useState<string | null>(null);
    const [timeError, setTimeError] = useState<string | null>(null);
    const {
        attributes,
        listeners,
        setNodeRef,
        transform,
        transition,
        isDragging,
    } = useSortable({ id: chapter.index.toString() });

    const style = {
        transform: CSS.Transform.toString(transform),
        transition,
    };

    const commit = () => {
        const next = (draft ?? "").trim();
        if (next && next !== chapter.title) onRename(chapter, next);
        setDraft(null);
    };

    const cancelTime = () => {
        setTimeDraft(null);
        setTimeError(null);
    };

    // The field opens on the start as it would be typed, so saving it unchanged moves nothing
    const commitTime = async () => {
        if (timeDraft === null || !onMoveStart) return;
        if (timeDraft.trim() === formatChapterTime(chapter.start_time_ms)) return cancelTime();
        const startMs = parseChapterTime(timeDraft);
        if (startMs === null) return setTimeError(`Type the start as ${TIME_FORMATS}.`);
        const refused = await onMoveStart(startMs);
        if (refused) return setTimeError(refused);
        cancelTime();
    };

    return (
        <div
            ref={setNodeRef}
            style={style}
            className={cn(
                "border-b border-border/60",
                isDragging && "relative z-10 bg-surface shadow-lift rounded-md border-transparent"
            )}
        >
            <div
                className={cn(
                    "group flex items-center gap-2 h-[42px] px-2 text-sm",
                    !isDragging && "hover:bg-accent/50",
                    check && "bg-accent/40"
                )}
            >
                <button
                    {...attributes}
                    {...listeners}
                    aria-label="Drag to reorder"
                    className="w-5 h-7 shrink-0 flex items-center justify-center text-muted-foreground/40 group-hover:text-muted-foreground cursor-grab active:cursor-grabbing touch-none"
                >
                    <Bars2Icon className="h-4 w-4" />
                </button>

                <span className="w-7 shrink-0 text-right text-xs text-muted-foreground nums">{displayIndex + 1}</span>

                {draft === null ? (
                    <button
                        onDoubleClick={() => setDraft(chapter.title)}
                        onKeyDown={(e) => {
                            if (e.key === "Enter" || e.key === "F2") setDraft(chapter.title);
                        }}
                        className="flex-1 min-w-0 truncate text-left rounded-sm"
                        title="Double-click or press Enter to rename"
                    >
                        {chapter.title}
                    </button>
                ) : (
                    <input
                        autoFocus
                        value={draft}
                        onChange={(e) => setDraft(e.target.value)}
                        onBlur={commit}
                        onKeyDown={(e) => {
                            if (e.key === "Enter") commit();
                            if (e.key === "Escape") setDraft(null);
                        }}
                        aria-label="Chapter title"
                        className="flex-1 min-w-0 rounded-md border border-input bg-background px-2 py-1 text-sm focus:outline-none focus-visible:ring-2 focus-visible:ring-ring"
                    />
                )}

                {timeError ? (
                    <span className="min-w-0 max-w-[260px] truncate text-[11px] text-destructive" title={timeError}>
                        {timeError}
                    </span>
                ) : (
                    chapter.placement &&
                    (onToggleCheck ? (
                        <button
                            onClick={onToggleCheck}
                            className="shrink-0 text-[11px] text-warning hover:underline underline-offset-2"
                            title={`${PLACEMENT_HINT[chapter.placement]} Click to see and hear its start.`}
                        >
                            {chapter.placement === "pause" ? "at a pause" : "estimated"}
                        </button>
                    ) : (
                        <span className="shrink-0 text-[11px] text-warning" title={PLACEMENT_HINT[chapter.placement]}>
                            {chapter.placement === "pause" ? "at a pause" : "estimated"}
                        </span>
                    ))
                )}

                <span className="shrink-0 flex items-center gap-1 text-xs text-muted-foreground nums">
                    {timeDraft !== null ? (
                        <input
                            autoFocus
                            value={timeDraft}
                            onChange={(e) => {
                                setTimeDraft(e.target.value);
                                setTimeError(null);
                            }}
                            // Opens selected, so typing a time replaces the old one
                            onFocus={(e) => e.target.select()}
                            onBlur={commitTime}
                            onKeyDown={(e) => {
                                if (e.key === "Enter") commitTime();
                                if (e.key === "Escape") cancelTime();
                            }}
                            aria-label="Chapter start time"
                            aria-invalid={timeError !== null}
                            className={cn(
                                "w-[84px] rounded-md border bg-background px-1.5 py-0.5 text-xs nums focus:outline-none focus-visible:ring-2 focus-visible:ring-ring",
                                timeError ? "border-destructive" : "border-input"
                            )}
                        />
                    ) : onMoveStart ? (
                        <button
                            onClick={() => setTimeDraft(formatChapterTime(chapter.start_time_ms))}
                            title="Change where this chapter starts"
                            className="rounded-sm hover:text-foreground hover:underline decoration-dotted underline-offset-2"
                        >
                            {formatDuration(chapter.start_time_ms)}
                        </button>
                    ) : (
                        <span>{formatDuration(chapter.start_time_ms)}</span>
                    )}
                    <span>– {formatDuration(chapter.end_time_ms)}</span>
                </span>

                <button
                    onClick={(e) => { e.stopPropagation(); onPlay(chapter); }}
                    aria-label={isPlaying ? "Stop" : "Play chapter"}
                    title={isPlaying ? "Stop" : "Play chapter"}
                    className={cn(
                        "w-7 h-7 shrink-0 rounded-md flex items-center justify-center transition-opacity",
                        isPlaying ? "text-primary" : "text-muted-foreground opacity-0 group-hover:opacity-100 focus:opacity-100 hover:bg-accent"
                    )}
                >
                    {isPlaying ? <StopIcon className="h-3.5 w-3.5" /> : <PlayIcon className="h-3.5 w-3.5" />}
                </button>

                {onToggleCheck ? (
                    <button
                        onClick={onToggleCheck}
                        aria-label="Check this chapter's start"
                        aria-expanded={check ? true : false}
                        title="See and hear where this chapter starts"
                        className={cn(
                            "w-7 h-7 shrink-0 rounded-md flex items-center justify-center transition-opacity hover:bg-accent",
                            check || chapter.placement
                                ? "text-primary opacity-100"
                                : "text-muted-foreground opacity-0 group-hover:opacity-100 focus:opacity-100"
                        )}
                    >
                        <AdjustmentsHorizontalIcon className="h-4 w-4" />
                    </button>
                ) : (
                    <span className="w-7 shrink-0" />
                )}
            </div>
            {check}
        </div>
    );
}

export function ChapterEditor() {
    const { activeAudiobook, setActiveAudiobook, refreshUndoState, speechModel } = useApp();
    const [isImporting, setIsImporting] = useState(false);
    const [isExporting, setIsExporting] = useState(false);
    const [isGenerating, setIsGenerating] = useState(false);
    const [isRenaming, setIsRenaming] = useState(false);
    const [error, setError] = useState<string | null>(null);
    const [regexPresets, setRegexPresets] = useState<ChaptersApi.RegexPreset[]>([]);
    const [customPattern, setCustomPattern] = useState("");
    const [customReplacement, setCustomReplacement] = useState("");
    const [detectedCue, setDetectedCue] = useState<CueDetectionResult | null>(null);
    const [isImportingCue, setIsImportingCue] = useState(false);
    const [playingChapterIndex, setPlayingChapterIndex] = useState<number | null>(null);
    // The chapter whose start is open in the waveform check
    const [checkingIndex, setCheckingIndex] = useState<number | null>(null);
    // Listening for spoken headings: "offer" asks for the speech model first
    const [listening, setListening] = useState<"offer" | "running" | null>(null);
    const [listenProgress, setListenProgress] = useState<SpokenChaptersProgress | null>(null);

    const sensors = useSensors(
        useSensor(PointerSensor, {
            activationConstraint: {
                distance: 8,
            },
        }),
        useSensor(KeyboardSensor, {
            coordinateGetter: sortableKeyboardCoordinates,
        })
    );

    useEffect(() => {
        let unlisten: (() => void) | undefined;
        let disposed = false;
        listen<SpokenChaptersProgress>("spoken_chapters_progress", (event) => setListenProgress(event.payload)).then(
            (fn) => {
                if (disposed) fn();
                else unlisten = fn;
            }
        );
        return () => {
            disposed = true;
            unlisten?.();
        };
    }, []);

    useEffect(() => {
        async function loadPresets() {
            try {
                const presets = await ChaptersApi.getChapterRegexPresets();
                setRegexPresets(presets);
            } catch (e) {
                console.error("Failed to load presets:", e);
            }
        }
        loadPresets();
    }, []);

    useEffect(() => {
        async function detectCue() {
            if (!activeAudiobook?.files?.length) {
                setDetectedCue(null);
                return;
            }
            try {
                const paths = activeAudiobook.files.map(f => f.path);
                const result = await FilesApi.detectCueFile(paths);
                setDetectedCue(result);
            } catch (e) {
                console.error("Failed to detect CUE:", e);
                setDetectedCue(null);
            }
        }
        detectCue();
    }, [activeAudiobook?.files]);

    const findFileForChapter = (chapter: Chapter): { path: string; offsetMs: number } | null => {
        if (!activeAudiobook?.files?.length) return null;
        let cumulativeDuration = 0;
        for (const file of activeAudiobook.files) {
            if (chapter.start_time_ms < cumulativeDuration + file.duration_ms) {
                return {
                    path: file.path,
                    offsetMs: chapter.start_time_ms - cumulativeDuration
                };
            }
            cumulativeDuration += file.duration_ms;
        }
        return activeAudiobook.files.length > 0
            ? { path: activeAudiobook.files[0].path, offsetMs: chapter.start_time_ms }
            : null;
    };

    const handlePlayChapter = async (chapter: Chapter) => {
        try {
            if (playingChapterIndex === chapter.index) {
                await PlaybackApi.stopAudio();
                setPlayingChapterIndex(null);
            } else {
                const fileInfo = findFileForChapter(chapter);
                if (fileInfo) {
                    await PlaybackApi.playAudio(fileInfo.path, fileInfo.offsetMs);
                    setPlayingChapterIndex(chapter.index);
                }
            }
        } catch (e) {
            console.error("Playback error:", e);
            setError(`Could not play this chapter: ${e}`);
            setPlayingChapterIndex(null);
        }
    };

    useEffect(() => {
        const checkPlaybackStatus = async () => {
            try {
                const status = await PlaybackApi.getPlaybackStatus();
                if (status.state === 'stopped') {
                    setPlayingChapterIndex(null);
                }
            } catch (e) {
                console.error("Failed to check playback status:", e);
            }
        };
        const interval = setInterval(checkPlaybackStatus, 1000);
        return () => clearInterval(interval);
    }, []);

    if (!activeAudiobook) {
        return (
            <div className="p-8 text-center border rounded-lg bg-muted/10 text-muted-foreground">
                No audiobook selected. Load files to edit chapters.
            </div>
        );
    }

    const handleImportClick = async () => {
        setIsImporting(true);
        setError(null);

        try {
            const path = await FilesApi.pickChaptersFile();
            if (!path) {
                setIsImporting(false);
                return;
            }

            const ext = path.split('.').pop()?.toLowerCase();
            let format: ChapterFormat = "audacity";
            if (ext === "csv") format = "csv";
            else if (ext === "ffmeta" || ext === "ffmetadata") format = "ffmetadata";

            const content = await FilesApi.readTextFile(path);

            const updated = await ChaptersApi.importChapters(
                activeAudiobook.id,
                format,
                content
            );
            setActiveAudiobook(updated);
            setError(null);
        } catch (e) {
            console.error("Failed to import chapters:", e);
            setError(`Import failed: ${e}`);
        } finally {
            setIsImporting(false);
        }
    };

    const handleExport = async (format: ChapterFormat) => {
        if (!activeAudiobook) return;
        setIsExporting(true);
        setError(null);

        try {
            const ext = format === "ffmetadata" ? "ffmeta" : format === "csv" ? "csv" : "txt";
            const defaultName = `${activeAudiobook.title || "chapters"}_chapters.${ext}`;

            const path = await FilesApi.pickExportSummaryFile(defaultName);
            if (!path) {
                setIsExporting(false);
                return;
            }

            const content = await ChaptersApi.exportChapters(activeAudiobook.id, format);
            await FilesApi.writeTextFile(path, content);
            setError(null);
        } catch (e) {
            console.error("Failed to export chapters:", e);
            setError(`Export failed: ${e}`);
        } finally {
            setIsExporting(false);
        }
    };

    const handleDetectSpoken = async () => {
        if (!activeAudiobook) return;
        if (!speechModel.status?.installed) {
            setListening("offer");
            return;
        }
        setListening("running");
        setListenProgress(null);
        setError(null);
        try {
            const updated = await SpeechApi.detectSpokenChapters(activeAudiobook.id);
            setActiveAudiobook(updated);
            await refreshUndoState();
        } catch (e) {
            if (String(e) !== "cancelled") {
                console.error("Failed to detect spoken chapters:", e);
                setError(`Listening failed: ${e}`);
            }
        } finally {
            setListening(null);
            setListenProgress(null);
        }
    };

    const handleGetModelAndListen = async () => {
        if (await speechModel.download()) {
            setListening(null);
            await handleDetectSpoken();
        }
    };

    const handleGenerateFromSource = async () => {
        if (!activeAudiobook) return;
        setIsGenerating(true);
        setError(null);

        try {
            const updated = await ChaptersApi.generateChaptersFromSource(activeAudiobook.id);
            setActiveAudiobook(updated);
        } catch (e) {
            console.error("Failed to generate chapters:", e);
            setError(`Generation failed: ${e}`);
        } finally {
            setIsGenerating(false);
        }
    };

    const handleImportFromCue = async (cuePath: string) => {
        if (!activeAudiobook) return;
        setIsImportingCue(true);
        setError(null);

        try {
            const updated = await ChaptersApi.importChaptersFromCue(activeAudiobook.id, cuePath);
            setActiveAudiobook(updated);
            setDetectedCue(null);
        } catch (e) {
            console.error("Failed to import CUE:", e);
            setError(`CUE import failed: ${e}`);
        } finally {
            setIsImportingCue(false);
        }
    };

    const handlePickCueFile = async () => {
        if (!activeAudiobook) return;
        setError(null);

        try {
            const path = await FilesApi.pickCueFile();
            if (path) {
                await handleImportFromCue(path);
            }
        } catch (e) {
            console.error("Failed to pick CUE file:", e);
            setError(`Failed to pick CUE file: ${e}`);
        }
    };

    const handleApplyPreset = async (preset: ChaptersApi.RegexPreset) => {
        if (!activeAudiobook) return;
        setIsRenaming(true);
        setError(null);

        try {
            const updated = await ChaptersApi.renameChaptersWithRegex(
                activeAudiobook.id,
                preset.pattern,
                preset.replacement
            );
            setActiveAudiobook(updated);
        } catch (e) {
            console.error("Failed to rename chapters:", e);
            setError(`Rename failed: ${e}`);
        } finally {
            setIsRenaming(false);
        }
    };

    const handleApplyCustomRegex = async () => {
        if (!activeAudiobook || !customPattern) return;
        setIsRenaming(true);
        setError(null);

        try {
            const updated = await ChaptersApi.renameChaptersWithRegex(
                activeAudiobook.id,
                customPattern,
                customReplacement
            );
            setActiveAudiobook(updated);
            setCustomPattern("");
            setCustomReplacement("");
        } catch (e) {
            console.error("Failed to rename chapters:", e);
            setError(`Rename failed: ${e}`);
        } finally {
            setIsRenaming(false);
        }
    };

    const formatDuration = (ms: number) => {
        const totalSeconds = Math.floor(ms / 1000);
        const hours = Math.floor(totalSeconds / 3600);
        const minutes = Math.floor((totalSeconds % 3600) / 60);
        const seconds = totalSeconds % 60;

        if (hours > 0) {
            return `${hours}:${minutes.toString().padStart(2, '0')}:${seconds.toString().padStart(2, '0')}`;
        }
        return `${minutes}:${seconds.toString().padStart(2, '0')}`;
    };

    const validateChapters = (chapters: Chapter[]) => {
        const warnings: string[] = [];

        for (let i = 0; i < chapters.length - 1; i++) {
            if (chapters[i].end_time_ms > chapters[i + 1].start_time_ms) {
                warnings.push(`Overlap between chapter ${i + 1} and ${i + 2}`);
            }
        }

        for (let i = 0; i < chapters.length - 1; i++) {
            const gap = chapters[i + 1].start_time_ms - chapters[i].end_time_ms;
            if (gap > 1000) {
                warnings.push(`Gap of ${formatDuration(gap)} between chapter ${i + 1} and ${i + 2}`);
            }
        }

        return warnings;
    };

    const warnings = validateChapters(activeAudiobook.chapters);

    const recalculateChapterTimings = (chapters: Chapter[]): Chapter[] => {
        let currentTime = 0;
        return chapters.map((chapter, idx) => {
            const duration = chapter.end_time_ms - chapter.start_time_ms;
            const newChapter = {
                ...chapter,
                index: idx,
                start_time_ms: currentTime,
                end_time_ms: currentTime + duration,
            };
            currentTime += duration;
            return newChapter;
        });
    };

    const handleDragEnd = async (event: DragEndEvent) => {
        const { active, over } = event;

        if (!over || active.id === over.id || !activeAudiobook) {
            return;
        }

        const oldIndex = activeAudiobook.chapters.findIndex(
            (ch) => ch.index.toString() === active.id
        );
        const newIndex = activeAudiobook.chapters.findIndex(
            (ch) => ch.index.toString() === over.id
        );

        if (oldIndex === -1 || newIndex === -1) return;

        const reorderedChapters = arrayMove(activeAudiobook.chapters, oldIndex, newIndex);
        const reindexedChapters = recalculateChapterTimings(reorderedChapters);

        const previousAudiobook = activeAudiobook;
        setActiveAudiobook({ ...activeAudiobook, chapters: reindexedChapters });

        try {
            const updated = await ChaptersApi.setChapters(activeAudiobook.id, reindexedChapters);
            setActiveAudiobook(updated);
            setError(null);
        } catch (err) {
            console.error("Failed to reorder chapters:", err);
            setError(`Reorder failed: ${err}`);
            setActiveAudiobook(previousAudiobook);
        }
    };

    const chapterIds = activeAudiobook.chapters.map((ch) => ch.index.toString());
    const markedCount = activeAudiobook.chapters.filter((ch) => ch.placement).length;

    const handleMoveStart = async (index: number, startMs: number): Promise<string | null> => {
        const moved = moveChapterStart(activeAudiobook.chapters, index, startMs);
        if ("error" in moved) return moved.error;
        if (moved.chapters === activeAudiobook.chapters) return null;
        try {
            setActiveAudiobook(await ChaptersApi.setChapters(activeAudiobook.id, moved.chapters));
            await refreshUndoState();
            setError(null);
            return null;
        } catch (err) {
            console.error("Failed to move chapter start:", err);
            return String(err);
        }
    };

    const handleRename = async (chapter: Chapter, title: string) => {
        const next = activeAudiobook.chapters.map((ch) =>
            ch.index === chapter.index ? { ...ch, title } : ch
        );
        const previous = activeAudiobook;
        setActiveAudiobook({ ...activeAudiobook, chapters: next });
        try {
            setActiveAudiobook(await ChaptersApi.setChapters(activeAudiobook.id, next));
            setError(null);
        } catch (err) {
            console.error("Failed to rename chapter:", err);
            setError(`Rename failed: ${err}`);
            setActiveAudiobook(previous);
        }
    };

    return (
        <div className="h-full flex flex-col">
            <div className="shrink-0 h-11 px-3 border-b flex items-center gap-1.5 no-select">
                <Popover
                    align="start"
                    trigger={() => (
                        <Button variant="ghost" size="sm" disabled={isGenerating || isImportingCue || listening === "running"}>
                            <SparklesIcon className="w-4 h-4 mr-1.5" />
                            Generate
                        </Button>
                    )}
                >
                    {(close) => (
                        <>
                            <MenuLabel>Create chapters from</MenuLabel>
                            <MenuItem onClick={() => { close(); handleGenerateFromSource(); }}>
                                {isGenerating ? "Extracting…" : "Marks inside the files"}
                            </MenuItem>
                            <MenuItem onClick={() => { close(); handlePickCueFile(); }}>
                                A CUE sheet…
                            </MenuItem>
                            <MenuItem onClick={() => { close(); handleDetectSpoken(); }}>
                                Titles spoken in the audio
                            </MenuItem>
                            <MenuSeparator />
                            <MenuItem onClick={() => { close(); handleImportClick(); }}>
                                {isImporting ? "Importing…" : "A chapter file…"}
                            </MenuItem>
                        </>
                    )}
                </Popover>

                <Popover
                    align="start"
                    trigger={() => (
                        <Button
                            variant="ghost"
                            size="sm"
                            disabled={isExporting || activeAudiobook.chapters.length === 0}
                        >
                            <ArrowUpTrayIcon className="w-4 h-4 mr-1.5" />
                            Export
                        </Button>
                    )}
                >
                    {(close) => (
                        <>
                            <MenuLabel>Export chapter list as</MenuLabel>
                            <MenuItem onClick={() => { close(); handleExport("audacity"); }}>Audacity labels (.txt)</MenuItem>
                            <MenuItem onClick={() => { close(); handleExport("ffmetadata"); }}>FFmetadata (.ffmeta)</MenuItem>
                            <MenuItem onClick={() => { close(); handleExport("csv"); }}>CSV (.csv)</MenuItem>
                        </>
                    )}
                </Popover>

                <Popover
                    align="start"
                    className="w-[400px] p-4"
                    trigger={() => (
                        <Button variant="ghost" size="sm" disabled={activeAudiobook.chapters.length === 0}>
                            <WrenchScrewdriverIcon className="w-4 h-4 mr-1.5" />
                            Clean up titles
                        </Button>
                    )}
                >
                    <div className="space-y-4">
                        {regexPresets.length > 0 && (
                            <div>
                                <div className="mb-2 text-[11px] font-semibold uppercase tracking-wider text-muted-foreground">
                                    Presets
                                </div>
                                <div className="grid grid-cols-2 gap-2">
                                    {regexPresets.map((preset, idx) => (
                                        <Button
                                            key={idx}
                                            onClick={() => handleApplyPreset(preset)}
                                            disabled={isRenaming}
                                            size="sm"
                                            variant="secondary"
                                            title={preset.description}
                                            className="h-auto min-h-9 px-3 py-2 text-xs leading-snug whitespace-normal text-center"
                                        >
                                            {preset.name}
                                        </Button>
                                    ))}
                                </div>
                            </div>
                        )}

                        <div className="pt-4 border-t space-y-2.5">
                            <div className="text-[11px] font-semibold uppercase tracking-wider text-muted-foreground">
                                Find and replace
                            </div>
                            <Input
                                placeholder="Pattern, e.g. ^\d+ - "
                                value={customPattern}
                                onChange={(e) => setCustomPattern(e.target.value)}
                                className="font-mono text-xs"
                            />
                            <Input
                                placeholder="Replacement (blank to delete)"
                                value={customReplacement}
                                onChange={(e) => setCustomReplacement(e.target.value)}
                                className="font-mono text-xs"
                            />
                            <Button
                                onClick={handleApplyCustomRegex}
                                disabled={isRenaming || !customPattern}
                                size="sm"
                                variant="outline"
                                className="w-full"
                            >
                                {isRenaming ? "Applying…" : "Apply to all titles"}
                            </Button>
                        </div>
                    </div>
                </Popover>

                <div className="flex-1" />
                <span className="text-xs text-muted-foreground nums pr-1">
                    {activeAudiobook.chapters.length}{" "}
                    {activeAudiobook.chapters.length === 1 ? "chapter" : "chapters"}
                </span>
            </div>

            {listening && (
                <div className="shrink-0 px-3 py-2 bg-primary/[0.07] border-b text-xs">
                    {listening === "running" ? (
                        <ListeningProgress progress={listenProgress} onCancel={SpeechApi.cancelSpokenChapters} />
                    ) : speechModel.downloading ? (
                        <SpeechModelDownload progress={speechModel.progress} onCancel={speechModel.cancelDownload} />
                    ) : (
                        <div className="flex items-center justify-between gap-3">
                            <span className="text-muted-foreground">
                                Listening for chapter titles needs the speech model (
                                {formatBytes(speechModel.status?.size_bytes ?? 0)}). It runs on this computer; no audio
                                is sent anywhere.
                            </span>
                            <div className="flex shrink-0 gap-1">
                                <Button size="sm" variant="ghost" className="h-7" onClick={() => setListening(null)}>
                                    Not now
                                </Button>
                                <Button size="sm" className="h-7" onClick={handleGetModelAndListen}>
                                    {speechModel.error ? "Try again" : "Get it and listen"}
                                </Button>
                            </div>
                        </div>
                    )}
                    {listening === "offer" && speechModel.error && (
                        <p className="mt-1 text-destructive">{speechModel.error}</p>
                    )}
                </div>
            )}

            {detectedCue && (
                <div className="shrink-0 flex items-center gap-3 px-3 py-2 bg-primary/[0.07] border-b">
                    <div className="min-w-0 flex-1">
                        <p className="text-xs font-medium">
                            CUE sheet found — {detectedCue.track_count} tracks
                            {detectedCue.album_title && ` · "${detectedCue.album_title}"`}
                        </p>
                        <p className="text-[11px] text-muted-foreground truncate" title={detectedCue.cue_path}>
                            {detectedCue.cue_path.split(/[/\\]/).pop()}
                        </p>
                    </div>
                    <Button
                        onClick={() => handleImportFromCue(detectedCue.cue_path)}
                        disabled={isImportingCue}
                        size="sm"
                    >
                        {isImportingCue ? "Importing…" : "Use as chapters"}
                    </Button>
                </div>
            )}

            {markedCount > 0 && (
                <div className="shrink-0 flex items-center gap-2 px-3 py-1.5 border-b text-xs text-muted-foreground">
                    <ExclamationTriangleIcon className="w-4 h-4 text-warning shrink-0" />
                    {markedCount === 1
                        ? "1 chapter was placed without a track start. Play it to check."
                        : `${markedCount} chapters were placed without a track start. Play each to check it.`}
                </div>
            )}

            {warnings.length > 0 && (
                <div className="shrink-0 flex items-start gap-2 px-3 py-2 bg-warning/10 border-b text-xs">
                    <ExclamationTriangleIcon className="w-4 h-4 text-warning shrink-0 mt-px" />
                    <ul className="space-y-0.5">
                        {warnings.map((warning, idx) => (
                            <li key={idx}>{warning}</li>
                        ))}
                    </ul>
                </div>
            )}

            {error && (
                <div className="shrink-0 px-3 py-2 bg-destructive/10 border-b text-xs text-destructive">
                    {error}
                </div>
            )}

            <div className="flex-1 overflow-y-auto">
                {activeAudiobook.chapters.length > 0 ? (
                    <DndContext sensors={sensors} collisionDetection={closestCenter} onDragEnd={handleDragEnd}>
                        <SortableContext items={chapterIds} strategy={verticalListSortingStrategy}>
                            {activeAudiobook.chapters.map((chapter, idx) => (
                                <SortableChapter
                                    key={chapter.index}
                                    chapter={chapter}
                                    displayIndex={idx}
                                    formatDuration={formatDuration}
                                    onPlay={handlePlayChapter}
                                    isPlaying={playingChapterIndex === chapter.index}
                                    onRename={handleRename}
                                    onMoveStart={idx > 0 ? (startMs) => handleMoveStart(idx, startMs) : undefined}
                                    onToggleCheck={
                                        idx > 0
                                            ? () => setCheckingIndex(checkingIndex === chapter.index ? null : chapter.index)
                                            : undefined
                                    }
                                    check={
                                        idx > 0 && checkingIndex === chapter.index ? (
                                            <ChapterStartCheck
                                                key={chapter.start_time_ms}
                                                audiobookId={activeAudiobook.id}
                                                files={activeAudiobook.files}
                                                startMs={chapter.start_time_ms}
                                                minMs={activeAudiobook.chapters[idx - 1].start_time_ms}
                                                maxMs={chapter.end_time_ms}
                                                onApply={(startMs) => handleMoveStart(idx, startMs)}
                                                onClose={() => setCheckingIndex(null)}
                                            />
                                        ) : undefined
                                    }
                                />
                            ))}
                        </SortableContext>
                    </DndContext>
                ) : (
                    <div className="h-full flex flex-col items-center justify-center gap-3 text-center px-8">
                        <p className="text-sm font-medium">No chapters yet</p>
                        <p className="text-xs text-muted-foreground max-w-[320px]">
                            Without chapters the M4B is one long track. Generate them from the files, a CUE
                            sheet, or a chapter list.
                        </p>
                    </div>
                )}
            </div>
        </div>
    );
}

/** Progress of listening for spoken chapter titles, with Cancel. */
function ListeningProgress({
    progress,
    onCancel,
}: {
    progress: SpokenChaptersProgress | null;
    onCancel: () => void;
}) {
    const [cancelling, setCancelling] = useState(false);
    // Finding pauses is quick next to listening, so it fills the first tenth
    const share = (p: SpokenChaptersProgress) => (p.total > 0 ? p.done / p.total : 0);
    const value = !progress ? 0 : progress.stage === "pauses" ? share(progress) * 10 : 10 + share(progress) * 90;
    const label = !progress || progress.stage === "pauses"
        ? "Finding pauses…"
        : `Listening for chapter titles… ${progress.done} of ${progress.total} places`;
    return (
        <div className="space-y-1.5">
            <span className="nums">{cancelling ? "Stopping…" : label}</span>
            <div className="flex items-center gap-3">
                <Progress value={value} className="h-2" />
                <Button
                    variant="ghost"
                    size="sm"
                    className="h-7 shrink-0"
                    disabled={cancelling}
                    onClick={() => {
                        setCancelling(true);
                        onCancel();
                    }}
                >
                    Cancel
                </Button>
            </div>
        </div>
    );
}
