# xTranslator - Rust Rewrite

[![Version: 1.2.0](https://img.shields.io/badge/Version-1.2.0-blue.svg)](RELEASE.md)
[![License: MPL-2.0](https://img.shields.io/badge/License-MPL--2.0-brightgreen.svg)](LICENSE)
[![Tests](https://img.shields.io/badge/Tests-359_+_64_E2E_passing-brightgreen.svg)](CHANGELOG.md)

[中文版](README_zh-CN.md)

A modern Rust-based translator for Bethesda game mods (Skyrim, Skyrim SE, Fallout 4, Starfield). This is a complete rewrite of the original Delphi xTranslator tool, featuring a Tauri 2.x desktop UI with React frontend.

## Features

### Core Functionality
- **ESP/ESM Parsing + Write-Back**: Load, parse, and directly edit Bethesda ESP/ESM plugin files with full record tree support (XXXX field management, zlib recompression, automatic backup before write)
- **Strings Files**: Support for `.STRINGS`, `.DLSTRINGS`, `.ILSTRINGS` formats (with deduplication, ~17% size reduction)
- **BSA Archive Support**: Extract and load strings from Bethesda archive files (.bsa, .ba2)
- **XML Import/Export**: Compatible with Delphi xTranslator XML format (UTF-8, entity escaping)
- **SST Dictionaries**: Full v8 bidirectional compatibility with Delphi xTranslator (UTF-16LE, FNV-1a, 24B EspPointer)
- **Heuristic Search**: Find similar translated strings using Levenshtein distance, LCS, and LCP algorithms
- **Codepage Fallback**: UTF-8 primary with Windows codepage fallback (932/936/949/950/1250-1257)
- **Text Normalization**: String normalization (NFKC) and tokenization for heuristic search and translation consistency
- **TCSC Conversion**: Traditional/Simplified Chinese conversion with OpenCC dictionary (3960 pairs) + Delphi fallback (2552 pairs)
- **Config Persistence**: JSON config file survives restart (theme, language, API keys, proxy)
- **API Config**: Parse Delphi `ApiTranslator.txt` for provider metadata, language code resolution, and query templates
- **CRLF Protection**: `<L_F>` tag protect/restore cycle for translation API calls

### Translation APIs
- **OpenAI Compatible**: OpenAI, DeepSeek, and other Chat Completions API providers (supports prompt templates)
- **DeepL**: Free and Pro API support (auto-detected from API key)
- **Baidu**: Chinese translation via Baidu Translate API (AppId + Key)
- **Youdao**: Chinese translation via Youdao Translate API (AppKey + SecretKey)
- **Azure**: Microsoft Translator API (key-based auth)
- **Google**: Google Cloud Translation API (key-based auth)

### Advanced Features
- **GMST:DATA Filtering**: Automatic detection of translatable vs numeric GMST records
- **ESP Compare**: Lightweight compare-only extractor with normalized FormID + field-occurrence matching
- **Record Type Filtering**: Filter strings by record type (INFO, QUST, etc.)
- **Status Filtering**: Filter by translation status (translated/incomplete/locked)
- **Virtual Rendering**: Efficient handling of large string lists (76K+ items)
- **Chunked Loading**: Batched data loading (25K items per batch, ~2MB JSON, concurrency 3)
- **Regex Search/Replace**: Full regex with capture groups ($1/$2), replace-all across filtered items
- **Spell Check**: Hunspell-based spell check with tag-aware word splitting, suggestions, and ignore list
- **Theme System**: Obsidian/Slate/Light/Auto themes with CSS variables + localStorage persistence
- **Undo/Redo**: Stack-based (max 100), Ctrl+Z/Y, IPC-synced
- **Auto-Backup**: 5-min SST snapshots, rotate last 10
- **Batch Processor**: Multi-file sequential ESP translate/export with progress events and cancellation
- **BSA/BA2 Archive Browser**: Browse, preview, and extract files from BSA v0x68/v0x69 and BA2 General archives
- **PEX Script Translation**: Parse Papyrus scripts, extract translatable strings, and write updated string tables while preserving binary structure
- **FUZ Audio Mapping**: Map dialog strings to WAV audio with playback
- **VMAD Fragment Handling**: Extract and write back VMAD script strings for PERK/PACK/SCEN/INFO/QUST records with fragment preservation
- **Heuristic Search (Delphi Scoring)**: Word-level hash matching, LCS, LCP, alias proxy penalty — aligned with original Delphi scoring algorithm
- **NPC/Dialog View**: Dialog tree grouped by QUST→DIAL→INFO with NPC association
- **Multi-Language UI**: 10 languages (zh-CN, en, de, es, fr, ja, ko, pl, pt, ru)
- **Toolbox**: 7 text transformation tools + exception word list (case conversion, alias fixing, header adding, trimming)

## Project Structure

```
xTranslator/
├── crates/
│   ├── xt-core/         # Core library: ESP parser + record tree + write-back, strings, SST, XML, BSA, heuristic search
│   ├── xt-shared/       # Shared DTOs for IPC between backend and frontend
│   └── xt-cli/          # CLI tool (legacy, superseded by Tauri UI)
├── src-tauri/           # Tauri 2.x desktop app backend
├── ui/                  # React + Vite frontend
├── Data/                # Shared game definitions used by the rewrite
├── docs/                # Documentation
└── legacy/original-delphi/ # Original Delphi project kept as reference
```

## Project Status

**v1.0.0 stable release is out!** 🎉

The rewrite now covers the primary desktop translation workflows and `SPEC.md` tracks **100 completed tasks** covering parsing, editing, ESP write-back, compare tools, archive support, translation APIs, config persistence, and language tooling.

All core functionality is implemented and tested:
- ✅ ESP parsing with record tree + write-back (T42-T45)
- ✅ Strings read/write with deduplication (~17% size reduction)
- ✅ SST v8 bidirectional compatibility
- ✅ XML import/export (Delphi-compatible)
- ✅ BSA v0x68/v0x69 + BA2 General archive support
- ✅ PEX script parsing + string extraction + write-back
- ✅ FUZ audio mapping
- ✅ MCM translation file support
- ✅ ESP comparison engine
- ✅ Translation APIs (6 providers: OpenAI, DeepL, Baidu, Youdao, Azure, Google)
- ✅ Heuristic search (Levenshtein + LCS + LCP)
- ✅ Config persistence (JSON + proxy settings)
- ✅ TCSC 繁简转换 (OpenCC + Delphi fallback)
- ✅ Batch processor with cancellation
- ✅ Auto-backup (5-min SST snapshots)
- ✅ Undo/Redo (stack-based, max 100)
- ✅ Virtual scrolling (react-window v2, 76K+ items)
- ✅ 10-language i18n UI
- ✅ Theme system (Obsidian/Slate/Light/Auto)

**Skyrim SE validation hardened** — golden snapshot locked (75,754 strings, 118 top GRUPs), regression script available. Delphi cross-check still pending (requires Delphi 1.6.0 runtime).

Recent additions in v1.2.0:
- FUZ LIP keyframe visualization — colored bar chart preview with shape legend and timeline
- Skyrim SE validation hardening — L1/L2/L3 validation + golden snapshot + regression script
- Toolbox exception words list (P6) — Title Case exception word editor with persistence
- SST v1-v8 version enum — auto-detect magic for legacy format support
- E2E test suite fully green — 64 tests passing (12.4s)
- Windows env var whitespace fix — E2E mock alias loading on Windows
- Advanced Search panel (DP-04) — per-field regex with presets, integrated into the filter pipeline
- BatchProcessor command script engine (DP-05) — Delphi-equivalent command set for multi-file automation
- BSA/BA2 injection replacement (DP-06) — safe atomic in-place archive replacement
- Localized/Hybrid strings loading strategies (DP-07) — source-tracked load modes
- XML Export options + EDID metadata completeness (DP-08/DP-09) — Delphi-parity XML output
- DEF_UI component generator (DP-10) — generate DEF_UI components from translation data
- Codepage manual selection/override (DP-11) — `ChooseCpDialog` + runtime reload
- RTL live preview and Arabic/Hebrew shaping (DP-12)
- FUZ voice mapping and rename tool (DP-13)
- AddId FormID batch offset tool (DP-14) — bulk FormID shift with full-range filtering

## Documentation

Start with [`docs/README.md`](docs/README.md) for the organized documentation map. The most-used project references are:

- [`SPEC.md`](SPEC.md) — canonical goals, constraints, interfaces, invariants, and tasks
- [`ARCHITECTURE.md`](ARCHITECTURE.md) — implementation architecture and IPC/data-flow notes
- [`docs/feature_comparison.md`](docs/feature_comparison.md) — Delphi parity and remaining gaps
- [`docs/release_qa.md`](docs/release_qa.md) — reusable release QA checklist

## Build & Test

### Prerequisites
- Rust 1.70+ (edition 2021)
- Node.js 18+ and npm
- Tauri CLI: `cargo install tauri-cli`

### Commands

```bash
# Full backend build
cargo build -p xtranslator-tauri

# Core library tests (no external deps)
cargo test -p xt-core --lib

# Run a single test
cargo test -p xt-core --lib test_name_here

# E2E tests (requires Skyrim SE installed)
cargo test -p xt-core --test e2e_real_data

# TypeScript check
cd ui && npx tsc --noEmit

# Frontend dev server
cd ui && npm run dev

# Full Tauri app (run Vite dev server first)
cargo run -p xtranslator-tauri
```

### One-Click Development Startup

```powershell
# From project root - starts Vite + Tauri automatically
.\dev.ps1
```

This script:
1. Kills any stale `node` / `xtranslator-tauri` processes
2. Starts Vite dev server (`:5173`) in a background job
3. Waits for port 5173 to be ready (max 30s)
4. Launches `cargo run -p xtranslator-tauri`
5. Cleans up the background job when Tauri exits

## Architecture

### Backend-Frontend IPC
- **DTO Source of Truth**: `crates/xt-shared/src/dto.rs` defines Rust structs; `ui/src/api/strings.ts` mirrors them in TypeScript
- **Data Strategy**: ESP loads → frontend fetches chunks via `get_strings_chunk` (25K items/batch, concurrency 3, ~2MB JSON) → client-side filter/sort/scroll
- **Update by ID**: `update_translation` takes a `u32 id` and looks up the string in the Vec. Frontend uses `selectedId` (not array index) — indices become invalid after filtering/sorting
- **Data Refresh**: SST load / XML import → backend mutates `AppState.strings` → frontend re-calls `loadAllStrings()` to refresh chunks. Single translation update → frontend local `updateItemTranslation(id, text)` (zero IPC)

### Data Formats (Bethesda)
- **Strings Files**: `.STRINGS` = null-terminated; `.DLSTRINGS` / `.ILSTRINGS` = 4-byte length prefix
- **ESP Compressed Records**: `[4-byte decompressedSize LE] + [zlib data]`. Decompress before parsing subrecords
- **ESP dsize Semantics**: Record `dsize` **excludes** the 16B record header; GRUP `dsize` **includes** its own 24B header (GenericHeader 8B + GrupHeader 16B)
- **Codepage Fallback**: UTF-8 primary; on decode failure, fall back to Windows codepage via `CodepageTable` (932/936/949/950/1250-1257)

### Status Values
- `"translated"` — has non-empty translation
- `"incomplete"` — partial/work-in-progress
- `"locked"` — non-translatable (e.g., GMST numeric DATA fields)

### GMST:DATA Filtering
GMST records contain a `DATA` field that can be either:
- **Numeric** (int/float) — filtered out, not translatable
- **String reference** (when EDID starts with `s`) — kept and resolved via Strings files

Filtering logic: during ESP parsing, if a GMST record's `EDID` field starts with `'s'`, its `DATA` field is treated as a string ID and looked up in `.STRINGS`. Otherwise (EDID starts with `f`/`i`/`b` or missing), the DATA field is assumed numeric and skipped.

### Heuristic Search
- Only searches strings already marked `translated`
- Uses Levenshtein distance + LCS + LCP
- Default threshold: 0.5 similarity, max 5 results
- Backend: `crates/xt-core/src/heuristic/mod.rs`; IPC: `heuristic_search` command

### XML Import/Export
- **Export**: `export_xml` command → `write_xml_export()` → Delphi-compatible UTF-8 XML with entity escaping
- **Import**: `import_xml` command → `parse_xml_file()` → `import_xml_to_sky_strings()` — matches by `(str_id, record_sig, field_sig)` triple. Returns `XmlImportResponse { matched, unmatched, total, updated_ids }`

### ESP Write-Back
- **Record Tree**: Full in-memory parse tree (`EspField` → `EspRecord` → `EspGrup` → `EspFile`) built during ESP parsing
- **Write Commands**:
  - `save_esp`: Apply translations → rebuild records (XXXX management, zlib recompression) → serialize → save with optional backup
  - `finalize_esp`: Apply SST translations → rebuild → serialize → export .STRINGS/.DLSTRINGS/.ILSTRINGS
  - `delocalize_esp`: Convert localized ESP to delocalized format (sequential IDs from 1)
- **Backup**: `.backup.<timestamp>` created before any ESP write (configurable)
- **XXXX Handling**: Automatic insertion/removal when field size crosses 65535 boundary
- **Module**: `crates/xt-core/src/esp/record_tree.rs`, `src/esp/parser.rs`

## Known Limitations

- **E2E Tests**: Require Skyrim SE installed at `D:\SteamLibrary\steamapps\common\Skyrim Special Edition\Data\Skyrim.esm`
- **Record Defs Loading**: Best-effort; if `Data/<Game>/record_defs` is missing, parser falls back to generic parsing
- **BA2 Archives**: General archives are supported; texture-specific BA2 variants and archive injection are intentionally out of scope for now

## Credits

Original xTranslator by McGuffin and contributors. This Rust rewrite preserves the functionality and spirit of the original Delphi tool while modernizing the codebase and UI.

### Third-Party Components (Original)
- SynEdit: https://github.com/SynEdit/SynEdit
- VirtualStringTree: Mike Lischke (www.soft-gems.net)
- Diff: http://www.angusj.com/delphi/textdiff.html (Angus Johnson)
- HtmlViewer: https://github.com/BerndGabriel/HtmlViewer
- ZLibex: http://www.dellapasqua.com and xEdit
- LZ4: https://github.com/atelierw/LZ4Delphi and xEdit
- OmniXML: https://github.com/mremec/omnixml
- PCRE Regex: http://www.regular-expressions.info/delphi.html
- Hunspell: https://github.com/hunspell/hunspell

### Translation API References
- DeepL: https://www.deepl.com/translator
- OpenAI: https://api.openai.com/

## License

MPL-2.0 License. See [LICENSE](LICENSE) for details.
