---
id: 002-detect-pending-cache
unit: 002-translation-cache
intent: 001-batch-translation
status: complete
implemented: true
priority: should
created: 2026-05-01T12:00:00Z
assigned_bolt: null
implemented: false
---

# Story: 002-detect-pending-cache

## User Story

**As a** translator
**I want** the app to detect unapplied translations on startup
**So that** I don't lose work from a previous crash

## Acceptance Criteria

- [ ] **Given** a journal file exists for the current ESP, **When** the app loads the ESP, **Then** `detect_pending(esp_hash)` is called
- [ ] **Given** the journal has records not yet applied to ESP, **When** detection runs, **Then** it returns `RecoveryResult { esp_name, pending_count, cache_file_path }`
- [ ] **Given** the journal exists but all records are already applied, **When** detection runs, **Then** it returns `None` (no recovery needed)
- [ ] **Given** detection finds pending translations, **When** the result is returned, **Then** it is passed to the frontend via `check_pending_cache` IPC command

## Technical Notes

- Compare journal records against ESP string translations:
  - If `str_id` exists in ESP and `translation` matches → already applied, skip
  - If `str_id` exists in ESP and `translation` differs → unapplied (or user edited)
  - If `str_id` not in ESP → skip (ESP may have changed)
- Detection runs during `load_esp` or immediately after, before the UI is ready
- Return count of records that differ from current ESP state

## Dependencies

### Requires
- 001-journal-file-io (journal read capability)

### Enables
- 003-apply-recovery

## Edge Cases

| Scenario | Expected Behavior |
|----------|-------------------|
| Journal file is empty | Returns None immediately |
| Journal for different ESP | No match (checked by esp_hash in filename) |
| ESP re-loaded after journal created | str_id comparison handles this (skip non-matching) |
| Journal has 1000 records | Detection runs efficiently (< 100ms) |

## Out of Scope

- Applying recovery (003-apply-recovery)
- Recovery confirmation UI (003-batch-translation-ui)
