# SPEC — xTranslator Rust Rewrite

## §G Goals

G1: Parse ESP/ESM (Skyrim SE/FO4/Starfield) → extract translatable strings via record_defs
G2: Load/write Bethesda .STRINGS/.DLSTRINGS/.ILSTRINGS with codepage fallback (932/936/949/950/1250-1257)
G3: Full SST v8 bidirectional compatibility with Delphi xTranslator (UTF-16LE, FNV-1a, 24B EspPointer)
G4: Delphi-compatible XML import/export (entity escape, trim, REC:FIELD sigs)
G5: Heuristic similarity search (Levenshtein + LCS + LCP) on translated corpus
G6: Translation API provider trait; OpenAI + DeepL implementations (env key or runtime set)
G7: Tauri 2.x desktop app; React frontend with client-side virtual scroll (react-window v2)
G8: BSA v0x68/v0x69 and BA2 General fallback for strings when standalone files missing

## §C Constraints

C1: DTO source of truth = `crates/xt-shared/src/dto.rs`; TypeScript mirror = `ui/src/api/strings.ts`. Both must sync on field changes.
C2: IPC payload >1MB risks WebView2 `postMessage` limits. Bulk data must use chunking (`get_strings_chunk`, ~10K items / ~2MB per batch).
C3: Strings write-back uses HashMap dedup (V16); shared data offsets for identical content → ~17% size reduction vs undeduplicated output.
C4: E2E tests require Skyrim SE at `D:\SteamLibrary\steamapps\common\Skyrim Special Edition\Data\Skyrim.esm`.
C5: `record_defs` loading best-effort; missing `Data/<Game>/record_defs` → fallback to built-in default defs.
C6: Tauri dev: `beforeDevCommand: "echo ok"` in `tauri.conf.json` because `cd ui && npm run dev` fails in PowerShell. Use `dev.ps1` or two terminals.
C7: `update_translation` uses internal `u32 id` (not array index). Indices invalid after filter/sort.
C8: react-window@2.x API uses `rowComponent`/`rowCount`/`rowHeight`/`rowProps`. Do NOT install `@types/react-window`.
C9: FNV-1a hash uses low byte of UTF-16 code units (matches Delphi `byte(str[i])`).
C10: EspPointer SIZE = 24 bytes LE; SST magic = `0x39555353`.

## §I Interfaces

### IPC Commands (Tauri invoke)

api: `load_esp` → `LoadEspResponse { total, compressed_records, strings_loaded, parse_time_ms, record_counts, cached }`
api: `load_sst` → `LoadSstResponse { matched, unmatched }`
api: `save_sst` → `()`
api: `update_translation` → `()` (takes `u32 id`, not index)
api: `get_strings_chunk` → `Vec<SkyStringDTO>` (bulk fetch, avoids IPC limit)
api: `get_strings_count` → `u32`
api: `get_all_strings` → `Vec<SkyStringDTO>` (fallback, small datasets only)
api: `query_strings_command` → `QueryResponse` (backend filter/sort/paginate, deprecated for virtual scroll)
api: `get_stats` → `String` (human-readable counts + memory estimate)
api: `heuristic_search` → `Vec<HeuristicMatchDTO>` (candidates from translated strings only)
api: `translate_string` → `String` (current provider, `spawn_blocking`)
api: `set_openai_api_key` → `()` (memory only, no disk persistence)
api: `set_deepl_api_key` → `()` (memory only, no disk persistence)
api: `set_translation_provider` → `()` (switch between openai/deepl)
api: `get_translation_providers` → `Vec<String>` (available provider list)
api: `export_xml` → `u32` (exported count, emits `xml-progress` events)
api: `import_xml` → `XmlImportResponse { matched, unmatched, total, updated_ids }`
api: `get_is_dirty` → `bool`
api: `save_strings` → `SaveStringsResponse { strings_count, dlstrings_count, ilstrings_count, translated_count }`
api: `compare_esp_files` → `EspCompareResultDto { identical_count, added_count, removed_count, modified_count, identical[], added[], removed[], modified[] }`
api: `load_mcm_file` → `McmFileDto { path, entry_count, encoding, entries[] }`
api: `save_mcm_file` → `()` (takes `McmSaveRequest { path, entries[] }`)
api: `compile_pex` → `String` (output path, takes pex_path, output_path, translations[])
api: `list_ba2_files` → `BsaFileListDto` (reuses BSA DTOs)
api: `extract_ba2_file` / `extract_ba2_folder` → same as BSA equivalents
api: `load_config` → `AppConfigDto` (persisted JSON config: theme, language, API keys, proxy)
api: `save_config` → `()` (takes `AppConfigDto`, merge-only update, writes to disk)
api: `get_api_config` → `ApiConfigResponse { providers: Vec<ApiProviderInfo> }` (parsed from `Misc/ApiTranslator.txt`)
api: `tcsc_batch_convert` → `Vec<u32>` (batch convert all filtered translations, takes direction + optional ids) — direction: "to_simplified"|"to_traditional", returns updated IDs
api: `load_vocabulary` → `VocabularyInfo { pair_count, base_names }` (parse vocabulary.txt, load Strings pairs, store in AppState, enrich heuristic search)
api: `compare_source_dest` → `u32` (compare source vs translation hashes; mode: "diff" or "same", returns tagged count)
api: `check_aliases` → `AliasCheckResult { source_aliases, trans_aliases, missing_in_trans, extra_in_trans, has_mismatch }` (alias integrity check for a single string by id)

