//! Download a fixed FFmpeg build when none is installed
//!
//! Each supported target has exactly one build. Its URL, size and SHA-256 are
//! compiled in, so a file that changes on the host is refused rather than
//! installed. `tests/ffmpeg_sources_live.rs` downloads every row weekly in CI.

use crate::ffmpeg::detection::{
    clear_ffmpeg_cache, verify_ffmpeg_at_path, FFmpegSource, FFMPEG_EXE,
};
use camino::{Utf8Path, Utf8PathBuf};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::io::{self, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tracing::{info, warn};

/// How a build's file is packed
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArchiveKind {
    Zip,
    SevenZ,
}

/// One fixed FFmpeg build
#[derive(Debug, Clone, Copy)]
pub struct FixedBuild {
    /// Rust target triple the build runs on
    pub target: &'static str,
    /// FFmpeg version, also the install folder's name
    pub version: &'static str,
    pub url: &'static str,
    pub size_bytes: u64,
    /// Lowercase hex SHA-256 of the file at `url`
    pub sha256: &'static str,
    pub archive: ArchiveKind,
    /// Archive entry to keep: an exact name, or `*/` and a path suffix
    pub entry: &'static str,
    /// Who publishes the build, as shown to the user
    pub host: &'static str,
    /// Page describing the build, linked from Settings
    pub page_url: &'static str,
}

/// Every fixed build. Linux has none: the `.deb` depends on the distribution's FFmpeg.
pub const FIXED_BUILDS: &[FixedBuild] = &[
    FixedBuild {
        target: "aarch64-apple-darwin",
        version: "9.0.2",
        url: "https://ffmpeg.martin-riedl.de/download/macos/arm64/1789931890_9.0.2/ffmpeg.zip",
        size_bytes: 28_395_699,
        sha256: "c8ed4c4e6978a03c485edbfe4e0a5dc2380f8a30bba5150531b31b094492d924",
        archive: ArchiveKind::Zip,
        entry: "ffmpeg",
        host: "ffmpeg.martin-riedl.de",
        page_url: "https://ffmpeg.martin-riedl.de/",
    },
    FixedBuild {
        target: "x86_64-apple-darwin",
        version: "9.0.2",
        url: "https://ffmpeg.martin-riedl.de/download/macos/amd64/1789931006_9.0.2/ffmpeg.zip",
        size_bytes: 33_816_391,
        sha256: "7c6b4125b191cbf773832dc51f424cf2b6bb7da43007d1e066f95909e47cacd4",
        archive: ArchiveKind::Zip,
        entry: "ffmpeg",
        host: "ffmpeg.martin-riedl.de",
        page_url: "https://ffmpeg.martin-riedl.de/",
    },
    FixedBuild {
        target: "x86_64-pc-windows-msvc",
        version: "9.0.2",
        url: "https://github.com/GyanD/codexffmpeg/releases/download/9.0.2/ffmpeg-9.0.2-essentials_build.7z",
        size_bytes: 35_430_500,
        sha256: "4705843ccaaf54257c16ad90f3e952ece33c17df964ecf7bfdbb0f49c7171077",
        archive: ArchiveKind::SevenZ,
        entry: "*/bin/ffmpeg.exe",
        host: "gyan.dev",
        page_url: "https://www.gyan.dev/ffmpeg/builds/",
    },
];

/// Target triple this app was compiled for, among those with a fixed build
const CURRENT_TARGET: &str = if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
    "aarch64-apple-darwin"
} else if cfg!(all(target_os = "macos", target_arch = "x86_64")) {
    "x86_64-apple-darwin"
} else if cfg!(all(target_os = "windows", target_arch = "x86_64")) {
    "x86_64-pc-windows-msvc"
} else {
    ""
};

/// What the UI shows when it offers a download
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DownloadOffer {
    pub version: String,
    pub size_bytes: u64,
    pub host: String,
    pub page_url: String,
}

impl From<&FixedBuild> for DownloadOffer {
    fn from(build: &FixedBuild) -> Self {
        Self {
            version: build.version.to_string(),
            size_bytes: build.size_bytes,
            host: build.host.to_string(),
            page_url: build.page_url.to_string(),
        }
    }
}

