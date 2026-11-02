# Contributing to AudiobookConvertRs

Thank you for your interest in contributing to AudiobookConvertRs! This document provides guidelines and information for contributors.

## Code of Conduct

Please be respectful and constructive in all interactions. We welcome contributors of all experience levels.

## How to Contribute

### Reporting Bugs

1. Check if the bug has already been reported in [Issues](https://github.com/yourusername/AudiobookConvertRs/issues)
2. If not, create a new issue with:
   - Clear, descriptive title
   - Steps to reproduce
   - Expected vs actual behavior
   - System information (OS, FFmpeg version)
   - Relevant logs or screenshots

### Suggesting Features

1. Check existing issues and discussions for similar suggestions
2. Open a new issue with the "enhancement" label
3. Describe the feature and its use case
4. Be open to feedback and discussion

### Pull Requests

1. Fork the repository
2. Create a feature branch from `main`:
   ```bash
   git checkout -b feature/your-feature-name
   ```
3. Make your changes
4. Ensure tests pass:
   ```bash
   cargo test
   cd src-ui && npm run lint && npm run build
   ```
5. Commit with clear messages (see below)
6. Push to your fork
7. Open a Pull Request against `main`

## Development Setup

### Prerequisites

- Rust 1.85+ (`rustup update stable`)
- Node.js 18+ and npm
- FFmpeg installed and in PATH

### Getting Started

```bash
# Clone your fork
git clone https://github.com/YOUR_USERNAME/AudiobookConvertRs.git
cd AudiobookConvertRs

# Install frontend dependencies
cd src-ui
npm install
cd ..

# Run in development mode
cargo tauri dev
```

### Project Structure

```
AudiobookConvertRs/
├── src-tauri/           # Rust backend
│   ├── src/
│   │   ├── audio/       # Audio file loading
│   │   ├── metadata/    # Metadata extraction/writing
│   │   ├── ffmpeg/      # FFmpeg integration
│   │   ├── conversion/  # Conversion pipeline
│   │   ├── commands/    # Tauri IPC commands
│   │   ├── models.rs    # Data structures
│   │   ├── state.rs     # App state management
│   │   └── error.rs     # Error types
│   └── Cargo.toml
├── src-ui/              # React frontend
│   ├── src/
│   │   ├── components/  # UI components
│   │   ├── api/         # Backend API wrappers
│   │   ├── context/     # React context
│   │   └── types/       # TypeScript types
│   └── package.json
└── Cargo.toml           # Workspace config
```

## Coding Standards

### Rust

- Follow standard Rust conventions
- Run `cargo fmt` before committing
- Run `cargo clippy` and address warnings
- Add doc comments for public APIs
- Write tests for new functionality

```rust
/// Loads an audio file and extracts metadata.
///
/// # Arguments
/// * `path` - Path to the audio file
///
/// # Returns
/// * `Ok(AudioFile)` - Successfully loaded file
/// * `Err(AppError)` - If loading fails
pub fn load_audio_file(path: &Utf8Path) -> Result<AudioFile, AppError> {
    // Implementation
}
```

### TypeScript

- Follow the existing ESLint configuration
- Use TypeScript types (avoid `any`)
- Use functional components with hooks
- Keep components focused and composable

### Commit Messages

Use conventional commit format:

```
type(scope): description

[optional body]

[optional footer]
```

Types:
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation changes
- `style`: Code style changes (formatting)
- `refactor`: Code refactoring
- `test`: Adding or updating tests
- `chore`: Maintenance tasks

Examples:
```
feat(chapters): add auto-split by duration
fix(conversion): handle special characters in filenames
docs(readme): add installation instructions for Linux
```

## Testing

### Running Tests

```bash
# Run all Rust tests
cargo test

# Run specific test
cargo test test_name

# Run with output
cargo test -- --nocapture

# Frontend lint check
cd src-ui && npm run lint
```

### Writing Tests

- Place unit tests in the same file as the code
- Place integration tests in `tests/`
- Use descriptive test names
- Test both success and error cases

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_duration_valid() {
        let result = parse_duration("01:30:45");
        assert_eq!(result, Ok(5445000)); // milliseconds
    }

    #[test]
    fn test_parse_duration_invalid() {
        let result = parse_duration("invalid");
        assert!(result.is_err());
    }
}
```

## Architecture Guidelines

### Backend (Rust)

1. **Error Handling**: Use `thiserror` for custom errors, convert external errors
2. **State Management**: Use `AppState` for shared state, avoid global state
3. **Async**: Use Tokio for async operations, keep Tauri commands async
4. **Logging**: Use `tracing` macros (`info!`, `debug!`, `error!`)

### Frontend (React)

1. **State**: Use React Context for global state, local state for components
2. **API Calls**: All backend calls go through `src/api/` wrappers
3. **Types**: Define types in `src/types/`, keep them in sync with Rust models
4. **Components**: Keep UI components in `src/components/ui/`, feature components at top level

### IPC Communication

- Tauri commands should validate inputs
- Return `Result<T, String>` for error handling
- Update and return state for sync operations
- Use events for progress reporting

## Release Process

1. Update version in `Cargo.toml` and `tauri.conf.json`
2. Update CHANGELOG.md
3. Create a git tag: `git tag v0.x.x`
4. Push tag: `git push origin v0.x.x`
5. GitHub Actions builds and publishes the release

## Getting Help

- Check existing documentation in `/docs`
- Search closed issues for similar problems
- Ask in GitHub Discussions
- Open an issue with the "question" label

## Recognition

Contributors will be recognized in:
- GitHub contributors list
- Release notes for significant contributions
- README acknowledgments for major features

Thank you for contributing!
