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

**Windows**: NSIS installer with FFmpeg
**macOS**: DMG with code signing
**Linux**: AppImage or Flatpak

**Key Tasks**:
- [ ] Optimize release build (LTO)
- [ ] Create platform-specific installers
- [ ] Test on clean system
- [ ] Bundle FFmpeg appropriately

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

**Good luck! This is a well-planned, achievable project. Follow the tasks systematically and you'll have a high-quality Rust audiobook converter. 🚀**

