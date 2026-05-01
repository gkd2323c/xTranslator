---
id: 004-cancel-and-progress
unit: 001-translation-queue
intent: 001-batch-translation
status: complete
implemented: true
priority: should
created: 2026-05-01T12:00:00Z
assigned_bolt: null
implemented: false
---

# Story: 004-cancel-and-progress

## User Story

**As a** translator
**I want** to see batch progress and cancel at any time
**So that** I know how much is done and can stop if needed

## Acceptance Criteria

- [ ] **Given** the queue is running, **When** jobs complete, **Then** a Tauri event `batch-progress` is emitted with `{ completed, total }` after each job
- [ ] **Given** the queue is running, **When** I call `cancel_batch`, **Then** no new jobs are started and in-flight jobs complete normally
- [ ] **Given** I cancel the batch, **When** all in-flight jobs finish, **Then** a `batch-cancelled` event is emitted with `{ completed, total, errors }`
- [ ] **Given** I cancel the batch, **When** in-flight jobs finish, **Then** their results are still saved to cache (not discarded)

## Technical Notes

- Use `tokio::sync::watch` channel for cancel signal: `watch::Sender<bool>`
- Each job task checks `cancel_receiver.has_changed()` before starting
- In-flight jobs are NOT cancelled (they complete their API call)
- Progress events emit after each job completes (not per-request retry)

## Dependencies

### Requires
- 001-create-translation-queue (queue infrastructure)

## Edge Cases

| Scenario | Expected Behavior |
|----------|-------------------|
| Cancel while all jobs are in-flight | Wait for in-flight jobs to complete, then emit batch-cancelled |
| Cancel with no jobs started yet | Immediately emit batch-cancelled with completed=0 |
| Cancel after all jobs done | No-op (already done) |
| App closed during batch | Jobs lost (cache writer handles persistence) |

## Out of Scope

- UI progress display (003-batch-translation-ui)
- Persisting progress for crash recovery (002-translation-cache)
