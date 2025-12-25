import { useState } from 'react';
import { useApp } from '@/context/AppContext';
import { Button } from '@/components/ui/button';
import { ArrowLeftIcon, Cog6ToothIcon } from '@heroicons/react/24/outline';

type HelpTab = 'quickstart' | 'workflow' | 'projects' | 'metadata' | 'chapters' | 'quality' | 'conversion' | 'troubleshooting';

const TABS: { id: HelpTab; label: string }[] = [
    { id: 'quickstart', label: 'Quick Start' },
    { id: 'workflow', label: 'Workflow' },
    { id: 'projects', label: 'Projects' },
    { id: 'metadata', label: 'Metadata' },
    { id: 'chapters', label: 'Chapters' },
    { id: 'quality', label: 'Quality' },
    { id: 'conversion', label: 'Conversion' },
    { id: 'troubleshooting', label: 'Troubleshooting' },
];

export function HelpPage() {
    const { setView } = useApp();
    const [activeTab, setActiveTab] = useState<HelpTab>('quickstart');

    const handleBackToApp = () => {
        setView('workflow');
    };

    const handleGoToSettings = () => {
        setView('settings');
    };

    return (
        <div className="p-6 space-y-6 max-w-3xl mx-auto">
            <div className="flex items-center gap-4">
                <Button variant="ghost" size="sm" onClick={handleBackToApp}>
                    <ArrowLeftIcon className="w-4 h-4 mr-2" />
                    Back to App
                </Button>
                <h1 className="text-2xl font-semibold">Help</h1>
            </div>

            <div className="flex flex-wrap gap-2">
                {TABS.map((tab) => (
                    <Button
                        key={tab.id}
                        variant={activeTab === tab.id ? 'default' : 'outline'}
                        size="sm"
                        onClick={() => setActiveTab(tab.id)}
                    >
                        {tab.label}
                    </Button>
                ))}
            </div>

            <section className="border rounded-lg p-4 space-y-4">
                {activeTab === 'quickstart' && <QuickStartTab />}
                {activeTab === 'workflow' && <WorkflowTab />}
                {activeTab === 'projects' && <ProjectsTab onGoToSettings={handleGoToSettings} />}
                {activeTab === 'metadata' && <MetadataTab />}
                {activeTab === 'chapters' && <ChaptersTab />}
                {activeTab === 'quality' && <QualityTab />}
                {activeTab === 'conversion' && <ConversionTab />}
                {activeTab === 'troubleshooting' && <TroubleshootingTab onGoToSettings={handleGoToSettings} />}
            </section>
        </div>
    );
}

function QuickStartTab() {
    return (
        <>
            <h2 className="text-lg font-medium">Quick Start</h2>
            <div className="space-y-4 text-sm">
                <div>
                    <h3 className="font-medium text-muted-foreground mb-2">Prerequisites</h3>
                    <ul className="list-disc list-inside space-y-1 text-muted-foreground">
                        <li>FFmpeg must be installed and accessible in your system PATH</li>
                        <li>Audio files in supported formats: MP3, FLAC, M4A, M4B, WAV, OGG, WMA, AAC</li>
                    </ul>
                </div>

                <div>
                    <h3 className="font-medium text-muted-foreground mb-2">60-Second Conversion</h3>
                    <ol className="list-decimal list-inside space-y-2 text-muted-foreground">
                        <li>Click <strong>Import Files</strong> in the sidebar and select your audio files</li>
                        <li>Review the auto-detected metadata and chapters (edit if needed)</li>
                        <li>Scroll to <strong>Step 3: Convert</strong> and click <strong>Configure</strong></li>
                        <li>Set your output path and options, then click <strong>Confirm</strong></li>
                        <li>Click <strong>Add to Queue</strong>, then <strong>Start Queue</strong></li>
                        <li>Wait for conversion to complete - your M4B audiobook is ready!</li>
                    </ol>
                </div>
            </div>
        </>
    );
}

