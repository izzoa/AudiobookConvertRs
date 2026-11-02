//! Batch file processing module
//!
//! Provides functions to load multiple audio files and create audiobooks
//! with proper validation, metadata merging, and chapter creation.

use crate::audio::format::{is_supported_format, SUPPORTED_FORMATS_LABEL};
use crate::audio::loader::load_audio_file;
use crate::error::{AppError, AppResult};
use crate::metadata::chapters::create_chapters_from_files;
use crate::models::{AudioBook, AudioFile, Metadata};
use camino::{Utf8Path, Utf8PathBuf};
use rayon::prelude::*;
use std::cmp::Ordering;
use tracing::{debug, info, warn};

/// Result of loading multiple files
pub struct BatchLoadResult {
    /// Successfully loaded files
    pub files: Vec<AudioFile>,
    /// Errors encountered during loading
    pub errors: Vec<(Utf8PathBuf, AppError)>,
}

impl BatchLoadResult {
    /// Check if all files loaded successfully
    pub fn all_successful(&self) -> bool {
        self.errors.is_empty()
    }

    /// Get count of successful loads
    pub fn success_count(&self) -> usize {
        self.files.len()
    }

    /// Get count of failed loads
    pub fn error_count(&self) -> usize {
        self.errors.len()
    }
}

/// Progress information for batch loading
#[derive(Debug, Clone)]
pub struct BatchProgress {
    /// Current file index (0-based)
    pub current_index: usize,
    /// Total number of files
    pub total_files: usize,
    /// Current file path
    pub current_file: String,
    /// Progress percentage (0.0 to 1.0)
    pub progress: f32,
}

impl BatchProgress {
    /// Create a new progress update
    pub fn new(index: usize, total: usize, file: &str) -> Self {
        Self {
            current_index: index,
            total_files: total,
            current_file: file.to_string(),
            progress: if total > 0 {
                index as f32 / total as f32
            } else {
                0.0
            },
        }
    }
}

/// Load multiple audio files in parallel
///
/// Returns both successful loads and errors, allowing partial success.
pub fn load_files_batch(paths: &[Utf8PathBuf]) -> BatchLoadResult {
    load_files_batch_with_progress(paths, |_| {})
}

/// Load multiple audio files with progress tracking
///
/// The callback is called after each file is processed (successfully or not).
/// Note: Due to parallel processing, callbacks may be out of order.
pub fn load_files_batch_with_progress<F>(
    paths: &[Utf8PathBuf],
    progress_callback: F,
) -> BatchLoadResult
where
    F: Fn(BatchProgress) + Sync,
{
    info!("Loading {} audio files in batch", paths.len());

    let total = paths.len();
    let processed = std::sync::atomic::AtomicUsize::new(0);

    let results: Vec<(Utf8PathBuf, Result<AudioFile, AppError>)> = paths
        .par_iter()
        .map(|path| {
            let result = load_audio_file(path);

            // Update progress
            let current = processed.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
            let file_name = path.file_name().unwrap_or(path.as_str());
            progress_callback(BatchProgress::new(current, total, file_name));

            (path.clone(), result)
        })
        .collect();

    let mut files = Vec::new();
    let mut errors = Vec::new();

    for (path, result) in results {
        match result {
            Ok(file) => files.push(file),
            Err(e) => {
                warn!("Failed to load {}: {}", path, e);
                errors.push((path, e));
            }
        }
    }

    // Sort files by path for consistent ordering
    files.sort_by(|a, b| natural_sort(&a.path, &b.path));

    info!(
        "Batch load complete: {} successful, {} failed",
        files.len(),
        errors.len()
    );

    BatchLoadResult { files, errors }
}

