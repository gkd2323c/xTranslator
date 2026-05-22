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
G9: RTL/Arabic text processing (logical→presentation shaping, bidirectional reorder)
G10: Crash-safe translation cache (JSONL journal, recovery on restart)
G11: PEX decompiler (pseudocode output)
G12: Dialog tree view (NPC→DIAL→INFO grouping)
G13: Delphi-compatible heuristic scoring (6-dimension composite: word hash, LCS, LCP, alias proxy, dynamic threshold)

## §C Constraints

C1: DTO source of truth = `crates/xt-shared/src/dto.rs`; TypeScript mirror = `ui/src/api/strings.ts`. Both must sync on field changes.
C2: IPC payload >1MB risks WebView2 `postMessage` limits. Bulk data must use chunking (`get_strings_chunk`, 25K items/batch, concurrency 3).
C3: Strings write-back uses HashMap dedup (V16); shared data offsets for identical content → ~17% size reduction vs undeduplicated output.
C4: E2E tests require Skyrim SE at `D:\SteamLibrary\steamapps\common\Skyrim Special Edition\Data\Skyrim.esm`.
C5: `record_defs` loading best-effort; missing `Data/<Game>/record_defs` → fallback to built-in default defs.
C6: Tauri dev: `beforeDevCommand: "echo ok"` in `tauri.conf.json` because `cd ui && npm run dev` fails in PowerShell. Use `dev.ps1` or two terminals.
C7: `update_translation` uses internal `u32 id` (not array index). Indices invalid after filter/sort.
C8: react-window@2.x API uses `rowComponent`/`rowCount`/`rowHeight`/`rowProps`. Do NOT install `@types/react-window`.
C9: FNV-1a hash uses low byte of UTF-16 code units (matches Delphi `byte(str[i])`).
C10: EspPointer SIZE = 24 bytes LE; SST magic = `0x39555353`.
C11: Filter debounce 150ms (`FILTER_DEBOUNCE_MS` in `appStore.ts`).
C12: MAX_UNDO_STACK = 100 in frontend store.
C13: AUTO_BACKUP_INTERVAL_MS = 300000 (5 min) in `App.tsx`.
C14: String-level batch translation concurrency clamped [1, 10].
C15: SQLite cache uses WAL mode + NORMAL synchronous.
C16: CacheIndex stored as JSON (`cache_index.json`) alongside SQLite cache files.
C17: Translation cache = append-only JSONL journal in `translation_cache/` subdirectory.
C18: ESP mode persisted in `config.json` (`esp_mode` field); controls save target (ESP vs .STRINGS).
C19: MCM partial threshold < 30% of source length.
C20: Jaccard threshold 0.5 for vocabulary overlap matching (`MIN_JACCARD` in `matching.rs`).

## §I Interfaces

### IPC Commands (Tauri invoke)

#### Core Load/Save

api: `load_esp` → `LoadEspResponse { total, compressed_records, strings_loaded, parse_time_ms, record_counts, cached, esp_hash }`
api: `load_sst` → `LoadSstResponse { matched, unmatched, updated_ids, tier_exact, tier_edid, tier_normalized, tier_vocab, ambiguous, pending_skipped, old_data_preserved, warning, big_warning }`
api: `save_sst` → `()`
api: `get_esp_header` → `EspHeaderInfoDto { version, num_records, next_object_id, author, description, masters[], flags }`
api: `save_esp` → `SaveEspResponse { bytes_written, records_modified }` (direct ESP write-back; takes `SaveEspRequest { path, create_backup }`)
api: `finalize_esp` → `FinalizeEspResponse { esp_path, strings_files[], records_modified }` (SST→ESP rebuild+serialize+Strings export)
api: `delocalize_esp` → `DelocalizeEspResponse { new_string_count, strings_files_paths[] }` (sequential ID reassignment)
api: `finalize` → `FinalizeResponse` (orchestrates save_strings + save_sst + export_xml)
api: `save_strings` → `SaveStringsResponse { strings_count, dlstrings_count, ilstrings_count, translated_count }`

#### String Access

