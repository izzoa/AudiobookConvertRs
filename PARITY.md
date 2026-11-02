# Feature Parity Plan: AudiobookConvertRs vs AudioBookConverter

This document outlines the implementation plan to achieve feature parity with [AudioBookConverter](https://github.com/yermak/AudioBookConverter) (Java).

## Current Status

AudiobookConvertRs already implements core functionality. The following features are needed to match the Java version:

### Missing Features

| Category | Feature | Status |
|----------|---------|--------|
| Input Formats | WMA support | Not implemented |
| Input Formats | FLAC/CUE sheet support | ✅ Complete |
| Output Formats | OGG/Opus output | Not implemented |
| Output Formats | MP3 (LAME) output | Not implemented |
| Playback | Audio preview before conversion | ✅ Complete |
| Artwork | Smart directory artwork discovery | ✅ Complete |
| UI | Quality management slider | ✅ Complete |
| Processing | Parallel book preparation | Partial |

---

## Implementation Plan

### Phase 1: Smart Artwork Discovery ✅

**Goal**: Automatically find cover artwork from input file directories.

**Tasks**:
- [x] Scan input file directories for common cover image names
- [x] Search patterns: `cover.jpg`, `cover.png`, `folder.jpg`, `album.jpg`, `front.jpg`
- [x] Implement priority order: embedded > cover.* > folder.* > album.*
- [x] Add UI to show discovered artwork options
- [x] Allow user to choose between multiple found images

**Files modified**:
- `src-tauri/src/metadata/artwork.rs` - Added `discover_artwork_options()`, `DiscoveredArtwork`, `ArtworkSource`
- `src-tauri/src/commands/metadata.rs` - Added `discover_artwork`, `select_discovered_artwork` commands
- `src-ui/src/components/ArtworkPicker.tsx` - Auto-scans on file load, shows discovered options with priority badges

---

### Phase 2: Audio Preview/Playback ✅

**Goal**: Allow users to preview audio before conversion.

**Approach**: Use `rodio` crate for native Rust audio playback.

**Tasks**:
- [x] Add `rodio` dependency to Cargo.toml
- [x] Create audio player service (`src-tauri/src/audio/player.rs`)
- [x] Implement playback state management
- [x] Add IPC commands:
  - `play_audio(path, start_ms)` - Start playback
  - `pause_audio()` - Pause playback
  - `stop_audio()` - Stop playback
  - `seek_audio(position_ms)` - Seek to position
  - `get_playback_status()` - Get current position/state
- [x] Add play button to chapter list items
- [x] Show playback position indicator
- [ ] Add play button to file list items (optional enhancement)

**Files created/modified**:
- `src-tauri/src/audio/player.rs` - Thread-based playback service using mpsc channels
- `src-tauri/src/commands/playback.rs` - IPC commands
- `src-ui/src/api/playback.ts` - TypeScript API
- `src-ui/src/components/AudioPlayer.tsx` - Fixed bottom player bar with controls
- `src-ui/src/components/ChapterEditor.tsx` - Play buttons on chapters

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

### Phase 5: Quality Management UI ✅

**Status**: Complete

**Goal**: Provide intuitive quality/size tradeoff controls with presets and real-time file size estimation.

**Presets**:
| Preset | Bitrate | Sample Rate | Mode |
|--------|---------|-------------|------|
| High | 256 kbps | 44100 Hz | VBR |
| Medium | 128 kbps | 44100 Hz | VBR |
| Low | 64 kbps | 22050 Hz | CBR |
| Custom | user-defined | user-defined | user-defined |

#### Phase 5.1: Backend Data Structures
- [x] Add `QualityPreset` enum to `src-tauri/src/models.rs` with variants: `High`, `Medium`, `Low`, `Custom`
- [x] Implement `QualityPreset::bitrate_kbps()` returning preset's default bitrate
- [x] Implement `QualityPreset::sample_rate_hz()` returning preset's default sample rate
- [x] Implement `QualityPreset::is_vbr()` returning whether preset uses VBR encoding
- [x] Add `EncodingMode` enum to `src-tauri/src/models.rs` with variants: `Vbr`, `Cbr`
- [x] Add `encoding_mode: EncodingMode` field to `ConversionSettings` struct
- [x] Add `encoding_mode: EncodingMode` field to `ConversionDefaults` struct
- [x] Add `estimate_file_size_bytes(duration_ms: u64, bitrate_kbps: u32) -> u64` helper function
- [x] Add unit tests for `QualityPreset` methods
- [x] Add unit tests for `EncodingMode` serialization
- [x] Add unit tests for file size estimation function

#### Phase 5.2: Configuration Updates
- [x] Add `default_quality_preset: QualityPreset` to `UserSettings` in `src-tauri/src/config.rs`
- [x] Add `default_encoding_mode: EncodingMode` to `UserSettings` in `src-tauri/src/config.rs`
- [x] Update `UserSettings::default()` to use `QualityPreset::Medium` and `EncodingMode::Vbr`
- [x] Update `ConversionDefaults::from_user_settings()` to include new fields
- [x] Add validation for encoding mode in settings validation

#### Phase 5.3: FFmpeg Command Builder Updates
- [x] Review `src-tauri/src/ffmpeg/command_builder.rs` for current AAC encoding approach
- [x] Add `EncodingMode` parameter to audio encoding command building
- [x] Implement VBR encoding using FFmpeg's `-q:a` quality scale (0-9 for AAC, lower = better)
- [x] Verify CBR encoding uses `-b:a` bitrate flag correctly
- [x] Map QualityPreset to appropriate VBR quality values (High=2, Medium=4, Low=6)
- [ ] Add integration test: generate FFmpeg command for VBR preset
- [ ] Add integration test: generate FFmpeg command for CBR preset

#### Phase 5.4: Backend Commands
- [x] Add `estimate_output_size` command to `src-tauri/src/commands/quality.rs`
  - Input: `duration_ms: u64, bitrate_kbps: u32`
  - Output: `FileSizeEstimate { bytes: u64, formatted: String }`
- [x] Add `get_quality_preset_info` command returning all preset configurations
- [x] Register new commands in `src-tauri/src/commands/mod.rs`
- [x] Register new commands in `src-tauri/src/main.rs` invoke handler

#### Phase 5.5: Frontend Types
- [x] Add `QualityPreset` type to `src-ui/src/types/models.ts`: `'high' | 'medium' | 'low' | 'custom'`
- [x] Add `EncodingMode` type to `src-ui/src/types/models.ts`: `'vbr' | 'cbr'`
- [x] Add `encoding_mode?: EncodingMode` to `ConversionSettings` interface
- [x] Add `default_quality_preset?: QualityPreset` to `UserSettings` interface
- [x] Add `default_encoding_mode?: EncodingMode` to `UserSettings` interface
- [x] Add `FileSizeEstimate` interface: `{ bytes: number, formatted: string }`

#### Phase 5.6: Frontend API
- [x] Create `src-ui/src/api/quality.ts` file
- [x] Add `estimateOutputSize(durationMs: number, bitrateKbps: number): Promise<FileSizeEstimate>`
- [x] Add `getQualityPresetInfo(): Promise<PresetInfo[]>` returning preset configurations
- [x] Export functions from `src-ui/src/api/index.ts` if using barrel exports

#### Phase 5.7: UI Component - Slider (if needed)
- [x] Check if `src-ui/src/components/ui/slider.tsx` exists
- [x] Install Radix Slider: `npm install @radix-ui/react-slider`
- [x] Create `src-ui/src/components/ui/slider.tsx` wrapper component
- [x] Style slider to match existing UI theme (colors, sizing)

#### Phase 5.8: ConversionSettingsDialog Enhancement
- [x] Add `qualityPreset` state initialized from defaults
- [x] Add `encodingMode` state initialized from defaults
- [x] Add Quality Preset dropdown at top of audio settings section
- [x] Implement `handlePresetChange` function:
  - Set bitrate from preset's `bitrate_kbps()`
  - Set sample rate from preset's `sample_rate_hz()`
  - Set encoding mode from preset's `is_vbr()`
  - Enable/disable manual controls based on Custom selection
- [x] Add VBR/CBR radio button group below preset dropdown
- [x] Disable encoding mode radios when preset is not Custom
- [x] Replace bitrate dropdown with slider component (range: 32-320, step: 8)
- [x] Add bitrate value label next to slider showing current kbps
- [x] Disable bitrate slider when preset is not Custom
- [x] Disable sample rate dropdown when preset is not Custom
- [x] Add estimated file size display below quality controls
- [x] Implement `calculateEstimatedSize` function using `(bitrate * duration) / 8000`
- [x] Format estimated size as MB/GB with 2 decimal places
- [x] Show source file size and compression ratio (e.g., "~45 MB (65% of source)")
- [x] Update `handleConfirm` to include `encoding_mode` in settings object
- [x] Add visual indicator showing current preset settings summary

#### Phase 5.9: Settings Page Updates
- [x] Add "Audio Quality Defaults" section to `src-ui/src/components/SettingsPage.tsx`
- [x] Add default quality preset dropdown
- [x] Add default encoding mode radio buttons
- [x] Wire preset/mode to settings state
- [x] Ensure changes persist via settings API
- [x] Add section description explaining these become defaults for new conversions

#### Phase 5.10: Testing & Validation
- [x] Unit test: QualityPreset returns correct bitrates
- [x] Unit test: QualityPreset returns correct sample rates
- [x] Unit test: EncodingMode serializes correctly to JSON
- [x] Unit test: File size estimation matches expected formula
- [ ] Integration test: Preset selection generates correct FFmpeg args
- [ ] Integration test: VBR mode produces smaller file than CBR at same quality
- [ ] Integration test: Settings persist and reload correctly
- [x] Manual test: UI controls enable/disable correctly with presets
- [x] Manual test: File size updates in real-time as bitrate changes
- [ ] Manual test: Converted file size approximately matches estimate

#### Phase 5.11: Documentation & Polish
- [x] Add tooltips to VBR/CBR explaining the difference
- [x] Add preset descriptions (e.g., "High: Best quality, larger files")
- [x] Ensure disabled controls have proper visual styling
- [x] Verify keyboard navigation works for all controls
- [x] Update PARITY.md to mark Phase 5 as complete

**Files Created**:
- `src-ui/src/api/quality.ts`
- `src-ui/src/components/ui/slider.tsx`

**Files Modified**:
- `src-tauri/src/models.rs`
- `src-tauri/src/config.rs`
- `src-tauri/src/ffmpeg/command_builder.rs`
- `src-tauri/src/commands/quality.rs`
- `src-tauri/src/commands/mod.rs`
- `src-tauri/src/main.rs`
- `src-ui/src/types/models.ts`
- `src-ui/src/components/ConversionSettingsDialog.tsx`
- `src-ui/src/components/SettingsPage.tsx`

**Dependency Flow**:
```
[5.1 Data Structures] ──┬──> [5.2 Config] ──> [5.4 Commands]
                        │                           │
                        └──> [5.3 FFmpeg] ──────────┤
                        │                           │
                        └──> [5.5 Types] ──> [5.6 API]
                                                    │
                                    [5.7 Slider] ───┤
                                                    v
                                          [5.8 Dialog UI]
                                                    │
                                                    v
                                          [5.9 Settings Page]
                                                    │
                                                    v
                                          [5.10 Testing]
                                                    │
                                                    v
                                          [5.11 Documentation]
```

**Success Criteria for Phase 5**:
- [x] Quality preset dropdown with High/Medium/Low/Custom options
- [x] Presets auto-configure bitrate, sample rate, and encoding mode
- [x] Custom mode enables individual control adjustment
- [x] Bitrate slider (or enhanced dropdown) with 32-320 kbps range
- [x] VBR/CBR toggle functional and affects FFmpeg output
- [x] Real-time estimated file size updates as settings change
- [x] Compression ratio shown relative to source files
- [x] Quality defaults saved in user preferences
- [ ] All unit and integration tests passing (some integration tests pending)

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
- [x] Research `cue_sheet` crate vs custom parser trade-offs
- [x] Add `cue_sheet` crate to Cargo.toml (or decide on custom implementation)
- [x] Review existing `Chapter` struct in `models.rs` for compatibility

#### Phase 6.2: CUE Parser Module
- [x] Create `src-tauri/src/metadata/cue_parser.rs`
- [x] Define `CueSheet` struct (file references, tracks, metadata)
- [x] Define `CueTrack` struct (index, title, performer, start_time)
- [x] Implement `parse_cue_file(path: &Path) -> Result<CueSheet, CueError>`
- [x] Handle CUE timestamp format (MM:SS:FF where FF is frames, 75 frames/sec)
- [x] Convert CUE timestamps to milliseconds
- [x] Parse TITLE, PERFORMER, FILE, TRACK, INDEX commands
- [x] Handle quoted strings and escape sequences
- [x] Add unit tests with sample CUE content

#### Phase 6.3: CUE Detection in Audio Loader
- [x] Modify `src-tauri/src/audio/loader.rs`
- [x] Add `find_cue_file(flac_path: &Path) -> Option<PathBuf>` function
- [x] Check for `.cue` file with same base name as `.flac`
- [x] Check for `.cue` file in same directory with matching FILE reference
- [x] Return CUE path if found

#### Phase 6.4: Chapter Generation from CUE
- [x] Modify `src-tauri/src/metadata/chapters.rs`
- [x] Add `generate_chapters_from_cue(cue: &CueSheet, total_duration_ms: u64) -> Vec<Chapter>`
- [x] Map each `CueTrack` to a `Chapter`
- [x] Calculate chapter end times (next track start or total duration)
- [x] Use track TITLE as chapter title (fallback to "Track N")
- [x] Preserve PERFORMER in chapter metadata if available

#### Phase 6.5: Multi-file CUE Support
- [x] Handle CUE sheets referencing multiple FILE entries
- [x] Map each FILE to corresponding audio file
- [x] Calculate cumulative offsets for multi-file chapters
- [x] Validate all referenced files exist

#### Phase 6.6: IPC Command Integration
- [x] Add `detect_cue_file` command to `commands/files.rs`
- [x] Add `import_chapters_from_cue` command to `commands/metadata.rs`
- [x] Return structured response with CUE detection status and track count

#### Phase 6.7: Frontend UI
- [x] Modify `ChapterPanel.tsx` or create `CueImportDialog.tsx`
- [x] Show notification when CUE file detected alongside FLAC
- [x] Display parsed track list for user confirmation
- [x] Add "Import as Chapters" button
- [x] Update chapter list after import

#### Phase 6.8: Testing & Edge Cases
- [x] Test with single-file CUE sheets
- [x] Test with multi-file CUE sheets
- [x] Test with Unicode track titles (Japanese, Cyrillic, etc.)
- [x] Test with missing/malformed INDEX entries
- [x] Test with hidden Track 00 (pregap)
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
- [x] Can import FLAC with CUE sheets (tracks become chapters)
- [ ] Can output to M4B, MP3, and OGG/Opus (M4B done, MP3/OGG pending)
- [x] Can preview audio before conversion
- [x] Auto-discovers cover art from directories
- [x] Quality slider with file size estimation
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
