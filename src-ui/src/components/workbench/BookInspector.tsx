import { useEffect, useRef, useState } from "react";
import {
    PhotoIcon,
    MagnifyingGlassIcon,
    SparklesIcon,
    TrashIcon,
    ChevronDownIcon,
    ChevronRightIcon,
} from "@heroicons/react/24/outline";
import { FindDetailsDialog } from "@/components/workbench/FindDetailsDialog";
import { useToast } from "@/components/ui/use-toast";
import { useApp } from "@/context/useApp";
import * as FilesApi from "@/api/files";
import * as MetadataApi from "@/api/metadata";
import type { Metadata } from "@/types/models";
import { cn } from "@/lib/utils";

/**
 * The inspector shows the artifact being produced — cover and tags — and stays
 * on screen while files and chapters change, so edits are never a context
 * switch away from the thing they describe.
 */
export function BookInspector() {
    const { activeAudiobook } = useApp();
    const [showMore, setShowMore] = useState(false);
    const [finding, setFinding] = useState(false);

    if (!activeAudiobook) return null;

    return (
        <aside className="w-[264px] lg:w-[312px] shrink-0 border-l bg-surface flex flex-col overflow-y-auto">
            <div className="flex items-center justify-between px-4 pt-3 -mb-1 no-select">
                <span className="text-[11px] font-semibold uppercase tracking-wider text-muted-foreground">Details</span>
                <CoverAction icon={<MagnifyingGlassIcon className="w-3.5 h-3.5" />} onClick={() => setFinding(true)}>
                    Find details…
                </CoverAction>
            </div>
            <CoverWell onFindOnline={() => setFinding(true)} />
            <FindDetailsDialog open={finding} onClose={() => setFinding(false)} />

            <div className="px-4 pb-5 space-y-3.5">
                <Field name="title" label="Title" placeholder="Book title" />
                <Field name="artist" label="Author" placeholder="Author name" />
                <Field name="narrator" label="Narrator" placeholder="Who reads it" />

                <div className="grid grid-cols-[1fr_68px] gap-2">
                    <Field name="series" label="Series" placeholder="Series name" />
                    <Field name="series_index" label="#" placeholder="1" />
                </div>

                <div className="grid grid-cols-2 gap-2">
                    <Field name="year" label="Year" numeric placeholder="2024" />
                    <Field name="genre" label="Genre" placeholder="Fiction" />
                </div>

                <button
                    onClick={() => setShowMore((v) => !v)}
                    aria-expanded={showMore}
                    className="flex items-center gap-1 -ml-1 px-1 h-7 rounded-md text-xs font-medium text-muted-foreground hover:text-foreground"
                >
                    {showMore ? (
                        <ChevronDownIcon className="w-3.5 h-3.5" />
                    ) : (
                        <ChevronRightIcon className="w-3.5 h-3.5" />
                    )}
                    More fields
                </button>

                {showMore && (
                    <div className="space-y-3.5 animate-fade-in">
                        <Field name="album" label="Album" placeholder="Album tag" />
                        <Field name="album_artist" label="Album artist" placeholder="Album artist tag" />
                        <Field name="publisher" label="Publisher" placeholder="Publisher" />
                        <Field name="language" label="Language" placeholder="eng" />
                        <Field name="asin" label="ASIN" placeholder="Audible product ID" />
                        <Field name="copyright" label="Copyright" placeholder="© 2024" />
                        <Field name="description" label="Description" multiline placeholder="Blurb or summary" />
                        <Field name="comments" label="Comments" multiline placeholder="Private notes" />
                    </div>
                )}
            </div>
        </aside>
    );
}

