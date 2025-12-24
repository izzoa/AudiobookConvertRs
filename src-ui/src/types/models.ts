export type AudioFormat = 'mp3' | 'flac' | 'm4a' | 'm4b' | 'wav' | 'ogg' | 'wma' | 'aac';

export interface Metadata {
    title?: string;
    artist?: string;
    album?: string;
    album_artist?: string;
    narrator?: string;
    composer?: string;
    year?: number;
    genre?: string;
    track_number?: number;
    total_tracks?: number;
    disc_number?: number;
    total_discs?: number;
    comments?: string;
    description?: string;
    publisher?: string;
    copyright?: string;
    series?: string;
    series_index?: number;
    language?: string;
    artwork_path?: string;
    artwork_data?: string;
}

export interface Chapter {
    index: number;
    title: string;
    start_time_ms: number;
    end_time_ms: number;
    auto_detected: boolean;
}

// For updating chapters
export interface ChapterUpdate {
    title?: string;
    start_time_ms?: number;
    end_time_ms?: number;
}

export interface AudioFile {
    path: string;
    format: AudioFormat;
    duration_ms: number;
    bitrate_kbps?: number;
    sample_rate_hz?: number;
    channels?: number;
    file_size_bytes: number;
    metadata: Metadata;
    has_artwork: boolean;
}

export interface AudioBook {
    id: string;
    title: string;
    author?: string;
    files: AudioFile[];
    chapters: Chapter[];
    output_metadata: Metadata;
    total_duration_ms: number;
    total_size_bytes: number;
    created_at: string; // ISO string
    modified_at: string; // ISO string
    is_dirty: boolean;
}

export interface ConversionSettings {
    bitrate_kbps: number;
    sample_rate_hz?: number;
    output_format: AudioFormat;
    output_path: string;
    concurrent_jobs: number;
    overwrite_existing: boolean;
    embed_chapters: boolean;
    embed_artwork: boolean;
    copy_metadata: boolean;
}

export type JobStatus = 'queued' | 'running' | 'paused' | 'complete' | 'error' | 'cancelled';
export const JobStatus = {
    Queued: 'queued',
    Running: 'running',
    Paused: 'paused',
    Complete: 'complete',
    Error: 'error',
    Cancelled: 'cancelled',
} as const;

export interface ConversionJob {
    id: string;
    audiobook: AudioBook;
    settings: ConversionSettings;
    status: JobStatus;
    progress_percent: number;
    current_file_index: number;
    total_files: number;
    error_message?: string;
    progress?: ConversionProgress;
    output_path?: string;
    status_message?: string;
    created_at: string; // ISO string
    started_at?: string; // ISO string
    completed_at?: string; // ISO string
}

export interface ConversionProgress {
    job_id: string;
    file_index: number;
    current_file: string;
    file_progress: number;
    total_progress: number;
    time_remaining_sec?: number;
    speed?: string;
    status_message: string;
}

export interface UserSettings {
    theme: 'light' | 'dark' | 'system';
    default_output_dir?: string;
    default_conversion_settings: ConversionSettings;
    check_for_updates: boolean;
}

export interface FFmpegInfo {
    version: string;
    path: string;
    copyright: string;
    build_config: string;
}

// Audio Quality Types
export interface AudioLevels {
    integrated_loudness_lufs: number;
    loudness_range_lu: number;
    true_peak_dbtp: number;
    threshold_lufs: number;
    target_offset: number;
}

export interface NormalizationSuggestion {
    recommended: boolean;
    reason: string;
    target_lufs: number;
    target_true_peak: number;
    target_lra: number;
}

export interface NormalizationSettings {
    target_lufs: number;
    target_true_peak: number;
    target_lra: number;
    two_pass: boolean;
}

// Preset Types
export interface PresetSettings {
    bitrate_kbps: number;
    sample_rate?: number;
    concurrent_jobs: number;
    overwrite_existing: boolean;
    embed_chapters: boolean;
    embed_artwork: boolean;
    copy_metadata: boolean;
    normalize: boolean;
    target_lufs?: number;
}

export interface ConversionPreset {
    name: string;
    description?: string;
    settings: PresetSettings;
    created_at: string;
    modified_at: string;
}

// Dry Run Types
export interface DryRunResult {
    title: string;
    input_duration_ms: number;
    input_file_count: number;
    input_size_bytes: number;
    estimated_output_size_bytes: number;
    estimated_time_seconds: number;
    output_path: string;
    output_exists: boolean;
    chapter_count: number;
    has_artwork: boolean;
    warnings: string[];
}

// Project Types
export interface Project {
    version: number;
    audiobook: AudioBook;
    conversion_settings?: ConversionSettings;
    created_at: string;
    saved_at: string;
    app_version: string;
    notes?: string;
}

export interface RecentProject {
    path: string;
    title: string;
    last_opened: string;
    exists: boolean;
}

export interface ProjectExport {
    title: string;
    author?: string;
    narrator?: string;
    duration_formatted: string;
    chapter_count: number;
    file_count: number;
    chapters: ChapterExport[];
}

export interface ChapterExport {
    index: number;
    title: string;
    start_time: string;
    end_time: string;
    duration: string;
}

// Undo Types
export interface UndoState {
    canUndo: boolean;
    canRedo: boolean;
    undoDescription?: string;
    redoDescription?: string;
}

// Open Library Types
export interface OpenLibrarySearchResult {
    key: string;
    title: string;
    author_name?: string[];
    first_publish_year?: number;
    cover_i?: number;
    publisher?: string[];
    subject?: string[];
}

export interface OpenLibrarySelection {
    key: string;
    title: string;
    author?: string;
    year?: number;
    cover_id?: number;
    publisher?: string;
    genre?: string;
}
