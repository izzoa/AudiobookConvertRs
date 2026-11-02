import {
    BookOpenIcon,
    QueueListIcon,
    Cog6ToothIcon,
    QuestionMarkCircleIcon,
    RectangleStackIcon,
} from "@heroicons/react/24/outline";
import { cn } from "@/lib/utils";
import { useApp, type AppView } from "@/context/AppContext";
import { useConversionStore } from "@/context/ConversionStore";
import { useBatch } from "@/context/BatchStore";
import { JobStatus } from "@/types/models";

/**
 * The rail is navigation only — never actions.
 *
 * The old sidebar mixed destinations (Jobs, Settings) with commands (Import
 * Files, Open Project), so nothing in it had a predictable result. Commands now
 * live where their object is: in the book pane and the top bar.
 */
export function Rail() {
    const { view, setView } = useApp();
    const { jobs } = useConversionStore();
    const { entries } = useBatch();

    const activeCount = jobs.filter(
        (j) => j.status === JobStatus.Running || j.status === JobStatus.Queued
    ).length;
    const failedCount = jobs.filter((j) => j.status === JobStatus.Error).length;

    return (
        <nav className="w-[68px] shrink-0 bg-rail border-r flex flex-col items-center py-2 gap-1 no-select">
            <RailItem
                icon={<BookOpenIcon className="w-[22px] h-[22px]" />}
                label="Book"
                target="book"
                current={view}
                onSelect={setView}
            />
            {/* Only while there is a batch to return to; it is a place, not a mode. */}
            {entries.length > 0 && (
                <RailItem
                    icon={<RectangleStackIcon className="w-[22px] h-[22px]" />}
                    label="Batch"
                    target="batch"
                    current={view}
                    onSelect={setView}
                    badge={entries.length}
                    badgeLabel="books"
                />
            )}
            <RailItem
                icon={<QueueListIcon className="w-[22px] h-[22px]" />}
                label="Queue"
                target="queue"
                current={view}
                onSelect={setView}
                badge={activeCount || undefined}
                alert={failedCount > 0 ? `${failedCount} failed` : undefined}
            />

            <div className="flex-1" />

            <RailItem
                icon={<Cog6ToothIcon className="w-[22px] h-[22px]" />}
                label="Settings"
                target="settings"
                current={view}
                onSelect={setView}
            />
            <RailItem
                icon={<QuestionMarkCircleIcon className="w-[22px] h-[22px]" />}
                label="Help"
                target="help"
                current={view}
                onSelect={setView}
            />
        </nav>
    );
}

interface RailItemProps {
    icon: React.ReactNode;
    label: string;
    target: AppView;
    current: AppView;
    onSelect: (view: AppView) => void;
    badge?: number;
    /** What the badge counts, for screen readers. */
    badgeLabel?: string;
    /** A failure needs noticing even while other jobs are running, so it is its own mark. */
    alert?: string;
}

function RailItem({ icon, label, target, current, onSelect, badge, badgeLabel = "active", alert }: RailItemProps) {
    const active = current === target;
    return (
        <button
            onClick={() => onSelect(target)}
            aria-label={badge || alert ? [label, badge && `${badge} ${badgeLabel}`, alert].filter(Boolean).join(", ") : undefined}
            aria-current={active ? "page" : undefined}
            className={cn(
                "relative w-[58px] h-[54px] rounded-lg flex flex-col items-center justify-center gap-[3px]",
                "transition-colors",
                active
                    ? "bg-surface text-primary shadow-plane ring-1 ring-border"
                    : "text-muted-foreground hover:bg-accent/60 hover:text-foreground"
            )}
        >
            <span className="relative">
                {icon}
                {badge !== undefined && badge > 0 && (
                    <span className="absolute -top-1.5 -right-2.5 min-w-[16px] h-[16px] px-1 rounded-full bg-primary text-primary-foreground text-[10px] font-semibold leading-[16px] text-center nums">
                        {badge > 99 ? "99+" : badge}
                    </span>
                )}
                {alert && (
                    <span
                        title={alert}
                        className="absolute -bottom-0.5 -right-1 w-2 h-2 rounded-full bg-destructive ring-2 ring-rail"
                    />
                )}
            </span>
            <span className="text-[10px] font-medium tracking-wide">{label}</span>
        </button>
    );
}
