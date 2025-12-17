import { createContext, useContext, useState, useEffect, type ReactNode } from 'react';
import type { AudioBook, UserSettings, ConversionJob } from '../types/models';
import * as SettingsApi from '../api/settings';
// import * as AudiobookApi from '../api/audiobook'; // Unused for now

interface AppState {
    activeAudiobook: AudioBook | null;
    setActiveAudiobook: (audiobook: AudioBook | null) => void;
    settings: UserSettings | null;
    updateSettings: (settings: UserSettings) => Promise<void>;
    conversionQueue: ConversionJob[];
    addToQueue: (job: ConversionJob) => void;
    isLoading: boolean;
}

const AppContext = createContext<AppState | undefined>(undefined);

export function AppProvider({ children }: { children: ReactNode }) {
    const [activeAudiobook, setActiveAudiobook] = useState<AudioBook | null>(null);
    const [settings, setSettings] = useState<UserSettings | null>(null);
    const [conversionQueue, setConversionQueue] = useState<ConversionJob[]>([]);
    const [isLoading, setIsLoading] = useState(true);

    useEffect(() => {
        async function loadInitialData() {
            try {
                const loadedSettings = await SettingsApi.getSettings();
                setSettings(loadedSettings);

                // TODO: Load last open audiobook if applicable
            } catch (error) {
                console.error("Failed to load settings:", error);
            } finally {
                setIsLoading(false);
            }
        }
        loadInitialData();
    }, []);

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
            isLoading
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
