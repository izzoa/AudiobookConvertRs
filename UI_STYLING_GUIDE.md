# AudioBookConverter - UI Styling & CSS Guide

**For: Frontend Implementation (React + Tauri)**
**Purpose**: Design system and CSS/styling recommendations
**Version**: 1.0
**Last Updated**: December 17, 2025

---

## Frontend Framework Choice

### Recommended Stack: React + Tailwind CSS + shadcn/ui

**Why this combination for AudioBookConverter**:

| Criteria | React | Tailwind | shadcn/ui |
|----------|-------|----------|-----------|
| **Tauri Ecosystem Support** | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ |
| **Community Size** | Largest | Large | Growing |
| **Learning Curve** | Moderate | Easy | Easy |
| **Bundle Size** | ~40KB | ~10KB | ~5KB |
| **Dark Mode Support** | ✓ Built-in | ✓ Easy | ✓ Excellent |
| **Customization** | Good | Excellent | Very easy |
| **Desktop App Feel** | Good | Excellent | Excellent |

**What you get**:
- React: Component state management, hooks, ecosystem
- Tailwind: Utility-first CSS (faster development)
- shadcn/ui: Pre-built accessible components (buttons, dialogs, forms)

**Similar apps using this stack**:
- VS Code (different framework, but similar design)
- Many modern Electron apps
- Figma (different stack, but similar polish)

---

## Alternative Stacks (Ranked)

### 2nd Choice: Vue 3 + Tailwind + Naive UI
- **Best if**: You prefer Vue's simplicity
- **Pros**: Simpler learning curve, good component library
- **Cons**: Smaller community for Tauri
- **Use case**: Team familiar with Vue

### 3rd Choice: Svelte + Tailwind
- **Best if**: You want smallest bundle size
- **Pros**: Smallest output (~30KB total), compiler-based
- **Cons**: Smaller ecosystem, fewer UI libraries
- **Use case**: Performance-critical, experienced team

### 4th Choice: SolidJS + Tailwind
- **Best if**: You want fine-grained reactivity
- **Pros**: Modern, efficient, excellent performance
- **Cons**: Newer, smaller community
- **Use case**: Performance-obsessed developers

---

## NOT Recommended

❌ **Angular** - Too heavy for desktop app, overkill for UI
❌ **Material UI (web)** - Designed for web, not desktop feel
❌ **Bootstrap** - Heavier than Tailwind, web-first design
❌ **Plain HTML/CSS** - Too time-consuming, no component reuse

---

## Design System for AudioBookConverter

### Application Layout

```
┌─────────────────────────────────────────────────────┐
│ [≡]  AudioBook Converter              [_][□][✕]   │ ← Custom Titlebar
├────────────────┬──────────────────────────────────┤
│ Sidebar (220px)│ Main Content Area                │
│                │                                  │
│ ◆ Open Project │ ┌────────────────────────────┐  │
│ ◆ Recent Files │ │ Step 1: Import Files      │  │
│ ◆ Settings     │ │ ┌──────────────────────┐  │  │
│ ◆ Help         │ │ │ ◈ Drag files here   │  │  │
│                │ │ │   or click to        │  │  │
│                │ │ │   browse             │  │  │
│                │ │ └──────────────────────┘  │  │
│                │ └────────────────────────────┘  │
│                │ Step 2: Edit Metadata          │
│                │ ┌────────────────────────────┐ │
│                │ │ Title: [__________]        │ │
│                │ │ Author: [________]         │ │
│                │ │ Narrator: [_____]          │ │
│                │ └────────────────────────────┘ │
│                │ Step 3: Create Chapters       │
│                │ [+ Add Chapter] [Auto Split]  │
│                │ ┌────────────────────────────┐ │
│                │ │ #  Title    Time  Actions  │ │
│                │ │ 1  Chapter1 0:00  [✎][✕]  │ │
│                │ │ 2  Chapter2 12:30 [✎][✕]  │ │
│                │ └────────────────────────────┘ │
│                │ Step 4: Convert & Save        │
│                │ Bitrate: [128 ▼]              │
│                │ [         Convert Now        ] │
│                │ ├──────────────────────────┤  │
│                │ │ ████████░░░░░░░░ 65%   │ │  │ (Progress)
│                │ │ 2 min remaining          │  │
│                │ └────────────────────────────┘ │
│                │                                │
└────────────────┴──────────────────────────────────┘
```

