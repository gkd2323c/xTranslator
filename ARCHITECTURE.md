# xTranslator Architecture

## Overview

xTranslator is a Rust + Tauri 2.x + React rewrite of the Delphi-based Bethesda game mod translation tool. It parses ESP/ESM files (Skyrim, Fallout 4, Starfield, etc.), extracts translatable strings, and provides a desktop UI for editing translations with dictionary (SST) and XML exchange support.

**Design goals:**
- 100% SST v8 bidirectional compatibility with Delphi xTranslator
- Cross-platform desktop app (Windows, macOS, Linux)
- Full-load + client-side virtual scroll: all strings loaded in chunks, filter/sort on frontend
- React + Vite frontend with 10-language i18n

---

## Workspace Structure

```
xTranslator/
├── Cargo.toml              # Workspace manifest (4 members)
├── crates/
│   ├── xt-core/            # Core library: ESP parser, strings, SST, XML, BSA, PEX, FUZ, heuristic search, translation API
│   ├── xt-shared/          # IPC DTOs shared between backend and frontend
│   └── xt-cli/             # CLI tool (legacy, mostly superseded by Tauri UI)
├── src-tauri/              # Tauri 2.x desktop app backend
│   ├── src/
│   │   ├── main.rs         # App setup, command registration, state management
│   │   ├── commands.rs     # IPC command implementations
│   │   └── batch.rs        # Batch processor
│   └── tauri.conf.json     # Tauri config (dev/build commands, window settings)
└── ui/                     # React + Vite frontend (NOT a Cargo workspace member)
    ├── src/
    │   ├── main.tsx        # Entry point
    │   ├── App.tsx         # Root layout
    │   ├── api/strings.ts  # Frontend API: DTO types + Tauri invoke wrappers
    │   ├── stores/appStore.ts  # Zustand state management
    │   ├── i18n.ts         # react-i18next initialization (10 languages)
    │   ├── locales/        # Language files (zh-CN, en, de, es, fr, ja, ko, pl, pt, ru)
    │   └── components/     # MenuBar, SidePanel, StringTable, EditorPanel, BatchPanel, BsaBrowser, PexPanel, FuzPanel, DialogView
    └── package.json
├── Data/                   # Shared game definitions loaded by xt-core
└── legacy/original-delphi/ # Original Delphi project kept as read-only reference
```

### Member Roles

| Member | Role | Key Files |
|--------|------|-----------|
| `xt-core` | Domain logic: ESP parsing + record tree + write-back, Bethesda strings formats, SST v8 read/write, XML import/export, BSA v0x68/v0x69 + BA2 General archive support, PEX script parsing + decompile + compile, FUZ audio, TCSC conversion, config persistence, heuristic similarity search, translation API providers (with CRLF protection, API config parsing, proxy builder), ESP cache, ESP compare, MCM parsing, data configs | `src/lib.rs`, `src/esp/parser.rs`, `src/esp/record_tree.rs`, `src/esp/compare.rs`, `src/strings/`, `src/sst/`, `src/xml/`, `src/bsa/`, `src/ba2/`, `src/pex/`, `src/fuz/`, `src/tcsc.rs`, `src/config.rs`, `src/heuristic/`, `src/translation_api/`, `src/cache.rs`, `src/mcm.rs`, `src/data_config.rs` |
| `xt-shared` | Serializable DTOs for IPC. Source of truth for data shapes. | `src/dto.rs` |
| `xt-cli` | Legacy CLI for testing core functionality without UI. | `src/main.rs` |
| `src-tauri` | Tauri backend: holds `AppState`, exposes commands to frontend. | `src/main.rs`, `src/commands.rs` |
| `ui` | React frontend: file dialogs via Tauri plugin, table with virtual scrolling, editor panel. | `src/api/strings.ts`, `src/stores/appStore.ts`, `src/components/` |

---

## Data Flow

### Loading a File

