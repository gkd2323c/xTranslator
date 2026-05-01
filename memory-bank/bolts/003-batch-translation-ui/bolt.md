---
id: 003-batch-translation-ui
unit: 003-batch-translation-ui
intent: 001-batch-translation
type: simple-construction-bolt
status: complete
stories:
  - 001-batch-control-bar
  - 002-live-progress-display
  - 003-cancel-translation
  - 004-recovery-prompt
created: 2026-05-01T12:00:00Z
started: 2026-05-01T12:00:00Z
completed: 2026-05-01T12:00:00Z
current_stage: null
stages_completed:
  - name: plan
    completed: 2026-05-01T12:00:00Z
    artifact: implementation-plan.md
  - name: implement
    completed: 2026-05-01T12:00:00Z
    artifact: implementation-walkthrough.md
  - name: test
    completed: 2026-05-01T12:00:00Z
    artifact: test-walkthrough.md

requires_bolts:
  - 002-translation-queue
enables_bolts: []
requires_units:
  - 001-translation-queue
  - 002-translation-cache
blocks: false

complexity:
  avg_complexity: 1
  avg_uncertainty: 1
  max_dependencies: 1
  testing_scope: 2
---

# Bolt: 003-batch-translation-ui

## Overview

Frontend UI for batch translation. Includes the control bar (button + concurrency slider), real-time progress display, cancel flow, and crash recovery prompt.

## Objective

Build the user-facing batch translation interface — toolbar controls, live progress, cancel functionality, and recovery modal — that integrates with the backend queue and cache.

## Stories Included

- **001-batch-control-bar**: Batch translate button + concurrency slider (Must)
- **002-live-progress-display**: Real-time progress and completion summary (Should)
- **003-cancel-translation**: Cancel in-progress batch (Should)
- **004-recovery-prompt**: Startup recovery modal (Should)

## Bolt Type

**Type**: Simple Construction Bolt
**Definition**: `.specsmd/aidlc/templates/construction/bolt-types/simple-construction-bolt.md`

## Stages

- [ ] **1. plan**: Pending → Implementation plan (component tree, state design)
- [ ] **2. implement**: Pending → React components + Zustand extensions
- [ ] **3. test**: Pending → Vitest unit tests + manual QA

## Dependencies

### Requires
- 002-translation-queue (IPC commands and events must exist)
- 001-translation-cache (recovery detection IPC must exist)

### Enables
- None (final bolt in intent)

## Success Criteria

- [ ] Batch translate button works with selection
- [ ] Concurrency slider controls queue concurrency
- [ ] Progress updates in real-time without blocking UI
- [ ] Cancel flow works end-to-end
- [ ] Recovery modal appears on startup when pending cache detected

## Notes

- Components: `BatchTranslateBar`, `ProgressIndicator`, `TranslationSummaryModal`, `RecoveryPromptModal`
- State: Zustand store extensions (`batchState`, `batchProgress`, `batchErrors`, `batchConcurrency`)
- Uses existing `updateItemTranslation(id, text)` for real-time table updates
- Icons: lucide-react (Play, Square, RefreshCw)
