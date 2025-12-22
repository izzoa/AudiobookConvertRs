# Chapter & Metadata Editing Improvements

## Summary

Implemented comprehensive chapter and metadata editing enhancements focused on interoperability, smart generation, and rich metadata support.

## Features Implemented

### 1. Chapter Import/Export (Interoperability)

**Backend:**
- Created `src-tauri/src/metadata/chapter_io.rs` with full parsing/rendering support for:
  - **Audacity Label Track** (.txt): Industry-standard format for audio editing
  - **FFmpeg Metadata** (.ffmeta): Direct compatibility with FFmpeg workflows
  - **CSV** (.csv): Spreadsheet-friendly format with flexible time parsing (HH:MM:SS or seconds)
- Added Tauri commands: `import_chapters`, `export_chapters`, `set_chapters`
- Added file picker and text file I/O commands: `pick_chapters_file`, `read_text_file`

**Frontend:**
- Created `src-ui/src/components/ChapterEditor.tsx` with:
  - Live chapter list display with overlap/gap warnings
  - One-click import from file (auto-detects format)
  - Export to all three formats with appropriate file extensions
- Added `src-ui/src/api/chapters.ts` for typed API calls

### 2. Smart Chapter Generation

**Backend:**
- Created `src-tauri/src/metadata/chapter_generation.rs` with:
  - `extract_chapters_from_sources()`: Extracts embedded chapters from M4B, MP3 (ID3 CHAP), and FLAC (cue sheets)
  - `rename_chapters_regex()`: Powerful regex-based title cleanup
  - Built-in regex presets for common patterns:
    - Remove numbered prefixes (01 -, 001., etc.)
    - Remove "Track" prefixes
    - Replace underscores with spaces
    - Remove file extensions
    - Clean multiple spaces
    - Remove "Chapter" prefixes
- Added Tauri commands: `generate_chapters_from_source`, `rename_chapters_with_regex`, `get_chapter_regex_presets`

**Frontend:**
- Extended `ChapterEditor.tsx` with:
  - "Extract from Source Metadata" button
  - Quick preset buttons for common cleanup patterns
  - Custom regex input for advanced users

### 3. Rich Metadata Fields

**Backend:**
- Extended `Metadata` struct in `src-tauri/src/models.rs` with:
  - **Series fields**: `series`, `series_index`
  - **Language**: `language`
  - Already had: `album_artist`, `composer`, `publisher`, `copyright`, `description`, `comments`, `track_number`, `total_tracks`, `disc_number`, `total_discs`
- Updated validation, normalization, and merge logic for new fields

**Frontend:**
- Completely redesigned `MetadataEditor.tsx` with grouped sections:
  - **Core Information**: title, author, narrator, album, album_artist, genre, year, language, composer
  - **Series**: series name and book number
  - **Publishing**: publisher, copyright
  - **Indexing**: track/disc numbering
  - **Notes & Description**: description (multi-line), comments (multi-line)
- Updated TypeScript types in `src-ui/src/types/models.ts`

### 4. Undo/Redo System (Safety)

**Backend:**
- Fixed `UndoState` management in `src-tauri/src/main.rs` (now properly `.manage()`d)
- Added `ReplaceChapters` command variant for bulk operations
- Integrated undo tracking into all edit operations:
  - `update_audiobook_metadata` now records metadata changes
  - `import_chapters` records bulk chapter replacement
  - `set_chapters` records chapter replacement
  - `generate_chapters_from_source` records generation
  - `rename_chapters_with_regex` records bulk rename
- Added `get_undo_state` combined API for efficiency
- Added `get_active_audiobook` command for state refresh

**Frontend:**
- Updated `AppContext.tsx` to refresh active audiobook after undo/redo
- Undo/redo now properly updates UI immediately
- Added `getActiveAudiobook()` to `src-ui/src/api/audiobook.ts`

## Files Changed

