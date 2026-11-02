import { createContext, useContext } from 'react';
import type {
    AudioBook,
    UserSettings,
    ConversionJob,
    UndoState,
    ConversionDefaults,
    Project,
    ConversionSettings,
    FFmpegDownloadProgress,
    FFmpegStatus,
} from '../types/models';

// Lives apart from AppProvider so that editing the provider hot-reloads in place.

export type AppView = 'book' | 'batch' | 'queue' | 'settings' | 'help';

/** A Settings section other views can open directly. */
export type SettingsSection = 'ffmpeg';

/** FFmpeg's status and the actions that change it. App-wide, so a download keeps running when the user leaves Settings. */
export interface FFmpegState {
    /** Null until the first check answers. */
    status: FFmpegStatus | null;
    checking: boolean;
    downloading: boolean;
    /** Bytes received while downloading; null otherwise. */
    progress: FFmpegDownloadProgress | null;
    /** Why the last download, removal or choice failed. Cleared by the next attempt. */
    error: string | null;
    /** Search again instead of reusing the last result. */
    check: () => Promise<void>;
    download: () => Promise<void>;
    cancelDownload: () => Promise<void>;
    removeDownload: () => Promise<void>;
    /** Use this executable, or `null` to go back to finding one automatically. */
    choosePath: (path: string | null) => Promise<void>;
}

export interface AppState {
    // Navigation
    view: AppView;
    setView: (view: AppView) => void;
    /** Go to Settings, scrolled to `section` when given. */
    openSettings: (section?: SettingsSection) => void;

    // FFmpeg
    ffmpeg: FFmpegState;

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
