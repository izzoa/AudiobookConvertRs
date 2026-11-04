import { useEffect, useMemo, useRef, useState, type FormEvent, type ReactNode } from "react";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { ArrowLeftIcon, CheckIcon, MagnifyingGlassIcon, PhotoIcon, XMarkIcon } from "@heroicons/react/24/outline";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Select } from "@/components/ui/select";
import { useToast } from "@/components/ui/use-toast";
import { useApp } from "@/context/useApp";
import * as LookupApi from "@/api/lookup";
import { AUDIBLE_REGIONS, DEFAULT_AUDIBLE_REGION } from "@/lib/audibleRegions";
import { formatBytes, formatClock, formatDuration } from "@/lib/format";
import {
    applyPayload,
    initialSelection,
    isSelectable,
    reviewRows,
    type ReviewKey,
    type FlaggedChapter,
    type ReviewRow,
} from "@/lib/lookupReview";
import { cn } from "@/lib/utils";
import { SpeechModelDownload } from "@/components/settings/SpeechModelPanel";
import type { AudioBook, ChaptersFittedEvent, LookupCandidate, LookupRecord, LookupSource } from "@/types/models";

/** Ten letters and digits with at least one digit: an ASIN (or an ISBN-10), not a word. */
const ASIN_PATTERN = /^(?=.*\d)[A-Z0-9]{10}$/i;

const SOURCE_LABEL: Record<LookupSource, string> = { audible: "Audible", open_library: "Open Library" };

/**
 * Find details: search Audible (or Open Library), pick a result, review what
 * it would change, and apply the ticked values as one undo step.
 *
 * Nothing is sent anywhere until the user searches or picks a result.
 */
export function FindDetailsDialog({ open, onClose }: { open: boolean; onClose: () => void }) {
    const { activeAudiobook } = useApp();
    const dialogRef = useRef<HTMLDialogElement>(null);

    useEffect(() => {
        const dialog = dialogRef.current;
        if (!dialog) return;
        if (open && !dialog.open) dialog.showModal();
        if (!open && dialog.open) dialog.close();
    }, [open]);

    return (
        <dialog
            ref={dialogRef}
            onCancel={(e) => {
                e.preventDefault();
                onClose();
            }}
            aria-label="Find book details"
            className="m-auto w-[min(720px,calc(100vw-32px))] h-[min(620px,calc(100vh-48px))] rounded-xl border bg-popover p-0 text-popover-foreground shadow-lift backdrop:bg-[hsl(var(--scrim))] open:animate-slide-up"
        >
            {/* Mounted only while open, so every opening starts from a fresh search. */}
            {open && activeAudiobook && <FindDetails book={activeAudiobook} onClose={onClose} />}
        </dialog>
    );
}

