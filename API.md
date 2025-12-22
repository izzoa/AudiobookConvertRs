# Tauri IPC API Reference

This document describes the Tauri IPC commands for frontend-backend communication.

> **Status:** Core commands implemented (Phase 4-6). Advanced chapter/metadata editing commands added in Phase 6.7.

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

## Chapter Import/Export Commands

### `import_chapters`
Import chapters from external file formats (Audacity, FFmpeg metadata, CSV).

```typescript
function import_chapters(
  audiobook_id: string,
  format: 'audacity' | 'ffmetadata' | 'csv',
  content: string
): Promise<AudioBook>
```

**Format Examples:**

**Audacity Label Track:**
```
0.000000	60.000000	Chapter 1
60.000000	120.000000	Chapter 2
```

**FFmpeg Metadata:**
```
;FFMETADATA1

[CHAPTER]
TIMEBASE=1/1000
START=0
END=60000
title=Chapter 1
```

**CSV:**
```
title,start,end
Chapter 1,00:00:00,00:01:00
Chapter 2,00:01:00,00:02:00
```

### `export_chapters`
Export chapters to external file format.

```typescript
function export_chapters(
  audiobook_id: string,
  format: 'audacity' | 'ffmetadata' | 'csv'
): Promise<string>
```

Returns the rendered chapter content as a string. Frontend writes to file.

### `set_chapters`
Directly set all chapters (replaces existing).

```typescript
function set_chapters(
  audiobook_id: string,
  chapters: Chapter[]
): Promise<AudioBook>
```

## Smart Chapter Generation Commands

### `generate_chapters_from_source`
Extract chapters from source file metadata (M4B, MP3 ID3 CHAP, FLAC cue sheets).

```typescript
function generate_chapters_from_source(
  audiobook_id: string
): Promise<AudioBook>
```

Automatically:
- Reads QuickTime chapters from M4B files
- Reads ID3 CHAP frames from MP3 files
- Reads cue sheets from FLAC files
- Adjusts times for multi-file concatenation
- Returns error if no chapters found in sources

### `rename_chapters_with_regex`
Apply regex pattern replacement to all chapter titles.

```typescript
function rename_chapters_with_regex(
  audiobook_id: string,
  pattern: string,
  replacement: string
): Promise<AudioBook>
```

**Examples:**
```typescript
// Remove "01 - " prefix
rename_chapters_with_regex(id, "^\\d+\\s*-\\s*", "")

// Replace underscores with spaces
rename_chapters_with_regex(id, "_", " ")

// Remove file extensions
rename_chapters_with_regex(id, "\\.(mp3|flac|m4a)$", "")
```

### `get_chapter_regex_presets`
Get built-in regex presets for common cleanup patterns.

```typescript
interface RegexPreset {
  name: string;
  description: string;
  pattern: string;
  replacement: string;
}

function get_chapter_regex_presets(): Promise<RegexPreset[]>
```

**Built-in Presets:**
1. Remove numbered prefix (01 -, 001., etc.)
2. Remove "Track" prefix
3. Replace underscores with spaces
4. Remove file extensions
5. Clean multiple spaces
6. Remove "Chapter" prefix

## Enhanced File Picker Commands

### `pick_chapters_file`
Open file picker for chapter files.

```typescript
function pick_chapters_file(): Promise<string | null>
```

Filters for: .txt, .csv, .ffmeta, .ffmetadata

### `read_text_file`
Read a text file's contents.

```typescript
function read_text_file(path: string): Promise<string>
```

## Enhanced Audiobook Commands

### `get_active_audiobook`
Get the currently active audiobook (useful after undo/redo).

```typescript
function get_active_audiobook(): Promise<AudioBook | null>
```

## Undo/Redo Commands

### `get_undo_state`
Get complete undo/redo state (combined query).

```typescript
interface UndoStateResponse {
  can_undo: boolean;
  can_redo: boolean;
  undo_description?: string;
  redo_description?: string;
}

function get_undo_state(): Promise<UndoStateResponse>
```

More efficient than calling `can_undo`, `can_redo`, `get_undo_description`, and `get_redo_description` separately.