### Backend (Rust)
- **New modules:**
  - `src-tauri/src/metadata/chapter_io.rs` (275 lines)
  - `src-tauri/src/metadata/chapter_generation.rs` (185 lines)
- **Modified:**
  - `src-tauri/src/metadata/mod.rs` (exports)
  - `src-tauri/src/commands/metadata.rs` (5 new commands, undo integration)
  - `src-tauri/src/commands/files.rs` (2 new commands)
  - `src-tauri/src/commands/audiobook.rs` (1 new command)
  - `src-tauri/src/commands/undo.rs` (public API improvements)
  - `src-tauri/src/main.rs` (command registration, UndoState management)
  - `src-tauri/src/models.rs` (series/language fields, merge logic)
  - `src-tauri/src/undo.rs` (ReplaceChapters variant, apply/unapply logic)
  - `src-tauri/src/metadata/editor.rs` (validation for new fields)
  - `src-tauri/src/metadata/extraction.rs` (normalization for new fields)

### Frontend (TypeScript/React)
- **New files:**
  - `src-ui/src/api/chapters.ts`
  - `src-ui/src/components/ChapterEditor.tsx`
- **Modified:**
  - `src-ui/src/App.tsx` (added Chapter Editor section)
  - `src-ui/src/api/files.ts` (2 new functions)
  - `src-ui/src/api/audiobook.ts` (1 new function)
  - `src-ui/src/types/models.ts` (series/language fields)
  - `src-ui/src/components/MetadataEditor.tsx` (complete redesign with grouped sections)
  - `src-ui/src/context/AppContext.tsx` (undo/redo audiobook refresh)

## Testing

All code compiles successfully:
- Backend: `cargo check --all-features` passes ✓
- Frontend: ESLint passes for new files ✓
- Pre-existing warnings in UI components remain (not introduced by this work)

## Usage Examples

### Import Chapters from Audacity
1. Create chapters in Audacity using labels
2. Export as label track
3. Click "Import from File" in Chapter Editor
4. Select the .txt file → chapters auto-imported

### Extract Chapters from M4B
1. Load an M4B file that has embedded chapters
2. Click "Extract Chapters from Files"
3. Existing chapters are discovered and imported

### Clean Up Chapter Titles
1. After importing or generating chapters
2. Click a preset (e.g., "Remove numbered prefix")
3. Chapter titles instantly cleaned
4. Use Ctrl+Z to undo if needed

### Edit Rich Metadata
1. Navigate to "Edit Metadata" section
2. Fill in series name and book number
3. Add publisher, language, description
4. All fields saved and embedded in output M4B

### Undo/Redo Safety
1. Make any metadata or chapter changes
2. Press Ctrl+Z to undo
3. Press Ctrl+Shift+Z or Ctrl+Y to redo
4. UI updates immediately with rolled-back state

## Architecture Notes

### Chapter I/O Design
- Format detection via file extension with fallback
- Flexible time parsing (seconds, MM:SS, HH:MM:SS, HH:MM:SS.ms)
- Validation ensures chapters don't overlap or exceed duration
- Proper escaping for FFmetadata special characters

### Undo System Integration
- All bulk operations (import, generate, regex rename) are single undo steps
- Metadata updates tracked for full reversibility
- UndoManager properly injected via Tauri's `.manage()` API
- Frontend automatically refreshes after undo/redo

### Metadata Model Evolution
- Backward compatible (all new fields are `Option<T>`)
- Serialization handles missing fields gracefully
- Validation applies sensible length limits
- Normalization cleans whitespace and control characters

## Next Steps (Future Enhancements)

1. **Chapter preview before import**: Show parsed chapters in a dialog before applying
2. **Silence-based chapter detection**: Use FFmpeg `silencedetect` to suggest chapter boundaries
3. **Inline chapter editing**: Edit chapter titles/times directly in the list
4. **Drag-to-reorder**: Visual chapter reordering in the UI
5. **Metadata templates**: Save/load metadata field combinations
6. **Bulk metadata operations**: Apply same metadata to multiple projects

