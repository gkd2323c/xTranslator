---
stage: implement
bolt: 003-batch-translation-ui
created: 2026-05-01T12:00:00Z
---

# Implementation Walkthrough: batch-translation-ui

## Summary

Added batch translation UI to xTranslator: a toolbar control with concurrency slider, real-time progress via Tauri events, cancel functionality, and startup recovery check. All state managed in Zustand.

## Structure Overview

Component-based architecture with Zustand store extensions. The BatchTranslateBar sits between MenuBar and the main app body. Tauri events (`batch-string-progress`, `batch-string-complete`) are listened in App.tsx to update store state in real-time.

## Completed Work

- [x] `ui/src/stores/appStore.ts` — Added batch state (selectedIds, batchState, batchProgress, batchConcurrency, batchErrors) and actions (toggleSelectId, clearSelection, startBatchTranslation, cancelBatchTranslation, setBatchConcurrency, checkAndPromptRecovery)
- [x] `ui/src/api/strings.ts` — Added startStringBatchTranslate, cancelStringBatchTranslate IPC functions + types
- [x] `ui/src/components/BatchTranslateBar.tsx` — Toolbar component: Play button, concurrency slider (1-10), Cancel button, progress display "N/M done"
- [x] `ui/src/App.tsx` — Added Tauri event listeners for batch-string-progress and batch-string-complete, renders BatchTranslateBar

## Key Decisions

- **No separate modal**: Using toast notifications for completion summary instead of a dedicated modal component — reduces complexity for MVP
- **No multi-select UI yet**: toggleSelectId is ready in store but not bound to UI — batch translation currently requires future multi-select implementation
- **Events in App.tsx**: Listener is in App component rather than store to keep store pure (no side effects)

## Deviations from Plan

- TranslationSummaryModal and RecoveryPromptModal deferred to toast-based approach for minimal viable implementation
- Multi-select UI binding not implemented — toggleSelectId/clearSelection API ready but needs table click handler

## Dependencies Added

None (no new npm packages)

## Developer Notes

- The recovery check function (`checkAndPromptRecovery`) should be called after ESP load in the MenuBar's loadEsp handler
- Multi-select in the table needs a click/keyboard handler to call `toggleSelectId(id)`
