# CLAUDE.md - AudioBookConverter Rust Implementation Guide

**For: Claude AI (or other LLM assistants)**
**Project**: AudioBookConverter - Java → Rust Rewrite
**Version**: 1.0
**Last Updated**: December 17, 2025

---

## Purpose

This document guides Claude (or other LLM assistants) through implementing the AudioBookConverter Rust project systematically. Use this alongside `implementation_tasks.md` for the complete task list.

---

## Project Overview

**What**: Rewrite AudioBookConverter from Java to Rust
**Why**: Better performance, lower memory, modern language
**Technologies**: Rust + Tauri + Symphonia + FFmpeg + mp4ameta
**Scope**: Full feature parity with Java v6.5.2
**Duration**: 12-16 weeks (9 phases)
**Output**: Cross-platform desktop app (Windows, macOS, Linux)

---

## Key Principles for Implementation

### 1. **Symphonia First for Audio Loading**
- Use Symphonia for ALL audio reading (MP3, FLAC, M4A, WAV, OGG)
- Symphonia handles: format detection, decoding, metadata extraction, artwork
- **Do NOT** use individual libraries like minimp3-rs + metaflac
- Symphonia is pure Rust, well-tested (7000+ projects), performs well

### 2. **FFmpeg Only for Encoding**
- Use FFmpeg ONLY for: encoding to AAC, concatenating files, audio processing
- Build FFmpeg commands programmatically using `ffmpeg-sys`
- Execute via `std::process::Command` (not direct FFmpeg API unless necessary)
- Parse stderr output for progress tracking

### 3. **mp4ameta for M4B Metadata Writing**
- Symphonia reads metadata
- mp4ameta writes metadata to M4B files
- Separation of concerns: read (Symphonia) vs write (mp4ameta)

### 4. **Tauri for GUI**
- Backend: Rust with IPC commands
- Frontend: React or Vue (HTML/CSS/JavaScript)
- Communication: Tauri commands + events
- Modern, lightweight, cross-platform alternative to Electron

### 5. **Error Handling as First-Class**
- Create custom error types with `thiserror` early (Phase 1)
- Use `Result<T, AppError>` everywhere
- Convert external errors (Symphonia, FFmpeg) to app errors
- Provide user-friendly error messages

### 6. **Testing Throughout**
- Write tests as you implement, not after
- Unit tests for each module
- Integration tests for full pipelines
- Target >80% code coverage for core modules

### 7. **Cross-Platform from Day One**
- Use CI/CD to build on Windows, macOS, Linux
- Handle path differences (backslashes vs forward slashes)
- Test FFmpeg detection on all platforms
- Use `camino` for better path types than `std::path`

---

## How to Use This Project

### For Individual Phases

**Pattern**:
1. Read phase overview from `implementation_tasks.md`
2. Read this document's guidance for that phase
3. Implement each task in the checklist
4. Run tests for that phase
5. Commit to feature branch
6. Move to next phase

### For Specific Modules

**When implementing audio module**:
1. Check "Audio Module Guidance" section below
2. Follow the recommended file structure
3. Use Symphonia as primary tool
4. Write tests for each function
5. Ensure error handling is comprehensive

### For Troubleshooting

1. Check "Common Issues" section
2. Check error handling patterns
3. Review related phase guidance
4. Look at dependency documentation links

---

## Phase-Specific Guidance

### PHASE 0: Setup (Weeks 1-2)

**Your Job**: Create the skeleton and understand what needs to be built

**Key Tasks**:
- [ ] Create GitHub repo
- [ ] Set up Cargo workspace
- [ ] Create directory structure
- [ ] Set up CI/CD pipeline
- [ ] Write POCs for Symphonia, FFmpeg, Tauri

**Guidance**:
- Don't skip POCs—they validate your approach
- CI/CD now saves debugging headaches later
- Document everything as you discover it
- Create `DEPENDENCIES.md` explaining each crate

**Critical Success**: GitHub repo exists with working CI/CD build

---

### PHASE 1: Foundation (Weeks 3-4)

**Your Job**: Build the core architecture and data structures

**Key Modules**:
- `error.rs` - Custom error types using `thiserror`
- `config.rs` - User settings and defaults
- `models.rs` - All data structures (AudioBook, Chapter, Metadata, etc.)
- `state.rs` - App state management

**Guidance**:
- Make error types comprehensive—this pays dividends later
- Use Serde derive on all structs (for IPC + persistence)
- Implement proper Default implementations
- Add validation methods to models

**Error Handling Pattern**:
```rust
// Define errors
#[derive(thiserror::Error, Debug)]
pub enum AppError {
    #[error("Audio file not found: {0}")]
    FileNotFound(String),
    #[error("Unsupported format: {0}")]
    UnsupportedFormat(String),
    #[error("FFmpeg error: {0}")]
    FFmpegError(String),
}

// Use in functions
fn load_file(path: &Path) -> Result<AudioFile, AppError> {
    // ...
}
```

