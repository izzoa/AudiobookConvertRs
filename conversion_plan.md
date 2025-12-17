# AudioBookConverter: Java → Rust Conversion Plan

**Status**: Comprehensive conversion roadmap (no code implementation)
**Target**: Full feature parity with Java version 6.5.2
**Timeline**: 12-16 weeks for production-ready release

---

## PHASE 0: Pre-Conversion Planning (Weeks 1-2)

### 0.1 Codebase Analysis
- [ ] Clone and analyze full AudioBookConverter Java repository
- [ ] Map Java class structure and dependencies
- [ ] Document all FFmpeg integration points
- [ ] Identify third-party library equivalents in Rust
- [ ] Extract and document all metadata handling logic
- [ ] Review and understand M4B/MP4 container structure
- [ ] Document chapter metadata format standards
- [ ] Analyze batch processing and concurrency patterns

### 0.2 Architecture Decision Documentation
- [ ] Finalize GUI framework choice: Tauri (recommended)
  - Reason: Modern, lightweight, web-based UI, excellent cross-platform
  - Alternative: Druid (native Rust UI)
- [ ] Define backend/frontend communication protocol (IPC)
- [ ] Plan FFmpeg integration strategy:
  - Use `ffmpeg-sys` for FFmpeg bindings
  - Consider `ez-ffmpeg` wrapper for simplified API
  - Alternative: `rsmpeg` bindings
- [ ] Document metadata handling approach
- [ ] Plan database requirements (if any)
- [ ] Design error handling and logging strategy

### 0.3 Dependency Mapping
- [ ] Audio processing: `mp4ameta`, `metaflac`, `minimp3-rs`
- [ ] FFmpeg: `ffmpeg-sys` or `ez-ffmpeg`
- [ ] Image processing: `image` crate (artwork handling)
- [ ] Serialization: `serde` + `toml`/`json`
- [ ] UI framework: Tauri or Druid
- [ ] Logging: `tracing` or `log` + `env_logger`
- [ ] Async runtime: `tokio` (if async needed)
- [ ] Concurrency: `rayon` (for parallel batch processing)
- [ ] Command execution: `std::process::Command` or `anyhow` error handling
- [ ] Path handling: `camino` (better path types than std::path)

### 0.4 Development Environment Setup
- [ ] Create Rust project structure with Cargo
- [ ] Set up CI/CD pipeline (GitHub Actions)
- [ ] Configure cross-platform build targets
- [ ] Document build prerequisites (FFmpeg installation)
- [ ] Set up testing framework (`cargo test`)
- [ ] Document release procedures

---

## PHASE 1: Foundation & Core Architecture (Weeks 3-4)

### 1.1 Project Initialization
- [ ] Create main Rust project: `audio_book_converter_rs`
- [ ] Organize directory structure:
  ```
  audio_book_converter_rs/
  ├── src-tauri/                 # Tauri backend
  │   └── src/
  │       ├── lib.rs             # Main library
  │       ├── models.rs          # Data structures
  │       ├── audio/             # Audio processing modules
  │       ├── metadata/          # Metadata handling
  │       ├── ffmpeg/            # FFmpeg integration
  │       └── error.rs           # Error types
  ├── src-ui/                    # Tauri frontend (React/Vue)
  │   └── src/
  │       ├── components/
  │       ├── pages/
  │       └── services/
  ├── Cargo.toml                 # Dependency manifest
  └── tauri.conf.json            # Tauri configuration
  ```

### 1.2 Core Data Structures
- [ ] Define Rust types for main Java classes:
  - `AudioBook` → struct with title, chapters, metadata
  - `Chapter` → struct with start_time, end_time, metadata
  - `AudioFile` → struct with path, format, duration, bitrate
  - `Metadata` → struct with title, artist, album, cover art path
  - `ConversionSettings` → struct with bitrate, sample_rate, output_format
  - `ConversionJob` → struct with input files, progress, status
  - `ProcessingQueue` → struct managing batch operations

### 1.3 Error Handling Framework
- [ ] Create custom error type using `thiserror`
- [ ] Define error categories:
  - FileIO errors
  - FFmpeg execution errors
  - Metadata parsing errors
  - Audio format errors
  - Invalid input errors
- [ ] Implement error propagation with `?` operator
- [ ] Set up logging with `tracing`