/// The build for this platform, if it has one
pub fn current_build() -> Option<&'static FixedBuild> {
    build_for(CURRENT_TARGET)
}

/// The build for a target triple, if it has one
pub fn build_for(target: &str) -> Option<&'static FixedBuild> {
    FIXED_BUILDS.iter().find(|b| b.target == target)
}

/// Whether an archive entry name is the one a build keeps. Windows archives
/// may use backslashes, so both separators are accepted.
pub fn entry_matches(pattern: &str, name: &str) -> bool {
    let name = name.replace('\\', "/");
    match pattern.strip_prefix('*') {
        Some(suffix) => name.ends_with(suffix),
        None => name == pattern,
    }
}

/// Why getting FFmpeg stopped, in words the user can act on
#[derive(Debug, thiserror::Error)]
pub enum DownloadError {
    #[error("there is no FFmpeg download for this platform")]
    NoBuild,
    #[error("a download is already running")]
    Busy,
    #[error("download cancelled")]
    Cancelled,
    #[error("could not reach {host}: {reason}")]
    Network { host: &'static str, reason: String },
    #[error("{host} answered with HTTP {status}")]
    HttpStatus { host: &'static str, status: u16 },
    #[error("the download is larger than the expected {expected} bytes, so it was stopped")]
    TooLarge { expected: u64 },
    #[error("the download ended early: {received} of {expected} bytes")]
    Incomplete { received: u64, expected: u64 },
    #[error(
        "the download failed its integrity check (SHA-256 mismatch), so nothing was installed"
    )]
    ChecksumMismatch,
    #[error("the download does not contain {0}")]
    EntryMissing(&'static str),
    #[error("could not unpack the download: {0}")]
    Unpack(String),
    #[error("the downloaded FFmpeg does not work: {0}")]
    CheckFailed(String),
    #[error("could not write the download: {0}")]
    Io(#[from] io::Error),
}

/// Writes a download to disk while hashing it. The file is kept only when
/// `finish` confirms its size and SHA-256; any other ending deletes it.
pub struct VerifiedWriter<'a> {
    path: Utf8PathBuf,
    file: Option<File>,
    hasher: Sha256,
    received: u64,
    expected_size: u64,
    expected_sha256: String,
    cancel: &'a AtomicBool,
    kept: bool,
}

impl<'a> VerifiedWriter<'a> {
    pub fn create(
        path: &Utf8Path,
        expected_size: u64,
        expected_sha256: &str,
        cancel: &'a AtomicBool,
    ) -> Result<Self, DownloadError> {
        Ok(Self {
            path: path.to_owned(),
            file: Some(File::create(path)?),
            hasher: Sha256::new(),
            received: 0,
            expected_size,
            expected_sha256: expected_sha256.to_ascii_lowercase(),
            cancel,
            kept: false,
        })
    }

    /// Bytes written so far
    pub fn received(&self) -> u64 {
        self.received
    }

    /// Append the next chunk. Stops on cancel, and as soon as the data
    /// outgrows the expected size, before the checksum could say so.
    pub fn push(&mut self, chunk: &[u8]) -> Result<(), DownloadError> {
        if self.cancel.load(Ordering::SeqCst) {
            return Err(DownloadError::Cancelled);
        }
        self.received += chunk.len() as u64;
        if self.received > self.expected_size {
            return Err(DownloadError::TooLarge {
                expected: self.expected_size,
            });
        }
        self.hasher.update(chunk);
        if let Some(file) = self.file.as_mut() {
            file.write_all(chunk)?;
        }
        Ok(())
    }

    /// Check size and checksum; keep the file only when both match
    pub fn finish(mut self) -> Result<Utf8PathBuf, DownloadError> {
        if let Some(mut file) = self.file.take() {
            file.flush()?;
        }
        if self.received != self.expected_size {
            return Err(DownloadError::Incomplete {
                received: self.received,
                expected: self.expected_size,
            });
        }
        let actual = format!("{:x}", std::mem::take(&mut self.hasher).finalize());
        if actual != self.expected_sha256 {
            return Err(DownloadError::ChecksumMismatch);
        }
        self.kept = true;
        Ok(self.path.clone())
    }
}

impl Drop for VerifiedWriter<'_> {
    fn drop(&mut self) {
        if !self.kept {
            self.file.take();
            let _ = fs::remove_file(&self.path);
        }
    }
}

