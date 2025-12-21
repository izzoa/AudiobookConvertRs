# AudioBookConverter Rust Implementation: Complete Task-Based Plan

**Status**: Phase 6 Complete (Advanced Features + GUI Integration)
**Version**: 1.0 with Symphonia integration
**Total Phases**: 9
**Estimated Duration**: 12-16 weeks
**Team Size**: 1-2 developers

**Progress Summary**:
- ✅ Phase 0: Pre-Conversion Planning & Setup
- ✅ Phase 1: Foundation & Core Architecture
- ✅ Phase 2: Audio Processing & Format Support
- ✅ Phase 3: Metadata Handling & Chapters
- ✅ Phase 4: GUI Implementation - Backend
- ✅ Phase 5: Conversion Pipeline & Processing
- ✅ Phase 6: Advanced Features (Backend + Frontend GUI)
- ⏳ Phase 7: Testing & Quality Assurance
- ⏳ Phase 8: Packaging & Distribution
- ⏳ Phase 9: Launch & Post-Launch

---

# PHASE 0: Pre-Conversion Planning & Setup (Weeks 1-2)

## 0.1 Environment & Repository Setup

- [x] Create new GitHub repository: `AudiobookConvertRs`
- [x] Clone and locally archive original Java AudioBookConverter for reference
- [x] Set up Rust project with Cargo: `cargo new --name AudiobookConvertRs`
- [x] Create initial directory structure:
  ```
  AudiobookConvertRs/
  ├── Cargo.toml
  ├── Cargo.lock
  ├── src-tauri/              # Backend Rust code
  │   ├── src/
  │   │   ├── lib.rs
  │   │   ├── main.rs
  │   │   ├── models.rs
  │   │   ├── error.rs
  │   │   ├── config.rs
  │   │   ├── audio/
  │   │   ├── metadata/
  │   │   ├── ffmpeg/
  │   │   └── conversion/
  │   └── Cargo.toml
  ├── src-ui/                 # Frontend React/Vue code
  │   ├── src/
  │   ├── public/
  │   └── package.json
  ├── tests/
  ├── README.md
  ├── DEVELOPMENT.md
  └── .github/
      └── workflows/
          └── ci.yml
  ```
- [x] Initialize Git with appropriate `.gitignore` (Rust, Node, OS files)
- [x] Create GitHub Actions CI/CD pipeline for multi-platform builds (Windows, macOS, Linux)

## 0.2 Documentation & Analysis

- [x] Create `ARCHITECTURE.md` documenting overall design
- [x] Create `AUDIO_FORMATS.md` documenting all supported audio formats
- [x] Create `METADATA_SPEC.md` documenting metadata handling (ID3, Vorbis, MP4 atoms)
- [x] Create `API.md` documenting Tauri IPC commands
- [x] Create `DEVELOPMENT.md` with setup instructions for contributors
- [ ] Document migration from Java version (feature differences, compatibility notes)
- [ ] List all known limitations compared to Java version

## 0.3 Dependency Research & Selection

- [x] Verify Symphonia latest version and feature flags
- [x] Research mp4ameta API and version compatibility
- [x] Verify FFmpeg bindings (`ffmpeg-sys`) version and platform support
- [x] Confirm image crate supports all needed formats (JPEG, PNG, BMP, GIF)
- [x] Research Tauri latest version and platform requirements
- [x] Document all dependencies in `DEPENDENCIES.md` with rationale
- [x] Check license compatibility (ensure all crates compatible with your license)

## 0.4 Project Configuration

- [x] Create `Cargo.toml` with all confirmed dependencies
- [x] Set up feature flags for:
  - [x] `symphonia` with features: isomp4, flac, mp3, aac, wav, ogg, vorbis
  - [x] `symphonia-metadata` for metadata reading
  - [ ] Optional: test feature for integration tests
- [x] Create `tauri.conf.json` with basic configuration
- [x] Set up build profile optimizations:
  - [x] `debug` profile for development
  - [x] `release` profile with LTO and optimizations
- [x] Create `.env.example` for environment variables (FFmpeg path, etc.)
- [x] Set up Rust toolchain version file (`.rust-version` or `rust-toolchain.toml`)

## 0.5 Proof of Concepts

- [ ] Create small POC for Symphonia loading an MP3 file
- [ ] Create small POC for Symphonia extracting metadata
- [ ] Create small POC for FFmpeg command execution from Rust
- [ ] Create small POC for Tauri IPC command
- [ ] Document POC findings in separate branch or document

---

# PHASE 1: Foundation & Core Architecture (Weeks 3-4)

## 1.1 Error Handling Framework

- [x] Create `src-tauri/src/error.rs` with custom error types
- [x] Define error variants for:
  - [x] File I/O errors
  - [x] FFmpeg execution errors
  - [x] Metadata parsing errors
  - [x] Audio format errors
  - [x] Validation errors
  - [x] IPC/Tauri errors
- [x] Implement `thiserror::Error` trait for all error types
- [x] Implement From/Into conversions for common errors
- [x] Create error display messages (user-friendly messages)
- [x] Document error codes and recovery strategies

## 1.2 Logging Configuration

- [x] Set up `tracing` and `tracing-subscriber`
- [x] Create logging module: `src-tauri/src/logging.rs`
- [x] Configure log levels:
  - [x] Default: INFO for release builds
  - [x] Default: DEBUG for development builds
- [x] Implement structured logging for key operations
- [x] Configure log output (file and console)
- [ ] Add performance timing logs for audio operations

## 1.3 Configuration Management

- [x] Create `src-tauri/src/config.rs` module
- [x] Define UserSettings struct with fields:
  - [x] Default bitrate (128, 192, 256 kbps)
  - [x] Default output directory
  - [x] Auto-detect artwork (true/false)
  - [x] Concurrent job count
  - [x] FFmpeg path (auto-detect or custom)
  - [x] Theme preference (light/dark)
  - [x] Recent files list
- [x] Implement config serialization/deserialization (serde_json)
- [x] Create config file location:
  - [x] Windows: `%APPDATA%/AudiobookConvertRs/config.json`
  - [x] macOS: `~/Library/Application Support/AudiobookConvertRs/config.json`
  - [x] Linux: `~/.config/AudiobookConvertRs/config.json`
- [x] Implement config loading with defaults fallback
- [x] Implement config saving when user updates settings

## 1.4 Core Data Structures

Create `src-tauri/src/models.rs` with all data types:

- [x] `AudioFormat` enum:
  ```
  Mp3, Flac, M4a, M4b, Wav, Ogg, Wma
  ```
- [x] `Metadata` struct:
  - [x] title: Option<String>
  - [x] artist: Option<String>
  - [x] album: Option<String>
  - [x] narrator: Option<String>
  - [x] artwork_path: Option<PathBuf>
  - [x] year: Option<u16>
  - [x] genre: Option<String>
  - [x] comments: Option<String>
- [x] `Chapter` struct:
  - [x] index: usize
  - [x] title: String
  - [x] start_time_ms: u64
  - [x] end_time_ms: u64
  - [x] auto_detected: bool
- [x] `AudioFile` struct:
  - [x] path: PathBuf
  - [x] format: AudioFormat
  - [x] duration_ms: u64
  - [x] bitrate: Option<u32>
  - [x] sample_rate: Option<u32>
  - [x] metadata: Metadata
- [x] `AudioBook` struct:
  - [x] id: String (UUID)
  - [x] title: String
  - [x] author: Option<String>
  - [x] files: Vec<AudioFile>
  - [x] chapters: Vec<Chapter>
  - [x] output_metadata: Metadata
  - [x] created_at: DateTime
  - [x] modified_at: DateTime
- [x] `ConversionSettings` struct:
  - [x] bitrate_kbps: u32
  - [x] sample_rate: Option<u32>
  - [x] output_format: AudioFormat (always M4b for now)
  - [x] output_path: PathBuf
  - [x] concurrent_jobs: u8
