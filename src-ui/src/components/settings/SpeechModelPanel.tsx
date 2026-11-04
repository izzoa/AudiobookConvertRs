import { ArrowDownTrayIcon, CheckCircleIcon } from '@heroicons/react/24/outline';
import { Button } from '@/components/ui/button';
import { Progress } from '@/components/ui/progress';
import { useConfirm } from '@/components/ui/confirm';
import { useApp } from '@/context/useApp';
import type { SpeechModelDownloadProgress } from '@/types/models';
import { formatBytes } from '@/lib/format';

/** Settings' Speech model section: whether it is installed, getting it, and removing it. */
export function SpeechModelPanel() {
    const { speechModel } = useApp();
    const confirm = useConfirm();
    const { status, downloading, progress, error } = speechModel;

    const remove = async () => {
        const ok = await confirm({
            title: 'Remove the speech model?',
            description: 'Find details will place chapters by track starts and pauses only, until it is downloaded again.',
            confirmLabel: 'Remove',
            danger: true,
        });
        if (ok) await speechModel.remove();
    };

    if (!status) {
        return <p className="text-sm text-muted-foreground">Checking…</p>;
    }

    return (
        <div className="rounded-lg border bg-surface px-4 py-3 space-y-3">
            {status.installed ? (
                <div className="flex items-center justify-between gap-4">
                    <div className="min-w-0 space-y-0.5 text-sm">
                        <p className="flex items-center gap-2 font-medium text-success">
                            <CheckCircleIcon className="w-4 h-4 shrink-0" />
                            Installed
                        </p>
                        <p className="text-muted-foreground nums">Whisper base, {formatBytes(status.size_bytes)}</p>
                    </div>
                    <Button variant="ghost" size="sm" className="shrink-0 text-destructive" onClick={remove} disabled={downloading}>
                        Remove
                    </Button>
                </div>
            ) : downloading ? (
                <SpeechModelDownload progress={progress} onCancel={speechModel.cancelDownload} />
            ) : (
                <div className="flex items-center justify-between gap-4">
                    <div className="min-w-0 space-y-0.5">
                        <p className="text-sm">
                            Not installed. Get it from {status.host}{' '}
                            <span className="nums whitespace-nowrap">({formatBytes(status.size_bytes)})</span>.
                        </p>
                        <p className="text-xs text-muted-foreground">
                            The download is checked before it is installed.{' '}
                            <a href={status.page_url} target="_blank" rel="noopener noreferrer" className="underline underline-offset-2">
                                About the model
                            </a>
                        </p>
                    </div>
                    <Button size="sm" className="shrink-0" onClick={speechModel.download}>
                        <ArrowDownTrayIcon className="w-4 h-4 mr-1.5" />
                        {error ? 'Try again' : 'Get the model'}
                    </Button>
                </div>
            )}

            {error && <p className="text-xs text-destructive">{error}</p>}

            <p className="text-xs text-muted-foreground leading-relaxed">
                With it, Find details listens near each chapter it cannot place on a track start, and starts the chapter
                where the narrator announces its title; and Generate can make chapters wherever a heading is announced.
                Listening happens on this computer; no audio is sent anywhere.
            </p>
        </div>
    );
}

/** Progress of a running model download, with Cancel. Also used by Find details. */
export function SpeechModelDownload({
    progress,
    onCancel,
}: {
    progress: SpeechModelDownloadProgress | null;
    onCancel: () => void;
}) {
    const received = progress?.received ?? 0;
    const total = progress?.total ?? 0;
    const installing = total > 0 && received >= total;
    return (
        <div className="space-y-1.5">
            <div className="flex items-center justify-between gap-4 text-sm">
                <span>{installing ? 'Checking and installing…' : 'Downloading the speech model…'}</span>
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