### Color Palette

#### Light Mode
```
Primary Colors:
├─ Background:         #FAFAF9 (cream - warm off-white)
├─ Surface:            #FFFFFF (pure white)
├─ Surface (secondary):#F3F4F6 (light gray)
└─ Border:             #E4E4E7 (light border)

Semantic Colors:
├─ Primary Action:     #0891b2 (teal/cyan - buttons, links)
├─ Success:            #16A34A (green - checkmarks, done)
├─ Warning:            #EA580C (orange - caution)
├─ Error:              #DC2626 (red - errors, delete)
└─ Info:               #0284C7 (blue - informational)

Text Colors:
├─ Primary Text:       #18181B (dark gray/black)
├─ Secondary Text:     #71717A (medium gray)
└─ Disabled Text:      #A1A1AA (light gray)
```

#### Dark Mode
```
Primary Colors:
├─ Background:         #1F2937 (dark gray)
├─ Surface:            #111827 (very dark)
├─ Surface (secondary):#374151 (medium dark)
└─ Border:             #4B5563 (dark border)

Semantic Colors:
├─ Primary Action:     #22D3EE (bright cyan)
├─ Success:            #4ADE80 (bright green)
├─ Warning:            #FB923C (bright orange)
├─ Error:              #F87171 (bright red)
└─ Info:               #38BDF8 (bright blue)

Text Colors:
├─ Primary Text:       #F3F4F6 (light)
├─ Secondary Text:     #D1D5DB (medium light)
└─ Disabled Text:      #9CA3AF (medium gray)
```

#### Why These Colors?
- **Teal/Cyan**: Good contrast, not blue-light heavy, professional
- **Warm cream background**: Reduces eye strain for long conversions
- **Clear semantic colors**: Users instantly understand status

---

## Component Design Guide

### 1. File Input / Drag-Drop Area

**What it does**: Users drop/select audio files here

```jsx
<div className="
  border-2 border-dashed border-teal-500 dark:border-teal-400
  rounded-lg p-12 text-center
  hover:bg-teal-50 dark:hover:bg-teal-950
  transition-colors cursor-pointer
">
  <FileIcon className="w-16 h-16 mx-auto mb-4 text-teal-500" />
  <p className="text-lg font-semibold">Drop audiobooks here</p>
  <p className="text-sm text-gray-500 dark:text-gray-400">
    Supports MP3, FLAC, M4A, WAV, OGG
  </p>
  <p className="text-xs text-gray-400 dark:text-gray-500 mt-2">
    or click to browse
  </p>
  
  {/* Hidden file input */}
  <input type="file" multiple accept=".mp3,.flac,.m4a,.wav,.ogg" className="hidden" />
</div>
```

**CSS Requirements**:
- Min height: 150px (large click target)
- Animated on hover (color shift)
- Clear file types supported
- Hidden actual input
- Click-activated

### 2. Metadata Editing Panel

**What it does**: Edit title, author, narrator, artwork

```jsx
<div className="space-y-4 p-4">
  {/* Field Group */}
  <div>
    <label className="block text-sm font-semibold mb-1">Title</label>
    <input 
      className="
        w-full px-3 py-2
        border border-gray-300 dark:border-gray-600
        rounded-md
        bg-white dark:bg-gray-800
        text-black dark:text-white
        placeholder-gray-400 dark:placeholder-gray-500
        focus:outline-none focus:ring-2 focus:ring-teal-500
        transition-all
      "
      placeholder="Enter audiobook title"
      value={title}
      onChange={(e) => setTitle(e.target.value)}
    />
    {error && <p className="text-red-500 text-xs mt-1">{error}</p>}
  </div>

  {/* Similar for Author, Narrator */}

  {/* Artwork Upload */}
  <div>
    <label className="block text-sm font-semibold mb-2">Artwork</label>
    <div className="flex gap-3">
      {artwork && (
        <img 
          src={artwork} 
          alt="Artwork" 
          className="w-24 h-24 rounded object-cover"
        />
      )}
      <div className="flex-1">
        <button className="
          w-full border-2 border-dashed border-gray-300 dark:border-gray-600
          rounded p-4 text-center hover:border-teal-500
          transition-colors
        ">
          Upload Cover Art
        </button>
        <p className="text-xs text-gray-500 mt-1">Recommended: 500×500px, JPEG/PNG</p>
      </div>
    </div>
  </div>

  {/* Save Button */}
  <button className="
    w-full bg-teal-500 hover:bg-teal-600 dark:bg-teal-600 dark:hover:bg-teal-700
    text-white font-semibold py-2 rounded
    transition-colors active:opacity-80
    disabled:opacity-50 disabled:cursor-not-allowed
  ">
    Save Metadata
  </button>
</div>
```

