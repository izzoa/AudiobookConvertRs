# Tauri IPC API Reference

This document describes the Tauri IPC commands that will be implemented for frontend-backend communication.

> **Note:** This API is planned for Phase 4. Commands are not yet implemented.

## File Operations

### `load_files`
Load audio files and create an audiobook.

```typescript
interface LoadFilesRequest {
  paths: string[];
}

interface LoadFilesResponse {
  audiobook: AudioBook;
}
```

### `get_directory_files`
Scan a directory for audio files.

```typescript
interface ScanDirectoryRequest {
  path: string;
  recursive: boolean;
}

interface ScanDirectoryResponse {
  files: AudioFile[];
}
```

## Audiobook Management

### `get_audiobook`
Get the current audiobook.

```typescript
interface GetAudiobookResponse {
  audiobook: AudioBook | null;
}
```

### `update_metadata`
Update audiobook metadata.

```typescript
interface UpdateMetadataRequest {
  metadata: Metadata;
}
```

### `update_chapter`
Update a single chapter.

```typescript
interface UpdateChapterRequest {
  index: number;
  title?: string;
  start_time_ms?: number;
  end_time_ms?: number;
}
```

### `reorder_chapters`
Reorder chapters.

```typescript
interface ReorderChaptersRequest {
  old_index: number;
  new_index: number;
}
```

## Conversion

### `start_conversion`
Start the conversion process.

```typescript
interface StartConversionRequest {
  settings: ConversionSettings;
}

interface StartConversionResponse {
  job_id: string;
}
```

### `cancel_conversion`
Cancel an active conversion job.

```typescript
interface CancelConversionRequest {
  job_id: string;
}
```

### `get_conversion_status`
Get status of a conversion job.

```typescript
interface ConversionStatus {
  job_id: string;
  status: 'queued' | 'running' | 'paused' | 'complete' | 'error' | 'cancelled';
  progress: number;
  current_file: string;
  error_message?: string;
}
```

## Events

### `conversion_progress`
Emitted during conversion with progress updates.

```typescript
interface ConversionProgressEvent {
  job_id: string;
  file_index: number;
  current_file: string;
  file_progress: number;
  total_progress: number;
  time_remaining_sec: number;
}
```

## Settings

### `get_settings`
Get user settings.

```typescript
interface UserSettings {
  default_bitrate_kbps: number;
  default_output_dir: string;
  auto_detect_artwork: boolean;
  concurrent_jobs: number;
  ffmpeg_path?: string;
  theme: 'light' | 'dark' | 'system';
}
```

### `save_settings`
Save user settings.

```typescript
interface SaveSettingsRequest {
  settings: UserSettings;
}
```

## System

### `check_ffmpeg`
Check if FFmpeg is available.

```typescript
interface FFmpegStatus {
  available: boolean;
  version?: string;
  path?: string;
  error?: string;
}
```

### `get_app_info`
Get application information.

```typescript
interface AppInfo {
  version: string;
  os: string;
  ffmpeg_status: FFmpegStatus;
}
```
