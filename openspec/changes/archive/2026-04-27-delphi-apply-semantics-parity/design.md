## Context

The current Rust rewrite has a shared dictionary matcher for XML imports and SST loads. It applies entries through exact, EDID, normalized-source, and vocabulary tiers, and it rejects ambiguous candidates. This matches the direction suggested by the Delphi implementation, but the Delphi workflow also carries apply-time semantics that are not fully represented yet: pending entries do not overwrite translations, locked and incomplete states affect the target status, old SST data is preserved for later saves, EDID/index mismatches create warnings, tag-only application can update collaboration metadata without text replacement, and some flows can replace the localized string ID.

The change should build on the existing matcher instead of replacing it. The matcher answers "which loaded string is safe to target"; a new apply semantics layer should answer "what should happen to that target and to any unmatched dictionary entry."

## Goals / Non-Goals

**Goals:**

- Keep the existing deterministic tier order and ambiguity handling.
- Add an explicit apply policy that covers SST and XML differences without duplicating matcher code.
- Preserve Delphi-derived status semantics for pending, locked, incomplete, translated, validated, oldData, warning, bigWarning, colab ID, and string ID changes where the Rust data model already supports them.
- Track unapplied SST entries as old data so a later SST save can retain them.
- Add regression tests before changing implementation behavior.
- Correct stale documentation that still describes dictionary apply as a simple triple match.

**Non-Goals:**

- Do not add BA2, PEX write-back, VMAD parsing, MCM custom text import, or additional translation providers in this change.
- Do not implement SST v1-v7 loading unless a test fixture or user requirement makes it necessary.
- Do not relax ambiguity handling. Ambiguous matches remain unapplied.
- Do not add new third-party dependencies.

## Decisions

1. Keep matching and apply semantics separate.

   The existing `apply_dictionary_entries` path should be refactored so match selection can remain stable while the application step receives a policy and produces richer outcomes. This avoids rewriting the exact/EDID/normalized/vocab logic that is already covered by tests.

   Alternative considered: merge all Delphi behavior into each tier finder. Rejected because status transitions are independent of how a target was found, and tier-local status logic would duplicate behavior.

2. Introduce an explicit apply policy.

   Add a small policy/options structure for caller-controlled behavior such as same-language mode, tag-only mode, string ID replacement, reset-state behavior, and old-data preservation. XML import and SST load can share defaults but override source-specific behavior.

   Alternative considered: infer all behavior from `DictionarySourceFormat`. Rejected because Delphi exposes separate workflow switches such as tag-only and string ID application, and hidden inference would make tests brittle.

3. Extend dictionary entries with metadata needed by the apply layer.

   SST entries should carry `colab_id`, params, list index, string ID, record/field/index/indexMax, EDID hash, source, and translation into the matcher. XML entries should carry their parsed `Partial`, `sID`, `REC id/idMax`, EDID, source, and destination data. Missing metadata should be explicit rather than guessed.

   Alternative considered: read from `SkyString` directly in the apply layer. Rejected because XML imports are not `SkyString` rows, and the shared matcher needs one source-neutral entry shape.

4. Track old SST entries in session state.

   When an SST entry is not safely applied, the loader should record it as old data rather than dropping it. `save_sst` should include current strings plus preserved old-data entries, matching Delphi's `keepOldData` intent.

   Alternative considered: only report old-data counts. Rejected because it would still lose data on the next save.

5. Use internal params for warnings and string ID changes.

   Index/indexMax mismatch handling should use existing warning/internal param bits. String ID replacement should update `EspPointer.str_id` and mark the row with the existing `StringIdChanged` internal bit if available.

   Alternative considered: expose warnings only in response stats. Rejected because warnings need to persist in the in-memory row for UI filtering and later save/export behavior.

## Risks / Trade-offs

- Wider response DTOs may require frontend updates -> keep new fields additive and optional where possible.
- Preserving old SST entries increases in-memory session state -> store only the original entry data needed for a future SST save.
- Delphi same-language behavior has several branches -> encode it as small, named helper functions and test the observed cases from `findStrMatchEx` and `findEdidMatchEx`.
- Existing tests may assume a simple param copy -> update tests to assert the new explicit state policy rather than incidental bit copying.
