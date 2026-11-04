import { useState, useEffect, useLayoutEffect, useRef, type ReactNode } from 'react';
import { useApp } from '@/context/useApp';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Select } from '@/components/ui/select';
import { useConfirm } from '@/components/ui/confirm';
import { useToast } from '@/components/ui/use-toast';
import { FolderOpenIcon } from '@heroicons/react/24/outline';
import * as SettingsApi from '@/api/settings';
import * as FilesApi from '@/api/files';
import type { UserSettings, ConversionDefaults, QualityPreset, EncodingMode } from '@/types/models';
import { FFmpegPanel } from '@/components/settings/FFmpegPanel';
import { SpeechModelPanel } from '@/components/settings/SpeechModelPanel';
import { QualityPreset as QualityPresetValue, EncodingMode as EncodingModeValue } from '@/types/models';
import { cn } from '@/lib/utils';
import { AUDIBLE_REGIONS, DEFAULT_AUDIBLE_REGION } from '@/lib/audibleRegions';
import {
    DEFAULT_OUTPUT_TEMPLATE,
    TEMPLATE_FIELDS,
    TEMPLATE_PRESETS,
    renderOutputPath,
    templateValues,
    unknownFields,
    type TemplateValues,
} from '@/lib/outputTemplate';

const BITRATE_OPTIONS = [320, 256, 192, 160, 128, 96, 64, 48, 32];

