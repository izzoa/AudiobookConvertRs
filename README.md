<p align="center">
  <img src="src-tauri/icons/icon.png" alt="AudiobookConvertRs Logo" width="128" height="128">
</p>

<h1 align="center">AudiobookConvertRs</h1>

<p align="center">
  <strong>A modern, cross-platform audiobook converter built with Rust and Tauri</strong>
</p>

<p align="center">
  <a href="#features">Features</a> •
  <a href="#installation">Installation</a> •
  <a href="#usage">Usage</a> •
  <a href="#building-from-source">Building</a> •
  <a href="#contributing">Contributing</a> •
  <a href="#license">License</a>
</p>

<p align="center">
  <img src="https://img.shields.io/badge/rust-1.75+-orange.svg" alt="Rust 1.75+">
  <img src="https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux-blue.svg" alt="Platform">
  <img src="https://img.shields.io/badge/license-MIT-green.svg" alt="License MIT">
</p>

---

> ⚠️ **ALPHA STATUS**: This project is in early alpha development. Features may be incomplete, APIs may change, and bugs are expected. Contributions and PRs are very welcome!

---

## Overview

AudiobookConvertRs converts your audio files (MP3, FLAC, M4A, WAV, OGG) into the M4B audiobook format with full support for chapters, metadata, and cover artwork. Built with Rust for performance and Tauri for a lightweight, native UI experience.

