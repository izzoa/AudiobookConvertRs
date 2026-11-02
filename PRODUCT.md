# Product

<!-- impeccable:product-schema 1 -->

## Platform

web

## Users

**Primary: the self-hosted library curator.** They run a self-hosted audiobook server (Audiobookshelf or Plex are the typical examples; which one is not confirmed) and feed it books in batches. They care about consistent tags, file naming and series data across the whole library, not just one book looking right.

No other audience has been confirmed.

## Product Purpose

AudiobookConvertRs turns loose audio files, or a whole folder of them, into a single M4B audiobook with chapters, metadata and cover art, ready to drop into a self-hosted library.

It exists because a curator's source material is messy: per-chapter MP3s, CD rips split across disc folders, inconsistent or missing tags. Success is a book that arrives in the server correctly tagged, correctly ordered and correctly chaptered, without hand repair afterwards.

## Positioning

The lightweight, native alternative to [AudioBookConverter](https://github.com/yermak/AudioBookConverter) (Java, by yermak), which inspired it. Its claim is no Java runtime, a smaller footprint, fast startup, and first-class support on macOS and Linux as well as Windows, where the Java app lists macOS and Linux as experimental.

The README's figures (binary ~15–25 MB versus ~80–100 MB, lower memory, faster startup) are estimates that have not been measured. Treat them as unverified until benchmarked.

## Operating Context

- Desktop app for Windows, macOS and Linux, built with Tauri 2 (Rust backend, React web UI in a native window). There is no mobile or browser-hosted version.
- FFmpeg must already be installed on the user's machine. The app detects it on PATH and in common locations and uses it for encoding.
- Input arrives as individual files or folders, and folders are walked recursively. Multi-disc books in `CD1/`, `CD2/` subfolders are an expected shape, not an edge case.
- Work can be saved as a project file (`.abcproj`) and reopened later. Recent projects are remembered.
- Metadata and covers can be looked up from Open Library.
- Conversions run through a persistent queue with parallel workers; jobs survive an app restart.

## Capabilities and Constraints

**Confirmed functionality**
- Input: MP3, FLAC, M4A, M4B, WAV, OGG/OGA, AAC. This is the backend's `SUPPORTED_EXTENSIONS`, and it is the source of truth.
- Not supported as input: WMA, AAX/AAXC, Opus.
- Output: M4B only. The Java app's OGG/Opus and MP3 outputs are not available.
- Chapters: generated from file boundaries (named from file stems), imported from CUE sheets, Audacity labels, FFmetadata or CSV, extracted from existing chapter marks, renamed by regex, reordered by drag.
- Metadata: read from source tags (ID3v2, Vorbis, MP4), edited per book, including narrator, series and series index, with undo/redo.
- Artwork: discovered from embedded art or images beside the files, picked from disk, or fetched from Open Library.
- Audio: optional loudness normalization (LUFS, true peak, LRA), bitrate, VBR/CBR, saved conversion presets, and in-app playback preview.

**Current state**
- Alpha. Features may be incomplete and bugs are expected (README).
- MIT licensed (README).
- FFmpeg is user-installed today and not bundled (README, DEPLOY.md).
- Releases are built by GitHub Actions on version tags (NSIS `.exe`, `.dmg`, `.deb`).

**Open decisions.** The user named none of these as firm commitments, so none are settled:
- Whether the app will ever handle DRM-protected formats (e.g. Audible AAX).
- Whether FFmpeg stays user-installed or becomes bundled.
- Whether the product stays free with no accounts or telemetry.
- Whether it ships publicly to strangers or remains a personal tool.

**Terminology:** *book* (one audiobook being assembled), *project* (a saved book plus its settings), *job* (one queued conversion), *queue*, *chapter*, *preset*.

## Brand Commitments

- Name: **AudiobookConvertRs**.
- Existing icon: `src-tauri/icons/icon.png`, with platform variants in `src-tauri/icons/`.
- The README credits yermak's AudioBookConverter as the inspiration.

No voice, personality or identity rules have been confirmed beyond these.

## Evidence on Hand

- Test fixtures: `src-tauri/tests/fixtures/minimal.m4b`, plus the FFmpeg-generated MP3 tests in `src-tauri/tests/mp3_import_e2e.rs`.
- Feature comparison with the Java app: `README.md`, `FEATURE_COMPARISON.md`, `PARITY.md`.
- **Absent:** screenshots (the README says "coming soon"), user counts, testimonials, reviews, press, and measured benchmarks. Future work must not fabricate any of these.

## Product Principles

1. **Batch is the normal case.** A curator processes many books. Nothing should assume one book is the whole session.
2. **Tags are the product.** The output is judged by how it lands in the server: title, author, narrator, series, chapters and cover, consistent across the library. Silently losing metadata is a severe defect, not a cosmetic one.
3. **Predictable over clever.** Ordering, naming and chapter boundaries must come out the same way every time. A multi-disc folder keeps disc order, and the same input gives the same book.
4. **Earn "lightweight."** Small footprint, fast startup and a native feel on all three platforms are the claim against the Java app. Anything that compromises them has to justify itself.
