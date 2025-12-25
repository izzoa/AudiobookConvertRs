# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Initial release of AudiobookConvertRs
- Multi-format audio import (MP3, FLAC, M4A, WAV, OGG)
- M4B audiobook output with chapters
- Metadata editing (title, author, narrator, year, genre, publisher, description)
- Cover artwork support (drag-and-drop, file picker, Open Library fetch)
- Open Library integration for metadata lookup
- Chapter management (create, edit, reorder, delete)
- Auto-chapter generation (from files, by count, by duration, from source)
- Chapter regex renaming with presets
- Conversion queue for batch processing
- Real-time conversion progress tracking
- Project save/load functionality
- Conversion presets
- Undo/redo support for metadata changes
- Cross-platform support (Windows, macOS, Linux)
- Custom application icon

### Technical
- Rust backend with Tauri 2.0
- Symphonia for audio file reading
- FFmpeg for audio encoding
- mp4ameta for M4B metadata writing
- React + TypeScript frontend
- Tailwind CSS + Radix UI components

## [0.1.0] - 2025-12-25

### Added
- Initial development release
- Core conversion functionality
- Basic UI implementation