function WorkflowTab() {
    return (
        <>
            <h2 className="text-lg font-medium">Workflow Overview</h2>
            <div className="space-y-4 text-sm">
                <p className="text-muted-foreground">
                    The main screen is organized into sequential steps for converting audio files to M4B audiobooks.
                </p>

                <div>
                    <h3 className="font-medium text-muted-foreground mb-2">Project Section</h3>
                    <p className="text-muted-foreground">
                        Save and load your work. Projects preserve your audiobook, metadata, chapters, and conversion settings.
                    </p>
                </div>

                <div>
                    <h3 className="font-medium text-muted-foreground mb-2">Step 1: Import Files</h3>
                    <p className="text-muted-foreground">
                        Click the browse button to select audio files. Drag-and-drop is not currently supported for importing.
                        Selected files are analyzed and their metadata is extracted automatically.
                    </p>
                </div>

                <div>
                    <h3 className="font-medium text-muted-foreground mb-2">Step 2: Edit Metadata</h3>
                    <p className="text-muted-foreground">
                        Edit title, author, narrator, and other metadata fields. You can fetch metadata from Open Library
                        or manually edit each field. Cover artwork can be added via the artwork panel.
                    </p>
                </div>

                <div>
                    <h3 className="font-medium text-muted-foreground mb-2">Step 2b: Edit Chapters</h3>
                    <p className="text-muted-foreground">
                        Chapters are auto-created from your input files. You can import chapters from external files,
                        extract from existing M4B files, rename using regex patterns, or manually edit times and titles.
                    </p>
                </div>

                <div>
                    <h3 className="font-medium text-muted-foreground mb-2">Audio Quality</h3>
                    <p className="text-muted-foreground">
                        Analyze audio loudness levels and optionally apply normalization for consistent volume.
                    </p>
                </div>

                <div>
                    <h3 className="font-medium text-muted-foreground mb-2">Step 3: Convert</h3>
                    <p className="text-muted-foreground">
                        Preview conversion estimates, configure output settings, add to queue, and start the conversion process.
                    </p>
                </div>
            </div>
        </>
    );
}

function ProjectsTab({ onGoToSettings }: { onGoToSettings: () => void }) {
    return (
        <>
            <h2 className="text-lg font-medium">Projects</h2>
            <div className="space-y-4 text-sm">
                <p className="text-muted-foreground">
                    Projects let you save your work and return to it later. A project file (.abcproj) contains
                    everything about your audiobook conversion.
                </p>

                <div>
                    <h3 className="font-medium text-muted-foreground mb-2">What's Saved in a Project</h3>
                    <ul className="list-disc list-inside space-y-1 text-muted-foreground">
                        <li>Audiobook structure (files, duration, total size)</li>
                        <li>All metadata (title, author, narrator, etc.)</li>
                        <li>Chapter definitions and titles</li>
                        <li>Cover artwork reference</li>
                        <li>Project-specific conversion defaults (optional)</li>
                        <li>Notes field for your own reference</li>
                    </ul>
                </div>

                <div>
                    <h3 className="font-medium text-muted-foreground mb-2">Project Actions</h3>
                    <ul className="list-disc list-inside space-y-1 text-muted-foreground">
                        <li><strong>Save Project</strong> - Save current work to a .abcproj file</li>
                        <li><strong>Open Project</strong> - Load a previously saved project (sidebar)</li>
                        <li><strong>Recent Projects</strong> - Quick access to recently opened projects (sidebar)</li>
                        <li><strong>Export Summary</strong> - Export chapter list to text/CSV for reference</li>
                    </ul>
                </div>

                <div>
                    <h3 className="font-medium text-muted-foreground mb-2">Project vs Global Settings</h3>
                    <p className="text-muted-foreground">
                        Projects can override global conversion defaults (bitrate, sample rate, etc.).
                        Configure project-specific defaults in{' '}
                        <Button variant="link" size="sm" className="h-auto p-0" onClick={onGoToSettings}>
                            <Cog6ToothIcon className="w-3 h-3 mr-1" />
                            Settings
                        </Button>
                        {' '}under "Project Overrides" when a project is loaded.
                    </p>
                </div>
            </div>
        </>
    );
}