api: `update_translation` → `()` (takes `u32 id`, not index)
api: `batch_update_translations` → `u32` (batch update multiple translations, returns count)
api: `get_strings_chunk` → `Vec<SkyStringDTO>` (25K items/batch, concurrency 3)
api: `get_strings_count` → `u32`
api: `get_all_strings` → `Vec<SkyStringDTO>` (fallback, small datasets only)
api: `query_strings_command` → `QueryResponse` (backend filter/sort/paginate, deprecated for virtual scroll)
api: `get_stats` → `String` (human-readable counts + memory estimate)
api: `compare_esp_files` → `EspCompareResultDto { identical_count, added_count, removed_count, modified_count, identical[], added[], removed[], modified[] }`
api: `compare_source_dest` → `u32` (compare source vs translation hashes; mode: "diff"|"same", returns tagged count)
api: `check_aliases` → `AliasCheckResult { source_aliases, trans_aliases, missing_in_trans, extra_in_trans, has_mismatch }`

#### Translation

api: `heuristic_search` → `Vec<HeuristicMatchDTO>` (candidates from translated strings only)
api: `translate_string` → `String` (current provider, `spawn_blocking`)
api: `set_openai_api_key` → `()` (memory only, no disk persistence)
api: `set_deepl_api_key` → `()` (memory only, no disk persistence)
api: `set_translation_provider` → `()` (switch between openai/deepl/baidu/youdao)
api: `get_translation_providers` → `TranslationProvidersResponse { providers[], openaiConfigured, deeplConfigured, baiduConfigured, youdaoConfigured }`
api: `set_baidu_api_key` → `()` (takes app_id + key, memory only)
api: `set_yooudao_api_key` → `()` (takes app_key + secret_key, memory only)
api: `tcsc_batch_convert` → `Vec<u32>` (batch convert filtered translations; direction: "to_simplified"|"to_traditional", optional ids)
api: `tcsc_convert` → `String` (single-string TCSC conversion; takes text + direction)
api: `rtl_reverse` → `String` (reverse RTL text for Arabic/Hebrew display)
api: `shape_arabic` → `String` (logical-order → presentation forms)
api: `deshape_arabic` → `String` (presentation forms → logical base chars)
api: `load_vocabulary` → `VocabularyInfo { pair_count, base_names }` (parse vocabulary.txt, enrich heuristic search)

#### XML

api: `export_xml` → `u32` (exported count, emits `xml-progress` events)
api: `import_xml` → `XmlImportResponse { matched, unmatched, total, updated_ids, tier_exact, tier_edid, tier_vocab, tier_normalized, ambiguous, pending_skipped, old_data_preserved, warning, big_warning }`

#### BSA/BA2 Archive

api: `list_bsa_files` → `BsaFileListDto` (list BSA archive contents)
api: `extract_bsa_file` → `()` (extract single file from BSA)
api: `extract_bsa_folder` → `()` (extract folder from BSA)
api: `list_ba2_files` → `BsaFileListDto` (reuses BSA DTOs)
api: `extract_ba2_file` / `extract_ba2_folder` → same as BSA equivalents

#### PEX

api: `parse_pex_strings` → `PexScriptDto { script_name, game_id, versions[], translatable[] }` (extract translatable strings)
api: `compile_pex` → `String` (output path; takes pex_path, output_path, translations[])
api: `decompile_pex` → `DecompilePexResponse { script_name, object_count, function_count, instruction_count, pseudocode }`

#### MCM

api: `load_mcm_file` → `McmFileDto { path, entry_count, encoding, entries[] }`
api: `save_mcm_file` → `()` (takes `McmSaveRequest { path, entries[] }`)
api: `mcm_compare` → `McmCompareResult { matched, unmatched, updated_entries[] }` (takes `McmCompareRequest { entries[], reference_path, policy }`)

#### FUZ / Dialog

api: `scan_fuz_directory` → `FuzScanResponse { fuz_mappings[], total_fuz_files }` (map FUZ files to dialog strings)
api: `get_fuz_audio_data` → `Vec<u8>` (extract WAV bytes from FUZ file)
api: `build_dialog_tree` → `DialogTreeDto { npcs[] }` (group INFO by parent DIAL FormID, associate NPC_ names)

#### Config