- [x] `ConversionJob` struct:
  - [x] id: String (UUID)
  - [x] audiobook: AudioBook
  - [x] settings: ConversionSettings
  - [x] status: JobStatus (Queued, Running, Paused, Complete, Error, Cancelled)
  - [x] progress_percent: f32
  - [x] current_file_index: usize
  - [x] error_message: Option<String>
  - [x] created_at: DateTime
  - [x] started_at: Option<DateTime>
  - [x] completed_at: Option<DateTime>
- [x] `ConversionProgress` struct (for IPC events):
  - [x] job_id: String
  - [x] file_index: usize
  - [x] current_file: String
  - [x] file_progress: f32
  - [x] total_progress: f32
  - [x] time_remaining_sec: u32
  - [x] estimated_finish: DateTime

- [x] Implement Serialize/Deserialize for all structs (serde)
- [x] Implement Display trait for user-facing strings
- [x] Implement Default trait where appropriate
- [x] Add validation methods (e.g., validate_chapter_times)

## 1.5 Application State Management

- [x] Create `src-tauri/src/state.rs` module
- [x] Define AppState struct:
  - [x] current_audiobook: Option<AudioBook>
  - [x] conversion_queue: Vec<ConversionJob>
  - [x] active_job: Option<String> (job_id)
  - [x] user_settings: UserSettings
  - [x] ffmpeg_available: bool
  - [x] recent_audiobooks: Vec<String> (file paths)
- [x] Implement thread-safe state with `Arc<Mutex<AppState>>`
- [x] Create state mutation methods
- [x] Implement state persistence (save/load from disk)

---

# PHASE 2: Audio Processing & Format Support (Weeks 5-6)

## 2.1 Symphonia Integration - Audio Loading

- [x] Create `src-tauri/src/audio/loader.rs` module
- [x] Implement `AudioLoader` struct with methods:
  - [x] `detect_format(path: &Path) -> Result<AudioFormat>`
    - [x] Use Symphonia::Probe to auto-detect
    - [x] Validate against file extension as fallback
    - [x] Return appropriate error if unsupported
  - [x] `load_audio_file(path: &Path) -> Result<AudioFile>`
    - [x] Open file with Symphonia
    - [x] Extract duration from format reader
    - [x] Read codec information (bitrate, sample rate)
    - [x] Extract metadata using symphonia-metadata
    - [x] Detect and extract artwork
    - [x] Populate AudioFile struct
  - [x] `get_duration_ms(path: &Path) -> Result<u64>`
    - [x] Quick duration calculation without full decode
  - [x] `get_metadata(path: &Path) -> Result<Metadata>`
    - [x] Extract all available metadata
    - [x] Return standardized Metadata struct

- [x] Handle Symphonia error types and convert to app errors
- [ ] Add tests for each format (MP3, FLAC, M4A, WAV):
  - [ ] Load valid file
  - [ ] Verify duration is correct
  - [ ] Verify metadata extracted
  - [ ] Verify artwork detected
- [ ] Add tests for error cases:
  - [ ] File not found
  - [ ] Corrupted file
  - [ ] Unsupported format
  - [ ] Permission denied

## 2.2 Format Detection & Validation

- [x] Create `src-tauri/src/audio/format.rs` module
- [x] Implement format detection strategy:
  - [x] Use Symphonia for authoritative format detection
  - [x] Cache format detection results
- [x] Create validation functions:
  - [x] `is_supported_format(path: &Path) -> bool`
  - [x] `validate_audio_file(path: &Path) -> Result<()>`
    - [x] File exists
    - [x] Readable
    - [x] Valid audio format
    - [x] Has audio stream
- [x] Create file type checking:
  - [x] Check magic bytes (file signature)
  - [x] Verify with Symphonia
  - [x] Detect incorrect extensions
- [x] Add comprehensive logging for format detection

## 2.3 Duration Calculation & Timing

- [x] Create `src-tauri/src/audio/timing.rs` module
- [x] Implement duration functions:
  - [x] `calculate_total_duration(files: &[AudioFile]) -> u64`
  - [x] `format_duration_ms(ms: u64) -> String` (HH:MM:SS)
  - [x] `parse_time_string(s: &str) -> Result<u64>` (parse user input)
  - [x] `convert_ms_to_chapters(duration_ms: u64, chapter_count: usize) -> Vec<u64>` (auto-split)
- [x] Handle edge cases:
  - [x] Zero duration files
  - [x] Very large durations (24+ hours)
  - [x] Rounding errors

## 2.4 Symphonia Metadata Extraction

**NOTE:** This section is implemented in `src-tauri/src/metadata/extraction.rs` (Phase 3), not as a separate `symphonia_metadata.rs` module.

- [x] Create `src-tauri/src/metadata/extraction.rs` module (see Phase 3.1)
- [x] Implement metadata extraction functions:
  - [x] `extract_metadata(path)` - unified extraction for all formats
    - [x] Extract from ID3v1 tags (MP3)
    - [x] Extract from ID3v2 tags (MP3)
    - [x] Extract from Vorbis comments (FLAC, OGG)
    - [x] Extract from MP4 atoms (M4A/M4B)
    - [x] Handle missing fields gracefully
  - [x] `extract_embedded_artwork(path)` in artwork.rs
    - [x] Extract embedded artwork
    - [x] Return image data
    - [x] Support multiple formats (JPEG, PNG, GIF, BMP)
  - [x] Bitrate/sample rate extraction in loader.rs
- [x] Implement metadata normalization:
  - [x] Trim whitespace
  - [x] Handle encoding issues
  - [x] Sanitize for filesystem safety
- [x] Add tests:
  - [x] Extract from MP3 with ID3v2
  - [x] Extract from FLAC with Vorbis
  - [x] Extract from M4A/M4B
  - [x] Handle missing metadata fields
  - [x] Extract artwork from each format

## 2.5 Batch File Processing

