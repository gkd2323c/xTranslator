---
id: 004-recovery-prompt
unit: 003-batch-translation-ui
intent: 001-batch-translation
status: complete
implemented: true
priority: should
created: 2026-05-01T12:00:00Z
assigned_bolt: null
implemented: false
---

# Story: 004-recovery-prompt

## User Story

**As a** translator
**I want** to be prompted to recover unapplied translations on startup
**So that** I don't lose work from a previous session crash

## Acceptance Criteria

- [ ] **Given** the ESP loads and pending cache is detected, **When** the UI initializes, **Then** a modal appears: "Found 50 unapplied translations. Recover?"
- [ ] **Given** the recovery modal is shown, **When** I click "Recover", **Then** `invoke("apply_translation_cache", { esp_hash })` is called and a toast shows "50 translations recovered"
- [ ] **Given** the recovery modal is shown, **When** I click "Dismiss", **Then** the modal closes and cache is kept for later
- [ ] **Given** I recover translations, **When** recovery completes, **Then** the table refreshes to show the recovered translations
- [ ] **Given** no pending cache exists, **When** the app starts, **Then** no recovery modal is shown

## Technical Notes

- On app startup, after `load_all_strings()` completes, call `invoke("check_pending_cache")`
- If result is not null, show `RecoveryPromptModal`
- After recovery, trigger `load_all_strings()` again to refresh table data
- Use react-hot-toast for success/error messages
- Component: `RecoveryPromptModal` in `ui/src/components/`

## Dependencies

### Requires
- 002-translation-cache unit (journal file + detection + apply)

## Edge Cases

| Scenario | Expected Behavior |
|----------|-------------------|
| Recovery apply fails (disk error) | Toast error, modal stays open for retry |
| User dismisses, loads different ESP | Cache for previous ESP not shown (wrong esp_hash) |
| Recovery succeeds but table refresh fails | Toast shows recovery count, manual refresh available |
| Multiple journal files for different ESPs | Only show recovery for currently loaded ESP |

## Out of Scope

- Manual recovery trigger button (future enhancement)
- Cache file cleanup/management UI
