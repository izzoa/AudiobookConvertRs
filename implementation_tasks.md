# AudioBookConverter Rust Implementation: Complete Task-Based Plan

**Status**: Ready for LLM-assisted implementation
**Version**: 1.0 with Symphonia integration
**Total Phases**: 9
**Estimated Duration**: 12-16 weeks
**Team Size**: 1-2 developers

---

# PHASE 0: Pre-Conversion Planning & Setup (Weeks 1-2)

## 0.1 Environment & Repository Setup

- [ ] Create new GitHub repository: `AudiobookConvertRs`
- [ ] Clone and locally archive original Java AudioBookConverter for reference
- [ ] Set up Rust project with Cargo: `cargo new --name AudiobookConvertRs`
- [ ] Create initial directory structure:
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
- [ ] Initialize Git with appropriate `.gitignore` (Rust, Node, OS files)
- [ ] Create GitHub Actions CI/CD pipeline for multi-platform builds (Windows, macOS, Linux)

## 0.2 Documentation & Analysis

- [ ] Create `ARCHITECTURE.md` documenting overall design
- [ ] Create `AUDIO_FORMATS.md` documenting all supported audio formats
- [ ] Create `METADATA_SPEC.md` documenting metadata handling (ID3, Vorbis, MP4 atoms)
- [ ] Create `API.md` documenting Tauri IPC commands
- [ ] Create `DEVELOPMENT.md` with setup instructions for contributors
- [ ] Document migration from Java version (feature differences, compatibility notes)
- [ ] List all known limitations compared to Java version

## 0.3 Dependency Research & Selection

- [ ] Verify Symphonia latest version and feature flags
- [ ] Research mp4ameta API and version compatibility
- [ ] Verify FFmpeg bindings (`ffmpeg-sys`) version and platform support
- [ ] Confirm image crate supports all needed formats (JPEG, PNG, BMP, GIF)
- [ ] Research Tauri latest version and platform requirements
- [ ] Document all dependencies in `DEPENDENCIES.md` with rationale
- [ ] Check license compatibility (ensure all crates compatible with your license)

## 0.4 Project Configuration

- [ ] Create `Cargo.toml` with all confirmed dependencies
- [ ] Set up feature flags for:
  - [ ] `symphonia` with features: isomp4, flac, mp3, aac, wav, ogg, vorbis
  - [ ] `symphonia-metadata` for metadata reading
  - [ ] Optional: test feature for integration tests
- [ ] Create `tauri.conf.json` with basic configuration
- [ ] Set up build profile optimizations:
  - [ ] `debug` profile for development
  - [ ] `release` profile with LTO and optimizations
- [ ] Create `.env.example` for environment variables (FFmpeg path, etc.)
- [ ] Set up Rust toolchain version file (`.rust-version` or `rust-toolchain.toml`)

## 0.5 Proof of Concepts

- [ ] Create small POC for Symphonia loading an MP3 file
- [ ] Create small POC for Symphonia extracting metadata
- [ ] Create small POC for FFmpeg command execution from Rust
- [ ] Create small POC for Tauri IPC command
- [ ] Document POC findings in separate branch or document

---

# PHASE 1: Foundation & Core Architecture (Weeks 3-4)

## 1.1 Error Handling Framework

- [ ] Create `src-tauri/src/error.rs` with custom error types
- [ ] Define error variants for:
  - [ ] File I/O errors
  - [ ] FFmpeg execution errors
  - [ ] Metadata parsing errors
  - [ ] Audio format errors
  - [ ] Validation errors
  - [ ] IPC/Tauri errors
- [ ] Implement `thiserror::Error` trait for all error types
- [ ] Implement From/Into conversions for common errors
- [ ] Create error display messages (user-friendly messages)
- [ ] Document error codes and recovery strategies

## 1.2 Logging Configuration

- [ ] Set up `tracing` and `tracing-subscriber`
- [ ] Create logging module: `src-tauri/src/logging.rs`
- [ ] Configure log levels:
  - [ ] Default: INFO for release builds
  - [ ] Default: DEBUG for development builds
- [ ] Implement structured logging for key operations
- [ ] Configure log output (file and console)
- [ ] Add performance timing logs for audio operations

## 1.3 Configuration Management

- [ ] Create `src-tauri/src/config.rs` module
- [ ] Define UserSettings struct with fields:
  - [ ] Default bitrate (128, 192, 256 kbps)
  - [ ] Default output directory
  - [ ] Auto-detect artwork (true/false)
  - [ ] Concurrent job count
  - [ ] FFmpeg path (auto-detect or custom)
  - [ ] Theme preference (light/dark)
  - [ ] Recent files list