function FindDetails({ book, onClose }: { book: AudioBook; onClose: () => void }) {
    const { settings, setActiveAudiobook, refreshUndoState } = useApp();
    const { toast } = useToast();
    const meta = book.output_metadata ?? {};

    const [source, setSource] = useState<LookupSource>("audible");
    const [region, setRegion] = useState(settings?.audible_region || DEFAULT_AUDIBLE_REGION);
    const [title, setTitle] = useState(meta.title || book.title || "");
    const [author, setAuthor] = useState(meta.artist || book.author || "");
    const [results, setResults] = useState<LookupCandidate[] | null>(null);
    const [record, setRecord] = useState<LookupRecord | null>(null);
    const [busy, setBusy] = useState<"search" | "record" | "apply" | null>(null);
    const [error, setError] = useState<string | null>(null);
    // Bumped to fit again from scratch once the speech model is installed
    const [fits, setFits] = useState(0);

    // Chapters that need pause searches arrive after the record. A result for a
    // record no longer shown (another pick, or Back) does not match and is dropped.
    useEffect(() => {
        let unlisten: UnlistenFn | undefined;
        let closed = false;
        listen<ChaptersFittedEvent>("lookup_chapters_fitted", ({ payload }) =>
            setRecord((shown) =>
                shown?.chapters.kind === "fitting" && shown.chapters.request_id === payload.request_id
                    ? { ...shown, chapters: payload.offer }
                    : shown
            )
        )
            .then((fn) => (closed ? fn() : (unlisten = fn)))
            .catch((e) => console.error("Could not listen for fitted chapters:", e));
        return () => {
            closed = true;
            unlisten?.();
        };
    }, []);

    const search = async (event?: FormEvent, from: LookupSource = source) => {
        event?.preventDefault();
        const text = title.trim();
        if (!text) return;
        // A typed ASIN goes straight to that book.
        if (from === "audible" && ASIN_PATTERN.test(text) && !author.trim()) {
            await openRecord("audible", text.toUpperCase());
            return;
        }
        setBusy("search");
        setError(null);
        try {
            setResults(
                from === "audible"
                    ? await LookupApi.audibleSearch(text, author.trim(), region)
                    : await LookupApi.openLibrarySearch([text, author.trim()].filter(Boolean).join(" "))
            );
        } catch (e) {
            setResults(null);
            setError(String(e));
        } finally {
            setBusy(null);
        }
    };

    const openRecord = async (from: LookupSource, id: string) => {
        setBusy("record");
        setError(null);
        try {
            setRecord(await LookupApi.lookupRecord(from, id, book.id, from === "audible" ? region : undefined));
        } catch (e) {
            setError(String(e));
        } finally {
            setBusy(null);
        }
    };

    const switchSource = (next: LookupSource) => {
        setSource(next);
        setResults(null);
        setError(null);
    };

    const apply = async (payload: ReturnType<typeof applyPayload>, count: number) => {
        if (!record) return;
        setBusy("apply");
        setError(null);
        try {
            setActiveAudiobook(await LookupApi.applyLookup(book.id, record.source, payload));
            await refreshUndoState();
            toast({ title: `Applied ${count} ${count === 1 ? "change" : "changes"} from ${SOURCE_LABEL[record.source]}` });
            onClose();
        } catch (e) {
            setError(String(e));
            setBusy(null);
        }
    };

    if (record) {
        return (
            <Review
                key={`${record.source}-${record.id}-${fits}`}
                book={book}
                record={record}
                busy={busy === "apply"}
                error={error}
                onBack={() => {
                    setRecord(null);
                    setError(null);
                }}
                onApply={apply}
                onClose={onClose}
                onRefit={() => {
                    setFits((n) => n + 1);
                    openRecord(record.source, record.id);
                }}
            />
        );
    }

    const exactAsin = source === "audible" ? meta.asin : undefined;

    return (
        <div className="flex h-full flex-col">
            <header className="flex items-center justify-between gap-3 px-5 pt-4 pb-3 no-select">
                <h2 className="text-[15px] font-semibold">Find book details</h2>
                <CloseButton onClose={onClose} />
            </header>

            <form onSubmit={(e) => search(e)} className="px-5 pb-4 space-y-3 border-b">
                <div className="flex items-center gap-2">
                    <SourceSwitch value={source} onChange={switchSource} />
                    {source === "audible" && (
                        <Select
                            aria-label="Audible store"
                            value={region}
                            onChange={(e) => setRegion(e.target.value)}
                            className="w-44"
                        >
                            {AUDIBLE_REGIONS.map((r) => (
                                <option key={r.code} value={r.code}>
                                    {r.label}
                                </option>
                            ))}
                        </Select>
                    )}
                </div>
                <div className="flex items-end gap-2">
                    <label className="flex-1 min-w-0">
                        <span className="block mb-1 text-[11px] font-semibold uppercase tracking-wider text-muted-foreground">
                            {source === "audible" ? "Title or ASIN" : "Title"}
                        </span>
                        <Input value={title} onChange={(e) => setTitle(e.target.value)} autoFocus />
                    </label>
                    <label className="w-[38%] min-w-0">
                        <span className="block mb-1 text-[11px] font-semibold uppercase tracking-wider text-muted-foreground">
                            Author
                        </span>
                        <Input value={author} onChange={(e) => setAuthor(e.target.value)} />
                    </label>
                    <Button type="submit" disabled={!title.trim() || busy !== null} className="h-10">
                        <MagnifyingGlassIcon className="w-4 h-4 mr-1.5" />
                        {busy === "search" ? "Searching…" : "Search"}
                    </Button>
                </div>
            </form>

            <div className="flex-1 min-h-0 overflow-y-auto px-3 py-2">
                {exactAsin && (
                    <ResultRow
                        heading="Exact match from this book's ASIN"
                        candidate={{
                            source: "audible",
                            id: exactAsin,
                            title: meta.title || book.title,
                            authors: meta.artist ? [meta.artist] : [],
                            narrators: [],
                            cover_url: null,
                        }}
                        disabled={busy !== null}
                        onPick={() => openRecord("audible", exactAsin)}
                    />
                )}

                {error && <p className="m-2 rounded-md border border-destructive/30 p-2.5 text-sm text-destructive">{error}</p>}

                {busy === "record" && <p className="px-2 py-3 text-sm text-muted-foreground">Fetching details…</p>}

                {results?.length === 0 && (
                    <div className="px-2 py-6 text-center text-sm text-muted-foreground">
                        <p>Nothing found on {SOURCE_LABEL[source]}.</p>
                        {source === "audible" && (
                            <button
                                type="button"
                                className="mt-1 text-foreground underline underline-offset-4 hover:text-primary"
                                onClick={() => {
                                    setSource("open_library");
                                    void search(undefined, "open_library");
                                }}
                            >
                                Search Open Library for “{title.trim()}” instead
                            </button>
                        )}
                    </div>
                )}

                {results?.map((candidate) => (
                    <ResultRow
                        key={candidate.id}
                        candidate={candidate}
                        disabled={busy !== null}
                        onPick={() => openRecord(candidate.source, candidate.id)}
                    />
                ))}

                {results === null && !exactAsin && !error && (
                    <p className="px-2 py-6 text-center text-sm text-muted-foreground">
                        {source === "audible"
                            ? "Search Audible for narrator, series, summary, cover and chapters."
                            : "Open Library covers books that are not on Audible, such as public-domain recordings."}
                    </p>
                )}
            </div>
        </div>
    );
}