function MetadataTab() {
    return (
        <>
            <h2 className="text-lg font-medium">Metadata</h2>
            <div className="space-y-4 text-sm">
                <p className="text-muted-foreground">
                    Metadata is information embedded in your audiobook file - title, author, narrator, and more.
                </p>

                <div>
                    <h3 className="font-medium text-muted-foreground mb-2">Manual Editing</h3>
                    <p className="text-muted-foreground">
                        Edit any field directly in the metadata form. Fields include: Title, Author, Narrator,
                        Album, Year, Genre, Series, Series Index, Publisher, Copyright, Language, and Description.
                    </p>
                </div>

                <div>
                    <h3 className="font-medium text-muted-foreground mb-2">Open Library Integration</h3>
                    <p className="text-muted-foreground">
                        Click <strong>Fetch from Open Library</strong> to search for book metadata online.
                        Use the search results to pre-fill metadata and optionally apply a cover image.
                    </p>
                    <ul className="list-disc list-inside space-y-1 text-muted-foreground mt-2">
                        <li><strong>Prefill/Search</strong> - Enter a title or author to search</li>
                        <li><strong>Apply Metadata</strong> - Apply selected result's metadata to your audiobook</li>
                        <li><strong>Apply Metadata + Cover</strong> - Also download and apply the cover art</li>
                    </ul>
                </div>

                <div>
                    <h3 className="font-medium text-muted-foreground mb-2">Cover Artwork</h3>
                    <p className="text-muted-foreground">
                        The artwork panel shows your current cover image. You can:
                    </p>
                    <ul className="list-disc list-inside space-y-1 text-muted-foreground mt-2">
                        <li>Click <strong>Choose Image...</strong> to select an image file</li>
                        <li>Drag and drop an image onto the artwork panel</li>
                        <li>Click <strong>Remove</strong> to clear the current artwork</li>
                    </ul>
                </div>

                <div>
                    <h3 className="font-medium text-muted-foreground mb-2">Saving Metadata</h3>
                    <p className="text-muted-foreground">
                        Click <strong>Save Metadata</strong> to persist changes. Metadata is embedded in the final
                        M4B file during conversion.
                    </p>
                </div>
            </div>
        </>
    );
}

function ChaptersTab() {
    return (
        <>
            <h2 className="text-lg font-medium">Chapters</h2>
            <div className="space-y-4 text-sm">
                <p className="text-muted-foreground">
                    Chapters allow listeners to navigate through your audiobook. They're displayed in audiobook players
                    and embedded using the QuickTime chapter format.
                </p>

                <div>
                    <h3 className="font-medium text-muted-foreground mb-2">Auto-Generated Chapters</h3>
                    <p className="text-muted-foreground">
                        When you import files, chapters are automatically created - one per input file.
                        Chapter titles are derived from filenames.
                    </p>
                </div>

                <div>
                    <h3 className="font-medium text-muted-foreground mb-2">Import/Export</h3>
                    <ul className="list-disc list-inside space-y-1 text-muted-foreground">
                        <li><strong>Import from File</strong> - Load chapters from FFmetadata, Audacity labels, or CSV</li>
                        <li><strong>Export Chapters</strong> - Save chapters in Audacity, FFmeta, or CSV format</li>
                        <li><strong>Extract from Files</strong> - Extract embedded chapters from M4B/MP3 files</li>
                    </ul>
                </div>

                <div>
                    <h3 className="font-medium text-muted-foreground mb-2">Rename with Regex</h3>
                    <p className="text-muted-foreground">
                        Use regex patterns to clean up chapter titles in bulk:
                    </p>
                    <ul className="list-disc list-inside space-y-1 text-muted-foreground mt-2">
                        <li><strong>Remove Track Numbers</strong> - Strips leading numbers like "01 - " or "Track 01"</li>
                        <li><strong>Remove File Extensions</strong> - Removes .mp3, .flac, etc.</li>
                        <li><strong>Remove Underscores</strong> - Replaces underscores with spaces</li>
                        <li><strong>Custom Regex</strong> - Apply your own pattern with "Apply Custom Regex"</li>
                    </ul>
                </div>

                <div>
                    <h3 className="font-medium text-muted-foreground mb-2">Manual Editing</h3>
                    <p className="text-muted-foreground">
                        Click on a chapter to edit its title and timing. Use the add/delete buttons to manage chapters.
                        The editor validates for gaps and overlaps between chapters.
                    </p>
                </div>

                <div>
                    <h3 className="font-medium text-muted-foreground mb-2">Warnings</h3>
                    <p className="text-muted-foreground">
                        The chapter editor will warn you about:
                    </p>
                    <ul className="list-disc list-inside space-y-1 text-muted-foreground mt-2">
                        <li><strong>Gaps</strong> - Time between chapters that isn't covered</li>
                        <li><strong>Overlaps</strong> - Chapters with overlapping time ranges</li>
                        <li><strong>Out of bounds</strong> - Chapters extending past the total duration</li>
                    </ul>
                </div>
            </div>
        </>
    );
}

