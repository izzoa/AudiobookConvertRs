import { ArrowDownTrayIcon, ArrowPathIcon, CheckCircleIcon, XCircleIcon } from '@heroicons/react/24/outline';
import { Button } from '@/components/ui/button';
import { Progress } from '@/components/ui/progress';
import { useConfirm } from '@/components/ui/confirm';
import { useToast } from '@/components/ui/use-toast';
import { useApp } from '@/context/useApp';
import * as FilesApi from '@/api/files';
import type { DownloadOffer, FFmpegDownloadProgress, FFmpegSource } from '@/types/models';
import { formatBytes } from '@/lib/format';
import { cn } from '@/lib/utils';

const SOURCE_LABEL: Record<FFmpegSource, string> = {
    chosen: 'Chosen in Settings',
    system: 'Installed on this computer',
    downloaded: 'Downloaded by this app',
};

/** Settings' FFmpeg section: what is in use, Get FFmpeg, and choosing an executable. */
export function FFmpegPanel() {
    const { ffmpeg } = useApp();
    const confirm = useConfirm();
    const { toast } = useToast();
    const { status, checking, downloading, progress, error } = ffmpeg;
    const missing = status ? !status.available : false;

    const chooseFile = async () => {
        try {
            const path = await FilesApi.pickFFmpegFile();
            if (path) await ffmpeg.choosePath(path);
        } catch (e) {
            toast({ title: 'Could not open the file picker', description: String(e), variant: 'destructive' });
        }
    };

    const removeDownload = async () => {
        const ok = await confirm({
            title: 'Remove the downloaded FFmpeg?',
            description:
                status?.source === 'downloaded'
                    ? 'Books cannot convert until FFmpeg is downloaded again or installed on this computer.'
                    : 'The FFmpeg installed on this computer stays in use.',
            confirmLabel: 'Remove',
            danger: true,
        });
        if (ok) await ffmpeg.removeDownload();
    };

    return (
        <>
            <div className="rounded-lg border bg-surface px-4 py-3 space-y-3">
                <div className="flex items-start justify-between gap-4">
                    {status ? (
                        <div className="min-w-0 space-y-0.5 text-sm">
                            <p className={cn('flex items-center gap-2 font-medium', status.available ? 'text-success' : 'text-destructive')}>
                                {status.available ? (
                                    <CheckCircleIcon className="w-4 h-4 shrink-0" />
                                ) : (
                                    <XCircleIcon className="w-4 h-4 shrink-0" />
                                )}
                                {status.available ? 'Found' : 'Not found — books cannot convert'}
                            </p>
                            {status.available && (
                                <p className="text-muted-foreground">
                                    <span className="nums">Version {status.version}</span>
                                    {status.source && ` · ${SOURCE_LABEL[status.source]}`}
                                </p>
                            )}
                            {status.path && (
                                <p className="text-xs text-muted-foreground font-mono truncate" title={status.path}>
                                    {status.path}
                                </p>
                            )}
                            {status.supports_libfdk_aac && (
                                <p className="text-xs text-muted-foreground">Includes libfdk_aac, the higher-quality AAC encoder.</p>
                            )}
                        </div>
                    ) : (
                        <p className="text-sm text-muted-foreground">Checking…</p>
                    )}
                    <Button variant="ghost" size="sm" className="shrink-0" onClick={ffmpeg.check} disabled={checking || downloading}>
                        <ArrowPathIcon className={cn('w-4 h-4 mr-1.5', checking && 'animate-spin')} />
                        Check again
                    </Button>
                </div>

                {status?.chosen_path_error && (
                    <p className="text-xs text-warning">The FFmpeg chosen below is not used: {status.chosen_path_error}</p>
                )}

                {downloading ? (
                    <DownloadProgress progress={progress} onCancel={ffmpeg.cancelDownload} />
                ) : (
                    missing && status?.offer && <Offer offer={status.offer} retry={!!error} onDownload={ffmpeg.download} />
                )}

                {error && <p className="text-xs text-destructive">{error}</p>}

                {missing && !downloading && (
                    <p className="text-xs text-muted-foreground leading-relaxed">
                        {status?.offer ? 'Or install it yourself' : 'Install it'}: on macOS{' '}
                        <code className="font-mono">brew install ffmpeg</code>, on Windows{' '}
                        <code className="font-mono">winget install ffmpeg</code>, on Linux{' '}
                        <code className="font-mono">sudo apt install ffmpeg</code>. Then press Check again.
                    </p>
                )}

                {status?.downloaded_version && !downloading && (
                    <div className="flex items-center justify-between gap-4 border-t pt-3 text-sm">
                        <p className="text-muted-foreground">
                            Downloaded copy: <span className="nums">FFmpeg {status.downloaded_version}</span>
                        </p>
                        <Button variant="ghost" size="sm" className="shrink-0 text-destructive" onClick={removeDownload}>
                            Remove
                        </Button>
                    </div>
                )}
            </div>

            <div className="flex items-center justify-between gap-4">
                <div className="min-w-0 text-sm">
                    <p className="font-medium">FFmpeg to use</p>
                    {status?.chosen_path ? (
                        <p className="text-xs text-muted-foreground font-mono truncate" title={status.chosen_path}>
                            {status.chosen_path}
                        </p>
                    ) : (
                        <p className="text-xs text-muted-foreground">Found automatically</p>
                    )}
                </div>
                <div className="flex gap-2 shrink-0">
                    {status?.chosen_path && (
                        <Button variant="ghost" size="sm" onClick={() => ffmpeg.choosePath(null)} disabled={downloading}>
                            Clear
                        </Button>
                    )}
                    <Button variant="outline" size="sm" onClick={chooseFile} disabled={downloading}>
                        Choose FFmpeg…
                    </Button>
                </div>
            </div>
        </>
    );
}

