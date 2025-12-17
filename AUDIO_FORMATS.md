# Supported Audio Formats

AudiobookConvertRs uses [Symphonia](https://github.com/pdeljanov/Symphonia) for audio format handling.

## Input Formats

| Format | Extension | Codec | Metadata | Notes |
|--------|-----------|-------|----------|-------|
| MP3 | `.mp3` | MPEG-1/2 Layer III | ID3v1, ID3v2 | Most common audiobook format |
| FLAC | `.flac` | FLAC | Vorbis Comments | Lossless, maintains quality |
| M4A | `.m4a` | AAC | MP4 atoms | Apple format |
| M4B | `.m4b` | AAC | MP4 atoms, chapters | Apple audiobook format |
| WAV | `.wav` | PCM | BWF, RIFF INFO | Uncompressed audio |
| OGG | `.ogg`, `.oga` | Vorbis | Vorbis Comments | Open format |
| WMA | `.wma` | WMA | ASF metadata | Windows Media Audio |
| AAC | `.aac` | AAC | Limited | Raw AAC stream |

## Output Format

**M4B (MPEG-4 Audiobook)** is the only supported output format:
- Container: MP4 (MPEG-4 Part 14)
- Audio codec: AAC (Advanced Audio Coding)
- Chapter support: QuickTime chapter track
- Metadata: MP4 atoms (iTunes-style)
- Preferred by: iOS, iTunes, Apple Books, many audiobook players

## Format Detection

Format detection uses a multi-layered approach:

1. **Magic Bytes** - Check file signature bytes
2. **Symphonia Probe** - Authoritative format detection
3. **Extension Fallback** - Use file extension if ambiguous

### Magic Bytes Reference

| Format | Magic Bytes (hex) |
|--------|-------------------|
| MP3 (ID3) | `49 44 33` (ID3) |
| MP3 (sync) | `FF FB` or `FF FA` |
| FLAC | `66 4C 61 43` (fLaC) |
| M4A/M4B | `00 00 00 xx 66 74 79 70` (ftyp) |
| WAV | `52 49 46 46` (RIFF) |
| OGG | `4F 67 67 53` (OggS) |

## Bitrate Options

Supported output bitrates:
- 32 kbps - Low quality, very small files
- 64 kbps - Acceptable for speech
- 96 kbps - Good quality for audiobooks
- **128 kbps** - Default, recommended
- 192 kbps - High quality
- 256 kbps - Very high quality
- 320 kbps - Maximum quality

## Sample Rate Options

Supported sample rates:
- 22050 Hz - Low (saves space)
- **44100 Hz** - Default, CD quality
- 48000 Hz - DVD/professional quality