**Key features**:
- Clear labels with small font
- Inline validation errors
- Artwork preview thumbnail
- Disabled state for buttons during save
- Smooth focus ring on inputs

### 3. Chapters Editor

**What it does**: Create, edit, reorder chapters

```jsx
<div className="border rounded-lg overflow-hidden">
  {/* Header Row */}
  <div className="
    grid grid-cols-12 gap-2 bg-gray-100 dark:bg-gray-800 p-3
    font-semibold text-sm border-b
  ">
    <div className="col-span-1">#</div>
    <div className="col-span-5">Title</div>
    <div className="col-span-4">Time Range</div>
    <div className="col-span-2">Actions</div>
  </div>

  {/* Chapter Rows (scrollable) */}
  <div className="max-h-64 overflow-y-auto">
    {chapters.map((ch, idx) => (
      <div 
        key={idx}
        className="
          grid grid-cols-12 gap-2 p-3 border-b
          hover:bg-gray-50 dark:hover:bg-gray-900
          transition-colors
        "
        draggable
        onDragStart={(e) => handleDragStart(e, idx)}
        onDrop={(e) => handleDrop(e, idx)}
      >
        <div className="col-span-1 text-gray-500">{idx + 1}</div>
        
        <input 
          className="col-span-5 bg-transparent border-0 px-0"
          value={ch.title}
          onChange={(e) => updateChapter(idx, 'title', e.target.value)}
          placeholder="Chapter title"
        />
        
        <div className="col-span-4 flex gap-1">
          <TimeInput value={ch.startTime} onChange={(v) => updateChapter(idx, 'start', v)} />
          <span className="text-gray-500">–</span>
          <TimeInput value={ch.endTime} onChange={(v) => updateChapter(idx, 'end', v)} />
        </div>
        
        <div className="col-span-2 flex gap-1">
          <button onClick={() => editChapter(idx)} className="text-blue-500 hover:text-blue-700">
            ✎
          </button>
          <button onClick={() => deleteChapter(idx)} className="text-red-500 hover:text-red-700">
            ✕
          </button>
        </div>
      </div>
    ))}
  </div>

  {/* Add Chapter Button */}
  <div className="p-3 bg-gray-50 dark:bg-gray-900 border-t">
    <button className="
      w-full text-teal-600 dark:text-teal-400 font-semibold py-1
      hover:bg-teal-50 dark:hover:bg-teal-950 rounded
      transition-colors
    ">
      + Add Chapter
    </button>
  </div>
</div>
```

**Key features**:
- Drag-to-reorder functionality
- Inline editing
- Time picker inputs
- Max height with scroll (not expanding page)
- Click to add new chapter

### 4. Conversion Progress Panel

**What it does**: Real-time progress during conversion

