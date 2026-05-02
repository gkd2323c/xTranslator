## ADDED Requirements

### Requirement: save_esp IPC command
The system SHALL expose a `save_esp` IPC command that writes all in-memory translations back into the ESP file.

#### Scenario: Save ESP with modified translations
- **WHEN** user has loaded a delocalized ESP and modified 10 translations
- **AND** calls `save_esp`
- **THEN** all 10 modified translations are encoded into their field buffers
- **AND** the ESP file on disk contains the updated inline strings
- **AND** the response includes `{ bytes_written, records_modified }`

#### Scenario: Save ESP rejected for localized ESP
- **WHEN** the loaded ESP is localized (has external strings files)
- **AND** user calls `save_esp`
- **THEN** the command returns an error: "Use save_strings for localized ESP"

### Requirement: finalize_esp IPC command
The system SHALL expose a `finalize_esp` command that performs the full ESP save pipeline: apply SST → update translations → rebuild records → serialize ESP → export Strings files.

#### Scenario: Finalize ESP with SST dictionary
- **WHEN** an SST dictionary is loaded
- **AND** user calls `finalize_esp`
- **THEN** SST translations are applied first
- **AND** the ESP is serialized with all translations
- **AND** `.STRINGS` files are exported alongside the ESP

### Requirement: delocalize_esp IPC command
The system SHALL expose a `delocalize_esp` command that converts a localized ESP to delocalized format.

#### Scenario: Delocalize ESP
- **WHEN** user calls `delocalize_esp` on a localized ESP
- **THEN** string IDs in record fields are replaced with inline text
- **AND** new sequential IDs are assigned
- **AND** `.STRINGS`/`.DLSTRINGS`/`.ILSTRINGS` files are exported
- **AND** the ESP is saved in delocalized state
- **AND** the response includes `{ new_string_count, strings_files_paths }`

### Requirement: ESP mode toggle in app config
The system SHALL persist the ESP mode preference in `AppConfig` and expose a toggle via the UI.

#### Scenario: Switch to ESP mode
- **WHEN** user toggles to ESP mode in settings
- **THEN** `save_config` persists `esp_mode: true`
- **AND** subsequent `save` operations route to `save_esp` instead of `save_strings`

#### Scenario: ESP mode survives restart
- **WHEN** user sets ESP mode and restarts the app
- **THEN** `load_config` returns `esp_mode: true`
- **AND** the UI shows ESP mode as active
