# xTranslator Architecture

## Overview

xTranslator is a Rust + Tauri 2.x + React rewrite of the Delphi-based Bethesda game mod translation tool. It parses ESP/ESM files (Skyrim, Fallout 4, Starfield, etc.), extracts translatable strings, and provides a desktop UI for editing translations with dictionary (SST) and XML exchange support.

**Design goals:**
- 100% SST v8 bidirectional compatibility with Delphi xTranslator
- Cross-platform desktop app (Windows, macOS, Linux)
- Backend-only pagination: frontend never receives more than 100 items at once
- All filter/sort/paginate logic lives in Rust

---

## Workspace Structure

```
xTranslator/
├── Cargo.toml              # Workspace manifest (4 members)
├── crates/
│   ├── xt-core/            # Core library: ESP parser, strings, SST, XML, heuristic search, translation API
│   ├── xt-shared/          # IPC DTOs shared between backend and frontend
│   └── xt-cli/             # CLI tool (legacy, mostly superseded by Tauri UI)
├── src-tauri/              # Tauri 2.x desktop app backend
│   ├── src/
│   │   ├── main.rs         # App setup, command registration, state management
│   │   └── commands.rs     # IPC command implementations
│   └── tauri.conf.json     # Tauri config (dev/build commands, window settings)
└── ui/                     # React + Vite frontend (NOT a Cargo workspace member)
    ├── src/
    │   ├── main.tsx        # Entry point
    │   ├── App.tsx         # Root layout
    │   ├── api/strings.ts  # Frontend API: DTO types + Tauri invoke wrappers
    │   ├── stores/appStore.ts  # Zustand state management
    │   └── components/     # MenuBar, SidePanel, StringTable, EditorPanel
    └── package.json
```

### Member Roles

| Member | Role | Key Files |
|--------|------|-----------|
| `xt-core` | Domain logic: ESP parsing, Bethesda strings formats, SST v8 read/write, XML import/export, heuristic similarity search, translation API providers | `src/lib.rs`, `src/esp/`, `src/sst/`, `src/xml/`, `src/heuristic/`, `src/translation_api/` |
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
      → xt-core::EspParser::with_game() → parse() → decompress records → extract strings
      → xt-core::StringsFiles::load_from_dir_with_language() → load .STRINGS/.DLSTRINGS/.ILSTRINGS
    → Store Vec<SkyString> in AppState.strings
  → Return LoadEspResponse { total, compressed_records, strings_loaded, parse_time_ms, record_counts }
  → appStore.setEspLoaded() → SidePanel shows stats
```

### Querying / Editing (Virtual Scroll)

```
ESP loaded → StringTable.tsx useEffect
  → invoke("get_all_strings")
  → src-tauri/src/commands.rs::get_all_strings()
    → Map all SkyString → SkyStringDTO (Vec of 76K items)
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

ESP 加载后前端通过 `get_all_strings` 一次性拉取全部 DTOs（~15-20MB for 76K items）。之后筛选、排序、滚动全部在客户端完成（零延迟，<10ms）。`query_strings_command` 保留作为降级方案。

**Rationale**: 76K DTOs 的序列化 + 传输约 200-500ms，可接受。客户端 filter+sort 远快于 46ms IPC 往返。消除 760 页翻页的中断体验，对标 Delphi 原版 VirtualTreeView。

**⚠️ Risk**: 15-20MB JSON may exceed WebView2 `postMessage` limits on Windows. Monitor for runtime failures.

### 2. Update by ID, Not Index

`update_translation` takes a `u32 id` and performs a linear scan in the Vec. Frontend uses `selectedId` (not array index) — indices become invalid after filtering/sorting.

### 3. DTO Source of Truth

`crates/xt-shared/src/dto.rs` defines the canonical Rust structs. `ui/src/api/strings.ts` mirrors them in TypeScript. When adding fields, both files must be kept in sync.

### 4. Data Refresh After Mutation

