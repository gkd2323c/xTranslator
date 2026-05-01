## ADDED Requirements

### Requirement: ESP file serialization
The system SHALL serialize an in-memory ESP record tree to a valid ESP/ESM file, writing the TES4 header, top-level GRUPs, records, and nested GRUPs in the correct binary format.

#### Scenario: Serialize minimal ESP
- **WHEN** an ESP tree with TES4 header + 3 records in 1 GRUP is serialized
- **THEN** the output file begins with "TES4" magic
- **AND** each GRUP begins with "GRUP" magic
- **AND** record headers (GenericHeader: type + dsize + flags + id + revision + version + unknown) are correctly written

#### Scenario: Serialize compressed records
- **WHEN** a record has the compressed flag
- **THEN** the compressed blob is written directly (already rebuilt by record-rebuild)
- **AND** the record header `dsize` equals the compressed blob length

### Requirement: GRUP size recalculation
The system SHALL recalculate all GRUP header `dsize` values after record rebuilding. GRUP `dsize` SHALL include the GRUP's own 24-byte header (GenericHeader 8B + GrupHeader 16B).

#### Scenario: Recalculate GRUP sizes after record modification
- **WHEN** one record in a GRUP grew by 100 bytes
- **THEN** the GRUP header `dsize` increases by 100
- **AND** parent GRUP sizes cascade upward

#### Scenario: Recalculate nested GRUP sizes
- **WHEN** a child GRUP (e.g., CELL children) is modified
- **THEN** the parent WRLD GRUP `dsize` is recalculated to include the child GRUP's new size

### Requirement: Raw record pass-through
The system SHALL write raw (never-decompressed) records as-is, without modification. Their field buffers are not parsed and not mutated.

#### Scenario: Raw record is written unchanged
- **WHEN** a record has the raw flag (was not decompressed during parsing)
- **THEN** its original buffer is written directly to the output
- **AND** the buffer bytes match the input exactly

### Requirement: Automatic backup before write
The system SHALL create a backup of the original ESP file before writing, unless the user opts out. The backup filename SHALL be `<original>.backup.<timestamp>`.

#### Scenario: Backup created before save
- **WHEN** user saves ESP with backup enabled
- **THEN** a backup file exists at `<original>.backup.<timestamp>`
- **AND** the backup content matches the original ESP byte-for-byte

#### Scenario: Backup skipped when disabled
- **WHEN** user disables backup in settings
- **AND** saves ESP
- **THEN** no backup file is created