```
User selects ESP → MenuBar.tsx
  → invoke("load_esp", { espPath, stringsDir, language, game })
  → src-tauri/src/commands.rs::load_esp()
    → spawn_blocking: CPU-intensive ESP parsing
      → Check EsmCache (SHA-256 of ESP → {hash}.cache in %LOCALAPPDATA%/xTranslator/cache/)
      → Cache hit: deserialize bincode blob → return instantly (cached=true, parse_time_ms=0)
      → Cache miss: full parse
        → xt-core::EspParser::with_game() → parse() → decompress records → extract strings
        → xt-core::StringsFiles::load_from_dir_with_language()
          → First: try standalone .STRINGS/.DLSTRINGS/.ILSTRINGS files
          → Fallback: scan .bsa archives in ESP directory, extract strings/ folder via BSAhash64
        → Store result in cache (bincode serialized, max 50 entries, prune oldest)
    → Store Vec<SkyString> in AppState.strings
  → Emit "esp-load-progress" events (stage: parsing / finalizing / cached for cache hits)
  → Return LoadEspResponse { total, compressed_records, strings_loaded, parse_time_ms, record_counts, cached }
  → appStore.setEspLoaded() → SidePanel shows stats
```

### Querying / Editing (Virtual Scroll)

```
ESP loaded → StringTable.tsx useEffect
  → invoke("get_strings_chunk") × N batches (~10K items/batch, ~2MB JSON)
  → src-tauri/src/commands.rs::get_strings_chunk()
    → Map SkyString → SkyStringDTO chunks
  → appStore.setAllItems(allItems) → client-side filter + sort
  → react-window List renders visible rows only

User scrolls → react-window virtualizes rows (no IPC)
User filters/sorts → appStore applyFilterAndSort() on allItems (no IPC, <10ms)
User selects row → appStore.setSelectedById(id) → find in allItems by id

User edits translation → EditorPanel.tsx
  → invoke("update_translation", { id, translation })
  → commands.rs::update_translation()
    → Find by id in Vec (NOT by index)
    → Update SkyString.translation + params flags
  → appStore.updateItemTranslation(id, translation) (optimistic UI update)
```

### Saving / Exporting

```
User clicks Save SST → invoke("save_sst", { sstPath, masters? })
  → SstDictionary::from_entries() → save_to_file()

User clicks Export XML → invoke("export_xml", { file_path, addon, source_lang, dest_lang, version })
  → Filter strings with non-empty translation
  → xt-core::xml::write_xml_export() → write UTF-8 XML

User clicks Import XML → invoke("import_xml", { xml_path })
  → parse_xml_file() → import_xml_to_sky_strings()
  → Match by str_id + record_sig + field_sig
  → Update translation + set TRANSLATED flag
  → Return XmlImportResponse { matched, unmatched, total, updated_ids }
  → appStore.loadAllStrings() (reload full dataset since backend mutated)
```

---

## Key Design Decisions

### 1. Full-Load + Client-Side Virtual Scroll

ESP 加载后前端通过 `get_strings_chunk` 分块拉取全量数据（每批 10K 条 ~2MB JSON，76K 条约 8 批），之后筛选、排序、滚动全部在客户端完成（零延迟，<10ms）。`query_strings_command` 保留作为降级方案。

**Rationale**: 分块避免一次性传输 15-20MB JSON 超出 WebView2 `postMessage` 限制。客户端 filter+sort 远快于 46ms IPC 往返。消除翻页中断体验，对标 Delphi 原版 VirtualTreeView。

### 2. Update by ID, Not Index

`update_translation` takes a `u32 id` and performs a linear scan in the Vec. Frontend uses `selectedId` (not array index) — indices become invalid after filtering/sorting.

### 3. DTO Source of Truth

`crates/xt-shared/src/dto.rs` defines the canonical Rust structs. `ui/src/api/strings.ts` mirrors them in TypeScript. When adding fields, both files must be kept in sync.

### 4. Data Refresh After Mutation

SST 加载 / XML 导入 → backend mutates `AppState.strings` → frontend calls `loadAllStrings()` to reload full dataset. Single-item updates use optimistic local update (`updateItemTranslation(id, text)`) with zero IPC.

### 5. BSA Strings Fallback

If standalone Strings files are not found in `Data/Strings/`, the loader scans all `.bsa` files in the ESP directory and extracts matching files from the `strings/` folder inside the archive. This matches Bethesda's loading behavior where SSE stores strings inside `Skyrim - Interface.bsa`.

### 6. Tauri Dev Workaround

`tauri.conf.json` sets `beforeDevCommand: "echo ok"` because `cd ui && npm run dev` fails in Windows PowerShell. Use `dev.ps1` for one-click startup, or run two terminals:
1. `cd ui && npm run dev` (Vite on :5173)
2. `cargo run -p xtranslator-tauri` (connects to :5173)

