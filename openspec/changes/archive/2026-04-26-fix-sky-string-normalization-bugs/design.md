## Context

Current state from SPEC.md:
- B1: `compressed_records` stat always 0 in `LoadEspResponse`
- B2: `edid_hash` always 0 in `EspPointer` (EDID hashing not implemented)
- B3: `source_normalized` & `normalized_hash` always None
- B4: `SkyString.word_hashes` always empty (tokenization not implemented)

Partial fix code exists in the working tree but needs completion and verification.

## Goals / Non-Goals

**Goals:**
- Fix all 4 listed bugs with 100% test coverage
- Ensure heuristic search can now properly use normalized strings and word hashes
- Maintain Delphi SST v8 binary compatibility (no changes to persisted format)
- Minimal performance impact (<5% overhead on ESP parsing)

**Non-Goals:**
- Changing the SST v8 binary format
- Implementing new heuristic search algorithms (out of scope)
- Refactoring the EspParser architecture (focus on bug fixes only)

## Decisions

**Decision 1: Normalization algorithm scope**
- Unicode `to_lowercase()` for full Unicode support (handles German ß → "ss", Greek, Cyrillic, etc.)
- Non-alphanumeric characters replaced with single space
- Consecutive spaces compressed
- Leading/trailing spaces trimmed
- Rationale: Matches Delphi xTranslator behavior for cross-tool compatibility

**Decision 2: Tokenization strategy**
- Split on any non-alphanumeric character (same as normalization boundary)
- Filter out empty tokens
- Apply same FNV-1a hash function to each token
- Rationale: Simple implementation that matches normalization, no double-processing needed

**Decision 3: Breaking change to SkyString constructor**
- Add `record_sig` and `field_sig` as required parameters to `SkyString::new()`
- These values are readily available at all call sites (ESP parser, SST reader, XML importer)
- Rationale: Duplicating these fields in SkyString simplifies DTO mapping and frontend display

**Decision 4: EspPointer::null() convenience constructor**
- Centralize the "null pointer" definition
- Rationale: Avoids magic number repetition and ensures consistent initialization

## Risks / Trade-offs

| Risk | Mitigation |
|------|------------|
| Breaking change to `SkyString::new()` affects 5+ call sites | Audit all call sites (SST v8, XML, test generator, commands) and update together |
| Normalization performance impact on large datasets | Benchmark; ASCII fast-path already implemented (70% of game strings are ASCII) |
| Existing SST files have incorrect hashes | Hashes are runtime-only; persisted SST files use only `EspPointer` for matching |