### 1.4 Configuration & Defaults
- [ ] Define application configuration structure
- [ ] Set default conversion settings (bitrate, sample rate)
- [ ] Create user preferences storage mechanism
- [ ] Document supported audio formats
- [ ] Document supported metadata standards

---

## PHASE 2: Audio Processing Foundation (Weeks 5-6)

### 2.1 Audio Format Support
- [ ] MP3 format handling:
  - Audio stream reading with `minimp3-rs` or FFmpeg
  - Duration calculation
  - Bitrate detection
  - Sample rate detection
  
- [ ] M4A/AAC format handling:
  - Container parsing
  - Stream information extraction
  
- [ ] FLAC format handling:
  - Use `metaflac` for FLAC files
  - Extract metadata (title, artist, etc.)
  - Calculate duration
  
- [ ] WMA format handling:
  - FFmpeg-based parsing
  - Duration and bitrate detection

### 2.2 FFmpeg Integration
- [ ] Set up FFmpeg bindings:
  - Choose between `ffmpeg-sys` (low-level) or `ez-ffmpeg` (high-level)
  - Verify FFmpeg is installed in system
  - Create wrapper module for FFmpeg operations
  
- [ ] Implement core FFmpeg operations:
  - Audio format conversion (MP3 → AAC)
  - Bitrate configuration
  - Sample rate adjustment
  - Audio concatenation
  - Metadata embedding
  
- [ ] Create command execution abstraction:
  - Build FFmpeg commands programmatically
  - Execute as subprocesses using `std::process::Command`
  - Capture output and errors
  - Handle progress reporting

### 2.3 Audio Duration & Timing
- [ ] Implement duration calculation:
  - Extract from file metadata using FFmpeg
  - Handle variable bitrate files
  - Support different sample rates
  
- [ ] Implement time formatting utilities:
  - Convert seconds to HH:MM:SS
  - Parse user-entered time formats
  
- [ ] Implement file merging logic:
  - Calculate cumulative durations
  - Build chapter timings

### 2.4 Concurrency & Batch Processing
- [ ] Design parallel processing architecture using `rayon`:
  - Process multiple files in parallel
  - Configure thread pool size
  - Handle resource management
  
- [ ] Implement progress tracking:
  - Track conversion progress per file
  - Aggregate total progress
  - Implement cancellation mechanism
  
- [ ] Implement queue management:
  - Queue jobs for processing
  - Execute jobs in order
  - Skip/retry failed jobs

---

## PHASE 3: Metadata Handling (Weeks 7-8)

### 3.1 M4B Metadata Structure
- [ ] Study M4B/MP4 container format:
  - Understand atoms/boxes structure
  - Learn metadata atom types
  - Study chapter metadata formats
  
- [ ] Use `mp4ameta` crate for M4B operations:
  - Read existing metadata
  - Write new metadata
  - Update chapter information
  - Embed artwork
  
- [ ] Implement metadata extraction:
  - Extract from source audio files
  - Parse ID3 tags (MP3)
  - Parse Vorbis comments (FLAC)
  - Extract from MP4 atoms

### 3.2 Chapter Management
- [ ] Design chapter data structure:
  - Chapter number
  - Start time
  - End time
  - Title
  - Metadata
  
- [ ] Implement chapter operations:
  - Create chapters from file splits
  - Auto-detect chapters from filenames
  - Reorder chapters
  - Edit chapter titles/times
  
- [ ] Implement chapter format support:
  - QuickTime chapters (standard M4B)
  - Nero chapters (alternative format)
  - Convert between formats if needed
  
- [ ] Validate chapter logic:
  - Ensure no overlapping chapters
  - Validate start/end times
  - Handle edge cases (zero duration)

### 3.3 Artwork Handling
- [ ] Implement artwork detection:
  - Extract from source files
  - Scan directory for cover images (cover.jpg, folder.jpg, etc.)
  - Use `image` crate for format detection
  
- [ ] Implement artwork embedding:
  - Convert images to JPEG/PNG as needed
  - Embed in M4B metadata
  - Handle multiple artwork formats
  
- [ ] Implement artwork management:
  - Allow user to select artwork
  - Display preview
  - Remove artwork if desired

### 3.4 Metadata Mapping
- [ ] Implement smart metadata extraction:
  - Map "Album" to audiobook "Title"
  - Map "Artist" to audiobook "Author"
  - Extract duration and other info
  
