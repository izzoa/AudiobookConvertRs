# Deployment Guide

How a release of AudiobookConvertRs is made. The short version:

```bash
# 1. Bump the version in Cargo.toml ([workspace.package] version), then:
cargo update --workspace   # records it in Cargo.lock
# 2. Commit both files, push main
# 3. Wait for CI to pass on main, then tag:
git tag v0.2.0
git push origin v0.2.0
# 4. Check the draft release on GitHub, then press Publish
```

## What a release contains

A `v*` tag runs `.github/workflows/release.yml`, which builds three installers and attaches them to a **draft** release. Nothing is public until you publish it.

| File | Platform | Signed |
|---|---|---|
| `AudiobookConvertRs_X.Y.Z_universal.dmg` | macOS, Apple Silicon and Intel | Developer ID, notarized and stapled |
| `AudiobookConvertRs_X.Y.Z_x64-setup.exe` | Windows 10/11, 64-bit | No: SmartScreen warns on first run |
| `AudiobookConvertRs_X.Y.Z_amd64.deb` | Ubuntu 22.04+ and Debian-based | No (normal for a `.deb`) |
| `SHA256SUMS` | checksums of the three files | |

## Making a release

1. **Bump the version** in `Cargo.toml` under `[workspace.package]`. It is the only place: `tauri.conf.json` has no version of its own and takes this one.
2. **Run `cargo update --workspace`.** `Cargo.lock` also records the app's version, and CI builds with `--locked`, so it fails if the lock file still says the old one. It updates only the app's own entry and leaves every dependency alone.
3. **Commit both files and push `main`**, and wait for the CI workflow to pass.
4. **Tag and push the tag** (`git tag vX.Y.Z` then `git push origin vX.Y.Z`). The tag must equal the version, or the first job stops the release.
5. **Watch Actions → Release.** The jobs are: the version check, macOS (about 20 minutes, most of it compiling twice and waiting for Apple's notarization), Windows, Linux, and the draft release, which runs only when all three builds pass.
6. **Check the draft** on the Releases page:
   - All four files are attached. Download them and run `shasum -a 256 -c SHA256SUMS` (macOS) or `sha256sum -c SHA256SUMS` (Linux).
   - **macOS:** open the DMG you downloaded with a browser, drag the app to Applications and open it. It must open with no warning. `spctl -a -vv /Applications/AudiobookConvertRs.app` must say `source=Notarized Developer ID`.
   - **Windows:** run the setup file. SmartScreen's "Windows protected your PC" is expected until the installer is signed; choose More info → Run anyway.
   - **Linux:** `sudo apt install ./AudiobookConvertRs_X.Y.Z_amd64.deb` installs the app and FFmpeg.
7. **Edit the generated notes if needed, then press Publish.**

A tag with a hyphen, such as `v1.0.0-beta.1`, needs the same version in `Cargo.toml`, and its draft is marked as a pre-release.

## Signing setup

Needed once, and again when the certificate or API key is replaced. The five secrets live in the repository's **`release` environment** (Settings → Environments → release), which only `v*` tags may use. Pull requests and branch pushes never see them.

| Secret | What it is | How to set it |
|---|---|---|
| `APPLE_CERTIFICATE` | The Developer ID Application certificate and its private key, as base64 | In Keychain Access, under My Certificates, export "Developer ID Application: Anthony Izzo (CMB5ZUJ2TB)" as a `.p12` with a password, then `base64 -i certificate.p12 \| gh secret set APPLE_CERTIFICATE --env release` |
| `APPLE_CERTIFICATE_PASSWORD` | The password you gave that export | `gh secret set APPLE_CERTIFICATE_PASSWORD --env release` (prompts for it) |
| `APPLE_API_ISSUER` | Issuer ID of your App Store Connect API keys | App Store Connect → Users and Access → Integrations → App Store Connect API, above the key list |
| `APPLE_API_KEY` | Key ID of a key with **Developer** access | The same page, after creating the key |
| `APPLE_API_KEY_P8` | The key's `.p8` file (it can be downloaded only once) | `gh secret set APPLE_API_KEY_P8 --env release < AuthKey_XXXXXXXXXX.p8` |

Delete the exported `.p12` once the secret is set, and keep the `.p8` somewhere safe.

- **The certificate expires on 2027-11-17.** Releases signed before then keep working, because their signatures are timestamped. Before the next release after that date, create a new Developer ID Application certificate and replace the two certificate secrets. If the certificate's name changes, update `APPLE_SIGNING_IDENTITY` in `release.yml` too.
- **If the API key leaks,** revoke it in App Store Connect, create a new one, and replace `APPLE_API_KEY` and `APPLE_API_KEY_P8`.

## What the release checks

| Job | Fails when |
|---|---|
| Tag matches Cargo.toml | the tag (without `v`) differs from the `Cargo.toml` version |
| macOS | the app's signature, Gatekeeper's verdict on the app or the DMG, or the stapled ticket does not verify, or the app is not universal (`scripts/release/macos-verify.sh`) |
| Windows | a silent install (`/S`) leaves no app, no uninstaller or no Start menu entry, or the silent uninstall leaves the app behind |
| Linux | the package does not depend on `ffmpeg`, or `apt` cannot install it |
| Draft release | any of the three builds failed |

## Building locally

Unsigned installers for the machine you are on:

```bash
npm ci --prefix src-ui
npm exec --prefix src-ui tauri build -- --bundles dmg    # macOS
npm exec --prefix src-ui tauri build -- --bundles nsis   # Windows
npm exec --prefix src-ui tauri build -- --bundles deb    # Linux
```

They land in `target/release/bundle/<format>/`.

The signed universal Mac build, exactly as CI makes it (the Developer ID certificate must be in your login keychain):

```bash
export APPLE_SIGNING_IDENTITY='Developer ID Application: Anthony Izzo (CMB5ZUJ2TB)'
export APPLE_API_ISSUER=<issuer id> APPLE_API_KEY=<key id> APPLE_API_KEY_PATH=<path to AuthKey_*.p8>
npm exec --prefix src-ui tauri build -- --target universal-apple-darwin --bundles app,dmg
dmg=$(ls target/universal-apple-darwin/release/bundle/dmg/*.dmg)
scripts/release/macos-notarize-dmg.sh "$dmg"
scripts/release/macos-verify.sh "$dmg"
```

## Continuous integration

- **`ci.yml`** runs on pushes to `main` and on pull requests. It builds and tests on Linux, macOS and Windows with `--locked`, so a `Cargo.lock` that no longer matches `Cargo.toml` fails the run; runs the frontend tests on Linux; reports clippy warnings without failing; and fails on unformatted Rust (`cargo fmt --all --check`). `npm run lint` is not enforced yet because of its existing errors.
- **`ffmpeg-links.yml`** runs weekly and on demand. It downloads every fixed FFmpeg build that Get FFmpeg offers and checks its size and SHA-256. When it fails, a host moved or replaced a file and `src-tauri/src/ffmpeg/download.rs` needs a new row.

## Troubleshooting

- **"Tag vX does not match version Y in Cargo.toml":** fix the version, or delete the tag and tag the right commit (see below).
- **"no identity found" or the certificate import step fails:** the `.p12` was exported without its private key, or `APPLE_CERTIFICATE_PASSWORD` is wrong. Export it again from the private key's entry in Keychain Access.
- **Notarization status "Invalid":** the job prints Apple's log. The usual cause is an unsigned binary inside the app.
- **A job stalls at notarization:** Apple's queue is slow. Re-run the failed jobs from the Actions page.
- **Re-tagging:** delete the draft release first, then delete and push the tag again:

  ```bash
  git tag -d vX.Y.Z
  git push origin :refs/tags/vX.Y.Z
  git tag vX.Y.Z
  git push origin vX.Y.Z
  ```

## Rolling back

- **Before publishing:** delete the draft release and the tag.
- **After publishing:** do not delete the release, because that breaks download links. Mark it as a pre-release, add a warning at the top of its notes, and ship a patch release.
