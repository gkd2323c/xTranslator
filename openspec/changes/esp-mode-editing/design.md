## Context

The Delphi original supports two fundamentally different ESP loading/saving modes:

| Mode | Loading | Saving |
|------|---------|--------|
| `sTESVEsp` (delocalized) | Strings are inline text in record subfield buffers | Mutate field buffers → rebuild records → recompress → serialize entire ESP |
| `sTESVEspStrings` (localized) | 4-byte string IDs referencing external `.STRINGS` files | Only write external `.STRINGS` files (already implemented) |

The Rust rewrite currently only supports `sTESVEspStrings` mode. ESP parsing stores strings in `SkyString` but the original field buffer offsets are not tracked, so we cannot write back.

**Current parser architecture**: `EspParser::parse()` iterates records and GRUPs, decompressing compressed records on the fly. The `trecord` (list of fields) is NOT retained — only extracted strings are kept. This is read-only by design.

**Delphi architecture reference** (`TESVT_espDefinition.pas:1770-1849`):
- `trecord.rebuildData()`: walk fields → manage XXXX size prefixes → recalc dsize → zlib recompress
- `tEspLoader.saveEsp()`: rebuildAllRecords → RebuildGrupsSize → serialize tree to disk
- `tField.updateBuffer()`: encode translation into field's buffer, update dsize

## Goals / Non-Goals

**Goals:**
- Full ESP record rebuild pipeline: modify field buffers → recalculate sizes → recompress → serialize
- Support both delocalized ESP editing AND localized→delocalized conversion
- Preserve 100% binary-fidelity for non-string fields (everything not in the translation list is passed through unchanged)
- ESP mode toggle in UI, preserved in app config
- Automatic backup before any ESP write (mirrors Delphi's `makeBackup` behavior)

**Non-Goals:**
- ESPM/ESL flag manipulation (only modify translatable string fields)
- Record addition/removal (only modify existing records; the Delphi's `saveArrayRefr` is out of scope)
- GRUP restructuring (groups stay where they are)
- Starfield ESL v1.1 format (ESL v1.0 only, same as current parser)

## Decisions

### D1: Rich parse tree instead of offset patching

**Choice**: During ESP parsing in "ESP mode", retain the full in-memory record tree (`Vec<EspRecord>` with `Vec<EspField>`) alongside the extracted `Vec<SkyString>`. Each `SkyString` gets a back-reference to its owning record+field.

**Rationale**: Offset patching (seek-and-write into the original file) is fragile — compressed records change size after modification, XXXX size prefixes cascade. The Delphi approach of rebuilding the full tree in memory is simpler and proven to work.

**Alternative considered**: Patch offsets in-place. Rejected because compressed record sizes change unpredictably after recompression.

### D2: Two-phase parse — light scan then full parse

**Choice**: Default load uses the existing light parser (Strings mode, quick). ESP mode triggers a second pass that builds the full record tree. The light parse result can be cached; the full tree is always built fresh when ESP mode is activated.

**Rationale**: Most users open the tool to browse strings. Full tree parsing adds ~30% memory/time overhead. Only build the tree when the user explicitly switches to ESP mode or clicks "Save to ESP".

### D3: zlib via `flate2` (already a dependency)

**Choice**: Use `flate2::Compress` for zlib recompression, matching the Delphi's `ZCompress`.

**Rationale**: `flate2` with `zlib-ng` backend is already a workspace dependency. Delphi uses standard zlib (RFC 1950). `flate2::Compress::new(Compression::best())` with `compress_vec()` produces compatible output.

### D4: Codepage encoding for field buffers

**Choice**: Use `encoding_rs` (already a dependency) for codepage encoding when writing translations into field buffers, matching the Delphi's `codePageProc`.

**Rationale**: The current codepage table already handles 932/936/949/950/1250-1257. Field buffers must be encoded in the ESP's target codepage, not always UTF-8.

### D5: Localized→delocalized via SST match-first strategy

**Choice**: When delocalizing, first apply SST dictionary matches to fill translations, then for unmatched strings use the existing Strings files as source. Reassign sequential string IDs starting from 1.

**Rationale**: Matches the Delphi's `SaveLocalizedClick` flow: apply SST (2 passes: strict + relaxed) → update all record field buffers with new string IDs → save ESP → export `.STRINGS`.

## Risks / Trade-offs

- **Memory**: Full record tree for Skyrim.esm (~238MB compressed, ~800MB decompressed) uses ~1.5GB RAM during edit. Acceptable for a desktop tool. Mitigation: tree is only built when ESP mode is active.
- **Binary fidelity**: Non-string subrecords must pass through unchanged. Any deviation could corrupt the ESP. Mitigation: extensive roundtrip tests (parse → serialize → re-parse, assert identical strings).
- **XXXX size prefix cascades**: When a field exceeds 65535 bytes after translation, a new XXXX field must be inserted. The Delphi handles this in `rebuildData` with backward iteration. Mitigation: port the exact same algorithm.
- **Compressed record size changes**: After recompression, the record's `dsize` changes. GRUP `dsize` must be recalculated. Mitigation: two-pass approach — rebuild all records first, then recalculate GRUP sizes in a second pass.
- **GRUP header type field**: Not all GRUPs store records the same way (world children have grid coords). Mitigation: only support top-level GRUPs and child GRUPs for known types (WRLD/CELL). Other nested GRUPs are passed through as opaque blobs.
