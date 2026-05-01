## ADDED Requirements

### Requirement: Localized string ID replacement
The system SHALL replace 4-byte string ID fields in localized ESP records with inline text, converting each localized reference to a delocalized inline string.

#### Scenario: Replace string ID with inline text
- **WHEN** a localized ESP field contains a 4-byte string ID (e.g., 0x00001234)
- **AND** the matching `.STRINGS` file has the text for that ID
- **THEN** the field buffer is replaced with the inline text
- **AND** the record's localized flag is cleared

### Requirement: Sequential string ID reassignment
The system SHALL assign new sequential string IDs (starting from 1) to all strings in the delocalized ESP, and write 4-byte ID fields into the record buffers.

#### Scenario: Assign sequential IDs
- **WHEN** a localized ESP has 5000 strings with scattered IDs
- **THEN** each string is assigned a new ID from 1 to 5000 in source text order
- **AND** the field buffer contains the 4-byte LE representation of the new ID

### Requirement: Export .STRINGS on delocalize
The system SHALL export `.STRINGS`, `.DLSTRINGS`, and `.ILSTRINGS` files after delocalization, using the newly assigned sequential IDs and the previously loaded source strings.

#### Scenario: Export strings files after delocalization
- **WHEN** delocalization completes successfully
- **THEN** `<esp_name>_<language>.STRINGS` is written with null-terminated format
- **AND** `<esp_name>_<language>.DLSTRINGS` is written with length-prefixed format
- **AND** `<esp_name>_<language>.ILSTRINGS` is written with length-prefixed format

### Requirement: SST dictionary merge during delocalize
The system SHALL apply SST dictionary translations before delocalization, using a 2-pass strategy: strict match first, then relaxed match for unmatched strings.

#### Scenario: Apply SST during delocalization
- **WHEN** an SST dictionary is loaded before delocalization
- **THEN** pass 1: exact (str_id, record_sig, field_sig) matches are applied
- **AND** pass 2: relaxed matches (normalized source text match) are applied for remaining unmatched strings
- **AND** matched translations appear in the exported `.STRINGS` files