- [x] Create `src-tauri/src/audio/batch.rs` module
- [x] Implement batch loading:
  - [x] `load_audiobook_from_files(files: Vec<PathBuf>) -> Result<AudioBook>`
    - [x] Load each file with Symphonia
    - [x] Validate all files
    - [x] Calculate total duration
    - [x] Merge metadata (use first file's artist, etc.)
    - [x] Create initial chapters from file boundaries
- [x] Implement file validation for batch:
  - [x] Check all files valid
  - [x] Check sufficient disk space for output
  - [x] Check all files same sample rate (optional warning)
- [x] Implement progress tracking for batch loading:
  - [x] Emit progress events via callback (`load_files_batch_with_progress`)
  - [x] Track files processed with `BatchProgress` struct

## 2.6 FFmpeg Integration - Command Building

- [x] Create `src-tauri/src/ffmpeg/mod.rs` module
- [x] Create `src-tauri/src/ffmpeg/command_builder.rs`
- [x] Implement FFmpeg command building:
  - [x] `FFmpegCommand` struct to build commands
  - [x] `builder()` method to create builder
  - [x] `.input(path)` to add input file
  - [x] `.audio_codec("aac")` to set codec
  - [x] `.bitrate(kbps)` to set bitrate
  - [x] `.sample_rate(hz)` to set sample rate
  - [x] `.output(path)` to set output
  - [x] `.build()` to create final command string
- [x] Implement specific command builders:
  - [x] `build_convert_command(input, output, settings) -> String`
    - [x] Input file with format auto-detection
    - [x] Output format M4B/MP4
    - [x] AAC codec
    - [x] User-specified bitrate
    - [x] User-specified sample rate
  - [x] `build_concat_command(input_files, output) -> String`
    - [x] Use FFmpeg concat demuxer
    - [x] Multiple input files
    - [x] Output single M4B
  - [x] `build_metadata_embed_command(input, output, metadata) -> String`
    - [x] Embed metadata into M4B
    - [x] Add title, artist, etc.
- [x] Add validation:
  - [x] Verify bitrate valid (8-320 kbps typical)
  - [x] Verify sample rate valid (8000, 16000, 22050, 44100, 48000 Hz)
  - [x] Verify output path writable

## 2.7 FFmpeg Detection & Setup

- [x] Create `src-tauri/src/ffmpeg/detection.rs`
- [x] Implement FFmpeg detection:
  - [x] Check PATH environment variable
  - [x] Check common installation locations:
    - [x] Windows: C:\ffmpeg\bin, Program Files
    - [x] macOS: /usr/local/bin, /opt/homebrew/bin
    - [x] Linux: /usr/bin, /usr/local/bin
  - [x] Check user-configured path in settings
  - [x] Verify FFmpeg executable exists and is runnable
  - [x] Cache detection result
- [x] Implement version checking:
  - [x] Run `ffmpeg -version` to get version
  - [x] Verify minimum version (e.g., 4.0+)
  - [x] Log version info for debugging
- [x] Create error handling:
  - [x] FFmpeg not found error with helpful instructions
  - [x] FFmpeg version too old error
  - [x] Log installation instructions per OS

---

# PHASE 3: Metadata Handling & Chapters (Weeks 7-8)

## 3.1 Metadata Extraction (Symphonia)

- [x] Create `src-tauri/src/metadata/extraction.rs`
- [x] Implement ID3 tag extraction (MP3):
  - [x] `extract_id3_tags(path: &Path) -> Result<Metadata>`
  - [x] Read ID3v1 tags (back of file)
  - [x] Read ID3v2 tags (front of file, takes priority)
  - [x] Extract: title, artist, album, year, genre, comments
- [x] Implement Vorbis comment extraction (FLAC, OGG):
  - [x] `extract_vorbis_comments(path: &Path) -> Result<Metadata>`
  - [x] Extract all standard fields
  - [x] Handle multi-value fields
- [x] Implement MP4 atom extraction (M4A/M4B):
  - [x] `extract_mp4_metadata(path: &Path) -> Result<Metadata>`
  - [x] Use mp4ameta for reading
  - [x] Extract: title, artist, album, year, genre, comments
- [x] Implement metadata normalization:
  - [x] Trim whitespace
  - [x] Handle encoding issues (UTF-8, Latin-1, etc.)
  - [x] Unify field names across formats
- [x] Add comprehensive tests:
  - [x] MP3 with ID3v1
  - [x] MP3 with ID3v2
  - [x] FLAC with Vorbis
  - [x] M4A with MP4 atoms
  - [x] Missing metadata fields

## 3.2 Artwork Handling

- [x] Create `src-tauri/src/metadata/artwork.rs`
- [x] Implement artwork detection:
  - [x] `extract_embedded_artwork(path: &Path) -> Result<Option<Vec<u8>>>`
    - [x] Use Symphonia to get embedded artwork
    - [x] Convert to standardized format (JPEG)
    - [x] Return image data bytes
  - [x] `scan_directory_for_artwork(dir: &Path) -> Result<Option<PathBuf>>`
    - [x] Search for: cover.jpg, folder.jpg, cover.png, etc.
    - [x] Return path to found artwork
- [x] Implement artwork processing:
  - [x] `resize_artwork_if_needed(data: &[u8], max_size: usize) -> Result<Vec<u8>>`
    - [x] Use image crate to resize if needed
    - [x] Target: 500x500 px or similar
  - [x] `detect_image_format(data: &[u8]) -> Result<ImageFormat>`
    - [x] Identify JPEG, PNG, GIF, BMP
    - [x] Return format enum
- [x] Store artwork:
  - [x] Save to temp directory during conversion
  - [x] Embed into final M4B with mp4ameta

## 3.3 Chapter Management

- [x] Create `src-tauri/src/metadata/chapters.rs`
- [x] Implement chapter data structures (already in models.rs)
- [x] Implement chapter creation:
  - [x] `create_chapters_from_files(files: &[AudioFile]) -> Vec<Chapter>`
    - [x] Each file becomes a chapter
    - [x] Start times calculated from durations
    - [x] Titles from filenames (remove extensions)
  - [x] `create_chapters_auto_split(total_duration_ms: u64, count: usize) -> Vec<Chapter>`
    - [x] Split by time evenly
    - [x] E.g., 4-hour book split into 12x 20-minute chapters
  - [x] `extract_chapters_from_metadata(path: &Path) -> Result<Vec<Chapter>>`
    - [x] Extract QuickTime chapters from M4B if present
    - [x] Use mp4ameta to read
- [x] Implement chapter validation:
  - [x] `validate_chapters(chapters: &[Chapter], total_duration_ms: u64) -> Result<()>`
    - [x] No overlapping chapters
    - [x] No gaps (optional)
    - [x] End times within total duration
    - [x] Valid chapter order
- [x] Implement chapter editing:
  - [x] `update_chapter(chapters: &mut [Chapter], index: usize, update: ChapterUpdate) -> Result<()>`
  - [x] `reorder_chapters(chapters: &mut [Chapter], old_index: usize, new_index: usize) -> Result<()>`
  - [x] `delete_chapter(chapters: &mut [Chapter], index: usize) -> Result<()>`
  - [x] `add_chapter(chapters: &mut [Chapter], chapter: Chapter) -> Result<()>`

## 3.4 Metadata Writing (mp4ameta)

- [x] Create `src-tauri/src/metadata/writing.rs`
- [x] Implement metadata writing to M4B:
  - [x] `write_metadata_to_m4b(m4b_path: &Path, metadata: &Metadata) -> Result<()>`
    - [x] Use mp4ameta library
    - [x] Write title, artist, album, etc.
    - [x] Handle optional fields
  - [x] `embed_artwork_in_m4b(m4b_path: &Path, artwork: &[u8]) -> Result<()>`
    - [x] Convert artwork to appropriate format if needed
    - [x] Embed using mp4ameta
  - [x] `write_chapters_to_m4b(m4b_path: &Path, chapters: &[Chapter], total_duration_ms: u64) -> Result<()>`
    - [x] Use mp4ameta for QuickTime chapters
    - [x] Write all chapters with correct timing
    - **NOTE**: mp4ameta 0.13.0 does not support writing chapters directly. Chapter embedding will be handled via FFmpeg in Phase 5 using FFmetadata format.
- [ ] Implement metadata backup:
  - [ ] Before writing to M4B, backup original metadata
  - [ ] Save to temporary file in case of error
- [x] Add comprehensive tests:
  - [x] Write metadata to new M4B
  - [x] Update existing M4B metadata
  - [x] Add artwork to M4B
  - [x] Add chapters to M4B
  - [x] Verify written metadata is readable


## 3.5 Metadata Editing API

- [x] Create `src-tauri/src/metadata/editor.rs`
- [x] Implement in-memory metadata editing:
  - [x] `AudioBook` stores editable metadata
  - [x] `update_metadata(audiobook: &mut AudioBook, new_metadata: Metadata) -> Result<()>`
  - [x] `update_chapter(audiobook: &mut AudioBook, index: usize, title: String, times: (u64, u64)) -> Result<()>`
- [x] Implement validation on edits:
  - [x] Sanitize input
  - [x] Validate chapter times don't exceed duration
  - [x] Validate metadata fields (length limits, special characters)
- [x] Track unsaved changes:
  - [x] `audiobook.is_dirty()` method
  - [x] Mark clean on save/conversion

---

# PHASE 4: GUI Implementation - Backend (Weeks 9-10)

## 4.1 Tauri Project Setup

- [x] Initialize Tauri in project:
  - [x] Run `npm create tauri-app@latest` or equivalent
  - [x] Configure Tauri backend/frontend split
  - [x] Set up Cargo workspace if not already
- [x] Configure `tauri.conf.json`:
  - [x] Set app name, version, description
  - [x] Configure window settings (size, resizable, etc.)
  - [x] Set build targets (Windows, macOS, Linux)
  - [x] Configure updater (optional for now)
  - [x] Set security settings
- [x] Create Tauri app state:
  - [x] Integrate AppState from Phase 1.5
  - [x] Make available to all commands
  - [x] Handle state persistence

## 4.2 Tauri IPC Commands - Core

- [x] Create `src-tauri/src/commands/mod.rs`
- [x] Create `src-tauri/src/commands/audiobook.rs`
- [x] Implement audiobook commands:
  - [x] `#[tauri::command] create_audiobook_from_files(paths: Vec<String>) -> Result<AudioBook, String>`
    - [x] Load files using Symphonia
    - [x] Create AudioBook struct
    - [x] Return to frontend
  - [x] `#[tauri::command] load_audiobook_file(path: String) -> Result<AudioBook, String>`
    - [x] Load previously saved audiobook
  - [x] `#[tauri::command] save_audiobook_project(audiobook: AudioBook, path: String) -> Result<(), String>`
    - [x] Save audiobook metadata as JSON
    - [x] For later resumption
  - [x] `#[tauri::command] clear_current_audiobook() -> Result<(), String>`
    - [x] Reset app state

## 4.3 Tauri IPC Commands - Metadata

- [x] Create `src-tauri/src/commands/metadata.rs`
- [x] Implement metadata commands:
  - [x] `#[tauri::command] update_audiobook_metadata(audiobook_id: String, metadata: Metadata) -> Result<(), String>`
  - [x] `#[tauri::command] add_chapter(audiobook_id: String, chapter: Chapter) -> Result<(), String>`
  - [x] `#[tauri::command] update_chapter(audiobook_id: String, index: usize, chapter: Chapter) -> Result<(), String>`
  - [x] `#[tauri::command] delete_chapter(audiobook_id: String, index: usize) -> Result<(), String>`
  - [x] `#[tauri::command] reorder_chapters(audiobook_id: String, old_idx: usize, new_idx: usize) -> Result<(), String>`
  - [x] `#[tauri::command] auto_create_chapters(audiobook_id: String, count: usize) -> Result<(), String>`
  - [x] `#[tauri::command] upload_artwork(audiobook_id: String, image_path: String) -> Result<(), String>`

## 4.4 Tauri IPC Commands - File Operations

- [x] Create `src-tauri/src/commands/files.rs`
- [x] Implement file commands:
  - [x] `#[tauri::command] pick_files() -> Result<Vec<String>, String>`
    - [x] Open file picker dialog
    - [x] Return selected file paths
  - [x] `#[tauri::command] pick_directory() -> Result<String, String>`
    - [x] Open directory picker
    - [x] Return selected directory path
  - [x] `#[tauri::command] validate_files(paths: Vec<String>) -> Result<Vec<FileValidation>, String>`
    - [x] Check each file
    - [x] Return: valid, format, duration, size
  - [x] `#[tauri::command] get_file_info(path: String) -> Result<AudioFile, String>`
    - [x] Get detailed info on single file

## 4.5 Tauri IPC Commands - Settings

- [x] Create `src-tauri/src/commands/settings.rs`
- [x] Implement settings commands:
  - [x] `#[tauri::command] get_settings() -> Result<UserSettings, String>`
  - [x] `#[tauri::command] update_settings(settings: UserSettings) -> Result<(), String>`
  - [x] `#[tauri::command] get_default_settings() -> Result<UserSettings, String>`
  - [x] `#[tauri::command] reset_settings() -> Result<(), String>`
  - [x] `#[tauri::command] check_ffmpeg_available() -> Result<bool, String>`
  - [x] `#[tauri::command] get_ffmpeg_version() -> Result<String, String>`

## 4.6 Tauri IPC Commands - Conversion

- [x] Create `src-tauri/src/commands/conversion.rs`
- [x] Implement conversion commands:
  - [x] `#[tauri::command] start_conversion(audiobook_id: String, settings: ConversionSettings) -> Result<String, String>`
    - [x] Validate inputs
    - [x] Create ConversionJob
    - [x] Queue for processing
    - [x] Return job_id
  - [x] `#[tauri::command] cancel_conversion(job_id: String) -> Result<(), String>`
    - [x] Stop ongoing conversion
    - [x] Clean up temporary files
  - [x] `#[tauri::command] get_conversion_status(job_id: String) -> Result<ConversionJob, String>`
    - [x] Return current job status and progress
  - [x] `#[tauri::command] pause_conversion(job_id: String) -> Result<bool, String>`
  - [x] `#[tauri::command] resume_conversion(job_id: String) -> Result<bool, String>`

## 4.7 Tauri Events (Backend → Frontend)

- [x] Create event emitters in `src-tauri/src/events.rs`
- [x] Implement event types:
  - [x] `ConversionProgress` - emit during conversion
    - [x] job_id, current_file, file_progress, total_progress, time_remaining
  - [x] `ConversionComplete` - emit on success
    - [x] job_id, output_path, duration
  - [x] `ConversionError` - emit on error
    - [x] job_id, error_message, recoverable
  - [x] `FilesLoaded` - emit after files analyzed
    - [x] file_count, total_duration, metadata
  - [x] `MetadataUpdated` - emit after metadata changes
    - [x] audiobook_id

- [x] Implement event emission:
  - [x] `emit_progress(window, event: ConversionProgress)`
  - [x] `emit_completion(window, event: ConversionComplete)`
  - [x] `emit_error(window, event: ConversionError)`

## 4.8 Backend Integration Tests

- [ ] Create `tests/tauri_commands.rs`
- [ ] Test each IPC command:
  - [ ] Command accepts correct input
  - [ ] Command returns expected output
  - [ ] Error handling works
  - [ ] State updates correctly

---

# PHASE 5: Conversion Pipeline & Processing (Weeks 11-12)

## 5.1 FFmpeg Command Execution

- [x] Create `src-tauri/src/ffmpeg/executor.rs`
- [x] Implement command execution:
  - [x] `execute_ffmpeg_command(command: String) -> Result<FFmpegOutput, ConversionError>`
    - [x] Run as subprocess using `std::process::Command`
    - [x] Capture stdout and stderr
    - [x] Handle exit codes
    - [x] Parse output for progress info
  - [x] `run_command_with_timeout(command: String, timeout: Duration) -> Result<...>`
    - [x] Kill process if timeout exceeded
    - [x] Return timeout error
- [x] Implement output parsing:
  - [x] `parse_ffmpeg_progress(stderr: &str) -> Option<FFmpegProgress>`
    - [x] Parse output for: time, frame, speed, bitrate
    - [x] Extract current timestamp (time=HH:MM:SS.ms)
  - [x] `calculate_progress_percent(current_time_ms: u64, total_time_ms: u64) -> f32`
    - [x] Calculate percentage 0.0-100.0
- [x] Add error handling:
  - [x] FFmpeg not found
  - [x] FFmpeg execution failed
  - [x] Invalid command
  - [x] Output parsing errors

## 5.2 Progress Tracking & Reporting

- [x] Create `src-tauri/src/conversion/progress.rs`
- [x] Implement progress tracking:
  - [x] `ProgressTracker` struct to track conversion state
  - [x] `start_tracking()` when conversion begins
  - [x] `update_progress(current_ms: u64, total_ms: u64)` on each parse
  - [x] `calculate_time_remaining() -> Duration` based on current speed
  - [x] `get_current_progress() -> ConversionProgress` to emit
- [x] Implement progress thread:
  - [x] Spawn separate thread for FFmpeg output parsing
  - [x] Parse progress in background
  - [x] Send updates via channel to main thread
  - [x] Main thread emits Tauri events to frontend
- [x] Implement progress smoothing:
  - [x] Avoid too-frequent updates (1-2 sec minimum between events)
  - [x] Calculate average speed (not instantaneous)

## 5.3 Conversion Job Queue

- [x] Create `src-tauri/src/conversion/queue.rs`
- [x] Implement job queue:
  - [x] `ConversionQueue` struct maintaining job list
  - [x] `add_job(job: ConversionJob)` to add to queue
  - [x] `get_next_job() -> Option<ConversionJob>` to pop
  - [x] `update_job_status(job_id: String, status: JobStatus)`
  - [x] `cancel_job(job_id: String)` to remove from queue
  - [x] `get_all_jobs() -> Vec<ConversionJob>` for queue status
- [x] Implement queue persistence:
  - [x] Save queue to disk on state changes
  - [x] Load queue on app startup (resume interrupted conversions)
  - [x] Clear completed jobs periodically

## 5.4 Worker Thread & Parallel Processing

- [x] Create `src-tauri/src/conversion/worker.rs`
- [x] Implement worker management:
  - [x] Create worker thread pool using `rayon` or manual threads
  - [x] Configure max concurrent workers (from settings)
  - [x] Workers pull jobs from queue
  - [x] Workers execute conversion pipeline
- [x] Implement worker execution:
  - [x] `ConversionWorker` that processes single job
  - [x] Load input files
  - [x] Execute FFmpeg conversion
  - [x] Embed metadata with mp4ameta
  - [x] Embed artwork
  - [x] Write chapters
  - [x] Verify output file
  - [x] Update job status
  - [x] Emit completion event
- [x] Implement worker error handling:
  - [x] Catch all error types
  - [x] Log detailed error info
  - [x] Update job with error status
  - [x] Emit error event
  - [x] Move to next job

## 5.5 Conversion Pipeline - Main Flow

- [x] Create `src-tauri/src/conversion/pipeline.rs`
- [x] Implement complete conversion pipeline:
  ```
  1. Validate inputs
     - Files exist and readable
     - Output path writable
     - Metadata valid
     - Chapters don't exceed duration
  
  2. Prepare temporary files
     - Create temp directory
     - Generate temp filenames
  
  3. Execute FFmpeg encoding
     - Build command from settings
     - Run FFmpeg with progress tracking
     - Parse progress and emit events
     - Handle errors (retry? abort?)
  
  4. Embed metadata
     - Use mp4ameta to write metadata
     - Write title, artist, album, etc.
  
  5. Embed artwork
     - Load artwork image
     - Resize if needed
     - Embed into M4B using mp4ameta
  
  6. Embed chapters
     - Create QuickTime chapters
     - Write using mp4ameta
     - Verify chapter timings match audio
  
  7. Verify output
     - Check file exists
     - Check file size reasonable
     - Verify playable with Symphonia
  
  8. Finalize
     - Move to output path
     - Delete temp files
     - Update job status
     - Emit completion event
  ```

- [x] Implement each step:
  - [x] `validate_conversion_inputs(audiobook: &AudioBook, settings: &ConversionSettings) -> Result<()>`
  - [x] `prepare_temp_directory() -> Result<PathBuf>`
  - [x] `execute_encoding_pipeline(audiobook: &AudioBook, settings: &ConversionSettings, progress_cb: Fn(f32)) -> Result<PathBuf>`
  - [x] `embed_all_metadata(m4b_path: &Path, audiobook: &AudioBook, artwork: Option<Vec<u8>>) -> Result<()>`
  - [x] `verify_output_file(m4b_path: &Path) -> Result<()>`

## 5.6 Error Recovery

- [x] Create `src-tauri/src/conversion/recovery.rs`
- [x] Implement error recovery strategies:
  - [x] `retry_failed_conversion(job: &ConversionJob, max_retries: usize) -> Result<()>`
  - [x] `skip_current_file(job: &mut ConversionJob) -> Result<()>`
  - [x] `resume_from_checkpoint(job: &ConversionJob) -> Result<()>` (if possible)
  - [x] `rollback_partial_conversion(job: &ConversionJob) -> Result<()>`
    - [x] Delete incomplete output file
    - [x] Clean temp files
- [x] Implement error reporting:
  - [x] Detailed error logging
  - [x] User-friendly error messages
  - [x] Suggestions for recovery
  - [x] FFmpeg error code documentation

## 5.7 Temporary File Management

- [x] Create `src-tauri/src/conversion/temp.rs`
- [x] Implement temp directory management:
  - [x] `create_temp_directory() -> Result<PathBuf>`
    - [x] Create in OS temp directory
    - [x] Use unique name (job_id based)
  - [x] `cleanup_temp_directory(path: &Path) -> Result<()>`
    - [x] Delete temp directory and contents
    - [x] Handle Windows file locking issues
    - [x] Retry on failure
  - [x] `list_temp_files(job_id: String) -> Result<Vec<PathBuf>>`
    - [x] Find all temp files for job
- [x] Implement cleanup on errors:
  - [x] Register cleanup functions
  - [x] Ensure cleanup happens even on panic
  - [x] Use RAII pattern (Drop trait)
## 5.8 Conversion Integration Tests

- [ ] Create `tests/conversion.rs`
- [ ] Create test audio files:
  - [ ] Sample MP3
  - [ ] Sample FLAC
  - [ ] Sample M4A
- [ ] Test conversion pipeline:
  - [ ] Single MP3 → M4B
  - [ ] Multiple files → M4B
  - [ ] Metadata preservation
  - [ ] Chapter embedding
  - [ ] Artwork embedding
  - [ ] Error handling
  - [ ] Progress tracking

---

# PHASE 6: Advanced Features (Weeks 13-14)

## 6.1 Audio Quality Detection & Normalization

- [x] Create `src-tauri/src/audio/quality.rs`
- [x] Implement quality detection:
  - [x] `analyze_audio_levels(file_path: &Path) -> Result<AudioLevels>`
    - [x] Use FFmpeg to analyze
    - [x] Return peak level, RMS, loudness info
  - [x] `suggest_normalization(levels: &AudioLevels) -> NormalizationSuggestion`
    - [x] Recommend normalization if levels vary
    - [x] Suggest target level
- [x] Implement audio normalization:
  - [x] `apply_normalization(input: &Path, output: &Path, target_level: f32) -> Result<()>`
    - [x] Use FFmpeg loudnorm filter
    - [x] Re-encode with normalization

## 6.2 Format-Specific Handling

- [x] Implement existing M4B handling:
  - [x] Detect when input is M4B
  - [x] Read existing chapters
  - [x] Preserve chapters during re-encoding
  - [x] Allow overwriting metadata
- [x] Implement FLAC cue sheet support (optional):
  - [x] Extract cue sheets from FLAC
  - [x] Convert to chapters
- [x] Implement MP3 chapter support (optional):
  - [x] Use ID3 CHAP frames
  - [x] Convert to M4B chapters

## 6.3 Batch Processing Enhancements

- [x] Implement batch mode:
  - [x] Load multiple audiobooks
  - [x] Apply same settings to all
  - [x] Queue all for parallel processing
  - [x] Report progress per audiobook
- [x] Implement batch presets:
  - [x] Save conversion settings as preset
  - [x] Load preset for future conversions
  - [x] Manage presets (create, update, delete)
- [x] Implement dry-run mode:
  - [x] Analyze without converting
  - [x] Preview final output
  - [x] Estimate time and space needed

## 6.4 Project Management

- [x] Implement project save/load:
  - [x] Save audiobook with all metadata as JSON
  - [x] Load project to resume editing
  - [x] Version saved projects for compatibility
  - [x] Export project metadata
- [x] Implement recent projects:
  - [x] Track recently opened projects
  - [x] Quick-open from main menu
  - [x] Clean up old project files

## 6.5 User Experience Enhancements

- [x] Implement keyboard shortcuts (frontend):
  - [x] Ctrl+Z to undo
  - [x] Ctrl+Y / Ctrl+Shift+Z to redo
  - [x] Ctrl+O to open files
  - [x] Ctrl+S to save project
  - [x] Escape to cancel
- [x] Implement undo/redo:
  - [x] Track metadata edits
  - [x] Undo/redo chapter changes
  - [x] Implement command pattern
- [ ] Implement file browser (frontend):
  - [ ] Browse file system
  - [ ] Preview files
  - [x] Drag-and-drop support (partial - file dialog works)

## 6.6 GUI Integration (Frontend)

- [x] Create Tauri commands for Phase 6 features:
  - [x] `commands/quality.rs` - Audio level analysis and normalization
  - [x] `commands/presets.rs` - Preset management and dry-run
  - [x] `commands/project.rs` - Project save/load/export
  - [x] `commands/undo.rs` - Undo/redo operations
- [x] Create frontend API functions:
  - [x] `api/quality.ts` - Audio quality API
  - [x] `api/presets.ts` - Presets API
  - [x] `api/project.ts` - Project management API
  - [x] `api/undo.ts` - Undo/redo API
- [x] Create React components:
  - [x] `NormalizationPanel.tsx` - Audio level analysis and normalization UI
  - [x] `PresetManager.tsx` - Conversion presets management UI
  - [x] `DryRunPreview.tsx` - Conversion preview with estimates
  - [x] `ProjectManager.tsx` - Project save/load/recent projects UI
  - [x] `UndoRedoButtons.tsx` - Toolbar undo/redo buttons
- [x] Add TypeScript types for Phase 6 data structures:
  - [x] `AudioLevels`, `NormalizationSuggestion`, `NormalizationSettings`
  - [x] `ConversionPreset`, `PresetSettings`, `DryRunResult`
  - [x] `Project`, `RecentProject`, `ProjectExport`, `UndoState`
- [x] Update AppContext with undo/redo support
- [x] Wire up Sidebar navigation with actual handlers
- [x] Create `useKeyboardShortcuts` hook

## 6.7 Performance Optimization

- [x] Create `src-tauri/src/perf.rs` module:
  - [x] Timer utility for measuring operations
  - [x] MetricsCollector for tracking multiple operations
  - [x] ConversionMetrics struct for conversion stats
  - [x] `measure_time!` macro for timing code blocks
- [x] Profile conversion pipeline:
  - [x] Measure time spent in each phase
  - [x] Identify bottlenecks
- [ ] Optimize FFmpeg:
  - [ ] Test different encoding parameters
  - [ ] Compare speed vs quality
  - [ ] Use hardware acceleration if available
- [ ] Optimize metadata operations:
  - [ ] Batch metadata writes
  - [ ] Minimize re-encoding if possible

---

# PHASE 7: Testing & Quality Assurance (Week 15)

## 7.1 Unit Tests - Audio Module

- [ ] Test audio format detection:
  - [ ] Valid MP3
  - [ ] Valid FLAC
  - [ ] Valid M4A
  - [ ] Invalid/corrupted files
  - [ ] Unsupported formats
- [ ] Test metadata extraction:
  - [ ] MP3 with ID3v1/v2
  - [ ] FLAC with Vorbis
  - [ ] M4A/M4B with atoms
  - [ ] Missing metadata fields
  - [ ] Artwork extraction
- [ ] Test duration calculation:
  - [ ] Accurate duration
  - [ ] Multiple files
  - [ ] Edge cases (very long files)
- [ ] Test chapter operations:
  - [ ] Chapter creation
  - [ ] Chapter validation
  - [ ] Chapter editing
  - [ ] Chapter serialization

## 7.2 Unit Tests - FFmpeg Module

- [ ] Test command building:
  - [ ] Correct command syntax
  - [ ] All parameters included
  - [ ] Special characters escaped
- [x] Phase 5: Conversion Pipeline & Processing
  - [x] Create `ffmpeg/executor.rs`
    - [x] Implement logic to spawn FFmpeg process
    - [x] Implement timeout handling
    - [x] Capture stdout/stderr
  - [x] Create `conversion/progress.rs`
    - [x] Parse FFmpeg output for frame, time, q, size, bitrate, speed
    - [x] Calculate percentage based on duration
    - [x] Define `ConversionProgress` struct
  - [x] Create `conversion/queue.rs`
    - [x] Define `ConversionQueue` struct
    - [x] Implement thread-safe add/remove/update
    - [x] Implement persistent queue storage (save to disk)
  - [x] Create `conversion/worker.rs`
    - [x] Implement threaded worker loop
    - [x] Pick jobs from queue
    - [x] Update job status (Running, Compiling, etc.)
  - [x] Create `conversion/pipeline.rs` (Main flow)
    - [x] Step 1: Input Validation
    - [x] Step 2: Prepare inputs (concatenation list if needed)
    - [x] Step 3: Run FFmpeg encoding
    - [x] Step 4: Run FFmpeg chapter embedding
    - [x] Step 5: Finalize (move, cleanup)
  - [x] Create `conversion/recovery.rs`
    - [x] Implement retry logic for transient failures
    - [x] Implement checkpointing (optional/basic)
  - [x] Create `conversion/temp.rs`
    - [x] Manage safe temporary directories
    - [x] Auto-cleanup logic
  - [x] Update `src-tauri/src/conversion/mod.rs` exports
  - [x] Update `src-tauri/src/ffmpeg/mod.rs` exports
  - [x] Verify: Run unit tests for pipeline components
- [ ] Test output parsing:
  - [ ] Parses progress correctly
  - [ ] Handles various FFmpeg versions
  - [ ] Handles invalid output

## 7.3 Unit Tests - Metadata Module

- [ ] Test metadata writing:
  - [ ] Write to new M4B
  - [ ] Update existing M4B
  - [ ] Handle special characters
- [ ] Test artwork handling:
  - [ ] Embed artwork
  - [ ] Resize if needed
  - [ ] Handle various formats

## 7.4 Unit Tests - Conversion Module

- [ ] Test pipeline steps:
  - [ ] Input validation
  - [ ] Temp directory creation
  - [ ] FFmpeg execution
  - [ ] Metadata embedding
  - [ ] Output verification
- [ ] Test error handling:
  - [ ] FFmpeg failure
  - [ ] Metadata write failure
  - [ ] Disk space errors
  - [ ] Permission errors
- [ ] Test progress tracking:
  - [ ] Progress calculated correctly
  - [ ] Time remaining accurate
  - [ ] Progress events emitted

## 7.5 Integration Tests - Full Pipeline

- [ ] Test MP3 → M4B:
  - [ ] Load MP3 with metadata
  - [ ] Convert to M4B
  - [ ] Verify metadata preserved
  - [ ] Verify playable
- [ ] Test multiple files → M4B:
  - [ ] Load multiple files
  - [ ] Create chapters
  - [ ] Convert to single M4B
  - [ ] Verify chapter times
- [ ] Test FLAC → M4B:
  - [ ] Load FLAC with artwork
  - [ ] Convert to M4B
  - [ ] Verify artwork embedded
- [ ] Test WMA → M4B (via FFmpeg):
  - [ ] Load WMA
  - [ ] Convert to M4B
  - [ ] Verify success
- [ ] Test error scenarios:
  - [ ] Missing input file
  - [ ] Invalid output path
  - [ ] Insufficient disk space
  - [ ] FFmpeg error

## 7.6 Cross-Platform Testing

- [ ] Windows 10+ testing:
  - [ ] File paths with backslashes
  - [ ] File permissions
  - [ ] Executable detection
  - [ ] Temp directory cleanup
- [ ] macOS testing:
  - [ ] File permissions
  - [ ] Code signing issues (if applicable)
  - [ ] Homebrew FFmpeg path
- [ ] Linux testing:
  - [ ] Multiple distributions (Ubuntu, Fedora, Arch)
  - [ ] Package manager paths
  - [ ] File permissions

## 7.7 Performance Testing

- [ ] Benchmark conversion speed:
  - [ ] 100MB MP3 → M4B
  - [ ] 500MB FLAC → M4B
  - [ ] Multiple file concatenation
  - [ ] Compare against Java version (target: equal or faster)
- [ ] Memory profiling:
  - [ ] Peak memory during conversion
  - [ ] Memory growth during batch processing
  - [ ] No memory leaks (run long conversion)
- [ ] CPU profiling:
  - [ ] CPU usage during conversion
  - [ ] Core utilization (batch processing)

## 7.8 UI/UX Testing

- [ ] Test file import:
  - [ ] Drag-and-drop
  - [ ] File picker
  - [ ] Keyboard shortcuts
- [ ] Test metadata editing:
  - [ ] Edit each field
  - [ ] Add/remove chapters
  - [ ] Upload artwork
- [ ] Test conversion workflow:
  - [ ] Settings panel
  - [ ] Progress display
  - [ ] Error messages
  - [ ] Completion notification
- [ ] Test responsiveness:
  - [ ] No UI freeze during conversion
  - [ ] Progress updates smooth
  - [ ] Cancel button responsive

## 7.9 Regression Testing

- [ ] Automated test suite:
  - [ ] Run on each commit (CI/CD)
  - [ ] Test all unit tests pass
  - [ ] Test all integration tests pass
  - [ ] Test builds on all platforms
- [ ] Manual regression tests:
  - [ ] Basic workflow (load → convert → verify)
  - [ ] Edge cases (very long files, special characters)
  - [ ] Error recovery

## 7.10 Coverage Analysis

- [ ] Measure code coverage:
  - [ ] Target: >80% for core modules
  - [ ] Identify untested code
  - [ ] Add tests for critical paths
- [ ] Generate coverage report:
  - [ ] Use `cargo tarpaulin` or similar
  - [ ] Visualize coverage
  - [ ] Track coverage over time

---

# PHASE 8: Packaging & Distribution (Week 16)

## 8.1 Build Optimization

- [ ] Configure release build:
  - [ ] Enable LTO (Link Time Optimization)
  - [ ] Optimize for size or speed
  - [ ] Strip debug symbols
  - [ ] Test release build thoroughly
- [ ] Test release binary:
  - [ ] Verify functionality
  - [ ] Check binary size
  - [ ] Verify performance improvement
  - [ ] Check startup time

## 8.2 Windows Installer Creation

- [ ] Prepare Windows package:
  - [ ] Build release for x86_64
  - [ ] Include FFmpeg binary (or installer)
  - [ ] Create start menu shortcuts
  - [ ] Create uninstaller
- [ ] Create NSIS installer:
  - [ ] Install to Program Files
  - [ ] Add to Start Menu
  - [ ] Add/Remove Programs entry
  - [ ] Desktop shortcut (optional)
  - [ ] Associate .m4b files (optional)
- [ ] Test Windows installer:
  - [ ] Install on clean system
  - [ ] Verify all files installed
  - [ ] Test uninstall
  - [ ] Test re-install
  - [ ] Test upgrade from previous version

## 8.3 macOS Package Creation

- [ ] Prepare macOS package:
  - [ ] Build release for x86_64 and ARM64 (Apple Silicon)
  - [ ] Create universal binary if possible
  - [ ] Include FFmpeg (via Homebrew or bundled)
  - [ ] Code signing (if certificate available)
- [ ] Create DMG installer:
  - [ ] Drag-and-drop install
  - [ ] Add Applications folder shortcut
  - [ ] Custom background image (optional)
- [ ] Notarization (optional):
  - [ ] Upload to Apple for notarization
  - [ ] Prevent "Unknown Developer" warning
- [ ] Test macOS package:
  - [ ] Install on macOS system
  - [ ] Test functionality
  - [ ] Verify code signature

## 8.4 Linux Package Creation

- [ ] Prepare Linux package:
  - [ ] Build release for x86_64
  - [ ] Optional: ARM64 build
- [ ] Create AppImage:
  - [ ] Build with AppImage format
  - [ ] Bundle dependencies
  - [ ] Create desktop entry
  - [ ] Make executable
- [ ] Create Snap package (optional):
  - [ ] Create snapcraft.yaml
  - [ ] Define dependencies
  - [ ] Test snap installation
- [ ] Create Flatpak (optional):
  - [ ] Create flatpak manifest
  - [ ] Build and test
- [ ] Test Linux packages:
  - [ ] Test on multiple distributions
  - [ ] Verify dependencies
  - [ ] Test installation/uninstallation

## 8.5 Dependency Bundling

- [ ] FFmpeg dependency:
  - [ ] Windows: Include ffmpeg.exe or installer link
  - [ ] macOS: Use Homebrew or bundle
  - [ ] Linux: Assume system FFmpeg or provide in AppImage
- [ ] Document FFmpeg installation:
  - [ ] How to install FFmpeg per OS
  - [ ] Verify installation
  - [ ] Troubleshoot common issues
- [ ] Test FFmpeg bundling:
  - [ ] Installer includes FFmpeg
  - [ ] Application finds bundled FFmpeg
  - [ ] Fallback to system FFmpeg if needed

## 8.6 Release Notes & Documentation

- [ ] Create CHANGELOG.md:
  - [ ] List all features (from Java version)
  - [ ] List all bug fixes
  - [ ] List known limitations
  - [ ] Migration notes from Java version
- [ ] Create INSTALL.md:
  - [ ] Installation instructions per OS
  - [ ] System requirements
  - [ ] Troubleshooting
- [ ] Create USER_GUIDE.md:
  - [ ] Quick start guide
  - [ ] Feature overview
  - [ ] How-to guides
  - [ ] Screenshots
- [ ] Create TROUBLESHOOTING.md:
  - [ ] Common issues
  - [ ] Solutions
  - [ ] Where to find help

## 8.7 GitHub Release Setup

- [ ] Create GitHub Release:
  - [ ] Tag version (v1.0.0)
  - [ ] Write release notes
  - [ ] Upload Windows installer
  - [ ] Upload macOS DMG
  - [ ] Upload Linux AppImage
  - [ ] Upload source code archive
  - [ ] Mark as latest release
- [ ] Set up release automation:
  - [ ] CI/CD builds and uploads on tag
  - [ ] Automatic release creation
  - [ ] Deploy to GitHub Pages (documentation)

## 8.8 Final Quality Checks

- [ ] Run full test suite
- [ ] Manual testing on all platforms
- [ ] Check all documentation complete
- [ ] Verify all binaries work
- [ ] Test installer/package on clean system
- [ ] Performance benchmarking
- [ ] Security review

---

# PHASE 9: Launch & Post-Launch (Week 16+)

## 9.1 Beta Testing

- [ ] Release beta version:
  - [ ] Create beta branch
  - [ ] Build beta installers
  - [ ] Create beta release on GitHub
  - [ ] Mark as pre-release
- [ ] Recruit beta testers:
  - [ ] Post on r/rust, r/audiobooks
  - [ ] Email existing AudioBookConverter users (if possible)
  - [ ] Post on Linux communities
- [ ] Gather feedback:
  - [ ] Create feedback form
  - [ ] Monitor GitHub issues
  - [ ] Track bug reports
  - [ ] Note feature requests
- [ ] Fix critical bugs:
  - [ ] Prioritize showstoppers
  - [ ] Release beta updates
  - [ ] Target 1-2 week beta period

## 9.2 Launch Preparation

- [ ] Final checks:
  - [ ] All beta issues resolved
  - [ ] Documentation complete
  - [ ] All platforms tested
  - [ ] Performance acceptable
- [ ] Prepare announcement:
  - [ ] Write blog post or medium article
  - [ ] Prepare social media posts
  - [ ] Create feature comparison (Java vs Rust)
  - [ ] Record demo video (optional)

## 9.3 Official Release

- [ ] Release v1.0.0:
  - [ ] Update version number
  - [ ] Create final release notes
  - [ ] Tag commit as v1.0.0
  - [ ] Build and upload all installers
  - [ ] Publish GitHub release
- [ ] Announce release:
  - [ ] Post on r/rust, r/audiobooks
  - [ ] Post on HackerNews
  - [ ] Email AudioBookConverter community
  - [ ] Tweet/social media
  - [ ] Post on relevant forums
  - [ ] Update project README

## 9.4 Maintenance & Support

- [ ] Monitor feedback:
  - [ ] Watch GitHub issues
  - [ ] Respond to bug reports
  - [ ] Track user questions
- [ ] Bug fixes:
  - [ ] High priority: crashers, data loss
  - [ ] Medium priority: missing features, UI issues
  - [ ] Low priority: documentation, nice-to-haves
- [ ] Release schedule:
  - [ ] Hotfix releases within 24 hours if critical
  - [ ] Weekly releases for bug fixes
  - [ ] Monthly minor releases for features

## 9.5 Community Engagement

- [ ] Create discussion channels:
  - [ ] GitHub Discussions
  - [ ] Optional: Discord server
  - [ ] Optional: Matrix/IRC chat
- [ ] Documentation & FAQ:
  - [ ] Maintain FAQ based on questions
  - [ ] Write troubleshooting guides
  - [ ] Document workarounds
- [ ] Feature requests:
  - [ ] Prioritize community requests
  - [ ] Plan next features
  - [ ] Public roadmap (optional)

## 9.6 Dependency Maintenance

- [ ] Regular updates:
  - [ ] Monthly: check for crate updates
  - [ ] Quarterly: update dependencies
  - [ ] Test after updates
- [ ] Security patches:
  - [ ] Monitor security advisories
  - [ ] Apply patches immediately
  - [ ] Release security updates promptly
- [ ] FFmpeg compatibility:
  - [ ] Test with new FFmpeg versions
  - [ ] Update documentation
  - [ ] Maintain compatibility

## 9.7 Performance Monitoring

- [ ] Track performance:
  - [ ] Benchmark conversions regularly
  - [ ] Compare against Java version
  - [ ] Monitor user reported performance
- [ ] Optimization opportunities:
  - [ ] Profile code for bottlenecks
  - [ ] Optimize critical paths
  - [ ] Release performance improvements

## 9.8 Long-term Roadmap

- [ ] Plan next features (Tier 2):
  - [ ] Audio normalization
  - [ ] Online metadata lookup
  - [ ] MP3 splitting
  - [ ] Advanced metadata editing
- [ ] Plan major versions:
  - [ ] Architectural improvements
  - [ ] New features
  - [ ] Breaking changes
- [ ] Consider:
  - [ ] Mobile app (React Native)?
  - [ ] Web version?
  - [ ] Library/API for other tools?

---

# APPENDIX A: Testing Checklist

## Functional Testing

- [ ] File import works with all formats
- [ ] Metadata extraction accurate
- [ ] Chapter creation works
- [ ] Conversion produces valid M4B
- [ ] Metadata embedded correctly
- [ ] Artwork embedded correctly
- [ ] Settings saved/loaded
- [ ] Batch processing works
- [ ] Error handling graceful
- [ ] Progress tracking accurate

## Compatibility Testing

- [ ] Windows 10+ support
- [ ] macOS 10.13+ support
- [ ] Linux (Ubuntu 20.04+, Fedora, Arch)
- [ ] Various FFmpeg versions
- [ ] Various audio file sizes/types
- [ ] Various metadata formats
- [ ] Special characters in filenames

## Performance Testing

- [ ] Conversion speed ≥ Java version
- [ ] Memory usage < Java version
- [ ] Startup time < 1 second
- [ ] No memory leaks
- [ ] UI responsive during conversion
- [ ] Batch processing parallelizes well

## Usability Testing

- [ ] Intuitive workflow
- [ ] Clear error messages
- [ ] Helpful documentation
- [ ] Keyboard shortcuts work
- [ ] Settings are discoverable
- [ ] File dialogs work

---

# APPENDIX B: Documentation Checklist

- [ ] README.md - Project overview
- [ ] INSTALL.md - Installation instructions
- [ ] DEVELOPMENT.md - Development setup
- [ ] ARCHITECTURE.md - Architecture design
- [ ] API.md - IPC command documentation
- [ ] USER_GUIDE.md - User manual
- [ ] TROUBLESHOOTING.md - Common issues
- [ ] CHANGELOG.md - Version history
- [ ] CONTRIBUTING.md - Contributing guidelines
- [ ] LICENSE - License file
- [ ] inline code comments - Important logic explained

---

# APPENDIX C: Dependency Versions (Recommended)

```toml
[dependencies]
symphonia = { version = "0.5", features = ["all"] }
symphonia-metadata = "0.5"
mp4ameta = "0.14"
image = "0.25"
ffmpeg-sys = "4.4"
tauri = { version = "1.5", features = ["shell-open", "fs-read-dir", "dialog-open"] }
tokio = { version = "1", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
thiserror = "1.0"
tracing = "0.1"
tracing-subscriber = "0.3"
uuid = { version = "1.0", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }
camino = "1.1"
walkdir = "2.4"
anyhow = "1.0"
rayon = "1.7"
parking_lot = "0.12"
regex = "1.10"

[dev-dependencies]
tempfile = "3.8"
proptest = "1.4"
```

---

# APPENDIX D: Git Workflow

## Branching Strategy

```
main (stable releases only)
├── develop (integration branch)
    ├── feature/phase0-setup
    ├── feature/phase1-foundation
    ├── feature/phase2-audio
    ├── feature/phase3-metadata
    ├── feature/phase4-gui-backend
    ├── feature/phase5-conversion
    ├── feature/phase6-advanced
    ├── feature/phase7-testing
    ├── feature/phase8-packaging
    └── bugfix/issue-123
```

## Commit Message Format

```
[PHASE#] [TYPE] Brief description

Detailed explanation if needed.

Fixes #123
```

Where TYPE is:
- feat: New feature
- fix: Bug fix
- docs: Documentation
- refactor: Code refactoring
- test: Test addition
- build: Build system
- perf: Performance improvement

## Pull Request Workflow

1. Create feature branch from `develop`
2. Implement feature with tests
3. Create pull request
4. Code review
5. Merge to `develop`
6. Test in develop
7. Merge `develop` to `main` for release

---

# APPENDIX E: Success Criteria Checklist

## Functional Requirements

- [ ] Load MP3 files with Symphonia
- [ ] Load FLAC files with Symphonia
- [ ] Load M4A files with Symphonia
- [ ] Load WAV files with Symphonia
- [ ] Extract metadata from all formats
- [ ] Extract artwork from all formats
- [ ] Create chapters automatically
- [ ] Convert to M4B format
- [ ] Embed metadata in M4B
- [ ] Embed artwork in M4B
- [ ] Embed chapters in M4B
- [ ] Track conversion progress
- [ ] Handle errors gracefully
- [ ] Support batch processing
- [ ] Manage user settings

## Performance Targets

- [ ] Conversion speed ≥ Java version
- [ ] Binary size < 50MB
- [ ] Memory usage < 500MB peak
- [ ] Startup time < 1 second
- [ ] No memory leaks
- [ ] Responsive UI during conversion

## Quality Targets

- [ ] Code coverage > 80%
- [ ] All tests passing
- [ ] Cross-platform working
- [ ] No crashes
- [ ] Helpful error messages
- [ ] Complete documentation

## User Experience

- [ ] Intuitive interface
- [ ] Clear workflow
- [ ] Helpful feedback
- [ ] Keyboard shortcuts
- [ ] Fast responsiveness
- [ ] No freezes

---

# APPENDIX F: Common Tasks During Implementation

## When Adding New Format Support

1. Add AudioFormat variant to enum
2. Add Symphonia feature flag if needed
3. Update format detection
4. Add metadata extraction
5. Test loading sample file
6. Test metadata extraction
7. Add conversion support
8. Test full conversion
9. Update documentation
10. Add to compatibility matrix

## When Adding New Tauri Command

1. Create command function in appropriate module
2. Add `#[tauri::command]` attribute
3. Register command with Tauri builder
4. Test command manually
5. Add to API documentation
6. Implement frontend call
7. Test frontend-backend integration
8. Add integration test

## When Fixing a Bug

1. Create issue on GitHub
2. Create bugfix branch
3. Add test that reproduces bug
4. Implement fix
5. Verify test passes
6. Verify no regressions
7. Update documentation if needed
8. Create pull request
9. Link to issue

## When Optimizing Performance

1. Profile code to identify bottleneck
2. Understand why it's slow
3. Research solution
4. Implement optimization
5. Benchmark before/after
6. Verify no regression in quality
7. Update documentation
8. Share findings with team

---

This plan provides a complete, task-based implementation roadmap that can be followed systematically. Each task is specific enough to guide implementation while allowing flexibility in approach.