/// Expand any directories in `paths` into the supported audio files they hold.
///
/// Callers hand us whatever the user picked or dropped — files, a book folder,
/// or a folder of per-disc subfolders — so directories are walked recursively.
/// Files are returned in natural order within each directory, and anything that
/// is not a supported audio file is dropped (cover images, cue sheets, .DS_Store).
pub fn expand_input_paths(paths: &[Utf8PathBuf]) -> Vec<Utf8PathBuf> {
    let mut expanded = Vec::new();

    for path in paths {
        if !path.is_dir() {
            expanded.push(path.clone());
            continue;
        }

        let mut found: Vec<Utf8PathBuf> = walkdir::WalkDir::new(path.as_std_path())
            .follow_links(false)
            .into_iter()
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.file_type().is_file())
            .filter_map(|entry| Utf8PathBuf::from_path_buf(entry.into_path()).ok())
            // Resource forks and the like share real extensions, so skip dotfiles.
            .filter(|p| !p.file_name().map(|n| n.starts_with('.')).unwrap_or(false))
            .filter(|p| is_supported_format(p))
            .collect();

        found.sort_by(|a, b| natural_sort(a, b));
        debug!("Expanded directory {} to {} audio files", path, found.len());
        expanded.extend(found);
    }

    expanded
}

/// Create an audiobook from a list of file paths
///
/// This function:
/// 1. Loads all audio files using Symphonia
/// 2. Validates all files are readable
/// 3. Calculates total duration
/// 4. Merges metadata from files
/// 5. Creates initial chapters from file boundaries
pub fn load_audiobook_from_files(paths: &[Utf8PathBuf]) -> AppResult<AudioBook> {
    if paths.is_empty() {
        return Err(AppError::NoFilesProvided);
    }

    let requested = paths;
    let paths = expand_input_paths(paths);
    if paths.is_empty() {
        return Err(AppError::NoAudioFound {
            location: describe_inputs(requested),
            supported: SUPPORTED_FORMATS_LABEL.to_string(),
        });
    }

    info!("Creating audiobook from {} files", paths.len());

    // Load all files
    let batch_result = load_files_batch(&paths);

    if batch_result.files.is_empty() {
        // All files failed
        if let Some((_, first_error)) = batch_result.errors.into_iter().next() {
            return Err(first_error);
        }
        return Err(AppError::NoFilesProvided);
    }

    // Log any partial failures
    for (path, error) in &batch_result.errors {
        warn!("Skipping file {}: {}", path, error);
    }

    let files = batch_result.files;

    // Merge metadata from all files (use first file's metadata as base)
    let mut merged_metadata = merge_metadata_from_files(&files);

    // Create chapters from file boundaries
    let chapters = create_chapters_from_files(&files);

    // Calculate totals
    let total_duration_ms: u64 = files.iter().map(|f| f.duration_ms).sum();
    let total_size_bytes: u64 = files.iter().map(|f| f.file_size_bytes).sum();

    let file_paths: Vec<Utf8PathBuf> = files.iter().map(|f| f.path.clone()).collect();
    let title = choose_book_title(
        &files,
        &merged_metadata,
        book_folder(&file_paths).as_deref(),
    );

    // The merged tags start from the first file, so for a multi-file book they
    // describe its first chapter. The M4B's title and track tags must describe
    // the book instead.
    merged_metadata.title = Some(title.clone());
    if files.len() > 1 {
        merged_metadata.track_number = None;
        merged_metadata.total_tracks = None;
        merged_metadata.disc_number = None;
        merged_metadata.total_discs = None;
    }

    // Create audiobook
    let mut audiobook = AudioBook::new(title);
    audiobook.author = merged_metadata.artist.clone();
    audiobook.files = files;
    audiobook.chapters = chapters;
    audiobook.output_metadata = merged_metadata;
    audiobook.total_duration_ms = total_duration_ms;
    audiobook.total_size_bytes = total_size_bytes;

    info!(
        "Created audiobook: {} ({} files, {})",
        audiobook.title,
        audiobook.files.len(),
        audiobook.formatted_duration()
    );

    Ok(audiobook)
}

