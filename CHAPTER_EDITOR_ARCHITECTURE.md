# Chapter & Metadata Editor Architecture

## System Overview

```mermaid
flowchart TD
    subgraph Frontend [Frontend React UI]
        ChapterEditor[ChapterEditor.tsx]
        MetadataEditor[MetadataEditor.tsx]
        AppContext[AppContext.tsx]
        ChaptersAPI[api/chapters.ts]
        AudiobookAPI[api/audiobook.ts]
        FilesAPI[api/files.ts]
    end
    
    subgraph TauriIPC [Tauri IPC Layer]
        MetadataCommands[commands/metadata.rs]
        AudiobookCommands[commands/audiobook.rs]
        FilesCommands[commands/files.rs]
        UndoCommands[commands/undo.rs]
    end
    
    subgraph Backend [Backend Rust Logic]
        ChapterIO[metadata/chapter_io.rs]
        ChapterGen[metadata/chapter_generation.rs]
        UndoManager[undo.rs]
        AppState[state.rs]
        Models[models.rs]
    end
    
    ChapterEditor --> ChaptersAPI
    ChapterEditor --> FilesAPI
    MetadataEditor --> AppContext
    AppContext --> AudiobookAPI
    
    ChaptersAPI --> MetadataCommands
    FilesAPI --> FilesCommands
    AudiobookAPI --> AudiobookCommands
    
    MetadataCommands --> ChapterIO
    MetadataCommands --> ChapterGen
    MetadataCommands --> UndoManager
    UndoCommands --> UndoManager
    
    UndoManager --> AppState
    AppState --> Models
```

## Chapter Import/Export Flow

```mermaid
sequenceDiagram
    actor User
    participant UI as ChapterEditor
    participant API as chapters.ts
    participant Cmd as metadata commands
    participant IO as chapter_io.rs
    participant Undo as UndoManager
    participant State as AppState
    
    User->>UI: Click "Import from File"
    UI->>API: pickChaptersFile()
    API-->>UI: file path
    UI->>API: readTextFile(path)
    API-->>UI: file content
    UI->>API: importChapters(id, format, content)
    API->>Cmd: import_chapters
    Cmd->>IO: parse_chapters(content, format)
    IO-->>Cmd: Vec<Chapter>
    Cmd->>Cmd: validate_chapters
    Cmd->>Undo: execute(ReplaceChapters)
    Undo->>State: update_audiobook
    State-->>Cmd: AudioBook
    Cmd-->>API: AudioBook
    API-->>UI: AudioBook
    UI->>UI: setActiveAudiobook
```

## Smart Generation Flow

```mermaid
sequenceDiagram
    actor User
    participant UI as ChapterEditor
    participant API as chapters.ts
    participant Cmd as metadata commands
    participant Gen as chapter_generation.rs
    participant Format as format_specific.rs
    participant Undo as UndoManager
    
    User->>UI: Click "Extract from Source"
    UI->>API: generateChaptersFromSource(id)
    API->>Cmd: generate_chapters_from_source
    Cmd->>Gen: extract_chapters_from_sources(files)
    Gen->>Format: auto_extract_chapters(file)
    Format->>Format: Try M4B chapters
    Format->>Format: Try MP3 CHAP
    Format->>Format: Try FLAC cuesheet
    Format-->>Gen: Vec<Chapter>
    Gen->>Gen: Adjust times for concatenation
    Gen-->>Cmd: Vec<Chapter>
    Cmd->>Undo: execute(ReplaceChapters)
    Undo-->>Cmd: AudioBook
    Cmd-->>API: AudioBook
    API-->>UI: AudioBook
```

## Undo/Redo Flow

