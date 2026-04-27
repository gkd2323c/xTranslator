## 1. Matcher Model

- [x] 1.1 Add a neutral `DictionaryApplyEntry` type in `xt-core` with fields needed by XML and SST imports.
- [x] 1.2 Add a result model that tracks exact, EDID, normalized, vocabulary, ambiguous, unmatched, and updated IDs.
- [x] 1.3 Convert XML entries into `DictionaryApplyEntry` without changing XML parsing behavior.
- [x] 1.4 Convert SST entries into `DictionaryApplyEntry` while preserving optional SST params.

## 2. Matching Semantics

- [x] 2.1 Refactor `enhanced_import_match` into a shared dictionary apply function.
- [x] 2.2 Keep exact triple matching as Tier 1.
- [x] 2.3 Implement EDID matching with normalized-source disambiguation and ambiguity counting.
- [x] 2.4 Move normalized-source matching before vocabulary-overlap matching.
- [x] 2.5 Require vocabulary matches to be uniquely best and above threshold before applying.
- [x] 2.6 Ensure no loaded string can be matched more than once per import.

## 3. Integration

- [x] 3.1 Update XML import to call the shared dictionary apply function.
- [x] 3.2 Update SST loading to call the shared dictionary apply function instead of exact-only matching.
- [x] 3.3 Preserve SST params on applied SST entries.
- [x] 3.4 Keep XML status behavior equivalent for non-empty imported translations.

## 4. IPC and Frontend Types

- [x] 4.1 Extend `LoadSstResponse` with defaulted per-tier, ambiguous, and updated-id fields.
- [x] 4.2 Extend `XmlImportResponse` with a defaulted ambiguous field.
- [x] 4.3 Mirror response fields in `ui/src/api/strings.ts`.
- [x] 4.4 Update frontend import/load summaries to show match quality without requiring a new review UI.

## 5. Tests

- [x] 5.1 Add core unit tests for exact, EDID, normalized, vocabulary, ambiguous EDID, and ambiguous vocabulary behavior.
- [x] 5.2 Update existing matching tests for the new normalized-before-vocabulary tier order.
- [x] 5.3 Add or update SST load tests proving SST uses enhanced matching and preserves params.
- [x] 5.4 Add DTO/TypeScript coverage through `npx tsc --noEmit`.

## 6. Verification

- [x] 6.1 Run `cargo test -p xt-core --lib matching`.
- [x] 6.2 Run `cargo test -p xt-core --lib`.
- [x] 6.3 Run `cargo build -p xtranslator-tauri`.
- [x] 6.4 Run `cd ui && npx tsc --noEmit`.
- [x] 6.5 Record any skipped real-data E2E validation and why it was skipped. Re-ran `cargo test -p xt-core --test e2e_real_data` and it passed; nothing was skipped.