function SourceSwitch({ value, onChange }: { value: LookupSource; onChange: (source: LookupSource) => void }) {
    return (
        <div role="radiogroup" aria-label="Source" className="grid grid-cols-2 gap-0.5 rounded-md bg-muted p-0.5 w-56">
            {(["audible", "open_library"] as const).map((option) => {
                const active = option === value;
                return (
                    <button
                        key={option}
                        type="button"
                        role="radio"
                        aria-checked={active}
                        onClick={() => onChange(option)}
                        className={cn(
                            "h-8 rounded-[5px] text-xs font-medium transition-colors",
                            active
                                ? "bg-background dark:bg-accent text-foreground shadow-plane"
                                : "text-muted-foreground hover:text-foreground"
                        )}
                    >
                        {SOURCE_LABEL[option]}
                    </button>
                );
            })}
        </div>
    );
}

function ResultRow({
    candidate,
    heading,
    disabled,
    onPick,
}: {
    candidate: LookupCandidate;
    heading?: string;
    disabled: boolean;
    onPick: () => void;
}) {
    const byline = [
        candidate.authors.join(", "),
        candidate.narrators.length > 0 &&
            `read by ${candidate.narrators.slice(0, 2).join(", ")}${candidate.narrators.length > 2 ? ` +${candidate.narrators.length - 2}` : ""}`,
    ]
        .filter(Boolean)
        .join(" · ");
    const facts = [candidate.runtime_ms ? formatDuration(candidate.runtime_ms) : null, candidate.year].filter(Boolean);

    return (
        <button
            type="button"
            onClick={onPick}
            disabled={disabled}
            className="group w-full flex items-center gap-3 rounded-lg px-2 py-2 text-left transition-colors hover:bg-accent disabled:opacity-50 disabled:pointer-events-none"
        >
            <Thumb url={candidate.cover_url} />
            <span className="min-w-0 flex-1">
                {heading && (
                    <span className="block text-[11px] font-semibold uppercase tracking-wider text-muted-foreground">
                        {heading}
                    </span>
                )}
                <span className="block truncate text-sm font-medium">
                    {candidate.title}
                    {candidate.subtitle && <span className="font-normal text-muted-foreground"> · {candidate.subtitle}</span>}
                </span>
                {byline && <span className="block truncate text-xs text-muted-foreground">{byline}</span>}
                {candidate.series && (
                    <span className="block truncate text-xs text-muted-foreground">
                        {candidate.series}
                        {candidate.series_index && ` #${candidate.series_index}`}
                    </span>
                )}
            </span>
            {facts.length > 0 && (
                <span className="shrink-0 text-right text-xs text-muted-foreground nums">{facts.join(" · ")}</span>
            )}
            <span className="shrink-0 text-xs font-medium text-muted-foreground group-hover:text-foreground">
                {heading ? "Review" : "Review ›"}
            </span>
        </button>
    );
}