/// Delete what an interrupted attempt left in `root`: `.partial` files and `.tmp` folders
pub fn clean_leftovers(root: &Utf8Path) -> io::Result<()> {
    let Ok(entries) = fs::read_dir(root) else {
        return Ok(());
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.ends_with(".partial") {
            fs::remove_file(entry.path())?;
        } else if name.ends_with(".tmp") {
            fs::remove_dir_all(entry.path())?;
        }
    }
    Ok(())
}

/// Unpack a verified download and install it as `root/<version>/ffmpeg`.
///
/// Only the build's one entry is extracted, into `<version>.tmp/`. It must pass
/// the same check detection uses before the folder is renamed into place, so a
/// broken copy is never visible. The archive, older versions and leftovers are
/// then deleted and the detection cache cleared.
pub fn install(
    archive: &Utf8Path,
    build: &FixedBuild,
    root: &Utf8Path,
) -> Result<Utf8PathBuf, DownloadError> {
    let staging = root.join(format!("{}.tmp", build.version));
    let target = root.join(build.version);
    if staging.exists() {
        fs::remove_dir_all(&staging)?;
    }
    fs::create_dir_all(&staging)?;

    let result = (|| {
        let exe = staging.join(FFMPEG_EXE);
        extract_entry(archive, build, &exe)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&exe, fs::Permissions::from_mode(0o755))?;
        }
        verify_ffmpeg_at_path(&exe, FFmpegSource::Downloaded)
            .map_err(|e| DownloadError::CheckFailed(e.to_string()))?;
        if target.exists() {
            fs::remove_dir_all(&target)?;
        }
        fs::rename(&staging, &target)?;
        Ok::<_, DownloadError>(target.join(FFMPEG_EXE))
    })();

    if result.is_err() {
        let _ = fs::remove_dir_all(&staging);
    }
    let _ = fs::remove_file(archive);
    let installed = result?;

    // Only the new version stays
    for entry in fs::read_dir(root)?.flatten() {
        if entry.path() != target.as_std_path() {
            let path = entry.path();
            let removed = if path.is_dir() {
                fs::remove_dir_all(&path)
            } else {
                fs::remove_file(&path)
            };
            if let Err(e) = removed {
                warn!("Could not remove {}: {}", path.display(), e);
            }
        }
    }

    clear_ffmpeg_cache();
    info!("Installed FFmpeg {} at {}", build.version, installed);
    Ok(installed)
}

/// Copy the build's entry out of `archive` to `dest`
pub fn extract_entry(
    archive: &Utf8Path,
    build: &FixedBuild,
    dest: &Utf8Path,
) -> Result<(), DownloadError> {
    let unpack = |e: &dyn std::fmt::Display| DownloadError::Unpack(e.to_string());
    match build.archive {
        ArchiveKind::Zip => {
            let mut zip = zip::ZipArchive::new(File::open(archive)?).map_err(|e| unpack(&e))?;
            for i in 0..zip.len() {
                let mut file = zip.by_index(i).map_err(|e| unpack(&e))?;
                if file.is_file() && entry_matches(build.entry, file.name()) {
                    io::copy(&mut file, &mut File::create(dest)?)?;
                    return Ok(());
                }
            }
            Err(DownloadError::EntryMissing(build.entry))
        }
        ArchiveKind::SevenZ => {
            let mut reader =
                sevenz_rust2::ArchiveReader::open(archive, sevenz_rust2::Password::empty())
                    .map_err(|e| unpack(&e))?;
            let mut found = false;
            reader
                .for_each_entries(|entry, data| {
                    if !entry.is_directory() && entry_matches(build.entry, entry.name()) {
                        io::copy(data, &mut File::create(dest)?)?;
                        found = true;
                        return Ok(false);
                    }
                    // Solid archives must be read through to reach later entries
                    io::copy(data, &mut io::sink())?;
                    Ok(true)
                })
                .map_err(|e| unpack(&e))?;
            found
                .then_some(())
                .ok_or(DownloadError::EntryMissing(build.entry))
        }
    }
}

