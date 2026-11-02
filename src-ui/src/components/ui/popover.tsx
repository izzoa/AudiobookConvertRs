import { useEffect, useRef, useState, type ReactNode } from "react";
import { cn } from "@/lib/utils";

interface PopoverProps {
    /** Receives `open` so triggers can show their own pressed state. */
    trigger: (open: boolean) => ReactNode;
    children: ReactNode | ((close: () => void) => ReactNode);
    align?: "start" | "end";
    side?: "top" | "bottom";
    className?: string;
}

/**
 * Minimal anchored panel. Radix is only pulled in for primitives that need
 * focus trapping; a menu that closes on outside-click and Escape does not.
 */
export function Popover({ trigger, children, align = "start", side = "bottom", className }: PopoverProps) {
    const [open, setOpen] = useState(false);
    const rootRef = useRef<HTMLDivElement>(null);

    useEffect(() => {
        if (!open) return;
        const onPointerDown = (e: MouseEvent) => {
            if (!rootRef.current?.contains(e.target as Node)) setOpen(false);
        };
        const onKeyDown = (e: KeyboardEvent) => {
            if (e.key === "Escape") setOpen(false);
        };
        document.addEventListener("mousedown", onPointerDown);
        document.addEventListener("keydown", onKeyDown);
        return () => {
            document.removeEventListener("mousedown", onPointerDown);
            document.removeEventListener("keydown", onKeyDown);
        };
    }, [open]);

    const close = () => setOpen(false);

    return (
        <div ref={rootRef} className="relative">
            <div onClick={() => setOpen((v) => !v)}>{trigger(open)}</div>
            {open && (
                <div
                    className={cn(
                        "absolute z-50 min-w-[220px] rounded-lg border bg-popover text-popover-foreground",
                        "shadow-lift p-1.5 animate-slide-up",
                        side === "bottom" ? "top-[calc(100%+6px)]" : "bottom-[calc(100%+6px)]",
                        align === "start" ? "left-0" : "right-0",
                        className
                    )}
                >
                    {typeof children === "function" ? children(close) : children}
                </div>
            )}
        </div>
    );
}

interface MenuItemProps {
    icon?: ReactNode;
    children: ReactNode;
    onClick?: () => void;
    disabled?: boolean;
    danger?: boolean;
    shortcut?: string;
}

export function MenuItem({ icon, children, onClick, disabled, danger, shortcut }: MenuItemProps) {
    return (
        <button
            onClick={onClick}
            disabled={disabled}
            className={cn(
                "w-full flex items-center gap-2.5 px-2.5 py-[7px] rounded-md text-sm text-left",
                "transition-colors disabled:opacity-40 disabled:pointer-events-none",
                danger ? "text-destructive hover:bg-destructive/10" : "hover:bg-accent"
            )}
        >
            {icon && <span className="w-4 h-4 shrink-0 opacity-70">{icon}</span>}
            <span className="flex-1 truncate">{children}</span>
            {shortcut && <span className="text-xs text-muted-foreground nums">{shortcut}</span>}
        </button>
    );
}

export function MenuSeparator() {
    return <div className="my-1.5 h-px bg-border" />;
}

export function MenuLabel({ children }: { children: ReactNode }) {
    return (
        <div className="px-2.5 pt-2 pb-1 text-[11px] font-semibold uppercase tracking-wider text-muted-foreground">
            {children}
        </div>
    );
}
