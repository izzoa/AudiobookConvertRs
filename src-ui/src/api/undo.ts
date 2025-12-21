import { invoke } from '@tauri-apps/api/core';
import type { UndoState } from '../types/models';

export async function undo(): Promise<string | null> {
    return await invoke('undo');
}

export async function redo(): Promise<string | null> {
    return await invoke('redo');
}

export async function canUndo(): Promise<boolean> {
    return await invoke('can_undo');
}

export async function canRedo(): Promise<boolean> {
    return await invoke('can_redo');
}

export async function getUndoDescription(): Promise<string | null> {
    return await invoke('get_undo_description');
}

export async function getRedoDescription(): Promise<string | null> {
    return await invoke('get_redo_description');
}

export async function getUndoState(): Promise<UndoState> {
    const [canUndoResult, canRedoResult, undoDesc, redoDesc] = await Promise.all([
        canUndo(),
        canRedo(),
        getUndoDescription(),
        getRedoDescription(),
    ]);
    return {
        canUndo: canUndoResult,
        canRedo: canRedoResult,
        undoDescription: undoDesc ?? undefined,
        redoDescription: redoDesc ?? undefined,
    };
}
