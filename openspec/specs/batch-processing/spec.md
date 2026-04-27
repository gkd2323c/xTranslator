## Purpose

Define how batch translation and export jobs operate across multiple ESP files, including progress reporting, isolation from the interactive editor, and the backend reference used by batch IPC commands.

## Requirements

### Requirement: Batch translation across multiple ESP files
System SHALL accept list of ESP file entries and run AI translation sequentially across all, saving translated strings files per file.

#### Scenario: Translate three files with DeepL
- **GIVEN** 3 ESP files with untranslated strings
- **WHEN** user starts batch translate with DeepL provider and target "chinese"
- **THEN** each file SHALL be processed sequentially: parse → translate → save
- **THEN** progress events SHALL be emitted to frontend after each string
- **THEN** `batch-complete` event SHALL fire with total/translated/errors summary

#### Scenario: Skip already translated strings
- **GIVEN** a file with 500 strings, 300 already translated
- **WHEN** `skip_translated = true`
- **THEN** only 200 untranslated strings SHALL be sent to translation API

#### Scenario: Single file parse failure does not abort batch
- **GIVEN** a batch with 3 files, 2nd file is corrupted
- **WHEN** batch runs
- **THEN** 1st file SHALL complete successfully
- **THEN** 2nd file SHALL log error and be skipped
- **THEN** 3rd file SHALL process normally
- **THEN** `batch-complete` SHALL report 2 success, 1 failed

#### Scenario: Cancel during translation
- **GIVEN** a running batch with 5 files, currently on file 2
- **WHEN** user cancels
- **THEN** current API call SHALL finish, then processing stops
- **THEN** already-saved files (file 1) remain saved
- **THEN** `batch-complete` SHALL fire with cancellation status

#### Scenario: Invalid API key aborts immediately
- **GIVEN** batch with invalid API key
- **WHEN** batch attempts first translation
- **THEN** entire batch SHALL abort
- **THEN** error SHALL report "API key invalid"

### Requirement: Batch export to XML/SST
System SHALL accept list of ESP files and export translations to XML or SST format per file.

#### Scenario: Export 5 files to XML
- **GIVEN** 5 ESP files with translations loaded
- **WHEN** user starts batch XML export
- **THEN** each file SHALL be processed: load strings → export → next
- **THEN** 5 XML files SHALL be created in output directory

### Requirement: Frontend batch panel
UI SHALL provide a panel for configuring, monitoring, and controlling batch jobs.

#### Scenario: Add files via multi-select dialog
- **WHEN** user clicks [Add Files]
- **THEN** native file dialog SHALL open with .esp/.esm filter
- **THEN** selected files SHALL appear in batch file list
- **THEN** each file SHALL auto-detect: strings dir, language, game

#### Scenario: Scan directory for ESP files
- **WHEN** user clicks [Scan Directory] and selects a directory
- **THEN** all .esp/.esm files SHALL be discovered recursively
- **THEN** each file SHALL appear with auto-detected config

#### Scenario: Progress display during run
- **GIVEN** a running batch
- **THEN** each file SHALL show status icon: ○ waiting, ◐ running, ✓ done, ✗ failed
- **THEN** current file SHALL show progress bar and string count
- **THEN** overall progress SHALL show "3/10 files complete"

### Requirement: Batch state isolation
Batch processing SHALL operate independently of the interactive editor session, using its own file reads and writes.

#### Scenario: Batch does not affect interactive session
- **GIVEN** file A is loaded interactively in EditorPanel
- **WHEN** batch processes file B (not A)
- **THEN** interactive session remains unchanged
- **THEN** batch completes without affecting file A's AppState

### Requirement: AppState gains BatchExecutor reference
AppState SHALL hold an `Arc<BatchExecutor>` to manage batch job lifecycle, accessible from batch IPC commands.

#### Scenario: Batch executor available to commands
- **WHEN** batch translate or export IPC commands are invoked
- **THEN** the commands SHALL access the shared `BatchExecutor` from AppState
