export type AudioFormat = 'mp3' | 'flac' | 'm4a' | 'm4b' | 'wav' | 'ogg' | 'wma' | 'aac';

export type QualityPreset = 'high' | 'medium' | 'low' | 'custom';
export const QualityPreset = {
    High: 'high',
    Medium: 'medium',
    Low: 'low',
    Custom: 'custom',
} as const;

export type EncodingMode = 'vbr' | 'cbr';
export const EncodingMode = {
    Vbr: 'vbr',
    Cbr: 'cbr',
} as const;

export interface FileSizeEstimate {
    bytes: number;
    formatted: string;
}

export interface QualityPresetInfo {
    preset: QualityPreset;
    name: string;
    description: string;
    bitrate_kbps: number;
    sample_rate_hz: number;
    is_vbr: boolean;
}

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

export interface NormalizationConfig {
    target_lufs: number;
    target_true_peak: number;
    target_lra: number;
    two_pass: boolean;
}

export interface ConversionSettings {
    bitrate_kbps: number;
    sample_rate_hz?: number;
    encoding_mode: EncodingMode;
    output_format: AudioFormat;
    output_path: string;
    concurrent_jobs: number;
    overwrite_existing: boolean;
    embed_chapters: boolean;
    embed_artwork: boolean;
    copy_metadata: boolean;
    normalize_audio: boolean;
    normalization_settings?: NormalizationConfig;
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

/// User settings matching Rust UserSettings struct
export interface UserSettings {
    // Conversion defaults
    default_bitrate_kbps: number;
    default_output_directory?: string;
    default_sample_rate?: number;
    embed_chapters_by_default: boolean;
    copy_metadata_by_default: boolean;
    embed_artwork_by_default: boolean;
    overwrite_existing_by_default: boolean;

    // Artwork behavior
    auto_detect_artwork: boolean;
    scan_for_cover_images: boolean;

    // Queue defaults
    concurrent_jobs: number;

    // FFmpeg
    ffmpeg_path?: string;

    // UI preferences
    theme: 'light' | 'dark' | 'system';
    window_width: number;
    window_height: number;
    show_advanced_options: boolean;
    confirm_overwrite: boolean;
    notification_sound: boolean;

    // Recent files
    recent_files: string[];
    max_recent_files: number;

    // Debug
    log_level: 'error' | 'warn' | 'info' | 'debug' | 'trace';

    // Quality defaults
    default_quality_preset: QualityPreset;
    default_encoding_mode: EncodingMode;

    // Normalization defaults
    normalization_target_lufs: number;
    normalization_target_true_peak: number;
    normalization_target_lra: number;
    normalization_two_pass: boolean;
}

/// Conversion defaults without output path (for global/project defaults)
export interface ConversionDefaults {
    bitrate_kbps: number;
    sample_rate_hz?: number;
    encoding_mode: EncodingMode;
    overwrite_existing: boolean;
    embed_chapters: boolean;
    embed_artwork: boolean;
    copy_metadata: boolean;
}

/// FFmpeg status from backend
export interface FFmpegStatus {
    available: boolean;
    path?: string;
    version?: string;
    supports_libfdk_aac: boolean;
}

// Legacy FFmpegInfo - kept for backwards compatibility
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

export interface NormalizationProgress {
    percent: number;
    stage: string;
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
    conversion_defaults?: ConversionDefaults;
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
