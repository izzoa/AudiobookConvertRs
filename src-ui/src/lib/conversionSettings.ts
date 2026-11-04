import type { ConversionDefaults, ConversionSettings, EncodingMode, UserSettings } from "@/types/models";

/** Per-book choices that override the defaults (the Convert bar's Quality menu). */
export interface QualityChoice {
    bitrate_kbps?: number;
    /** `null` keeps each file's own rate; leaving it out uses the default. */
    sample_rate_hz?: number | null;
    encoding_mode?: EncodingMode;
    embed_chapters?: boolean;
    embed_artwork?: boolean;
    copy_metadata?: boolean;
    normalize?: boolean;
    /** Keep the audio as it is instead of encoding it. */
    copy_audio?: boolean;
    /** Replace a file already at the output path without asking. */
    overwrite_existing?: boolean;
}

/**
 * The settings one conversion job runs with. The single-book Convert bar and
 * the batch both build jobs here, so a book converts the same way whichever
 * route it took.
 */
export function buildConversionSettings(
    defaults: ConversionDefaults,
    settings: UserSettings | null,
    outputPath: string,
    choice: QualityChoice = {}
): ConversionSettings {
    const normalize = choice.normalize ?? false;
    return {
        bitrate_kbps: choice.bitrate_kbps ?? defaults.bitrate_kbps,
        sample_rate_hz:
            choice.sample_rate_hz === undefined ? defaults.sample_rate_hz : (choice.sample_rate_hz ?? undefined),
        encoding_mode: choice.encoding_mode ?? defaults.encoding_mode,
        output_format: "m4b",
        output_path: outputPath,
        concurrent_jobs: settings?.concurrent_jobs ?? 1,
        overwrite_existing: choice.overwrite_existing ?? defaults.overwrite_existing,
        embed_chapters: choice.embed_chapters ?? defaults.embed_chapters,
        embed_artwork: choice.embed_artwork ?? defaults.embed_artwork,
        copy_metadata: choice.copy_metadata ?? defaults.copy_metadata,
        normalize_audio: normalize,
        normalization_settings: normalize
            ? {
                  target_lufs: settings?.normalization_target_lufs ?? -18,
                  target_true_peak: settings?.normalization_target_true_peak ?? -2,
                  target_lra: settings?.normalization_target_lra ?? 7,
                  two_pass: settings?.normalization_two_pass ?? true,
              }
            : undefined,
        copy_audio: choice.copy_audio ?? false,
    };
}