```jsx
<div className="space-y-4 p-4 bg-gray-50 dark:bg-gray-900 rounded-lg border">
  {/* Progress Bar */}
  <div>
    <div className="flex justify-between text-sm mb-2">
      <span className="font-semibold">Converting...</span>
      <span className="text-teal-600 dark:text-teal-400 font-semibold">
        {progress.toFixed(0)}%
      </span>
    </div>
    <div className="w-full bg-gray-200 dark:bg-gray-700 rounded-full h-2 overflow-hidden">
      <div 
        className="
          bg-gradient-to-r from-teal-500 to-teal-600
          h-full transition-all duration-300 ease-out
        "
        style={{ width: `${progress}%` }}
      />
    </div>
  </div>

  {/* Current File Info */}
  <div className="text-sm">
    <p className="text-gray-600 dark:text-gray-400">
      File 3 of 5: <span className="font-mono">{currentFile}</span>
    </p>
    <p className="text-gray-600 dark:text-gray-400 mt-1">
      ⏱ Time remaining: <span className="font-semibold">{timeRemaining}</span>
    </p>
    <p className="text-gray-500 dark:text-gray-500 text-xs mt-2">
      Speed: {speed}x • Bitrate: 128kbps
    </p>
  </div>

  {/* Control Buttons */}
  <div className="flex gap-2 pt-2">
    <button className="
      flex-1 bg-orange-500 hover:bg-orange-600
      text-white font-semibold py-2 rounded
      transition-colors active:opacity-80
    ">
      ⏸ Pause
    </button>
    <button className="
      flex-1 bg-red-500 hover:bg-red-600
      text-white font-semibold py-2 rounded
      transition-colors active:opacity-80
    ">
      ✕ Cancel
    </button>
  </div>

  {/* Status Messages */}
  {error && (
    <div className="
      p-3 bg-red-50 dark:bg-red-950 border border-red-200 dark:border-red-800
      rounded text-red-800 dark:text-red-200 text-sm
    ">
      ⚠ {error}
    </div>
  )}
</div>
```

**Key features**:
- Smooth progress bar animation
- Real-time updates
- Current file display
- Time remaining estimate
- Pause/Cancel buttons
- Error message display
- Speed indicator

### 5. Settings Panel

**What it does**: Configure app preferences

```jsx
<div className="space-y-6 p-4">
  {/* Setting Group */}
  <div className="border-b dark:border-gray-700 pb-4">
    <h3 className="text-lg font-semibold mb-4">Audio Quality</h3>
    
    <div className="space-y-3">
      <div>
        <label className="block text-sm font-medium mb-1">Bitrate</label>
        <select className="
          w-full border rounded px-3 py-2
          bg-white dark:bg-gray-800 text-black dark:text-white
          focus:ring-2 focus:ring-teal-500
        ">
          <option>128 kbps</option>
          <option>192 kbps</option>
          <option>256 kbps</option>
        </select>
      </div>

      <div>
        <label className="block text-sm font-medium mb-1">Sample Rate</label>
        <select className="w-full border rounded px-3 py-2 bg-white dark:bg-gray-800">
          <option>Auto</option>
          <option>44.1 kHz</option>
          <option>48 kHz</option>
        </select>
      </div>
    </div>
  </div>

  {/* Another Group */}
  <div className="border-b dark:border-gray-700 pb-4">
    <h3 className="text-lg font-semibold mb-4">Appearance</h3>
    
    <div>
      <label className="block text-sm font-medium mb-2">Theme</label>
      <div className="flex gap-2">
        {['Light', 'Dark', 'System'].map(theme => (
          <button
            key={theme}
            className={`
              px-4 py-2 rounded border transition-all
              ${selectedTheme === theme
                ? 'bg-teal-500 text-white border-teal-500'
                : 'border-gray-300 dark:border-gray-600 hover:border-teal-500'
              }
            `}
            onClick={() => setTheme(theme)}
          >
            {theme}
          </button>
        ))}
      </div>
    </div>
  </div>

  {/* Save Button */}
  <button className="
    w-full bg-teal-500 hover:bg-teal-600
    text-white font-semibold py-2 rounded
    transition-colors
  ">
    Save Settings
  </button>
</div>
```

**Key features**:
- Grouped by category
- Clear section headers
- Select dropdowns
- Theme toggle buttons
- Save button at bottom

---

## Tailwind CSS Configuration

### tailwind.config.js

```javascript
/** @type {import('tailwindcss').Config} */
export default {
  darkMode: ['class'], // Use class selector for dark mode
  content: [
    "./index.html",
    "./src/**/*.{js,ts,jsx,tsx}",
  ],
  theme: {
    extend: {
      colors: {
        // Custom semantic colors for app
        primary: {
          50: '#f0fdfa',
          500: '#0891b2',
          600: '#0e7490',
        },
      },
      spacing: {
        'sidebar': '220px',
      },
      fontFamily: {
        sans: ['Inter', 'system-ui', '-apple-system', 'sans-serif'],
        mono: ['Fira Code', 'ui-monospace', 'monospace'],
      },
      borderRadius: {
        sm: '0.375rem',
        md: '0.5rem',
        lg: '0.75rem',
      },
    },
  },
  plugins: [
    require('@tailwindcss/forms'),      // Better form styling
    require('@tailwindcss/typography'), // Rich text support
  ],
}
```