SST 加载 / XML 导入 → backend mutates `AppState.strings` → frontend calls `loadAllStrings()` to reload full dataset. Single-item updates use optimistic local update (`updateItemTranslation(id, text)`) with zero IPC.

### 5. Tauri Dev Workaround

`tauri.conf.json` sets `beforeDevCommand: "echo ok"` because `cd ui && npm run dev` fails in Windows PowerShell. Development requires two terminals:
1. `cd ui && npm run dev` (Vite on :5173)
2. `cargo run -p xtranslator-tauri` (connects to :5173)

Production builds use `beforeBuildCommand: "cd ui && npm run build"` which works correctly.

---

## Module Details

### xt-core

| Module | Responsibility |
|--------|----------------|
| `esp::parser` | ESP/ESM binary parser: record headers, GRUP nesting, compressed record decompression (zlib), subrecord extraction, codepage-aware string decoding |
| `strings` | Bethesda `.STRINGS` (null-terminated), `.DLSTRINGS`/`.ILSTRINGS` (4-byte length prefix) read/write. Codepage fallback table (932/936/949/950/1250-1257) |
| `sst::v8` | SST v8 dictionary format: read/write with Delphi-compatible UTF-16LE encoding, FNV-1a hashing, bidirectional roundtrip |
| `xml` | Delphi xTranslator XML export/import: `parse_xml_export`, `write_xml_export`, `import_xml_to_sky_strings` |
| `heuristic` | Similarity search for translation suggestions: Levenshtein distance, longest common substring (LCS), longest common prefix (LCP) |
| `translation_api` | Translation provider trait + OpenAI provider implementation. Supports API key via env var or runtime setting |
| `types` | Core types: `SkyString`, `EspPointer`, `SkyStringParams`, `GameId` |

### src-tauri

| File | Responsibility |
|------|----------------|
| `main.rs` | Tauri app builder: plugin initialization (`shell`, `dialog`), `AppState` management, command handler registration |
| `commands.rs` | IPC command implementations: `load_esp`, `load_sst`, `save_sst`, `update_translation`, `query_strings_command`, `get_stats`, `heuristic_search`, `translate_string`, `set_api_key`, `export_xml`, `import_xml` |

### ui

| File | Responsibility |
|------|----------------|
| `api/strings.ts` | TypeScript DTO interfaces + Tauri invoke wrappers for every backend command |
| `stores/appStore.ts` | Zustand store: holds items, pagination state, filter/sort, selection, file info |
| `components/MenuBar.tsx` | Load ESP/SST, Save SST, Export/Import XML, Reset |
| `components/SidePanel.tsx` | Stats display: total/translated/incomplete/locked counts |
| `components/StringTable.tsx` | Virtual scroll list (react-window FixedSizeList): 76K+ strings seamless scroll, client-side filter/sort, ResizeObserver adaptive height |
| `components/EditorPanel.tsx` | Translation editor: source text, textarea, Save (Ctrl+Enter), heuristic search, translate API, API Key dialog, status badge |

---

## Data Formats

### Bethesda Strings Files
- `.STRINGS` — null-terminated UTF-8 (or codepage) strings, 4-byte ID prefix
- `.DLSTRINGS` / `.ILSTRINGS` — 4-byte length-prefixed strings

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

# Core library unit tests (no external deps)
cargo test -p xt-core --lib

# End-to-end tests (requires Skyrim SE at D:\SteamLibrary\...)
cargo test -p xt-core --test e2e_real_data

# TypeScript type check
cd ui && npx tsc --noEmit

# Frontend dev server (run separately — see Dev Workaround above)
cd ui && npm run dev
```

---

## Adding a New IPC Command

1. Add DTOs to `crates/xt-shared/src/dto.rs` (derive `Serialize, Deserialize`)
2. Add TypeScript interfaces to `ui/src/api/strings.ts`
3. Implement command in `src-tauri/src/commands.rs`
4. Register in `src-tauri/src/main.rs` via `generate_handler!`
5. Export frontend wrapper from `ui/src/api/strings.ts`
6. Build: `cargo test -p xt-core --lib` + `npx tsc --noEmit`