function Review({
    book,
    record,
    busy,
    error,
    onBack,
    onApply,
    onClose,
    onRefit,
}: {
    book: AudioBook;
    record: LookupRecord;
    busy: boolean;
    error: string | null;
    onBack: () => void;
    onApply: (payload: ReturnType<typeof applyPayload>, count: number) => void;
    onClose: () => void;
    /** Fetch the record again so its chapters are fitted with the speech model */
    onRefit: () => void;
}) {
    const { speechModel } = useApp();
    const rows = useMemo(
        () =>
            reviewRows(
                {
                    metadata: book.output_metadata ?? {},
                    hasCover: Boolean(book.output_metadata?.artwork_data),
                    chapters: book.chapters,
                },
                record
            ),
        [book, record]
    );
    const [selected, setSelected] = useState<Set<ReviewKey>>(() => initialSelection(rows));
    // A row still being worked out starts unticked; once it is ready it gets
    // the selection it would have had from the start.
    const chaptersRow = rows.find((r) => r.key === "chapters");
    const [chaptersPending, setChaptersPending] = useState(Boolean(chaptersRow?.pending));
    if (chaptersPending && chaptersRow && !chaptersRow.pending) {
        setChaptersPending(false);
        if (chaptersRow.state === "selected" && isSelectable(chaptersRow)) {
            setSelected((prev) => new Set(prev).add("chapters"));
        }
    }
    const toggle = (key: ReviewKey) =>
        setSelected((prev) => {
            const next = new Set(prev);
            if (next.has(key)) next.delete(key);
            else next.add(key);
            return next;
        });

    const count = selected.size;
    // Chapters a speech model could place better, when there is none yet
    const offerSpeech =
        speechModel.status !== null && !speechModel.status.installed && (chaptersRow?.flagged?.length ?? 0) > 0;
    const getSpeechModel = async () => {
        if (await speechModel.download()) onRefit();
    };
    const speechOffer = offerSpeech ? (
        <div className="mt-2 rounded-md border bg-surface-2 px-2.5 py-2 text-xs">
            {speechModel.downloading ? (
                <SpeechModelDownload progress={speechModel.progress} onCancel={speechModel.cancelDownload} />
            ) : (
                <div className="flex items-center justify-between gap-3">
                    <span className="text-muted-foreground">
                        Get the speech model ({formatBytes(speechModel.status?.size_bytes ?? 0)}) to place these by their
                        spoken titles.
                    </span>
                    <Button size="sm" variant="outline" className="h-7 shrink-0" onClick={getSpeechModel}>
                        {speechModel.error ? "Try again" : "Get it"}
                    </Button>
                </div>
            )}
            {speechModel.error && <p className="mt-1 text-destructive">{speechModel.error}</p>}
        </div>
    ) : null;
    const currentCover = book.output_metadata?.artwork_data
        ? `data:image/jpeg;base64,${book.output_metadata.artwork_data}`
        : null;

    return (
        <div className="flex h-full flex-col">
            <header className="flex items-center gap-2 px-3 pt-3 pb-3 border-b no-select">
                <Button variant="ghost" size="sm" onClick={onBack} disabled={busy} className="px-2">
                    <ArrowLeftIcon className="w-4 h-4 mr-1" />
                    Results
                </Button>
                <h2 className="min-w-0 flex-1 truncate text-[15px] font-semibold">
                    {record.fields.title || "Found details"}
                    <span className="ml-2 text-xs font-normal text-muted-foreground">
                        {SOURCE_LABEL[record.source]} {record.source === "audible" ? record.id : ""}
                    </span>
                </h2>
                <CloseButton onClose={onClose} />
            </header>

            <div className="flex-1 min-h-0 overflow-y-auto px-3 py-2">
                <div className="grid grid-cols-[28px_96px_minmax(0,1fr)_minmax(0,1fr)] gap-x-3 px-2 pb-1.5 text-[11px] font-semibold uppercase tracking-wider text-muted-foreground no-select">
                    <span />
                    <span>Field</span>
                    <span>This book</span>
                    <span>Found</span>
                </div>
                {rows.map((row) => (
                    <ReviewLine
                        key={row.key}
                        row={row}
                        checked={selected.has(row.key)}
                        onToggle={() => toggle(row.key)}
                        currentCover={currentCover}
                        disabled={busy}
                        extra={row.key === "chapters" ? speechOffer : null}
                    />
                ))}
            </div>

            <footer className="flex items-center justify-between gap-3 border-t px-5 py-3">
                <p className="min-w-0 text-sm text-destructive truncate" title={error ?? undefined}>
                    {error}
                </p>
                <div className="flex shrink-0 gap-2">
                    <Button variant="outline" size="sm" onClick={onClose} disabled={busy}>
                        Cancel
                    </Button>
                    <Button
                        size="sm"
                        disabled={count === 0 || busy}
                        onClick={() => onApply(applyPayload(record, selected), count)}
                    >
                        {busy ? "Applying…" : `Apply ${count} ${count === 1 ? "change" : "changes"}`}
                    </Button>
                </div>
            </footer>
        </div>
    );
}

