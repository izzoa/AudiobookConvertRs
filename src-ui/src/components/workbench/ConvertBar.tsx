import { useCallback, useEffect, useMemo, useState } from "react";
import {
    CheckCircleIcon,
    ExclamationTriangleIcon,
    AdjustmentsHorizontalIcon,
    FolderIcon,
    ArrowRightIcon,
} from "@heroicons/react/24/outline";
import { Button } from "@/components/ui/button";
import { Popover } from "@/components/ui/popover";
import { Select } from "@/components/ui/select";
import { useConfirm } from "@/components/ui/confirm";
import { useToast } from "@/components/ui/use-toast";
import { useApp } from "@/context/AppContext";
import { useConversionStore } from "@/context/ConversionStore";
import * as FilesApi from "@/api/files";
import * as ConversionApi from "@/api/conversion";
import { dryRunConversion, listPresets, savePreset } from "@/api/presets";
import { suggestNormalization } from "@/api/quality";
import type { ConversionPreset, ConversionSettings, DryRunResult, EncodingMode } from "@/types/models";
import { formatBytes, formatEta, shortenPath } from "@/lib/format";
import { cn } from "@/lib/utils";

const BITRATES = [32, 48, 64, 96, 128, 160, 192, 256];

/** The backend rejects bitrates above this; lossless sources (FLAC, WAV) would exceed it. */
const MAX_BITRATE = 320;

/** Dry-run warnings (from `conversion/batch.rs`) that a readiness chip already states. */
const CHIP_COVERED = ["No input files", "No chapters", "No artwork", "Output file already exists"];

/**
 * One permanent home for "what will be produced, and go".
 *
 * This replaces three separate surfaces in the old UI — a dry-run card, a
 * settings modal and a queue panel — none of which said whether the book was
 * actually ready. The readiness row answers that without being a wizard.
 */