- [ ] Implement metadata editing:
  - Allow bulk metadata updates
  - Support regex operations on filenames/metadata
  - Auto-populate from filename patterns
  
- [ ] Implement metadata validation:
  - Check for required fields
  - Validate metadata types
  - Handle special characters

---

## PHASE 4: GUI Implementation (Weeks 9-10)

### 4.1 Tauri Project Setup
- [ ] Initialize Tauri project:
  - Create frontend structure (React/Vue recommended)
  - Configure Tauri backend communication
  - Set up build configuration
  
- [ ] Define IPC commands (Rust ↔ Frontend):
  - `convert_audiobook` - start conversion
  - `get_file_metadata` - extract audio info
  - `update_metadata` - modify metadata
  - `add_chapter` - add chapter
  - `cancel_conversion` - stop current job
  - `get_conversion_settings` - fetch defaults
  - `save_conversion_settings` - persist user prefs

### 4.2 Frontend UI Components
- [ ] File input section:
  - Drag-and-drop file/folder area
  - File picker button
  - File list with metadata preview
  - Remove file button
  
- [ ] Metadata editor:
  - Title, author, narrator fields
  - Cover art display/upload
  - Chapter list with edit capabilities
  - Bulk metadata operations
  
- [ ] Conversion settings panel:
  - Bitrate selector (preset + custom)
  - Sample rate selector
  - Output format selector
  - Concurrent jobs configuration
  - Output directory picker
  
- [ ] Progress display:
  - Current file progress bar
  - Overall progress indicator
  - Time remaining estimate
  - Cancel button
  - Log/details view
  
- [ ] Settings/preferences:
  - Default bitrate
  - Default output directory
  - Auto-detect artwork
  - Theme selection (light/dark)

### 4.3 User Workflow Implementation
- [ ] Implement main workflow:
  1. Import files (drag-drop or picker)
  2. Review/edit metadata
  3. Manage chapters
  4. Configure conversion settings
  5. Start conversion
  6. Monitor progress
  7. Export completed M4B
  
- [ ] Implement chapter management UI:
  - Visual chapter editor
  - Chapter timing visualization
  - Auto-chapter detection
  - Drag-to-reorder
  
- [ ] Implement metadata preview:
  - Display extracted metadata
  - Show artwork preview
  - Display duration and file info

### 4.4 Communication Layer (IPC)
- [ ] Implement Tauri command handlers:
  - Define commands as Rust functions
  - Handle async operations
  - Return results as JSON
  - Implement error serialization
  
- [ ] Implement state management:
  - Store current job data
  - Persist user preferences
  - Manage conversion queue
  
- [ ] Implement event streaming:
  - Send progress updates to frontend
  - Send error notifications
  - Send completion events

---

## PHASE 5: Core Conversion Logic (Weeks 11-12)

### 5.1 Main Conversion Pipeline
- [ ] Design conversion flow:
  1. Validate input files
  2. Extract metadata from all files
  3. Create chapter list
  4. Prepare FFmpeg commands
  5. Execute conversion with progress tracking
  6. Embed metadata in output
  7. Embed chapters in output
  8. Embed artwork in output
  9. Verify output file
  10. Report completion
  
- [ ] Implement validation:
  - File existence checks
  - Format compatibility checks
  - Metadata validation
  - Output path validation

### 5.2 FFmpeg Command Building
- [ ] Implement audio encoding command:
  - Set input file
  - Configure bitrate
  - Set sample rate
  - Configure codec (AAC)
  - Set output format (M4B)
  - Handle special FFmpeg options
  
- [ ] Implement audio concatenation:
  - Build concat demuxer command
  - Handle multiple input files
  - Preserve audio quality
  
- [ ] Implement metadata embedding:
  - Construct ffmpeg metadata file
  - Embed title, artist, etc.
  - Use ffmpeg filters if needed

### 5.3 Progress Tracking
- [ ] Implement file-level progress:
  - Parse FFmpeg output
  - Extract current timestamp
  - Calculate progress percentage
  - Estimate time remaining
  
- [ ] Implement overall progress:
  - Track position in job queue
  - Aggregate per-file progress
  - Calculate total time estimate
  
- [ ] Implement progress reporting:
  - Send updates to frontend via Tauri event
  - Frequency: every 1-2 seconds
  - Include: current file, percentage, time remaining

### 5.4 Error Handling & Recovery
- [ ] Implement error detection:
  - FFmpeg command failures
  - File I/O errors
  - Metadata parsing errors
  - Cancellation handling
  