**Critical Success**: 
- [ ] `cargo test` passes for Phase 1
- [ ] All models serialize/deserialize correctly
- [ ] Error types cover all failure modes

---

### PHASE 2: Audio Processing with Symphonia (Weeks 5-6)

**Your Job**: Integrate Symphonia for all audio loading

**Key Modules**:
- `audio/loader.rs` - Symphonia integration, auto-detection
- `audio/format.rs` - Format validation
- `audio/timing.rs` - Duration calculations
- `audio/symphonia_metadata.rs` - Metadata extraction
- `audio/batch.rs` - Loading multiple files
- `ffmpeg/command_builder.rs` - Build FFmpeg commands
- `ffmpeg/detection.rs` - Detect FFmpeg on system

**Symphonia Usage**:
```rust
// Detect format automatically
use symphonia::default;
let source = std::fs::File::open(path)?;
let mss = MediaSourceStream::new(Box::new(source), Default::default());
let format = default::get_probe().format(&mss)?;

// Extract metadata
let metadata = format.metadata().current();
let title = metadata.general().title().next();

// Get duration
let total_duration_ms = calculate_duration(&format);
```

**Key Points**:
- Symphonia::Probe auto-detects format (don't rely on extension)
- symphonia-metadata gives unified metadata API
- Handle missing metadata gracefully (Option types)
- Cache format detection results

**FFmpeg Detection Pattern**:
```rust
// Check PATH, common locations, user config
// Verify executable works: ffmpeg -version
// Cache result to avoid repeated checks
// Return helpful error if not found
```

**Critical Success**:
- [ ] Load MP3, FLAC, M4A, WAV successfully
- [ ] Extract metadata from each format
- [ ] Extract artwork from each format
- [ ] FFmpeg detected on system
- [ ] All tests passing

---

### PHASE 3: Metadata & Chapters (Weeks 7-8)

**Your Job**: Handle metadata reading/writing and chapter management

**Key Modules**:
- `metadata/extraction.rs` - Extract from formats (Symphonia does most)
- `metadata/artwork.rs` - Detect, extract, embed artwork
- `metadata/chapters.rs` - Chapter creation, validation, editing
- `metadata/writing.rs` - Write metadata to M4B (mp4ameta)
- `metadata/editor.rs` - In-memory editing API

**Using mp4ameta for Writing**:
```rust
use mp4ameta::Tag;

let mut tag = Tag::read_from_path("file.m4b")?;
tag.set_title("My Audiobook");
tag.set_artist("Author Name");
tag.write_to_path("file.m4b")?;
```

**Chapter Handling**:
- Create from files (each file = chapter)
- Auto-split by count (divide duration evenly)
- Extract from existing M4B (if present)
- Validate (no overlaps, within duration)
- Edit (add, remove, reorder)

**Key Points**:
- Metadata extraction = mostly Symphonia (unify outputs)
- Metadata writing = mp4ameta (write M4B atoms)
- Chapters = QuickTime format (standard M4B chapters)
- Artwork = embed into M4B with metadata

**Critical Success**:
- [ ] Read metadata from all formats
- [ ] Write metadata to M4B file
- [ ] Embed artwork in M4B
- [ ] Create chapters from files
- [ ] Edit chapters in memory
- [ ] Write chapters to M4B

---

### PHASE 4: GUI Backend (Weeks 9-10)

**Your Job**: Create Tauri IPC commands connecting frontend to backend

**Key Modules**:
- `commands/audiobook.rs` - Load/create audiobooks
- `commands/metadata.rs` - Edit metadata/chapters
- `commands/files.rs` - File dialogs
- `commands/settings.rs` - User settings
- `commands/conversion.rs` - Start/cancel conversions
- `events.rs` - Emit progress/completion events

**Tauri Command Pattern**:
```rust
#[tauri::command]
pub async fn load_files(paths: Vec<String>) -> Result<AudioBook, String> {
    // Convert Vec<String> to Vec<PathBuf>
    // Load with Symphonia
    // Return AudioBook or error message
    // Errors: must convert to String for JSON
}

// Main.rs: register command
.invoke_handler(tauri::generate_handler![load_files])
```

**Event Emission Pattern**:
```rust
// Backend spawns conversion thread
// Thread updates progress, emits events
window.emit("conversion_progress", ConversionProgress {
    job_id: "job-123",
    current_file: "file3.mp3",
    file_progress: 75.0,
    total_progress: 50.0,
    time_remaining_sec: 120,
})?;
```

**Key Points**:
- IPC commands must return JSON-serializable types
- Use `Result<T, String>` for errors (serialize to JSON)
- Events stream from backend to frontend in real-time
- State persists in `AppState` (thread-safe with Arc<Mutex>)

**Critical Success**:
- [ ] All IPC commands work
- [ ] State persists correctly
- [ ] Events emit to frontend
- [ ] Integration tests pass

---

### PHASE 5: Conversion Pipeline (Weeks 11-12)

**Your Job**: Implement the complete conversion workflow

**Key Modules**:
- `ffmpeg/executor.rs` - Run FFmpeg commands
- `conversion/progress.rs` - Track progress
- `conversion/queue.rs` - Manage job queue
- `conversion/worker.rs` - Process jobs in parallel
- `conversion/pipeline.rs` - Main workflow
- `conversion/recovery.rs` - Error recovery
- `conversion/temp.rs` - Temporary file cleanup

**Conversion Pipeline Flow**:
```
Input Files (MP3, FLAC, etc.)
    ↓
1. Validate inputs
    - Files exist
    - Output path writable
    - Metadata valid
    ↓
2. Load with Symphonia
    - Get duration
    - Extract metadata
    - Extract artwork
    ↓
3. Execute FFmpeg
    - Build convert command
    - Parse progress in background thread
    - Emit progress events
    ↓
4. Embed metadata (mp4ameta)
    - Write title, artist, etc.
    ↓
5. Embed artwork
    - Resize if needed
    - Embed into M4B
    ↓
6. Embed chapters
    - Write QuickTime chapters
    ↓
7. Verify output
    - File exists
    - Playable with Symphonia
    ↓
8. Finalize
    - Move to final location
    - Delete temp files
    - Emit completion
    ↓
Output M4B ✓
```

**FFmpeg Execution**:
```rust
// Build command
let cmd = FFmpegCommand::builder()
    .input("input.mp3")
    .audio_codec("aac")
    .bitrate(128)
    .output("output.m4b")
    .build();

// Execute with progress tracking
execute_ffmpeg_with_progress(cmd, |progress| {
    emit_progress_event(progress);
})?;
```

**Progress Parsing**:
```rust
// FFmpeg outputs to stderr:
// frame=12345 time=00:15:30.45 speed=2.5x ...
// Extract time=HH:MM:SS.ms
// Calculate percentage: current_ms / total_ms * 100
```

**Parallel Processing**:
```rust
// Use rayon for parallel file processing
use rayon::prelude::*;

files.par_iter().map(|file| {
    convert_file(file)
}).collect::<Result<_, _>>()?;
```

**Error Recovery**:
- Retry failed conversions
- Skip problematic files
- Rollback partial conversions
- Clean up temp files on error

**Critical Success**:
- [ ] Single file conversion works
- [ ] Multiple files → single M4B works
- [ ] Metadata preserved
- [ ] Artwork embedded
- [ ] Chapters correct
- [ ] Progress tracking accurate
- [ ] Error handling robust

---

### PHASE 6: Advanced Features (Weeks 13-14)

**Your Job**: Polish and add nice-to-have features

**Optional But Valuable**:
- Audio normalization detection
- Batch processing presets
- Project save/load
- Keyboard shortcuts
- Undo/redo
- Performance optimizations

**Guidance**:
- Focus on what's most requested by users
- Don't over-engineer—keep it simple
- Test thoroughly before release
- Document new features

---

### PHASE 7: Testing & QA (Week 15)

**Your Job**: Comprehensive testing across all modules

**Test Coverage Targets**:
- Unit tests: >80% for core modules
- Integration tests: Full pipeline scenarios
- Cross-platform: Windows, macOS, Linux
- Performance: Benchmark vs Java version

**Test Organization**:
```
tests/
├── unit/
│   ├── audio.rs
│   ├── metadata.rs
│   ├── ffmpeg.rs
│   └── conversion.rs
├── integration/
│   ├── full_pipeline.rs
│   └── tauri_commands.rs
└── fixtures/
    ├── sample.mp3
    ├── sample.flac
    └── sample.m4a
```

**Running Tests**:
```bash
cargo test                          # Run all tests
cargo test --lib                    # Unit tests only
cargo test --test '*'               # Integration tests
cargo tarpaulin --out Html          # Coverage report
```

---

### PHASE 8: Packaging (Week 16)

**Your Job**: Create installers for all platforms

**Status**: ✅ **AUTOMATED** - GitHub Actions workflow configured

**Automated Release Pipeline**:
The project now has an automated GitHub Actions release workflow (`.github/workflows/release.yml`) that:
- Triggers on version tags (e.g., `v1.0.0`)
- Builds installers for all platforms in parallel:
  - **Windows**: NSIS `.exe` installer
  - **macOS**: `.dmg` installer
  - **Linux**: `.deb` package (Ubuntu/Debian)
- Creates a GitHub Release with all installers attached
- Applies LTO and optimization settings from `Cargo.toml` profile

**How to Create a Release**:
1. Update version in `Cargo.toml` and `src-tauri/tauri.conf.json`
2. Commit changes: `git commit -m "Bump version to X.Y.Z"`
3. Create and push tag: `git tag vX.Y.Z && git push origin main && git push origin vX.Y.Z`
4. GitHub Actions automatically builds and publishes the release

**For detailed deployment instructions, see [DEPLOY.md](DEPLOY.md)**

**Key Tasks**:
- [x] Optimize release build (LTO) - configured in `Cargo.toml`
- [x] Create platform-specific installers - automated via GitHub Actions
- [ ] Test on clean system - download from GitHub Releases
- [ ] Bundle FFmpeg appropriately - user must have FFmpeg installed (documented in README)
- [x] Generate app icons (.icns, .ico) - completed via `tauri icon` command

---

### PHASE 9: Launch & Maintenance (Week 16+)

**Your Job**: Get it to users and keep it running

**Beta Testing**: Gather real-world feedback
**Official Release**: Announce on communities
**Maintenance**: Bug fixes, dependency updates
**Community**: Respond to issues, plan next features

---

## Module Implementation Checklist

### Audio Module Pattern

When implementing a new audio function:
```rust
// 1. Define what it does
pub fn detect_format(path: &Path) -> Result<AudioFormat, AppError> {
    // 2. Validate input
    if !path.exists() {
        return Err(AppError::FileNotFound(path.to_string()));
    }
    
    // 3. Use Symphonia
    let file = std::fs::File::open(path)?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());
    
    // 4. Handle result
    match default::get_probe().format(&mss) {
        Ok(probed) => Ok(detect_from_format(&probed)),
        Err(e) => Err(AppError::from(e)),
    }
    
    // 5. Add tests
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_detect_mp3() { /* ... */ }
    #[test]
    fn test_detect_flac() { /* ... */ }
    #[test]
    fn test_invalid_file() { /* ... */ }
}
```

### Tauri Command Pattern

When adding a new IPC command:
```rust
// 1. Define command signature
#[tauri::command]
pub async fn my_command(arg: String) -> Result<OutputType, String> {
    // 2. Validate input
    if arg.is_empty() {
        return Err("Argument required".to_string());
    }
    
    // 3. Call backend logic
    my_backend_function(&arg)
        .map_err(|e| e.to_string())  // Convert to JSON-serializable
    
    // 4. Add integration test
}

// In main.rs:
.invoke_handler(tauri::generate_handler![my_command])
```

### Error Handling Pattern

When adding error variants:
```rust
#[derive(thiserror::Error, Debug)]
pub enum AppError {
    #[error("File not found: {0}")]
    FileNotFound(String),
    
    #[error("Invalid format: {0}")]
    InvalidFormat(String),
    
    #[error("Symphonia error: {0}")]
    SymphoniaError(#[from] symphonia::Error),
}

// Convert external errors
impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        AppError::IoError(e)
    }
}
```

---

## Common Issues & Solutions

### Issue 1: "Symphonia can't find format"
**Cause**: Wrong feature flags or corrupted file
**Solution**:
- Verify feature flags in Cargo.toml: `features = ["all"]` or specific ones
- Test file with FFmpeg: `ffmpeg -i file.mp3 -f null -` 
- Check magic bytes if extension wrong

### Issue 2: "FFmpeg not found"
**Cause**: Not in PATH or not installed
**Solution**:
- Install FFmpeg: https://ffmpeg.org/download.html
- Add to PATH on Windows
- Use Homebrew on macOS: `brew install ffmpeg`
- Use package manager on Linux

### Issue 3: "Progress not updating"
**Cause**: FFmpeg output parsing not working
**Solution**:
- Check FFmpeg outputs to stderr, not stdout
- Test parsing: `ffmpeg -i input.mp3 -aac -b:a 128k output.m4b 2>&1 | grep time=`
- Verify regex pattern matches output format

### Issue 4: "Metadata not embedding"
**Cause**: mp4ameta write failing or wrong M4B file
**Solution**:
- Verify M4B is valid: `ffprobe output.m4b`
- Test mp4ameta separately: `mp4ameta --help`
- Check temp file cleanup isn't deleting M4B too early

### Issue 5: "Tests failing on Linux"
**Cause**: Path handling, FFmpeg path, permissions
**Solution**:
- Use `camino::Utf8Path` instead of `std::path::Path`
- Don't hardcode paths
- Use CI/CD environment variables for test setup

### Issue 6: "UI freezes during conversion"
**Cause**: Blocking operation on main thread
**Solution**:
- Spawn conversion in separate thread
- Use channels for communication
- Emit events from worker thread, not main thread
- Ensure all I/O is non-blocking

---

## Testing Strategies

### Unit Test Pattern
```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_happy_path() {
        let result = my_function(valid_input);
        assert!(result.is_ok());
    }
    
    #[test]
    fn test_error_case() {
        let result = my_function(invalid_input);
        assert!(result.is_err());
    }
    
    #[test]
    #[ignore]  // Only run with: cargo test -- --ignored
    fn test_slow_operation() {
        // FFmpeg encoding, etc.
    }
}
```

### Integration Test Pattern
```rust
#[test]
fn test_full_conversion() {
    // 1. Set up test files
    let input = PathBuf::from("tests/fixtures/sample.mp3");
    let output = PathBuf::from("tests/output.m4b");
    
    // 2. Run conversion
    let result = convert_audio(&input, &output);
    
    // 3. Verify output
    assert!(result.is_ok());
    assert!(output.exists());
    
    // 4. Verify metadata
    let audiobook = load_audiobook(&output).unwrap();
    assert_eq!(audiobook.title, "Test Title");
    
    // 5. Clean up
    std::fs::remove_file(output).ok();
}
```

---

## Code Style & Conventions

### Naming
- `fn_name()` for functions
- `StructName` for structs
- `CONSTANT_NAME` for constants
- `module_name` for modules

### Error Messages
- Use lowercase for start: `"file not found"`
- Include context: `"file not found: /path/to/file"`
- Don't repeat error type: ❌ `"Error: file not found"` ✅ `"file not found"`

### Comments
- Explain WHY, not WHAT
- Good: `// Symphonia requires consuming all frames before closing`
- Bad: `// Get duration`

### Module Organization
```rust
// 1. Imports
use crate::models::*;
use symphonia::*;

// 2. Type definitions
pub type MyResult<T> = Result<T, AppError>;

// 3. Public API
pub fn public_function() { /* ... */ }

// 4. Private helpers
fn private_helper() { /* ... */ }

// 5. Tests
#[cfg(test)]
mod tests { /* ... */ }
```

---

## Performance Considerations

### Audio Loading
- Symphonia is fast (±15% of FFmpeg)
- Cache format detection
- Load only needed metadata
- Don't decode entire file unless necessary

### Conversion
- FFmpeg is highly optimized
- Parallel processing (rayon) for batch files
- Progress tracking in separate thread (non-blocking)
- Stream output to avoid memory buildup

### Memory
- Target: <500MB peak during conversion
- Don't load entire audio into memory
- Stream processing where possible
- Clean up temp files immediately

### Startup
- Target: <1 second startup
- Lazy-load components
- Cache config on disk
- Avoid heavy initialization

---

## Documentation Requirements

### For Each Module
- [ ] Module-level doc comment explaining purpose
- [ ] Public function doc comments with examples
- [ ] Error types documented
- [ ] Complex logic explained in comments

### For Each Phase
- [ ] README for phase goals
- [ ] Architecture decisions documented
- [ ] Known limitations noted
- [ ] Next phase dependencies identified

### Before Release
- [ ] User guide complete
- [ ] Troubleshooting guide complete
- [ ] API documentation complete
- [ ] Contributing guide written

---

## Key Dependencies Explained

### Symphonia
- **Purpose**: Audio format demuxing and decoding
- **Why**: Pure Rust, well-tested, comprehensive format support
- **Usage**: Load any audio format, extract metadata
- **Docs**: https://docs.rs/symphonia/

### FFmpeg (ffmpeg-sys)
- **Purpose**: Audio encoding to AAC/M4B
- **Why**: Industry standard, best quality, optimized
- **Usage**: Execute as subprocess, not direct API
- **Docs**: https://ffmpeg.org/

### mp4ameta
- **Purpose**: Read/write MP4 metadata atoms (M4B metadata)
- **Why**: Specialized for MP4 container, pure Rust
- **Usage**: Write metadata after FFmpeg creates M4B
- **Docs**: https://docs.rs/mp4ameta/

### Tauri
- **Purpose**: Desktop application framework
- **Why**: Modern, lightweight, cross-platform, Rust backend
- **Usage**: Create GUI, handle IPC, emit events
- **Docs**: https://tauri.app/

### Tokio
- **Purpose**: Async runtime
- **Why**: Battle-tested, standard for async in Rust
- **Usage**: Async IPC commands, background tasks
- **Docs**: https://tokio.rs/

### Serde
- **Purpose**: Serialization/deserialization
- **Why**: Standard, comprehensive, derive macros
- **Usage**: JSON IPC, config storage, project files
- **Docs**: https://serde.rs/

### Thiserror
- **Purpose**: Custom error types
- **Why**: Minimal boilerplate, automatic Display impl
- **Usage**: Define AppError variants
- **Docs**: https://docs.rs/thiserror/

---

## Debugging Tips

### Enable Verbose Logging
```rust
std::env::set_var("RUST_LOG", "debug");
tracing_subscriber::fmt::init();

tracing::debug!("This will print in debug mode");
tracing::info!("This always prints (if INFO level)");
tracing::warn!("Warning message");
tracing::error!("Error message");
```

### Test Single Phase
```bash
# Test only Phase 2 audio module
cargo test audio::

# Test only metadata module
cargo test metadata::

# Run specific test
cargo test test_detect_format
```

### Generate Detailed Backtrace
```bash
# Run with full backtrace
RUST_BACKTRACE=full cargo run

# Or for tests
RUST_BACKTRACE=full cargo test -- --nocapture
```

### Profile Binary
```bash
# Generate flamegraph (install: cargo install flamegraph)
cargo flamegraph --bin audiobookconverter

# Memory profiling on macOS
instruments -t "Allocations" ./target/debug/audiobookconverter
```

---

## When Stuck

1. **Check the task description** in `implementation_tasks.md`
2. **Review phase guidance** in this document
3. **Look at dependency docs**: Symphonia, FFmpeg, Tauri, etc.
4. **Search GitHub issues** in related projects
5. **Ask on r/rust** with minimal reproducible example
6. **Check CI/CD logs** if cross-platform issue

---

## Pre-Release Checklist

Before moving forward:
- [ ] All Phase tests passing
- [ ] Code coverage >80%
- [ ] No compiler warnings
- [ ] Documentation complete
- [ ] Cross-platform tested (if applicable)
- [ ] Performance benchmarked
- [ ] No memory leaks detected
- [ ] Error messages user-friendly
- [ ] Git history clean
- [ ] Commit messages descriptive

---

## Questions This Document Answers

**Q: Should I use Symphonia or minimp3-rs?**
A: Symphonia. It's unified, well-tested, handles all formats, and includes metadata extraction.

**Q: How do I handle FFmpeg not being installed?**
A: Detect early (Phase 0 POC), check PATH and common locations, provide installation instructions.

**Q: Should I decode entire files or stream?**
A: Stream where possible. For audio conversion, FFmpeg handles streaming internally.

**Q: How do I test Tauri commands?**
A: Use integration tests that call the command function directly (not through IPC).

**Q: What if Symphonia doesn't support a format?**
A: Fall back to FFmpeg for initial analysis, but aim to use Symphonia for everything possible.

**Q: When should I emit Tauri events?**
A: In worker threads during long operations. Main thread receives via channel, forwards to UI.

---

## Success Indicators

### Per Phase
- [ ] All tasks checked off
- [ ] Tests passing
- [ ] Documentation updated
- [ ] No compiler warnings
- [ ] Commit message clear

### Overall Project
- [ ] Binary <50MB
- [ ] Startup <1 second
- [ ] Conversion speed ≥ Java version
- [ ] Memory <500MB peak
- [ ] Crash-free operation
- [ ] All platforms supported
- [ ] 100% feature parity

---

## Next Steps After This Document

1. Read `implementation_tasks.md` completely
2. Start with Phase 0 setup
3. Complete each task in checklist
4. Run tests after each phase
5. Commit regularly with clear messages
6. Update this document as you learn
7. Document any deviations from plan
8. Track time spent per phase
9. Note any issues encountered
10. Update roadmap based on learnings

---

## Contact / Questions

This project is designed to be self-sufficient, but if you need clarification:

1. Check `implementation_tasks.md` for task details
2. Check relevant dependency documentation
3. Search for similar implementations on GitHub
4. Post on r/rust with specific error/question
5. Check phase guidance in this document

---

## Patterns Learned During Development

### Tauri 2.x API Changes

**Event Emission from Background Threads**:
- Tauri 2.x requires importing `Emitter` trait to use `emit()` on `AppHandle`
- Use `handle.emit("event_name", payload)` after importing `tauri::Emitter`
- The old `emit_all()` method no longer exists

```rust
use tauri::{AppHandle, Emitter, Manager};

// Correct way to emit events in Tauri 2.x
if let Err(e) = handle.emit("conversion_progress", event) {
    error!("Failed to emit event: {}", e);
}
```

**State Management in Commands**:
- Cannot construct `State<T>` directly - it's a wrapper with private fields
- Use `State<'_, T>` as parameter, Tauri injects it automatically
- For sharing state between commands and background tasks, use `Arc<T>`

### parking_lot vs std Synchronization

**Condvar Usage**:
- `parking_lot::Condvar::wait_for()` takes `&mut MutexGuard`, not `&MutexGuard`
- Must declare guard as mutable: `let mut guard = lock.lock();`
- Different API than `std::sync::Condvar`

```rust
use parking_lot::{Condvar, Mutex};

let (lock, cvar) = &*work_available;
let mut guard = lock.lock();  // Note: mut required
let _result = cvar.wait_for(&mut guard, Duration::from_secs(5));
```

### Symphonia Metadata Lives in Two Places

Tags can surface on the format reader **or** on the probe result, depending on
the container. MP4/FLAC use the reader; an MP3's ID3v2 block sits ahead of the
stream and lands on `probed.metadata`. Reading only one source silently loses
every tag, and the code then falls back to the filename stem — so it looks like
it works.

```rust
let mut probed = get_probe().format(&hint, mss, &fmt_opts, &meta_opts)?;
let mut reader = probed.format; // partial move; probed.metadata stays usable

if let Some(rev) = reader.metadata().current() { extract_from_revision(&mut m, rev); }
if let Some(pm) = probed.metadata.get() {
    if let Some(rev) = pm.current() { extract_from_revision(&mut m, rev); }
}
```

Use the shared `metadata::extraction::extract_from_revision` for both — the
loader once kept its own copy of the tag mapper, it drifted, and MP3 imports came
back untagged. `tests/mp3_import_e2e.rs` guards this (run with `-- --ignored`).

### Path Handling (camino vs std)

**Converting Between Path Types**:
- `camino::Utf8PathBuf` doesn't implement `From<Cow<str>>`
- Convert via `.to_string()` first when using `to_string_lossy()`

```rust
use camino::Utf8PathBuf;

// Correct conversion from std::path::PathBuf
let std_path = std::path::PathBuf::from("/some/path");
let utf8_path = Utf8PathBuf::from(std_path.to_string_lossy().to_string());
```

### Frontend TypeScript Patterns

**JobStatus as Type and Value**:
- TypeScript types exist at compile time only
- To use as runtime values, export both type and const object:

```typescript
// In types/models.ts
export type JobStatus = 'queued' | 'running' | 'complete' | 'error' | 'cancelled';
export const JobStatus = {
    Queued: 'queued',
    Running: 'running',
    Complete: 'complete',
    Error: 'error',
    Cancelled: 'cancelled',
} as const;

// Usage - import value, not just type
import { JobStatus } from '../types/models';
job.status === JobStatus.Running  // Runtime comparison
```

**Type-only Imports**:
- With `verbatimModuleSyntax`, use `type` keyword for type-only imports
- Mix regular and type imports when needed:

```typescript
import { JobStatus } from '../types/models';  // Value import
import type { ConversionJob } from '../types/models';  // Type-only import
```

**Tauri Event Listeners**:
- Use `type UnlistenFn` for the unlisten function type
- Clean up listeners in useEffect return function

```typescript
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

useEffect(() => {
    const unlisteners: UnlistenFn[] = [];

    const setup = async () => {
        unlisteners.push(await listen<EventType>('event_name', handler));
    };
    setup();

    return () => unlisteners.forEach(fn => fn());
}, []);
```

### Module Export Patterns

**Re-exporting from mod.rs**:
- When adding new modules, add both `pub mod` and `pub use`:

```rust
// In conversion/mod.rs
pub mod runner;  // Declare module
pub use runner::*;  // Re-export public items
```

**Checking Available Exports**:
- Use `cargo check` to verify module visibility
- Check actual location of types before importing (e.g., `UserSettings` is in `config`, not `models`)

### UI Component Availability

**Check Existing Components First**:
- Before importing UI components, verify they exist in `src/components/ui/`
- Available base components: `button`, `input`, `label`, `progress`, `toast`
- Use native HTML elements (select, checkbox) when Radix components unavailable
- Can build simple modals with divs + fixed positioning

### Build & Compile Workflow

**Iterative Error Fixing**:
1. Run `cargo check` (faster than `cargo build`)
2. Fix errors one category at a time
3. For frontend: `npm run build` shows TypeScript errors
4. Check both lib and bin compilation separately

**Common Error Categories**:
- E0432: Unresolved import - check module path and exports
- E0599: Method not found - check trait imports (e.g., `Emitter`, `Manager`)
- E0308: Type mismatch - check function signatures and conversions
- E0596: Cannot borrow as mutable - add `mut` keyword

---

**Good luck! This is a well-planned, achievable project. Follow the tasks systematically and you'll have a high-quality Rust audiobook converter. 🚀**


---

## UI Architecture (Workbench, Nov 2026)

The step-by-step scrolling workflow was replaced with a fixed three-pane
workbench. The shape of the UI is now the shape of the task: one book open at a
time, its material in the center, its product on the right, and the action that
produces it pinned to the bottom.

### Layout

```
TopBar          book chip (identity + project menu) · undo/redo · queue pill
Rail            Book / Queue / Settings / Help — navigation only, never commands
Center          Files | Chapters tabs, each pane owns its own toolbar + scroll
Inspector       cover art + metadata, always visible while editing the book
ConvertBar      readiness strip, output path, quality popover, Convert to M4B
AudioPlayer     in flow at the bottom; renders null when nothing is playing
```

Nothing scrolls except the inner panes. The Convert action can never scroll out
of reach of the files it applies to — that was the core failure of the old
single-document layout.

### File map

| Path | Role |
| --- | --- |
| `components/shell/` | `AppShell`, `TopBar`, `Rail` — the fixed chrome |
| `components/workbench/` | `Workbench`, `FilesPanel`, `BookInspector`, `ConvertBar`, `EmptyState` |
| `components/queue/QueuePage.tsx` | Jobs as a destination, with queue-level commands |
| `components/batch/BatchPage.tsx` | Several books from one import, reviewed and converted together |
| `context/BatchStore.tsx` | Batch state: import routing (one book → workbench, several → batch), background loading, Convert all |
| `context/useApp.ts`, `useConversionStore.ts`, `useBatch.ts` | Each provider's hook, context object and types (see "Hooks live apart from providers") |
| `components/ui/popover.tsx` | Anchored panel + menu items (no Radix dependency) |
| `components/ui/select.tsx` | Native `<select>` in the field style — use it, not a bare select |
| `components/ui/confirm.tsx` | `useConfirm()`: in-app confirm dialog on `<dialog>`; provider is in `main.tsx` |
| `hooks/useProjectActions.ts` | Import / open / save / export / close, shared by the menu and shortcuts |
| `hooks/useRebuildBook.ts` | Rebuild the open book from a new file list (reorder, sort, remove, add) |
| `lib/conversionSettings.ts` | Builds a job's `ConversionSettings`; shared by the Convert bar and the batch |
| `lib/outputTemplate.ts` | Renders the naming template (`{author}/{series}/{title}`) into an output path; tested by `npm test` |
| `hooks/useFileDrop.ts` | Native Tauri drag-drop; degrades to no-op outside a webview |
| `lib/format.ts` | `formatBytes` / `formatDuration` / `formatClock` / `formatEta` |
| `lib/formats.ts` | Supported input formats as shown to users; mirrors `SUPPORTED_EXTENSIONS` in `audio/format.rs` |
| `dev/mockBackend.ts` | Dev-only `?mock` fixtures (see below) |

### Conventions

- **The rail holds destinations, not actions.** Commands live on the object they
  act on: the book menu in `TopBar`, pane commands in each pane's toolbar.
- **Design tokens only.** Use `bg-surface`, `bg-surface-2`, `bg-rail`, `text-success`,
  `text-warning` and the usual shadcn tokens from `index.css`. Both themes are
  defined there; never hard-code a hex value.
- **The palette comes from the app icon** (`src-tauri/icons/icon.png`): indigo
  primary, green success, amber warning. Change the icon and the tokens together.
- **Indigo means "go" or "here".** `primary` marks a view's one main action (Convert,
  Save changes, Retry), the current location (rail, active tab) and focus. Selected
  segments and filter chips use neutral `accent`/`background` instead, so the main
  action is never ambiguous.
