import { createContext, useContext, useState, useEffect, useCallback, type ReactNode } from 'react';
import type { AudioBook, UserSettings, ConversionJob, UndoState, ConversionDefaults, Project } from '../types/models';
import * as SettingsApi from '../api/settings';
import * as UndoApi from '../api/undo';
import * as AudiobookApi from '../api/audiobook';

export type AppView = 'workflow' | 'settings';

interface AppState {
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

const AppContext = createContext<AppState | undefined>(undefined);

export function AppProvider({ children }: { children: ReactNode }) {
    const [view, setView] = useState<AppView>('workflow');
    const [activeAudiobook, setActiveAudiobook] = useState<AudioBook | null>(null);
    const [activeProject, setActiveProject] = useState<Project | null>(null);
    const [settings, setSettings] = useState<UserSettings | null>(null);
    const [conversionQueue, setConversionQueue] = useState<ConversionJob[]>([]);
    const [isLoading, setIsLoading] = useState(true);
    const [undoState, setUndoState] = useState<UndoState>({
        canUndo: false,
        canRedo: false,
    });

    const refreshSettings = useCallback(async () => {
        try {
            const loadedSettings = await SettingsApi.getSettings();
            setSettings(loadedSettings);
        } catch (error) {
            console.error("Failed to refresh settings:", error);
        }
    }, []);

    const refreshUndoState = useCallback(async () => {
        try {
            const state = await UndoApi.getUndoState();
            setUndoState(state);
        } catch (error) {
            console.error("Failed to refresh undo state:", error);
        }
    }, []);

    const handleUndo = useCallback(async () => {
        try {
            const description = await UndoApi.undo();
            if (description) {
                console.log("Undid:", description);
            }
            // Refresh the active audiobook to reflect undo changes
            const updatedAudiobook = await AudiobookApi.getActiveAudiobook();
            if (updatedAudiobook) {
                setActiveAudiobook(updatedAudiobook);
            }
            await refreshUndoState();
        } catch (error) {
            console.error("Undo failed:", error);
        }
    }, [refreshUndoState]);

    const handleRedo = useCallback(async () => {
        try {
            const description = await UndoApi.redo();
            if (description) {
                console.log("Redid:", description);
            }
            // Refresh the active audiobook to reflect redo changes
            const updatedAudiobook = await AudiobookApi.getActiveAudiobook();
            if (updatedAudiobook) {
                setActiveAudiobook(updatedAudiobook);
            }
            await refreshUndoState();
        } catch (error) {
            console.error("Redo failed:", error);
        }
    }, [refreshUndoState]);

    useEffect(() => {
        async function loadInitialData() {
            try {
                const loadedSettings = await SettingsApi.getSettings();
                setSettings(loadedSettings);
                await refreshUndoState();
            } catch (error) {
                console.error("Failed to load settings:", error);
            } finally {
                setIsLoading(false);
            }
        }
        loadInitialData();
    }, [refreshUndoState]);

    // Apply theme effect
    useEffect(() => {
        if (settings?.theme) {
            const root = window.document.documentElement;
            root.classList.remove('light', 'dark');

            if (settings.theme === 'system') {
                const systemTheme = window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
                root.classList.add(systemTheme);
            } else {
                root.classList.add(settings.theme);
            }
        }
    }, [settings?.theme]);

    const handleUpdateSettings = async (newSettings: UserSettings) => {
        try {
            await SettingsApi.updateSettings(newSettings);
            setSettings(newSettings);
        } catch (error) {
            console.error("Failed to update settings:", error);
            throw error;
        }
    };

    const addToQueue = (job: ConversionJob) => {
        setConversionQueue(prev => [...prev, job]);
    };

    // Compute effective conversion defaults: project overrides > global defaults
    const getEffectiveConversionDefaults = useCallback((): ConversionDefaults => {
        // Start with global defaults from settings
        const globalDefaults: ConversionDefaults = {
            bitrate_kbps: settings?.default_bitrate_kbps ?? 128,
            sample_rate_hz: settings?.default_sample_rate,
            overwrite_existing: settings?.overwrite_existing_by_default ?? false,
            embed_chapters: settings?.embed_chapters_by_default ?? true,
            embed_artwork: settings?.embed_artwork_by_default ?? true,
            copy_metadata: settings?.copy_metadata_by_default ?? true,
        };

        // If there's an active project with conversion defaults, use those
        if (activeProject?.conversion_defaults) {
            return { ...globalDefaults, ...activeProject.conversion_defaults };
        }

        return globalDefaults;
    }, [settings, activeProject]);

    return (
        <AppContext.Provider value={{
            view,
            setView,
            activeAudiobook,
            setActiveAudiobook,
            activeProject,
            setActiveProject,
            settings,
            updateSettings: handleUpdateSettings,
            refreshSettings,
            getEffectiveConversionDefaults,
            conversionQueue,
            addToQueue,
            isLoading,
            undoState,
            handleUndo,
            handleRedo,
            refreshUndoState,
        }}>
            {children}
        </AppContext.Provider>
    );
}

export function useApp() {
    const context = useContext(AppContext);
    if (context === undefined) {
        throw new Error('useApp must be used within an AppProvider');
    }
    return context;
}
