## 1. Core Module Verification

- [x] 1.1 Verify `crates/xt-core/src/normalization.rs` module is complete (tests pass)
- [x] 1.2 Add `pub mod normalization;` to `crates/xt-core/src/lib.rs`

## 2. SkyString Implementation

- [x] 2.1 Update `SkyString::new()` signature to include `record_sig`, `field_sig`
- [x] 2.2 Implement auto-normalization in `SkyString::new()` (populate `source_normalized`, `normalized_hash`)
- [x] 2.3 Implement auto-word-hash computation in `SkyString::new()` (populate `word_hashes`)

## 3. EspPointer Implementation

- [x] 3.1 Add `EspPointer::null()` convenience constructor

## 4. ESP Parser Fixes

- [x] 4.1 Add `compressed_records: u32` field to `EspParser`
- [x] 4.2 Increment `compressed_records` counter when decompressing records
- [x] 4.3 Compute `edid_hash` when creating `EspPointer` during record parsing
- [x] 4.4 Pass `record_sig` and `field_sig` to `SkyString::new()`

## 5. Update all Call Sites

- [x] 5.1 Update `crates/xt-core/src/sst/v8.rs` SST reader call sites
- [x] 5.2 Update `crates/xt-core/src/xml/mod.rs` XML importer call sites
- [x] 5.3 Update `crates/xt-core/src/testing/generator.rs` test generator
- [x] 5.4 Update `crates/xt-cli/src/commands/sst.rs` SST generator (found additional call site)

## 6. Testing & Verification

- [x] 6.1 Run `cargo test -p xt-core` and ensure all tests pass (73/73 passed)
- [x] 6.2 Run `cargo build --workspace` to verify compilation
- [x] 6.3 Verify SPEC.md B1-B4 are marked as fixed
