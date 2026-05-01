---
id: 003-error-handling-retry
unit: 001-translation-queue
intent: 001-batch-translation
status: complete
implemented: true
priority: must
created: 2026-05-01T12:00:00Z
assigned_bolt: null
implemented: false
---

# Story: 003-error-handling-retry

## User Story

**As a** translator
**I want** failed translations to be retried automatically and errors logged without blocking the queue
**So that** temporary API issues don't stop the entire batch

## Acceptance Criteria

- [ ] **Given** an API call fails with a transient error (timeout, 429, 5xx), **When** the error occurs, **Then** the job is retried up to 3 times with exponential backoff (1s, 2s, 4s)
- [ ] **Given** a job fails after 3 retries, **When** retries are exhausted, **Then** the job is marked as `failed`, the error is recorded, and the queue moves to the next job
- [ ] **Given** the batch completes, **When** all jobs finish, **Then** a Tauri event `batch-complete` is emitted with `{ total, succeeded, failed, errors: [{ str_id, error }] }`
- [ ] **Given** some jobs failed, **When** the batch completes, **Then** the failed jobs are listed with their error reasons

## Technical Notes

- Use `tokio::time::sleep` for backoff delays
- Error categories: transient (retriable) vs permanent (skip immediately)
- Transient: timeout, 429 rate limit, 5xx server errors
- Permanent: 401 unauthorized, 403 forbidden, invalid request
- Store errors in `BatchResult.errors: Vec<(u32, String)>`

## Dependencies

### Requires
- 002-call-api-translate

## Edge Cases

| Scenario | Expected Behavior |
|----------|-------------------|
| All 3 retries hit rate limit (429) | Mark as failed with "Rate limited after 3 retries" |
| API returns 200 but empty body | Retry (transient, may be network glitch) |
| API Key revoked (401) during batch | Skip remaining retries, mark as permanent failure |
| Network down for all jobs | All jobs retry, eventually all fail, batch completes with error summary |

## Out of Scope

- Changing API provider mid-batch fallback (e.g., fallback to DeepL if OpenAI fails)
- User notification UI (batch-translation-ui unit)
