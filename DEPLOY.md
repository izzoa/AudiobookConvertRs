# Deployment Guide

This guide explains how to create and publish releases for AudiobookConvertRs.

## Automated Release Pipeline

The project uses GitHub Actions to automatically build installers for all platforms when you push a version tag.

### Release Artifacts

Each release automatically generates:
- **macOS**: `.dmg` installer
- **Windows**: `.exe` installer (NSIS)
- **Linux**: `.deb` package (Ubuntu/Debian)

## Creating a Release

### Prerequisites

- Write access to the repository
- All tests passing on `main` branch
- Changes merged and ready to release

### Step-by-Step Process

#### 1. Update Version Numbers

Update the version in **two** files:

**File 1: `Cargo.toml`**
```toml
[workspace.package]
version = "X.Y.Z"  # Update this line
```

**File 2: `src-tauri/tauri.conf.json`**
```json
{
  "version": "X.Y.Z"  # Update this line
}
```

#### 2. Commit Version Changes

```bash
git add Cargo.toml src-tauri/tauri.conf.json
git commit -m "Bump version to X.Y.Z"
```

#### 3. Create and Push Tag

```bash
# Create the tag (must start with 'v')
git tag vX.Y.Z

# Push the commit
git push origin main

# Push the tag (this triggers the release workflow)
git push origin vX.Y.Z
```

#### 4. Monitor the Build

1. Go to the GitHub repository
2. Click the "Actions" tab
3. Watch the "Release" workflow progress
4. The workflow will:
   - Build on macOS, Windows, and Linux in parallel
   - Run all tests
   - Create installers for each platform
   - Create a GitHub Release
   - Upload all installers to the release

Build time: Approximately 10-15 minutes total (parallel execution)

#### 5. Verify the Release

1. Go to the repository's "Releases" page
2. Find your new release (tagged `vX.Y.Z`)
3. Verify all three installers are attached:
   - `AudiobookConvertRs_X.Y.Z_x64.dmg`
   - `AudiobookConvertRs_X.Y.Z_x64-setup.exe`
   - `audiobookconvert-rs_X.Y.Z_amd64.deb`

#### 6. Test the Release (Recommended)

Download and test each installer on a clean system:
- macOS: Open the DMG and drag to Applications
- Windows: Run the EXE installer
- Linux: Install the DEB with `sudo dpkg -i audiobookconvert-rs_X.Y.Z_amd64.deb`

#### 7. Edit Release Notes (Optional)

GitHub automatically generates release notes from commits. You can edit them to:
- Add highlights of major features
- Include breaking changes
- Add upgrade instructions
- Thank contributors

## Version Naming Convention

