import { useCallback, useEffect, useRef, useState } from "react";
import { PlayIcon, StopIcon } from "@heroicons/react/20/solid";
import { Button } from "./ui/button";
import * as PlaybackApi from "@/api/playback";
import type { AudioFile, Waveform } from "@/types/models";
import { formatChapterTime, locateInBook, snapToPause } from "@/lib/chapterTime";
import { cn } from "@/lib/utils";

/** How much audio is shown either side of the start */
const SPAN_MS = 10_000;
/** How long before the start playing begins */
const PLAY_BEFORE_MS = 3_000;
/** How close to its neighbours a start may go */
const EDGE_MS = 100;

interface ChapterStartCheckProps {
    audiobookId: string;
    files: AudioFile[];
    /** The chapter's start as it is now */
    startMs: number;
    /** The previous chapter's start and this chapter's end; the new start goes between them */
    minMs: number;
    maxMs: number;
    /** Saves the new start; resolves to why it was refused, or null */
    onApply: (startMs: number) => Promise<string | null>;
    onClose: () => void;
}

/**
 * The audio around a chapter's start: its waveform with pauses shaded, the
 * start as a line to click or nudge, and playback from just before it.
 */
export function ChapterStartCheck({ audiobookId, files, startMs, minMs, maxMs, onApply, onClose }: ChapterStartCheckProps) {
    const [view, setView] = useState<Waveform | null>(null);
    const [loadError, setLoadError] = useState<string | null>(null);
    const [draft, setDraft] = useState(startMs);
    const [applyError, setApplyError] = useState<string | null>(null);
    const [saving, setSaving] = useState(false);
    // Book time being played, while playing from here
    const [playhead, setPlayhead] = useState<number | null>(null);
    const playing = useRef<{ path: string; fileStartMs: number } | null>(null);
    const svgRef = useRef<SVGSVGElement>(null);

    useEffect(() => {
        let stale = false;
        PlaybackApi.getWaveform(audiobookId, startMs, SPAN_MS)
            .then((w) => !stale && setView(w))
            .catch((e) => !stale && setLoadError(String(e)));
        return () => {
            stale = true;
        };
    }, [audiobookId, startMs]);

    const stop = useCallback(async () => {
        if (!playing.current) return;
        playing.current = null;
        setPlayhead(null);
        await PlaybackApi.stopAudio().catch(() => {});
    }, []);

    // Stop when the panel closes
    useEffect(() => () => void stop(), [stop]);

    // Follow playback while playing from here
    useEffect(() => {
        if (playhead === null) return;
        const timer = window.setInterval(async () => {
            const now = playing.current;
            if (!now) return;
            try {
                const status = await PlaybackApi.getPlaybackStatus();
                if (status.state === "stopped" || status.current_path !== now.path) {
                    playing.current = null;
                    setPlayhead(null);
                } else {
                    setPlayhead(now.fileStartMs + status.position_ms);
                }
            } catch {
                // A missed poll changes nothing
            }
        }, 100);
        return () => window.clearInterval(timer);
    }, [playhead === null]); // eslint-disable-line react-hooks/exhaustive-deps

    const clamp = (ms: number) => Math.round(Math.min(maxMs - EDGE_MS, Math.max(minMs + EDGE_MS, ms)));
    const move = (ms: number) => {
        setDraft(clamp(ms));
        setApplyError(null);
    };

    const play = async () => {
        if (playing.current) return stop();
        const from = Math.max(0, draft - PLAY_BEFORE_MS);
        const at = locateInBook(files, from);
        if (!at) return;
        try {
            await PlaybackApi.playAudio(at.path, at.offsetMs);
            playing.current = { path: at.path, fileStartMs: at.fileStartMs };
            setPlayhead(from);
        } catch (e) {
            setApplyError(`Could not play: ${e}`);
        }
    };

    const apply = async () => {
        setSaving(true);
        await stop();
        const refused = await onApply(draft);
        setSaving(false);
        if (refused) setApplyError(refused);
        else onClose();
    };

    const snapped = view ? snapToPause(view.pauses, draft, minMs, maxMs) : null;
    const changed = draft !== startMs;
    const delta = (draft - startMs) / 1000;

    // Drawing: x is book time, y is level around the middle
    const width = view?.peaks.length ?? 600;
    const span = view ? Math.max(1, view.to_ms - view.from_ms) : 1;
    const x = (ms: number) => (view ? ((ms - view.from_ms) / span) * width : 0);
    const inView = (ms: number) => view !== null && ms >= view.from_ms && ms <= view.to_ms;

    const pick = (clientX: number) => {
        const box = svgRef.current?.getBoundingClientRect();
        if (!box || !view) return;
        move(view.from_ms + ((clientX - box.left) / box.width) * span);
    };

    return (
        <div
            className="px-3 pt-2.5 pb-3 bg-surface-2/60 border-t border-border/60 space-y-2.5"
            onKeyDown={(e) => {
                if (e.key === "Escape") onClose();
                if (e.key === "ArrowLeft") move(draft - (e.shiftKey ? 1000 : 100));
                if (e.key === "ArrowRight") move(draft + (e.shiftKey ? 1000 : 100));
            }}
        >
            <div className="relative h-[72px] rounded-md bg-background border overflow-hidden">
                {view ? (
                    <svg
                        ref={svgRef}
                        viewBox={`0 0 ${width} 100`}
                        preserveAspectRatio="none"
                        className="absolute inset-0 h-full w-full cursor-crosshair"
                        onClick={(e) => pick(e.clientX)}
                        role="img"
                        aria-label="The audio around this chapter's start. Click to start the chapter there."
                    >
                        {view.pauses.map((p, i) => (
                            <rect
                                key={i}
                                x={x(p.start_ms)}
                                width={Math.max(1, x(p.end_ms) - x(p.start_ms))}
                                y={0}
                                height={100}
                                className="fill-primary/20"
                            />
                        ))}
                        {view.peaks.map((peak, i) => {
                            // Square root, so quiet speech still shows
                            const h = Math.max(0.5, Math.sqrt(peak) * 46);
                            return (
                                <rect key={i} x={i} width={1} y={50 - h} height={h * 2} className="fill-muted-foreground/60" />
                            );
                        })}
                        {changed && inView(startMs) && (
                            <line
                                x1={x(startMs)}
                                x2={x(startMs)}
                                y1={0}
                                y2={100}
                                strokeDasharray="4 3"
                                vectorEffect="non-scaling-stroke"
                                className="stroke-muted-foreground"
                            />
                        )}
                        {playhead !== null && inView(playhead) && (
                            <line
                                x1={x(playhead)}
                                x2={x(playhead)}
                                y1={0}
                                y2={100}
                                vectorEffect="non-scaling-stroke"
                                className="stroke-foreground"
                            />
                        )}
                        {inView(draft) && (
                            <line
                                x1={x(draft)}
                                x2={x(draft)}
                                y1={0}
                                y2={100}
                                strokeWidth={2}
                                vectorEffect="non-scaling-stroke"
                                className="stroke-primary"
                            />
                        )}
                    </svg>
                ) : (
                    <div className="h-full flex items-center justify-center text-xs text-muted-foreground">
                        {loadError ? <span className="text-destructive">Could not read the audio: {loadError}</span> : "Reading the audio…"}
                    </div>
                )}
            </div>

            {view && (
                <div className="flex justify-between text-[11px] text-muted-foreground nums -mt-1">
                    <span>{formatChapterTime(view.from_ms)}</span>
                    <span>Shaded areas are pauses. Click to start the chapter there; ← → nudge it.</span>
                    <span>{formatChapterTime(view.to_ms)}</span>
                </div>
            )}

            <div className="flex flex-wrap items-center gap-1.5">
                <Button size="sm" variant="outline" className="h-7 px-2 text-xs nums" onClick={() => move(draft - 1000)}>
                    −1 s
                </Button>
                <Button size="sm" variant="outline" className="h-7 px-2 text-xs nums" onClick={() => move(draft - 250)}>
                    −¼ s
                </Button>
                <span className="min-w-[112px] text-center text-xs nums">
                    {formatChapterTime(draft)}
                    {changed && (
                        <span className="text-muted-foreground"> ({delta > 0 ? "+" : "−"}{Math.abs(delta).toFixed(2)} s)</span>
                    )}
                </span>
                <Button size="sm" variant="outline" className="h-7 px-2 text-xs nums" onClick={() => move(draft + 250)}>
                    +¼ s
                </Button>
                <Button size="sm" variant="outline" className="h-7 px-2 text-xs nums" onClick={() => move(draft + 1000)}>
                    +1 s
                </Button>
                <Button
                    size="sm"
                    variant="outline"
                    className="h-7 px-2 text-xs"
                    disabled={snapped === null || snapped === draft}
                    title={snapped === null ? "No pause nearby" : "Start just before the speech after the nearest pause"}
                    onClick={() => snapped !== null && move(snapped)}
                >
                    Snap to pause
                </Button>
                <Button
                    size="sm"
                    variant="outline"
                    className={cn("h-7 px-2 text-xs", playhead !== null && "text-primary")}
                    title="Play from 3 seconds before the start"
                    onClick={play}
                >
                    {playhead !== null ? <StopIcon className="h-3.5 w-3.5 mr-1" /> : <PlayIcon className="h-3.5 w-3.5 mr-1" />}
                    {playhead !== null ? "Stop" : "Play"}
                </Button>

                <div className="ml-auto flex items-center gap-1.5">
                    <Button size="sm" variant="ghost" className="h-7 px-2.5 text-xs" onClick={onClose}>
                        {changed ? "Cancel" : "Close"}
                    </Button>
                    <Button size="sm" className="h-7 px-2.5 text-xs" disabled={!changed || saving} onClick={apply}>
                        {saving ? "Saving…" : "Use this start"}
                    </Button>
                </div>
            </div>

            {applyError && <p className="text-xs text-destructive">{applyError}</p>}
        </div>
    );
}