- **No `window.confirm` / `window.prompt`.** They are unstyled, cannot name their
  buttons, and are not guaranteed inside a Tauri webview. Use `useConfirm()`, or
  an inline input for naming things.
- **One icon family: heroicons** (24/outline for UI, 20/solid for small transport
  glyphs). `lucide-react` was removed; do not add a second set.
- **`nums` on any numeric column** (durations, sizes, counts) so digits do not jitter.
- **`no-select` on chrome** — toolbars, rails and headers are not documents.
- **Hooks live apart from providers.** `AppContext.tsx`, `ConversionStore.tsx` and
  `BatchStore.tsx` export only their provider; the hook, the context object and
  the types are in `context/useX.ts`. Exporting a hook from the provider file
  makes Vite reload the whole module on edit, which creates a second context and
  leaves every consumer throwing "must be used within" until a full refresh.
- **Metadata commits on blur**, not per keystroke: one undo entry and one backend
  round trip per field edit.
- **Reordering or removing files rebuilds the audiobook** via
  `create_audiobook_from_files`, because the backend has no move/remove command.
  `FilesPanel.rebuild()` re-applies typed metadata afterwards and prompts first
  when chapters were hand-edited, since they are regenerated.

### Batch import

`group_paths_into_books` (`audio/batch.rs`) splits a drop into books by folder
shape alone: a folder is one book when it holds audio or only disc folders
(`CD1/`), otherwise its subfolders are searched. It never reads tags, so the
same library always splits the same way. Batch books load with
`create_audiobook_from_files(paths, activate: false)` so the book open in the
workbench stays the active one.

**Output paths come from one template.** `UserSettings.output_template` is only
stored by the backend; `lib/outputTemplate.ts` renders it for the Convert bar,
the batch and the Settings preview. Empty fields drop out with their separators,
and tag values are sanitized so they can never add folders. Its tests run under
plain Node (`npm test --prefix src-ui`), so keep that file free of runtime imports.

**Undo is per book.** The backend keeps every loaded book, and `UndoManager`
filters its shared history by `audiobook_id`. Never make undo/redo pop the global
stack — it would replay one book's edit onto another.

### Previewing the UI without a Tauri build

```bash
npm run dev --prefix src-ui   # then open http://localhost:5173/?mock (or ?mock=library for a batch)
```

`?mock` installs `window.__TAURI_INTERNALS__` from `src/dev/mockBackend.ts` with a
fixture audiobook, so panes can be laid out and screenshotted without waiting on
`cargo build`. It is gated on `import.meta.env.DEV` and is stripped from release
bundles. Add a handler there whenever a new command is called during startup or
the book flow, or it logs `[mock] unhandled command`.
