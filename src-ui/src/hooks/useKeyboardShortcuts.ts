import { useEffect, useCallback } from 'react';
import { useApp } from '@/context/AppContext';

export interface KeyboardShortcutHandlers {
    onOpen?: () => void;
    onSave?: () => void;
    onCancel?: () => void;
}

export function useKeyboardShortcuts(handlers: KeyboardShortcutHandlers = {}) {
    const { handleUndo, handleRedo, undoState } = useApp();

    const handleKeyDown = useCallback((event: KeyboardEvent) => {
        const isMod = event.metaKey || event.ctrlKey;
        
        if (isMod && event.key === 'z' && !event.shiftKey) {
            event.preventDefault();
            if (undoState.canUndo) {
                handleUndo();
            }
            return;
        }

        if ((isMod && event.key === 'z' && event.shiftKey) || (isMod && event.key === 'y')) {
            event.preventDefault();
            if (undoState.canRedo) {
                handleRedo();
            }
            return;
        }

        if (isMod && event.key === 'o') {
            event.preventDefault();
            handlers.onOpen?.();
            return;
        }

        if (isMod && event.key === 's') {
            event.preventDefault();
            handlers.onSave?.();
            return;
        }

        if (event.key === 'Escape') {
            handlers.onCancel?.();
            return;
        }
    }, [handleUndo, handleRedo, undoState, handlers]);

    useEffect(() => {
        window.addEventListener('keydown', handleKeyDown);
        return () => {
            window.removeEventListener('keydown', handleKeyDown);
        };
    }, [handleKeyDown]);
}