/// Merge metadata from multiple files
///
/// Strategy:
/// - Use non-empty values from the first file as the base
/// - For album/series info, prefer consistency across files
fn merge_metadata_from_files(files: &[AudioFile]) -> Metadata {
    if files.is_empty() {
        return Metadata::default();
    }

    let mut merged = files[0].metadata.clone();

    // Try to fill in missing fields from other files
    for file in files.iter().skip(1) {
        if merged.title.is_none() {
            merged.title = file.metadata.title.clone();
        }
        if merged.artist.is_none() {
            merged.artist = file.metadata.artist.clone();
        }
        if merged.album.is_none() {
            merged.album = file.metadata.album.clone();
        }
        if merged.album_artist.is_none() {
            merged.album_artist = file.metadata.album_artist.clone();
        }
        if merged.year.is_none() {
            merged.year = file.metadata.year;
        }
        if merged.genre.is_none() {
            merged.genre = file.metadata.genre.clone();
        }
        if merged.narrator.is_none() {
            merged.narrator = file.metadata.narrator.clone();
        }
        if merged.publisher.is_none() {
            merged.publisher = file.metadata.publisher.clone();
        }
        if merged.artwork_data.is_none() && file.metadata.artwork_data.is_some() {
            merged.artwork_data = file.metadata.artwork_data.clone();
        }
        if merged.artwork_path.is_none() && file.metadata.artwork_path.is_some() {
            merged.artwork_path = file.metadata.artwork_path.clone();
        }
    }

    merged
}

/// Picks the book's title.
///
/// A single file is the whole book, so its own title tag names it. Across
/// several files the title tag names each chapter instead ("Opening", "Part 2"),
/// and the album tag carries the book, so the album wins. When the tags give
/// nothing usable, the folder the book lives in is a better name than the first
/// file's stem ("01 - Opening").
fn choose_book_title(files: &[AudioFile], merged: &Metadata, folder: Option<&Utf8Path>) -> String {
    let non_empty = |v: &Option<String>| {
        v.as_deref()
            .map(str::trim)
            .filter(|t| !t.is_empty())
            .map(String::from)
    };

    if files.len() == 1 {
        if let Some(title) =
            non_empty(&files[0].metadata.title).or_else(|| non_empty(&merged.album))
        {
            return title;
        }
    } else {
        if let Some(album) = non_empty(&merged.album) {
            return album;
        }
        // Some rips stamp the book title on every track rather than in album.
        let first = non_empty(&files[0].metadata.title);
        if first.is_some() && files.iter().all(|f| non_empty(&f.metadata.title) == first) {
            return first.unwrap();
        }
    }

    folder
        .and_then(|dir| dir.file_name())
        .map(String::from)
        .or_else(|| files.first().map(|f| f.stem().to_string()))
        .unwrap_or_else(|| "Untitled Audiobook".to_string())
}

/// True for folders that split one book rather than name it: `CD1`, `Disc 2`,
/// `disk_03`, `Part 4`, `Vol. 1`.
pub fn is_disc_folder_name(name: &str) -> bool {
    let lower = name.trim().to_lowercase();
    let rest = ["volume", "disc", "disk", "part", "vol", "cd"]
        .iter()
        .find_map(|prefix| lower.strip_prefix(prefix));
    match rest {
        Some(rest) => {
            let digits =
                rest.trim_start_matches(|c: char| c == ' ' || c == '_' || c == '-' || c == '.');
            !digits.is_empty() && digits.chars().all(|c| c.is_ascii_digit())
        }
        None => false,
    }
}