function CoverWell({ onFindOnline }: { onFindOnline: () => void }) {
    const { activeAudiobook, setActiveAudiobook, settings } = useApp();
    const { toast } = useToast();
    const [busy, setBusy] = useState(false);
    const autoDetectedFor = useRef<string | null>(null);

    const metadata = activeAudiobook?.output_metadata;
    const preview = metadata?.artwork_data ? `data:image/jpeg;base64,${metadata.artwork_data}` : null;

    /**
     * Importing a book usually means its cover is already embedded in the files
     * or sitting beside them, so claim it without being asked. Runs once per
     * book and never over a cover that is already set.
     */
    useEffect(() => {
        const book = activeAudiobook;
        if (!book || settings?.auto_detect_artwork === false) return;
        if (book.output_metadata?.artwork_data) return;
        if (autoDetectedFor.current === book.id) return;

        autoDetectedFor.current = book.id;
        let cancelled = false;

        MetadataApi.discoverArtwork(book.id)
            .then((found) => {
                if (cancelled || found.length === 0) return;
                return MetadataApi.selectDiscoveredArtwork(book.id, found[0].source).then((updated) => {
                    if (!cancelled) setActiveAudiobook(updated);
                });
            })
            .catch((error) => console.error("Artwork auto-detect failed:", error));

        return () => {
            cancelled = true;
        };
    }, [activeAudiobook?.id, settings?.auto_detect_artwork, setActiveAudiobook]);

    const choose = async () => {
        if (!activeAudiobook) return;
        setBusy(true);
        try {
            const path = await FilesApi.pickArtworkFile();
            if (!path) return;
            setActiveAudiobook(await MetadataApi.uploadArtwork(activeAudiobook.id, path));
        } catch (error) {
            toast({ title: "Could not set cover", description: String(error), variant: "destructive" });
        } finally {
            setBusy(false);
        }
    };

    const autoDetect = async () => {
        if (!activeAudiobook) return;
        setBusy(true);
        try {
            const found = await MetadataApi.discoverArtwork(activeAudiobook.id);
            if (found.length === 0) {
                toast({
                    title: "No cover found",
                    description: "Nothing embedded in the files and no image beside them.",
                });
                return;
            }
            setActiveAudiobook(
                await MetadataApi.selectDiscoveredArtwork(activeAudiobook.id, found[0].source)
            );
            toast({ title: "Cover found", description: found[0].name });
        } catch (error) {
            toast({ title: "Cover search failed", description: String(error), variant: "destructive" });
        } finally {
            setBusy(false);
        }
    };

    const clear = async () => {
        if (!activeAudiobook) return;
        setBusy(true);
        try {
            setActiveAudiobook(await MetadataApi.clearArtwork(activeAudiobook.id));
        } catch (error) {
            toast({ title: "Could not remove cover", description: String(error), variant: "destructive" });
        } finally {
            setBusy(false);
        }
    };

    return (
        <div className="p-4 space-y-2">
            <button
                onClick={choose}
                disabled={busy}
                aria-label={preview ? "Replace cover art" : "Add cover art"}
                className={cn(
                    // Capped so title through genre stay visible at the default window height.
                    "group relative mx-auto block w-full max-w-[220px] aspect-square rounded-lg overflow-hidden border shadow-plane",
                    "flex items-center justify-center transition-colors",
                    preview ? "bg-muted" : "border-dashed bg-muted/40 hover:border-primary/60 hover:bg-primary/[0.04]",
                    busy && "opacity-60 pointer-events-none"
                )}
            >
                {preview ? (
                    <>
                        <img src={preview} alt="" className="w-full h-full object-cover" />
                        <span className="absolute inset-0 bg-foreground/55 opacity-0 group-hover:opacity-100 transition-opacity flex items-center justify-center text-xs font-medium text-background">
                            Replace cover
                        </span>
                    </>
                ) : (
                    <span className="flex flex-col items-center gap-1.5 text-muted-foreground">
                        <PhotoIcon className="w-7 h-7" />
                        <span className="text-xs font-medium">Add cover art</span>
                    </span>
                )}
            </button>

            <div className="flex items-center justify-center gap-1">
                <CoverAction icon={<SparklesIcon className="w-3.5 h-3.5" />} onClick={autoDetect} disabled={busy}>
                    Auto
                </CoverAction>

                <CoverAction icon={<MagnifyingGlassIcon className="w-3.5 h-3.5" />} onClick={onFindOnline} disabled={busy}>
                    Find online
                </CoverAction>

                {preview && (
                    <CoverAction
                        icon={<TrashIcon className="w-3.5 h-3.5" />}
                        onClick={clear}
                        disabled={busy}
                        label="Remove cover art"
                        danger
                    />
                )}
            </div>
        </div>
    );
}

