---
id: 001-create-translation-queue
unit: 001-translation-queue
intent: 001-batch-translation
status: complete
implemented: true
priority: must
created: 2026-05-01T12:00:00Z
assigned_bolt: null
implemented: false
---

# Story: 001-create-translation-queue

## User Story

**As a** translator
**I want** to select multiple strings and start a batch translation
**So that** I can translate many strings at once without clicking each one

## Acceptance Criteria

- [ ] **Given** I have selected 10 strings and configured concurrency to 3, **When** I click "Batch Translate", **Then** a translation queue is created and processing begins
- [ ] **Given** the queue is running with concurrency 3, **When** I inspect, **Then** at most 3 API requests are in-flight simultaneously
- [ ] **Given** the queue is running, **When** a slot frees up, **Then** the next pending string is picked up immediately
- [ ] **Given** no API Key is configured, **When** I click "Batch Translate", **Then** I see an error "Please configure an API Key first"

## Technical Notes

- Use `tokio::sync::Semaphore` with `concurrency` permits for concurrency control
- Each job is a `tokio::spawn` task that acquires a semaphore permit before calling the API
- The queue should be a `Vec<TranslationJob>` with status tracking
- Expose via Tauri command: `#[tauri::command] async fn start_batch_translation(...)`

## Dependencies

### Requires
- None (first story in unit)

### Enables
- 002-call-api-translate
- 003-error-handling-retry
- 004-cancel-and-progress

## Edge Cases

| Scenario | Expected Behavior |
|----------|-------------------|
| Concurrency set to 1 | Sequential processing, one at a time |
| Concurrency set to > number of strings | All strings processed concurrently (capped at selection count) |
| User changes selection after starting | Queue uses snapshot of IDs at start time |
| Empty selection | Button disabled, cannot start |

## Out of Scope

- API call logic (002-call-api-translate)
- Progress/UI events (004-cancel-and-progress)