/// The folder that holds the whole book, when there is one.
///
/// That is the files' shared folder, stepping out of a disc folder (`CD1/`)
/// when the files sit in one. Files picked from unrelated places have no book
/// folder; reaching further up would name the book after something like the
/// user's home directory.
pub fn book_folder(paths: &[Utf8PathBuf]) -> Option<Utf8PathBuf> {
    let parents: Vec<&Utf8Path> = paths.iter().filter_map(|p| p.parent()).collect();
    let first = *parents.first()?;

    let mut root = first.to_path_buf();
    while !parents.iter().all(|p| p.starts_with(&root)) {
        root = root.parent()?.to_path_buf();
    }
    if root.file_name().map(is_disc_folder_name).unwrap_or(false) {
        root = root.parent()?.to_path_buf();
    }

    // Every file must be in the book folder or one level below it.
    let close = parents
        .iter()
        .all(|p| *p == root || p.parent() == Some(root.as_path()));
    close.then_some(root)
}

/// One book found by [`group_into_books`]: its name and its audio files in
/// playback order.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct BookGroup {
    /// The book folder's name, used until the files' tags give a better one.
    pub name: String,
    /// The folder the book lives in, when it has one.
    pub folder: Option<Utf8PathBuf>,
    pub files: Vec<Utf8PathBuf>,
}

/// How deep a library is searched for book folders (Library/Author/Series/Book).
const MAX_LIBRARY_DEPTH: usize = 6;

/// Splits dropped paths into books, so a whole library can be imported at once.
///
/// A folder is one book when it holds audio itself, or when every subfolder
/// that holds audio is a disc folder (`CD1/`, `Disc 2/`): everything under it
/// then belongs to that book, extras folders included. Any other folder is a
/// shelf, and each of its subfolders is judged the same way. Loose files are
/// grouped by the folder they sit in, stepping out of a disc folder.
///
/// The rule only looks at folder shape, never at tags, so the same library
/// always splits the same way.
pub fn group_into_books(paths: &[Utf8PathBuf]) -> Vec<BookGroup> {
    let mut groups: Vec<BookGroup> = Vec::new();
    let mut loose: Vec<Utf8PathBuf> = Vec::new();

    for path in paths {
        if path.is_dir() {
            collect_book_folders(path, 0, &mut groups);
        } else if is_supported_format(path) {
            loose.push(path.clone());
        }
    }

    for file in loose {
        let folder = file.parent().map(|parent| {
            if parent.file_name().map(is_disc_folder_name).unwrap_or(false) {
                parent.parent().unwrap_or(parent).to_path_buf()
            } else {
                parent.to_path_buf()
            }
        });
        match groups
            .iter_mut()
            .find(|g| g.folder.is_some() && g.folder == folder)
        {
            Some(group) => group.files.push(file),
            None => groups.push(BookGroup {
                name: folder
                    .as_deref()
                    .and_then(Utf8Path::file_name)
                    .map(String::from)
                    .unwrap_or_else(|| file.file_stem().unwrap_or("Untitled").to_string()),
                folder,
                files: vec![file],
            }),
        }
    }

    for group in &mut groups {
        group.files.sort_by(|a, b| natural_sort(a, b));
        group.files.dedup();
    }
    groups.retain(|g| !g.files.is_empty());
    groups
}

fn collect_book_folders(dir: &Utf8Path, depth: usize, groups: &mut Vec<BookGroup>) {
    let Ok(entries) = std::fs::read_dir(dir.as_std_path()) else {
        return;
    };
    let mut has_audio = false;
    let mut subdirs: Vec<Utf8PathBuf> = Vec::new();
    for entry in entries.flatten() {
        let Ok(path) = Utf8PathBuf::from_path_buf(entry.path()) else {
            continue;
        };
        if path.file_name().map(|n| n.starts_with('.')).unwrap_or(true) {
            continue;
        }
        if path.is_dir() {
            subdirs.push(path);
        } else if is_supported_format(&path) {
            has_audio = true;
        }
    }
    subdirs.sort_by(|a, b| natural_sort(a, b));

    let audio_subdirs: Vec<&Utf8PathBuf> = subdirs.iter().filter(|d| contains_audio(d)).collect();
    let only_discs = !audio_subdirs.is_empty()
        && audio_subdirs
            .iter()
            .all(|d| d.file_name().map(is_disc_folder_name).unwrap_or(false));

    if has_audio || only_discs || depth >= MAX_LIBRARY_DEPTH {
        let files = expand_input_paths(&[dir.to_path_buf()]);
        if !files.is_empty() {
            groups.push(BookGroup {
                name: dir.file_name().unwrap_or(dir.as_str()).to_string(),
                folder: Some(dir.to_path_buf()),
                files,
            });
        }
        return;
    }

    for sub in audio_subdirs {
        collect_book_folders(sub, depth + 1, groups);
    }
}

