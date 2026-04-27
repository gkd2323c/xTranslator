## 1. Baseline And Tests

- [x] 1.1 Review current dictionary apply tests and mark which existing assertions cover exact, EDID, normalized, vocabulary, and ambiguity behavior.
- [x] 1.2 Add regression tests for pending SST entries that must not overwrite target translation text.
- [x] 1.3 Add regression tests for locked and incomplete SST params taking precedence over translated or validated states.
- [x] 1.4 Add regression tests for same-language versus different-language translated/validated policy.
- [x] 1.5 Add regression tests for tag-only application and optional string ID replacement.
- [x] 1.6 Add regression tests for indexMax warning and bigWarning outcomes.
- [x] 1.7 Add regression tests for preserving unmatched and ambiguous SST entries as oldData on later SST save.

## 2. Core Apply Semantics

- [x] 2.1 Introduce dictionary apply policy/options without changing the existing match tier order.
- [x] 2.2 Extend dictionary entry metadata to carry colab ID, params, string ID, index, indexMax, source format, and EDID hash consistently for SST and XML.
- [x] 2.3 Refactor apply logic so target selection and target mutation are separate helpers.
- [x] 2.4 Implement pending, locked, incomplete, translated, and validated status mapping.
- [x] 2.5 Implement tag-only colab ID application without text or status mutation.
- [x] 2.6 Implement optional string ID replacement and StringIdChanged internal flag handling.
- [x] 2.7 Implement warning and bigWarning internal flag handling for index cardinality mismatches.

## 3. Old Data Preservation

- [x] 3.1 Add session storage for unapplied SST entries that must be retained as old data.
- [x] 3.2 Record unmatched and ambiguous SST entries into the old-data store during SST load.
- [x] 3.3 Update SST save to include current strings plus preserved old-data entries.
- [x] 3.4 Ensure preserved entries are written with the oldData param and are cleared on session reset or new ESP load.

## 4. IPC And UI Surface

- [x] 4.1 Add additive response fields for pending skipped, old-data preserved, warning, and bigWarning counts.
- [x] 4.2 Keep frontend API types in sync with Rust DTO changes.
- [x] 4.3 Display or log the new SST/XML apply counts without changing the main workflow layout.

## 5. Documentation And Verification

- [x] 5.1 Update `docs/feature_comparison.md` so dictionary apply reflects the implemented shared matcher and the remaining Delphi parity work.
- [x] 5.2 Update `docs/delphi_analysis.md` with the analyzed Delphi apply, SST, XML, PEX, and API evidence used by this change.
- [x] 5.3 Run `openspec validate delphi-apply-semantics-parity --strict`.
- [x] 5.4 Run `cargo test -p xt-core --lib`.
- [x] 5.5 Run `cargo build -p xtranslator-tauri`.
- [x] 5.6 Run `cd ui && npx tsc --noEmit`.
