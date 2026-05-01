# xTranslator - Rust Rewrite

[![License: GPL-3.0](https://img.shields.io/badge/License-GPL--3.0-blue.svg)](LICENSE)

[中文版](README_zh-CN.md)

A modern Rust-based translator for Bethesda game mods (Skyrim, Skyrim SE, Fallout 4, Starfield). This is a complete rewrite of the original Delphi xTranslator tool, featuring a Tauri-based desktop UI with React frontend.

## Features

### Core Functionality
- **ESP/ESM Parsing**: Load and parse Bethesda ESP/ESM plugin files
- **Strings Files**: Support for `.STRINGS`, `.DLSTRINGS`, `.ILSTRINGS` formats
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
- **DeepL**: Free and Pro API support (auto-detected from API key)
- **OpenAI Compatible**: OpenAI, DeepSeek, and other Chat Completions API providers (supports prompt templates)
- Other translation providers from the Delphi original are not yet implemented here.

### Advanced Features
- **GMST:DATA Filtering**: Automatic detection of translatable vs numeric GMST records
- **ESP Compare**: Lightweight compare-only extractor with normalized FormID + field-occurrence matching
- **Record Type Filtering**: Filter strings by record type (INFO, QUST, etc.)
- **Status Filtering**: Filter by translation status (translated/incomplete/locked)
- **Virtual Rendering**: Efficient handling of large string lists (76K+ items)
- **Chunked Loading**: Batch data loading (~10K items per batch, ~2MB JSON)
- **Regex Search/Replace**: Full regex with capture groups ($1/$2), replace-all across filtered items
- **Theme System**: Dark/Light/Gray/Auto themes with CSS variables + localStorage persistence
- **Undo/Redo**: Stack-based (max 100), Ctrl+Z/Y, IPC-synced
- **Auto-Backup**: 5-min SST snapshots, rotate last 10
- **Batch Processor**: Multi-file sequential ESP translate/export with progress events and cancellation
- **BSA/BA2 Archive Browser**: Browse, preview, and extract files from BSA v0x68/v0x69 and BA2 General archives
- **PEX Script Translation**: Parse Papyrus scripts, extract translatable strings, and write updated string tables while preserving binary structure
- **FUZ Audio Mapping**: Map dialog strings to WAV audio with playback
- **NPC/Dialog View**: Dialog tree grouped by QUST→DIAL→INFO with NPC association
- **Multi-Language UI**: 10 languages (zh-CN, en, de, es, fr, ja, ko, pl, pt, ru)

## Project Structure

```
xTranslator/
├── crates/
│   ├── xt-core/         # Core library: ESP parser, strings, SST, XML, BSA, heuristic search
│   ├── xt-shared/       # Shared DTOs for IPC between backend and frontend
│   └── xt-cli/          # CLI tool (legacy, superseded by Tauri UI)
├── src-tauri/           # Tauri 2.x desktop app backend
├── ui/                  # React + Vite frontend
├── Data/                # Shared game definitions used by the rewrite
├── docs/                # Documentation
└── legacy/original-delphi/ # Original Delphi project kept as reference
```

## Project Status

The rewrite is feature-complete for the main desktop translation workflow. `SPEC.md` currently tracks 41 completed tasks covering parsing, editing, compare tools, archive support, translation APIs, config persistence, and language tooling.

The remaining work is mostly parity polish and deeper validation against the Delphi original: direct ESP editing, Delphi-style SQLite cache parity, richer compare workflows, and more real-data cross-checking.

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
- **Data Strategy**: ESP loads → frontend fetches chunks via `get_strings_chunk` (10K items/batch, ~2MB JSON) → client-side filter/sort/scroll
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

GPL-3.0 License. See [LICENSE](LICENSE) for details.