Use [Semantic Versioning](https://semver.org/):

- **MAJOR** (X.0.0): Breaking changes, incompatible API changes
- **MINOR** (0.X.0): New features, backwards-compatible
- **PATCH** (0.0.X): Bug fixes, backwards-compatible

### Examples

- `v1.0.0` - First stable release
- `v1.1.0` - Added chapter auto-generation feature
- `v1.1.1` - Fixed metadata bug
- `v2.0.0` - Breaking: New project file format

### Pre-release Versions

For beta or release candidate versions:

```bash
git tag v1.0.0-beta.1
git tag v1.0.0-rc.1
```

The release workflow will mark these as "pre-release" automatically.

## Manual Local Build

For testing or development purposes, you can build installers locally:

```bash
# Install frontend dependencies (first time only)
cd src-ui
npm install
cd ..

# Build the application with bundler
npm exec --prefix src-ui tauri build

# Or build specific bundles:
npm exec --prefix src-ui tauri build -- --bundles dmg    # macOS
npm exec --prefix src-ui tauri build -- --bundles nsis   # Windows
npm exec --prefix src-ui tauri build -- --bundles deb    # Linux
```

### Output Locations

Built installers will be in:
- **macOS**: `target/release/bundle/dmg/`
- **Windows**: `target/release/bundle/nsis/`
- **Linux**: `target/release/bundle/deb/`

## Troubleshooting

### Build Fails on GitHub Actions

1. Check the Actions tab for error logs
2. Common issues:
   - **Syntax errors**: Run `cargo clippy` locally
   - **Tests failing**: Run `cargo test` locally
   - **Icon missing**: Ensure `src-tauri/icons/icon.{icns,ico}` exist

### Tag Already Exists

If you need to recreate a tag:

```bash
# Delete local tag
git tag -d vX.Y.Z

# Delete remote tag
git push origin :refs/tags/vX.Y.Z

# Recreate and push
git tag vX.Y.Z
git push origin vX.Y.Z
```

### Release Artifacts Missing

If some platform builds fail:
1. Check the workflow logs in GitHub Actions
2. Common issues by platform:
   - **macOS**: Icon format issues
   - **Windows**: NSIS installation failed
   - **Linux**: Missing system dependencies

You can manually re-run failed jobs from the Actions tab.

### Version Mismatch Errors

Ensure versions match in both files:
```bash
# Check versions
grep -A1 '\[workspace.package\]' Cargo.toml
grep version src-tauri/tauri.conf.json
```

## Release Checklist

Before creating a release, ensure:

- [ ] All features for this release are merged to `main`
- [ ] All tests pass locally: `cd src-tauri && cargo test`
- [ ] Linter passes: `cd src-tauri && cargo clippy`
- [ ] Version bumped in both `Cargo.toml` and `tauri.conf.json`
- [ ] Version numbers match in both files
- [ ] CHANGELOG updated (if maintaining one)
- [ ] Known issues documented in release notes
- [ ] Breaking changes clearly communicated

## CI/CD Architecture

The release pipeline consists of:

1. **`.github/workflows/release.yml`**: Main release workflow
   - Triggered by tags matching `v*`
   - Builds on `macos-latest`, `windows-latest`, `ubuntu-22.04`
   - Uses optimized release profile from `Cargo.toml`

2. **Build Steps** (per platform):
   - Checkout code
   - Setup Node.js 20 with npm cache
   - Setup Rust toolchain
   - Install platform-specific dependencies
   - Install frontend dependencies
   - Run Tauri build with platform bundler
   - Upload artifacts

3. **Release Step**:
   - Download all artifacts
   - Create GitHub Release
   - Upload all installers as release assets
   - Generate release notes from commits

## Platform-Specific Notes

### macOS
- Builds universal binary for x86_64
- Creates standard DMG with drag-to-Applications
- Code signing: Not configured (installers show as "unverified")
- To enable signing: Add signing certificate to GitHub Secrets

### Windows
- Uses NSIS installer (recommended for desktop apps)
- Installer includes uninstaller
- No code signing (users see SmartScreen warning)
- To enable signing: Add certificate to GitHub Secrets

### Linux (Debian/Ubuntu)
- Builds DEB package for Ubuntu 22.04+
- Users must have FFmpeg installed separately
- Install command: `sudo dpkg -i audiobookconvert-rs_X.Y.Z_amd64.deb`

## Post-Release

After a successful release:

1. **Announce the release**:
   - Update project README if needed
   - Post to relevant communities
   - Update documentation site (if any)

2. **Monitor for issues**:
   - Watch GitHub Issues for bug reports
   - Be ready to release a patch version if critical bugs found

3. **Plan next release**:
   - Create milestone for next version
   - Label issues and PRs for next release

## Emergency Rollback

If a release has critical issues:

1. Create a new patch release with the fix
2. Mark the broken release as "pre-release" in GitHub
3. Add warning to broken release notes
4. Do NOT delete releases (breaks user links)

## Questions?

- Check `.github/workflows/release.yml` for workflow details
- See `DEVELOPMENT.md` for development setup
- Open an issue for deployment problems