This project is a modern Rust rewrite inspired by [AudioBookConverter](https://github.com/yermak/AudioBookConverter) (Java), aiming to provide similar functionality with improved performance and a smaller footprint.

## Comparison with AudioBookConverter (Java)

AudiobookConvertRs is inspired by the excellent [AudioBookConverter](https://github.com/yermak/AudioBookConverter) by @yermak. Here's how they compare:

| Feature | AudiobookConvertRs (Rust) | AudioBookConverter (Java) |
|---------|---------------------------|---------------------------|
| **Language** | Rust + TypeScript | Java |
| **GUI Framework** | Tauri 2.0 (web-based) | JavaFX |
| **Binary Size** | ~15-25 MB | ~80-100 MB (with JRE) |
| **Memory Usage** | Lower (no JVM overhead) | Higher (JVM required) |
| **Startup Time** | Fast (native binary) | Slower (JVM startup) |
| **Installation** | Standalone installer | Requires Java or bundled JRE |
| **License** | MIT | GPL-2.0 |

### Feature Comparison

| Feature | AudiobookConvertRs | AudioBookConverter |
|---------|:------------------:|:------------------:|
| MP3/FLAC/M4A/WAV input | ✅ | ✅ |
| M4B output | ✅ | ✅ |
| OGG/Opus output | ❌ | ✅ |
| MP3 (LAME) output | ❌ | ✅ |
| Chapter management | ✅ | ✅ |
| Metadata editing | ✅ | ✅ |
| Cover artwork | ✅ | ✅ |
| Conversion queue | ✅ | ✅ |
| Parallel encoding | ✅ | ✅ |
| Open Library integration | ✅ | ❌ |
| Project save/load | ✅ | ❌ |
| Undo/redo | ✅ | ❌ |
| FLAC/CUE support | ❌ | ✅ |
| WMA input | ❌ | ✅ |
| Instant playback preview | ❌ | ✅ |
| Steam distribution | ❌ | ✅ |
| Windows | ✅ | ✅ |
| macOS | ✅ | ⚠️ Experimental |
| Linux | ✅ | ⚠️ Experimental |

### Why Choose AudiobookConvertRs?

- **Lightweight** - No Java runtime required, smaller download and memory footprint
- **Fast startup** - Native binary starts instantly
- **Modern UI** - Clean, responsive web-based interface
- **MIT License** - More permissive for integration and modification
- **Open Library** - Fetch metadata and covers from Open Library API
- **Project files** - Save and resume your work later

### Why Choose AudioBookConverter (Java)?

- **More output formats** - Supports OGG/Opus and MP3 output
- **More input formats** - WMA and FLAC/CUE support
- **Audio preview** - Listen before converting
- **Mature & stable** - 8+ years of development, 100+ releases
- **Steam distribution** - Easy installation via Steam

## Features

- **Multi-format Support** - Import MP3, FLAC, M4A, WAV, and OGG files
- **Chapter Management** - Create, edit, reorder, and auto-generate chapters
- **Metadata Editing** - Edit title, author, narrator, year, genre, and more
- **Cover Artwork** - Drag-and-drop, file picker, or fetch from Open Library
- **Open Library Integration** - Search and apply metadata from Open Library API
- **Conversion Queue** - Queue multiple audiobooks for batch conversion
- **Real-time Progress** - Live progress tracking during conversion
- **Project Management** - Save and load projects for later editing
- **Conversion Presets** - Save your preferred conversion settings
- **Undo/Redo** - Full undo/redo support for metadata changes
- **Cross-platform** - Runs on Windows, macOS, and Linux

## Screenshots

<!-- Add screenshots here -->
*Coming soon*

## Installation

### Prerequisites

**FFmpeg is required** for audio encoding. Install it for your platform:

<details>
<summary><strong>macOS</strong></summary>

```bash
brew install ffmpeg
```
</details>

<details>
<summary><strong>Windows</strong></summary>

1. Download FFmpeg from [ffmpeg.org](https://ffmpeg.org/download.html)
2. Extract to a folder (e.g., `C:\ffmpeg`)
3. Add the `bin` folder to your PATH environment variable
</details>

<details>
<summary><strong>Linux (Ubuntu/Debian)</strong></summary>

```bash
sudo apt update
sudo apt install ffmpeg
```
</details>

<details>
<summary><strong>Linux (Fedora)</strong></summary>

```bash
sudo dnf install ffmpeg
```
</details>

### Download

Download the latest release for your platform from the [Releases](https://github.com/yourusername/AudiobookConvertRs/releases) page:

| Platform | Download |
|----------|----------|
| Windows | `AudiobookConvertRs_x.x.x_x64-setup.exe` |
| macOS | `AudiobookConvertRs_x.x.x_aarch64.dmg` (Apple Silicon) or `_x64.dmg` (Intel) |
| Linux | `AudiobookConvertRs_x.x.x_amd64.deb` |

## Usage

### Quick Start

1. **Import Audio Files** - Click "Add Files" or drag-and-drop audio files
2. **Edit Metadata** - Fill in title, author, and other metadata
3. **Manage Chapters** - Auto-generate or manually create chapters
4. **Add Cover Art** - Upload an image or fetch from Open Library
5. **Convert** - Choose output location and start conversion

### Keyboard Shortcuts

| Action | Shortcut |
|--------|----------|
| Undo | `Ctrl/Cmd + Z` |
| Redo | `Ctrl/Cmd + Shift + Z` |
| Save Project | `Ctrl/Cmd + S` |
| Open Project | `Ctrl/Cmd + O` |

### Chapter Generation Options

- **From Files** - Create one chapter per input file
- **By Count** - Split into N equal chapters
- **By Duration** - Create chapters of X minutes each
- **From Source** - Import chapters from existing M4B files

## Building from Source

### Prerequisites

- [Rust](https://rustup.rs/) 1.75 or later
- [Node.js](https://nodejs.org/) 18 or later
- [FFmpeg](https://ffmpeg.org/) installed and in PATH

### Build Steps

```bash
# Clone the repository
git clone https://github.com/yourusername/AudiobookConvertRs.git
cd AudiobookConvertRs

# Install frontend dependencies
cd src-ui
npm install
cd ..

# Build and run in development mode
cargo tauri dev

# Build for production
cargo tauri build
```

The built application will be in `src-tauri/target/release/bundle/`.

## Architecture

```
AudiobookConvertRs/
├── src-tauri/           # Rust backend
│   ├── src/
│   │   ├── audio/       # Audio file loading (Symphonia)
│   │   ├── metadata/    # Metadata extraction/writing
│   │   ├── ffmpeg/      # FFmpeg integration
│   │   ├── conversion/  # Conversion pipeline
│   │   ├── commands/    # Tauri IPC commands
│   │   └── ...
│   └── icons/           # Application icons
├── src-ui/              # React/TypeScript frontend
│   ├── src/
│   │   ├── components/  # UI components
│   │   ├── api/         # Backend API wrappers
│   │   └── ...
│   └── ...
└── ...
```

### Technology Stack

| Component | Technology |
|-----------|------------|
| Backend | Rust |
| Frontend | React + TypeScript |
| Desktop Framework | Tauri 2.0 |
| Audio Reading | Symphonia |
| Audio Encoding | FFmpeg |
| M4B Metadata | mp4ameta |
| Styling | Tailwind CSS |
| UI Components | Radix UI |

## Configuration

Settings are stored in the platform-specific config directory:

| Platform | Location |
|----------|----------|
| Windows | `%APPDATA%\AudiobookConvertRs\` |
| macOS | `~/Library/Application Support/AudiobookConvertRs/` |
| Linux | `~/.config/AudiobookConvertRs/` |

## Contributing

Contributions are welcome! Please read our contributing guidelines before submitting a PR.

### Development Setup

1. Fork the repository
2. Create a feature branch: `git checkout -b feature/amazing-feature`
3. Make your changes
4. Run tests: `cargo test`
5. Commit: `git commit -m 'Add amazing feature'`
6. Push: `git push origin feature/amazing-feature`
7. Open a Pull Request

### Code Style

- Rust: Follow standard Rust conventions (`cargo fmt`)
- TypeScript: ESLint configuration included
- Commits: Use conventional commit messages

## Roadmap

See [PARITY.md](PARITY.md) for detailed implementation plans.

### High Priority
- [ ] **Audio Preview/Playback** - Listen to audio before converting
- [ ] **Smart Artwork Discovery** - Auto-detect cover images from file directories

### Medium Priority
- [ ] **MP3 Output Format** - Export to MP3 with chapter support via ID3v2
- [ ] **OGG/Opus Output Format** - Export to open Opus format

### Lower Priority
- [ ] **FLAC/CUE Sheet Support** - Import FLAC with CUE, map tracks to chapters
- [ ] **WMA Input Support** - Import Windows Media Audio files

### Polish & Enhancements
- [ ] **Quality Management UI** - Bitrate slider with file size estimation
- [ ] Audio normalization options
- [ ] Additional metadata providers (Google Books, Audible)
- [ ] Localization support

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Acknowledgments

- [Symphonia](https://github.com/pdeljanov/Symphonia) - Pure Rust audio decoding
- [Tauri](https://tauri.app/) - Desktop application framework
- [mp4ameta](https://github.com/Saecki/mp4ameta) - MP4 metadata library
- [FFmpeg](https://ffmpeg.org/) - Audio/video processing
- [Open Library](https://openlibrary.org/) - Book metadata API

## Support

- **Issues**: [GitHub Issues](https://github.com/yourusername/AudiobookConvertRs/issues)
- **Discussions**: [GitHub Discussions](https://github.com/yourusername/AudiobookConvertRs/discussions)

---

<p align="center">
  Made with ❤️ and Rust
</p>