function Offer({ offer, retry, onDownload }: { offer: DownloadOffer; retry: boolean; onDownload: () => void }) {
    return (
        <div className="flex items-center justify-between gap-4 border-t pt-3">
            <div className="min-w-0 space-y-0.5">
                <p className="text-sm">
                    Get FFmpeg {offer.version} from {offer.host}{' '}
                    <span className="nums whitespace-nowrap">({formatBytes(offer.size_bytes)})</span>.
                </p>
                <p className="text-xs text-muted-foreground">
                    FFmpeg is free software under the GPL. The download is checked before it is installed.{' '}
                    <a href={offer.page_url} target="_blank" rel="noopener noreferrer" className="underline underline-offset-2">
                        About this build
                    </a>
                </p>
            </div>
            <Button size="sm" className="shrink-0" onClick={onDownload}>
                <ArrowDownTrayIcon className="w-4 h-4 mr-1.5" />
                {retry ? 'Try again' : 'Get FFmpeg'}
            </Button>
        </div>
    );
}

function DownloadProgress({ progress, onCancel }: { progress: FFmpegDownloadProgress | null; onCancel: () => void }) {
    const received = progress?.received ?? 0;
    const total = progress?.total ?? 0;
    const installing = total > 0 && received >= total;
    return (
        <div className="space-y-1.5 border-t pt-3">
            <div className="flex items-center justify-between gap-4 text-sm">
                <span>{installing ? 'Checking and installing…' : 'Downloading FFmpeg…'}</span>
                {total > 0 && (
                    <span className="nums text-xs text-muted-foreground">
                        {formatBytes(received)} of {formatBytes(total)}
                    </span>
                )}
            </div>
            <div className="flex items-center gap-3">
                <Progress value={total > 0 ? (received / total) * 100 : 0} className="h-2" />
                <Button variant="ghost" size="sm" className="shrink-0" onClick={onCancel} disabled={installing}>
                    Cancel
                </Button>
            </div>
        </div>
    );
}