Production builds use `beforeBuildCommand: "cd ui && npm run build"` which works correctly.

### 7. ESP Parse Result Caching

ESP 解析结果通过基于文件 SHA-256 的内容寻址缓存到本地磁盘（`%LOCALAPPDATA%/xTranslator/cache/`）。同一文件的后续加载直接反序列化 bincode blob，跳过 STRINGS 加载和 ESP 解析，实现秒开。缓存文件以 `{sha256}.cache` 命名，自动清理超出 50 条上限的最旧条目。

Format: `CachePayload { version: u32, strings: Vec<SkyString>, compressed_records: u32, strings_loaded: u8 }` — versioned for forward compatibility.

---

## Module Details

### xt-core

| Module | Responsibility |
|--------|----------------|
| `esp::parser` | ESP/ESM binary parser: record headers, GRUP nesting, compressed record decompression (zlib), subrecord extraction, codepage-aware string decoding |
| `esp::compare` | ESP comparison engine: compare-only extraction, duplicate-field occurrence tracking, master-normalized FormID matching, lightweight compare cache, identical/modified/added/removed classification |
| `strings` | Bethesda `.STRINGS` (null-terminated), `.DLSTRINGS`/`.ILSTRINGS` (4-byte length prefix) read/write. Codepage fallback table (932/936/949/950/1250-1257) |
| `bsa` | BSA v0x68/v0x69 archive parser and file extraction. SSE uses LZ4, Skyrim uses zlib. Supports `BSAhash64` lookup, `list_all_files`, `extract_file` |
| `ba2` | BA2 (Bethesda Archive 2) parser for Fallout 4/Starfield. GNRL type support, file listing and extraction |
| `pex` | PEX (Papyrus) script parser + compiler: string table extraction, in-place translation update with index preservation |
| `mcm` | MCM (Mod Configuration Menu) translation file parser: UTF-16LE/UTF-8/ANSI encoding detection, BOM handling, key-value extraction, save with original encoding+line endings |
| `fuz` | FUZ audio container parser: FuzHeader + WAV extraction, Sound/Voice/ directory scanning, RESP/INFO association |
| `tcsc` | Traditional/Simplified Chinese conversion: OpenCC dictionary (primary, 3960 pairs) + Delphi Charset_SCTC.txt (fallback, 2552 pairs), compile-time embedded, `to_simplified()` / `to_traditional()` |
| `config` | App configuration persistence: `AppConfig` (theme, language, API keys, proxy), JSON load/save, merge-only updates |
| `sst::v8` | SST v8 dictionary format: read/write with Delphi-compatible UTF-16LE encoding, FNV-1a hashing, bidirectional roundtrip |
| `xml` | Delphi xTranslator XML export/import: `parse_xml_export`, `write_xml_export`, `import_xml_to_sky_strings` |
| `heuristic` | Similarity search for translation suggestions: Levenshtein distance, longest common substring (LCS), longest common prefix (LCP) |
| `normalization` | String normalization (case-folding, punctuation stripping, whitespace compression) for heuristic search and dictionary matching |
| `cache` | ESP parse result cache (SHA-256 keyed, bincode blob, auto-prune oldest) |
| `translation_api` | Translation provider trait; OpenAI + DeepL implementations. API config parsing (ApiTranslator.txt), language code resolution, CRLF protection (`<L_F>` tag), HTTP proxy builder (not yet wired). Supports API key via env var, runtime, or config persistence, provider switching |
| `types` | Core types: `SkyString`, `EspPointer`, `SkyStringParams`, `GameId` |

### src-tauri

| File | Responsibility |
|------|----------------|
| `main.rs` | Tauri app builder: plugin initialization (`shell`, `dialog`), `AppState` management, command handler registration |
| `commands.rs` | IPC command implementations: `load_esp`, `load_sst`, `save_sst`, `update_translation`, `get_strings_chunk`, `query_strings_command`, `get_stats`, `heuristic_search`, `translate_string`, `set_api_key`, `export_xml`, `import_xml`, `save_strings`, `list_bsa_files`, `extract_bsa_file`, `parse_pex_strings`, `load_fuz_mapping`, `build_dialog_tree`, `start_batch_translate`, `start_batch_export`, `cancel_batch_job`, `load_config`, `save_config`, `get_api_config` |
| `batch.rs` | Batch processor: sequential ESP file processing, cooperative cancellation, Tauri event emission |

