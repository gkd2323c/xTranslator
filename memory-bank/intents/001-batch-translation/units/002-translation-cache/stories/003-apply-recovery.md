---
id: 003-apply-recovery
unit: 002-translation-cache
intent: 001-batch-translation
status: complete
implemented: true
priority: should
created: 2026-05-01T12:00:00Z
assigned_bolt: null
implemented: false
---

# Story: 003-apply-recovery

## User Story

**As a** translator
**I want** to apply recovered translations to my ESP and clear the cache
**So that** my previously translated strings are restored without manual re-entry

## Acceptance Criteria

- [ ] **Given** I confirm recovery, **When** `apply_and_clear(esp_hash)` is called, **Then** each cached translation is written to the ESP string's `translation` field
- [ ] **Given** recovery is applied, **When** all records are processed, **Then** the journal file is deleted
- [ ] **Given** I decline recovery, **When** `discard_cache(esp_hash)` is called, **Then** the journal file is kept (for manual recovery later)
- [ ] **Given** recovery is applied, **When** some str_ids no longer exist in ESP, **Then** those records are skipped (not an error)

## Technical Notes

- IPC command: `#[tauri::command] fn apply_translation_cache(esp_hash: String) -> Result<u32>`
- Returns count of successfully applied records
- After applying, trigger a frontend data refresh (reload strings chunk)
- On success: delete journal file via `std::fs::remove_file`
- On decline: optional `delete_cache` IPC command to delete later

## Dependencies

### Requires
- 001-journal-file-io (read capability)
- 002-detect-pending-cache (detection result)

## Edge Cases

| Scenario | Expected Behavior |
|----------|-------------------|
| Journal file deleted externally between detect and apply | Apply returns 0, no error |
| ESP changed between detect and apply | str_id comparison ensures only matching records applied |
| Partial apply (crash during apply) | Next startup: re-detect remaining unapplied records |
| User declines but later wants to recover | Journal file stays; can add "Recover Now" button later |

## Out of Scope

- Recovery UI prompt (003-batch-translation-ui)
- Undo recovery