- [ ] Implement error logging:
  - Log FFmpeg stderr output
  - Log file operations
  - Include timestamps and context
  
- [ ] Implement retry logic:
  - Allow retry for failed files
  - Skip to next file option
  - Detailed error messages to user

---

## PHASE 6: Advanced Features (Weeks 13-14)

### 6.1 Audio Quality Optimization
- [ ] Implement audio normalization detection:
  - Analyze input file levels
  - Suggest normalization if needed
  
- [ ] Implement silence detection:
  - Optional silence removal/trimming
  - Configure sensitivity
  
- [ ] Implement audio enhancement:
  - Basic EQ options
  - Audio level adjustment
  - Optional dynamic range compression

### 6.2 Format-Specific Handling
- [ ] Implement MP3 chapter detection:
  - Parse ID3v2 CHAP frames
  - Convert to M4B chapters
  
- [ ] Implement FLAC chapter support:
  - Parse FLAC cue sheets
  - Extract chapter information
  
- [ ] Implement existing M4B handling:
  - Read existing chapters
  - Preserve or update metadata
  - Re-encode if needed

### 6.3 Batch Operations
- [ ] Implement batch mode:
  - Process multiple audiobooks
  - Queue management
  - Parallel execution (configurable)
  
- [ ] Implement batch settings:
  - Apply same settings to all files
  - Save batch presets
  - Dry-run mode

### 6.4 User Experience Enhancements
- [ ] Implement file browser:
  - Navigate and select input directory
  - Preview files in directory
  - Auto-import folder contents
  
- [ ] Implement save/load projects:
  - Save conversion project (JSON)
  - Load previous project
  - Resume interrupted jobs
  
- [ ] Implement keyboard shortcuts:
  - Common actions (convert, cancel, etc.)

---

## PHASE 7: Testing & Quality Assurance (Weeks 15)

### 7.1 Unit Testing
- [ ] Test audio format detection
- [ ] Test duration calculation accuracy
- [ ] Test metadata parsing and extraction
- [ ] Test chapter management logic
- [ ] Test error handling paths
- [ ] Test file I/O operations
- [ ] Test FFmpeg command generation
- [ ] Achieve >80% code coverage for core modules

### 7.2 Integration Testing
- [ ] Test full conversion pipeline:
  - MP3 → M4B with chapters
  - Multiple files → single M4B
  - FLAC → M4B with metadata
  - WMA → M4B
  
- [ ] Test metadata preservation:
  - Artwork survives conversion
  - Metadata fields preserved
  - Chapters correctly positioned
  
- [ ] Test user workflows:
  - Import → edit → convert cycle
  - Batch processing
  - Error recovery

### 7.3 Cross-Platform Testing
- [ ] Test on Windows (x86_64)
- [ ] Test on macOS (x86_64 and ARM64)
- [ ] Test on Linux (x86_64)
- [ ] Test file path handling on all platforms
- [ ] Test FFmpeg installation detection
- [ ] Test installer packaging

### 7.4 Performance Testing
- [ ] Benchmark conversion speed:
  - Compare with Java version
  - Goal: equal or faster
  
- [ ] Memory profiling:
  - Test with large files (500MB+)
  - Monitor memory usage during batch processing
  - Check for memory leaks
  
- [ ] UI responsiveness:
  - Ensure UI doesn't freeze during conversion
  - Progress updates responsive

---

## PHASE 8: Packaging & Distribution (Week 16)

### 8.1 Build Optimization
- [ ] Release build configuration:
  - Enable LTO (Link Time Optimization)
  - Strip debug symbols
  - Optimize for binary size
  
- [ ] Dependency verification:
  - FFmpeg minimum version
  - Platform-specific dependencies
  - License compliance check

### 8.2 Installer Creation
- [ ] Windows installer (NSIS or MSI)
  - Include FFmpeg dependency
  - Desktop shortcuts
  - Add to PATH (optional)
  - Uninstaller
  
- [ ] macOS package:
  - DMG with drag-and-drop install
  - Code signing (if available)
  - Notarization setup
  
- [ ] Linux packages:
  - AppImage for universal Linux
  - Snap package
  - Flatpak if applicable
  - Dependency specifications

### 8.3 Release Preparation
- [ ] Write release notes:
  - Feature list
  - Bug fixes
  - Known limitations
  - Breaking changes from Java version
  
