## ADDED Requirements

### Requirement: String normalization function
The system SHALL provide a `normalize` function that converts source strings into a canonical form for fuzzy matching.

#### Scenario: Basic ASCII normalization
- **WHEN** normalizing "  Hello,  World!  "
- **THEN** the result SHALL be "hello world"

#### Scenario: Unicode lowercase conversion
- **WHEN** normalizing "Привет МИР" (Cyrillic)
- **THEN** the result SHALL be "привет мир"

#### Scenario: Punctuation handling
- **WHEN** normalizing "Hello-World.Test_case"
- **THEN** the result SHALL be "hello world test case"

#### Scenario: Empty and whitespace-only input
- **WHEN** normalizing "" or "   "
- **THEN** the result SHALL be ""

### Requirement: SkyString auto-normalization on creation
When a `SkyString` is created, the system SHALL automatically compute and populate `source_normalized` and `normalized_hash` fields.

#### Scenario: Normalized fields populated at creation
- **WHEN** creating a SkyString with source "Hello, World!"
- **THEN** `source_normalized` SHALL be Some("hello world")
- **THEN** `normalized_hash` SHALL be Some(FNV1a_low_byte("hello world"))

#### Scenario: Empty source yields None
- **WHEN** creating a SkyString with empty source
- **THEN** `source_normalized` SHALL be None
- **THEN** `normalized_hash` SHALL be None

### Requirement: SkyString word hashes on creation
When a `SkyString` is created, the system SHALL automatically tokenize the source string and populate `word_hashes` with FNV-1a hashes of each token.

#### Scenario: Word hashes populated at creation
- **WHEN** creating a SkyString with source "Hello world test"
- **THEN** `word_hashes` SHALL contain 3 hashes corresponding to "hello", "world", "test"

#### Scenario: Punctuation splits tokens
- **WHEN** creating a SkyString with source "Hello.World-Test_case"
- **THEN** the system SHALL generate 3 word hashes

### Requirement: EDID hash computation in ESP parsing
During ESP parsing, the system SHALL compute the FNV-1a hash of each record's EDID and populate `EspPointer.edid_hash`.

#### Scenario: EDID hashed during record parsing
- **WHEN** parsing a record with EDID = "NPC_Enemy01"
- **THEN** `EspPointer.edid_hash` SHALL equal `string_hash("NPC_Enemy01")`

#### Scenario: Missing EDID yields zero
- **WHEN** parsing a record with no EDID field
- **THEN** `EspPointer.edid_hash` SHALL be 0

### Requirement: Compressed records counter
During ESP parsing, the system SHALL maintain a counter of compressed records encountered and expose it in `EspParser.compressed_records`.

#### Scenario: Compressed record increments counter
- **WHEN** parsing 5 compressed records
- **THEN** `compressed_records` SHALL equal 5