api: `load_config` → `AppConfigDto` (persisted JSON config: theme, language, API keys, proxy, esp_mode)
api: `save_config` → `()` (takes `AppConfigDto`, merge-only update, writes to disk)
api: `get_api_config` → `ApiConfigResponse { providers: Vec<ApiProviderInfo> }` (parsed from `Misc/ApiTranslator.txt`)
api: `get_is_dirty` → `bool`
api: `load_data_configs` → `DataConfigsDto { ctda_funcs[], field_size_ref[], dial_sub_type[], emote_definition[] }` (parse `Data/<Game>/` config files)

#### Batch (File-Level)

api: `start_batch_translate` → `String` (job_id; takes `BatchConfig { entries[], provider, target_lang, skip_translated }`)
api: `start_batch_export` → `String` (job_id; takes `Vec<BatchEntry>`)
api: `get_batch_status` → `BatchStatus { job_id, job_type, progress, errors[], elapsed_ms }`
api: `cancel_batch_job` → `()` (takes job_id)

#### Batch (String-Level)

api: `start_string_batch_translate` → `()` (concurrent translation of selected strings; concurrency clamped [1,10])
api: `cancel_string_batch_translate` → `()` (cancel in-progress string batch)

#### ESP Utilities

api: `list_esp_files` → `Vec<String>` (scan directory for ESP/ESM files)

#### Translation Cache (Crash Recovery)

api: `check_pending_cache` → `CheckPendingCacheResponse { recovery: Option<RecoveryInfo> }` (check for unapplied translations from previous session)
api: `apply_translation_cache` → `ApplyCacheResponse { applied_count }` (apply cached translations from JSONL journal)
api: `discard_translation_cache` → `()` (discard journal)

#### Auto-Backup

api: `auto_backup_sst` → `AutoBackupResponse { backup_path, total_backups }` (takes `AutoBackupRequest { sst_path, max_backups }`)

#### Tooling

api: `toolbox_transform` → `u32` (modified count; takes tool name, target [source|translation|both], optional ids, optional header_text)

### Events

evt: `esp-load-progress` → `EspLoadProgress { stage, current, total, percentage, message }` (stage may be "cached" on cache hit)
evt: `xml-progress` → `XmlProgress { stage, current, total, percentage, message }`
evt: `batch-progress` → `BatchProgress { job_id, file_path, stage, current_file, total_files, strings_translated, total_strings, message }`
evt: `batch-file-complete` → `BatchFileComplete { job_id, file_path, translated, skipped, errors, duration_ms }`
evt: `batch-complete` → `BatchComplete { job_id, total_files, success, failed, total_translated, total_errors, duration_ms, is_cancelled, errors[] }`
evt: `batch-string-progress` → `{ str_id, translated, error, completed, total }`
evt: `batch-string-complete` → `{ total, succeeded, failed, errors[] }`

### Core Types

type: `SkyString` → `{ id: u32, source: String, translation: String, record_sig: [u8;4], field_sig: [u8;4], source_normalized: Option<String>, normalized_hash: Option<u32>, hash: u32, hash_trans: u32, word_hashes: Vec<u32>, rec_refs: Vec<u64>, esp_ptr: EspPointer, params: SkyStringParams, internal_params: SkyStringInternalParams, list_index: u8, colab_id: u8, ld_result: f32, ld_found: i32, min_word: i32, tag_hash: u32, parent_form_id: Option<u32> }`
type: `EspPointer` → 24B LE: `{ str_id: i32, form_id: u32, record_sig: [u8;4], field_sig: [u8;4], index: u16, index_max: u16, edid_hash: u32 }`
type: `SkyStringParams` → `u8` bitflags: TRANSLATED=0x01, LOCKED_TRANS=0x02, INCOMPLETE_TRANS=0x04, VALIDATED=0x08, OLD_DATA=0x40, PENDING=0x80
type: `SkyStringInternalParams` → `u64` runtime-only flags (not persisted to SST)
type: `SkyStringDTO` → `{ id: u32, str_id: i32, source: String, translation: String, record_sig: String, field_sig: String, edid: String, is_translated: bool, is_locked: bool, is_incomplete: bool, is_vmad: bool, list_index: u8, hash: u32, hash_trans: u32, ld_result: f32, params: u8 }`
type: `StringsFormat` → `NullTerminated` | `LengthPrefixed`
type: `GameId` → `Skyrim | SkyrimSE | Fallout4 | FalloutNV | Fallout76 | Starfield`

