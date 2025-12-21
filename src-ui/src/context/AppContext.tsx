import { createContext, useContext, useState, useEffect, useCallback, type ReactNode } from 'react';
import type { AudioBook, UserSettings, ConversionJob, UndoState } from '../types/models';
import * as SettingsApi from '../api/settings';
import * as UndoApi from '../api/undo';

interface AppState {
    activeAudiobook: AudioBook | null;
    setActiveAudiobook: (audiobook: AudioBook | null) => void;
    settings: UserSettings | null;
    updateSettings: (settings: UserSettings) => Promise<void>;
    conversionQueue: ConversionJob[];
    addToQueue: (job: ConversionJob) => void;
    isLoading: boolean;
    undoState: UndoState;
    handleUndo: () => Promise<void>;
    handleRedo: () => Promise<void>;
    refreshUndoState: () => Promise<void>;
}

const AppContext = createContext<AppState | undefined>(undefined);

export function AppProvider({ children }: { children: ReactNode }) {
    const [activeAudiobook, setActiveAudiobook] = useState<AudioBook | null>(null);
    const [settings, setSettings] = useState<UserSettings | null>(null);
    const [conversionQueue, setConversionQueue] = useState<ConversionJob[]>([]);
    const [isLoading, setIsLoading] = useState(true);
    const [undoState, setUndoState] = useState<UndoState>({
        canUndo: false,
        canRedo: false,
    });

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

    return (
        <AppContext.Provider value={{
            activeAudiobook,
            setActiveAudiobook,
            settings,
            updateSettings: handleUpdateSettings,
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
