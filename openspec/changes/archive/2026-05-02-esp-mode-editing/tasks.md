## 1. Core data structures

- [x] 1.1 Define `EspField` struct in `crates/xt-core/src/esp/`: `header` (GenericHeader), `buffer` (Vec<u8>), `is_compressed` flag, `is_size_xxxx` flag
- [x] 1.2 Define `EspRecord` struct: `header` (RecordHeader), `fields` (Vec<EspField>), `compressed` flag, `raw` flag, `form_id` (u32), `editor_id` (Option<String>)
- [x] 1.3 Define `EspGrup` struct: `header` (GenericHeader + GrupHeader), `records` (Vec<EspRecord>), `children` (Vec<EspGrup>)
- [x] 1.4 Add `record_tree: Vec<EspGrup>` to `EspParser` state, populated only when ESP mode is active
- [x] 1.5 Add `field_ref: Option<usize>` (index into record.fields) to `SkyString` for back-reference during write-back

## 2. Record rebuild

- [x] 2.1 Implement `EspField::update_buffer(text: &str, codepage: &CodepageTable)` — encode translation into field buffer, update header.dsize
- [x] 2.2 Implement `EspRecord::rebuild_data()` — walk fields, manage XXXX size prefix fields (backward iteration per Delphi algorithm), recalculate all dsize values
- [x] 2.3 Implement zlib recompression in rebuild: serialize fields to contiguous buffer → `flate2::Compress::compress_vec()` → prepend 4-byte decompressed size LE
- [x] 2.4 Handle raw records: pass through unchanged, no rebuild or recompression
- [x] 2.5 Unit tests: test_rebuild_no_change, test_rebuild_with_translation, test_rebuild_compressed, test_rebuild_xxxx_field

## 3. ESP serialization

- [x] 3.1 Implement `EspGrup::recalculate_size()` — compute dsize from children (records + sub-GRUPs), including 24B GRUP header
- [x] 3.2 Implement `EspRecord::serialize(writer)` — write GenericHeader + RecordHeader + (compressed blob or fields sequentially)
- [x] 3.3 Implement `EspGrup::serialize(writer)` — recursive: write GRUP header, then serialize children (records and sub-GRUPs)
- [x] 3.4 Implement `save_esp_to_file(path, tree)` — TES4 header + top-level GRUPs, backup creation before write
- [x] 3.5 Unit tests: test_serialize_roundtrip (parse → serialize → re-parse, assert identical strings), test_backup_created

## 4. ESP mode parser integration

- [x] 4.1 Modify `EspParser::parse()` to optionally build `record_tree` when a flag is set (`build_record_tree: bool`)
- [x] 4.2 Wire up `SkyString.field_ref` during parsing — when a string field is extracted, record the field index into its parent record
- [x] 4.3 Add `EspParser::enable_esp_mode(&mut self)` — triggers full tree build on next parse (re-parse if needed)
- [x] 4.4 Handle GRUP types: top-level GRUPs always parsed; child GRUPs for WRLD/CELL decomposed by grid coordinates

## 5. Localized→delocalized conversion

- [x] 5.1 Implement `delocalize_record(record, strings_map)` — replace 4-byte string ID fields with inline text from strings files
- [x] 5.2 Implement `reassign_string_ids(strings)` — assign sequential IDs (1..N) in source text order
- [x] 5.3 Implement `export_strings_on_delocalize(strings, output_dir, language)` — write .STRINGS, .DLSTRINGS, .ILSTRINGS with new IDs
- [x] 5.4 Implement 2-pass SST match before delocalization: strict (triple match) then relaxed (normalized text)
- [x] 5.5 Unit tests: test_delocalize_minimal, test_id_reassignment, test_strings_export

## 6. IPC commands

- [x] 6.1 Add DTOs to `crates/xt-shared/src/dto.rs`: `FinalizeEspRequest`, `FinalizeEspResponse`, `DelocalizeEspRequest`, `DelocalizeEspResponse`, `EspModeConfig`
- [x] 6.2 Add `esp_mode: Option<bool>` field to `AppConfigDto` (merge-only, per V24)
- [x] 6.3 Implement `save_esp` command in `src-tauri/src/commands.rs`: route to ESP write when in ESP mode, Strings write otherwise
- [x] 6.4 Implement `finalize_esp` command: apply SST → rebuild → serialize → export Strings
- [x] 6.5 Implement `delocalize_esp` command: localized ESP → delocalized conversion flow
- [x] 6.6 Register new commands in `src-tauri/src/main.rs` via `generate_handler!`
- [x] 6.7 Add TypeScript types and invoke wrappers in `ui/src/api/strings.ts`

## 7. Frontend integration

- [x] 7.1 Add ESP mode toggle to Settings dialog (persisted via `save_config`)
- [x] 7.2 Add "Finalize ESP" menu item and toolbar button (visible only in ESP mode)
- [x] 7.3 Add "Delocalize ESP" menu item (visible only for localized ESPs)
- [x] 7.4 Show save-mode indicator in status bar or SidePanel ("Strings mode" / "ESP mode")
- [x] 7.5 Wire save button click to conditional routing: `save_esp` or `save_strings` based on mode

## 8. SPEC invariants

- [x] 8.1 Add V29-V32 to `SPEC.md`: backup, compression format, non-string pass-through, sequential ID assignment
- [x] 8.2 Update V7: remove "ESP itself unmodified" clause, add conditional routing based on ESP mode
- [x] 8.3 Add new tasks T42-T45 to `SPEC.md` §T

## 9. Verification

- [x] 9.1 Run full unit test suite: `cargo test -p xt-core --lib` (247 passed, 0 failed)
- [ ] 9.2 Run E2E tests in release mode: `cargo test --release -p xt-core --test e2e_real_data` (pre-existing: requires Skyrim.esm, fails with memory allocation)
- [ ] 9.3 Run smoke tests in release mode: `cargo test --release -p xt-core --test smoke_test` (pre-existing: requires Skyrim.esm, fails with memory allocation)
- [x] 9.4 Build check: `cargo build -p xtranslator-tauri` (succeeded)
- [x] 9.5 TypeScript check: `cd ui && npx tsc --noEmit` (clean)
- [ ] 9.6 Manual test: open delocalized ESP → translate → save_esp → reopen → verify translations persist
- [ ] 9.7 Manual test: open localized ESP → delocalize → verify ESP has inline text and .STRINGS files are exported