# Development Guide

This guide helps you set up a development environment for AudiobookConvertRs.

## Prerequisites

### Required
- **Rust** 1.85+ (install via [rustup](https://rustup.rs/))
- **Node.js** 22+ (for frontend)
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
  libwebkit2gtk-4.1-dev \
  libayatana-appindicator3-dev \
  librsvg2-dev \
  patchelf \
  libxdo-dev \
  libssl-dev

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

## Running the Application (Dev Mode)

To start the application with both Frontend and Backend in development mode:

1.  Navigate to `src-ui` and install dependencies:
    ```bash
    cd src-ui
    npm install
    cd ..
    ```
2.  Run the development server from the root directory:
    ```bash
    ./src-ui/node_modules/.bin/tauri dev
    ```
    *Or if you have `tauri-cli` installed globally (`cargo install tauri-cli`):*
    ```bash
    cargo tauri dev
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

## Creating Releases

For detailed instructions on creating and publishing releases, see **[DEPLOY.md](DEPLOY.md)**.

Quick summary:
1. Update versions in `Cargo.toml` and `src-tauri/tauri.conf.json`
2. Commit and create a tag: `git tag vX.Y.Z`
3. Push tag: `git push origin vX.Y.Z`
4. GitHub Actions automatically builds installers for macOS, Windows, and Linux

## Common Issues

### FFmpeg Not Found
Ensure FFmpeg is installed and in your PATH:
```bash
ffmpeg -version
```

### Build Fails on Linux
Install missing dependencies:
```bash
sudo apt-get install libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev patchelf
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
