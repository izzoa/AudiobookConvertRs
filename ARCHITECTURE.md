# AudiobookConvertRs Architecture

## Overview

AudiobookConvertRs is a desktop application for converting audiobook files to M4B format. It's built using Rust for the backend and will use Tauri for the cross-platform GUI.

## High-Level Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                         Frontend (Tauri)                         │
│                    React/TypeScript UI Layer                     │
└─────────────────────────────────────────────────────────────────┘
                              │ IPC
┌─────────────────────────────────────────────────────────────────┐
│                      Backend (Rust/Tauri)                        │
│                                                                  │
│  ┌─────────┐  ┌──────────┐  ┌────────┐  ┌──────────────────┐   │
│  │ Audio   │  │ Metadata │  │ FFmpeg │  │    Conversion    │   │
│  │ Module  │  │  Module  │  │ Module │  │     Pipeline     │   │
│  └────┬────┘  └────┬─────┘  └───┬────┘  └────────┬─────────┘   │
│       │            │            │                │              │
│  ┌────┴────────────┴────────────┴────────────────┴─────────┐   │
│  │                     Core Services                        │   │
│  │  (State Management, Config, Error Handling, Logging)     │   │
│  └──────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
```

## Module Structure

### Core (`src-tauri/src/`)
- **lib.rs** - Main library entry point
- **main.rs** - Tauri application entry
- **error.rs** - Custom error types with `thiserror`
- **config.rs** - User settings and configuration
- **models.rs** - All data structures (AudioBook, Chapter, Metadata, etc.)
- **state.rs** - Thread-safe application state
- **logging.rs** - Structured logging with `tracing`

### Audio Module (`src-tauri/src/audio/`)
- **loader.rs** - Symphonia-based audio file loading
- **format.rs** - Format detection and validation
- **timing.rs** - Duration calculation and time utilities
- **batch.rs** - Batch file processing

### Metadata Module (`src-tauri/src/metadata/`)
- **extraction.rs** - Metadata extraction via Symphonia
- **artwork.rs** - Artwork extraction and processing
- **chapters.rs** - Chapter creation and editing
- **writing.rs** - M4B metadata writing via mp4ameta
- **editor.rs** - In-memory metadata editing API

### FFmpeg Module (`src-tauri/src/ffmpeg/`)
- **detection.rs** - FFmpeg installation detection
- **command_builder.rs** - FFmpeg command construction

### Conversion Module (`src-tauri/src/conversion/`)
- **pipeline.rs** - Conversion orchestration (Phase 5)

## Key Design Decisions

### 1. Symphonia for Audio Reading
- Pure Rust, no external dependencies
- Supports MP3, FLAC, WAV, OGG, AAC, M4A/M4B
- Handles metadata extraction and format probing

### 2. FFmpeg for Encoding
- Industry-standard encoder
- Used as subprocess (not linked)
- User must have FFmpeg installed

### 3. mp4ameta for M4B Metadata
- Pure Rust MP4/M4B metadata library
- Reads and writes metadata atoms
- **Note:** Chapters are read-only; writing via FFmpeg

### 4. Thread-Safe State
- `Arc<Mutex<AppState>>` for shared state
- Supports concurrent job execution
- Persistent configuration

## Data Flow

```
┌────────────┐     ┌───────────┐     ┌────────────┐     ┌──────────┐
│ Audio File │────▶│ Symphonia │────▶│ AudioFile  │────▶│ AudioBook│
│  (MP3/FLAC)│     │  Loading  │     │   Struct   │     │  Struct  │
└────────────┘     └───────────┘     └────────────┘     └──────────┘
                                                              │
                                                              ▼
┌────────────┐     ┌───────────┐     ┌────────────┐     ┌──────────┐
│ M4B Output │◀────│  FFmpeg   │◀────│ Conversion │◀────│ Settings │
│   File     │     │  Encoding │     │  Pipeline  │     │          │
└────────────┘     └───────────┘     └────────────┘     └──────────┘
                         │
                         ▼
                   ┌───────────┐
                   │ mp4ameta  │
                   │ Metadata  │
                   │  Writing  │
                   └───────────┘
```