/// Delete every downloaded FFmpeg
pub fn remove_downloaded(root: &Utf8Path) -> io::Result<()> {
    if root.exists() {
        fs::remove_dir_all(root)?;
    }
    clear_ffmpeg_cache();
    Ok(())
}

/// Download `build`, verify it and install it under `root`. `progress` gets
/// (bytes received, total) after every chunk.
pub async fn download_and_install(
    build: &'static FixedBuild,
    root: &Utf8Path,
    cancel: &AtomicBool,
    progress: impl FnMut(u64, u64),
) -> Result<Utf8PathBuf, DownloadError> {
    fs::create_dir_all(root)?;
    clean_leftovers(root)?;
    let archive = fetch_verified(build, root, cancel, progress).await?;

    let root = root.to_owned();
    tokio::task::spawn_blocking(move || install(&archive, build, &root))
        .await
        .map_err(|e| DownloadError::Unpack(e.to_string()))?
}

/// Download `build` to `dir/<version>.partial`, kept only when its size and
/// SHA-256 match the table. Shared with the weekly link check.
pub async fn fetch_verified(
    build: &FixedBuild,
    dir: &Utf8Path,
    cancel: &AtomicBool,
    mut progress: impl FnMut(u64, u64),
) -> Result<Utf8PathBuf, DownloadError> {
    let network = |e: reqwest::Error| DownloadError::Network {
        host: build.host,
        reason: e.to_string(),
    };
    let client = reqwest::Client::builder()
        .user_agent(concat!("AudiobookConvertRs/", env!("CARGO_PKG_VERSION")))
        .connect_timeout(Duration::from_secs(20))
        .build()
        .map_err(network)?;

    info!("Downloading FFmpeg {} from {}", build.version, build.url);
    let mut response = client.get(build.url).send().await.map_err(network)?;
    if !response.status().is_success() {
        return Err(DownloadError::HttpStatus {
            host: build.host,
            status: response.status().as_u16(),
        });
    }

    let partial = dir.join(format!("{}.partial", build.version));
    let mut writer = VerifiedWriter::create(&partial, build.size_bytes, build.sha256, cancel)?;
    loop {
        // A stalled connection must not outlive a cancel for long
        let next = tokio::time::timeout(Duration::from_secs(30), response.chunk())
            .await
            .map_err(|_| DownloadError::Network {
                host: build.host,
                reason: "no data for 30 seconds".into(),
            })?
            .map_err(network)?;
        let Some(chunk) = next else { break };
        writer.push(&chunk)?;
        progress(writer.received(), build.size_bytes);
    }
    writer.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_build_is_complete() {
        for build in FIXED_BUILDS {
            assert!(build.url.starts_with("https://"), "{}", build.target);
            assert!(build.size_bytes > 0, "{}", build.target);
            assert_eq!(build.sha256.len(), 64, "{}", build.target);
            assert!(
                build
                    .sha256
                    .chars()
                    .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()),
                "{}",
                build.target
            );
            assert!(build.page_url.starts_with("https://"), "{}", build.target);
        }
    }

    #[test]
    fn current_target_picks_its_row() {
        let expected = if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
            Some("aarch64-apple-darwin")
        } else if cfg!(all(target_os = "macos", target_arch = "x86_64")) {
            Some("x86_64-apple-darwin")
        } else if cfg!(all(target_os = "windows", target_arch = "x86_64")) {
            Some("x86_64-pc-windows-msvc")
        } else {
            None
        };
        assert_eq!(current_build().map(|b| b.target), expected);
        assert!(build_for("x86_64-unknown-linux-gnu").is_none());
    }

    fn sha256_hex(data: &[u8]) -> String {
        format!("{:x}", Sha256::digest(data))
    }

    /// Feeds `data` in two chunks to a writer expecting `size` bytes and `sha`
    fn write(
        path: &Utf8Path,
        data: &[u8],
        size: u64,
        sha: &str,
        cancel: &AtomicBool,
    ) -> Result<Utf8PathBuf, DownloadError> {
        let mut writer = VerifiedWriter::create(path, size, sha, cancel)?;
        let (a, b) = data.split_at(data.len() / 2);
        writer.push(a)?;
        writer.push(b)?;
        writer.finish()
    }

    #[test]
    fn verified_writer_keeps_only_matching_data() {
        let temp = tempfile::tempdir().unwrap();
        let path = Utf8PathBuf::from_path_buf(temp.path().join("9.0.2.partial")).unwrap();
        let data = b"pretend this is an ffmpeg archive";
        let (size, sha) = (data.len() as u64, sha256_hex(data));
        let go = AtomicBool::new(false);

        assert_eq!(write(&path, data, size, &sha, &go).unwrap(), path);
        assert_eq!(fs::read(&path).unwrap(), data);
        fs::remove_file(&path).unwrap();

        let wrong = sha256_hex(b"something else");
        assert!(matches!(
            write(&path, data, size, &wrong, &go),
            Err(DownloadError::ChecksumMismatch)
        ));
        assert!(!path.exists(), "checksum mismatch must leave no file");

        assert!(matches!(
            write(&path, data, size - 1, &sha, &go),
            Err(DownloadError::TooLarge { .. })
        ));
        assert!(!path.exists(), "oversize data must leave no file");

        assert!(matches!(
            write(&path, data, size + 1, &sha, &go),
            Err(DownloadError::Incomplete { .. })
        ));
        assert!(!path.exists(), "a short download must leave no file");

        let stop = AtomicBool::new(true);
        assert!(matches!(
            write(&path, data, size, &sha, &stop),
            Err(DownloadError::Cancelled)
        ));
        assert!(!path.exists(), "cancel must leave no file");
    }

    #[test]
    fn leftovers_are_cleaned() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8Path::from_path(temp.path()).unwrap();
        fs::write(root.join("9.0.2.partial"), "half").unwrap();
        fs::create_dir(root.join("9.0.2.tmp")).unwrap();
        fs::create_dir(root.join("9.0.2")).unwrap();
        clean_leftovers(root).unwrap();
        let left: Vec<_> = fs::read_dir(root)
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        assert_eq!(left, ["9.0.2"]);
        clean_leftovers(&root.join("missing")).unwrap();
    }

    #[test]
    fn removing_the_download_leaves_nothing_to_find() {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().join("ffmpeg")).unwrap();
        fs::create_dir_all(root.join("9.0.2")).unwrap();
        fs::write(root.join("9.0.2").join(FFMPEG_EXE), "").unwrap();
        assert!(crate::ffmpeg::detection::installed_download(&root).is_some());

        remove_downloaded(&root).unwrap();
        assert!(!root.exists());
        // With system installs skipped, detection consults only this folder
        assert!(crate::ffmpeg::detection::installed_download(&root).is_none());
        remove_downloaded(&root).unwrap();
    }

    #[cfg(unix)]
    mod install {
        use super::super::*;
        use crate::ffmpeg::detection::test_support::fake_ffmpeg;
        use std::os::unix::fs::PermissionsExt;

        fn build(archive: ArchiveKind, entry: &'static str) -> FixedBuild {
            FixedBuild {
                archive,
                entry,
                ..FIXED_BUILDS[0]
            }
        }

        /// Bytes of a fake `ffmpeg` that reports `version`
        fn fake_bytes(dir: &Utf8Path, version: &str) -> Vec<u8> {
            fs::read(fake_ffmpeg(
                &dir.join(format!("fake-{version}")),
                version,
                "aac",
            ))
            .unwrap()
        }

        fn zip_with(path: &Utf8Path, entries: &[(&str, &[u8])]) {
            let mut zip = zip::ZipWriter::new(File::create(path).unwrap());
            for (name, data) in entries {
                zip.start_file(*name, zip::write::SimpleFileOptions::default())
                    .unwrap();
                zip.write_all(data).unwrap();
            }
            zip.finish().unwrap();
        }

        fn seven_z_with(path: &Utf8Path, entries: &[(&str, &[u8])]) {
            let mut writer = sevenz_rust2::ArchiveWriter::new(File::create(path).unwrap()).unwrap();
            for (name, data) in entries {
                writer
                    .push_archive_entry(sevenz_rust2::ArchiveEntry::new_file(name), Some(*data))
                    .unwrap();
            }
            writer.finish().unwrap();
        }

        fn names(root: &Utf8Path) -> Vec<String> {
            let mut names: Vec<_> = fs::read_dir(root)
                .unwrap()
                .map(|e| e.unwrap().file_name().into_string().unwrap())
                .collect();
            names.sort();
            names
        }

        #[test]
        fn zip_installs_only_the_executable() {
            let temp = tempfile::tempdir().unwrap();
            let work = Utf8Path::from_path(temp.path()).unwrap();
            let root = work.join("ffmpeg");
            fs::create_dir_all(root.join("8.1")).unwrap();
            fs::create_dir_all(root.join("9.0.2.tmp")).unwrap();
            let archive = root.join("9.0.2.partial");
            zip_with(
                &archive,
                &[
                    ("README.txt", b"read me"),
                    ("ffmpeg", &fake_bytes(work, "9.0.2")),
                ],
            );

            let installed = install(&archive, &build(ArchiveKind::Zip, "ffmpeg"), &root).unwrap();
            assert_eq!(installed, root.join("9.0.2").join(FFMPEG_EXE));
            assert_eq!(
                fs::metadata(&installed).unwrap().permissions().mode() & 0o777,
                0o755
            );
            assert_eq!(names(&root), ["9.0.2"]);
            assert_eq!(names(&root.join("9.0.2")), [FFMPEG_EXE]);
        }

        #[test]
        fn seven_z_installs_the_matching_entry() {
            let temp = tempfile::tempdir().unwrap();
            let work = Utf8Path::from_path(temp.path()).unwrap();
            let root = work.join("ffmpeg");
            fs::create_dir_all(&root).unwrap();
            let archive = root.join("9.0.2.partial");
            seven_z_with(
                &archive,
                &[
                    (
                        "ffmpeg-9.0.2-essentials_build/bin/ffprobe.exe",
                        b"not this one",
                    ),
                    (
                        "ffmpeg-9.0.2-essentials_build/bin/ffmpeg.exe",
                        &fake_bytes(work, "9.0.2"),
                    ),
                    ("ffmpeg-9.0.2-essentials_build/LICENSE", b"GPL"),
                ],
            );

            let installed = install(
                &archive,
                &build(ArchiveKind::SevenZ, "*/bin/ffmpeg.exe"),
                &root,
            )
            .unwrap();
            assert_eq!(fs::read(&installed).unwrap(), fake_bytes(work, "9.0.2"));
            assert_eq!(names(&root), ["9.0.2"]);
        }

        #[test]
        fn a_copy_that_fails_the_check_is_not_installed() {
            let temp = tempfile::tempdir().unwrap();
            let work = Utf8Path::from_path(temp.path()).unwrap();
            let root = work.join("ffmpeg");
            fs::create_dir_all(&root).unwrap();
            let archive = root.join("9.0.2.partial");
            zip_with(&archive, &[("ffmpeg", &fake_bytes(work, "3.0"))]);

            let result = install(&archive, &build(ArchiveKind::Zip, "ffmpeg"), &root);
            assert!(
                matches!(result, Err(DownloadError::CheckFailed(_))),
                "{result:?}"
            );
            assert!(names(&root).is_empty(), "left behind: {:?}", names(&root));

            zip_with(&archive, &[("other", b"x")]);
            let result = install(&archive, &build(ArchiveKind::Zip, "ffmpeg"), &root);
            assert!(
                matches!(result, Err(DownloadError::EntryMissing(_))),
                "{result:?}"
            );
            assert!(names(&root).is_empty());
        }
    }

    #[test]
    fn entry_patterns() {
        assert!(entry_matches("ffmpeg", "ffmpeg"));
        assert!(!entry_matches("ffmpeg", "bin/ffmpeg"));
        assert!(entry_matches(
            "*/bin/ffmpeg.exe",
            "ffmpeg-9.0.2-essentials_build/bin/ffmpeg.exe"
        ));
        assert!(entry_matches(
            "*/bin/ffmpeg.exe",
            "ffmpeg-9.0.2-essentials_build\\bin\\ffmpeg.exe"
        ));
        assert!(!entry_matches(
            "*/bin/ffmpeg.exe",
            "ffmpeg-9.0.2-essentials_build/bin/ffprobe.exe"
        ));
    }
}
