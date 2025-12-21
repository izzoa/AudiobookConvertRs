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

## Artwork Commands

### `pick_artwork_file`
Open a file picker dialog to select an artwork image file.

```typescript
// Returns the selected file path, or null if cancelled
function pick_artwork_file(): Promise<string | null>
```

### `upload_artwork`
Upload artwork from a file path.

```typescript
interface UploadArtworkRequest {
  audiobook_id: string;
  image_path: string;
}

interface UploadArtworkResponse {
  audiobook: AudioBook; // Returns updated audiobook
}
```

### `set_artwork_base64`
Set artwork from base64-encoded image data (e.g., from drag-and-drop).

```typescript
interface SetArtworkBase64Request {
  audiobook_id: string;
  artwork_base64: string; // Base64-encoded image bytes
}

interface SetArtworkBase64Response {
  audiobook: AudioBook; // Returns updated audiobook
}
```

### `clear_artwork`
Remove artwork from the audiobook.

```typescript
interface ClearArtworkRequest {
  audiobook_id: string;
}

interface ClearArtworkResponse {
  audiobook: AudioBook; // Returns updated audiobook
}
```

## Open Library Integration

### `open_library_search`
Search for books on Open Library.

```typescript
interface OpenLibrarySearchRequest {
  query: string;
  limit?: number; // Default: 10, max: 100
}

interface OpenLibrarySearchResult {
  key: string;
  title: string;
  author_name?: string[];
  first_publish_year?: number;
  cover_i?: number; // Cover ID for fetching artwork
  publisher?: string[];
  subject?: string[];
}

type OpenLibrarySearchResponse = OpenLibrarySearchResult[];
```

### `open_library_apply`
Apply selected Open Library metadata to the audiobook.

```typescript
interface OpenLibraryApplyRequest {
  audiobook_id: string;
  selection: OpenLibrarySelection;
  download_cover: boolean; // Whether to download and apply cover art
}

interface OpenLibrarySelection {
  key: string;
  title: string;
  author?: string;
  year?: number;
  cover_id?: number;
  publisher?: string;
  genre?: string;
}

interface OpenLibraryApplyResponse {
  audiobook: AudioBook; // Returns updated audiobook with new metadata
}
```

## Updated Metadata Commands

All metadata commands now require `audiobook_id` and return the updated `AudioBook`:

### `update_audiobook_metadata`
```typescript
function update_audiobook_metadata(
  audiobook_id: string,
  metadata: Metadata
): Promise<AudioBook>
```

### `add_chapter`
```typescript
function add_chapter(
  audiobook_id: string,
  chapter: Chapter
): Promise<AudioBook>
```

### `update_chapter`
```typescript
function update_chapter(
  audiobook_id: string,
  index: number,
  update: ChapterUpdate
): Promise<AudioBook>
```

### `delete_chapter`
```typescript
function delete_chapter(
  audiobook_id: string,
  index: number
): Promise<AudioBook>
```

### `reorder_chapters`
```typescript
function reorder_chapters(
  audiobook_id: string,
  old_index: number,
  new_index: number
): Promise<AudioBook>
```

### `auto_create_chapters`
```typescript
function auto_create_chapters(
  audiobook_id: string,
  count: number
): Promise<AudioBook>
```
