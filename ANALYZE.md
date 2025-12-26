# Audio Normalization Feature - Implementation TODO List

## Overview

The Audio Normalization panel is partially functional but has gaps that need to be addressed for production readiness.

```
Current State                          Target State
+---------------------------+          +---------------------------+
| [x] Analyze Audio         |          | [x] Analyze Audio         |
| [x] Display LUFS/LU/dBTP  |          | [x] Display LUFS/LU/dBTP  |
| [x] Show recommendation   |          | [x] Show recommendation   |
| [x] Editable targets      |          | [x] Editable targets      |
| [x] Apply normalization   |          | [x] Apply normalization   |
| [ ] Progress feedback     |    -->   | [x] Progress feedback     |
| [ ] Success notification  |          | [x] Success notification  |
| [ ] Settings persistence  |          | [x] Settings persistence  |
| [ ] Pipeline integration  |          | [x] Pipeline integration  |
+---------------------------+          +---------------------------+
```

---

## Task 1: Fix LRA Default Mismatch

**Priority:** Critical (Quick Fix)

**Problem:** Frontend default `target_lra: 7` does not match backend default `11`

**File:** `src-ui/src/components/NormalizationPanel.tsx`

**Steps:**
- [x] 1.1 Open `NormalizationPanel.tsx`
- [x] 1.2 Locate line 19: `target_lra: 7`
- [x] 1.3 Change to `target_lra: 11`
- [x] 1.4 Verify the change compiles without error

---

## Task 2: Add Success Notification After Normalization

**Priority:** High

**Problem:** User receives no feedback when normalization completes successfully

**Files:**
- `src-ui/src/components/NormalizationPanel.tsx`
- `src-ui/src/components/ui/toast.tsx` (verify exists)

**Steps:**
- [x] 2.1 Check if toast component exists at `src-ui/src/components/ui/toast.tsx`
- [x] 2.2 If toast exists, import the toast hook into NormalizationPanel
- [x] 2.3 In `handleApplyNormalization`, after successful API call:
  ```typescript
  toast({
    title: "Normalization Complete",
    description: `Output saved to: ${outputPath}`,
  });
  ```
- [x] 2.4 Replace inline error display with error toast:
  ```typescript
  toast({
    title: "Normalization Failed",
    description: error.message,
    variant: "destructive",
  });
  ```
- [ ] 2.5 Test success and error scenarios

---

## Task 3: Improve Output Path Handling

**Priority:** High

**Problem:** Output path uses naive `_normalized` suffix; user cannot choose location

**Files:**
- `src-ui/src/components/NormalizationPanel.tsx`
- `src-ui/src/api/files.ts` (for save dialog)

**Steps:**
- [x] 3.1 Check if file save dialog API exists (look for `save` from `@tauri-apps/plugin-dialog`)
- [x] 3.2 Import save dialog function into NormalizationPanel
- [x] 3.3 Modify `handleApplyNormalization`:
  ```typescript
  const outputPath = await save({
    defaultPath: inputPath.replace(/(\.[^.]+)$/, "_normalized$1"),
    filters: [{ name: 'Audio', extensions: ['m4a', 'mp3', 'wav'] }]
  });
  if (!outputPath) return; // User cancelled
  ```
- [x] 3.4 Store selected output path in state for success notification
- [ ] 3.5 Test file dialog opens and saves to correct location

---

## Task 4: Persist Normalization Settings to User Preferences

**Priority:** Medium

**Problem:** Settings reset on component remount; not saved to user preferences

**Files:**
- `src-tauri/src/config.rs`
- `src-ui/src/types/models.ts`
- `src-ui/src/components/NormalizationPanel.tsx`

**Steps:**
- [x] 4.1 Check `src-tauri/src/config.rs` for UserSettings struct
- [x] 4.2 Add normalization fields to UserSettings:
  ```rust
  pub normalization_target_lufs: f32,
  pub normalization_target_true_peak: f32,
  pub normalization_target_lra: f32,
  pub normalization_two_pass: bool,
  ```
- [x] 4.3 Add default values in Default impl
- [x] 4.4 Update TypeScript types in `src-ui/src/types/models.ts`
- [x] 4.5 In NormalizationPanel, load settings on mount:
  ```typescript
  useEffect(() => {
    loadUserSettings().then(settings => {
      if (settings.normalization_target_lufs) {
        setSettings({
          target_lufs: settings.normalization_target_lufs,
          target_true_peak: settings.normalization_target_true_peak,
          target_lra: settings.normalization_target_lra,
          two_pass: settings.normalization_two_pass,
        });
      }
    });
  }, []);
  ```
