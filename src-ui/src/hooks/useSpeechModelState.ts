import { useCallback, useEffect, useState } from 'react';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import * as SpeechApi from '../api/speech';
import type { SpeechModelDownloadProgress, SpeechModelStatus } from '../types/models';
import type { SpeechModelState } from '../context/useApp';

/** The backend's message when a download was cancelled; not an error worth showing. */
const CANCELLED = 'download cancelled';

/** Owns the speech model's status for the whole app. Used once, by `AppProvider`. */
export function useSpeechModelState(): SpeechModelState {
    const [status, setStatus] = useState<SpeechModelStatus | null>(null);
    const [downloading, setDownloading] = useState(false);
    const [progress, setProgress] = useState<SpeechModelDownloadProgress | null>(null);
    const [error, setError] = useState<string | null>(null);

    // Every action answers with the new status; a failure becomes `error`.
    const run = useCallback(async (action: () => Promise<SpeechModelStatus>): Promise<boolean> => {
        setError(null);
        try {
            setStatus(await action());
            return true;
        } catch (e) {
            const message = String(e);
            if (message !== CANCELLED) setError(message);
            return false;
        }
    }, []);

    useEffect(() => {
        SpeechApi.getSpeechModelStatus()
            .then(setStatus)
            .catch((e) => console.error('Failed to load the speech model status:', e));
    }, []);

    useEffect(() => {
        let unlisten: UnlistenFn | undefined;
        let disposed = false;
        listen<SpeechModelDownloadProgress>('speech_model_download_progress', (event) => setProgress(event.payload)).then(
            (fn) => {
                if (disposed) fn();
                else unlisten = fn;
            }
        );
        return () => {
            disposed = true;
            unlisten?.();
        };
    }, []);

    const download = useCallback(async () => {
        setDownloading(true);
        setProgress(null);
        const ok = await run(SpeechApi.downloadSpeechModel);
        setDownloading(false);
        setProgress(null);
        return ok;
    }, [run]);

    const cancelDownload = useCallback(async () => {
        await SpeechApi.cancelSpeechModelDownload();
    }, []);

    const remove = useCallback(async () => {
        await run(SpeechApi.removeSpeechModel);
    }, [run]);

    return { status, downloading, progress, error, download, cancelDownload, remove };
}
