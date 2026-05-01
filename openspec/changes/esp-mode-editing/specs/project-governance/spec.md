## ADDED Requirements

### Requirement: ESP write-back invariants
The repository SHALL maintain invariants covering ESP write-back behavior in `SPEC.md` (§V29-§V32).

#### Scenario: SPEC.md has ESP write-back invariants
- **WHEN** ESP mode editing is implemented
- **THEN** SPEC.md SHALL contain at minimum:
  - V29: ∀ save_esp → backup original ESP before write (unless user opted out)
  - V30: ∀ save_esp → compressed record output format is `[4-byte decompressedSize LE] + [zlib data]`
  - V31: ∀ record rebuild → non-string fields pass through unchanged; only fields with matching SkyString entries are modified
  - V32: ∀ delocalize_esp → new string IDs are sequential starting from 1, ordered by source text

## MODIFIED Requirements

### Requirement: Public documentation matches implemented behavior
Public-facing documentation SHALL distinguish implemented features from roadmap or aspirational features.

#### Scenario: Translation provider documentation
- **WHEN** documentation lists supported translation providers
- **THEN** the list SHALL match providers exposed by the implemented provider registry or explicitly mark missing providers as roadmap items

#### Scenario: ESP mode feature documentation
- **WHEN** documentation describes ESP editing capabilities
- **THEN** it SHALL list both Strings mode (external files) and ESP mode (direct record modification) with their respective capabilities and limitations
