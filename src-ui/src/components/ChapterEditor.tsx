import { useState, useEffect } from "react";
import { Button } from "./ui/button";
import { Label } from "./ui/label";
import { Input } from "./ui/input";
import { useApp } from "@/context/AppContext";
import * as ChaptersApi from "@/api/chapters";
import * as FilesApi from "@/api/files";
import * as PlaybackApi from "@/api/playback";
import type { Chapter } from "@/types/models";
import type { CueDetectionResult } from "@/api/files";
import { GripVertical, Play, Square } from "lucide-react";
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

interface SortableChapterProps {
    chapter: Chapter;
    displayIndex: number;
    formatDuration: (ms: number) => string;
    onPlay: (chapter: Chapter) => void;
    isPlaying: boolean;
}

function SortableChapter({ chapter, displayIndex, formatDuration, onPlay, isPlaying }: SortableChapterProps) {
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
        opacity: isDragging ? 0.5 : 1,
        zIndex: isDragging ? 1 : 0,
    };

    return (
        <div
            ref={setNodeRef}
            style={style}
            className={`flex items-center justify-between p-2 border rounded text-sm bg-card ${
                isDragging ? 'shadow-lg border-primary' : 'hover:bg-muted/50'
            }`}
        >
            <div className="flex items-center gap-2 flex-1 min-w-0">
                <button
                    onClick={(e) => { e.stopPropagation(); onPlay(chapter); }}
                    className="p-1 hover:bg-muted rounded"
                    aria-label={isPlaying ? "Stop" : "Play chapter"}
                    title={isPlaying ? "Stop" : "Play chapter"}
                >
                    {isPlaying ? (
                        <Square className="h-3 w-3 text-primary" />
                    ) : (
                        <Play className="h-3 w-3 text-muted-foreground" />
                    )}
                </button>
                <button
                    {...attributes}
                    {...listeners}
                    className="cursor-grab active:cursor-grabbing touch-none p-1 hover:bg-muted rounded"
                    aria-label="Drag to reorder"
                >
                    <GripVertical className="h-4 w-4 text-muted-foreground" />
                </button>
                <span className="font-medium">{displayIndex + 1}. </span>
                <span className="truncate">{chapter.title}</span>
            </div>
            <div className="text-muted-foreground text-xs ml-4 flex-shrink-0">
                {formatDuration(chapter.start_time_ms)} - {formatDuration(chapter.end_time_ms)}
            </div>
        </div>
    );
}

