# Dependencies

This document lists all crate dependencies and their purposes.

## Core Dependencies

| Crate | Version | Purpose | License |
|-------|---------|---------|---------|
| `symphonia` | 0.5 | Audio format decoding and metadata | MPL-2.0 |
| `symphonia-bundle-mp3` | 0.5 | MP3 decoder | MPL-2.0 |
| `symphonia-bundle-flac` | 0.5 | FLAC decoder | MPL-2.0 |
| `mp4ameta` | 0.13 | M4A/M4B metadata read/write | MIT |
| `image` | 0.25 | Image processing for artwork | MIT |

## Tauri & Frontend

| Crate | Version | Purpose | License |
|-------|---------|---------|---------|
| `tauri` | 2.x | Cross-platform GUI framework | MIT/Apache-2.0 |
| `tauri-build` | 2.x | Tauri build script | MIT/Apache-2.0 |

## Utility Crates

| Crate | Version | Purpose | License |
|-------|---------|---------|---------|
| `serde` | 1.0 | Serialization framework | MIT/Apache-2.0 |
| `serde_json` | 1.0 | JSON serialization | MIT/Apache-2.0 |
| `thiserror` | 1.0 | Error derive macros | MIT/Apache-2.0 |
| `anyhow` | 1.0 | Flexible error handling | MIT/Apache-2.0 |
| `tracing` | 0.1 | Structured logging | MIT |
| `tracing-subscriber` | 0.3 | Log formatting | MIT |
| `camino` | 1.1 | UTF-8 path handling | MIT/Apache-2.0 |
| `uuid` | 1.0 | UUID generation | MIT/Apache-2.0 |
| `chrono` | 0.4 | Date/time handling | MIT/Apache-2.0 |
| `dirs` | 5.0 | Platform directory paths | MIT/Apache-2.0 |
| `base64` | 0.22 | Base64 encoding | MIT/Apache-2.0 |
| `regex` | 1.10 | Regular expressions | MIT/Apache-2.0 |
| `which` | 6.0 | Executable path finding | MIT |

## Symphonia Features

The following Symphonia features are enabled:

```toml
[dependencies.symphonia]
version = "0.5"
features = [
    "aac",      # AAC decoder
    "flac",     # FLAC decoder  
    "isomp4",   # MP4/M4A container
    "mp3",      # MP3 decoder
    "ogg",      # OGG container
    "vorbis",   # Vorbis decoder
    "wav",      # WAV decoder
]
```

## External Dependencies

| Tool | Version | Purpose | Required |
|------|---------|---------|----------|
| FFmpeg | 4.0+ | Audio encoding | Yes (runtime) |

## License Compatibility

All dependencies use permissive licenses (MIT, Apache-2.0, MPL-2.0) compatible with both open-source and commercial use.

## Version Pinning

Major versions are pinned in `Cargo.toml` to ensure stability:
- Use `^1.0` for stable APIs
- Use exact versions for critical dependencies

## Security Considerations

- All crate dependencies are from crates.io
- FFmpeg is executed as a subprocess (not linked)
- No network dependencies for core functionality
