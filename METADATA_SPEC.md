# Metadata Specification

This document describes how AudiobookConvertRs handles metadata across different audio formats.

## Metadata Fields

| Field | Description | Source Priority |
|-------|-------------|-----------------|
| Title | Audiobook/track title | ID3v2 > MP4 > Vorbis > ID3v1 |
| Artist | Author/performer | ID3v2 > MP4 > Vorbis > ID3v1 |
| Album | Audiobook title | ID3v2 > MP4 > Vorbis > ID3v1 |
| Album Artist | Author name | ID3v2 > MP4 > Vorbis |
| Narrator | Reader/narrator | Custom mapping |
| Composer | Book author | ID3v2 > MP4 > Vorbis |
| Year | Publication year | ID3v2 > MP4 > Vorbis > ID3v1 |
| Genre | Genre (usually "Audiobook") | ID3v2 > MP4 > Vorbis > ID3v1 |
| Comment | Description/notes | ID3v2 > MP4 > Vorbis > ID3v1 |
| Copyright | Copyright notice | ID3v2 > MP4 > Vorbis |
| Publisher | Publishing house | ID3v2 > MP4 |
| Track Number | Part number | ID3v2 > MP4 > Vorbis |
| Total Tracks | Total parts | ID3v2 > MP4 > Vorbis |
| Disc Number | Disc/volume number | ID3v2 > MP4 > Vorbis |
| Artwork | Cover image | Embedded > External file |

## Format-Specific Handling

### ID3 Tags (MP3)

**ID3v2** (preferred):
- Located at the beginning of the file
- Supports Unicode (UTF-8, UTF-16)
- Common frames: TIT2 (title), TPE1 (artist), TALB (album), APIC (artwork)

**ID3v1** (fallback):
- Located at the end of the file
- ASCII only, 30-character limits
- Basic fields only

### Vorbis Comments (FLAC, OGG)

- Key-value pairs with uppercase keys
- UTF-8 encoded
- Common tags: TITLE, ARTIST, ALBUM, DATE
- Artwork in METADATA_BLOCK_PICTURE

### MP4 Atoms (M4A, M4B)

Using mp4ameta library to read/write:
- ©nam (title)
- ©ART (artist)
- ©alb (album)
- aART (album artist)
- ©wrt (composer)
- ©day (year)
- ©gen (genre)
- ©cmt (comment)
- cprt (copyright)
- covr (artwork)

## Chapter Handling

### Chapter Structure
```rust
struct Chapter {
    index: usize,       // 0-based position
    title: String,      // Display name
    start_time_ms: u64, // Start in milliseconds
    end_time_ms: u64,   // End in milliseconds
    auto_detected: bool // Created from file boundaries
}
```

### Chapter Sources

1. **File Boundaries** - Each source file becomes a chapter
2. **Auto-Split** - Divide by time into N equal chapters
3. **Existing M4B** - Extract QuickTime chapters from M4B files

### M4B Chapter Writing

**Note:** mp4ameta 0.13.0 does not support writing chapters directly.

Chapters are embedded via FFmpeg using the FFmetadata format:

```ini
;FFMETADATA1
[CHAPTER]
TIMEBASE=1/1000
START=0
END=60000
title=Chapter 1

[CHAPTER]
TIMEBASE=1/1000
START=60000
END=120000
title=Chapter 2
```

## Artwork Handling

### Embedded Artwork
- Extracted from audio file metadata
- Supported formats: JPEG, PNG, GIF, BMP

### External Artwork
Search order in source directory:
1. cover.jpg / cover.jpeg / cover.png
2. folder.jpg / folder.jpeg / folder.png
3. artwork.jpg / artwork.png
4. album.jpg / album.png
5. front.jpg / front.png

### Artwork Processing
- Resize if larger than 500x500 pixels
- Convert to JPEG for M4B embedding
- Maintain aspect ratio

## Metadata Normalization

All extracted metadata is normalized:
- Trim leading/trailing whitespace
- Remove null characters and control chars
- Convert encoding to UTF-8
- Empty strings become `None`
