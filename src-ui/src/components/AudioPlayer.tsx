import { useState, useEffect, useCallback } from 'react';
import { Button } from './ui/button';
import { Progress } from './ui/progress';
import * as PlaybackApi from '@/api/playback';
import type { PlaybackStatus } from '@/api/playback';
import { Play, Pause, Square, Volume2, VolumeX } from 'lucide-react';

interface AudioPlayerProps {
    className?: string;
}

export function AudioPlayer({ className }: AudioPlayerProps) {
    const [status, setStatus] = useState<PlaybackStatus | null>(null);
    const [volume, setVolume] = useState(1.0);
    const [isMuted, setIsMuted] = useState(false);
    const [error, setError] = useState<string | null>(null);

    const pollStatus = useCallback(async () => {
        try {
            const s = await PlaybackApi.getPlaybackStatus();
            setStatus(s);
            if (s.volume !== volume && !isMuted) {
                setVolume(s.volume);
            }
        } catch (e) {
            console.error('Failed to get playback status:', e);
        }
    }, [volume, isMuted]);

    useEffect(() => {
        pollStatus();
        const interval = setInterval(pollStatus, 500);
        return () => clearInterval(interval);
    }, [pollStatus]);

    const handlePlayPause = async () => {
        setError(null);
        try {
            if (status?.state === 'playing') {
                await PlaybackApi.pauseAudio();
            } else if (status?.state === 'paused') {
                await PlaybackApi.resumeAudio();
            }
            await pollStatus();
        } catch (e) {
            setError(String(e));
        }
    };

    const handleStop = async () => {
        setError(null);
        try {
            await PlaybackApi.stopAudio();
            await pollStatus();
        } catch (e) {
            setError(String(e));
        }
    };

    const handleVolumeChange = async (newVolume: number) => {
        setError(null);
        try {
            setVolume(newVolume);
            setIsMuted(newVolume === 0);
            await PlaybackApi.setAudioVolume(newVolume);
        } catch (e) {
            setError(String(e));
        }
    };

    const handleMuteToggle = async () => {
        const newMuted = !isMuted;
        setIsMuted(newMuted);
        await handleVolumeChange(newMuted ? 0 : volume || 1.0);
    };

    const formatTime = (ms: number): string => {
        const totalSeconds = Math.floor(ms / 1000);
        const minutes = Math.floor(totalSeconds / 60);
        const seconds = totalSeconds % 60;
        return `${minutes}:${seconds.toString().padStart(2, '0')}`;
    };

    const getFilename = (path: string | null): string => {
        if (!path) return '';
        const parts = path.split(/[/\\]/);
        return parts[parts.length - 1] || path;
    };

    if (!status || status.state === 'stopped') {
        return null;
    }

    const progressPercent = status.duration_ms > 0
        ? (status.position_ms / status.duration_ms) * 100
        : 0;

    return (
        <div className={`fixed bottom-0 left-0 right-0 bg-background border-t p-3 shadow-lg ${className || ''}`}>
            <div className="max-w-4xl mx-auto flex items-center gap-4">
                <div className="flex items-center gap-2">
                    <Button
                        variant="ghost"
                        size="icon"
                        onClick={handlePlayPause}
                        title={status.state === 'playing' ? 'Pause' : 'Play'}
                    >
                        {status.state === 'playing' ? (
                            <Pause className="h-5 w-5" />
                        ) : (
                            <Play className="h-5 w-5" />
                        )}
                    </Button>
                    <Button
                        variant="ghost"
                        size="icon"
                        onClick={handleStop}
                        title="Stop"
                    >
                        <Square className="h-4 w-4" />
                    </Button>
                </div>

                <div className="flex-1 min-w-0">
                    <div className="text-sm font-medium truncate">
                        {getFilename(status.current_path)}
                    </div>
                    <div className="flex items-center gap-2 mt-1">
                        <span className="text-xs text-muted-foreground w-10">
                            {formatTime(status.position_ms)}
                        </span>
                        <Progress value={progressPercent} className="flex-1 h-1" />
                        <span className="text-xs text-muted-foreground w-10 text-right">
                            {formatTime(status.duration_ms)}
                        </span>
                    </div>
                </div>

                <div className="flex items-center gap-2">
                    <Button
                        variant="ghost"
                        size="icon"
                        onClick={handleMuteToggle}
                        title={isMuted ? 'Unmute' : 'Mute'}
                    >
                        {isMuted ? (
                            <VolumeX className="h-4 w-4" />
                        ) : (
                            <Volume2 className="h-4 w-4" />
                        )}
                    </Button>
                    <input
                        type="range"
                        min="0"
                        max="1"
                        step="0.05"
                        value={isMuted ? 0 : volume}
                        onChange={(e) => handleVolumeChange(parseFloat(e.target.value))}
                        className="w-20 h-1 accent-primary"
                        title="Volume"
                    />
                </div>
            </div>

            {error && (
                <div className="text-xs text-destructive text-center mt-1">
                    {error}
                </div>
            )}
        </div>
    );
}

interface PlayButtonProps {
    path: string;
    startMs?: number;
    className?: string;
    size?: 'sm' | 'md';
}

export function PlayButton({ path, startMs, className, size = 'sm' }: PlayButtonProps) {
    const [isPlaying, setIsPlaying] = useState(false);
    const [, setError] = useState<string | null>(null);

    useEffect(() => {
        const checkStatus = async () => {
            try {
                const status = await PlaybackApi.getPlaybackStatus();
                setIsPlaying(
                    status.state === 'playing' &&
                    status.current_path === path
                );
            } catch (e) {
                console.error('Failed to check playback status:', e);
            }
        };

        const interval = setInterval(checkStatus, 500);
        checkStatus();
        return () => clearInterval(interval);
    }, [path]);

    const handleClick = async (e: React.MouseEvent) => {
        e.stopPropagation();
        setError(null);
        try {
            if (isPlaying) {
                await PlaybackApi.stopAudio();
            } else {
                await PlaybackApi.playAudio(path, startMs);
            }
        } catch (e) {
            setError(String(e));
            console.error('Playback error:', e);
        }
    };

    const iconSize = size === 'sm' ? 'h-3 w-3' : 'h-4 w-4';
    const buttonSize = size === 'sm' ? 'h-6 w-6' : 'h-8 w-8';

    return (
        <Button
            variant="ghost"
            size="icon"
            className={`${buttonSize} ${className || ''}`}
            onClick={handleClick}
            title={isPlaying ? 'Stop' : 'Play'}
        >
            {isPlaying ? (
                <Square className={iconSize} />
            ) : (
                <Play className={iconSize} />
            )}
        </Button>
    );
}