#### Batch DTOs

type: `BatchEntry` → `{ esp_path, strings_dir, language, game, sst_path }`
type: `BatchConfig` → `{ entries: Vec<BatchEntry>, provider, target_lang, skip_translated }`
type: `BatchStatus` → `{ job_id, job_type, progress, errors[], elapsed_ms }`
type: `BatchProgress` → `{ job_id, file_path, stage, current_file, total_files, strings_translated, total_strings, message }`
type: `BatchFileComplete` → `{ job_id, file_path, translated, skipped, errors, duration_ms }`
type: `BatchComplete` → `{ job_id, total_files, success, failed, total_translated, total_errors, duration_ms, is_cancelled, errors[] }`
type: `BatchFileError` → `{ file_path, error }`

#### ESP DTOs

type: `EspHeaderInfoDto` → `{ version, num_records, next_object_id, author, description, masters[], flags }`
type: `SaveEspRequest` → `{ path, create_backup }`
type: `SaveEspResponse` → `{ bytes_written, records_modified }`
type: `FinalizeEspRequest` → `{ esp_path, source_lang, target_lang, create_backup }`
type: `FinalizeEspResponse` → `{ esp_path, strings_files[], records_modified }`
type: `DelocalizeEspRequest` → `{ esp_path, source_lang, target_lang, create_backup }`
type: `DelocalizeEspResponse` → `{ new_string_count, strings_files_paths[] }`
type: `EspCompareResultDto` → `{ identical_count, added_count, removed_count, modified_count, identical[], added[], removed[], modified[] }`

#### PEX DTOs

type: `PexTranslatableDto` → `{ index, function_name, param_name, source, translation }`
type: `PexScriptDto` → `{ script_name, game_id, versions[], translatable[] }`
type: `DecompilePexResponse` → `{ script_name, object_count, function_count, instruction_count, pseudocode }`

#### FUZ / Dialog DTOs

type: `FuzMapping` → `{ response_id, dialog_text, fuz_file, duration_secs }`
type: `FuzScanResponse` → `{ fuz_mappings[], total_fuz_files }`
type: `DialogInfoDto` → `{ id, form_id, source, translation, dialog_text }`
type: `NpcDialogDto` → `{ npc_edid, dialogues[] }`
type: `DialogTreeDto` → `{ npcs[] }`

#### MCM DTOs

type: `McmComparePolicy` → `All | NoTrans | NoTransAndPartial | PartialOnly`
type: `McmCompareRequest` → `{ entries[], reference_path, policy: McmComparePolicy }`
type: `McmCompareResult` → `{ matched, unmatched, updated_entries[] }`

#### Config DTOs

type: `AppConfigDto` → `{ theme?, language?, openai_api_key?, deepl_api_key?, translation_provider?, proxy_server?, proxy_port?, proxy_username?, proxy_password?, esp_mode? }`
type: `ApiConfigResponse` → `{ providers: Vec<ApiProviderInfo> }`
type: `DataConfigsDto` → `{ ctda_funcs[], field_size_ref[], dial_sub_type[], emote_definition[] }`
type: `CtdaFuncDto` → `{ id, name, params }`
type: `FieldSizeInfoDto` → `{ max_size, can_wrap }`

#### Translation Cache DTOs

type: `AutoBackupRequest` → `{ sst_path, max_backups }`
type: `AutoBackupResponse` → `{ backup_path, total_backups }`
type: `CheckPendingCacheResponse` → `{ recovery: Option<RecoveryInfo> }`
type: `RecoveryInfo` → `{ esp_name, pending_count, cache_file_path }`
type: `ApplyCacheResponse` → `{ applied_count }`
type: `FinalizeRequest` → `{ output_dir?, target_lang?, base_name? }`
type: `FinalizeResponse` → `{ strings_count, sst_saved, xml_exported }`
type: `TcscDirection` → `ToSimplified | ToTraditional`
type: `VocabularyInfo` → `{ pair_count, base_names[] }`
type: `AliasCheckResult` → `{ source_aliases[], trans_aliases[], missing_in_trans[], extra_in_trans[], has_mismatch }`