- [ ] Implement config serialization/deserialization (serde_json)
- [ ] Create config file location:
  - [ ] Windows: `%APPDATA%/AudiobookConvertRs/config.json`
  - [ ] macOS: `~/Library/Application Support/AudiobookConvertRs/config.json`
  - [ ] Linux: `~/.config/AudiobookConvertRs/config.json`
- [ ] Implement config loading with defaults fallback
- [ ] Implement config saving when user updates settings

## 1.4 Core Data Structures

Create `src-tauri/src/models.rs` with all data types:

- [ ] `AudioFormat` enum:
  ```
  Mp3, Flac, M4a, M4b, Wav, Ogg, Wma
  ```
- [ ] `Metadata` struct:
  - [ ] title: Option<String>
  - [ ] artist: Option<String>
  - [ ] album: Option<String>
  - [ ] narrator: Option<String>
  - [ ] artwork_path: Option<PathBuf>
  - [ ] year: Option<u16>
  - [ ] genre: Option<String>
  - [ ] comments: Option<String>
- [ ] `Chapter` struct:
  - [ ] index: usize
  - [ ] title: String
  - [ ] start_time_ms: u64
  - [ ] end_time_ms: u64
  - [ ] auto_detected: bool
- [ ] `AudioFile` struct:
  - [ ] path: PathBuf
  - [ ] format: AudioFormat
  - [ ] duration_ms: u64
  - [ ] bitrate: Option<u32>
  - [ ] sample_rate: Option<u32>
  - [ ] metadata: Metadata
- [ ] `AudioBook` struct:
  - [ ] id: String (UUID)
  - [ ] title: String
  - [ ] author: Option<String>
  - [ ] files: Vec<AudioFile>
  - [ ] chapters: Vec<Chapter>
  - [ ] output_metadata: Metadata
  - [ ] created_at: DateTime
  - [ ] modified_at: DateTime
- [ ] `ConversionSettings` struct:
  - [ ] bitrate_kbps: u32
  - [ ] sample_rate: Option<u32>
  - [ ] output_format: AudioFormat (always M4b for now)
  - [ ] output_path: PathBuf
  - [ ] concurrent_jobs: u8
- [ ] `ConversionJob` struct:
  - [ ] id: String (UUID)
  - [ ] audiobook: AudioBook
  - [ ] settings: ConversionSettings
  - [ ] status: JobStatus (Queued, Running, Paused, Complete, Error, Cancelled)
  - [ ] progress_percent: f32
  - [ ] current_file_index: usize
  - [ ] error_message: Option<String>
  - [ ] created_at: DateTime
  - [ ] started_at: Option<DateTime>
  - [ ] completed_at: Option<DateTime>
- [ ] `ConversionProgress` struct (for IPC events):
  - [ ] job_id: String
  - [ ] file_index: usize
  - [ ] current_file: String
  - [ ] file_progress: f32
  - [ ] total_progress: f32
  - [ ] time_remaining_sec: u32
  - [ ] estimated_finish: DateTime

- [ ] Implement Serialize/Deserialize for all structs (serde)
- [ ] Implement Display trait for user-facing strings
- [ ] Implement Default trait where appropriate
- [ ] Add validation methods (e.g., validate_chapter_times)

## 1.5 Application State Management

- [ ] Create `src-tauri/src/state.rs` module
- [ ] Define AppState struct:
  - [ ] current_audiobook: Option<AudioBook>
  - [ ] conversion_queue: Vec<ConversionJob>
  - [ ] active_job: Option<String> (job_id)
  - [ ] user_settings: UserSettings
  - [ ] ffmpeg_available: bool
  - [ ] recent_audiobooks: Vec<String> (file paths)
- [ ] Implement thread-safe state with `Arc<Mutex<AppState>>`
- [ ] Create state mutation methods
- [ ] Implement state persistence (save/load from disk)

---

# PHASE 2: Audio Processing & Format Support (Weeks 5-6)

## 2.1 Symphonia Integration - Audio Loading

- [ ] Create `src-tauri/src/audio/loader.rs` module
- [ ] Implement `AudioLoader` struct with methods:
  - [ ] `detect_format(path: &Path) -> Result<AudioFormat>`
    - [ ] Use Symphonia::Probe to auto-detect
    - [ ] Validate against file extension as fallback
    - [ ] Return appropriate error if unsupported
  - [ ] `load_audio_file(path: &Path) -> Result<AudioFile>`
    - [ ] Open file with Symphonia
    - [ ] Extract duration from format reader
    - [ ] Read codec information (bitrate, sample rate)
    - [ ] Extract metadata using symphonia-metadata
    - [ ] Detect and extract artwork
    - [ ] Populate AudioFile struct
  - [ ] `get_duration_ms(path: &Path) -> Result<u64>`
    - [ ] Quick duration calculation without full decode
  - [ ] `get_metadata(path: &Path) -> Result<Metadata>`
    - [ ] Extract all available metadata
    - [ ] Return standardized Metadata struct