fn contains_audio(dir: &Utf8Path) -> bool {
    walkdir::WalkDir::new(dir.as_std_path())
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file())
        .filter_map(|e| Utf8PathBuf::from_path_buf(e.into_path()).ok())
        .any(|p| {
            !p.file_name().map(|n| n.starts_with('.')).unwrap_or(false) && is_supported_format(&p)
        })
}

/// Names what was dropped, for an error that says where nothing was found.
fn describe_inputs(paths: &[Utf8PathBuf]) -> String {
    match paths {
        [one] => format!("\"{}\"", one.file_name().unwrap_or(one.as_str())),
        many => format!("the {} dropped items", many.len()),
    }
}

/// Natural sort comparison for file paths
///
/// Handles numeric sequences properly (e.g., "Chapter 2" before "Chapter 10")
fn natural_sort(a: &Utf8Path, b: &Utf8Path) -> Ordering {
    // Compare the containing directory first: a book split across CD1/CD2
    // holds "01 - …" in both, and comparing names alone interleaves the discs.
    let a_parent = a.parent().map(|p| p.as_str()).unwrap_or("");
    let b_parent = b.parent().map(|p| p.as_str()).unwrap_or("");
    match natural_cmp(a_parent, b_parent) {
        Ordering::Equal => {}
        other => return other,
    }

    let a_name = a.file_name().unwrap_or(a.as_str());
    let b_name = b.file_name().unwrap_or(b.as_str());

    natural_cmp(a_name, b_name)
}

/// Natural string comparison
fn natural_cmp(a: &str, b: &str) -> Ordering {
    let mut a_chars = a.chars().peekable();
    let mut b_chars = b.chars().peekable();

    loop {
        match (a_chars.peek(), b_chars.peek()) {
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(&ac), Some(&bc)) => {
                if ac.is_ascii_digit() && bc.is_ascii_digit() {
                    // Compare numeric sequences
                    let a_num = extract_number(&mut a_chars);
                    let b_num = extract_number(&mut b_chars);
                    match a_num.cmp(&b_num) {
                        Ordering::Equal => continue,
                        other => return other,
                    }
                } else {
                    // Compare characters (case-insensitive)
                    let ac_lower = ac.to_ascii_lowercase();
                    let bc_lower = bc.to_ascii_lowercase();
                    match ac_lower.cmp(&bc_lower) {
                        Ordering::Equal => {
                            a_chars.next();
                            b_chars.next();
                            continue;
                        }
                        other => return other,
                    }
                }
            }
        }
    }
}

/// Extract a number from an iterator of characters
fn extract_number<I: Iterator<Item = char>>(chars: &mut std::iter::Peekable<I>) -> u64 {
    let mut num = 0u64;
    while let Some(&c) = chars.peek() {
        if c.is_ascii_digit() {
            num = num * 10 + c.to_digit(10).unwrap() as u64;
            chars.next();
        } else {
            break;
        }
    }
    num
}

