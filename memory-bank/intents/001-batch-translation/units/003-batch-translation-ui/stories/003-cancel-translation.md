---
id: 003-cancel-translation
unit: 003-batch-translation-ui
intent: 001-batch-translation
status: complete
implemented: true
priority: should
created: 2026-05-01T12:00:00Z
assigned_bolt: null
implemented: false
---

# Story: 003-cancel-translation

## User Story

**As a** translator
**I want** to cancel an in-progress batch translation
**So that** I can stop if I realize something is wrong

## Acceptance Criteria

- [ ] **Given** the batch is running, **When** I click "Cancel", **Then** `invoke("cancel_batch_translation")` is called and the button shows "Cancelling..."
- [ ] **Given** cancel is requested, **When** in-flight jobs complete, **Then** `batch-cancelled` event is received
- [ ] **Given** the batch is cancelled, **When** the event arrives, **Then** the summary modal shows: "Cancelled: 15 completed, 35 remaining"
- [ ] **Given** the batch is cancelled, **When** I view the table, **Then** the 15 completed translations remain visible (not reverted)

## Technical Notes

- Cancel state flow: `running → cancelling → cancelled`
- `batch-cancelled` event payload: `{ completed: number, total: number, errors: [...] }`
- After cancellation, toolbar returns to idle state (button re-enabled, slider re-enabled)
- Completed translations are NOT removed — they were already written to journal

## Dependencies

### Requires
- 001-batch-control-bar (cancel button lives there)
- 002-live-progress-display (state transitions)

## Edge Cases

| Scenario | Expected Behavior |
|----------|-------------------|
| Cancel when 0 jobs completed | Summary shows "Cancelled: 0 completed" |
| Cancel during final in-flight jobs | Wait for in-flight to finish normally |
| Double-click cancel | Second click ignored while "Cancelling..." |
| App closed during cancelling | Cancel signal lost; recovery handles on restart |

## Out of Scope

- Undo completed translations
- Selective cancel (cancel specific job)