- [ ] Documentation:
  - Installation guide
  - User manual
  - Troubleshooting guide
  - FAQ
  
- [ ] GitHub setup:
  - Create releases with binary downloads
  - Set up CI/CD for automatic builds
  - Document build process

---

## PHASE 9: Launch & Post-Launch (Week 16+)

### 9.1 Beta Testing
- [ ] Release beta version to community
- [ ] Gather feedback on:
  - Conversion quality
  - Metadata handling accuracy
  - UI usability
  - Performance
  - Cross-platform compatibility
  
- [ ] Fix critical issues
- [ ] Iterate on UX based on feedback

### 9.2 Documentation & Community
- [ ] Create GitHub wiki with:
  - Installation instructions per OS
  - Quick start guide
  - Advanced features
  - Troubleshooting
  
- [ ] Set up discussion forums or Discord
- [ ] Create video tutorials
- [ ] Document FFmpeg dependency installation

### 9.3 Maintenance Plan
- [ ] Monitor GitHub issues
- [ ] Create bug fix releases as needed
- [ ] Plan feature improvements:
  - MP3 splitting (parse audio and split by time)
  - Online metadata lookup
  - Batch rename with regex
  - Audio format detection improvements

---

## Technical Architecture Details

### Backend (Rust) Modules

```
Backend Structure:
├── main.rs / lib.rs
│   └── Command handlers for Tauri IPC
├── models/
│   ├── audiobook.rs        # AudioBook, Chapter structures
│   ├── settings.rs         # User preferences
│   └── job.rs              # Conversion job state
├── audio/
│   ├── format.rs           # Format detection
│   ├── duration.rs         # Duration calculation
│   └── processor.rs        # High-level audio operations
├── metadata/
│   ├── extractor.rs        # Extract from source files
│   ├── m4b.rs              # M4B-specific metadata
│   ├── id3.rs              # MP3 ID3 tags
│   ├── flac.rs             # FLAC metadata
│   └── artwork.rs          # Cover art handling
├── ffmpeg/
│   ├── command.rs          # Command builder
│   ├── executor.rs         # Command execution
│   ├── parser.rs           # Output parsing
│   └── progress.rs         # Progress tracking
├── conversion/
│   ├── pipeline.rs         # Main conversion flow
│   ├── validator.rs        # Input validation
│   ├── queue.rs            # Job queue management
│   └── worker.rs           # Parallel worker thread
├── error.rs                # Error types
├── config.rs               # Configuration
└── logging.rs              # Logging setup
```

### Frontend (React/Vue) Structure

```
Frontend Structure:
├── components/
│   ├── FileImport/         # Drag-drop, file picker
│   ├── MetadataEditor/     # Title, artist, etc.
│   ├── ChapterManager/     # Chapter editing UI
│   ├── SettingsPanel/      # Bitrate, sample rate, etc.
│   ├── ProgressDisplay/    # Conversion progress
│   └── AppLayout/          # Main layout
├── pages/
│   ├── ConvertPage/        # Main conversion page
│   └── SettingsPage/       # User preferences
├── services/
│   ├── tauri.ts            # IPC communication
│   ├── audio.ts            # Audio operations
│   └── state.ts            # State management
├── types/
│   └── index.ts            # TypeScript definitions
└── App.tsx                 # Main component
```

### Data Flow

```
User Action (UI)
    ↓
Tauri IPC Command
    ↓
Backend Command Handler (Rust)
    ↓
Audio Processing/Metadata Operations
    ↓
FFmpeg Execution
    ↓
Progress Events (streaming back to UI)
    ↓
File Written to Disk
    ↓
Completion Event to UI
    ↓
Update UI with Results
```

---

## Dependency List (Cargo.toml)

### Core Dependencies
- `tauri` - Desktop application framework
- `tokio` - Async runtime
- `serde` + `serde_json` - Serialization
- `thiserror` - Error handling
- `tracing` + `tracing-subscriber` - Logging

### Audio/FFmpeg
- `ffmpeg-sys` or `ez-ffmpeg` - FFmpeg bindings
- `mp4ameta` - M4B metadata handling
- `metaflac` - FLAC metadata
- `minimp3-rs` - MP3 reading (optional)

### File/Path Handling
- `camino` - Better path types
- `walkdir` - Directory traversal
- `anyhow` - Error handling

