---
id: 001-translation-cache
unit: 002-translation-cache
intent: 001-batch-translation
type: ddd-construction-bolt
status: complete
stories:
  - 001-journal-file-io
  - 002-detect-pending-cache
  - 003-apply-recovery
created: 2026-05-01T12:00:00Z
started: 2026-05-01T12:00:00Z
completed: 2026-05-01T12:00:00Z
current_stage: null
stages_completed:
  - name: model
    completed: 2026-05-01T12:00:00Z
    artifact: ddd-01-domain-model.md
  - name: design
    completed: 2026-05-01T12:00:00Z
    artifact: ddd-02-technical-design.md
  - name: implement
    completed: 2026-05-01T12:00:00Z
    artifact: translation_cache.rs
  - name: test
    completed: 2026-05-01T12:00:00Z
    artifact: ddd-03-test-report.md

requires_bolts: []
enables_bolts:
  - 002-translation-queue
requires_units: []
blocks: false

complexity:
  avg_complexity: 1
  avg_uncertainty: 1
  max_dependencies: 1
  testing_scope: 2
---

# Bolt: 001-translation-cache

## Overview

Foundation bolt for the translation cache unit. Implements append-only journal file I/O, pending cache detection on startup, and recovery logic.

## Objective

Create the independent translation cache system — a crash-safe append-only journal that persists each translation result immediately and supports recovery after unexpected termination.

## Stories Included

- **001-journal-file-io**: Journal file I/O (Must)
- **002-detect-pending-cache**: Detect pending cache on startup (Should)
- **003-apply-recovery**: Apply recovery to ESP (Should)

## Bolt Type

**Type**: DDD Construction Bolt
**Definition**: `.specsmd/aidlc/templates/construction/bolt-types/ddd-construction-bolt.md`

## Stages

- [ ] **1. model**: Pending → Domain model (TranslationRecord, CacheFile entities)
- [ ] **2. design**: Pending → Technical design (file format, I/O patterns, recovery flow)
- [ ] **3. implement**: Pending → `crates/xt-core/src/translation_cache.rs`
- [ ] **4. test**: Pending → Unit tests + integration tests

## Dependencies

### Requires
- None (foundation bolt)

### Enables
- 002-translation-queue (needs cache for persisting translations)
- 003-batch-translation-ui (needs recovery detection)

## Success Criteria

- [ ] Journal file writes are crash-safe (append + flush)
- [ ] Pending cache detected on startup
- [ ] Recovery applies translations correctly
- [ ] Tests passing

## Notes

- Journal file location: `%LOCALAPPDATA%/xTranslator/translation_cache/{esp_sha256}.journal`
- Format: JSONL (one JSON object per line)
- Separate from existing ESP cache (EsmCache)
