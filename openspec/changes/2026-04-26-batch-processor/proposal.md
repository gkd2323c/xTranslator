## Why

Current xTranslator processes one file at a time. Translators working with multiple mods (follower packs, quest series, city overhauls) must repeat: load ESP → AI translate → save strings → next file. A 10-mod translation session requires 30+ manual steps.

Batch processor automates this: configure file list once, run, walk away.

## What Changes

- New `BatchExecutor` module in `src-tauri/src/batch.rs` — background sequential file processor, independent of `AppState`
- 4 new IPC commands: `start_batch_translate`, `start_batch_export`, `get_batch_status`, `cancel_batch_job`
- 3 new events: `batch-progress`, `batch-file-complete`, `batch-complete`
- New `BatchPanel` React component — file queue, progress bars, start/cancel controls
- New DTOs: `BatchConfig`, `BatchEntry`, `BatchStatus`, `BatchProgress`

### Out of scope (future)

- Parallel file processing (memory + API rate limit concerns)
- Batch SST merge / batch import XML
- Batch job history / persistence across sessions
- Post-batch quality review workflow

## Capabilities

### New

- `batch-processing`: Background sequential processing of ESP files for translation and export

## Impact

- `src-tauri/src/batch.rs` (new)
- `src-tauri/src/commands.rs` (new batch commands)
- `src-tauri/src/main.rs` (register commands + manage BatchExecutor state)
- `crates/xt-shared/src/dto.rs` (new DTOs)
- `ui/src/api/strings.ts` (TypeScript types + invoke wrappers)
- `ui/src/components/BatchPanel.tsx` (new)
- `ui/src/App.tsx` (add BatchPanel to layout)
- `ui/src/stores/appStore.ts` (batch state + event listeners)
- `SPEC.md` (T19 status update)
