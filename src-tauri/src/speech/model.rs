//! The optional speech model: where it lives, whether it is there, and
//! getting or removing it. It is downloaded only when the user asks, checked
//! like a downloaded FFmpeg, and kept in the app's data folder.

use crate::ffmpeg::download::{clean_leftovers, fetch_verified_file, DownloadError, FixedFile};
use camino::{Utf8Path, Utf8PathBuf};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::sync::atomic::AtomicBool;
use tracing::info;

/// Whisper "base", multilingual, quantized: the Audible stores the app
/// searches cover several languages, and base hears names better than tiny.
pub const MODEL: FixedFile = FixedFile {
    url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base-q5_1.bin",
    host: "huggingface.co",
    size_bytes: 59_707_625,
    sha256: "422f1ae452ade6f30a004d7e5c6a43195e4433bc370bf23fac9cc591f01a8898",
    stem: "ggml-base-q5_1",
};

/// Page about the model, linked from Settings
pub const MODEL_PAGE: &str = "https://huggingface.co/ggerganov/whisper.cpp";

/// Folder holding the speech model
pub fn model_root() -> Option<Utf8PathBuf> {
    let dir = dirs::data_local_dir()?
        .join("AudiobookConvertRs")
        .join("speech");
    Utf8PathBuf::from_path_buf(dir).ok()
}

fn file_name(file: &FixedFile) -> String {
    format!("{}.bin", file.stem)
}

/// The installed model in `root`, when one of the right size is there
pub fn installed(root: &Utf8Path, file: &FixedFile) -> Option<Utf8PathBuf> {
    let path = root.join(file_name(file));
    let size = fs::metadata(&path).ok()?.len();
    (size == file.size_bytes).then_some(path)
}

/// The model the app uses, when it is installed
pub fn installed_model() -> Option<Utf8PathBuf> {
    installed(&model_root()?, &MODEL)
}

/// What Settings and the review show about the model
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpeechModelStatus {
    pub installed: bool,
    pub size_bytes: u64,
    pub host: String,
    pub page_url: String,
}

pub fn status() -> SpeechModelStatus {
    SpeechModelStatus {
        installed: installed_model().is_some(),
        size_bytes: MODEL.size_bytes,
        host: MODEL.host.to_string(),
        page_url: MODEL_PAGE.to_string(),
    }
}

/// Put a download that passed its checks in place as `root/<stem>.bin`
pub fn install(partial: &Utf8Path, root: &Utf8Path, file: &FixedFile) -> io::Result<Utf8PathBuf> {
    let target = root.join(file_name(file));
    fs::rename(partial, &target)?;
    info!("Installed the speech model at {}", target);
    Ok(target)
}

/// Download `file` into `root`, verify it and install it. `progress` gets
/// (bytes received, total) after every chunk.
pub async fn download(
    file: &FixedFile,
    root: &Utf8Path,
    cancel: &AtomicBool,
    progress: impl FnMut(u64, u64),
) -> Result<Utf8PathBuf, DownloadError> {
    fs::create_dir_all(root)?;
    clean_leftovers(root)?;
    let partial = fetch_verified_file(file, root, cancel, progress).await?;
    Ok(install(&partial, root, file)?)
}

/// Delete the model and anything a download left behind
pub fn remove(root: &Utf8Path) -> io::Result<()> {
    if root.exists() {
        fs::remove_dir_all(root)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ffmpeg::download::VerifiedWriter;
    use sha2::{Digest, Sha256};

    const DATA: &[u8] = b"not really a speech model";

    fn fake(sha256: &'static str) -> FixedFile {
        FixedFile {
            url: "https://example.invalid/model.bin",
            host: "example.invalid",
            size_bytes: DATA.len() as u64,
            sha256,
            stem: "test-model",
        }
    }

    fn leak(s: String) -> &'static str {
        Box::leak(s.into_boxed_str())
    }

    /// What `download` does once the bytes have arrived
    fn receive(root: &Utf8Path, file: &FixedFile) -> Result<Utf8PathBuf, DownloadError> {
        let cancel = AtomicBool::new(false);
        let partial = root.join(format!("{}.partial", file.stem));
        let mut writer = VerifiedWriter::create(&partial, file.size_bytes, file.sha256, &cancel)?;
        writer.push(DATA)?;
        let checked = writer.finish()?;
        Ok(install(&checked, root, file)?)
    }

    fn root() -> (tempfile::TempDir, Utf8PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(dir.path().join("speech")).unwrap();
        fs::create_dir_all(&root).unwrap();
        (dir, root)
    }

    #[test]
    fn a_matching_download_is_installed() {
        let (_dir, root) = root();
        let file = fake(leak(format!("{:x}", Sha256::digest(DATA))));
        let path = receive(&root, &file).unwrap();
        assert_eq!(installed(&root, &file), Some(path));
    }

    #[test]
    fn a_mismatched_download_installs_nothing_and_leaves_nothing() {
        let (_dir, root) = root();
        let file = fake("0000000000000000000000000000000000000000000000000000000000000000");
        assert!(matches!(
            receive(&root, &file),
            Err(DownloadError::ChecksumMismatch)
        ));
        assert_eq!(installed(&root, &file), None);
        assert_eq!(fs::read_dir(&root).unwrap().count(), 0);
    }

    #[test]
    fn a_file_of_the_wrong_size_does_not_count_as_installed() {
        let (_dir, root) = root();
        let file = fake("unused");
        fs::write(root.join("test-model.bin"), b"short").unwrap();
        assert_eq!(installed(&root, &file), None);
    }

    #[test]
    fn remove_deletes_the_model() {
        let (_dir, root) = root();
        let file = fake(leak(format!("{:x}", Sha256::digest(DATA))));
        receive(&root, &file).unwrap();
        remove(&root).unwrap();
        assert_eq!(installed(&root, &file), None);
        assert!(!root.exists());
    }

    #[test]
    fn a_cancelled_download_leaves_no_partial_file() {
        let (_dir, root) = root();
        let file = fake(leak(format!("{:x}", Sha256::digest(DATA))));
        let cancel = AtomicBool::new(false);
        let partial = root.join("test-model.partial");
        let mut writer =
            VerifiedWriter::create(&partial, file.size_bytes, file.sha256, &cancel).unwrap();
        writer.push(&DATA[..4]).unwrap();
        cancel.store(true, std::sync::atomic::Ordering::SeqCst);
        assert!(matches!(
            writer.push(&DATA[4..]),
            Err(DownloadError::Cancelled)
        ));
        drop(writer);
        assert_eq!(fs::read_dir(&root).unwrap().count(), 0);
        assert_eq!(installed(&root, &file), None);
    }

    #[test]
    fn the_model_entry_is_complete() {
        assert!(MODEL.url.starts_with("https://huggingface.co/"));
        assert_eq!(MODEL.sha256.len(), 64);
        assert!(MODEL.size_bytes > 50_000_000);
    }
}