function CoverAction({
    icon,
    children,
    onClick,
    disabled,
    danger,
    label,
}: {
    icon: React.ReactNode;
    children?: React.ReactNode;
    onClick?: () => void;
    disabled?: boolean;
    danger?: boolean;
    /** Required when the action is icon-only. */
    label?: string;
}) {
    return (
        <button
            onClick={onClick}
            disabled={disabled}
            aria-label={label}
            title={label}
            className={cn(
                "h-7 px-2 rounded-md flex items-center gap-1.5 text-xs font-medium transition-colors",
                "disabled:opacity-40 disabled:pointer-events-none",
                danger
                    ? "text-destructive hover:bg-destructive/10"
                    : "text-muted-foreground hover:bg-accent hover:text-foreground"
            )}
        >
            {icon}
            {children}
        </button>
    );
}

type TextKey = Extract<
    keyof Metadata,
    | "title"
    | "artist"
    | "narrator"
    | "album"
    | "album_artist"
    | "series"
    | "series_index"
    | "genre"
    | "publisher"
    | "language"
    | "asin"
    | "copyright"
    | "description"
    | "comments"
>;
type NumberKey = Extract<keyof Metadata, "year">;

/**
 * Commits on blur rather than per keystroke: every write is an undo entry and a
 * backend round trip, so one entry per field edit is the useful granularity.
 */
function Field({
    name,
    label,
    placeholder,
    numeric,
    multiline,
}: {
    name: TextKey | NumberKey;
    label: string;
    placeholder?: string;
    numeric?: boolean;
    multiline?: boolean;
}) {
    const { activeAudiobook, setActiveAudiobook, refreshUndoState } = useApp();
    const { toast } = useToast();
    const stored = activeAudiobook?.output_metadata?.[name];
    const [value, setValue] = useState(stored == null ? "" : String(stored));
    const [failed, setFailed] = useState(false);
    const committed = useRef(value);

    // Re-sync when the book changes underneath us (import, undo, Find details).
    useEffect(() => {
        const next = stored == null ? "" : String(stored);
        if (next !== committed.current) {
            committed.current = next;
            setValue(next);
        }
    }, [stored, activeAudiobook?.id]);

    const commit = async () => {
        if (!activeAudiobook) return;
        const trimmed = value.trim();
        if (trimmed === committed.current) return;

        const patch: Metadata = { ...activeAudiobook.output_metadata };
        if (numeric) {
            const parsed = Number.parseInt(trimmed, 10);
            (patch[name as NumberKey] as number | undefined) = Number.isFinite(parsed) ? parsed : undefined;
        } else {
            (patch[name as TextKey] as string | undefined) = trimmed || undefined;
        }

        const previous = committed.current;
        committed.current = trimmed;
        try {
            setActiveAudiobook(await MetadataApi.updateAudiobookMetadata(activeAudiobook.id, patch));
            setFailed(false);
            await refreshUndoState();
        } catch (error) {
            // Keep the typed text so it is not lost, but leave the field marked
            // unsaved so the next blur retries instead of assuming it landed.
            committed.current = previous;
            setFailed(true);
            toast({ title: `Could not save ${label}`, description: String(error), variant: "destructive" });
        }
    };

    const revert = () => {
        setValue(committed.current);
        setFailed(false);
    };

    const shared = cn(
        "w-full rounded-md border border-input bg-background px-2.5 py-1.5 text-sm transition-colors",
        "placeholder:text-muted-foreground/70 hover:border-muted-foreground/40",
        "focus:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:border-ring",
        failed && "border-destructive"
    );

    return (
        <label className="block">
            <span className="block mb-1 text-[11px] font-semibold uppercase tracking-wider text-muted-foreground">
                {label}
            </span>
            {multiline ? (
                <textarea
                    rows={3}
                    value={value}
                    placeholder={placeholder}
                    onChange={(e) => setValue(e.target.value)}
                    onBlur={commit}
                    onKeyDown={(e) => {
                        if (e.key === "Escape") revert();
                    }}
                    aria-invalid={failed || undefined}
                    className={cn(shared, "resize-y min-h-[64px]")}
                />
            ) : (
                <input
                    type="text"
                    inputMode={numeric ? "numeric" : undefined}
                    value={value}
                    placeholder={placeholder}
                    onChange={(e) => setValue(e.target.value)}
                    onBlur={commit}
                    onKeyDown={(e) => {
                        if (e.key === "Enter") (e.target as HTMLInputElement).blur();
                        if (e.key === "Escape") revert();
                    }}
                    aria-invalid={failed || undefined}
                    className={cn(shared, numeric && "nums")}
                />
            )}
        </label>
    );
}
