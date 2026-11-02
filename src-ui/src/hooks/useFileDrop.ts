import { useEffect, useState } from "react";
import { getCurrentWebview } from "@tauri-apps/api/webview";

/**
 * Tauri's unlisten is async, so a plain call would surface teardown failures as
 * unhandled rejections — noisy during hot reload and in non-Tauri previews.
 */
function detach(unlisten?: () => void) {
    if (!unlisten) return;
    try {
        Promise.resolve(unlisten() as unknown).catch(() => {});
    } catch {
        /* nothing to detach */
    }
}

/**
 * Window-level drag and drop.
 *
 * Browser drag events never expose real filesystem paths inside a webview, so
 * this listens to Tauri's native drag-drop stream instead.
 *
 * Every dropped path goes to the backend unfiltered. The webview cannot tell a
 * folder from a file — "Vol. 2" or anything under "/Users/john.doe" looks like
 * a file by its dots — while the backend can: it expands folders, skips files
 * it cannot read, and names the problem when nothing usable was dropped.
 */
export function useFileDrop(onDrop: (paths: string[]) => void, enabled = true) {
    const [isDraggingOver, setIsDraggingOver] = useState(false);

    useEffect(() => {
        if (!enabled) return;
        let unlisten: (() => void) | undefined;
        let cancelled = false;

        // Outside a Tauri webview (browser preview, tests) the API is absent;
        // the app still works, it just cannot receive native drops.
        let webview: ReturnType<typeof getCurrentWebview>;
        try {
            webview = getCurrentWebview();
        } catch {
            return;
        }

        webview
            .onDragDropEvent((event) => {
                if (event.payload.type === "over") {
                    setIsDraggingOver(true);
                } else if (event.payload.type === "leave") {
                    setIsDraggingOver(false);
                } else if (event.payload.type === "drop") {
                    setIsDraggingOver(false);
                    const paths = event.payload.paths ?? [];
                    if (paths.length > 0) onDrop(paths);
                }
            })
            .then((fn) => {
                if (cancelled) detach(fn);
                else unlisten = fn;
            })
            .catch((error) => {
                console.error("Failed to attach drag-drop listener:", error);
            });

        return () => {
            cancelled = true;
            detach(unlisten);
        };
    }, [onDrop, enabled]);

    return isDraggingOver;
}
