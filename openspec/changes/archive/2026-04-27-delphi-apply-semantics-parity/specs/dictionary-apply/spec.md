## ADDED Requirements

### Requirement: Apply policy controls dictionary side effects
The system SHALL apply matched dictionary entries through an explicit policy that distinguishes match selection from target mutation.

#### Scenario: Existing match tiers remain authoritative
- **WHEN** a dictionary entry is evaluated for application
- **THEN** the system SHALL select the target using the existing exact, EDID, normalized-source, and vocabulary tier order
- **THEN** the system SHALL apply status, warning, tag, string-id, and old-data behavior only after a unique target is selected

#### Scenario: Ambiguous match has no side effects
- **WHEN** a dictionary entry is ambiguous under the existing matching rules
- **THEN** the system SHALL NOT update translation text
- **THEN** the system SHALL NOT update target status, colab ID, string ID, or warning flags

### Requirement: SST params drive target translation status
The system SHALL map SST entry params to the target string using Delphi-compatible status precedence.

#### Scenario: Pending entry does not overwrite translation
- **WHEN** an SST entry matched to a target has the pending param
- **THEN** the system SHALL NOT replace the target translation text
- **THEN** the system SHALL count the entry as skipped for pending state rather than as an applied translation

#### Scenario: Locked entry applies locked status
- **WHEN** an SST entry matched to a target has the lockedTrans param and is not pending
- **THEN** the system SHALL replace the target translation text with the entry translation
- **THEN** the target SHALL be marked locked
- **THEN** the target SHALL NOT be marked translated, incomplete, or validated

#### Scenario: Incomplete entry applies incomplete status
- **WHEN** an SST entry matched to a target has the incompleteTrans param and is not pending or locked
- **THEN** the system SHALL replace the target translation text with the entry translation
- **THEN** the target SHALL be marked incomplete
- **THEN** the target SHALL NOT be marked translated, locked, or validated

#### Scenario: Normal translated SST entry applies translated status
- **WHEN** an SST entry matched to a target has a non-empty translation and is not pending, locked, or incomplete
- **THEN** the system SHALL replace the target translation text with the entry translation
- **THEN** the target SHALL be marked translated or validated according to the active language policy

### Requirement: Language policy determines translated versus validated status
The system SHALL choose translated or validated status through an explicit same-language policy instead of implicit source-format assumptions.

#### Scenario: Different-language apply marks translated
- **WHEN** a non-pending dictionary entry is applied under different-language policy
- **THEN** the target SHALL be marked translated unless the entry params require locked or incomplete status

#### Scenario: Same-language changed translation marks validated
- **WHEN** a non-pending dictionary entry is applied under same-language policy
- **AND** the applied translation differs from the target's previous translation
- **THEN** the target SHALL be marked validated unless the entry params require locked or incomplete status

### Requirement: Tag-only application preserves translation text
The system SHALL support tag-only dictionary application for workflows that only update collaboration metadata.

#### Scenario: Tag-only SST match updates colab ID
- **WHEN** an SST entry with a colab ID matches a target under tag-only policy
- **THEN** the target colab ID SHALL be updated from the entry
- **THEN** the target translation text SHALL remain unchanged
- **THEN** the target translation status SHALL remain unchanged

### Requirement: String ID replacement is explicit
The system SHALL replace a target string ID only when the active apply policy enables string ID replacement.

#### Scenario: String ID replacement enabled
- **WHEN** a dictionary entry with a string ID matches a target
- **AND** the active policy enables string ID replacement
- **THEN** the target `EspPointer.str_id` SHALL be replaced with the entry string ID
- **THEN** the target SHALL be marked with the internal StringIdChanged flag

#### Scenario: String ID replacement disabled
- **WHEN** a dictionary entry with a string ID matches a target
- **AND** the active policy disables string ID replacement
- **THEN** the target `EspPointer.str_id` SHALL remain unchanged

### Requirement: Index cardinality mismatches produce warnings
The system SHALL mark EDID or record-field fallback matches as incomplete when index cardinality indicates a potentially unsafe positional match.

#### Scenario: Matching nonzero indexMax values differ
- **WHEN** a dictionary entry is applied through an EDID or record-field fallback tier
- **AND** either the entry or target has a nonzero indexMax
- **AND** the entry indexMax differs from the target indexMax
- **THEN** the target SHALL be marked incomplete
- **THEN** the target SHALL include the bigWarning internal flag

#### Scenario: Matching nonzero indexMax values are equal
- **WHEN** a dictionary entry is applied through an EDID or record-field fallback tier
- **AND** either the entry or target has a nonzero indexMax
- **AND** the entry indexMax equals the target indexMax
- **THEN** the target SHALL be marked incomplete
- **THEN** the target SHALL include the warning internal flag

### Requirement: Unapplied SST entries are preserved as old data
The system SHALL retain safely loaded SST entries that are not applied so a later SST save can preserve historical dictionary data.

#### Scenario: Unmatched SST entry becomes old data
- **WHEN** an SST entry cannot be matched to any loaded string
- **THEN** the system SHALL preserve the entry as old data in the current session
- **THEN** the system SHALL include the preserved entry in subsequent SST saves unless the session is reset

#### Scenario: Ambiguous SST entry becomes old data
- **WHEN** an SST entry is ambiguous and therefore not automatically applied
- **THEN** the system SHALL preserve the entry as old data in the current session
- **THEN** the system SHALL include the preserved entry in subsequent SST saves unless the session is reset

#### Scenario: Old data is flagged on save
- **WHEN** preserved old-data entries are written to an SST file
- **THEN** each preserved entry SHALL be written with the oldData param

### Requirement: Apply responses expose semantic skips and warnings
The system SHALL report semantic apply outcomes in addition to tier match counts.

#### Scenario: Pending and old-data counts are reported
- **WHEN** an SST dictionary is loaded
- **THEN** the response SHALL include counts for pending-skipped entries and preserved old-data entries

#### Scenario: Warning counts are reported
- **WHEN** dictionary application marks targets with warning or bigWarning due to index cardinality
- **THEN** the response SHALL include counts for warning and bigWarning outcomes
