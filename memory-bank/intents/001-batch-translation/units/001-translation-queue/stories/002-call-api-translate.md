---
id: 002-call-api-translate
unit: 001-translation-queue
intent: 001-batch-translation
status: complete
implemented: true
priority: must
created: 2026-05-01T12:00:00Z
assigned_bolt: null
implemented: false
---

# Story: 002-call-api-translate

## User Story

**As a** translator
**I want** each string in the batch to be translated via the configured API provider
**So that** I get AI-generated translations for all selected strings

## Acceptance Criteria

- [ ] **Given** a job is dequeued, **When** the API is called with the source text and configured provider (OpenAI/DeepL), **Then** the translation result is returned
- [ ] **Given** a translation succeeds, **When** the result is received, **Then** the string's translation field is updated in memory and the translation-cache is notified
- [ ] **Given** a translation succeeds, **When** the result is received, **Then** a Tauri event `translation-complete` is emitted with `{ str_id, translated }`
- [ ] **Given** the existing single-translate API exists, **When** batch translation is running, **Then** single translation still works independently

## Technical Notes

- Reuse existing `xt-core::translation_api::translate_string()` method
- Provider is determined by `AppState.current_provider` at the time of each job
- Emit events via `app_handle.emit("translation-complete", payload)`
- The single-translate Tauri command path must remain unchanged

## Dependencies

### Requires
- 001-create-translation-queue (queue infrastructure)

### Enables
- 003-error-handling-retry

## Edge Cases

| Scenario | Expected Behavior |
|----------|-------------------|
| Provider API Key is invalid | Job fails immediately with "Invalid API Key" error |
| Provider returns empty string | Treat as valid (empty translation may be intentional) |
| Network timeout | Triggers retry logic (003-error-handling-retry) |
| Provider switched mid-batch | Each job uses the provider active when it starts |

## Out of Scope

- Retry logic (003-error-handling-retry)
- Cache file writing (translation-cache unit)
