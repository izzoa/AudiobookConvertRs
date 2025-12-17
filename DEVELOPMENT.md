# Development Guide

This guide helps you set up a development environment for AudiobookConvertRs.

## Prerequisites

### Required
- **Rust** 1.75+ (install via [rustup](https://rustup.rs/))
- **Node.js** 18+ (for frontend)
- **FFmpeg** 4.0+ (for audio conversion)

### Platform-Specific

**macOS:**
```bash
# Install FFmpeg
brew install ffmpeg

# Rust (if not installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

**Linux (Ubuntu/Debian):**
```bash
# Install dependencies
sudo apt-get update
sudo apt-get install -y \
  ffmpeg \
  build-essential \
  libgtk-3-dev \
  libwebkit2gtk-4.0-dev \
  libappindicator3-dev \
  librsvg2-dev \
  patchelf

# Rust (if not installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

**Windows:**
1. Install [Rust](https://rustup.rs/)
2. Install [FFmpeg](https://ffmpeg.org/download.html) and add to PATH
3. Install Visual Studio Build Tools (C++ workload)

## Project Setup

```bash
# Clone the repository
git clone https://github.com/yourusername/AudiobookConvertRs.git
cd AudiobookConvertRs

# Build Rust backend
cd src-tauri
cargo build

# Run tests
cargo test
```

## Project Structure

```
AudiobookConvertRs/
├── src-tauri/           # Rust backend
│   ├── src/
│   │   ├── audio/       # Audio loading/format detection
│   │   ├── metadata/    # Metadata extraction/writing
│   │   ├── ffmpeg/      # FFmpeg integration
│   │   ├── conversion/  # Conversion pipeline
│   │   └── commands/    # Tauri IPC commands
│   └── Cargo.toml
├── src-ui/              # Frontend (future)
├── tests/               # Integration tests
└── docs/                # Documentation
```

## Development Workflow

### Running Tests
```bash
cd src-tauri
cargo test           # Run all tests
cargo test --lib     # Run only library tests
cargo test -- --nocapture  # Show test output
```

### Checking Code
```bash
cargo check          # Fast syntax check
cargo clippy         # Linter
cargo fmt            # Format code
```

### Building
```bash
cargo build          # Debug build
cargo build --release  # Release build
```

## Key Modules

| Module | Purpose |
|--------|---------|
| `audio::loader` | Load audio files via Symphonia |
| `audio::format` | Format detection and validation |
| `metadata::extraction` | Extract metadata from files |
| `metadata::chapters` | Chapter management |
| `metadata::writing` | Write metadata to M4B |
| `ffmpeg::detection` | Detect FFmpeg installation |
| `ffmpeg::command_builder` | Build FFmpeg commands |

## Adding New Features

1. Create a new branch: `git checkout -b feature/my-feature`
2. Write tests first (TDD recommended)
3. Implement the feature
4. Run `cargo test` and `cargo clippy`
5. Submit a pull request

## Common Issues

### FFmpeg Not Found
Ensure FFmpeg is installed and in your PATH:
```bash
ffmpeg -version
```

### Build Fails on Linux
Install missing dependencies:
```bash
sudo apt-get install libgtk-3-dev libwebkit2gtk-4.0-dev
```

### Tests Fail
Make sure you have audio test fixtures or run with:
```bash
cargo test --lib  # Skip integration tests
```

## Code Style

- Use `rustfmt` for formatting
- Follow Rust naming conventions
- Document public APIs with doc comments
- Write tests for new functionality