### postcss.config.js

```javascript
export default {
  plugins: {
    tailwindcss: {},
    autoprefixer: {},
  },
}
```

---

## Dark Mode Implementation

### Using Tailwind's Dark Mode

```jsx
// In main HTML (App.tsx)
import { useEffect, useState } from 'react'

export default function App() {
  const [darkMode, setDarkMode] = useState(
    localStorage.getItem('theme') === 'dark' ||
    window.matchMedia('(prefers-color-scheme: dark)').matches
  )

  useEffect(() => {
    const html = document.documentElement
    if (darkMode) {
      html.classList.add('dark')
      localStorage.setItem('theme', 'dark')
    } else {
      html.classList.remove('dark')
      localStorage.setItem('theme', 'light')
    }
  }, [darkMode])

  return (
    <div className="bg-white dark:bg-gray-900 text-black dark:text-white">
      {/* Use dark: prefix everywhere */}
    </div>
  )
}
```

### Component Pattern

```jsx
<div className="
  bg-white dark:bg-gray-900
  text-black dark:text-white
  border border-gray-300 dark:border-gray-700
  hover:bg-gray-50 dark:hover:bg-gray-800
  transition-colors
">
  Content
</div>
```

**Pattern**: Every color property gets a `dark:` variant

---

## Critical CSS Patterns for Desktop Apps

### 1. Prevent Scrollbar Jank

```css
/* Prevents layout shift when scrollbar appears */
html {
  scrollbar-gutter: stable;
}

/* Prevent overscroll glow on macOS/Linux */
body {
  overscroll-behavior: contain;
}
```

### 2. Custom Titlebar

```jsx
// In tauri.conf.json
{
  "windows": [{
    "fullscreen": false,
    "resizable": true,
    "title": "AudioBook Converter",
    "decorations": false,  // Remove native titlebar
    "transparent": true
  }]
}

// In React component
<div 
  className="
    h-8 bg-white dark:bg-gray-900 border-b
    flex items-center justify-between px-4
    select-none
  "
  data-tauri-drag-region  // Makes this area draggable
>
  <span className="font-semibold">AudioBook Converter</span>
  
  <div className="flex gap-2">
    <button onClick={minimize} className="hover:bg-gray-100 dark:hover:bg-gray-800 p-1 rounded">
      _
    </button>
    <button onClick={toggleMaximize} className="hover:bg-gray-100 dark:hover:bg-gray-800 p-1 rounded">
      □
    </button>
    <button onClick={close} className="hover:bg-red-500 text-red-500 hover:text-white p-1 rounded">
      ✕
    </button>
  </div>
</div>
```

### 3. Sidebar + Main Layout

```jsx
<div className="flex h-screen overflow-hidden">
  {/* Sidebar */}
  <aside className="
    w-sidebar
    bg-gray-50 dark:bg-gray-900
    border-r border-gray-200 dark:border-gray-800
    flex flex-col
  ">
    <nav className="flex-1 overflow-y-auto p-4 space-y-2">
      {/* Navigation items */}
    </nav>
  </aside>

  {/* Main content */}
  <main className="flex-1 overflow-y-auto bg-white dark:bg-gray-900">
    {/* Content */}
  </main>
</div>
```

### 4. Scrollable Sections with Fixed Headers

```jsx
<div className="flex flex-col h-full">
  {/* Header (fixed) */}
  <div className="px-4 py-3 border-b flex-shrink-0">
    <h2>Files</h2>
  </div>

  {/* Content (scrollable) */}
  <div className="flex-1 overflow-y-auto">
    {/* List items */}
  </div>
</div>
```

### 5. Input Focus Styling (Accessible)

```jsx
<input
  className="
    border border-gray-300 dark:border-gray-600
    focus:outline-none
    focus:ring-2 focus:ring-teal-500 focus:ring-offset-0
    dark:focus:ring-teal-400
    transition-all
    rounded px-3 py-2
  "
/>
```

---

