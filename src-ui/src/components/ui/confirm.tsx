import { createContext, useCallback, useContext, useEffect, useRef, useState, type ReactNode } from "react";
import { Button } from "@/components/ui/button";

interface ConfirmOptions {
    title: string;
    description?: ReactNode;
    confirmLabel?: string;
    cancelLabel?: string;
    danger?: boolean;
}

type Confirm = (options: ConfirmOptions) => Promise<boolean>;

const ConfirmContext = createContext<Confirm | null>(null);

/**
 * In-app replacement for `window.confirm`.
 *
 * The native one renders as an unstyled system sheet, cannot name its buttons,
 * and is not guaranteed to exist inside a Tauri webview. This uses `<dialog>`
 * so focus is trapped and Escape cancels without any extra code.
 */
export function ConfirmProvider({ children }: { children: ReactNode }) {
    const [request, setRequest] = useState<(ConfirmOptions & { resolve: (ok: boolean) => void }) | null>(null);
    const dialogRef = useRef<HTMLDialogElement>(null);

    const confirm = useCallback<Confirm>(
        (options) => new Promise<boolean>((resolve) => setRequest({ ...options, resolve })),
        []
    );

    useEffect(() => {
        const dialog = dialogRef.current;
        if (request && dialog && !dialog.open) dialog.showModal();
    }, [request]);

    const settle = (ok: boolean) => {
        request?.resolve(ok);
        dialogRef.current?.close();
        setRequest(null);
    };

    return (
        <ConfirmContext.Provider value={confirm}>
            {children}
            <dialog
                ref={dialogRef}
                onCancel={(e) => {
                    e.preventDefault();
                    settle(false);
                }}
                className="m-auto w-[min(420px,calc(100vw-32px))] rounded-xl border bg-popover p-0 text-popover-foreground shadow-lift backdrop:bg-[hsl(var(--scrim))] open:animate-slide-up"
            >
                {request && (
                    <div className="p-5">
                        <h2 className="text-[15px] font-semibold leading-snug">{request.title}</h2>
                        {request.description && (
                            <div className="mt-1.5 text-sm text-muted-foreground break-words">{request.description}</div>
                        )}
                        <div className="mt-5 flex justify-end gap-2">
                            <Button variant="outline" size="sm" onClick={() => settle(false)}>
                                {request.cancelLabel ?? "Cancel"}
                            </Button>
                            <Button
                                size="sm"
                                autoFocus
                                variant={request.danger ? "destructive" : "default"}
                                onClick={() => settle(true)}
                            >
                                {request.confirmLabel ?? "OK"}
                            </Button>
                        </div>
                    </div>
                )}
            </dialog>
        </ConfirmContext.Provider>
    );
}

export function useConfirm(): Confirm {
    const confirm = useContext(ConfirmContext);
    if (!confirm) throw new Error("useConfirm must be used inside ConfirmProvider");
    return confirm;
}