function ReviewLine({
    row,
    checked,
    onToggle,
    currentCover,
    disabled,
    extra,
}: {
    row: ReviewRow;
    checked: boolean;
    onToggle: () => void;
    currentCover: string | null;
    disabled: boolean;
    /** Shown under the found value, such as the speech model offer */
    extra?: ReactNode;
}) {
    const selectable = isSelectable(row);
    const id = `review-${row.key}`;
    const value = (text: string, side: "current" | "found") => {
        if (row.key === "cover") {
            const url = side === "current" ? currentCover : row.found;
            return url ? <Thumb url={url} size="lg" /> : <Empty />;
        }
        if (!text) return <Empty />;
        return (
            <span className={cn("block break-words", row.key === "description" && "line-clamp-3 whitespace-pre-line")}>
                {text}
            </span>
        );
    };

    return (
        <label
            htmlFor={id}
            className={cn(
                "grid grid-cols-[28px_96px_minmax(0,1fr)_minmax(0,1fr)] items-start gap-x-3 rounded-md px-2 py-2 text-sm",
                selectable ? "cursor-pointer hover:bg-accent/60" : "text-muted-foreground"
            )}
        >
            <span className="flex h-5 items-center">
                {row.state === "unchanged" ? (
                    <CheckIcon className="w-4 h-4 text-success" aria-label="Unchanged" />
                ) : (
                    <input
                        id={id}
                        type="checkbox"
                        checked={checked}
                        onChange={onToggle}
                        disabled={!selectable || disabled}
                        className="w-4 h-4 accent-[hsl(var(--primary))] disabled:opacity-40"
                    />
                )}
            </span>
            <span className="font-medium text-foreground">{row.label}</span>
            {value(row.current, "current")}
            {row.unavailable ? (
                <span className="text-xs text-warning">{row.unavailable}</span>
            ) : row.pending ? (
                <span className="text-xs text-muted-foreground animate-pulse">{row.pending}</span>
            ) : row.state === "unchanged" ? (
                <span className="text-xs">Same</span>
            ) : (
                <span className="min-w-0">
                    {value(row.found, "found")}
                    {row.flagged && row.flagged.length > 0 && <FlaggedList flagged={row.flagged} />}
                    {extra}
                </span>
            )}
        </label>
    );
}

/** Fitted chapters that did not land on a track start: where they went, and how far they moved. */
function FlaggedList({ flagged }: { flagged: FlaggedChapter[] }) {
    return (
        <ul className="mt-1.5 space-y-0.5 text-xs text-muted-foreground">
            {flagged.map((chapter) => (
                <li key={`${chapter.start_ms}-${chapter.title}`} className="flex gap-2">
                    <span className="nums shrink-0 text-warning">{formatClock(chapter.start_ms)}</span>
                    <span className="min-w-0 truncate" title={chapter.title}>
                        {chapter.title}
                    </span>
                    <span className="nums ml-auto shrink-0">
                        {chapter.kind === "pause" ? `at a pause, ${formatMoved(chapter.moved_ms)}` : "estimated"}
                    </span>
                </li>
            ))}
        </ul>
    );
}

/** "+2.4 s" */
function formatMoved(ms: number): string {
    return `${ms >= 0 ? "+" : "−"}${(Math.abs(ms) / 1000).toFixed(1)} s`;
}

function Thumb({ url, size = "sm" }: { url?: string | null; size?: "sm" | "lg" }) {
    const box = size === "lg" ? "w-16 h-16" : "w-11 h-11";
    return url ? (
        <img src={url} alt="" className={cn(box, "shrink-0 rounded object-cover bg-muted border")} />
    ) : (
        <span className={cn(box, "shrink-0 rounded bg-muted border flex items-center justify-center text-muted-foreground")}>
            <PhotoIcon className="w-4 h-4" />
        </span>
    );
}

function Empty() {
    return <span className="text-muted-foreground/70">—</span>;
}

function CloseButton({ onClose }: { onClose: () => void }) {
    return (
        <button
            type="button"
            onClick={onClose}
            aria-label="Close"
            className="h-8 w-8 shrink-0 rounded-md flex items-center justify-center text-muted-foreground hover:bg-accent hover:text-foreground"
        >
            <XMarkIcon className="w-4 h-4" />
        </button>
    );
}
