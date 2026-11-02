import { createContext, useContext } from 'react';
import type { AudioBook, UserSettings, ConversionJob, UndoState, ConversionDefaults, Project, ConversionSettings } from '../types/models';

// Lives apart from AppProvider so that editing the provider hot-reloads in place.

export type AppView = 'book' | 'batch' | 'queue' | 'settings' | 'help';

export interface AppState {
    // Navigation
    view: AppView;
    setView: (view: AppView) => void;

    // Project state
    activeAudiobook: AudioBook | null;
    setActiveAudiobook: (audiobook: AudioBook | null) => void;
    activeProject: Project | null;
    setActiveProject: (project: Project | null) => void;

    // Settings
    settings: UserSettings | null;
    updateSettings: (settings: UserSettings) => Promise<void>;
    refreshSettings: () => Promise<void>;

    // Effective defaults (project overrides > global)
    getEffectiveConversionDefaults: () => ConversionDefaults;

    // Pending conversion settings (set by ConversionSettingsDialog, used by DryRunPreview)
    pendingConversionSettings: ConversionSettings | null;
    setPendingConversionSettings: (settings: ConversionSettings | null) => void;

    // Conversion queue
    conversionQueue: ConversionJob[];
    addToQueue: (job: ConversionJob) => void;

    // Loading state
    isLoading: boolean;

    // Undo/Redo
    undoState: UndoState;
    handleUndo: () => Promise<void>;
    handleRedo: () => Promise<void>;
    refreshUndoState: () => Promise<void>;
}

export const AppContext = createContext<AppState | undefined>(undefined);

export function useApp() {
    const context = useContext(AppContext);
    if (context === undefined) {
        throw new Error('useApp must be used within an AppProvider');
    }
    return context;
}