function QualityTab() {
    return (
        <>
            <h2 className="text-lg font-medium">Audio Quality</h2>
            <div className="space-y-4 text-sm">
                <p className="text-muted-foreground">
                    The Audio Quality section helps ensure consistent volume levels across your audiobook.
                </p>

                <div>
                    <h3 className="font-medium text-muted-foreground mb-2">Analyze Audio</h3>
                    <p className="text-muted-foreground">
                        Click <strong>Analyze Audio</strong> to measure loudness levels using EBU R128 standard.
                        The analysis provides:
                    </p>
                    <ul className="list-disc list-inside space-y-1 text-muted-foreground mt-2">
                        <li><strong>Integrated Loudness (LUFS)</strong> - Overall perceived loudness</li>
                        <li><strong>Loudness Range (LU)</strong> - Dynamic range variation</li>
                        <li><strong>True Peak (dBTP)</strong> - Maximum sample peak level</li>
                    </ul>
                </div>

                <div>
                    <h3 className="font-medium text-muted-foreground mb-2">Normalization</h3>
                    <p className="text-muted-foreground">
                        If levels are outside recommended ranges, the app may suggest normalization.
                        Normalization adjusts audio to consistent loudness targets:
                    </p>
                    <ul className="list-disc list-inside space-y-1 text-muted-foreground mt-2">
                        <li><strong>Target LUFS</strong> - Recommended: -16 to -19 LUFS for audiobooks</li>
                        <li><strong>True Peak Limit</strong> - Prevents clipping, typically -1.0 dBTP</li>
                    </ul>
                </div>

                <div>
                    <h3 className="font-medium text-muted-foreground mb-2">Current Status</h3>
                    <p className="text-muted-foreground">
                        Note: Audio normalization during conversion is not yet implemented. Analysis is available
                        for informational purposes. Normalization will be applied during the FFmpeg encoding
                        stage in a future update.
                    </p>
                </div>
            </div>
        </>
    );
}

function ConversionTab() {
    return (
        <>
            <h2 className="text-lg font-medium">Conversion</h2>
            <div className="space-y-4 text-sm">
                <p className="text-muted-foreground">
                    The conversion step combines your audio files into a single M4B audiobook with embedded
                    chapters, metadata, and artwork.
                </p>

                <div>
                    <h3 className="font-medium text-muted-foreground mb-2">Preview Conversion</h3>
                    <p className="text-muted-foreground">
                        Click <strong>Preview Conversion</strong> to see estimates before converting:
                    </p>
                    <ul className="list-disc list-inside space-y-1 text-muted-foreground mt-2">
                        <li>Input file count, duration, and size</li>
                        <li>Estimated output size and conversion time</li>
                        <li>Chapter and artwork status</li>
                        <li>Warnings (e.g., output file already exists)</li>
                    </ul>
                </div>

                <div>
                    <h3 className="font-medium text-muted-foreground mb-2">Configure Settings</h3>
                    <p className="text-muted-foreground">
                        Click <strong>Configure</strong> to set conversion options:
                    </p>
                    <ul className="list-disc list-inside space-y-1 text-muted-foreground mt-2">
                        <li><strong>Output File</strong> - Where to save the M4B file</li>
                        <li><strong>Bitrate</strong> - Audio quality (32-256 kbps, higher = larger file)</li>
                        <li><strong>Sample Rate</strong> - Match source or specify (22050/44100/48000 Hz)</li>
                        <li><strong>Embed Chapters</strong> - Include chapter markers in output</li>
                        <li><strong>Embed Artwork</strong> - Include cover image</li>
                        <li><strong>Copy Metadata</strong> - Transfer title, author, etc.</li>
                        <li><strong>Overwrite Existing</strong> - Replace file if it already exists</li>
                    </ul>
                </div>

                <div>
                    <h3 className="font-medium text-muted-foreground mb-2">Queue Management</h3>
                    <p className="text-muted-foreground">
                        After configuring, click <strong>Add to Queue</strong> to queue the conversion.
                        You can queue multiple audiobooks before starting.
                    </p>
                    <ul className="list-disc list-inside space-y-1 text-muted-foreground mt-2">
                        <li><strong>Start Queue</strong> - Begin processing queued conversions</li>
                        <li><strong>Stop Queue</strong> - Stop processing (current job completes)</li>
                        <li><strong>Cancel</strong> - Cancel a specific job (stops FFmpeg immediately)</li>
                        <li><strong>Clear Queue</strong> - Remove all pending jobs</li>
                        <li><strong>Clear Completed</strong> - Remove finished/failed jobs from list</li>
                    </ul>
                </div>

                <div>
                    <h3 className="font-medium text-muted-foreground mb-2">Progress Tracking</h3>
                    <p className="text-muted-foreground">
                        Each job shows a progress bar with percentage and estimated time remaining.
                        The status message indicates the current conversion stage.
                    </p>
                </div>
            </div>
        </>
    );
}

