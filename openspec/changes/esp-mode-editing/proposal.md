## Why

The Rust rewrite currently only supports Strings mode (external `.STRINGS`/`.DLSTRINGS`/`.ILSTRINGS` files), leaving users who work with delocalized ESPs (strings embedded inline in record subfields) unable to translate those files end-to-end. The Delphi original supports full round-trip ESP editing — including record reconstruction, zlib recompression, and localized→delocalized conversion. Closing this gap is the biggest missing feature (~30% of Delphi parity).

## What Changes

- **New**: ESP record field buffer mutation — write translated strings (codepage-encoded) into in-memory field buffers with size recalculation
- **New**: ESP record rebuild pipeline — decompress → mutate fields → recalculate dsize → recompress with zlib
- **New**: Full ESP serialization — walk the in-memory record tree, serialize headers + fields + GRUPs, write to file
- **New**: Localized→delocalized conversion — replace 4-byte string IDs with inline text, reassign sequential IDs, export matching `.STRINGS` files
- **New**: IPC commands: `save_esp`, `finalize_esp`, `delocalize_esp`
- **Modified**: `save_strings` — when in ESP mode, route to ESP write-back instead of external file write

## Capabilities

### New Capabilities
- `esp-record-rebuild`: Reconstruct compressed ESP records with modified string data (field buffer mutation, dsize recalculation, zlib recompression)
- `esp-serialize`: Walk the in-memory record tree and serialize to a valid ESP/ESM file (headers, fields, GRUP sizes, compressed records)
- `esp-delocalize`: Convert localized ESP (4-byte string IDs) to delocalized (inline text), reassign IDs, export `.STRINGS` files
- `esp-writeback-commands`: IPC commands (`save_esp`, `finalize_esp`, `delocalize_esp`) and Tauri integration

### Modified Capabilities
- `project-governance`: Add invariants V29-V32 for ESP write-back behavior (compression header format, record reconstruction, field index preservation, localized→delocalized ID assignment)

## Impact

- **`crates/xt-core/src/esp/`**: Major additions — `writer.rs` (new module), `rebuild.rs` (record rebuild), `serialize.rs` (ESP serialization). Modifications to `parser.rs` to track record positions for in-place mutation.
- **`crates/xt-shared/src/dto.rs`**: New DTOs: `FinalizeEspRequest`, `FinalizeEspResponse`, `DelocalizeEspRequest`, `DelocalizeEspResponse`
- **`src-tauri/src/commands.rs`**: New IPC commands: `save_esp`, `finalize_esp`, `delocalize_esp`
- **`ui/src/api/strings.ts`**: New frontend API wrappers and TypeScript types
- **`ui/src/components/`**: New UI toggle for ESP mode vs Strings mode; "Finalize ESP" menu item
- **SPEC.md**: New invariants V29-V32, update V7 (remove "ESP itself unmodified" clause, add conditional routing)
- **BREAKING**: None. All existing Strings mode behavior is preserved. ESP mode is opt-in via a toggle.
