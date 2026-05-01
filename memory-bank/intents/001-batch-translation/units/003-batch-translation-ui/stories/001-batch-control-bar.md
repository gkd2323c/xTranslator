---
id: 001-batch-control-bar
unit: 003-batch-translation-ui
intent: 001-batch-translation
status: complete
implemented: true
priority: must
created: 2026-05-01T12:00:00Z
assigned_bolt: null
implemented: false
---

# Story: 001-batch-control-bar

## User Story

**As a** translator
**I want** a toolbar with a "Batch Translate" button and concurrency slider
**So that** I can configure and trigger batch translation from the UI

## Acceptance Criteria

- [ ] **Given** I have selected 1+ strings, **When** I view the toolbar, **Then** a "Batch Translate" button is visible and enabled
- [ ] **Given** I have selected 0 strings, **When** I view the toolbar, **Then** the "Batch Translate" button is disabled
- [ ] **Given** the toolbar is visible, **When** I view it, **Then** a concurrency slider (1-10, default 3) is shown next to the button
- [ ] **Given** I click "Batch Translate", **When** the button is clicked, **Then** `invoke("start_batch_translation", { ids, concurrency })` is called and button changes to "Translating..."
- [ ] **Given** the batch is running, **When** I view the toolbar, **Then** the concurrency slider is disabled and a "Cancel" button replaces the "Batch Translate" button

## Technical Notes

- Component: `BatchTranslateBar` in `ui/src/components/`
- Read concurrency from Zustand: `useAppStore(s => s.batchConcurrency)`
- Write concurrency to Zustand: `setBatchConcurrency(n)`
- Reuse `selectedIds` from existing appStore for selection tracking
- Icon: `Play` for translate, `Square` for cancel (lucide-react)

## Dependencies

### Requires
- None (first story in unit)

### Enables
- 002-live-progress-display
- 003-cancel-translation

## Edge Cases

| Scenario | Expected Behavior |
|----------|-------------------|
| No API Key configured | Button shows tooltip "Configure API Key first"; clicking shows toast warning |
| Click while already running | Ignored (button is disabled/state is "translating") |
| Resize window | Toolbar layout adapts, slider remains usable |
| Concurrency set to max (10) | Slider at max, value displayed |

## Out of Scope

- Progress display (002-live-progress-display)
- Translation execution (001-translation-queue)
