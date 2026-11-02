import { useEffect, useRef, useState, type ReactNode } from 'react';
import { useApp } from '@/context/useApp';
import { SUPPORTED_FORMATS_TEXT } from '@/lib/formats';
import { MOD_KEY } from '@/lib/format';
import { cn } from '@/lib/utils';

const SECTIONS = [
    { id: 'start', label: 'Getting started' },
    { id: 'batch', label: 'Many books at once' },
    { id: 'files', label: 'Files and order' },
    { id: 'chapters', label: 'Chapters' },
    { id: 'tags', label: 'Tags and cover' },
    { id: 'find-details', label: 'Find details' },
    { id: 'convert', label: 'Converting' },
    { id: 'queue', label: 'The queue' },
    { id: 'projects', label: 'Projects' },
    { id: 'shortcuts', label: 'Shortcuts' },
    { id: 'troubleshooting', label: 'Troubleshooting' },
] as const;

/**
 * One document with a contents rail, rather than tabs: help is read and
 * searched with ⌘F, and tabs hid most of it from both.
 */
export function HelpPage() {
    const { setView } = useApp();
    const [current, setCurrent] = useState<string>(SECTIONS[0].id);
    const bodyRef = useRef<HTMLDivElement>(null);

    // Highlight whichever section heading most recently crossed the top third.
    useEffect(() => {
        const root = bodyRef.current?.closest('.overflow-y-auto') ?? null;
        const observer = new IntersectionObserver(
            (entries) => {
                const visible = entries.filter((e) => e.isIntersecting);
                if (visible.length) setCurrent(visible[0].target.id);
            },
            { root, rootMargin: '0px 0px -66% 0px' }
        );
        bodyRef.current?.querySelectorAll('section[id]').forEach((el) => observer.observe(el));
        return () => observer.disconnect();
    }, []);

    const jump = (id: string) => {
        document.getElementById(id)?.scrollIntoView({ behavior: 'smooth', block: 'start' });
        setCurrent(id);
    };

    const toSettings = (
        <button
            onClick={() => setView('settings')}
            className="font-medium text-foreground underline underline-offset-4 decoration-border hover:decoration-foreground"
        >
            Settings
        </button>
    );

    return (
        <div className="max-w-4xl mx-auto px-6 pt-8 pb-24 grid grid-cols-[168px_1fr] gap-10">
            <nav className="sticky top-8 self-start no-select" aria-label="Help contents">
                <p className="mb-2 px-2 text-sm font-semibold">Help</p>
                <ul className="space-y-px">
                    {SECTIONS.map((section) => (
                        <li key={section.id}>
                            <button
                                onClick={() => jump(section.id)}
                                aria-current={current === section.id ? 'location' : undefined}
                                className={cn(
                                    'w-full text-left px-2 py-1 rounded-md text-sm transition-colors',
                                    current === section.id
                                        ? 'bg-accent text-foreground font-medium'
                                        : 'text-muted-foreground hover:text-foreground hover:bg-accent/50'
                                )}
                            >
                                {section.label}
                            </button>
                        </li>
                    ))}
                </ul>
            </nav>

            <div ref={bodyRef} className="min-w-0 max-w-[65ch] space-y-12 text-sm leading-relaxed text-foreground/90">
                <Section id="start" title="Getting started">
                    <p>
                        Drop audio files, or a whole folder, anywhere in the window. You can also use{' '}
                        <B>Choose files</B> or <B>Choose folder</B> on the start screen. Everything you add becomes one
                        book, which converts to a single M4B with chapters, tags and cover art.
                    </p>
                    <p>
                        Folders are read with all their subfolders. A book split into disc folders such as{' '}
                        <Code>CD1</Code> and <Code>CD2</Code> keeps disc order.
                    </p>
                    <List>
                        <li>Supported formats: {SUPPORTED_FORMATS_TEXT}. WMA, Audible AAX and Opus are not supported.</li>
                        <li>Converting needs FFmpeg. When none is found, {toSettings} can download it or use one you choose.</li>
                    </List>
                    <p>
                        The screen keeps everything about the open book in view: its files and chapters in the middle,
                        its cover and tags on the right, and the <B>Convert to M4B</B> bar along the bottom.
                    </p>
                </Section>

                <Section id="batch" title="Many books at once">
                    <p>
                        Drop a folder that holds several books, such as a whole library, and each book gets its own
                        row on the <B>Batch</B> page instead of being merged into one.
                    </p>
                    <List>
                        <li>
                            A folder is one book when it holds audio itself, or only disc folders like{' '}
                            <Code>CD1</Code>. Any other folder is searched for book folders inside it, so{' '}
                            <Code>Library/Author/Book</Code> works.
                        </li>
                        <li>
                            Each row shows what needs a look: no author, no cover, or a file already at the output path.
                            <B> Open</B> edits that book in full; the batch keeps your changes.
                        </li>
                        <li>
                            <B>Convert</B> queues every ticked book into one output folder, using the quality defaults and
                            file names from {toSettings}. Existing files are skipped unless you choose to replace them.
                        </li>
                        <li>
                            If a book was split wrongly, use <B>Combine into one book</B> from the batch menu. A folder of
                            finished single-file books can be split with <B>Split into one book per file</B>.
                        </li>
                    </List>
                </Section>

                <Section id="files" title="Files and order">
                    <p>The order of the Files list is the order listeners hear.</p>
                    <List>
                        <li>Drag a row by its handle to move it.</li>
                        <li>
                            <B>Sort</B> orders by filename (folder first, numbers counted properly), by track number, or by
                            duration.
                        </li>
                        <li>
                            Click a row to select it, {MOD_KEY}-click to select several, then <B>Remove</B>.
                        </li>
                        <li>Hover a row and press play to hear the file.</li>
                    </List>
                    <p>
                        A warning appears above the list when the filenames suggest a different order than the current
                        one. Moving or removing files rebuilds the book and recreates its chapters from the files; if you
                        renamed chapters, you are asked first.
                    </p>
                </Section>

                <Section id="chapters" title="Chapters">
                    <p>
                        Each file starts as one chapter, named by the file's title tag, or by its filename when the tag is missing or is just the book's title. Players use these to jump around the book.
                    </p>
                    <List>
                        <li>
                            <B>Generate</B> replaces them with chapter marks already inside the files, a CUE sheet, or a
                            chapter file (FFmetadata, Audacity labels or CSV).
                        </li>
                        <li>
                            <B>Export</B> saves the list as Audacity labels, FFmetadata or CSV.
                        </li>
                        <li>
                            <B>Clean up titles</B> strips track numbers and similar noise, or applies your own pattern.
                        </li>
                        <li>Double-click a title, or tab to it and press Enter, to rename it. Drag rows to reorder.</li>
                    </List>
                    <p>The editor warns about gaps, overlaps and chapters that run past the end of the book.</p>
                </Section>

                <Section id="tags" title="Tags and cover">
                    <p>
                        Tags are read from the source files on import and edited on the right. A field saves when you
                        leave it; Escape puts back what was there. <B>More fields</B> shows album, publisher, language,
                        ASIN, copyright, description and comments.
                    </p>
                    <List>
                        <li>Click the cover to choose an image.</li>
                        <li>
                            <B>Auto</B> uses art embedded in the files or an image beside them. This also runs once when a
                            book is imported.
                        </li>
                        <li>
                            <B>Find online</B> opens Find details, below.
                        </li>
                    </List>
                    <p>Every edit can be undone from the arrows in the top bar, or with {MOD_KEY}Z.</p>
                </Section>

                <Section id="find-details" title="Find details">
                    <p>
                        <B>Find details…</B> above the cover searches Audible for the book and fills in what it knows:
                        narrator, series and book number, summary, publisher, cover, ASIN and chapters. Books that are
                        not on Audible, such as public-domain recordings, can be searched on Open Library instead.
                    </p>
                    <List>
                        <li>
                            Pick a result to review it. Each value is shown beside what the book has now. Empty fields
                            start ticked; values you already have start unticked, so nothing you typed is replaced
                            unless you tick it.
                        </li>
                        <li>
                            Chapters from Audible are offered only when your files match that edition&apos;s length, or
                            match it with the Audible intro and outro removed. With a different edition but the same
                            number of chapters, only the titles are offered. Otherwise the review says why not.
                        </li>
                        <li>Applying is one step: a single undo puts back every field, the cover and the chapters.</li>
                        <li>
                            Typing an ASIN instead of a title goes straight to that book, and a book that already carries
                            an ASIN is offered as an exact match.
                        </li>
                    </List>
                    <p>
                        A search sends the title and author you typed to Audible (Amazon) or Open Library, and the
                        chosen book&apos;s ASIN to Audnexus (api.audnex.us). Nothing is sent until you search, and
                        nothing on import. The Audible store is set in {toSettings}.
                    </p>
                </Section>

                <Section id="convert" title="Converting">
                    <p>
                        The strip above <B>Convert to M4B</B> shows whether the book is ready: its files, cover, chapters
                        and output file. A missing cover is allowed; the others must be in place.
                    </p>
                    <List>
                        <li>
                            <B>Output</B> defaults to the output folder from {toSettings}, named after the book. Click it to
                            choose another place. If a file is already there, you are asked before it is replaced.
                        </li>
                        <li>
                            <B>File and folder names</B> in {toSettings} can file books into folders, for example{' '}
                            <code className="font-mono text-xs">{'{author}/{series}/{title}'}</code>. A field the book
                            does not have drops out, so one pattern suits books with and without a series. The output
                            path follows tag edits until you choose a place by hand.
                        </li>
                        <li>
                            <B>Quality</B> sets bitrate, VBR or CBR, what to embed, and loudness normalization. 64k is
                            plenty for spoken word. Save a combination as a preset to reuse it.
                        </li>
                        <li>Estimated size and conversion time update as you change these.</li>
                    </List>
                    <p>
                        <B>Convert to M4B</B> adds the book to the queue and starts it, then shows the queue.
                    </p>
                </Section>

                <Section id="queue" title="The queue">
                    <p>
                        Books convert in the background, so you can open and prepare the next one while earlier ones run.
                        How many convert at once is set in {toSettings}. Jobs are kept if you quit and reopen the app.
                    </p>
                    <List>
                        <li>
                            <B>Start queue</B> and <B>Pause queue</B> control the whole queue; <B>Clear finished</B> tidies
                            the list.
                        </li>
                        <li>
                            Select a job to see its stage, progress and output path, and to <B>Cancel</B>, <B>Retry</B> or{' '}
                            <B>Show in folder</B>.
                        </li>
                    </List>
                </Section>

                <Section id="projects" title="Projects">
                    <p>
                        A project file (<Code>.abcproj</Code>) saves a book to finish later: its files and their order,
                        tags, chapters, cover and any project-specific conversion settings. Project commands are in the
                        menu under the book's title in the top bar.
                    </p>
                    <List>
                        <li>
                            <B>Save project as</B>, <B>Open project</B> and recently opened projects.
                        </li>
                        <li>
                            <B>Export chapter summary</B> writes the chapter list out for reference.
                        </li>
                        <li>
                            Per-project conversion settings are under <B>This project</B> in {toSettings} while a project is
                            open.
                        </li>
                    </List>
                </Section>

                <Section id="shortcuts" title="Shortcuts">
                    <dl className="grid grid-cols-[max-content_1fr] gap-x-6 gap-y-2">
                        <Shortcut keys={`${MOD_KEY}O`}>Open project</Shortcut>
                        <Shortcut keys={`${MOD_KEY}S`}>Save project</Shortcut>
                        <Shortcut keys={`${MOD_KEY}Z`}>Undo</Shortcut>
                        <Shortcut keys={MOD_KEY === '⌘' ? '⇧⌘Z' : 'Ctrl+Y'}>Redo</Shortcut>
                    </dl>
                </Section>

                <Section id="troubleshooting" title="Troubleshooting">
                    <Topic title="FFmpeg not found">
                        <p>
                            Nothing can convert until the app has FFmpeg. The Convert bar then shows <B>No FFmpeg</B>; click it to
                            open the FFmpeg section of {toSettings}.
                        </p>
                        <List>
                            <li>
                                On macOS and Windows, press <B>Get FFmpeg</B>. The app downloads about 30 MB, checks it, and keeps it
                                for its own use. <B>Remove</B> deletes it again.
                            </li>
                            <li>
                                If FFmpeg is already installed somewhere the app does not look, press <B>Choose FFmpeg…</B> and
                                pick the program.
                            </li>
                            <li>
                                Or install it yourself: macOS <Code>brew install ffmpeg</Code>, Windows{' '}
                                <Code>winget install ffmpeg</Code>, Linux <Code>sudo apt install ffmpeg</Code>. The Linux package
                                installs it with the app. Then press <B>Check again</B>.
                            </li>
                        </List>
                    </Topic>
                    <Topic title="Files will not import">
                        <List>
                            <li>Check the format is one of {SUPPORTED_FORMATS_TEXT}.</li>
                            <li>Play the file in another app to rule out a damaged file.</li>
                            <li>Make sure you can read the files and the folder they are in.</li>
                        </List>
                    </Topic>
                    <Topic title="A conversion failed">
                        <List>
                            <li>
                                Open the job in the queue and read the error; <B>Copy error</B> copies it for a bug report.
                            </li>
                            <li>Check the output folder exists, is writable and has free space.</li>
                            <li>
                                Press <B>Check again</B> under FFmpeg in {toSettings} to confirm FFmpeg still works.
                            </li>
                            <li>
                                Press <B>Retry</B>, or convert to a different output path.
                            </li>
                        </List>
                    </Topic>
                    <Topic title="Progress seems stuck">
                        <p>
                            Long books can take a while to report their first progress, and normalization adds a full
                            analysis pass before encoding starts. If nothing changes for several minutes, cancel and
                            retry the job.
                        </p>
                    </Topic>
                    <Topic title="Something else">
                        <p>Check the project's GitHub repository for known issues, or report a new one there.</p>
                    </Topic>
                </Section>
            </div>
        </div>
    );
}