### Events

evt: `esp-load-progress` → `EspLoadProgress { stage, current, total, percentage, message }` (stage may be "cached" on cache hit)
evt: `xml-progress` → `XmlProgress { stage, current, total, percentage, message }`

### Core Types

type: `SkyString` → `{ id: u32, source: String, translation: String, record_sig: [u8;4], field_sig: [u8;4], source_normalized: Option<String>, normalized_hash: Option<u32>, hash: u32, hash_trans: u32, word_hashes: Vec<u32>, rec_refs: Vec<u64>, esp_ptr: EspPointer, params: SkyStringParams, internal_params: SkyStringInternalParams, list_index: u8, colab_id: u8, ld_result: f32, ld_found: i32, min_word: i32, tag_hash: u32 }`
type: `EspPointer` → 24B LE: `{ str_id: i32, form_id: u32, record_sig: [u8;4], field_sig: [u8;4], index: u16, index_max: u16, edid_hash: u32 }`
type: `SkyStringParams` → `u8` bitflags: TRANSLATED=0x01, LOCKED_TRANS=0x02, INCOMPLETE_TRANS=0x04, VALIDATED=0x08, OLD_DATA=0x40, PENDING=0x80
type: `SkyStringInternalParams` → `u64` runtime-only flags (not persisted to SST)
type: `StringsFormat` → `NullTerminated` | `LengthPrefixed`
type: `GameId` → `Skyrim | SkyrimSE | Fallout4 | FalloutNV | Fallout76 | Starfield`

### File Formats

fmt: `.STRINGS` → header {count, data_size} + directory[id,offset] * N + null-terminated data
fmt: `.DLSTRINGS`/`.ILSTRINGS` → same header + directory + 4-byte length-prefixed data (len includes trailing null)
fmt: ESP compressed record → `[4-byte decompressedSize LE] + [zlib data]`
fmt: ESP dsize semantics → Record dsize excludes 16B header; GRUP dsize includes 24B header
fmt: SST v8 → magic(4) + v4_flag(1) + master_list + colab_labels + entries[ list_index(1) + EspPointer(24) + colab_id(1) + params(1) + source(UTF-16LE) + translation(UTF-16LE) ]
fmt: XML export → `SSTXMLRessources/Params/Content/String` with `List`, `sID`, `EDID`, `REC[id,idMax]`, `Source`, `Dest`

### Environment

env: `XT_TRANSLATE_API_KEY` ? set → OpenAI provider auth (env fallback)
env: `XT_DEEPL_API_KEY` ? set → DeepL provider auth (env fallback, auto-detects free/pro)
env: `XT_TRANSLATE_API_BASE` ? override default OpenAI base URL
env: `XT_TRANSLATE_API_MODEL` ? override default model (default `gpt-4o-mini`)

## §V Invariants