- [x] 4.6 Add debounced save when settings change
- [ ] 4.7 Test settings persist across app restarts

---

## Task 5: Add Progress Feedback During Normalization

**Priority:** Medium

**Problem:** UI shows only "Applying..." with no progress indication

**Files:**
- `src-tauri/src/audio/quality.rs`
- `src-tauri/src/commands/quality.rs`
- `src-ui/src/components/NormalizationPanel.tsx`

**Backend Steps:**
- [x] 5.1 Create progress event struct:
  ```rust
  #[derive(Clone, Serialize)]
  pub struct NormalizationProgress {
      pub percent: f32,
      pub stage: String, // "analyzing" | "normalizing"
  }
  ```
- [x] 5.2 Modify `apply_normalization` signature to accept `AppHandle`
- [x] 5.3 Use `std::process::Command` with `Stdio::piped()` for stderr
- [x] 5.4 Spawn thread to read stderr and parse `time=HH:MM:SS.ms` pattern
- [x] 5.5 Calculate progress: `current_time_ms / total_duration_ms * 100`
- [x] 5.6 Emit events: `handle.emit("normalization_progress", progress)`
- [x] 5.7 Update Tauri command to pass AppHandle

**Frontend Steps:**
- [x] 5.8 Add progress state: `const [progress, setProgress] = useState<number | null>(null)`
- [x] 5.9 Add event listener in useEffect:
  ```typescript
  const unlisten = await listen<NormalizationProgress>('normalization_progress', (event) => {
    setProgress(event.payload.percent);
  });
  ```
- [x] 5.10 Import and use Progress component from ui/progress
- [x] 5.11 Show progress bar when `isApplying && progress !== null`
- [x] 5.12 Reset progress to null on completion
- [ ] 5.13 Test progress updates during normalization

---

## Task 6: Integrate Normalization into Conversion Pipeline

**Priority:** Low (Enhancement)

**Problem:** Normalization is standalone; doesn't integrate with main M4B conversion

**Files:**
- `src-tauri/src/conversion/pipeline.rs` or `batch.rs`
- `src-tauri/src/models.rs` (ConversionConfig)
- `src-ui/src/components/ConversionSettings.tsx` (or equivalent)

**Backend Steps:**
- [x] 6.1 Add to ConversionConfig:
  ```rust
  pub normalize_audio: bool,
  pub normalization_settings: Option<NormalizationSettings>,
  ```
- [x] 6.2 In conversion pipeline, before FFmpeg encoding:
  ```rust
  if config.normalize_audio {
      let settings = config.normalization_settings.unwrap_or_default();
      let temp_normalized = temp_dir.join("normalized_input.wav");
      apply_normalization(&input, &temp_normalized, &settings)?;
      input = temp_normalized;
  }
  ```
- [x] 6.3 Clean up temp normalized file after conversion (handled by TempDirectory)

**Frontend Steps:**
- [x] 6.4 Add checkbox in conversion UI: "Normalize audio before conversion"
- [x] 6.5 When enabled, show inline normalization settings
- [x] 6.6 Pass settings to conversion API
- [ ] 6.7 Test full conversion with normalization enabled

---

## Implementation Order

```
Phase 1: Quick Fixes
|
+-- Task 1: Fix LRA Default
|
+-- Task 2: Success Notification
|
+-- Task 3: Output Path Dialog

Phase 2: User Experience
|
+-- Task 4: Settings Persistence
|
+-- Task 5: Progress Feedback

Phase 3: Full Integration
|
+-- Task 6: Pipeline Integration
```

---

## Verification Checklist

After completing all tasks:

- [ ] LRA default matches between frontend and backend (11 LU)
- [ ] Success toast appears with output path after normalization
- [ ] Error toast appears on failure
- [ ] File save dialog allows user to choose output location
- [ ] Settings persist across app restarts
- [ ] Progress bar shows during normalization
- [ ] Conversion pipeline can optionally normalize before encoding
- [ ] All TypeScript compiles without errors (`npm run build`)
- [ ] All Rust compiles without errors (`cargo check`)
- [ ] Manual test: analyze -> adjust settings -> apply -> verify output