function Section({ id, title, children }: { id: string; title: string; children: ReactNode }) {
    return (
        <section id={id} className="scroll-mt-8 space-y-3">
            <h2 className="text-lg font-semibold tracking-tight text-foreground">{title}</h2>
            {children}
        </section>
    );
}

function Topic({ title, children }: { title: string; children: ReactNode }) {
    return (
        <div className="space-y-2 pt-1">
            <h3 className="font-semibold text-foreground">{title}</h3>
            {children}
        </div>
    );
}

function List({ children }: { children: ReactNode }) {
    return <ul className="list-disc pl-5 space-y-1.5 marker:text-muted-foreground">{children}</ul>;
}

function B({ children }: { children: ReactNode }) {
    return <strong className="font-semibold text-foreground">{children}</strong>;
}

function Code({ children }: { children: ReactNode }) {
    return <code className="rounded bg-muted px-1 py-0.5 font-mono text-[0.85em]">{children}</code>;
}

function Shortcut({ keys, children }: { keys: string; children: ReactNode }) {
    return (
        <>
            <dt>
                <kbd className="inline-block min-w-[2.25rem] rounded border bg-surface px-1.5 py-0.5 text-center font-mono text-xs shadow-plane">
                    {keys}
                </kbd>
            </dt>
            <dd className="text-muted-foreground">{children}</dd>
        </>
    );
}