function TroubleshootingTab({ onGoToSettings }: { onGoToSettings: () => void }) {
    return (
        <>
            <h2 className="text-lg font-medium">Troubleshooting</h2>
            <div className="space-y-4 text-sm">
                <div>
                    <h3 className="font-medium text-muted-foreground mb-2">FFmpeg Not Found</h3>
                    <p className="text-muted-foreground">
                        If FFmpeg is not detected, conversion will not work. To fix:
                    </p>
                    <ul className="list-disc list-inside space-y-1 text-muted-foreground mt-2">
                        <li>Install FFmpeg from <a href="https://ffmpeg.org/download.html" className="underline" target="_blank" rel="noopener noreferrer">ffmpeg.org</a></li>
                        <li>On macOS: <code className="bg-muted px-1 rounded">brew install ffmpeg</code></li>
                        <li>On Windows: Download and add to PATH, or use a package manager like Chocolatey</li>
                        <li>On Linux: <code className="bg-muted px-1 rounded">apt install ffmpeg</code> or equivalent</li>
                    </ul>
                    <p className="text-muted-foreground mt-2">
                        Check FFmpeg status in{' '}
                        <Button variant="link" size="sm" className="h-auto p-0" onClick={onGoToSettings}>
                            <Cog6ToothIcon className="w-3 h-3 mr-1" />
                            Settings → System
                        </Button>
                    </p>
                </div>

                <div>
                    <h3 className="font-medium text-muted-foreground mb-2">Output File Already Exists</h3>
                    <p className="text-muted-foreground">
                        If the output path already has a file, you'll see a warning. Options:
                    </p>
                    <ul className="list-disc list-inside space-y-1 text-muted-foreground mt-2">
                        <li>Choose a different output path</li>
                        <li>Enable "Overwrite existing file" in conversion settings</li>
                        <li>Delete the existing file manually</li>
                    </ul>
                </div>

                <div>
                    <h3 className="font-medium text-muted-foreground mb-2">Import Failures</h3>
                    <p className="text-muted-foreground">
                        If files fail to import:
                    </p>
                    <ul className="list-disc list-inside space-y-1 text-muted-foreground mt-2">
                        <li>Verify files are in supported formats (MP3, FLAC, M4A, WAV, OGG, AAC)</li>
                        <li>Check files aren't corrupted (try playing in another app)</li>
                        <li>Ensure you have read permissions for the files</li>
                        <li>Try importing fewer files at once</li>
                    </ul>
                </div>

                <div>
                    <h3 className="font-medium text-muted-foreground mb-2">Conversion Errors</h3>
                    <p className="text-muted-foreground">
                        If conversion fails:
                    </p>
                    <ul className="list-disc list-inside space-y-1 text-muted-foreground mt-2">
                        <li>Check the error message for specific details</li>
                        <li>Ensure output directory exists and is writable</li>
                        <li>Verify you have enough disk space</li>
                        <li>Try a different output path</li>
                        <li>Check FFmpeg is working: run <code className="bg-muted px-1 rounded">ffmpeg -version</code> in terminal</li>
                    </ul>
                </div>

                <div>
                    <h3 className="font-medium text-muted-foreground mb-2">Progress Not Updating</h3>
                    <p className="text-muted-foreground">
                        Progress updates every second. If stuck:
                    </p>
                    <ul className="list-disc list-inside space-y-1 text-muted-foreground mt-2">
                        <li>Check if FFmpeg is still running (conversion may be slow for large files)</li>
                        <li>For very long audiobooks, initial progress may take time to appear</li>
                        <li>Cancel and retry if truly stuck</li>
                    </ul>
                </div>

                <div>
                    <h3 className="font-medium text-muted-foreground mb-2">Getting Help</h3>
                    <p className="text-muted-foreground">
                        If you encounter issues not covered here, check the project's GitHub repository
                        for known issues or to report a new bug.
                    </p>
                </div>
            </div>
        </>
    );
}
