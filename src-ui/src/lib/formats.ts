/**
 * Input formats the backend will actually load, named for people.
 *
 * Mirrors `SUPPORTED_EXTENSIONS` in `src-tauri/src/audio/format.rs`; change
 * both together. OGA is an OGG variant there, so it is not listed on its own.
 * The UI once advertised WMA, which the loader rejects.
 */
export const SUPPORTED_FORMAT_LABELS = ["MP3", "M4A", "M4B", "FLAC", "WAV", "OGG", "AAC"] as const;

/** "MP3, M4A, M4B, FLAC, WAV, OGG, AAC" — for prose such as help text. */
export const SUPPORTED_FORMATS_TEXT = SUPPORTED_FORMAT_LABELS.join(", ");