### File Formats

fmt: `.STRINGS` → header {count, data_size} + directory[id,offset] * N + null-terminated data
fmt: `.DLSTRINGS`/`.ILSTRINGS` → same header + directory + 4-byte length-prefixed data (len includes trailing null)
fmt: ESP compressed record → `[4-byte decompressedSize LE] + [zlib data]`
fmt: ESP dsize semantics → Record dsize excludes 16B header; GRUP dsize includes 24B header
fmt: SST v8 → magic(4) + v4_flag(1) + master_list + colab_labels + entries[ list_index(1) + EspPointer(24) + colab_id(1) + params(1) + source(UTF-16LE) + translation(UTF-16LE) ]
fmt: XML export → `SSTXMLRessources/Params/Content/String` with `List`, `sID`, `EDID`, `REC[id,idMax]`, `Source`, `Dest`
fmt: Translation cache journal → append-only JSONL; each line `{ esp_hash, str_id, source, translation, timestamp }`; flushed per-entry
fmt: CacheIndex → JSON file mapping `(path, mtime, size)` → SHA-256; avoids full hash on every load
fmt: SQLite ESP cache → WAL mode, NORMAL synchronous; `{sha256}.db` per cached ESP

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
V7: ∀ save_strings → load source-language files as base, overwrite by translated entries, write target-language files. When esp_mode=true, route to save_esp instead (direct ESP write-back).
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
V19: ∀ loadAllStrings → get_strings_chunk primary (25K/batch, concurrency 3); get_all_strings fallback (small datasets); query_strings last resort (paginated, no full store)
V20: ∀ load_esp → before parsing, check CacheIndex (mtime+size fast lookup); on miss check SQLite cache via SHA-256 of ESP file in `%LOCALAPPDATA%/xTranslator/cache/`; on hit return cached blob; on miss parse then store
V21: ∀ tests that mutate/read `XT_TRANSLATE_API_*` → isolate env changes under shared lock and restore prior values; assertions must not depend on the caller's ambient env
V22: ∀ parse_record_debug on GRUP → decrement record_count saturatingly; nested/empty GRUPs must never underflow the unsigned counter
V23: ∀ translate_string → protect_crlf(source) before API call, restore_crlf(response) after; `\r\n` and `\n` both become `<L_F>` tag, restored to `\r\n`
V24: ∀ AppConfig.save → merge-only: only `Some` fields in the input DTO overwrite existing config; `None` fields leave the stored value unchanged
V25: ∀ TCSC conversion → OpenCC dictionary (primary, 3960 pairs) + Delphi Charset_SCTC.txt (fallback, 2552 pairs); both embedded at compile time via `include_str!`
V26: ∀ proxy settings UI → fields map directly to `AppConfig` proxy_server/proxy_port/proxy_username/proxy_password; `save_config` persists to disk; `translate_string` reads config on each call
V27: ∀ vocabulary loading → parse `Data/<Game>/vocabulary.txt` for STRINGS=Name entries, load source+target Strings files, match by str_id; pairs merged into heuristic search candidate set
V28: ∀ PEX string extraction → filter parameters of procedures listed in `Data/<Game>/pexNoTransProc.txt`; only translatable strings returned
V29: ∀ save_esp → backup original ESP before write (unless user opted out); backup filename: `<original>.backup.<timestamp>`
V30: ∀ save_esp → compressed record output format is `[4-byte decompressedSize LE] + [zlib data]`; record header dsize = compressed output length
V31: ∀ record rebuild → non-string fields pass through unchanged; only fields with matching SkyString entries are modified; XXXX size prefix managed via backward iteration
V32: ∀ delocalize_esp → new string IDs are sequential starting from 1, ordered by source text; .STRINGS/.DLSTRINGS/.ILSTRINGS exported with new IDs
V33: ∀ VMAD strings → `is_vmad: esp_ptr.str_id < 0`; negative str_id encodes byte offset in script property data
V34: ∀ translation cache → append-only JSONL journal; each entry flushed immediately after translation; crash recovery via `check_pending_cache` + `apply_translation_cache`
V35: ∀ CacheIndex::lookup → check (mtime, size) first; on mismatch compute SHA-256 and update index; avoids full hash read on unchanged files
V36: ∀ SQLite ESP cache → WAL journal mode, NORMAL synchronous; supports indexed queries + single-row updates; replaces prior bincode format
V37: ∀ filter input → debounced 150ms before re-filtering `allItems`
V38: ∀ string-level batch → concurrency clamped [1, 10]; per-string progress via `batch-string-progress` event; cancel via `cancel_string_batch_translate`
V39: ∀ MCM compare → "partial" = non-empty translation, differs from source, < 30% of source length
V40: ∀ esp_mode=true → save operations write back to ESP file directly; esp_mode=false → write .STRINGS files
V41: ∀ load_esp → `codepage_table` stored in `AppState` per-game for correct encoding
V42: ∀ build_dialog_tree → group INFO records by parent DIAL FormID; associate NPC_ names from linked FormID
V43: ∀ finalize → orchestrate save_strings + save_sst + export_xml in single IPC call
V44: ∀ delocalize_esp → reassign string IDs sequentially from 1; export .STRINGS/.DLSTRINGS/.ILSTRINGS with new IDs
V45: ∀ `shape_arabic` → logical-order Unicode chars → presentation forms; `deshape_arabic` reverses
V46: ∀ `rtl_reverse` → reverse string for RTL display in LTR UI context
V47: ∀ `finalize_esp` → apply SST translations → rebuild record tree → serialize → export Strings files
V48: ∀ `header_rules_apply` → build EDID map from EspFile top_level_grups (recursive); use `sk.esp_ptr.form_id` as form_id
V49: ∀ `HeaderRule.matches_string` → exclude_keywords negate match regardless of no_kw flag
V50: ∀ `HeaderRule.apply` → if regex set, use `regex.replace(source)` instead of header prepend/full_replace
V51: ∀ templates → each stored as `<name>.txt` INI file in templates directory; list/save/load/delete via TemplateManager
V52: ∀ `header_batch_process` → scan source_dir for .esp/.esm files; parse each with EspParser::with_game; apply rules; emit header-batch-progress/comple te events
V53: ∀ `PreProcessingOpts` → key-value HashMap stored as `[PreProcessingOpts]\nkey=value` INI section