```mermaid
sequenceDiagram
    actor User
    participant UI as Any Editor
    participant Ctx as AppContext
    participant API as audiobook/undo APIs
    participant Cmd as undo commands
    participant Undo as UndoManager
    participant State as AppState
    
    User->>UI: Edit metadata/chapters
    UI->>API: update/import/generate
    API->>Cmd: metadata command (with UndoState)
    Cmd->>Undo: execute(EditCommand)
    Undo->>Undo: Push to undo stack
    Undo->>State: apply changes
    
    User->>User: Press Ctrl+Z
    Ctx->>API: undo()
    API->>Cmd: undo
    Cmd->>Undo: undo()
    Undo->>Undo: Pop from undo stack
    Undo->>State: unapply changes
    State-->>Cmd: success
    Cmd-->>API: description
    API->>API: getActiveAudiobook()
    API-->>Ctx: updated AudioBook
    Ctx->>Ctx: setActiveAudiobook
    Ctx->>Ctx: refreshUndoState
```

## Key Design Decisions

### 1. Format Auto-Detection
- Import auto-detects format from file extension
- Supports both `.txt` (Audacity) and `.ffmeta` (FFmpeg) extensions
- CSV headers are optional (auto-detected)

### 2. Flexible Time Parsing
- Accepts multiple time formats: `HH:MM:SS`, `MM:SS`, or decimal seconds
- Converts all to milliseconds internally
- Graceful error handling with line numbers

### 3. Single Undo Step for Bulk Operations
- Chapter import = one undo operation
- Regex rename = one undo operation
- Generation = one undo operation
- Uses `ReplaceChapters` command to store old/new state

### 4. Validation at Multiple Layers
- Parse-time validation (format-specific)
- Pre-apply validation (overlap/duration checks)
- Save-time validation (metadata field limits)

### 5. Progressive Disclosure in UI
- Basic operations visible by default
- Advanced features (custom regex) in expandable sections
- Warnings shown inline when detected

## Data Models

### Chapter Structure
```rust
pub struct Chapter {
    pub index: usize,           // Zero-based position
    pub title: String,          // Display name
    pub start_time_ms: u64,     // Milliseconds from start
    pub end_time_ms: u64,       // Milliseconds from start
    pub auto_detected: bool,    // Source tracking
}
```

### Metadata Structure (Extended)
```rust
pub struct Metadata {
    // Core fields
    pub title: Option<String>,
    pub artist: Option<String>,
    pub narrator: Option<String>,
    
    // Series fields (NEW)
    pub series: Option<String>,
    pub series_index: Option<u32>,
    
    // Language (NEW)
    pub language: Option<String>,
    
    // Publishing
    pub publisher: Option<String>,
    pub copyright: Option<String>,
    
    // Indexing
    pub track_number: Option<u32>,
    pub total_tracks: Option<u32>,
    pub disc_number: Option<u32>,
    pub total_discs: Option<u32>,
    
    // Notes
    pub description: Option<String>,
    pub comments: Option<String>,
    
    // ... other fields
}
```

## Performance Characteristics

- **Import**: O(n) where n = lines in file
- **Export**: O(n) where n = chapters
- **Regex rename**: O(n*m) where n = chapters, m = title length
- **Extract from source**: O(n) where n = files (cached per file)
- **Undo/Redo**: O(1) stack operations + O(n) chapter copy

## Error Handling

### Parse Errors
- Invalid time values → validation error with line number
- Invalid format → descriptive error message
- Missing required fields → warning, skip line

### Runtime Errors
- Audiobook not found → user-friendly message
- File not readable → permission/not found error
- Regex invalid → pattern syntax error with description

### Recovery
- All operations are transactional (undo available)
- Failed imports don't corrupt existing chapters
- Export failures don't modify source data

## Future Enhancements

### Already Architected For
- Preview before import (UI state management ready)
- Batch chapter operations (undo system supports batching)
- Custom validation rules (validation layer is extensible)

### Easy to Add
- More export formats (JSON, YAML) - just add to `ChapterFormat` enum
- More regex presets - extend `get_regex_presets()`
- Inline editing - reuse existing `update_chapter` command
- Visual timeline - data model already has precise times

