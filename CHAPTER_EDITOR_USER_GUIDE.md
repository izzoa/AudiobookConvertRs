# Chapter & Metadata Editor User Guide

## Quick Start

### Accessing the Editors
1. Launch AudiobookConvertRs
2. Import audio files (Step 1)
3. **Metadata editing** is in "Step 2: Edit Metadata"
4. **Chapter editing** is in "Step 2b: Edit Chapters"

## Chapter Editor

### Viewing Current Chapters

The chapter list shows:
- Chapter number and title
- Start and end times (HH:MM:SS format)
- Total chapter count
- **Warnings** if chapters overlap or have gaps

### Importing Chapters

#### Supported Formats

**Audacity Label Track (.txt)**
```
0.000000	60.000000	Introduction
60.000000	180.000000	Chapter 1
180.000000	300.000000	Chapter 2
```

**FFmpeg Metadata (.ffmeta)**
```
;FFMETADATA1

[CHAPTER]
TIMEBASE=1/1000
START=0
END=60000
title=Introduction
```

**CSV (.csv)**
```
title,start,end
Introduction,00:00:00,00:01:00
Chapter 1,00:01:00,00:03:00
Chapter 2,00:03:00,00:05:00
```

#### Import Steps
1. Click **"Import from File"** button
2. Select your chapter file (.txt, .csv, or .ffmeta)
3. Chapters are automatically parsed and applied
4. Check the chapter list to verify
5. If something's wrong, press **Ctrl+Z** to undo

