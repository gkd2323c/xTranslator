---
id: 002-live-progress-display
unit: 003-batch-translation-ui
intent: 001-batch-translation
status: complete
implemented: true
priority: should
created: 2026-05-01T12:00:00Z
assigned_bolt: null
implemented: false
---

# Story: 002-live-progress-display

## User Story

**As a** translator
**I want** to see real-time progress during batch translation and a summary when complete
**So that** I know how much is done and which translations succeeded or failed

## Acceptance Criteria

- [ ] **Given** the batch is running, **When** `batch-progress` events arrive, **Then** a progress indicator shows "12/50 completed"
- [ ] **Given** a job completes successfully, **When** the event arrives, **Then** the corresponding table row updates its translation field in real-time
- [ ] **Given** the batch completes, **When** `batch-complete` event arrives, **Then** a summary modal shows: "Batch complete: 48 succeeded, 2 failed"
- [ ] **Given** the batch had failures, **When** the summary modal opens, **Then** the 2 failed entries are listed with their error reasons
- [ ] **Given** the batch is running, **When** I scroll/filter/sort the table, **Then** the UI remains responsive (no lag)

## Technical Notes

- Listen to Tauri events: `listen("batch-progress", callback)`, `listen("batch-complete", callback)`
- Update Zustand: `updateBatchProgress({ completed, total })` 
- Update individual string translation in table: `updateItemTranslation(id, text)` (already exists)
- Summary modal: new `TranslationSummaryModal` component
- Use Zustand selectors to avoid re-renders: `useAppStore(s => s.batchProgress)`
- Table virtualization (react-window) ensures scrolling performance

## Dependencies

### Requires
- 001-batch-control-bar (button triggers the batch)

## Edge Cases

| Scenario | Expected Behavior |
|----------|-------------------|
| Progress events arrive out of order | Completed count is cumulative, order doesn't matter |
| Batch finishes very fast (< 1s) | Still show summary modal (don't skip) |
| 0 failures | Summary shows "All 50 succeeded" (no error list) |
| User dismisses summary modal | Modal closes, batch state resets to idle |

## Out of Scope

- Cancel button behavior (003-cancel-translation)
- Recovery prompt (004-recovery-prompt)