## §T Tasks

id|status|task|cites
T1|x|ESP parser with zlib decompression, GRUP nesting, XXXX extended fields|G1,C5
T2|x|Strings file load/write (null-terminated + length-prefixed)|G2
T3|x|SST v8 read/write with Delphi-compatible UTF-16LE|G3
T4|x|XML import/export with entity escaping|G4
T5|x|Heuristic search (Levenshtein + LCS + LCP)|G5
T6|x|OpenAI translation provider|G6
T7|x|Tauri IPC commands + AppState|G7
T8|x|React frontend with react-window virtual scroll|G7,C8
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
T19|x|Batch processor (file-level)|G7
T20|x|NPC map / dialog view (parent_form_id tracking via GRUP s_type, grouping)|G7,G12
T21|x|Theme system (dark/light/gray/auto, CSS variables, Zustand + localStorage + matchMedia)|G7
T22|x|UI multi-language i18n (react-i18next, 10 languages, Zh-CN default)|G7
T23|x|Regex search/replace with capture groups (replaceAll across filtered items)|G7
T24|x|Strings write-back deduplication (shared data offsets, ~17% size reduction)|G2
T25|x|Auto-backup (5-min timer, SST snapshots, rotate last 10)|G3
T26|x|Undo/Redo (stack-based, Ctrl+Z/Y, max 100 depth)|G7,C12
T27|x|ESP parse result cache (SHA-256 key, SQLite WAL, CacheIndex mtime+size fast lookup)|G7,V35,V36
T28|x|MCM (Mod Configuration Menu) translation file support (UTF-16LE/BE/UTF-8, BOM, encoding preserve)|G1
T29|x|ESP comparison engine (old/new diff with identical/modified/added/removed classification)|G1
T30|x|BA2 archive support (Fallout 4 v0x01, Starfield v0x02, FO4B v0x08)|G8
T31|x|PEX compile (in-place string table update with index preservation, binary roundtrip)|G1
T32|x|Config persistence (JSON file, theme/language/API key/proxy/esp_mode survive restart)|G7,C18
T33|x|TCSC simplified/traditional Chinese conversion (character-pair mapping, OpenCC 3960 + Delphi 2552 fallback)|G7
T34|x|Proxy settings UI (settings dialog with proxy server/port/username/password fields, integrates with build_client)|G7
T35|x|Batch TCSC conversion (convert all filtered translations at once, not just selected item)|G7
T36|x|vocabulary.txt integration (parse STRINGS=Name entries, load source+target Strings, match by str_id, enrich heuristic search candidates)|G5
T37|x|pexNoTransProc.txt filtering (parse procedure names, filter non-translatable PEX string parameters)|G1
T38|x|HiDPI support (Tauri 2.x native HiDPI + decorations/dragDrop window config)|G7
T39|x|Drag-drop extension (route BSA/BA2, PEX, FUZ file drops to correct handlers)|G7
T40|x|Source/Dest compare (compare source hash vs translation hash, tag diff/same as incomplete)|G7
T41|x|Alias integrity check (extract `<Alias=...>` tags, compare source vs translation, highlight mismatches in EditorPanel)|G7
T42|x|ESP record tree (EspField/EspRecord/EspGrup structs, full in-memory parse tree for write-back)|V29,V31
T43|x|ESP record rebuild (field buffer mutation, XXXX size prefix, zlib recompression)|V30,V31
T44|x|ESP serialization (recursive GRUP/record serialize, backup before write, roundtrip fidelity)|V29
T45|x|Localized→delocalized conversion (string ID replacement, sequential ID reassignment, .STRINGS export)|V32
T46|x|RTL/Arabic text processing (rtl_reverse, shape_arabic, deshape_arabic)|G9,V45,V46
T47|x|Translation cache recovery (JSONL journal, check_pending_cache, apply_translation_cache, discard_translation_cache)|G10,V34
T48|x|String-level batch translation (concurrent, progress events, cancel)|G7,V38
T49|x|PEX decompiler (pseudocode output via decompile_pex)|G11
T50|x|MCM compare with policy (All/NoTrans/NoTransAndPartial/PartialOnly)|G7,V39
T51|x|Data configs loading (ctdaFunc.txt, fieldSizeRef.txt, DialSubType.txt, EmoteDefinition.txt)|G7
T52|x|Dialog tree building (INFO grouped by parent DIAL, NPC_ name association)|G12,V42
T53|x|Context menu component (right-click, keyboard shortcuts)|G7
T54|x|Status bar component (file, progress, ESP/SST mode, language)|G7
T55|x|Bottom panel system (Home/Vocabulary/Heuristic/ESP Tree/PEX/Quests/Dialogs/Log tabs)|G7
T56|x|Golden diff CLI tool (cross-validate Rust output vs Delphi golden files)|G7
T57|x|Delphi-style heuristic scoring (6-dimension composite: word hash, LCS, LCP, alias proxy, dynamic threshold)|G13
T58|x|CacheIndex (mtime+size fast lookup, JSON persistence)|G7,V35
T59|x|SQLite ESP cache (WAL mode, replaces bincode)|G7,V36
T60|x|RecoveryPromptModal (UI for crash-safe translation cache recovery)|G10
T61|x|Finalize workflow (save_strings + save_sst + export_xml in single call)|V43
T62|x|BA2 DX10 texture archive support|G8
T63|x|FUZ LIP parsing (lip-sync keyframe data)|G2
T64|x|Translation providers configured flag (openaiConfigured/deeplConfigured)|G6
T65|x|BatchTranslateBar (string-level batch progress UI)|G7
T66|x|ESP record tree panel (bottom panel tab)|G7
T67|x|Single-string TCSC convert command|G7
T68|x|Save ESP command (direct write-back)|V29,V30,V31
T69|x|Finalize ESP command (SST→ESP rebuild+serialize+Strings export)|V47
T70|x|Delocalize ESP command (sequential ID reassignment)|V44
T71|x|Get ESP header command|G1
T72|x|List ESP files command|G7
T73|x|List BSA/extract BSA commands|G8
T74|x|Scan FUZ directory command|G2
T75|x|Get FUZ audio data command|G2
T76|x|Build dialog tree command|G12
T77|x|Load data configs command|G7
T78|x|Batch update translations command|G7
T79|x|Auto-backup SST command|G3
T80|x|Vocabulary panel component|G7
T81|x|ESP tree panel component|G7
T82|x|Baidu translation provider (MD5 sign: appId+text+salt+key)|G6
T83|x|Youdao translation provider (MD5 sign: appKey+text+salt+secret)|G6
T84|x|Inline MD5 implementation for API signing|G6
T85|x|Baidu/Youdao API key set/get commands + config persistence|G6,G7
T86|x|Toolbox: 7 text transformation tools (uppercase/lowercase/title case/fix alias/add header/trim)|G7
T87|x|Toolbox IPC command (toolbox_transform) with tag-aware word splitting|G7
T88|x|ToolboxDialog UI component + MenuBar integration|G7
T89|x|MS Azure Translator provider (Ocp-Apim-Subscription-Key, POST JSON array)|G6
T90|x|Google Translate provider (keyless public endpoint, nested JSON response)|G6
T91|x|App startup: restore Baidu/Youdao/Azure API keys from config.json|G6,G7
T92|x|Header Processor core engine: rule struct, INI load/save, match, apply|G7
T93|x|Header Processor IPC commands: load/list/toggle/apply/save|G7
T94|x|HeaderProcessorPanel bottom tab: load rules, toggle enable, apply to strings|G7
T95|x|Header Processor: Exclude_ keyword parsing, EDID/form_id lookup via record tree, regex matching|G7
T96|x|Rule editor enhancements: search/filter, add/delete/reorder, inline field editing|G7
T97|x|Template manager: save/load/delete named rule templates as INI files|G7
T98|x|Pre-processing options: key-value INI storage + IPC (load/list/set/delete/save) + UI editor|G7
T99|x|Batch wizard: multi-ESP header processing (scan dir + parse + apply rules) with progress events|G7
T100|x|HeaderWizardPanel bottom tab: source dir, game selector, progress bar, result summary|G7