export function ChapterEditor() {
    const { activeAudiobook, setActiveAudiobook } = useApp();
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

    return (
        <div className="space-y-6">
            {/* Chapter Summary */}
            <div className="p-4 border rounded-lg bg-card">
                <div className="flex justify-between items-start mb-4">
                    <div>
                        <Label className="text-base font-semibold">Current Chapters</Label>
                        <p className="text-sm text-muted-foreground mt-1">
                            {activeAudiobook.chapters.length} chapter(s)
                        </p>
                    </div>
                </div>

                {warnings.length > 0 && (
                    <div className="mb-4 p-3 border border-yellow-500/30 rounded bg-yellow-500/5">
                        <p className="text-sm font-medium text-yellow-600 dark:text-yellow-400 mb-2">Warnings:</p>
                        <ul className="text-sm text-yellow-600 dark:text-yellow-400 space-y-1">
                            {warnings.map((warning, idx) => (
                                <li key={idx}>• {warning}</li>
                            ))}
                        </ul>
                    </div>
                )}

                {activeAudiobook.chapters.length > 0 ? (
                    <DndContext
                        sensors={sensors}
                        collisionDetection={closestCenter}
                        onDragEnd={handleDragEnd}
                    >
                        <SortableContext
                            items={chapterIds}
                            strategy={verticalListSortingStrategy}
                        >
                            <div className="space-y-1 max-h-60 overflow-y-auto">
                                {activeAudiobook.chapters.map((chapter, idx) => (
                                    <SortableChapter
                                        key={chapter.index}
                                        chapter={chapter}
                                        displayIndex={idx}
                                        formatDuration={formatDuration}
                                        onPlay={handlePlayChapter}
                                        isPlaying={playingChapterIndex === chapter.index}
                                    />
                                ))}
                            </div>
                        </SortableContext>
                    </DndContext>
                ) : (
                    <p className="text-sm text-muted-foreground">No chapters defined yet.</p>
                )}
            </div>

            {/* Import/Export Actions */}
            <div className="p-4 border rounded-lg bg-card space-y-4">
                <Label className="text-base font-semibold">Chapter Tools</Label>

                <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                    {/* Import */}
                    <div className="space-y-2">
                        <Label className="text-sm font-medium">Import Chapters</Label>
                        <Button
                            onClick={handleImportClick}
                            disabled={isImporting}
                            className="w-full"
                            variant="outline"
                        >
                            {isImporting ? "Importing..." : "Import from File"}
                        </Button>
                        <p className="text-xs text-muted-foreground">
                            Supports Audacity labels (.txt), FFmetadata (.ffmeta), and CSV (.csv)
                        </p>
                    </div>

                    {/* Export */}
                    <div className="space-y-2">
                        <Label className="text-sm font-medium">Export Chapters</Label>
                        <div className="grid grid-cols-3 gap-2">
                            <Button
                                onClick={() => handleExport("audacity")}
                                disabled={isExporting || activeAudiobook.chapters.length === 0}
                                size="sm"
                                variant="outline"
                            >
                                Audacity
                            </Button>
                            <Button
                                onClick={() => handleExport("ffmetadata")}
                                disabled={isExporting || activeAudiobook.chapters.length === 0}
                                size="sm"
                                variant="outline"
                            >
                                FFmeta
                            </Button>
                            <Button
                                onClick={() => handleExport("csv")}
                                disabled={isExporting || activeAudiobook.chapters.length === 0}
                                size="sm"
                                variant="outline"
                            >
                                CSV
                            </Button>
                        </div>
                    </div>
                </div>

                {error && (
                    <div className="text-sm text-destructive p-2 border border-destructive/30 rounded">
                        {error}
                    </div>
                )}
            </div>

            {/* Smart Generation */}
            <div className="p-4 border rounded-lg bg-card space-y-4">
                <Label className="text-base font-semibold">Smart Generation</Label>

                {/* CUE Sheet Detection */}
                {detectedCue && (
                    <div className="p-3 border border-blue-500/30 rounded bg-blue-500/5 space-y-2">
                        <div className="flex items-start justify-between">
                            <div>
                                <p className="text-sm font-medium text-blue-600 dark:text-blue-400">
                                    CUE Sheet Detected
                                </p>
                                <p className="text-xs text-muted-foreground mt-1">
                                    {detectedCue.track_count} track(s) found
                                    {detectedCue.album_title && ` - "${detectedCue.album_title}"`}
                                </p>
                            </div>
                            <Button
                                onClick={() => handleImportFromCue(detectedCue.cue_path)}
                                disabled={isImportingCue}
                                size="sm"
                                variant="default"
                            >
                                {isImportingCue ? "Importing..." : "Import as Chapters"}
                            </Button>
                        </div>
                        <p className="text-xs text-muted-foreground truncate" title={detectedCue.cue_path}>
                            {detectedCue.cue_path.split(/[/\\]/).pop()}
                        </p>
                    </div>
                )}

                {/* Import CUE Manually */}
                <div className="space-y-2">
                    <Label className="text-sm font-medium">Import from CUE Sheet</Label>
                    <Button
                        onClick={handlePickCueFile}
                        disabled={isImportingCue}
                        className="w-full"
                        variant="outline"
                    >
                        {isImportingCue ? "Importing..." : "Select CUE File..."}
                    </Button>
                    <p className="text-xs text-muted-foreground">
                        Import chapter markers from a CUE sheet file (.cue)
                    </p>
                </div>

                {/* Extract from Source */}
                <div className="space-y-2">
                    <Label className="text-sm font-medium">Extract from Source Metadata</Label>
                    <Button
                        onClick={handleGenerateFromSource}
                        disabled={isGenerating}
                        className="w-full"
                        variant="outline"
                    >
                        {isGenerating ? "Extracting..." : "Extract Chapters from Files"}
                    </Button>
                    <p className="text-xs text-muted-foreground">
                        Detects chapters from M4B, MP3 (ID3 CHAP), or FLAC (cue sheets)
                    </p>
                </div>

                {/* Title Cleanup */}
                <div className="space-y-2">
                    <Label className="text-sm font-medium">Title Cleanup (Regex)</Label>

                    {/* Presets */}
                    <div className="space-y-2">
                        <Label className="text-xs text-muted-foreground">Quick Presets:</Label>
                        <div className="grid grid-cols-2 gap-2">
                            {regexPresets.map((preset, idx) => (
                                <Button
                                    key={idx}
                                    onClick={() => handleApplyPreset(preset)}
                                    disabled={isRenaming || activeAudiobook.chapters.length === 0}
                                    size="sm"
                                    variant="secondary"
                                    title={preset.description}
                                    className="text-xs"
                                >
                                    {preset.name}
                                </Button>
                            ))}
                        </div>
                    </div>

                    {/* Custom Regex */}
                    <div className="space-y-2 pt-2 border-t">
                        <Label className="text-xs text-muted-foreground">Custom Regex:</Label>
                        <div className="grid grid-cols-2 gap-2">
                            <div>
                                <Label htmlFor="pattern" className="text-xs">Pattern</Label>
                                <Input
                                    id="pattern"
                                    placeholder="e.g. ^\d+ - "
                                    value={customPattern}
                                    onChange={(e) => setCustomPattern(e.target.value)}
                                    className="font-mono text-xs"
                                />
                            </div>
                            <div>
                                <Label htmlFor="replacement" className="text-xs">Replacement</Label>
                                <Input
                                    id="replacement"
                                    placeholder="e.g. (empty)"
                                    value={customReplacement}
                                    onChange={(e) => setCustomReplacement(e.target.value)}
                                    className="font-mono text-xs"
                                />
                            </div>
                        </div>
                        <Button
                            onClick={handleApplyCustomRegex}
                            disabled={isRenaming || !customPattern || activeAudiobook.chapters.length === 0}
                            size="sm"
                            variant="outline"
                            className="w-full"
                        >
                            Apply Custom Regex
                        </Button>
                    </div>
                </div>
            </div>
        </div>
    );
}