- [ ] Handle Symphonia error types and convert to app errors
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

- [ ] Create `src-tauri/src/audio/format.rs` module
- [ ] Implement format detection strategy:
  - [ ] Use Symphonia for authoritative format detection
  - [ ] Cache format detection results
- [ ] Create validation functions:
  - [ ] `is_supported_format(path: &Path) -> bool`
  - [ ] `validate_audio_file(path: &Path) -> Result<()>`
    - [ ] File exists
    - [ ] Readable
    - [ ] Valid audio format
    - [ ] Has audio stream
- [ ] Create file type checking:
  - [ ] Check magic bytes (file signature)
  - [ ] Verify with Symphonia
  - [ ] Detect incorrect extensions
- [ ] Add comprehensive logging for format detection

## 2.3 Duration Calculation & Timing

- [ ] Create `src-tauri/src/audio/timing.rs` module
- [ ] Implement duration functions:
  - [ ] `calculate_total_duration(files: &[AudioFile]) -> u64`
  - [ ] `format_duration_ms(ms: u64) -> String` (HH:MM:SS)
  - [ ] `parse_time_string(s: &str) -> Result<u64>` (parse user input)
  - [ ] `convert_ms_to_chapters(duration_ms: u64, chapter_count: usize) -> Vec<u64>` (auto-split)
- [ ] Handle edge cases:
  - [ ] Zero duration files
  - [ ] Very large durations (24+ hours)
  - [ ] Rounding errors

## 2.4 Symphonia Metadata Extraction

- [ ] Create `src-tauri/src/audio/symphonia_metadata.rs` module
- [ ] Implement metadata extraction functions:
  - [ ] `extract_metadata_from_symphonia(format_reader) -> Result<Metadata>`
    - [ ] Extract from ID3v1 tags (MP3)
    - [ ] Extract from ID3v2 tags (MP3)
    - [ ] Extract from Vorbis comments (FLAC, OGG)
    - [ ] Extract from MP4 atoms (M4A/M4B)
    - [ ] Handle missing fields gracefully
  - [ ] `extract_artwork(metadata) -> Result<Option<Vec<u8>>>`
    - [ ] Extract embedded artwork
    - [ ] Return image data
    - [ ] Support multiple formats (JPEG, PNG, GIF, BMP)
  - [ ] `get_bitrate(format_reader) -> Option<u32>`
  - [ ] `get_sample_rate(codec_params) -> Option<u32>`
- [ ] Implement metadata normalization:
  - [ ] Trim whitespace
  - [ ] Handle encoding issues
  - [ ] Sanitize for filesystem safety
- [ ] Add tests:
  - [ ] Extract from MP3 with ID3v2
  - [ ] Extract from FLAC with Vorbis
  - [ ] Extract from M4A/M4B
  - [ ] Handle missing metadata fields
  - [ ] Extract artwork from each format

## 2.5 Batch File Processing

