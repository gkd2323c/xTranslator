---
id: 001-journal-file-io
unit: 002-translation-cache
intent: 001-batch-translation
status: complete
implemented: true
priority: must
created: 2026-05-01T12:00:00Z
assigned_bolt: null
implemented: false
---

# Story: 001-journal-file-io

## User Story

**As a** system
**I want** to persist each translation result immediately to an append-only journal file
**So that** translations are never lost even if the application crashes

## Acceptance Criteria

- [ ] **Given** a translation completes, **When** `append_translation(str_id, source, translated)` is called, **Then** a new line is appended to the journal file
- [ ] **Given** the journal file exists, **When** the app starts, **Then** all records can be read back in order
- [ ] **Given** the journal file is written, **When** I inspect it, **Then** each record contains `{ str_id, source_text, translated_text, timestamp }`
- [ ] **Given** the journal file is at `%LOCALAPPDATA%/xTranslator/translation_cache/{esp_sha256}.journal`, **When** the file is written, **Then** it uses append mode (`OpenOptions::append(true)`) for crash safety

## Technical Notes

- Format: one JSON record per line (JSONL) for simplicity and crash-resistance
- File path: `dirs::cache_dir() / "xTranslator" / "translation_cache" / "{esp_hash}.journal"`
- On `append`: open file, seek to end, write line, flush
- On `read_all`: open file, read lines, deserialize each line, skip corrupt lines
- Use `serde_json` for serialization

## Dependencies

### Requires
- None (first story in unit)

### Enables
- 002-detect-pending-cache
- 003-apply-recovery

## Edge Cases

| Scenario | Expected Behavior |
|----------|-------------------|
| Journal file doesn't exist on first write | Create new file, start writing |
| Partial write on crash (half line) | Skip corrupt line on read, remaining lines intact |
| Disk full | Error propagated to caller, translation job marked as failed |
| Concurrent writes (multiple jobs completing) | Mutex or channel serialization to ensure atomic writes |

## Out of Scope

- Recovery detection (002-detect-pending-cache)
- Applying cached translations to ESP (003-apply-recovery)
