## Purpose

Define how imported dictionary translations are matched and applied to loaded ESP strings, including confidence tiers, ambiguity handling, and response statistics for XML imports and SST loads.

## Requirements

### Requirement: Shared dictionary apply engine
The system SHALL apply imported dictionary entries through a shared matching engine that supports both XML imports and SST dictionary loads.

#### Scenario: XML and SST use common tier logic
- **WHEN** XML import and SST load receive equivalent source, translation, EDID, record, field, and string-id data
- **THEN** both paths SHALL evaluate match tiers in the same order and produce equivalent match statistics

### Requirement: Exact triple matching has highest priority
The system SHALL match by `(str_id, record_sig, field_sig)` before attempting fuzzy or fallback matching.

#### Scenario: Exact match exists
- **WHEN** an imported dictionary entry has the same `str_id`, `record_sig`, and `field_sig` as a loaded string
- **THEN** the system SHALL apply the imported translation to that loaded string
- **THEN** the system SHALL count the match in the exact tier

### Requirement: EDID matching is deterministic
The system SHALL use EDID-based matching only when it identifies a single safe target after record/field filtering and normalized-source disambiguation.

#### Scenario: Unique EDID target
- **WHEN** an imported entry has an EDID whose hash matches exactly one loaded string with the same record and field
- **THEN** the system SHALL apply the imported translation to that loaded string
- **THEN** the system SHALL count the match in the EDID tier

#### Scenario: Ambiguous EDID target
- **WHEN** an imported entry has an EDID whose hash matches multiple loaded strings with the same record and field
- **AND** normalized-source disambiguation does not identify exactly one target
- **THEN** the system SHALL NOT apply the imported translation automatically
- **THEN** the system SHALL count the entry as ambiguous

### Requirement: Normalized-source matching precedes vocabulary matching
The system SHALL attempt normalized-source matching before vocabulary-overlap matching.

#### Scenario: Normalized source match
- **WHEN** an imported entry's normalized source hash matches exactly one loaded string with the same record and field
- **THEN** the system SHALL apply the imported translation to that loaded string
- **THEN** the system SHALL count the match in the normalized tier

### Requirement: Vocabulary matching requires a unique confident candidate
The system SHALL apply vocabulary-overlap matches only when one candidate is uniquely best and meets the configured confidence threshold.

#### Scenario: Unique vocabulary candidate
- **WHEN** an imported entry has no exact, EDID, or normalized match
- **AND** exactly one loaded string with the same record and field has vocabulary overlap at or above the threshold
- **THEN** the system SHALL apply the imported translation to that loaded string
- **THEN** the system SHALL count the match in the vocabulary tier

#### Scenario: Tie between vocabulary candidates
- **WHEN** multiple loaded strings have the same best vocabulary score at or above the threshold
- **THEN** the system SHALL NOT apply the imported translation automatically
- **THEN** the system SHALL count the entry as ambiguous

### Requirement: Applied entries update translation state
The system SHALL update loaded string translation text and status consistently when a dictionary entry is applied.

#### Scenario: SST entry with params
- **WHEN** an SST dictionary entry is applied
- **THEN** the loaded string SHALL receive the entry translation
- **THEN** the loaded string SHALL preserve applicable SST params from the entry

#### Scenario: XML entry without params
- **WHEN** an XML dictionary entry with non-empty translation is applied
- **THEN** the loaded string SHALL be marked translated
- **THEN** the loaded string SHALL NOT be marked incomplete

### Requirement: Import responses expose match quality
The system SHALL report total matched, unmatched, ambiguous, updated IDs, and per-tier counts to callers.

#### Scenario: SST load response
- **WHEN** an SST dictionary is loaded
- **THEN** the response SHALL include `matched`, `unmatched`, `updated_ids`, `tier_exact`, `tier_edid`, `tier_normalized`, `tier_vocab`, and `ambiguous`

#### Scenario: XML import response
- **WHEN** an XML dictionary is imported
- **THEN** the response SHALL include `matched`, `unmatched`, `updated_ids`, `tier_exact`, `tier_edid`, `tier_normalized`, `tier_vocab`, and `ambiguous`