#### Tips
- CSV format accepts both `HH:MM:SS` and decimal seconds (e.g., `60.5`)
- Headers in CSV are optional (auto-detected)
- Empty lines and comments (# in Audacity) are skipped
- Import validates chapters don't exceed your audiobook duration

### Exporting Chapters

1. Choose export format:
   - **Audacity**: For audio editing workflows
   - **FFmeta**: For FFmpeg command-line use
   - **CSV**: For spreadsheet editing
2. Pick save location
3. File is created instantly

### Smart Generation

#### Extract from Source Metadata

**When to use:** Your source files already have chapter information

Click **"Extract Chapters from Files"** to automatically:
- Read QuickTime chapters from M4B files
- Read ID3 CHAP frames from MP3 files
- Read cue sheets from FLAC files
- Combine chapters from multiple files with correct time offsets

**What happens:**
- Old chapters (if any) are replaced (undo available)
- New chapters validated and applied
- Last chapter adjusted to match total duration

#### Title Cleanup (Regex Presets)

**When to use:** Chapter titles have unwanted prefixes/formatting

**Quick Presets:**
- **Remove numbered prefix**: `01 - Title` → `Title`
- **Remove "Track" prefix**: `Track 03 - Title` → `Title`
- **Replace underscores**: `Chapter_One` → `Chapter One`
- **Remove file extensions**: `Title.mp3` → `Title`
- **Clean multiple spaces**: `Title  Name` → `Title Name`
- **Remove "Chapter" prefix**: `Chapter 1: Title` → `Title`

**How to use:**
1. Click any preset button
2. Titles are instantly updated
3. Press Ctrl+Z if you don't like the result

**Custom Regex:**
1. Enter a regex pattern (e.g., `^\d+\.`)
2. Enter replacement text (often empty)
3. Click "Apply Custom Regex"
4. Preview in chapter list

**Examples:**
```
Pattern: ^\d+\s*-\s*     Replacement: (empty)
Before:  "01 - Introduction"
After:   "Introduction"

Pattern: _                Replacement: " "
Before:  "Chapter_One_Part_A"
After:   "Chapter One Part A"

Pattern: \s*\(.*?\)\s*$   Replacement: (empty)
Before:  "Chapter Title (Bonus)"
After:   "Chapter Title"
```

## Metadata Editor

### Field Organization

The metadata editor is organized into logical sections:

#### Core Information
- **Title**: Audiobook title
- **Author (Artist)**: Primary author/artist
- **Narrator**: Voice actor(s)
- **Album**: Album name
- **Album Artist**: Album-level artist
- **Genre**: Fiction, Non-Fiction, Mystery, etc.
- **Year**: Publication year
- **Language**: ISO code (en, es, fr, de, etc.)
- **Composer**: Music composer (if applicable)

#### Series
- **Series Name**: Name of the book series
- **Book #**: Position in series (1, 2, 3, etc.)

#### Publishing
- **Publisher**: Publishing company
- **Copyright**: Copyright information

#### Indexing
- **Track #** / **of**: Track number and total (for multi-disc sets)
- **Disc #** / **of**: Disc number and total

#### Notes & Description
- **Description**: Book synopsis or description (multi-line)
- **Comments**: Additional notes (multi-line)

### Editing Workflow

1. Fill in the fields you need (all are optional)
2. Click **"Save Metadata"**
3. Changes are applied to the audiobook
4. Press **Ctrl+Z** to undo if needed

### Integration with Open Library

The Open Library fetcher (above the manual fields) can auto-fill:
- Title, author, year, genre
- Publisher (if available)
- Cover artwork

**Workflow:**
1. Click "Prefill" to use current title/author as search
2. Click "Search"
3. Browse results
4. Click "Metadata" to apply basic info
5. Click "+ Cover" to also fetch cover art
6. Manually adjust any fields below

## Undo/Redo

### Keyboard Shortcuts
- **Ctrl+Z**: Undo last change
- **Ctrl+Shift+Z** or **Ctrl+Y**: Redo
- **Ctrl+S**: Save project (preserves undo history)
- **Ctrl+O**: Open project

### What Can Be Undone
- Metadata field changes
- Chapter imports
- Chapter generation from source
- Regex title cleanup
- Manual chapter edits (add/update/delete)

### Undo Stack
- Stores last **50 operations** (configurable)
- Each bulk operation (import, generate) is **one undo step**
- Undo history persists during session
- Cleared when closing project or app

### Visual Feedback
- Undo button shows tooltip: "Undo: [operation]"
- Redo button shows tooltip: "Redo: [operation]"
- UI updates immediately after undo/redo
- Chapter count and metadata fields reflect rolled-back state

## Common Workflows

### Workflow A: Podcast → Audiobook
1. Import 50 MP3 files (episodes)
2. Chapters auto-created from filenames
3. Filenames are "001_episode_title.mp3"
4. Click "Remove numbered prefix" → `episode_title`
5. Click "Replace underscores" → `episode title`
6. Edit metadata: add series, language, description
7. Convert to M4B

### Workflow B: FLAC with Cue Sheet → M4B
1. Import FLAC file with embedded cue sheet
2. Click "Extract Chapters from Files"
3. Cue sheet chapters imported automatically
4. Edit metadata (add narrator, series)
5. Convert to M4B with preserved chapters

### Workflow C: Audacity Integration
1. Edit audio in Audacity
2. Create labels for chapter breaks
3. File → Export → Export Labels
4. In AudiobookConvertRs: Import from File
5. Select the labels.txt file
6. Chapters appear in list
7. Convert

### Workflow D: Fixing Purchased M4B
1. Load existing M4B file
2. Click "Extract Chapters from Files" → chapters preserved
3. Add missing metadata:
   - Series: "The Great Series"
   - Series Index: 3
   - Publisher: "Audio Publishing Co"
   - Language: en
   - Description: "Book synopsis here..."
4. Re-encode with complete metadata

### Workflow E: Chapter Experimentation
1. Import chapters from CSV
2. Try "Remove Chapter prefix" → not quite right
3. Ctrl+Z
4. Try "Remove numbered prefix" → better
5. Try custom regex `^Part \d+: ` → perfect
6. Export to CSV for record keeping

## Troubleshooting

### Import Issues

**Problem:** "Import failed: Invalid start time on line 5"  
**Solution:** Check that line 5 has valid time format (numbers or HH:MM:SS)

**Problem:** "No chapters found in source files"  
**Solution:** Your source files don't have embedded chapters. Use "Auto Create Chapters" instead.

**Problem:** "Chapter exceeds total duration"  
**Solution:** Your imported chapters are longer than the audiobook. Edit the chapter file to fix times.

### Export Issues

**Problem:** Export button is disabled  
**Solution:** You need at least one chapter to export. Create or import chapters first.

### Regex Issues

**Problem:** "Invalid regex pattern"  
**Solution:** Your regex syntax is incorrect. Try a preset first, or check regex syntax online.

**Problem:** Regex didn't change anything  
**Solution:** Your pattern doesn't match any chapter titles. Check the pattern against actual titles.

### Undo Issues

**Problem:** Ctrl+Z doesn't work  
**Solution:** Make sure the app window has focus. Undo is limited to the current session.

**Problem:** Can't undo far enough back  
**Solution:** Undo stack is limited to 50 operations. Save often to preserve states.

## Advanced Tips

### Regex Power Users

**Chain multiple patterns** (each is one undo step):
1. Remove "Track" prefix
2. Remove numbered prefix  
3. Replace underscores
4. Clean multiple spaces
5. Result: perfectly formatted titles

**Common patterns:**
```regex
^\d+[-.\s]+          # Remove number at start
\s+\(\d+\)$          # Remove (1) at end
\[.*?\]              # Remove [tags]
Track\s*\d+\s*[-:]?  # Remove "Track 03 -"
```

### CSV Editing Workflow

For complex edits:
1. Export to CSV
2. Open in Excel/LibreOffice
3. Use spreadsheet formulas to bulk-edit titles
4. Find/replace with spreadsheet tools
5. Save CSV
6. Re-import to AudiobookConvertRs

### Metadata Templates (Manual)

Create your own templates:
1. Set up metadata for one book in a series
2. Save project with descriptive name
3. For next book: open that project
4. Change title and series index
5. Import new audio files
6. Convert (metadata template applied)

## Keyboard Shortcuts Summary

| Shortcut | Action |
|----------|--------|
| Ctrl+O | Open project |
| Ctrl+S | Save project |
| Ctrl+Z | Undo |
| Ctrl+Shift+Z | Redo |
| Ctrl+Y | Redo (alternative) |
| Escape | Cancel (dialogs) |

## Getting Help

### Built-in Help
- Hover over buttons for tooltips
- Field placeholders show expected format
- Warnings appear inline (red boxes)
- Error messages describe the issue

### Debug Information
If something goes wrong:
1. Check the chapter warnings (overlap/gap indicators)
2. Try exporting to CSV to see exact values
3. Use undo to revert to known-good state
4. Save project to preserve work before experimenting