- [ ] Create `src-tauri/src/audio/batch.rs` module
- [ ] Implement batch loading:
  - [ ] `load_audiobook_from_files(files: Vec<PathBuf>) -> Result<AudioBook>`
    - [ ] Load each file with Symphonia
    - [ ] Validate all files
    - [ ] Calculate total duration
    - [ ] Merge metadata (use first file's artist, etc.)
    - [ ] Create initial chapters from file boundaries
- [ ] Implement file validation for batch:
  - [ ] Check all files valid
  - [ ] Check sufficient disk space for output
  - [ ] Check all files same sample rate (optional warning)
- [ ] Implement progress tracking for batch loading:
  - [ ] Emit progress events for UI
  - [ ] Track files processed

## 2.6 FFmpeg Integration - Command Building

- [ ] Create `src-tauri/src/ffmpeg/mod.rs` module
- [ ] Create `src-tauri/src/ffmpeg/command_builder.rs`
- [ ] Implement FFmpeg command building:
  - [ ] `FFmpegCommand` struct to build commands
  - [ ] `builder()` method to create builder
  - [ ] `.input(path)` to add input file
  - [ ] `.audio_codec("aac")` to set codec
  - [ ] `.bitrate(kbps)` to set bitrate
  - [ ] `.sample_rate(hz)` to set sample rate
  - [ ] `.output(path)` to set output
  - [ ] `.build()` to create final command string
- [ ] Implement specific command builders:
  - [ ] `build_convert_command(input, output, settings) -> String`
    - [ ] Input file with format auto-detection
    - [ ] Output format M4B/MP4
    - [ ] AAC codec
    - [ ] User-specified bitrate
    - [ ] User-specified sample rate
  - [ ] `build_concat_command(input_files, output) -> String`
    - [ ] Use FFmpeg concat demuxer
    - [ ] Multiple input files
    - [ ] Output single M4B
  - [ ] `build_metadata_embed_command(input, output, metadata) -> String`
    - [ ] Embed metadata into M4B
    - [ ] Add title, artist, etc.
- [ ] Add validation:
  - [ ] Verify bitrate valid (8-320 kbps typical)
  - [ ] Verify sample rate valid (8000, 16000, 22050, 44100, 48000 Hz)
  - [ ] Verify output path writable

## 2.7 FFmpeg Detection & Setup

- [ ] Create `src-tauri/src/ffmpeg/detection.rs`
- [ ] Implement FFmpeg detection:
  - [ ] Check PATH environment variable
  - [ ] Check common installation locations:
    - [ ] Windows: C:\ffmpeg\bin, Program Files
    - [ ] macOS: /usr/local/bin, /opt/homebrew/bin
    - [ ] Linux: /usr/bin, /usr/local/bin
  - [ ] Check user-configured path in settings
  - [ ] Verify FFmpeg executable exists and is runnable
  - [ ] Cache detection result
- [ ] Implement version checking:
  - [ ] Run `ffmpeg -version` to get version
  - [ ] Verify minimum version (e.g., 4.0+)
  - [ ] Log version info for debugging
- [ ] Create error handling:
  - [ ] FFmpeg not found error with helpful instructions
  - [ ] FFmpeg version too old error
  - [ ] Log installation instructions per OS

---

# PHASE 3: Metadata Handling & Chapters (Weeks 7-8)

## 3.1 Metadata Extraction (Symphonia)

- [ ] Create `src-tauri/src/metadata/extraction.rs`
- [ ] Implement ID3 tag extraction (MP3):
  - [ ] `extract_id3_tags(path: &Path) -> Result<Metadata>`
  - [ ] Read ID3v1 tags (back of file)
  - [ ] Read ID3v2 tags (front of file, takes priority)
  - [ ] Extract: title, artist, album, year, genre, comments
- [ ] Implement Vorbis comment extraction (FLAC, OGG):
  - [ ] `extract_vorbis_comments(path: &Path) -> Result<Metadata>`
  - [ ] Extract all standard fields
  - [ ] Handle multi-value fields
- [ ] Implement MP4 atom extraction (M4A/M4B):
  - [ ] `extract_mp4_metadata(path: &Path) -> Result<Metadata>`
  - [ ] Use mp4ameta for reading
  - [ ] Extract: title, artist, album, year, genre, comments
- [ ] Implement metadata normalization:
  - [ ] Trim whitespace
  - [ ] Handle encoding issues (UTF-8, Latin-1, etc.)
  - [ ] Unify field names across formats
- [ ] Add comprehensive tests:
  - [ ] MP3 with ID3v1
  - [ ] MP3 with ID3v2
  - [ ] FLAC with Vorbis
  - [ ] M4A with MP4 atoms
  - [ ] Missing metadata fields

## 3.2 Artwork Handling

- [ ] Create `src-tauri/src/metadata/artwork.rs`
- [ ] Implement artwork detection:
  - [ ] `extract_embedded_artwork(path: &Path) -> Result<Option<Vec<u8>>>`
    - [ ] Use Symphonia to get embedded artwork
    - [ ] Convert to standardized format (JPEG)
    - [ ] Return image data bytes
  - [ ] `scan_directory_for_artwork(dir: &Path) -> Result<Option<PathBuf>>`
    - [ ] Search for: cover.jpg, folder.jpg, cover.png, etc.
    - [ ] Return path to found artwork
- [ ] Implement artwork processing:
  - [ ] `resize_artwork_if_needed(data: &[u8], max_size: usize) -> Result<Vec<u8>>`
    - [ ] Use image crate to resize if needed
    - [ ] Target: 500x500 px or similar
  - [ ] `detect_image_format(data: &[u8]) -> Result<ImageFormat>`
    - [ ] Identify JPEG, PNG, GIF, BMP
    - [ ] Return format enum
- [ ] Store artwork:
  - [ ] Save to temp directory during conversion
  - [ ] Embed into final M4B with mp4ameta

## 3.3 Chapter Management

- [ ] Create `src-tauri/src/metadata/chapters.rs`
- [ ] Implement chapter data structures (already in models.rs)
- [ ] Implement chapter creation:
  - [ ] `create_chapters_from_files(files: &[AudioFile]) -> Vec<Chapter>`
    - [ ] Each file becomes a chapter
    - [ ] Start times calculated from durations
    - [ ] Titles from filenames (remove extensions)
  - [ ] `create_chapters_auto_split(total_duration_ms: u64, count: usize) -> Vec<Chapter>`
    - [ ] Split by time evenly
    - [ ] E.g., 4-hour book split into 12x 20-minute chapters
  - [ ] `extract_chapters_from_metadata(path: &Path) -> Result<Vec<Chapter>>`
    - [ ] Extract QuickTime chapters from M4B if present
    - [ ] Use mp4ameta to read
- [ ] Implement chapter validation:
  - [ ] `validate_chapters(chapters: &[Chapter], total_duration_ms: u64) -> Result<()>`
    - [ ] No overlapping chapters
    - [ ] No gaps (optional)
    - [ ] End times within total duration
    - [ ] Valid chapter order
- [ ] Implement chapter editing:
  - [ ] `update_chapter(chapters: &mut [Chapter], index: usize, update: ChapterUpdate) -> Result<()>`
  - [ ] `reorder_chapters(chapters: &mut [Chapter], old_index: usize, new_index: usize) -> Result<()>`
  - [ ] `delete_chapter(chapters: &mut [Chapter], index: usize) -> Result<()>`
  - [ ] `add_chapter(chapters: &mut [Chapter], chapter: Chapter) -> Result<()>`

## 3.4 Metadata Writing (mp4ameta)

- [ ] Create `src-tauri/src/metadata/writing.rs`
- [ ] Implement metadata writing to M4B:
  - [ ] `write_metadata_to_m4b(m4b_path: &Path, metadata: &Metadata) -> Result<()>`
    - [ ] Use mp4ameta library
    - [ ] Write title, artist, album, etc.
    - [ ] Handle optional fields
  - [ ] `embed_artwork_in_m4b(m4b_path: &Path, artwork: &[u8]) -> Result<()>`
    - [ ] Convert artwork to appropriate format if needed
    - [ ] Embed using mp4ameta
  - [ ] `write_chapters_to_m4b(m4b_path: &Path, chapters: &[Chapter], total_duration_ms: u64) -> Result<()>`
    - [ ] Use mp4ameta for QuickTime chapters
    - [ ] Write all chapters with correct timing
- [ ] Implement metadata backup:
  - [ ] Before writing to M4B, backup original metadata
  - [ ] Save to temporary file in case of error
- [ ] Add comprehensive tests:
  - [ ] Write metadata to new M4B
  - [ ] Update existing M4B metadata
  - [ ] Add artwork to M4B
  - [ ] Add chapters to M4B
  - [ ] Verify written metadata is readable

## 3.5 Metadata Editing API

- [ ] Create `src-tauri/src/metadata/editor.rs`
- [ ] Implement in-memory metadata editing:
  - [ ] `AudioBook` stores editable metadata
  - [ ] `update_metadata(audiobook: &mut AudioBook, new_metadata: Metadata) -> Result<()>`
  - [ ] `update_chapter(audiobook: &mut AudioBook, index: usize, title: String, times: (u64, u64)) -> Result<()>`
- [ ] Implement validation on edits:
  - [ ] Sanitize input
  - [ ] Validate chapter times don't exceed duration
  - [ ] Validate metadata fields (length limits, special characters)
- [ ] Track unsaved changes:
  - [ ] `audiobook.is_dirty()` method
  - [ ] Mark clean on save/conversion

---

# PHASE 4: GUI Implementation - Backend (Weeks 9-10)

## 4.1 Tauri Project Setup

- [ ] Initialize Tauri in project:
  - [ ] Run `npm create tauri-app@latest` or equivalent
  - [ ] Configure Tauri backend/frontend split
  - [ ] Set up Cargo workspace if not already
- [ ] Configure `tauri.conf.json`:
  - [ ] Set app name, version, description
  - [ ] Configure window settings (size, resizable, etc.)
  - [ ] Set build targets (Windows, macOS, Linux)
  - [ ] Configure updater (optional for now)
  - [ ] Set security settings
- [ ] Create Tauri app state:
  - [ ] Integrate AppState from Phase 1.5
  - [ ] Make available to all commands
  - [ ] Handle state persistence

## 4.2 Tauri IPC Commands - Core

- [ ] Create `src-tauri/src/commands/mod.rs`
- [ ] Create `src-tauri/src/commands/audiobook.rs`
- [ ] Implement audiobook commands:
  - [ ] `#[tauri::command] create_audiobook_from_files(paths: Vec<String>) -> Result<AudioBook, String>`
    - [ ] Load files using Symphonia
    - [ ] Create AudioBook struct
    - [ ] Return to frontend
  - [ ] `#[tauri::command] load_audiobook_file(path: String) -> Result<AudioBook, String>`
    - [ ] Load previously saved audiobook
  - [ ] `#[tauri::command] save_audiobook_project(audiobook: AudioBook, path: String) -> Result<(), String>`
    - [ ] Save audiobook metadata as JSON
    - [ ] For later resumption
  - [ ] `#[tauri::command] clear_current_audiobook() -> Result<(), String>`
    - [ ] Reset app state

## 4.3 Tauri IPC Commands - Metadata

- [ ] Create `src-tauri/src/commands/metadata.rs`
- [ ] Implement metadata commands:
  - [ ] `#[tauri::command] update_audiobook_metadata(audiobook_id: String, metadata: Metadata) -> Result<(), String>`
  - [ ] `#[tauri::command] add_chapter(audiobook_id: String, chapter: Chapter) -> Result<(), String>`
  - [ ] `#[tauri::command] update_chapter(audiobook_id: String, index: usize, chapter: Chapter) -> Result<(), String>`
  - [ ] `#[tauri::command] delete_chapter(audiobook_id: String, index: usize) -> Result<(), String>`
  - [ ] `#[tauri::command] reorder_chapters(audiobook_id: String, old_idx: usize, new_idx: usize) -> Result<(), String>`
  - [ ] `#[tauri::command] auto_create_chapters(audiobook_id: String, count: usize) -> Result<(), String>`
  - [ ] `#[tauri::command] upload_artwork(audiobook_id: String, image_path: String) -> Result<(), String>`

## 4.4 Tauri IPC Commands - File Operations

- [ ] Create `src-tauri/src/commands/files.rs`
- [ ] Implement file commands:
  - [ ] `#[tauri::command] pick_files() -> Result<Vec<String>, String>`
    - [ ] Open file picker dialog
    - [ ] Return selected file paths
  - [ ] `#[tauri::command] pick_directory() -> Result<String, String>`
    - [ ] Open directory picker
    - [ ] Return selected directory path
  - [ ] `#[tauri::command] validate_files(paths: Vec<String>) -> Result<Vec<FileValidation>, String>`
    - [ ] Check each file
    - [ ] Return: valid, format, duration, size
  - [ ] `#[tauri::command] get_file_info(path: String) -> Result<AudioFile, String>`
    - [ ] Get detailed info on single file

## 4.5 Tauri IPC Commands - Settings

- [ ] Create `src-tauri/src/commands/settings.rs`
- [ ] Implement settings commands:
  - [ ] `#[tauri::command] get_settings() -> Result<UserSettings, String>`
  - [ ] `#[tauri::command] update_settings(settings: UserSettings) -> Result<(), String>`
  - [ ] `#[tauri::command] get_default_settings() -> Result<UserSettings, String>`
  - [ ] `#[tauri::command] reset_settings() -> Result<(), String>`
  - [ ] `#[tauri::command] check_ffmpeg_available() -> Result<bool, String>`
  - [ ] `#[tauri::command] get_ffmpeg_version() -> Result<String, String>`

## 4.6 Tauri IPC Commands - Conversion

- [ ] Create `src-tauri/src/commands/conversion.rs`
- [ ] Implement conversion commands:
  - [ ] `#[tauri::command] start_conversion(audiobook_id: String, settings: ConversionSettings) -> Result<String, String>`
    - [ ] Validate inputs
    - [ ] Create ConversionJob
    - [ ] Queue for processing
    - [ ] Return job_id
  - [ ] `#[tauri::command] cancel_conversion(job_id: String) -> Result<(), String>`
    - [ ] Stop ongoing conversion
    - [ ] Clean up temporary files
  - [ ] `#[tauri::command] get_conversion_status(job_id: String) -> Result<ConversionJob, String>`
    - [ ] Return current job status and progress
  - [ ] `#[tauri::command] pause_conversion(job_id: String) -> Result<(), String>` (optional)
  - [ ] `#[tauri::command] resume_conversion(job_id: String) -> Result<(), String>` (optional)

## 4.7 Tauri Events (Backend → Frontend)

- [ ] Create event emitters in `src-tauri/src/events.rs`
- [ ] Implement event types:
  - [ ] `ConversionProgress` - emit during conversion
    - [ ] job_id, current_file, file_progress, total_progress, time_remaining
  - [ ] `ConversionComplete` - emit on success
    - [ ] job_id, output_path, duration
  - [ ] `ConversionError` - emit on error
    - [ ] job_id, error_message, recoverable
  - [ ] `FilesLoaded` - emit after files analyzed
    - [ ] file_count, total_duration, metadata
  - [ ] `MetadataUpdated` - emit after metadata changes
    - [ ] audiobook_id

- [ ] Implement event emission:
  - [ ] `emit_progress(window, event: ConversionProgress)`
  - [ ] `emit_completion(window, event: ConversionComplete)`
  - [ ] `emit_error(window, event: ConversionError)`

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

- [ ] Create `src-tauri/src/ffmpeg/executor.rs`
- [ ] Implement command execution:
  - [ ] `execute_ffmpeg_command(command: String) -> Result<FFmpegOutput, ConversionError>`
    - [ ] Run as subprocess using `std::process::Command`
    - [ ] Capture stdout and stderr
    - [ ] Handle exit codes
    - [ ] Parse output for progress info
  - [ ] `run_command_with_timeout(command: String, timeout: Duration) -> Result<...>`
    - [ ] Kill process if timeout exceeded
    - [ ] Return timeout error
- [ ] Implement output parsing:
  - [ ] `parse_ffmpeg_progress(stderr: &str) -> Option<FFmpegProgress>`
    - [ ] Parse output for: time, frame, speed, bitrate
    - [ ] Extract current timestamp (time=HH:MM:SS.ms)
  - [ ] `calculate_progress_percent(current_time_ms: u64, total_time_ms: u64) -> f32`
    - [ ] Calculate percentage 0.0-100.0
- [ ] Add error handling:
  - [ ] FFmpeg not found
  - [ ] FFmpeg execution failed
  - [ ] Invalid command
  - [ ] Output parsing errors

## 5.2 Progress Tracking & Reporting

- [ ] Create `src-tauri/src/conversion/progress.rs`
- [ ] Implement progress tracking:
  - [ ] `ProgressTracker` struct to track conversion state
  - [ ] `start_tracking()` when conversion begins
  - [ ] `update_progress(current_ms: u64, total_ms: u64)` on each parse
  - [ ] `calculate_time_remaining() -> Duration` based on current speed
  - [ ] `get_current_progress() -> ConversionProgress` to emit
- [ ] Implement progress thread:
  - [ ] Spawn separate thread for FFmpeg output parsing
  - [ ] Parse progress in background
  - [ ] Send updates via channel to main thread
  - [ ] Main thread emits Tauri events to frontend
- [ ] Implement progress smoothing:
  - [ ] Avoid too-frequent updates (1-2 sec minimum between events)
  - [ ] Calculate average speed (not instantaneous)

## 5.3 Conversion Job Queue

- [ ] Create `src-tauri/src/conversion/queue.rs`
- [ ] Implement job queue:
  - [ ] `ConversionQueue` struct maintaining job list
  - [ ] `add_job(job: ConversionJob)` to add to queue
  - [ ] `get_next_job() -> Option<ConversionJob>` to pop
  - [ ] `update_job_status(job_id: String, status: JobStatus)`
  - [ ] `cancel_job(job_id: String)` to remove from queue
  - [ ] `get_all_jobs() -> Vec<ConversionJob>` for queue status
- [ ] Implement queue persistence:
  - [ ] Save queue to disk on state changes
  - [ ] Load queue on app startup (resume interrupted conversions)
  - [ ] Clear completed jobs periodically

## 5.4 Worker Thread & Parallel Processing

- [ ] Create `src-tauri/src/conversion/worker.rs`
- [ ] Implement worker management:
  - [ ] Create worker thread pool using `rayon` or manual threads
  - [ ] Configure max concurrent workers (from settings)
  - [ ] Workers pull jobs from queue
  - [ ] Workers execute conversion pipeline
- [ ] Implement worker execution:
  - [ ] `ConversionWorker` that processes single job
  - [ ] Load input files
  - [ ] Execute FFmpeg conversion
  - [ ] Embed metadata with mp4ameta
  - [ ] Embed artwork
  - [ ] Write chapters
  - [ ] Verify output file
  - [ ] Update job status
  - [ ] Emit completion event
- [ ] Implement worker error handling:
  - [ ] Catch all error types
  - [ ] Log detailed error info
  - [ ] Update job with error status
  - [ ] Emit error event
  - [ ] Move to next job

## 5.5 Conversion Pipeline - Main Flow

- [ ] Create `src-tauri/src/conversion/pipeline.rs`
- [ ] Implement complete conversion pipeline:
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

- [ ] Implement each step:
  - [ ] `validate_conversion_inputs(audiobook: &AudioBook, settings: &ConversionSettings) -> Result<()>`
  - [ ] `prepare_temp_directory() -> Result<PathBuf>`
  - [ ] `execute_encoding_pipeline(audiobook: &AudioBook, settings: &ConversionSettings, progress_cb: Fn(f32)) -> Result<PathBuf>`
  - [ ] `embed_all_metadata(m4b_path: &Path, audiobook: &AudioBook, artwork: Option<Vec<u8>>) -> Result<()>`
  - [ ] `verify_output_file(m4b_path: &Path) -> Result<()>`

## 5.6 Error Recovery

- [ ] Create `src-tauri/src/conversion/recovery.rs`
- [ ] Implement error recovery strategies:
  - [ ] `retry_failed_conversion(job: &ConversionJob, max_retries: usize) -> Result<()>`
  - [ ] `skip_current_file(job: &mut ConversionJob) -> Result<()>`
  - [ ] `resume_from_checkpoint(job: &ConversionJob) -> Result<()>` (if possible)
  - [ ] `rollback_partial_conversion(job: &ConversionJob) -> Result<()>`
    - [ ] Delete incomplete output file
    - [ ] Clean temp files
- [ ] Implement error reporting:
  - [ ] Detailed error logging
  - [ ] User-friendly error messages
  - [ ] Suggestions for recovery
  - [ ] FFmpeg error code documentation

## 5.7 Temporary File Management

- [ ] Create `src-tauri/src/conversion/temp.rs`
- [ ] Implement temp directory management:
  - [ ] `create_temp_directory() -> Result<PathBuf>`
    - [ ] Create in OS temp directory
    - [ ] Use unique name (job_id based)
  - [ ] `cleanup_temp_directory(path: &Path) -> Result<()>`
    - [ ] Delete temp directory and contents
    - [ ] Handle Windows file locking issues
    - [ ] Retry on failure
  - [ ] `list_temp_files(job_id: String) -> Result<Vec<PathBuf>>`
    - [ ] Find all temp files for job
- [ ] Implement cleanup on errors:
  - [ ] Register cleanup functions
  - [ ] Ensure cleanup happens even on panic
  - [ ] Use RAII pattern (Drop trait)

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

- [ ] Create `src-tauri/src/audio/quality.rs`
- [ ] Implement quality detection:
  - [ ] `analyze_audio_levels(file_path: &Path) -> Result<AudioLevels>`
    - [ ] Use FFmpeg to analyze
    - [ ] Return peak level, RMS, loudness info
  - [ ] `suggest_normalization(levels: &AudioLevels) -> NormalizationSuggestion`
    - [ ] Recommend normalization if levels vary
    - [ ] Suggest target level
- [ ] Implement audio normalization:
  - [ ] `apply_normalization(input: &Path, output: &Path, target_level: f32) -> Result<()>`
    - [ ] Use FFmpeg loudnorm filter
    - [ ] Re-encode with normalization

## 6.2 Format-Specific Handling

- [ ] Implement existing M4B handling:
  - [ ] Detect when input is M4B
  - [ ] Read existing chapters
  - [ ] Preserve chapters during re-encoding
  - [ ] Allow overwriting metadata
- [ ] Implement FLAC cue sheet support (optional):
  - [ ] Extract cue sheets from FLAC
  - [ ] Convert to chapters
- [ ] Implement MP3 chapter support (optional):
  - [ ] Use ID3 CHAP frames
  - [ ] Convert to M4B chapters

## 6.3 Batch Processing Enhancements

- [ ] Implement batch mode:
  - [ ] Load multiple audiobooks
  - [ ] Apply same settings to all
  - [ ] Queue all for parallel processing
  - [ ] Report progress per audiobook
- [ ] Implement batch presets:
  - [ ] Save conversion settings as preset
  - [ ] Load preset for future conversions
  - [ ] Manage presets (create, update, delete)
- [ ] Implement dry-run mode:
  - [ ] Analyze without converting
  - [ ] Preview final output
  - [ ] Estimate time and space needed

## 6.4 Project Management

- [ ] Implement project save/load:
  - [ ] Save audiobook with all metadata as JSON
  - [ ] Load project to resume editing
  - [ ] Version saved projects for compatibility
  - [ ] Export project metadata
- [ ] Implement recent projects:
  - [ ] Track recently opened projects
  - [ ] Quick-open from main menu
  - [ ] Clean up old project files

## 6.5 User Experience Enhancements

- [ ] Implement keyboard shortcuts:
  - [ ] Ctrl+O to open files
  - [ ] Ctrl+S to save project
  - [ ] Ctrl+C to convert
  - [ ] Escape to cancel
- [ ] Implement undo/redo:
  - [ ] Track metadata edits
  - [ ] Undo/redo chapter changes
  - [ ] Implement command pattern
- [ ] Implement file browser:
  - [ ] Browse file system
  - [ ] Preview files
  - [ ] Drag-and-drop support

## 6.6 Performance Optimization

- [ ] Profile conversion pipeline:
  - [ ] Measure time spent in each phase
  - [ ] Identify bottlenecks
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
- [ ] Test FFmpeg detection:
  - [ ] Detects FFmpeg when present
  - [ ] Reports not found when absent
  - [ ] Finds FFmpeg in common locations
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
