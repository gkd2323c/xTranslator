# Design — Batch Processor

## Architecture

```
src-tauri/src/
├── main.rs          # +Arc<BatchExecutor> in manage()
├── commands.rs      # +4 batch IPC commands
└── batch.rs         # NEW — BatchExecutor, job types, processing loop

ui/src/
├── App.tsx                        # +<BatchPanel /> in layout
├── components/BatchPanel.tsx      # NEW
├── api/strings.ts                 # +batch types + invoke wrappers
└── stores/appStore.ts             # +batch state slice
```

## Core Decision: Independent State

`BatchExecutor` does NOT touch `AppState`. It reads ESP files, translates, saves — fully self-contained. This avoids:

- Memory contention (AppState holds 76K+ strings; batch shouldn't double)
- Lock conflicts (batch holds ESP parser output while user edits interactively)
- State corruption (batch can't accidentally mutate the interactive session)

```
AppState                  BatchExecutor
────────                  ─────────────
strings: Vec<>            job: Option<RunningJob>
file_info                 entries: Vec<BatchEntry>
is_dirty                  cancel_flag: AtomicBool
                          progress: BatchProgress
                          
Used by: EditorPanel      Used by: BatchPanel
          StringTable               (background task)
          SidePanel
```

## Data Flow

```
User adds files ──→ entries[] in BatchPanel state
     │
[Start] click ──→ start_batch_translate(entries, config) ──→ Rust
     │                                                            │
     │                                                    validate entries
     │                                                    spawn tokio::task
     │                                                            │
     ▼                                                            ▼
BatchPanel listens                              for each entry sequentially:
  batch-progress ←────────────────────────────── 1. parse ESP + strings
  batch-file-complete ←────────────────────────── 2. load SST if provided
  batch-complete ←─────────────────────────────── 3. filter untranslated
     │                                            4. translate one-by-one
     │                                               └── check cancel_flag
     ▼                                            5. save strings
Progress bars update                              6. emit file-complete
```

## Background Task Lifecycle

```
  Pending ──→ Running ──→ Completed
                │  ↑
                │  └── Paused (stretch)
                │
                ├──→ Cancelled (user request)
                └──→ Failed (fatal error, e.g. no API key)
```

Cancel is cooperative: flag checked between files and between API calls within a file. Finishes current API call, then stops. Never leaves partial writes — save_strings is atomic per file.

## Error Handling Strategy

| Error type | Action |
|------------|--------|
| Single string translation fails | Log warning, leave string untranslated, continue |
| ESP parse fails for one file | Log error, skip file, continue to next |
| API key invalid/expired | Abort entire batch immediately |
| Network timeout (30s) | Retry ×3, then skip string |
| Rate limit (429) | Wait 5s, retry once, then skip string |
| Disk write fails | Log error, skip file, continue |
| All files fail | Batch status = Failed |

## File Discovery: Two Modes

**Mode 1 — Add Files**: Multi-select file dialog (`.esp`, `.esm` filter). Auto-detect strings dir and language from each file's parent directory.

**Mode 2 — Scan Directory**: Pick a directory, find all `.esp`/`.esm` files recursively, auto-detect config. Useful for translating entire mod directories.

Auto-detection logic per file:
```
strings_dir = <esp_dir>/../Strings  if exists, else <esp_dir>
language    = detect from strings filenames (e.g. skyrim_english.* → "english")
game        = detect from directory structure (e.g. Data/SkyrimSE/ → SkyrimSE)
            fallback: prompt user or default "SkyrimSE"
```

## IPC Contract

```
Commands:
  start_batch_translate(config)   → { job_id: String }
    config: { entries[], provider, target_lang, skip_translated, sst_path? }
    
  start_batch_export(config)      → { job_id: String }
    config: { entries[], format: "xml"|"sst", output_dir }
    
  get_batch_status()              → BatchStatus | null
    
  cancel_batch_job()              → ()

Events:
  batch-progress:
    { job_id, file_path, stage, current_file: u32, total_files: u32, 
      strings_translated: u32, total_strings: u32, message }
    
  batch-file-complete:
    { job_id, file_path, translated: u32, skipped: u32, errors: u32, 
      duration_ms: u64 }
    
  batch-complete:
    { job_id, total_files: u32, success: u32, failed: u32, 
      total_translated: u32, total_errors: u32, duration_ms: u64,
      errors: [{file, message}] }
```

## Frontend Component Tree

```
App.tsx
├── MenuBar.tsx
│   └── [Batch] toggle button (shows/hides BatchPanel)
├── SidePanel.tsx
│   ├── record type filter (existing)
│   ├── statistics (existing)
│   └── BatchPanel.tsx              ← NEW (collapsible section)
│       ├── Header: title + progress summary
│       ├── Config bar: provider, target_lang, options
│       ├── FileList: scrollable, with status icons
│       │   └── FileRow: path, status icon, string count, progress
│       └── ActionBar: [Add Files] [Scan Folder] [Start] [Cancel]
├── StringTable.tsx (existing)
└── EditorPanel.tsx (existing)
```

## State: BatchSlice in Zustand

```typescript
interface BatchSlice {
  // Config (set before starting)
  batchEntries: BatchEntry[];
  batchProvider: string;
  batchTargetLang: string;
  batchSkipTranslated: boolean;
  
  // Runtime (updated by events)
  batchJobId: string | null;
  batchRunning: boolean;
  batchProgress: BatchProgress | null;
  batchFileStatuses: Map<string, FileBatchStatus>;  // path → status
  
  // Actions
  addBatchFiles: (entries: BatchEntry[]) => void;
  removeBatchFile: (path: string) => void;
  clearBatchEntries: () => void;
  setBatchConfig: (config) => void;
  startBatch: (config) => Promise<void>;
  cancelBatch: () => Promise<void>;
}
```

## UI States

### Empty
```
┌─ Batch Translate ──────────────────────┐
│                                         │
│   No files added yet                    │
│                                         │
│   [Add Files]  [Scan Directory]         │
└─────────────────────────────────────────┘
```

### Configured (idle)
```
┌─ Batch Translate ──────────────────────┐
│  Target: chinese   Provider: DeepL     │
│  ☑ Skip translated                     │
│  ────────────────────────────────────  │
│  ◉ emily_follower.esp    ~234 strings  │
│  ◉ sarah_follower.esp    ~189 strings  │
│  ◉ luna_follower.esp     ~312 strings  │
│  ────────────────────────────────────  │
│  [Add Files] [Scan Folder]  [▶ Start]  │
└─────────────────────────────────────────┘
```

### Running
```
┌─ Batch Translate ──────────────────────┐
│  Target: chinese   Provider: DeepL     │
│  Progress: ████████░░░░░░ 2/5 files    │
│  ────────────────────────────────────  │
│  ✓ emily_follower.esp    234 tr, 0 err │
│  ✓ sarah_follower.esp    189 tr, 2 err │
│  ◐ luna_follower.esp     156/312 str   │
│  ○ nova_follower.esp     waiting       │
│  ○ ayla_follower.esp     waiting       │
│  ────────────────────────────────────  │
│  [⏸ Pause]  [■ Cancel]                │
└─────────────────────────────────────────┘
```

### Completed
```
┌─ Batch Translate ──────────────────────┐
│  ✓ Complete: 5 files, 1204 strings     │
│  ⚠ 3 errors (see details)              │
│  Duration: 12m 34s                     │
│  ────────────────────────────────────  │
│  ✓ emily_follower.esp    234 tr        │
│  ✓ sarah_follower.esp    187 tr, 2 err │
│  ✓ luna_follower.esp    312 tr         │
│  ✓ nova_follower.esp    283 tr, 1 err  │
│  ✓ ayla_follower.esp    188 tr         │
│  ────────────────────────────────────  │
│  [New Batch]  [Add Files]              │
└─────────────────────────────────────────┘
```

## Risks

1. **Rate limiting**: DeepL free tier ~500K chars/month. Batch of 10 files × 20K chars untranslated = 200K → within limit. Pro tier unlimited. Mitigation: show estimated char count before starting.

2. **Memory**: 76K strings/file ≈ 15-20MB. Batch processes sequentially, so peak = 1 file. No memory risk.

3. **Long-running tasks**: Batch of 10 files with 2000 untranslated strings each at ~1s per API call = ~5.5 hours. Mitigation: cancel support, progress persistence (future).

4. **Conflict with interactive session**: If user has file X open in EditorPanel AND file X is in the batch queue. Mitigation: detect conflict on start, warn user, skip or abort.