export function ConvertBar({ onShowChapters }: { onShowChapters: () => void }) {
    const {
        activeAudiobook,
        settings,
        getEffectiveConversionDefaults,
        setView,
        setPendingConversionSettings,
    } = useApp();
    const { toast } = useToast();
    const confirm = useConfirm();
    const { refreshJobs, refreshQueueState } = useConversionStore();

    const defaults = getEffectiveConversionDefaults();
    const [outputPath, setOutputPath] = useState<string | null>(null);
    const [bitrate, setBitrate] = useState(defaults.bitrate_kbps);
    const [matchSource, setMatchSource] = useState(false);
    const [mode, setMode] = useState<EncodingMode>(defaults.encoding_mode);
    const [embedChapters, setEmbedChapters] = useState(defaults.embed_chapters);
    const [embedArtwork, setEmbedArtwork] = useState(defaults.embed_artwork);
    const [copyMetadata, setCopyMetadata] = useState(defaults.copy_metadata);
    const [normalize, setNormalize] = useState(false);
    const [preview, setPreview] = useState<DryRunResult | null>(null);
    const [submitting, setSubmitting] = useState(false);
    const [presets, setPresets] = useState<ConversionPreset[]>([]);
    const [loudness, setLoudness] = useState<string | null>(null);
    const [analyzing, setAnalyzing] = useState(false);
    const [presetName, setPresetName] = useState<string | null>(null);

    // A newly imported book gets a sensible destination without being asked.
    useEffect(() => {
        if (!activeAudiobook) {
            setOutputPath(null);
            return;
        }
        const safeTitle = (activeAudiobook.title || "audiobook").replace(/[\\/:*?"<>|]/g, "_");
        const dir = settings?.default_output_directory;
        setOutputPath(dir ? `${dir}/${safeTitle}.m4b` : null);
    }, [activeAudiobook?.id, settings?.default_output_directory]);

    // Average of the files' own bitrates, capped at what the encoder accepts.
    const sourceBitrate = useMemo(() => {
        const rates = (activeAudiobook?.files ?? [])
            .map((f) => f.bitrate_kbps)
            .filter((b): b is number => typeof b === "number" && b > 0);
        if (rates.length === 0) return undefined;
        return Math.min(MAX_BITRATE, Math.round(rates.reduce((a, b) => a + b, 0) / rates.length));
    }, [activeAudiobook?.files]);
    const effectiveBitrate = matchSource && sourceBitrate ? sourceBitrate : bitrate;

    const buildSettings = useCallback(
        (): ConversionSettings | null =>
            !activeAudiobook || !outputPath
                ? null
                : {
                      bitrate_kbps: effectiveBitrate,
                      sample_rate_hz: defaults.sample_rate_hz,
                      encoding_mode: mode,
                      output_format: "m4b",
                      output_path: outputPath,
                      concurrent_jobs: settings?.concurrent_jobs ?? 1,
                      overwrite_existing: defaults.overwrite_existing,
                      embed_chapters: embedChapters,
                      embed_artwork: embedArtwork,
                      copy_metadata: copyMetadata,
                      normalize_audio: normalize,
                      normalization_settings: normalize
                          ? {
                                target_lufs: settings?.normalization_target_lufs ?? -18,
                                target_true_peak: settings?.normalization_target_true_peak ?? -2,
                                target_lra: settings?.normalization_target_lra ?? 7,
                                two_pass: settings?.normalization_two_pass ?? true,
                            }
                          : undefined,
                  },
        [
            activeAudiobook,
            outputPath,
            effectiveBitrate,
            mode,
            embedChapters,
            embedArtwork,
            copyMetadata,
            normalize,
            defaults.sample_rate_hz,
            defaults.overwrite_existing,
            settings,
        ]
    );

    // Keep the shared pending settings in step so other views agree with the bar.
    useEffect(() => {
        setPendingConversionSettings(buildSettings());
    }, [buildSettings, setPendingConversionSettings]);

    // Estimates follow the knobs, debounced so dragging a bitrate is cheap.
    useEffect(() => {
        const next = buildSettings();
        if (!next) {
            setPreview(null);
            return;
        }
        const timer = setTimeout(() => {
            dryRunConversion(next)
                .then(setPreview)
                .catch((error) => {
                    console.error("Dry run failed:", error);
                    setPreview(null);
                });
        }, 350);
        return () => clearTimeout(timer);
    }, [buildSettings]);

    useEffect(() => {
        listPresets().then(setPresets).catch(() => setPresets([]));
    }, []);

    const applyPreset = (preset: ConversionPreset) => {
        setBitrate(preset.settings.bitrate_kbps);
        setMatchSource(false);
        setEmbedChapters(preset.settings.embed_chapters);
        setEmbedArtwork(preset.settings.embed_artwork);
        setCopyMetadata(preset.settings.copy_metadata);
        setNormalize(preset.settings.normalize);
    };

    const storePreset = async () => {
        const name = presetName?.trim();
        if (!name) return;
        const now = new Date().toISOString();
        try {
            await savePreset({
                name,
                settings: {
                    bitrate_kbps: effectiveBitrate,
                    sample_rate: defaults.sample_rate_hz,
                    concurrent_jobs: settings?.concurrent_jobs ?? 1,
                    overwrite_existing: defaults.overwrite_existing,
                    embed_chapters: embedChapters,
                    embed_artwork: embedArtwork,
                    copy_metadata: copyMetadata,
                    normalize,
                    target_lufs: settings?.normalization_target_lufs,
                },
                created_at: now,
                modified_at: now,
            });
            setPresets(await listPresets());
            setPresetName(null);
            toast({ title: "Preset saved", description: name });
        } catch (error) {
            toast({ title: "Could not save preset", description: String(error), variant: "destructive" });
        }
    };

    /** Measures the first file so the normalize toggle is an informed choice. */
    const checkLoudness = async () => {
        const first = activeAudiobook?.files[0];
        if (!first) return;
        setAnalyzing(true);
        try {
            const suggestion = await suggestNormalization(first.path);
            setLoudness(suggestion.reason);
            if (suggestion.recommended) setNormalize(true);
        } catch (error) {
            setLoudness(`Could not analyse: ${error}`);
        } finally {
            setAnalyzing(false);
        }
    };

    const chooseOutput = async () => {
        if (!activeAudiobook) return;
        try {
            const picked = await FilesApi.pickOutputFile(`${activeAudiobook.title || "audiobook"}.m4b`);
            if (picked) setOutputPath(picked);
        } catch (error) {
            toast({ title: "Could not choose output", description: String(error), variant: "destructive" });
        }
    };

    const convert = async () => {
        const next = buildSettings();
        if (!activeAudiobook || !next) {
            await chooseOutput();
            return;
        }
        if (preview?.output_exists && !next.overwrite_existing) {
            const overwrite = await confirm({
                title: "Replace the existing file?",
                description: <span className="font-mono text-xs">{next.output_path}</span>,
                confirmLabel: "Replace",
                danger: true,
            });
            if (!overwrite) return;
            next.overwrite_existing = true;
        }

        setSubmitting(true);
        try {
            await ConversionApi.enqueueConversion(activeAudiobook.id, next);
            await ConversionApi.startConversionQueue(settings?.concurrent_jobs ?? 1);
            // Load the new job before switching, so the queue never opens empty.
            await Promise.all([refreshJobs(), refreshQueueState()]);
            toast({
                title: "Conversion started",
                description: `Writing ${next.output_path}`,
            });
            setView("queue");
        } catch (error) {
            toast({ title: "Could not start conversion", description: String(error), variant: "destructive" });
        } finally {
            setSubmitting(false);
        }
    };

    const book = activeAudiobook;
    const checks = useMemo(
        () => [
            {
                key: "files",
                ok: !!book && book.files.length > 0,
                label: book ? `${book.files.length} files` : "No files",
            },
            {
                key: "cover",
                ok: !!book?.output_metadata?.artwork_data,
                label: book?.output_metadata?.artwork_data ? "Cover" : "No cover",
                optional: true,
            },
            {
                key: "chapters",
                ok: (book?.chapters.length ?? 0) > 0,
                label:
                    (book?.chapters.length ?? 0) > 0
                        ? `${book!.chapters.length} chapters`
                        : "No chapters",
                onClick: onShowChapters,
            },
            {
                key: "output",
                ok: !!outputPath && !preview?.output_exists,
                label: !outputPath ? "No output file" : preview?.output_exists ? "Output exists" : "Output set",
                hint: preview?.output_exists ? "A file is already there. You will be asked before it is replaced." : undefined,
                onClick: chooseOutput,
            },
        ],
        [book, outputPath, onShowChapters, preview?.output_exists]
    );

    const ready = !!book && book.files.length > 0 && !!outputPath;
    // The chips already report missing files, chapters, cover and an existing
    // output; repeating the backend's sentence for the same fact beside them
    // is noise. Anything else the dry run warns about still shows.
    const warnings = (preview?.warnings ?? []).filter((w) => !CHIP_COVERED.some((prefix) => w.startsWith(prefix)));

    return (
        <footer className="shrink-0 border-t bg-surface-2">
            <div className="h-9 px-3 flex items-center gap-1.5 border-b border-border/60 overflow-x-auto no-select">
                {checks.map((check) => (
                    <button
                        key={check.key}
                        onClick={check.onClick}
                        disabled={!check.onClick}
                        title={check.hint}
                        className={cn(
                            "flex items-center gap-1.5 h-6 px-2 rounded-full text-xs font-medium shrink-0",
                            "transition-colors",
                            check.ok
                                ? "text-success"
                                : check.optional
                                  ? "text-muted-foreground"
                                  : "text-warning",
                            check.onClick && "hover:bg-accent"
                        )}
                    >
                        {check.ok ? (
                            <CheckCircleIcon className="w-3.5 h-3.5" />
                        ) : (
                            <ExclamationTriangleIcon className="w-3.5 h-3.5" />
                        )}
                        <span className="nums">{check.label}</span>
                    </button>
                ))}

                {warnings.length > 0 && (
                    <>
                        <span className="w-px h-4 bg-border shrink-0" />
                        <span className="text-xs text-warning truncate" title={warnings.join("\n")}>
                            {warnings[0]}
                            {warnings.length > 1 && ` (+${warnings.length - 1} more)`}
                        </span>
                    </>
                )}
            </div>

            <div className="h-[58px] px-3 flex items-center gap-2">
                <button
                    onClick={chooseOutput}
                    disabled={!book}
                    className={cn(
                        "flex items-center gap-2 h-9 px-2.5 rounded-md min-w-0 max-w-[40%] transition-colors",
                        "hover:bg-accent disabled:opacity-40 disabled:pointer-events-none"
                    )}
                    title={outputPath ?? "Choose where to write the M4B"}
                >
                    <FolderIcon className="w-4 h-4 text-muted-foreground shrink-0" />
                    <span className="min-w-0 text-left">
                        <span className="block text-[10px] font-semibold uppercase tracking-wider text-muted-foreground leading-tight">
                            Output
                        </span>
                        <span
                            className={cn(
                                "block text-xs leading-tight truncate",
                                outputPath ? "text-foreground" : "text-warning"
                            )}
                        >
                            {outputPath ? shortenPath(outputPath, 46) : "Choose a destination…"}
                        </span>
                    </span>
                </button>

                <Popover
                    side="top"
                    align="start"
                    className="w-[300px] p-3"
                    trigger={(open) => (
                        <button
                            disabled={!book}
                            aria-haspopup="dialog"
                            aria-expanded={open}
                            className={cn(
                                "flex items-center gap-2 h-9 px-2.5 rounded-md transition-colors shrink-0",
                                "disabled:opacity-40 disabled:pointer-events-none",
                                open ? "bg-accent" : "hover:bg-accent"
                            )}
                        >
                            <AdjustmentsHorizontalIcon className="w-4 h-4 text-muted-foreground" />
                            <span className="text-left">
                                <span className="block text-[10px] font-semibold uppercase tracking-wider text-muted-foreground leading-tight">
                                    Quality
                                </span>
                                <span className="block text-xs leading-tight nums">
                                    {matchSource && sourceBitrate ? `Source ${sourceBitrate}k` : `${bitrate}k`}{" "}
                                    {mode.toUpperCase()}
                                    {normalize && " · normalized"}
                                </span>
                            </span>
                        </button>
                    )}
                >
                    <div className="space-y-3">
                        {presetName === null ? (
                            <div className="flex items-center gap-1.5">
                                <Select
                                    className="flex-1"
                                    aria-label="Load a preset"
                                    value=""
                                    disabled={presets.length === 0}
                                    onChange={(e) => {
                                        const found = presets.find((p) => p.name === e.target.value);
                                        if (found) applyPreset(found);
                                    }}
                                >
                                    <option value="">{presets.length ? "Load a preset…" : "No saved presets"}</option>
                                    {presets.map((preset) => (
                                        <option key={preset.name} value={preset.name}>
                                            {preset.name}
                                        </option>
                                    ))}
                                </Select>
                                <Button variant="outline" size="sm" className="h-9" onClick={() => setPresetName("")}>
                                    Save as…
                                </Button>
                            </div>
                        ) : (
                            <form
                                className="flex items-center gap-1.5"
                                onSubmit={(e) => {
                                    e.preventDefault();
                                    storePreset();
                                }}
                            >
                                <input
                                    autoFocus
                                    value={presetName}
                                    onChange={(e) => setPresetName(e.target.value)}
                                    onKeyDown={(e) => {
                                        // Escape would otherwise close the whole popover.
                                        if (e.key === "Escape") {
                                            e.stopPropagation();
                                            setPresetName(null);
                                        }
                                    }}
                                    placeholder="Preset name"
                                    aria-label="Preset name"
                                    className="flex-1 min-w-0 h-9 rounded-md border border-input bg-background px-3 text-sm focus:outline-none focus-visible:ring-2 focus-visible:ring-ring"
                                />
                                <Button type="submit" size="sm" className="h-9" disabled={!presetName.trim()}>
                                    Save
                                </Button>
                            </form>
                        )}

                        <div>
                            <div className="mb-1.5 text-[11px] font-semibold uppercase tracking-wider text-muted-foreground">
                                Bitrate
                            </div>
                            <Segmented<number | "source">
                                columns={4}
                                label="Bitrate"
                                options={[
                                    ...(sourceBitrate
                                        ? [{ value: "source" as const, label: `Match source (~${sourceBitrate}k)`, wide: true }]
                                        : []),
                                    ...BITRATES.map((rate) => ({ value: rate, label: `${rate}k` })),
                                ]}
                                value={matchSource && sourceBitrate ? "source" : bitrate}
                                onChange={(choice) => {
                                    setMatchSource(choice === "source");
                                    if (choice !== "source") setBitrate(choice);
                                }}
                            />
                            <p className="mt-1.5 text-[11px] text-muted-foreground">
                                {matchSource && sourceBitrate
                                    ? "AAC needs fewer kbps than MP3 for the same quality, so matching usually makes a larger file than needed."
                                    : "64k is plenty for spoken word. Higher mostly costs disk."}
                            </p>
                        </div>

                        <div>
                            <div className="mb-1.5 text-[11px] font-semibold uppercase tracking-wider text-muted-foreground">
                                Encoding
                            </div>
                            <Segmented
                                columns={2}
                                label="Encoding"
                                options={[
                                    { value: "vbr" as EncodingMode, label: "VBR" },
                                    { value: "cbr" as EncodingMode, label: "CBR" },
                                ]}
                                value={mode}
                                onChange={setMode}
                            />
                        </div>

                        <div className="pt-1 border-t space-y-0.5">
                            <Toggle checked={embedChapters} onChange={setEmbedChapters} label="Embed chapters" />
                            <Toggle checked={embedArtwork} onChange={setEmbedArtwork} label="Embed cover art" />
                            <Toggle checked={copyMetadata} onChange={setCopyMetadata} label="Copy metadata" />
                            <Toggle
                                checked={normalize}
                                onChange={setNormalize}
                                label="Normalize loudness"
                                hint="Evens out quiet and loud files. Adds an analysis pass."
                            />
                            <button
                                onClick={checkLoudness}
                                disabled={analyzing || !activeAudiobook?.files.length}
                                className="ml-[26px] text-[11px] text-primary hover:underline disabled:opacity-40"
                            >
                                {analyzing ? "Measuring…" : "Measure this book's loudness"}
                            </button>
                            {loudness && (
                                <p className="ml-[26px] text-[11px] text-muted-foreground">{loudness}</p>
                            )}
                        </div>
                    </div>
                </Popover>

                {preview && (
                    <div className="hidden lg:block shrink-0 px-2 text-xs text-muted-foreground nums leading-tight">
                        <div>≈ {formatBytes(preview.estimated_output_size_bytes)}</div>
                        <div>≈ {formatEta(preview.estimated_time_seconds)} to convert</div>
                    </div>
                )}

                <div className="flex-1" />

                <Button size="lg" onClick={convert} disabled={!ready || submitting} className="shrink-0">
                    {submitting ? "Starting…" : "Convert to M4B"}
                    <ArrowRightIcon className="w-4 h-4 ml-2" />
                </Button>
            </div>
        </footer>
    );
}

/**
 * Mutually exclusive choice. Selection is shown by lifting the segment, not by
 * the ember accent, which stays reserved for the Convert action.
 */
function Segmented<T extends string | number>({
    options,
    value,
    onChange,
    columns,
    label,
}: {
    options: { value: T; label: string; wide?: boolean }[];
    value: T;
    onChange: (value: T) => void;
    columns: number;
    label: string;
}) {
    return (
        <div
            role="radiogroup"
            aria-label={label}
            className="grid gap-0.5 rounded-md bg-muted p-0.5"
            style={{ gridTemplateColumns: `repeat(${columns}, minmax(0, 1fr))` }}
        >
            {options.map((option) => {
                const active = option.value === value;
                return (
                    <button
                        key={option.value}
                        role="radio"
                        aria-checked={active}
                        onClick={() => onChange(option.value)}
                        className={cn(
                            "h-7 rounded-[5px] text-xs font-medium nums transition-colors",
                            option.wide && "col-span-full",
                            active
                                ? "bg-background dark:bg-accent text-foreground shadow-plane"
                                : "text-muted-foreground hover:text-foreground"
                        )}
                    >
                        {option.label}
                    </button>
                );
            })}
        </div>
    );
}

function Toggle({
    checked,
    onChange,
    label,
    hint,
}: {
    checked: boolean;
    onChange: (value: boolean) => void;
    label: string;
    hint?: string;
}) {
    return (
        <label className="flex items-start gap-2.5 py-1.5 px-1 rounded-md hover:bg-accent/60 cursor-pointer">
            <input
                type="checkbox"
                checked={checked}
                onChange={(e) => onChange(e.target.checked)}
                className="mt-0.5 w-4 h-4 shrink-0 accent-[hsl(var(--primary))]"
            />
            <span className="min-w-0">
                <span className="block text-sm leading-tight">{label}</span>
                {hint && <span className="block text-[11px] text-muted-foreground mt-0.5">{hint}</span>}
            </span>
        </label>
    );
}