export function SettingsPage() {
    const { settings, updateSettings, refreshSettings, activeProject, setActiveProject, ffmpeg } = useApp();
    const confirm = useConfirm();
    const { toast } = useToast();
    const [draftSettings, setDraftSettings] = useState<UserSettings | null>(null);
    const [draftProjectDefaults, setDraftProjectDefaults] = useState<ConversionDefaults | null>(null);
    const [isSaving, setIsSaving] = useState(false);
    const [error, setError] = useState<string | null>(null);

    useEffect(() => {
        if (settings) {
            setDraftSettings({ ...settings });
        }
    }, [settings]);

    useEffect(() => {
        if (activeProject?.conversion_defaults) {
            setDraftProjectDefaults({ ...activeProject.conversion_defaults });
        } else {
            setDraftProjectDefaults(null);
        }
    }, [activeProject]);

    // Edits only reach disk on Save, so the page has to say when they haven't.
    const isDirty = !!settings && !!draftSettings && JSON.stringify(settings) !== JSON.stringify(draftSettings);

    const handleSave = async () => {
        if (!draftSettings) return;

        setIsSaving(true);
        setError(null);

        try {
            if (draftSettings.default_bitrate_kbps < 8 || draftSettings.default_bitrate_kbps > 320) {
                throw new Error('Bitrate must be between 8 and 320 kbps');
            }
            if (draftSettings.concurrent_jobs < 1 || draftSettings.concurrent_jobs > 16) {
                throw new Error('Concurrent jobs must be between 1 and 16');
            }

            await updateSettings(draftSettings);
            toast({ title: 'Settings saved' });
        } catch (e) {
            setError(e instanceof Error ? e.message : 'Failed to save settings');
        } finally {
            setIsSaving(false);
        }
    };

    const handleReset = async () => {
        const ok = await confirm({
            title: 'Reset all settings?',
            description: 'Every preference on this page returns to its default. Saved projects and presets are not affected.',
            confirmLabel: 'Reset settings',
            danger: true,
        });
        if (!ok) return;

        setIsSaving(true);
        setError(null);

        try {
            await SettingsApi.resetSettings();
            await refreshSettings();
            // Defaults have no chosen FFmpeg path
            await ffmpeg.check();
            toast({ title: 'Settings reset to defaults' });
        } catch (e) {
            setError(e instanceof Error ? e.message : 'Failed to reset settings');
        } finally {
            setIsSaving(false);
        }
    };

    const handleDiscard = () => {
        if (settings) {
            setDraftSettings({ ...settings });
        }
        setError(null);
    };

    const handlePickOutputDirectory = async () => {
        try {
            const path = await FilesApi.pickDirectory();
            if (path && draftSettings) {
                setDraftSettings({ ...draftSettings, default_output_directory: path });
            }
        } catch (e) {
            toast({ title: 'Could not open folder picker', description: String(e), variant: 'destructive' });
        }
    };

    const handleApplyProjectDefaults = () => {
        if (!activeProject || !draftProjectDefaults) return;

        setActiveProject({
            ...activeProject,
            conversion_defaults: draftProjectDefaults,
        });
        toast({ title: 'Project defaults updated', description: 'Save the project to keep them.' });
    };

    const handleClearProjectDefaults = () => {
        if (!activeProject) return;

        setActiveProject({
            ...activeProject,
            conversion_defaults: undefined,
        });
        setDraftProjectDefaults(null);
        toast({ title: 'Project uses the global defaults again' });
    };

    const updateDraft = <K extends keyof UserSettings>(key: K, value: UserSettings[K]) => {
        if (draftSettings) {
            setDraftSettings({ ...draftSettings, [key]: value });
        }
    };

    const updateProjectDraft = <K extends keyof ConversionDefaults>(key: K, value: ConversionDefaults[K]) => {
        const defaults = draftProjectDefaults ?? {
            bitrate_kbps: settings?.default_bitrate_kbps ?? 128,
            sample_rate_hz: settings?.default_sample_rate,
            encoding_mode: settings?.default_encoding_mode ?? (EncodingModeValue.Vbr as EncodingMode),
            overwrite_existing: settings?.overwrite_existing_by_default ?? false,
            embed_chapters: settings?.embed_chapters_by_default ?? true,
            embed_artwork: settings?.embed_artwork_by_default ?? true,
            copy_metadata: settings?.copy_metadata_by_default ?? true,
        };
        setDraftProjectDefaults({ ...defaults, [key]: value });
    };

    if (!draftSettings) {
        return (
            <div className="h-full flex items-center justify-center">
                <p className="text-sm text-muted-foreground">Loading settings…</p>
            </div>
        );
    }

    return (
        <div className="min-h-full flex flex-col">
            <div className="flex-1 w-full max-w-3xl mx-auto px-6 pt-8 pb-10">
                <h1 className="text-2xl font-semibold tracking-tight">Settings</h1>
                <p className="mt-1 text-sm text-muted-foreground">
                    Defaults for every new book. The Quality menu beside Convert can still change them per book.
                </p>

                <Section title="Conversion" description="What a new book is encoded with.">
                    <div className="grid grid-cols-2 gap-4">
                        <Field label="Bitrate" htmlFor="bitrate">
                            <Select
                                id="bitrate"
                                value={draftSettings.default_bitrate_kbps}
                                onChange={(e) => updateDraft('default_bitrate_kbps', parseInt(e.target.value))}
                            >
                                {BITRATE_OPTIONS.map((rate) => (
                                    <option key={rate} value={rate}>{rate} kbps</option>
                                ))}
                            </Select>
                        </Field>
                        <Field label="Sample rate" htmlFor="sample-rate">
                            <Select
                                id="sample-rate"
                                value={draftSettings.default_sample_rate ?? 'match'}
                                onChange={(e) => updateDraft('default_sample_rate', e.target.value === 'match' ? undefined : parseInt(e.target.value))}
                            >
                                <option value="match">Match source</option>
                                <option value="44100">44,100 Hz</option>
                                <option value="48000">48,000 Hz</option>
                                <option value="22050">22,050 Hz</option>
                            </Select>
                        </Field>
                        <Field label="Quality preset" htmlFor="quality-preset" hint="Starting point for new conversions.">
                            <Select
                                id="quality-preset"
                                value={draftSettings.default_quality_preset ?? QualityPresetValue.Medium}
                                onChange={(e) => updateDraft('default_quality_preset', e.target.value as QualityPreset)}
                            >
                                <option value={QualityPresetValue.High}>High (256 kbps, 44.1 kHz)</option>
                                <option value={QualityPresetValue.Medium}>Medium (128 kbps, 44.1 kHz)</option>
                                <option value={QualityPresetValue.Low}>Low (64 kbps, 22 kHz)</option>
                                <option value={QualityPresetValue.Custom}>Custom</option>
                            </Select>
                        </Field>
                        <Field label="Encoding mode" htmlFor="encoding-mode" hint="VBR gives better quality for the size.">
                            <Select
                                id="encoding-mode"
                                value={draftSettings.default_encoding_mode ?? EncodingModeValue.Vbr}
                                onChange={(e) => updateDraft('default_encoding_mode', e.target.value as EncodingMode)}
                            >
                                <option value={EncodingModeValue.Vbr}>VBR (variable bitrate)</option>
                                <option value={EncodingModeValue.Cbr}>CBR (constant bitrate)</option>
                            </Select>
                        </Field>
                    </div>
                    <div className="grid grid-cols-2 gap-x-4 gap-y-1">
                        <Check
                            checked={draftSettings.embed_chapters_by_default}
                            onChange={(v) => updateDraft('embed_chapters_by_default', v)}
                            label="Embed chapters"
                        />
                        <Check
                            checked={draftSettings.embed_artwork_by_default}
                            onChange={(v) => updateDraft('embed_artwork_by_default', v)}
                            label="Embed cover art"
                        />
                        <Check
                            checked={draftSettings.copy_metadata_by_default}
                            onChange={(v) => updateDraft('copy_metadata_by_default', v)}
                            label="Copy metadata"
                        />
                        <Check
                            checked={draftSettings.overwrite_existing_by_default}
                            onChange={(v) => updateDraft('overwrite_existing_by_default', v)}
                            label="Overwrite existing files"
                        />
                    </div>
                </Section>

                <Section title="Output" description="Where finished books are written, and what they are called.">
                    <Field
                        label="Output folder"
                        htmlFor="output-dir"
                        hint="Leave empty to choose a folder for each book. The names below apply inside it."
                    >
                        <div className="flex gap-2">
                            <Input
                                id="output-dir"
                                className="h-9"
                                value={draftSettings.default_output_directory ?? ''}
                                onChange={(e) => updateDraft('default_output_directory', e.target.value || undefined)}
                                placeholder="Ask each time"
                            />
                            <Button
                                variant="outline"
                                size="icon"
                                className="h-9 w-9 shrink-0"
                                onClick={handlePickOutputDirectory}
                                aria-label="Choose output folder"
                                title="Choose output folder"
                            >
                                <FolderOpenIcon className="w-4 h-4" />
                            </Button>
                        </div>
                    </Field>
                    <OutputTemplateField
                        value={draftSettings.output_template ?? DEFAULT_OUTPUT_TEMPLATE}
                        onChange={(template) => updateDraft('output_template', template)}
                    />
                </Section>

                <Section title="Queue" description="How much converts at once.">
                    <Field label="Concurrent jobs" htmlFor="concurrent" hint="Books converting in parallel, 1 to 16.">
                        <Input
                            id="concurrent"
                            type="number"
                            min={1}
                            max={16}
                            className="h-9 w-24 nums"
                            value={draftSettings.concurrent_jobs}
                            onChange={(e) => updateDraft('concurrent_jobs', parseInt(e.target.value) || 1)}
                        />
                    </Field>
                </Section>

                <Section title="Cover art" description="Found automatically when a book is imported.">
                    <div className="space-y-1">
                        <Check
                            checked={draftSettings.auto_detect_artwork}
                            onChange={(v) => updateDraft('auto_detect_artwork', v)}
                            label="Use art embedded in the audio files"
                        />
                        <Check
                            checked={draftSettings.scan_for_cover_images}
                            onChange={(v) => updateDraft('scan_for_cover_images', v)}
                            label="Look for cover images beside the files"
                        />
                    </div>
                </Section>

                <Section
                    title="Find details"
                    description="Fills narrator, series, summary, cover and chapters from Audible. Nothing is sent until you search."
                >
                    <Field
                        label="Audible store"
                        htmlFor="audible-region"
                        hint="Searched by default. Find details can switch stores for one search."
                    >
                        <Select
                            id="audible-region"
                            className="w-56"
                            value={draftSettings.audible_region || DEFAULT_AUDIBLE_REGION}
                            onChange={(e) => updateDraft('audible_region', e.target.value)}
                        >
                            {AUDIBLE_REGIONS.map((region) => (
                                <option key={region.code} value={region.code}>
                                    {region.label}
                                </option>
                            ))}
                        </Select>
                    </Field>
                </Section>

                <Section title="Interface">
                    <div className="grid grid-cols-2 gap-4">
                        <Field label="Theme" htmlFor="theme">
                            <Select
                                id="theme"
                                value={draftSettings.theme}
                                onChange={(e) => updateDraft('theme', e.target.value as 'light' | 'dark' | 'system')}
                            >
                                <option value="system">Match system</option>
                                <option value="light">Light</option>
                                <option value="dark">Dark</option>
                            </Select>
                        </Field>
                        <Field label="Log level" htmlFor="log-level" hint="Takes effect on next launch.">
                            <Select
                                id="log-level"
                                value={draftSettings.log_level}
                                onChange={(e) => updateDraft('log_level', e.target.value as UserSettings['log_level'])}
                            >
                                <option value="error">Error</option>
                                <option value="warn">Warn</option>
                                <option value="info">Info</option>
                                <option value="debug">Debug</option>
                                <option value="trace">Trace</option>
                            </Select>
                        </Field>
                    </div>
                </Section>

                <Section
                    id="settings-ffmpeg"
                    title="FFmpeg"
                    description="Encodes every book. Found on this computer, chosen by you, or downloaded here."
                >
                    <FFmpegPanel />
                </Section>

                <Section
                    id="settings-speech"
                    title="Speech model"
                    description="Optional. Places fitted chapters where the narrator announces them."
                >
                    <SpeechModelPanel />
                </Section>

                {activeProject && (
                    <Section
                        title="This project"
                        description="Overrides the defaults above for the open project only. Save the project to keep them."
                    >
                        <div className="grid grid-cols-2 gap-4">
                            <Field label="Bitrate" htmlFor="project-bitrate">
                                <Select
                                    id="project-bitrate"
                                    value={draftProjectDefaults?.bitrate_kbps ?? settings?.default_bitrate_kbps ?? 128}
                                    onChange={(e) => updateProjectDraft('bitrate_kbps', parseInt(e.target.value))}
                                >
                                    {BITRATE_OPTIONS.map((rate) => (
                                        <option key={rate} value={rate}>{rate} kbps</option>
                                    ))}
                                </Select>
                            </Field>
                            <Field label="Sample rate" htmlFor="project-sample-rate">
                                <Select
                                    id="project-sample-rate"
                                    value={draftProjectDefaults?.sample_rate_hz ?? 'match'}
                                    onChange={(e) => updateProjectDraft('sample_rate_hz', e.target.value === 'match' ? undefined : parseInt(e.target.value))}
                                >
                                    <option value="match">Match source</option>
                                    <option value="44100">44,100 Hz</option>
                                    <option value="48000">48,000 Hz</option>
                                    <option value="22050">22,050 Hz</option>
                                </Select>
                            </Field>
                        </div>
                        <div className="grid grid-cols-2 gap-x-4 gap-y-1">
                            <Check
                                checked={draftProjectDefaults?.embed_chapters ?? settings?.embed_chapters_by_default ?? true}
                                onChange={(v) => updateProjectDraft('embed_chapters', v)}
                                label="Embed chapters"
                            />
                            <Check
                                checked={draftProjectDefaults?.embed_artwork ?? settings?.embed_artwork_by_default ?? true}
                                onChange={(v) => updateProjectDraft('embed_artwork', v)}
                                label="Embed cover art"
                            />
                            <Check
                                checked={draftProjectDefaults?.copy_metadata ?? settings?.copy_metadata_by_default ?? true}
                                onChange={(v) => updateProjectDraft('copy_metadata', v)}
                                label="Copy metadata"
                            />
                            <Check
                                checked={draftProjectDefaults?.overwrite_existing ?? settings?.overwrite_existing_by_default ?? false}
                                onChange={(v) => updateProjectDraft('overwrite_existing', v)}
                                label="Overwrite existing files"
                            />
                        </div>
                        <div className="flex gap-2">
                            <Button size="sm" onClick={handleApplyProjectDefaults} disabled={!draftProjectDefaults}>
                                Apply to project
                            </Button>
                            <Button variant="outline" size="sm" onClick={handleClearProjectDefaults}>
                                Use global defaults
                            </Button>
                        </div>
                    </Section>
                )}

                <div className="mt-10 pt-6 border-t flex justify-end">
                    <Button
                        variant="ghost"
                        size="sm"
                        onClick={handleReset}
                        disabled={isSaving}
                        className="text-destructive hover:text-destructive hover:bg-destructive/10"
                    >
                        Reset to defaults
                    </Button>
                </div>
            </div>

            {(isDirty || error) && (
                <div className="sticky bottom-0 border-t bg-surface-2 animate-slide-up">
                    <div className="max-w-3xl mx-auto px-6 h-14 flex items-center gap-3">
                        <p className={cn('flex-1 text-sm', error ? 'text-destructive' : 'text-muted-foreground')}>
                            {error ?? 'You have unsaved changes.'}
                        </p>
                        <Button variant="ghost" size="sm" onClick={handleDiscard} disabled={isSaving}>
                            Discard
                        </Button>
                        <Button size="sm" onClick={handleSave} disabled={isSaving || !isDirty}>
                            {isSaving ? 'Saving…' : 'Save changes'}
                        </Button>
                    </div>
                </div>
            )}
        </div>
    );
}