V1: ∀ SkyString → `hash` == `FNV1a_low_byte(source)`, `hash_trans` == `FNV1a_low_byte(translation)`
V2: ∀ update_translation(id, text) → linear scan by `id` (not index), set TRANSLATED=true if text non-empty else INCOMPLETE_TRANS=true
V3: ∀ load_esp → overwrite `AppState.strings`, reset `is_dirty=false`
V4: ∀ SST/XML match → key = `(str_id, record_sig, field_sig)` triple. No other fields participate.
V5: ∀ XML export → only items with non-empty `translation` included
V6: ∀ heuristic_search → candidate set = strings where `params.is_translated() == true`
V7: ∀ save_strings → load source-language files as base, overwrite by translated entries, write target-language files. ESP itself unmodified.
V8: ∀ GMST:DATA → if EDID starts with 's' → keep (string ref); else skip (numeric)
V9: ∀ StringsFile.save → entries sorted by id asc; offsets relative to data section start
V10: ∀ codepage decode → UTF-8 primary; on failure use configured fallback; no fallback → byte-by-byte fallback
V11: ∀ archive fallback → scan `.bsa` and BA2 General archives in ESP dir; extract from `strings/<filename>` via archive lookup
V12: ∀ frontend state → `allItems` (full DTO) → filter/sort → `items` (display). SidePanel stats from `allItems`.
V13: ∀ Zustand selectors → `useAppStore((s) => s.field)` (not `const store = useAppStore()`)
V14: ∀ react-window v2 → props: `rowComponent`, `rowCount`, `rowHeight`, `rowProps`
V15: ∀ SkyString::new → source_normalized = normalize(source) (case-fold, punct→space, whitespace compress, trim); normalized_hash = FNV1a_low_byte(source_normalized) if non-empty else None
V16: ∀ save_with_format → HashMap dedup by encoded entry bytes; identical content share data offset → ~17% file size reduction
V17: ∀ theme change → localStorage `xtranslator-theme` updated + `data-theme` attr set on documentElement
V18: ∀ replaceAll → confirmation dialog required; batch-update each candidate via update_translation; reload all strings after; progress toast shown
V19: ∀ loadAllStrings → get_strings_chunk primary (10K/batch); get_all_strings fallback (small datasets); query_strings last resort (paginated, no full store)
V20: ∀ load_esp → before parsing, check EsmCache via SHA-256 of ESP file in `%LOCALAPPDATA%/xTranslator/cache/`; on hit return cached bincode blob; on miss parse then store
V21: ∀ tests that mutate/read `XT_TRANSLATE_API_*` → isolate env changes under shared lock and restore prior values; assertions must not depend on the caller's ambient env
V22: ∀ parse_record_debug on GRUP → decrement record_count saturatingly; nested/empty GRUPs must never underflow the unsigned counter
V23: ∀ translate_string → protect_crlf(source) before API call, restore_crlf(response) after; `\r\n` and `\n` both become `<L_F>` tag, restored to `\r\n`
V24: ∀ AppConfig.save → merge-only: only `Some` fields in the input DTO overwrite existing config; `None` fields leave the stored value unchanged
V25: ∀ TCSC conversion → OpenCC dictionary (primary, 3960 pairs) + Delphi Charset_SCTC.txt (fallback, 2552 pairs); both embedded at compile time via `include_str!`
V26: ∀ proxy settings UI → fields map directly to `AppConfig` proxy_server/proxy_port/proxy_username/proxy_password; `save_config` persists to disk; `translate_string` reads config on each call
V27: ∀ vocabulary loading → parse `Data/<Game>/vocabulary.txt` for STRINGS=Name entries, load source+target Strings files, match by str_id; pairs merged into heuristic search candidate set
V28: ∀ PEX string extraction → filter parameters of procedures listed in `Data/<Game>/pexNoTransProc.txt`; only translatable strings returned

## §T Tasks