/// Validate that all files can be combined
///
/// Checks:
/// - All files exist
/// - All files are valid audio
/// - Sufficient disk space for output (optional)
pub fn validate_files_for_conversion(files: &[AudioFile], output_path: &Utf8Path) -> AppResult<()> {
    if files.is_empty() {
        return Err(AppError::NoFilesProvided);
    }

    // Check all source files still exist
    for file in files {
        if !file.path.exists() {
            return Err(AppError::FileNotFound(
                file.path.as_std_path().to_path_buf(),
            ));
        }
    }

    // Check output directory exists and is writable
    if let Some(parent) = output_path.parent() {
        if !parent.exists() {
            return Err(AppError::file_write(
                output_path,
                "Output directory does not exist",
            ));
        }
    }

    // Check output doesn't already exist (unless overwrite is enabled)
    if output_path.exists() {
        return Err(AppError::OutputFileExists(
            output_path.as_std_path().to_path_buf(),
        ));
    }

    debug!("All {} files validated for conversion", files.len());
    Ok(())
}

/// Estimate output file size based on input files and settings
pub fn estimate_output_size(files: &[AudioFile], target_bitrate_kbps: u32) -> u64 {
    let total_duration_ms: u64 = files.iter().map(|f| f.duration_ms).sum();
    let duration_sec = total_duration_ms as f64 / 1000.0;

    // Output size ≈ (bitrate_kbps * duration_sec) / 8 * 1024 (convert to bytes)
    // Add ~5% overhead for container and metadata
    let base_size = (target_bitrate_kbps as f64 * duration_sec / 8.0 * 1024.0) as u64;
    (base_size as f64 * 1.05) as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    fn book_file(path: &str, title: Option<&str>, album: Option<&str>) -> AudioFile {
        use crate::models::AudioFormat;
        AudioFile {
            path: path.into(),
            format: AudioFormat::Mp3,
            duration_ms: 60_000,
            bitrate_kbps: Some(64),
            sample_rate_hz: Some(44_100),
            channels: Some(2),
            file_size_bytes: 1,
            metadata: Metadata {
                title: title.map(String::from),
                album: album.map(String::from),
                ..Metadata::default()
            },
            has_artwork: false,
        }
    }

    #[test]
    fn disc_folder_names() {
        for name in [
            "CD1", "cd 2", "Disc 03", "disk_4", "Part 5", "Vol. 1", "Volume-2",
        ] {
            assert!(is_disc_folder_name(name), "{name} should be a disc folder");
        }
        for name in ["Dune", "CD", "Discworld", "Part One", "Cdx1"] {
            assert!(
                !is_disc_folder_name(name),
                "{name} should not be a disc folder"
            );
        }
    }

    #[test]
    fn book_folder_steps_out_of_disc_folders() {
        let discs = [
            Utf8PathBuf::from("/lib/Dune/CD1/01.mp3"),
            Utf8PathBuf::from("/lib/Dune/CD2/01.mp3"),
        ];
        assert_eq!(
            book_folder(&discs).as_deref(),
            Some(Utf8Path::new("/lib/Dune"))
        );

        let one_disc = [Utf8PathBuf::from("/lib/Dune/Disc 1/01.mp3")];
        assert_eq!(
            book_folder(&one_disc).as_deref(),
            Some(Utf8Path::new("/lib/Dune"))
        );

        let flat = [
            Utf8PathBuf::from("/lib/Dune/01.mp3"),
            Utf8PathBuf::from("/lib/Dune/02.mp3"),
        ];
        assert_eq!(
            book_folder(&flat).as_deref(),
            Some(Utf8Path::new("/lib/Dune"))
        );
    }

    #[test]
    fn book_folder_is_none_for_unrelated_files() {
        let scattered = [
            Utf8PathBuf::from("/Users/me/Downloads/a/b/01.mp3"),
            Utf8PathBuf::from("/Users/me/Desktop/02.mp3"),
        ];
        assert_eq!(book_folder(&scattered), None);
    }

    #[test]
    fn multi_file_book_is_named_by_album_not_first_chapter() {
        let files = [
            book_file(
                "/lib/x/01 - Opening.mp3",
                Some("Opening"),
                Some("The Clockwork Chronicle"),
            ),
            book_file(
                "/lib/x/02 - Arrival.mp3",
                Some("Arrival"),
                Some("The Clockwork Chronicle"),
            ),
        ];
        let merged = merge_metadata_from_files(&files);
        assert_eq!(
            choose_book_title(&files, &merged, None),
            "The Clockwork Chronicle"
        );
    }

    #[test]
    fn multi_file_book_falls_back_to_shared_title_then_folder() {
        let stamped = [
            book_file("/lib/x/01.mp3", Some("Dune"), None),
            book_file("/lib/x/02.mp3", Some("Dune"), None),
        ];
        assert_eq!(
            choose_book_title(&stamped, &merge_metadata_from_files(&stamped), None),
            "Dune"
        );

        let untagged = [
            book_file("/lib/Dune/CD1/01 - Opening.mp3", Some("Opening"), None),
            book_file("/lib/Dune/CD1/02 - Arrival.mp3", Some("Arrival"), None),
        ];
        let folder = Utf8Path::new("/lib/Dune");
        assert_eq!(
            choose_book_title(
                &untagged,
                &merge_metadata_from_files(&untagged),
                Some(folder)
            ),
            "Dune"
        );
    }

    #[test]
    fn single_file_book_is_named_by_its_title_tag() {
        let files = [book_file(
            "/lib/dune.m4b",
            Some("Dune"),
            Some("Dune Chronicles"),
        )];
        assert_eq!(
            choose_book_title(&files, &merge_metadata_from_files(&files), None),
            "Dune"
        );
    }

    fn touch(path: &Utf8Path) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, b"not really audio").unwrap();
    }

    fn library(files: &[&str]) -> (tempfile::TempDir, Utf8PathBuf) {
        let temp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(temp.path().join("Library")).unwrap();
        for file in files {
            touch(&root.join(file));
        }
        (temp, root)
    }

    fn summary(groups: &[BookGroup], root: &Utf8Path) -> Vec<(String, Vec<String>)> {
        groups
            .iter()
            .map(|g| {
                let files = g
                    .files
                    .iter()
                    // Joined with `/` so the expected paths read the same on Windows
                    .map(|f| {
                        let relative = f.strip_prefix(root).unwrap();
                        relative
                            .components()
                            .map(|c| c.as_str())
                            .collect::<Vec<_>>()
                            .join("/")
                    })
                    .collect();
                (g.name.clone(), files)
            })
            .collect()
    }

    #[test]
    fn library_folder_splits_into_one_book_per_folder() {
        let (_t, root) = library(&[
            "Herbert/Dune/CD1/01.mp3",
            "Herbert/Dune/CD2/01.mp3",
            "Herbert/Dune/cover.jpg",
            "Herbert/Dune Messiah/01.mp3",
            "Herbert/Dune Messiah/02.mp3",
            "Le Guin/Earthsea/01 - Warrior.mp3",
            "Le Guin/Earthsea/Extras/interview.mp3",
            "Le Guin/notes.txt",
        ]);
        let groups = group_into_books(&[root.clone()]);
        assert_eq!(
            summary(&groups, &root),
            [
                (
                    "Dune".into(),
                    vec![
                        "Herbert/Dune/CD1/01.mp3".into(),
                        "Herbert/Dune/CD2/01.mp3".into()
                    ]
                ),
                (
                    "Dune Messiah".into(),
                    vec![
                        "Herbert/Dune Messiah/01.mp3".into(),
                        "Herbert/Dune Messiah/02.mp3".into()
                    ]
                ),
                (
                    "Earthsea".into(),
                    vec![
                        "Le Guin/Earthsea/01 - Warrior.mp3".into(),
                        "Le Guin/Earthsea/Extras/interview.mp3".into()
                    ]
                ),
            ]
        );
    }

    #[test]
    fn a_single_book_folder_stays_one_book() {
        let (_t, root) = library(&["CD1/01.mp3", "CD1/02.mp3", "CD2/01.mp3"]);
        let groups = group_into_books(&[root.clone()]);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].name, "Library");
        assert_eq!(groups[0].files.len(), 3);
    }

    #[test]
    fn loose_files_group_by_their_book_folder() {
        let (_t, root) = library(&["Dune/CD1/01.mp3", "Dune/CD2/01.mp3", "Emma/01.mp3"]);
        let groups = group_into_books(&[
            root.join("Dune/CD2/01.mp3"),
            root.join("Dune/CD1/01.mp3"),
            root.join("Emma/01.mp3"),
            root.join("Emma/readme.txt"),
        ]);
        assert_eq!(
            summary(&groups, &root),
            [
                (
                    "Dune".into(),
                    vec!["Dune/CD1/01.mp3".into(), "Dune/CD2/01.mp3".into()]
                ),
                ("Emma".into(), vec!["Emma/01.mp3".into()]),
            ]
        );
    }

    #[test]
    fn empty_folder_names_the_folder_in_its_error() {
        let temp = tempfile::tempdir().unwrap();
        let dir = Utf8PathBuf::from_path_buf(temp.path().join("Empty Book")).unwrap();
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("notes.txt"), "not audio").unwrap();

        let message = load_audiobook_from_files(&[dir]).unwrap_err().to_string();
        assert!(message.contains("\"Empty Book\""), "{message}");
        assert!(message.contains("MP3"), "{message}");
    }

    #[test]
    fn test_natural_cmp() {
        assert_eq!(natural_cmp("file1", "file2"), Ordering::Less);
        assert_eq!(natural_cmp("file2", "file10"), Ordering::Less);
        assert_eq!(natural_cmp("file10", "file2"), Ordering::Greater);
        assert_eq!(natural_cmp("abc", "abc"), Ordering::Equal);
        assert_eq!(natural_cmp("Chapter 1", "Chapter 2"), Ordering::Less);
        assert_eq!(natural_cmp("Chapter 2", "Chapter 10"), Ordering::Less);
    }

    #[test]
    fn test_extract_number() {
        let mut chars = "123abc".chars().peekable();
        assert_eq!(extract_number(&mut chars), 123);
        assert_eq!(chars.next(), Some('a'));
    }

    #[test]
    fn test_estimate_output_size() {
        use crate::models::{AudioFormat, Metadata};

        let files = vec![AudioFile {
            path: "/test/file.mp3".into(),
            format: AudioFormat::Mp3,
            duration_ms: 3600000, // 1 hour
            bitrate_kbps: Some(128),
            sample_rate_hz: Some(44100),
            channels: Some(2),
            file_size_bytes: 57_600_000,
            metadata: Metadata::default(),
            has_artwork: false,
        }];

        // 128 kbps * 3600 sec / 8 = 57,600,000 bytes
        let estimated = estimate_output_size(&files, 128);
        assert!(estimated > 55_000_000 && estimated < 65_000_000);
    }

    #[test]
    fn test_batch_load_result() {
        let result = BatchLoadResult {
            files: vec![],
            errors: vec![],
        };
        assert!(result.all_successful());
        assert_eq!(result.success_count(), 0);
        assert_eq!(result.error_count(), 0);
    }

    #[test]
    fn test_batch_progress() {
        let progress = BatchProgress::new(5, 10, "test.mp3");
        assert_eq!(progress.current_index, 5);
        assert_eq!(progress.total_files, 10);
        assert_eq!(progress.current_file, "test.mp3");
        assert!((progress.progress - 0.5).abs() < 0.001);

        // Edge case: zero total
        let progress_zero = BatchProgress::new(0, 0, "empty.mp3");
        assert_eq!(progress_zero.progress, 0.0);
    }
}