function Section({ id, title, description, children }: { id?: string; title: string; description?: string; children: ReactNode }) {
    return (
        <section id={id} className="mt-8 pt-6 border-t first-of-type:border-t-0 grid grid-cols-[180px_1fr] gap-x-8 gap-y-4 scroll-mt-4">
            <div>
                <h2 className="text-sm font-semibold">{title}</h2>
                {description && <p className="mt-1 text-xs text-muted-foreground leading-relaxed">{description}</p>}
            </div>
            <div className="min-w-0 space-y-4">{children}</div>
        </section>
    );
}

function Field({ label, htmlFor, hint, children }: { label: string; htmlFor: string; hint?: string; children: ReactNode }) {
    return (
        <div className="min-w-0">
            <label htmlFor={htmlFor} className="block mb-1.5 text-sm font-medium">
                {label}
            </label>
            {children}
            {hint && <p className="mt-1.5 text-xs text-muted-foreground">{hint}</p>}
        </div>
    );
}

function Check({ checked, onChange, label }: { checked: boolean; onChange: (value: boolean) => void; label: string }) {
    return (
        <label className="flex items-center gap-2.5 h-8 text-sm cursor-pointer">
            <input
                type="checkbox"
                checked={checked}
                onChange={(e) => onChange(e.target.checked)}
                className="w-4 h-4 shrink-0"
            />
            {label}
        </label>
    );
}

