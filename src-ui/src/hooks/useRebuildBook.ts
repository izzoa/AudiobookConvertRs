import { useCallback } from "react";
import { useToast } from "@/components/ui/use-toast";
import { useConfirm } from "@/components/ui/confirm";
import { useApp } from "@/context/useApp";
import { useBatch } from "@/context/useBatch";
import * as AudiobookApi from "@/api/audiobook";
import * as MetadataApi from "@/api/metadata";

/**
 * Rebuilds the open book from a new ordered file list (reorder, sort, remove,
 * add).
 *
 * The backend builds an audiobook from an ordered path list and has no
 * "move file" command, so changing the files means rebuilding. Typed metadata
 * is re-applied afterwards; chapters are derived from the files and are
 * regenerated, which is why hand-edited chapters prompt first. Resolves true
 * when the book was rebuilt.
 */
export function useRebuildBook() {
    const { activeAudiobook, setActiveAudiobook } = useApp();
    const { replaceBook } = useBatch();
    const { toast } = useToast();
    const confirm = useConfirm();

    return useCallback(
        async (paths: string[], action: string): Promise<boolean> => {
            if (!activeAudiobook) return false;
            const hasManualChapters = activeAudiobook.chapters.some((c) => !c.auto_detected);
            if (
                hasManualChapters &&
                !(await confirm({
                    title: "Replace your edited chapters?",
                    description: `${action} rebuilds the book from its files, which recreates its chapters from the files' tags and names. Chapter titles you edited will be lost.`,
                    confirmLabel: "Rebuild anyway",
                    danger: true,
                }))
            ) {
                return false;
            }

            const metadata = activeAudiobook.output_metadata;
            try {
                let book = await AudiobookApi.createAudiobookFromFiles(paths);
                if (metadata && Object.keys(metadata).length > 0) {
                    book = await MetadataApi.updateAudiobookMetadata(book.id, metadata);
                }
                replaceBook(activeAudiobook.id, book);
                setActiveAudiobook(book);
                return true;
            } catch (error) {
                toast({ title: `${action} failed`, description: String(error), variant: "destructive" });
                return false;
            }
        },
        [activeAudiobook, confirm, replaceBook, setActiveAudiobook, toast]
    );
}