### ui

| File | Responsibility |
|------|----------------|
| `api/strings.ts` | TypeScript DTO interfaces + Tauri invoke wrappers for every backend command |
| `stores/appStore.ts` | Zustand store: holds items, filter/sort, selection, file info, theme, language, batch state |
| `i18n.ts` | react-i18next initialization: 10 language locales, language detection, MenuBar integration |
| `components/MenuBar.tsx` | Load ESP/SST, Save SST/Strings, Export/Import XML, Reset, language selector, theme switcher, batch panel toggle |
| `components/SidePanel.tsx` | Stats display: total/translated/incomplete/locked counts, record type filter list, load progress |
| `components/StringTable.tsx` | Virtual scroll list (react-window FixedSizeList): 76K+ strings seamless scroll, client-side filter/sort, status filter buttons, audio playback column |
| `components/EditorPanel.tsx` | Translation editor: source text, textarea, Save (Ctrl+Enter), heuristic search, translate API, API Key dialog, status badge |
| `components/BatchPanel.tsx` | Batch file list, game/language detection, translate/export, progress tracking, cancellation |
| `components/BsaBrowser.tsx` | BSA archive browser: folder tree, file list, preview, extract single/batch |
| `components/PexPanel.tsx` | PEX script viewer: object tree, translatable strings list, XML export |
| `components/FuzPanel.tsx` | FUZ audio browser: file scan, RESP/INFO association, WAV playback |
| `components/DialogView.tsx` | Dialog tree: QUST→DIAL→INFO grouping, NPC view, inline editing |
| `App.tsx` | Root layout + global loading overlay (locks UI during critical operations) |

---

## Data Formats

### Bethesda Strings Files
- `.STRINGS` — null-terminated UTF-8 (or codepage) strings, 4-byte ID prefix
- `.DLSTRINGS` / `.ILSTRINGS` — 4-byte length-prefixed strings

### BSA Archives (v0x68 / v0x69)
- Folder/file lookup uses `BSAhash64` algorithm (must match Delphi exactly)
- SSE (v0x69): LZ4 compression, 64-bit offsets, folder names include null terminator
- Skyrim (v0x68): zlib compression, 32-bit offsets
- Strings files live inside `strings/` folder within BSA (e.g. `Skyrim - Interface.bsa`)

### ESP Compressed Records
Format: `[4-byte decompressedSize LE] + [zlib data]`. Must decompress before parsing subrecords.

### ESP dsize Semantics
- Record `dsize` **excludes** the 16-byte record header
- GRUP `dsize` **includes** its own 24-byte header (GenericHeader 8B + GrupHeader 16B)

### SST v8
- Delphi-compatible dictionary format
- UTF-16LE string encoding
- FNV-1a hash (low byte of UTF-16 code units)
- `EspPointer` is 24 bytes (little-endian)

### XML Export
Delphi xTranslator compatible:
```xml
<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<SSTXMLRessources>
  <Params>
    <Addon>Skyrim</Addon>
    <Source>english</Source>
    <Dest>chinese</Dest>
    <Version>2</Version>
  </Params>
  <Content>
    <String List="0" sID="000001">
      <EDID>EditorID</EDID>
      <REC id="0" idMax="0">INFO:NNAM</REC>
      <Source>Hello</Source>
      <Dest>你好</Dest>
    </String>
  </Content>
</SSTXMLRessources>
```

---

## Build & Test Commands

```bash
# Full backend build
cargo build -p xtranslator-tauri

# Core library unit tests (181 tests)
cargo test -p xt-core --lib

# End-to-end tests (requires Skyrim SE at D:\SteamLibrary\...)
cargo test -p xt-core --test e2e_real_data

# TypeScript type check
cd ui && npx tsc --noEmit

# One-click dev startup (PowerShell)
.\dev.ps1
```

---

## Adding a New IPC Command

1. Add DTOs to `crates/xt-shared/src/dto.rs` (derive `Serialize, Deserialize`)
2. Add TypeScript interfaces to `ui/src/api/strings.ts`
3. Implement command in `src-tauri/src/commands.rs`
4. Register in `src-tauri/src/main.rs` via `generate_handler!`
5. Export frontend wrapper from `ui/src/api/strings.ts`
6. Build: `cargo test -p xt-core --lib` + `npx tsc --noEmit`