// One book in a series and one without, so the preview shows how an empty
// field drops out of the name.
const SAMPLE_BOOKS: TemplateValues[] = [
    { author: 'Frank Herbert', title: 'Dune Messiah', series: 'Dune', series_index: '2', narrator: 'Scott Brick', year: '1969' },
    { author: 'Andy Weir', title: 'Project Hail Mary', narrator: 'Ray Porter', year: '2021' },
];

function OutputTemplateField({ value, onChange }: { value: string; onChange: (template: string) => void }) {
    const { activeAudiobook } = useApp();
    const inputRef = useRef<HTMLInputElement>(null);
    const caret = useRef<number | null>(null);
    const unknown = unknownFields(value);
    const preset = TEMPLATE_PRESETS.find((p) => p.template === value.trim())?.template ?? 'custom';
    const examples = activeAudiobook ? [templateValues(activeAudiobook), SAMPLE_BOOKS[0]] : SAMPLE_BOOKS;

    // Inserts at the caret, so a field can go between two others.
    const insert = (token: string) => {
        const input = inputRef.current;
        const text = `{${token}}`;
        const start = input?.selectionStart ?? value.length;
        const end = input?.selectionEnd ?? value.length;
        caret.current = start + text.length;
        onChange(value.slice(0, start) + text + value.slice(end));
    };

    // Put the caret after the inserted field once the new value is in the input.
    useLayoutEffect(() => {
        if (caret.current === null) return;
        inputRef.current?.focus();
        inputRef.current?.setSelectionRange(caret.current, caret.current);
        caret.current = null;
    }, [value]);

    return (
        <Field
            label="File and folder names"
            htmlFor="output-template"
            hint="Use / to make folders inside the output folder. A field with no value drops out, along with the separators around it."
        >
            <div className="flex gap-2">
                <Input
                    ref={inputRef}
                    id="output-template"
                    className="h-9 font-mono text-[13px]"
                    value={value}
                    onChange={(e) => onChange(e.target.value)}
                    placeholder={DEFAULT_OUTPUT_TEMPLATE}
                    spellCheck={false}
                    autoComplete="off"
                />
                <Select
                    aria-label="Naming presets"
                    className="w-48 shrink-0"
                    value={preset}
                    onChange={(e) => e.target.value !== 'custom' && onChange(e.target.value)}
                >
                    {TEMPLATE_PRESETS.map((p) => (
                        <option key={p.template} value={p.template}>
                            {p.label}
                        </option>
                    ))}
                    <option value="custom" disabled>
                        Custom
                    </option>
                </Select>
            </div>

            <div className="mt-2 flex flex-wrap items-center gap-1.5 no-select">
                <span className="mr-0.5 text-xs text-muted-foreground">Insert</span>
                {TEMPLATE_FIELDS.map((field) => (
                    <button
                        key={field.token}
                        type="button"
                        // Keeps focus (and the caret) in the template input.
                        onMouseDown={(e) => e.preventDefault()}
                        onClick={() => insert(field.token)}
                        className="h-6 px-2 rounded border text-xs transition-colors hover:bg-accent"
                    >
                        {field.label}
                    </button>
                ))}
            </div>

            {unknown.length > 0 && (
                <p className="mt-2 text-xs text-warning">
                    {unknown.map((name) => `{${name}}`).join(', ')} {unknown.length === 1 ? 'is not a field' : 'are not fields'}{' '}
                    and will be written as typed.
                </p>
            )}

            <div className="mt-3 rounded-md border bg-surface-2 px-3 py-2">
                <p className="mb-1 text-[10px] font-semibold uppercase tracking-wider text-muted-foreground no-select">
                    Preview
                </p>
                {examples.map((values, i) => {
                    const path = renderOutputPath('', value, values).slice(1);
                    return (
                        <p key={i} className="font-mono text-xs leading-relaxed truncate" title={path}>
                            {path}
                        </p>
                    );
                })}
            </div>
        </Field>
    );
}
