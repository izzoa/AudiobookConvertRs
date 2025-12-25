# Feature Parity Plan: AudiobookConvertRs vs AudioBookConverter

This document outlines the implementation plan to achieve feature parity with [AudioBookConverter](https://github.com/yermak/AudioBookConverter) (Java).

## Current Status

AudiobookConvertRs already implements core functionality. The following features are needed to match the Java version:

### Missing Features

| Category | Feature | Status |
|----------|---------|--------|
| Input Formats | WMA support | Not implemented |
| Input Formats | FLAC/CUE sheet support | Not implemented |
| Output Formats | OGG/Opus output | Not implemented |
| Output Formats | MP3 (LAME) output | Not implemented |
| Playback | Audio preview before conversion | Not implemented |
| Artwork | Smart directory artwork discovery | Not implemented |
| UI | Quality management slider | Partial |
| Processing | Parallel book preparation | Partial |

---

## Implementation Plan

### Phase 1: Smart Artwork Discovery

**Goal**: Automatically find cover artwork from input file directories.

**Tasks**:
- [ ] Scan input file directories for common cover image names
- [ ] Search patterns: `cover.jpg`, `cover.png`, `folder.jpg`, `album.jpg`, `front.jpg`
- [ ] Implement priority order: embedded > cover.* > folder.* > album.*
- [ ] Add UI to show discovered artwork options
- [ ] Allow user to choose between multiple found images

**Files to modify**:
- `src-tauri/src/metadata/artwork.rs` - Add directory scanning
- `src-tauri/src/commands/metadata.rs` - Add discovery command
- `src-ui/src/components/ArtworkPicker.tsx` - Show discovered options

---

### Phase 2: Audio Preview/Playback

**Goal**: Allow users to preview audio before conversion.

**Approach**: Use `rodio` crate for native Rust audio playback.

**Tasks**:
- [ ] Add `rodio` dependency to Cargo.toml
- [ ] Create audio player service (`src-tauri/src/audio/player.rs`)
- [ ] Implement playback state management
- [ ] Add IPC commands:
  - `play_audio(path, start_ms)` - Start playback
  - `pause_audio()` - Pause playback
  - `stop_audio()` - Stop playback
  - `seek_audio(position_ms)` - Seek to position
  - `get_playback_status()` - Get current position/state
- [ ] Add play button to file list items
- [ ] Add play button to chapter list items
- [ ] Show playback position indicator
- [ ] Handle seeking within chapters

**Fallback**: If rodio has cross-platform issues, use HTML5 Audio with blob URLs.

**Files to create**:
- `src-tauri/src/audio/player.rs` - Playback service
- `src-ui/src/components/AudioPlayer.tsx` - Player controls

---

### Phase 3: MP3 Output Format

**Goal**: Support MP3 as an output format with chapters.

**Tasks**:
- [ ] Add `OutputFormat::Mp3` variant to `ConversionSettings`
- [ ] Update FFmpeg command builder for `-c:a libmp3lame`
- [ ] Implement MP3 chapter support via ID3v2 CHAP/CTOC frames
- [ ] Add `id3` crate dependency for chapter writing
- [ ] Handle MP3 metadata writing (artist, title, album, etc.)
- [ ] Add MP3 option to output format selector in UI
- [ ] Add `.mp3` extension handling

**FFmpeg command template**:
```
ffmpeg -i input.mp3 -c:a libmp3lame -b:a 128k -ar 44100 output.mp3
```

**Files to modify**:
- `src-tauri/src/models.rs` - Add OutputFormat enum variant
- `src-tauri/src/ffmpeg/command_builder.rs` - MP3 encoding
- `src-tauri/src/metadata/writing.rs` - MP3 metadata/chapters
- `src-ui/src/components/ConversionSettings.tsx` - Format selector

---

### Phase 4: OGG/Opus Output Format

**Goal**: Support Opus as an output format.

**Tasks**:
- [ ] Add `OutputFormat::Opus` variant to `ConversionSettings`
- [ ] Update FFmpeg command builder for `-c:a libopus`
- [ ] Add `.opus` and `.ogg` extension handling
- [ ] Add Opus option to output format selector in UI
- [ ] Document limitation: Opus doesn't support embedded chapters
- [ ] Consider generating external chapter file (`.chapters.txt`)

**FFmpeg command template**:
```
ffmpeg -i input.mp3 -c:a libopus -b:a 64k output.opus
```

**Files to modify**:
- `src-tauri/src/models.rs` - Add OutputFormat enum variant
- `src-tauri/src/ffmpeg/command_builder.rs` - Opus encoding
- `src-ui/src/components/ConversionSettings.tsx` - Format selector

---

### Phase 5: Quality Management UI

**Goal**: Provide intuitive quality/size tradeoff controls.

**Tasks**:
- [ ] Add quality preset dropdown (High / Medium / Low / Custom)
- [ ] Add bitrate slider (64-320 kbps)
- [ ] Show real-time estimated file size based on duration and bitrate
- [ ] Add VBR/CBR toggle
- [ ] Add sample rate selection (44100 / 48000 / 22050)
- [ ] Persist quality settings in user preferences

**Presets**:
| Preset | Bitrate | Sample Rate | Mode |
|--------|---------|-------------|------|
| High | 256 kbps | 44100 Hz | VBR |
| Medium | 128 kbps | 44100 Hz | VBR |
| Low | 64 kbps | 22050 Hz | CBR |

**Files to modify**:
- `src-ui/src/components/ConversionSettings.tsx` - Quality controls
- `src-tauri/src/config.rs` - Persist settings

---

### Phase 6: FLAC/CUE Sheet Support

**Goal**: Import FLAC files with CUE sheets, mapping tracks to chapters.

**CUE format example**:
```
FILE "album.flac" WAVE
  TRACK 01 AUDIO
    TITLE "Chapter One"
    PERFORMER "Author Name"
    INDEX 01 00:00:00
  TRACK 02 AUDIO
    TITLE "Chapter Two"
    INDEX 01 05:32:15
```

#### Phase 6.1: Research & Dependencies
- [ ] Research `cue_sheet` crate vs custom parser trade-offs
- [ ] Add `cue_sheet` crate to Cargo.toml (or decide on custom implementation)
- [ ] Review existing `Chapter` struct in `models.rs` for compatibility

#### Phase 6.2: CUE Parser Module
- [ ] Create `src-tauri/src/metadata/cue_parser.rs`
- [ ] Define `CueSheet` struct (file references, tracks, metadata)
- [ ] Define `CueTrack` struct (index, title, performer, start_time)
- [ ] Implement `parse_cue_file(path: &Path) -> Result<CueSheet, CueError>`
- [ ] Handle CUE timestamp format (MM:SS:FF where FF is frames, 75 frames/sec)
- [ ] Convert CUE timestamps to milliseconds
- [ ] Parse TITLE, PERFORMER, FILE, TRACK, INDEX commands
- [ ] Handle quoted strings and escape sequences
- [ ] Add unit tests with sample CUE content

#### Phase 6.3: CUE Detection in Audio Loader
- [ ] Modify `src-tauri/src/audio/loader.rs`
- [ ] Add `find_cue_file(flac_path: &Path) -> Option<PathBuf>` function
- [ ] Check for `.cue` file with same base name as `.flac`
- [ ] Check for `.cue` file in same directory with matching FILE reference
- [ ] Return CUE path if found

#### Phase 6.4: Chapter Generation from CUE
- [ ] Modify `src-tauri/src/metadata/chapters.rs`
- [ ] Add `generate_chapters_from_cue(cue: &CueSheet, total_duration_ms: u64) -> Vec<Chapter>`
- [ ] Map each `CueTrack` to a `Chapter`
- [ ] Calculate chapter end times (next track start or total duration)
- [ ] Use track TITLE as chapter title (fallback to "Track N")
- [ ] Preserve PERFORMER in chapter metadata if available

#### Phase 6.5: Multi-file CUE Support
- [ ] Handle CUE sheets referencing multiple FILE entries
- [ ] Map each FILE to corresponding audio file
- [ ] Calculate cumulative offsets for multi-file chapters
- [ ] Validate all referenced files exist

#### Phase 6.6: IPC Command Integration
- [ ] Add `detect_cue_file` command to `commands/files.rs`
- [ ] Add `import_chapters_from_cue` command to `commands/metadata.rs`
- [ ] Return structured response with CUE detection status and track count

#### Phase 6.7: Frontend UI
- [ ] Modify `ChapterPanel.tsx` or create `CueImportDialog.tsx`
- [ ] Show notification when CUE file detected alongside FLAC
- [ ] Display parsed track list for user confirmation
- [ ] Add "Import as Chapters" button
- [ ] Update chapter list after import

#### Phase 6.8: Testing & Edge Cases
- [ ] Test with single-file CUE sheets
- [ ] Test with multi-file CUE sheets
- [ ] Test with Unicode track titles (Japanese, Cyrillic, etc.)
- [ ] Test with missing/malformed INDEX entries
- [ ] Test with hidden Track 00 (pregap)
- [ ] Integration test: FLAC + CUE -> M4B with chapters

**Dependency Flow**:
```
[6.1 Research] --> [6.2 Parser] --> [6.3 Detection]
                        |                  |
                        v                  v
                   [6.4 Chapters] <--------+
                        |
                        v
                   [6.5 Multi-file]
                        |
                        v
                   [6.6 IPC Commands] --> [6.7 Frontend UI]
                                                |
                                                v
                                          [6.8 Testing]
```

---

### Phase 7: WMA Input Support

**Goal**: Allow importing WMA audio files.

**Challenge**: Symphonia doesn't support WMA (proprietary Microsoft format).

**Solution**: Use FFmpeg for WMA files instead of Symphonia.

**Tasks**:
- [ ] Add WMA detection in `audio/format.rs`
- [ ] Create FFmpeg-based probe for WMA metadata extraction
- [ ] Extract duration, bitrate, sample rate via `ffprobe`
- [ ] Extract metadata tags via `ffprobe -show_format`
- [ ] Add WMA to supported formats list
- [ ] Test with WMA, WMA Pro, and WMA Lossless variants

**FFprobe command**:
```
ffprobe -v quiet -print_format json -show_format -show_streams input.wma
```

**Files to modify**:
- `src-tauri/src/audio/format.rs` - WMA detection
- `src-tauri/src/audio/loader.rs` - FFmpeg fallback for WMA
- `src-tauri/src/ffmpeg/` - Add ffprobe wrapper

---

### Phase 8: Polish and Testing

**Tasks**:
- [ ] Unicode metadata verification (Chinese, Arabic, Cyrillic characters)
- [ ] Cross-platform testing (Windows, macOS, Linux)
- [ ] End-to-end tests for each new format
- [ ] Performance benchmarking
- [ ] Documentation updates

---

## Dependencies

### New Rust Crates Required

| Crate | Purpose | Version |
|-------|---------|---------|
| `rodio` | Audio playback | 0.17+ |
| `id3` | MP3 ID3v2 tag writing | 1.x |

### Optional Crates

| Crate | Purpose | Notes |
|-------|---------|-------|
| `cue_sheet` | CUE parsing | May implement custom parser instead |

---

## Dependency Graph

```
[No dependencies - can start immediately]
|
+-- Smart Artwork Discovery
+-- Quality Management UI
+-- Unicode Verification
|
[Depends on FFmpeg command builder updates]
|
+-- WMA Support
+-- MP3 Output
+-- OGG/Opus Output
|
[Depends on audio decoding infrastructure]
|
+-- Audio Preview (requires rodio)
|
[Depends on CUE parser implementation]
|
+-- FLAC/CUE Support
```

---

## Priority Order

### P0 - High Impact
1. Audio Preview/Playback - Most requested usability feature
2. Smart Artwork Discovery - Quick win, high convenience

### P1 - Medium Impact
3. MP3 Output - Common format requirement
4. OGG/Opus Output - Open format, growing popularity

### P2 - Lower Impact
5. FLAC/CUE Support - Niche but important for CD rippers
6. WMA Support - Legacy format, declining usage

### P3 - Polish
7. Quality Management UI - Refinements
8. Parallel Processing - Performance optimization

---

## Success Criteria

Feature parity is achieved when:

- [ ] Can import WMA files
- [ ] Can import FLAC with CUE sheets (tracks become chapters)
- [ ] Can output to M4B, MP3, and OGG/Opus
- [ ] Can preview audio before conversion
- [ ] Auto-discovers cover art from directories
- [ ] Quality slider with file size estimation
- [ ] All features work on Windows, macOS, and Linux

---

## Risk Assessment

| Risk | Impact | Likelihood | Mitigation |
|------|--------|------------|------------|
| rodio cross-platform audio issues | Medium | Low | Fall back to HTML5 Audio element |
| MP3 chapter frame complexity | Medium | Medium | Use FFmpeg metadata injection as alternative |
| WMA licensing concerns | Low | Low | FFmpeg handles decoding, no direct WMA code |
| CUE parser edge cases | Low | Medium | Test with various CUE formats, handle errors gracefully |

---

## References

- [AudioBookConverter (Java)](https://github.com/yermak/AudioBookConverter)
- [rodio - Rust audio playback](https://github.com/RustAudio/rodio)
- [id3 - Rust ID3 tag library](https://github.com/polyfloyd/rust-id3)
- [CUE sheet format specification](https://wiki.hydrogenaud.io/index.php?title=Cue_sheet)
- [FFmpeg documentation](https://ffmpeg.org/documentation.html)
