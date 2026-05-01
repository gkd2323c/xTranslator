---
id: 002-translation-queue
unit: 001-translation-queue
intent: 001-batch-translation
type: ddd-construction-bolt
status: complete
stories:
  - 001-create-translation-queue
  - 002-call-api-translate
  - 003-error-handling-retry
  - 004-cancel-and-progress
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
    artifact: batch_queue.rs
  - name: test
    completed: 2026-05-01T12:00:00Z
    artifact: ddd-03-test-report.md

requires_bolts:
  - 001-translation-cache
enables_bolts:
  - 003-batch-translation-ui
requires_units:
  - 002-translation-cache
blocks: false

complexity:
  avg_complexity: 2
  avg_uncertainty: 1
  max_dependencies: 2
  testing_scope: 2
---

# Bolt: 002-translation-queue

## Overview

Core batch translation engine. Manages the translation queue with configurable concurrency, API calls via existing providers, retry logic, and cancel/progress signaling.

## Objective

Implement the batch translation engine — a concurrent queue that sends strings to OpenAI/DeepL, handles retries, emits progress events, and supports cancellation.

## Stories Included

- **001-create-translation-queue**: Queue setup with concurrency control (Must)
- **002-call-api-translate**: API integration via existing providers (Must)
- **003-error-handling-retry**: Auto-retry with exponential backoff (Must)
- **004-cancel-and-progress**: Cancel signal and progress events (Should)

## Bolt Type

**Type**: DDD Construction Bolt
**Definition**: `.specsmd/aidlc/templates/construction/bolt-types/ddd-construction-bolt.md`

## Stages

- [ ] **1. model**: Pending → Domain model (TranslationJob, TranslationQueue, BatchResult)
- [ ] **2. design**: Pending → Technical design (concurrency model, event flow, Tauri IPC)
- [ ] **3. implement**: Pending → `crates/xt-core/src/translation_queue.rs` + Tauri commands
- [ ] **4. test**: Pending → Unit tests + integration tests

## Dependencies

### Requires
- 001-translation-cache (must be complete — queue writes to cache after each job)

### Enables
- 003-batch-translation-ui (UI consumes queue events and IPC commands)

## Success Criteria

- [ ] Concurrency control (Semaphore) respects user setting
- [ ] Retry logic: 3 attempts, exponential backoff, skip on permanent failure
- [ ] Cancel stops new jobs, in-flight jobs complete
- [ ] Events emitted correctly (progress, complete, cancelled)
- [ ] Existing single-translate still works

## Notes

- Reuses `xt-core::translation_api::translate_string()` (no new API client)
- Concurrency via `tokio::sync::Semaphore`
- Cancel via `tokio::sync::watch`
- Events via `app_handle.emit()`
