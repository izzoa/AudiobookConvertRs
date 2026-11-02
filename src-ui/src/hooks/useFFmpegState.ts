import { useCallback, useEffect, useState } from 'react';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import * as SettingsApi from '../api/settings';
import type { FFmpegDownloadProgress, FFmpegStatus } from '../types/models';
import type { FFmpegState } from '../context/useApp';

/** The backend's message when a download was cancelled; not an error worth showing. */
const CANCELLED = 'download cancelled';

/** Owns FFmpeg's status for the whole app. Used once, by `AppProvider`. */
export function useFFmpegState(): FFmpegState {
    const [status, setStatus] = useState<FFmpegStatus | null>(null);
    const [checking, setChecking] = useState(false);
    const [downloading, setDownloading] = useState(false);
    const [progress, setProgress] = useState<FFmpegDownloadProgress | null>(null);
    const [error, setError] = useState<string | null>(null);

    // Every action answers with the new status; a failure becomes `error`.
    const run = useCallback(async (action: () => Promise<FFmpegStatus>) => {
        setError(null);
        try {
            setStatus(await action());
        } catch (e) {
            const message = String(e);
            if (message !== CANCELLED) setError(message);
        }
    }, []);

    useEffect(() => {
        // The backend searched at startup, so the cached answer is current.
        SettingsApi.getFFmpegInfo()
            .then(setStatus)
            .catch((e) => console.error('Failed to load FFmpeg info:', e));
    }, []);

    useEffect(() => {
        let unlisten: UnlistenFn | undefined;
        let disposed = false;
        listen<FFmpegDownloadProgress>('ffmpeg_download_progress', (event) => setProgress(event.payload)).then((fn) => {
            if (disposed) fn();
            else unlisten = fn;
        });
        return () => {
            disposed = true;
            unlisten?.();
        };
    }, []);

    const check = useCallback(async () => {
        setChecking(true);
        await run(() => SettingsApi.getFFmpegInfo(true));
        setChecking(false);
    }, [run]);

    const download = useCallback(async () => {
        setDownloading(true);
        setProgress(null);
        await run(SettingsApi.downloadFFmpeg);
        setDownloading(false);
        setProgress(null);
    }, [run]);

    const cancelDownload = useCallback(async () => {
        await SettingsApi.cancelFFmpegDownload();
    }, []);

    const removeDownload = useCallback(() => run(SettingsApi.removeDownloadedFFmpeg), [run]);

    const choosePath = useCallback((path: string | null) => run(() => SettingsApi.setFFmpegPath(path)), [run]);

    return { status, checking, downloading, progress, error, check, download, cancelDownload, removeDownload, choosePath };
}