id|status|task|cites
T1|x|ESP parser with zlib decompression, GRUP nesting, XXXX extended fields|G1,C5
T2|x|Strings file load/write (null-terminated + length-prefixed)|G2
T3|x|SST v8 read/write with Delphi-compatible UTF-16LE|G3
T4|x|XML import/export with entity escaping|G4
T5|x|Heuristic search (Levenshtein + LCS + LCP)|G5
T6|x|OpenAI translation provider|G6
T7|x|Tauri IPC commands + AppState|G7
T8|x|React frontend with react-window virtual scroll|G7,C14
T9|x|BSA v0x68/v0x69 archive extraction|G8
T10|x|Codepage fallback table (932/936/949/950/1250-1257)|G2
T11|x|Record type filtering (SidePanel click-to-filter)|G7
T12|x|Update by ID (not index) across filter/sort|C7
T13|x|Full-load + client-side filter/sort (<10ms)|C2
T14|x|XML progress events during import/export|G4
T15|x|DeepL translation provider|G6
T16|x|BSA v0x68/v0x69 and BA2 General archive browser/fallback|G8
T17|x|PEX script string extraction|G1
T18|x|FUZ audio mapping|G2
T19|x|Batch processor|G7
T20|x|NPC map / dialog view (parent_form_id tracking via GRUP s_type, grouping)|G7
T21|x|Theme system (dark/light/gray/auto, CSS variables, Zustand + localStorage + matchMedia)|G7
T22|x|UI multi-language i18n (react-i18next, 10 languages, Zh-CN default)|G7
T23|x|Regex search/replace with capture groups (replaceAll across filtered items)|G7
T24|x|Strings write-back deduplication (shared data offsets, ~17% size reduction)|G2
T25|x|Auto-backup (5-min timer, SST snapshots, rotate last 10)|G3
T26|x|Undo/Redo (stack-based, Ctrl+Z/Y, max 100 depth)|G7
T27|x|ESP parse result cache (SHA-256 key, bincode blob, auto-prune)|G7
T28|x|MCM (Mod Configuration Menu) translation file support (UTF-16LE/BE/UTF-8, BOM, encoding preserve)|G1
T29|x|ESP comparison engine (old/new diff with identical/modified/added/removed classification)|G1
T30|x|BA2 archive support (Fallout 4 v0x01, Starfield v0x02, FO4B v0x08)|G8
T31|x|PEX compile (in-place string table update with index preservation, binary roundtrip)|G1
T32|x|Config persistence (JSON file, theme/language/API key survive restart)|G7
T33|x|TCSC simplified/traditional Chinese conversion (character-pair mapping, OpenCC 3960 + Delphi 2552 fallback)|G7
T34|x|Proxy settings UI (settings dialog with proxy server/port/username/password fields, integrates with build_client)|G7
T35|x|Batch TCSC conversion (convert all filtered translations at once, not just selected item)|G7
T36|x|vocabulary.txt integration (parse STRINGS=Name entries, load source+target Strings, match by str_id, enrich heuristic search candidates)|G5
T37|x|pexNoTransProc.txt filtering (parse procedure names, filter non-translatable PEX string parameters)|G1
T38|x|HiDPI support (Tauri 2.x native HiDPI + decorations/dragDrop window config)|G7
T39|x|Drag-drop extension (route BSA/BA2, PEX, FUZ file drops to correct handlers)|G7
T40|x|Source/Dest compare (compare source hash vs translation hash, tag diff/same as incomplete)|G7
T41|x|Alias integrity check (extract <Alias=...> tags, compare source vs translation, highlight mismatches in EditorPanel)|G7

## §B Bugs

id|date|cause|fix
B1|2026-04-20|compressed_records stat always 0 in LoadEspResponse|Fixed: track count during parsing
B2|2026-04-20|edid_hash always 0 in EspPointer (EDID hashing not implemented)|Fixed: compute FNV-1a hash from EDID string
B3|2026-04-20|source_normalized & normalized_hash always None/None (normalization not implemented)|Fixed: add normalization and hash computation in SkyString::new
B4|2026-04-20|SkyString.word_hashes always empty (tokenization not implemented)|Fixed: tokenize source and compute hashes in SkyString::new
B5|2026-04-27|OpenAI provider tests read ambient XT_TRANSLATE_API_BASE/MODEL, so local env changed default assertions|Fixed: isolate and restore XT_TRANSLATE_API_* under shared test env lock (V21)
B6|2026-04-27|parse_record_debug decremented record_count unconditionally for GRUPs, underflowing on empty/leading groups in real Skyrim data|Fixed: saturating_sub on GRUP traversal + regression test (V22)
