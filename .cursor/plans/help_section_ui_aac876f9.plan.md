---
name: Help section UI
overview: Add an in-app Help page (TSX-authored) that matches the Settings page styling and provides a complete end-to-end workflow/how-to guide, navigable via tabbed sections from the sidebar.
todos:
  - id: help-view-plumbing
    content: Add `help` to the app view union and wire Sidebar/App.tsx to switch/render Help.
    status: pending
  - id: help-page-shell
    content: Create `HelpPage.tsx` with Settings-matching layout + tab strip state.
    status: pending
    dependencies:
      - help-view-plumbing
  - id: help-content-tabs
    content: Author TSX help content for each tab (Quick Start, Workflow, Projects, Metadata, Chapters, Quality, Conversion, Troubleshooting).
    status: pending
    dependencies:
      - help-page-shell
  - id: help-polish
    content: "Optional: add in-app links/actions (e.g., jump to Settings) and tighten UX copy."
    status: pending
    dependencies:
      - help-content-tabs
---

# In-app Help Section (Settings-styled)

## Goals

- Add a dedicated **Help** view reachable from the sidebar, using the same **page layout + card styling** as the existing Settings page.
- Provide a **complete end-to-end how-to/workflow guide** that matches the app’s real UI (button labels, steps, and current limitations).
- Implement Help content as **TSX-only** with **tabbed sections** (no markdown rendering dependencies).

## What we’ll build

### Navigation / View plumbing (mirrors Settings)

- Extend the app-level view enum to include `help`.
- Wire the sidebar **Help** item to switch the view.
- Render `<HelpPage />` from `App.tsx` when `view === 'help'`.

### Help page UI + styling (match Settings)

- New component: [`/Users/anthonyizzo/Development/AudiobookConvertRs/src-ui/src/components/HelpPage.tsx`](/Users/anthonyizzo/Development/AudiobookConvertRs/src-ui/src/components/HelpPage.tsx)
- Layout will intentionally match `SettingsPage`:
- Container: `p-6 space-y-6 max-w-3xl mx-auto`
- Header row: **Back to App** ghost button + `Help` title
- Content blocks: `section` cards with `border rounded-lg p-4` and Settings-like subheadings (`text-sm font-medium text-muted-foreground`)

### Tabbed sections (TSX state)

Implement a simple tab strip (no extra UI library) using local state, e.g.:

- Tab buttons row: `flex flex-wrap gap-2` using `Button` variants to indicate active tab.
- Tabs (recommended default set):
- **Quick Start** (prereqs, 60-second flow)
- **Workflow** (Step 1 → Step 3, how the main screen is organized)
- **Projects** (Save/Open/Recent/Export Summary; what a project contains)
- **Metadata** (Manual fields, Open Library, Cover Artwork)
- **Chapters** (Import/Export/Extract/Regex cleanup; warnings for gaps/overlaps)
- **Quality** (Analyze audio; what “Apply Normalization” does today)
- **Conversion** (Preview Conversion, Configure, Add to Queue, Start/Stop, Cancel)
- **Troubleshooting** (FFmpeg not found, output exists warnings, import failures, etc.)

## Content outline (high-signal, UI-accurate)

Each tab will be written with the app’s real labels and behaviors, for example:

- **Import Files**: Clarify that click-to-browse is the supported path today (file drop is not implemented yet for input import).
- **Metadata**: “Fetch from Open Library” (Prefill/Search, Apply Metadata/+Cover), “Cover Artwork” (Choose Image…, drag-drop image onto the panel, Remove), “Save Metadata”.
- **Chapters**: “Import from File”, “Export Chapters” (Audacity/FFmeta/CSV), “Extract Chapters from Files”, regex preset buttons, “Apply Custom Regex”, and the gap/overlap warnings.
- **Conversion**: “Preview Conversion” (estimates, output-exists warning), queue “Configure” → “Add to Queue” → “Start Queue” with concurrency, Cancel job, Clear Queue/Clear Completed.
- **Settings** (referenced where relevant): global defaults vs project overrides; FFmpeg status lives in Settings → System.

## Files we’ll change

- **Add**: [`/Users/anthonyizzo/Development/AudiobookConvertRs/src-ui/src/components/HelpPage.tsx`](/Users/anthonyizzo/Development/AudiobookConvertRs/src-ui/src/components/HelpPage.tsx)
- **Update**: [`/Users/anthonyizzo/Development/AudiobookConvertRs/src-ui/src/context/AppContext.tsx`](/Users/anthonyizzo/Development/AudiobookConvertRs/src-ui/src/context/AppContext.tsx)
- Change `AppView` from `'workflow' | 'settings'` → `'workflow' | 'settings' | 'help'`
- **Update**: [`/Users/anthonyizzo/Development/AudiobookConvertRs/src-ui/src/App.tsx`](/Users/anthonyizzo/Development/AudiobookConvertRs/src-ui/src/App.tsx)
- Render `<HelpPage />` when `view === 'help'`
- **Update**: [`/Users/anthonyizzo/Development/AudiobookConvertRs/src-ui/src/components/Sidebar.tsx`](/Users/anthonyizzo/Development/AudiobookConvertRs/src-ui/src/components/Sidebar.tsx)
- Replace the current `console.log` TODO with `setView('help')`
- Add `active={view === 'help'}` so it highlights like Settings

## Optional polish (if you want it)

- Add small “Jump to Settings” buttons inside Help to switch to `setView('settings')` for FFmpeg troubleshooting.
- Add a “Copy diagnostics” card (FFmpeg status/version/path + app version) if we want help requests to be easier.

## Implementation todos

- `help-view-plumbing`: Add `help` to `AppView`, wire `Sidebar` + `App.tsx` to render Help.
- `help-page-shell`: Build `HelpPage` header + Settings-like container + tab strip.