## §P6 新增功能 (v1.1.0)

> v1.1.0 发布后新增的功能（超出原 SPEC 范围）

| id|status|task|cites|
|---|------|-----|-----|
| P6.1|x|工具箱例外词列表：Title Case 例外词配置（word_exception_list）、持久化、UI 编辑器|G7|
| P6.2|x|SST 旧版兼容：读取 v1-v7 格式、SstVersion 枚举、版本感知解析|G3|

## §B Bugs

id|date|cause|fix
B1|2026-04-20|compressed_records stat always 0 in LoadEspResponse|Fixed: track count during parsing
B2|2026-04-20|edid_hash always 0 in EspPointer (EDID hashing not implemented)|Fixed: compute FNV-1a hash from EDID string
B3|2026-04-20|source_normalized & normalized_hash always None/None (normalization not implemented)|Fixed: add normalization and hash computation in SkyString::new
B4|2026-04-20|SkyString.word_hashes always empty (tokenization not implemented)|Fixed: tokenize source and compute hashes in SkyString::new
B5|2026-04-27|OpenAI provider tests read ambient XT_TRANSLATE_API_BASE/MODEL, so local env changed default assertions|Fixed: isolate and restore XT_TRANSLATE_API_* under shared test env lock (V21)
B6|2026-04-27|parse_record_debug decremented record_count unconditionally for GRUPs, underflowing on empty/leading groups in real Skyrim data|Fixed: saturating_sub on GRUP traversal + regression test (V22)
B7|2026-05-02|PEX opcode arg_count table incorrect|Fixed: corrected opcode table + integration tests
B8|2026-05-02|ESP write-back serialization issues|Fixed: serialization roundtrip + test updates
B9|2026-05-02|Codepage handling incorrect in ESP write-back|Fixed: codepage-aware field buffer encoding
B10|2026-05-02|4 IMPLEMENTATION_SUMMARY limitations for ESP write-back|Fixed: resolved all 4 limitations
