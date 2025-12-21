import { Button } from "./ui/button";
import { useApp } from "@/context/AppContext";

export function UndoRedoButtons() {
    const { undoState, handleUndo, handleRedo } = useApp();

    return (
        <div className="flex gap-1">
            <Button
                variant="ghost"
                size="sm"
                onClick={handleUndo}
                disabled={!undoState.canUndo}
                title={undoState.undoDescription ? `Undo: ${undoState.undoDescription}` : "Nothing to undo"}
            >
                <svg className="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M3 10h10a4 4 0 0 1 4 4v0a4 4 0 0 1-4 4H3m0-8l4-4m-4 4l4 4" />
                </svg>
                <span className="sr-only">Undo</span>
            </Button>
            <Button
                variant="ghost"
                size="sm"
                onClick={handleRedo}
                disabled={!undoState.canRedo}
                title={undoState.redoDescription ? `Redo: ${undoState.redoDescription}` : "Nothing to redo"}
            >
                <svg className="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M21 10H11a4 4 0 0 0-4 4v0a4 4 0 0 0 4 4h10m0-8l-4-4m4 4l-4 4" />
                </svg>
                <span className="sr-only">Redo</span>
            </Button>
        </div>
    );
}