## Icons

### Recommended Libraries

**Heroicons** (preferred for AudioBook Converter):
```bash
npm install @heroicons/react
```

```jsx
import { PlayIcon, PauseIcon, FolderIcon } from '@heroicons/react/24/solid'

<PlayIcon className="w-5 h-5" />  // 20x20
<PlayIcon className="w-6 h-6" />  // 24x24
```

**Tabler Icons** (alternative):
```bash
npm install @tabler/icons-react
```

### Icons Needed

```
Audio Control:
├─ PlayIcon - Play audio
├─ PauseIcon - Pause
├─ StopIcon - Stop
└─ SkipIcon - Skip chapter

File Management:
├─ FolderIcon - Browse
├─ DocumentIcon - File
├─ TrashIcon - Delete
└─ ArrowUpIcon - Upload

UI:
├─ Cog6ToothIcon - Settings
├─ QuestionMarkIcon - Help
├─ CheckIcon - Success
├─ XIcon - Error/Close
├─ PlusIcon - Add
└─ MinusIcon - Remove

States:
├─ SparkleIcon - Processing
├─ CheckCircleIcon - Complete
└─ ExclamationIcon - Warning
```

---

## Animation Guidelines

**Keep animations subtle** (don't distract during work):

```css
/* Page transitions */
.fade-enter {
  animation: fadeIn 0.2s ease-in;
}

@keyframes fadeIn {
  from { opacity: 0; }
  to { opacity: 1; }
}

/* Progress bar */
.progress {
  transition: width 0.3s cubic-bezier(0.4, 0, 0.2, 1);
}

/* Button hover */
.button {
  transition: background-color 0.15s, box-shadow 0.15s;
}

/* Slide-up modal */
.modal {
  animation: slideUp 0.25s ease-out;
}

@keyframes slideUp {
  from {
    opacity: 0;
    transform: translateY(12px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}
```

**Durations**:
- Instant: none
- Quick interactions: 150ms (hover, focus)
- Normal: 250-300ms (transitions)
- Slow: 500ms+ (only for important flows)

---

## Accessibility Checklist

✅ **Required**:
- [ ] Color contrast ≥ 4.5:1 for all text
- [ ] Focus indicators visible on all interactive elements
- [ ] ARIA labels on icon-only buttons
- [ ] Form inputs have associated labels
- [ ] Keyboard navigation works (Tab, Shift+Tab)
- [ ] Error messages linked to form fields
- [ ] Focus ring visible: `focus-visible:ring-2`

✅ **Strongly Recommended**:
- [ ] Dark mode support
- [ ] Respects `prefers-reduced-motion`
- [ ] Adjustable font size in settings
- [ ] Screen reader friendly
- [ ] Alt text on images
- [ ] Semantic HTML (buttons not divs)

### ARIA Example

```jsx
<button
  aria-label="Delete chapter 3"
  className="text-red-500 hover:text-red-700"
  onClick={handleDelete}
>
  ✕
</button>
```

---

## Performance & Bundle Size

### Target Metrics

- **CSS (production)**: 40-60 KB (gzipped)
- **JavaScript**: 150-200 KB (gzipped, excluding app code)
- **Total app bundle**: <10 MB

### How to Achieve

1. **Use Tailwind JIT mode** - Only generates used classes
2. **Tree-shake shadcn components** - Import only what you use
3. **Code splitting** - Lazy load heavy pages
4. **Image optimization** - Compress artwork previews
5. **Remove unused dependencies** - Audit regularly

```bash
# Check bundle size
npm install -D webpack-bundle-analyzer
# Then analyze your build
```

---

## Example Project Structure

```
src-ui/
├── src/
│   ├── App.tsx                 # Main app wrapper
│   ├── main.tsx                # React entry point
│   │
│   ├── pages/
│   │   ├── MainPage.tsx        # Primary workflow
│   │   ├── SettingsPage.tsx    # Settings panel
│   │   └── HelpPage.tsx        # Help/about
│   │
│   ├── components/
│   │   ├── common/
│   │   │   ├── Titlebar.tsx
│   │   │   ├── Sidebar.tsx
│   │   │   └── Button.tsx
│   │   │
│   │   ├── file-input/
│   │   │   ├── FileDropZone.tsx
│   │   │   └── FileList.tsx
│   │   │
│   │   ├── metadata/
│   │   │   ├── MetadataEditor.tsx
│   │   │   ├── ArtworkUpload.tsx
│   │   │   └── MetadataForm.tsx
│   │   │
│   │   ├── chapters/
│   │   │   ├── ChaptersEditor.tsx
│   │   │   ├── ChapterRow.tsx
│   │   │   └── ChapterForm.tsx
│   │   │
│   │   ├── conversion/
│   │   │   ├── ProgressPanel.tsx
│   │   │   ├── ConversionSettings.tsx
│   │   │   └── ProgressBar.tsx
│   │   │
│   │   └── settings/
│   │       ├── SettingsPanel.tsx
│   │       ├── AudioQualitySettings.tsx
│   │       └── ThemeSelector.tsx
│   │
│   ├── hooks/
│   │   ├── useConversion.ts    # Conversion state & logic
│   │   ├── useAudioBook.ts     # AudioBook state
│   │   ├── useSettings.ts      # Settings state
│   │   └── useDarkMode.ts      # Dark mode toggle
│   │
│   ├── services/
│   │   ├── api.ts              # Tauri command calls
│   │   ├── storage.ts          # LocalStorage helpers
│   │   └── ffmpeg.ts           # FFmpeg helpers
│   │
│   ├── types/
│   │   ├── audiobook.ts        # TypeScript interfaces
│   │   ├── conversion.ts
│   │   └── settings.ts
│   │
│   ├── styles/
│   │   ├── globals.css         # Global styles
│   │   ├── tailwind.css        # Tailwind imports
│   │   └── animations.css      # Custom animations
│   │
│   └── utils/
│       ├── format.ts           # Format utilities (time, size)
│       ├── validation.ts       # Form validation
│       └── constants.ts        # Constants
│
├── tailwind.config.js
├── postcss.config.js
├── tsconfig.json
├── vite.config.ts              # Vite config (with Tauri plugin)
├── package.json
└── index.html
```

---

## Setup Commands

### Create React + Tauri + Tailwind

```bash
# 1. Create Tauri app with React
npm create tauri-app@latest -- --template react --typescript

# 2. Install Tailwind
npm install -D tailwindcss postcss autoprefixer
npx tailwindcss init -p

# 3. Install shadcn/ui (optional but recommended)
npx shadcn-ui@latest init

# 4. Install Heroicons
npm install @heroicons/react

# 5. Install form handling (optional)
npm install react-hook-form

# 6. Install toast notifications (optional)
npm install sonner
```

### Basic App Structure

```tsx
// App.tsx
import { useEffect, useState } from 'react'
import { dark } from '@tauri-apps/api/window'

function App() {
  const [isDark, setIsDark] = useState(false)

  useEffect(() => {
    // Set initial theme
    const prefersDark = window.matchMedia('(prefers-color-scheme: dark)').matches
    setIsDark(prefersDark)
    updateTheme(prefersDark)
  }, [])

  const updateTheme = (dark: boolean) => {
    const html = document.documentElement
    if (dark) {
      html.classList.add('dark')
    } else {
      html.classList.remove('dark')
    }
  }

  return (
    <div className="flex flex-col h-screen bg-white dark:bg-gray-900">
      <Titlebar />
      <div className="flex flex-1 overflow-hidden">
        <Sidebar />
        <MainContent />
      </div>
    </div>
  )
}

export default App
```

---

## Next Steps

1. **Set up the React project** with commands above
2. **Configure Tailwind** with the provided config
3. **Implement Titlebar** component
4. **Build Sidebar** navigation
5. **Create FileDropZone** component
6. **Implement MetadataEditor**
7. **Build ChaptersEditor**
8. **Create ConversionProgress** panel
9. **Test dark mode** switching
10. **Verify accessibility** with axe DevTools

---

## Resources

- Tauri Docs: https://tauri.app/
- Tailwind CSS: https://tailwindcss.com/
- shadcn/ui: https://ui.shadcn.com/
- Heroicons: https://heroicons.com/
- React Docs: https://react.dev/

---

**This guide should take you from zero to professional-looking UI. Follow it step-by-step and you'll have a beautiful, accessible desktop application! 🎨**