### Concurrency & Performance
- `rayon` - Data parallelism
- `parking_lot` - Better synchronization primitives

### Image Processing
- `image` - Image format support

### Testing
- `proptest` - Property-based testing
- `tempfile` - Temporary files for tests

---

## Migration Considerations

### What Changes Between Versions

| Aspect | Java Version | Rust Version |
|--------|-------------|--------------|
| UI Framework | JavaFX/Swing | Tauri + Web (React/Vue) |
| Language | Java | Rust |
| Binary Size | Larger (~80MB with JVM) | Smaller (~20-30MB) |
| Memory Usage | Higher (JVM overhead) | Lower (no GC) |
| Startup Time | Slower (~2-3 sec) | Faster (~0.5-1 sec) |
| Performance | Good | Excellent |
| User Learning | Similar feature set | Identical (goal) |
| Deployment | Java + FFmpeg | Native + FFmpeg |

### Backward Compatibility

- Project files (.json format) compatible between versions
- Conversion settings portable
- M4B output format identical
- Metadata handling identical (QuickTime chapters standard)

---

## Risk Mitigation

### Identified Risks

| Risk | Impact | Mitigation |
|------|--------|-----------|
| FFmpeg integration complexity | High | Start with `ez-ffmpeg` wrapper, fallback to `ffmpeg-sys` |
| GUI development delay | High | Use pre-built Tauri + React template |
| M4B metadata issues | Medium | Test extensively, use well-tested `mp4ameta` crate |
| Performance regression | Medium | Benchmark early and often against Java version |
| Cross-platform issues | Medium | Use CI/CD to build all platforms automatically |
| User adoption | Low | Feature parity ensures existing users transition smoothly |

### Testing Strategy

- Extensive unit tests for audio/metadata modules
- Integration tests for full conversion pipeline
- Regression tests comparing output with Java version
- Platform-specific testing on CI/CD
- Community beta testing before release

---

## Success Criteria

- ✅ Full feature parity with Java 6.5.2
- ✅ Conversion speed equal or faster than Java version
- ✅ Output M4B files identical to Java version
- ✅ UI equally intuitive or better
- ✅ Memory usage lower than Java version
- ✅ Startup time faster than Java version
- ✅ Works on Windows, macOS, Linux
- ✅ All existing projects/settings importable
- ✅ Zero regressions from Java version
- ✅ >80% code test coverage

---

## Timeline Summary

```
Week 1-2:   Analysis & Planning
Week 3-4:   Foundation & Architecture
Week 5-6:   Audio Processing
Week 7-8:   Metadata Handling
Week 9-10:  GUI Implementation
Week 11-12: Core Conversion Logic
Week 13-14: Advanced Features
Week 15:    Testing & QA
Week 16:    Packaging & Release
Week 16+:   Beta, Launch, Maintenance
```

**Total: 12-16 weeks to production-ready release**

---

## Resources & References

### Documentation Links
- FFmpeg: https://ffmpeg.org/documentation.html
- Tauri: https://tauri.app/
- Rust Audio: https://rust-audio.org/
- MP4/M4B Format: https://en.wikipedia.org/wiki/MPEG-4_Part_12

### Rust Crates Used
- https://docs.rs/tauri/
- https://docs.rs/mp4ameta/
- https://docs.rs/metaflac/
- https://docs.rs/tokio/
- https://docs.rs/serde/

### Similar Projects
- Existing: AudioBookConverter (Java) - https://github.com/yermak/AudioBookConverter
- m4b-tool - https://github.com/sandreas/m4b-tool (shell scripts)
- ChapterForge - Similar feature set

---

## Maintenance Notes

After launch, plan for:

1. **Monthly updates** - Bug fixes and minor features
2. **Quarterly releases** - Major improvements
3. **Community engagement** - Monitor GitHub issues
4. **Dependency maintenance** - Update Rust crates quarterly
5. **FFmpeg compatibility** - Test with new FFmpeg versions
6. **User feedback** - Gather requirements for next version

---

## Questions for Clarification (If Needed)

Before starting implementation:

1. Priority: UI polish vs. feature completeness?
2. Minimum supported OS versions? (Windows 10+, macOS 10.13+, etc.)
3. Budget for code signing/notarization?
4. Community beta testing duration?
5. Support for other audio formats beyond MP3, M4A, FLAC, WMA?
6. Preference for Tauri + React vs. alternative stacks?
