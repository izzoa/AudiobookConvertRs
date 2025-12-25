import { useState, useEffect } from "react";
import { Button } from "./ui/button";
import { Label } from "./ui/label";
import { Input } from "./ui/input";
import { useApp } from "@/context/AppContext";
import * as ChaptersApi from "@/api/chapters";
import * as FilesApi from "@/api/files";
import type { Chapter } from "@/types/models";
import { GripVertical } from "lucide-react";
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
}

function SortableChapter({ chapter, displayIndex, formatDuration }: SortableChapterProps) {
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
