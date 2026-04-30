# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

xTranslator is a tool for translating Bethesda game mods (Skyrim, Fallout 4, Starfield, Fallout 76, Fallout NV) between languages. It's being incrementally rewritten in Rust with a Tauri 2.x + React frontend from the original Delphi codebase (~67k lines).

**Key Capabilities:**

- Load ESP/ESM files and extract translatable strings (including VMAD script properties)
- Support for .STRINGS/.DLSTRINGS/.ILSTRINGS files with codepage fallback
- SST dictionary format (bidirectional compatibility with Delphi xTranslator)
- XML import/export for translation exchange with tiered matching (T1-T4)
- Heuristic search with translation suggestions (Levenshtein + LCS + LCP)
- Translation API integration (OpenAI, DeepL, etc.) with API config parsing (ApiTranslator.txt) and CRLF protection
- Virtual scrolling with client-side filtering
- Traditional/Simplified Chinese conversion (TCSC, OpenCC + Delphi dictionaries)
- Config persistence (JSON, theme/language/API keys/proxy survive restart)
- Batch processing (translate/export multiple ESP files)
- BSA/BA2 archive browser with extraction
- PEX script string extraction and write-back
- FUZ audio mapping and WAV playback
- Dialog/NPC tree view
- UI i18n (10 languages via react-i18next)

## Project Structure

```
xTranslator/
├── crates/
│   ├── xt-core/          # Core business logic (Rust)
│   │   ├── src/esp/      # ESP/ESM binary parser
│   │   ├── src/strings/  # .STRINGS files + codepage encoding
│   │   ├── src/sst/      # SST dictionary v8 format
│   │   ├── src/xml/      # Delphi XML export format parser
│   │   ├── src/heuristic/ # Similarity search algorithms
│   │   ├── src/translation_api/ # Translation providers + API config + CRLF protection
│   │   ├── src/tcsc.rs   # Traditional/Simplified Chinese conversion
│   │   ├── src/config.rs # App config persistence (JSON)
│   │   └── src/types/    # Core types (SkyString, EspPointer, etc.)
│   ├── xt-shared/        # IPC DTOs shared between Tauri backend/frontend
│   └── xt-cli/           # CLI tool for testing and batch operations
├── src-tauri/            # Tauri 2.x desktop app backend
│   ├── src/main.rs       # App setup, command registration
│   ├── src/commands.rs   # IPC command implementations
│   └── src/batch.rs      # Batch processing state machine
├── ui/                   # React + TypeScript frontend (Vite)
│   ├── src/api/          # Tauri invoke wrappers + DTO types
│   ├── src/stores/       # Zustand state management
│   └── src/components/   # MenuBar, SidePanel, StringTable, EditorPanel
├── tests/                # Integration tests
│   └── fixtures/         # Test data files
└── docs/                 # Format documentation
```

## Build Commands

### Full Application

```bash
# Dev mode (watching + hot reload) - run in separate terminals:
cd ui && npm run dev
cargo run -p xtranslator-tauri

# Production build
cargo tauri build
```

### Rust Crates

```bash
# Build all workspace crates
cargo build --workspace

# Run all tests
cargo test --workspace

# Run specific crate tests
cargo test -p xt-core
cargo test -p xt-shared

# Run doc tests
cargo test --doc

# Run CLI tool
cargo run -p xt-cli -- --help
```

### Frontend

```bash
cd ui
npm run dev      # Development server (localhost:5173)
npm run build    # Production build (outputs to ui/dist)
npm run preview  # Preview production build
```

## Testing Workflow

### Test Categories

1. **Unit Tests** - In each Rust crate (`#[test]`)
2. **Doc Tests** - Example code in documentation (`cargo test --doc`)
3. **Integration Tests** - In `tests/` directory
4. **End-to-End Tests** - Requires game files (Skyrim.esm, etc.) via environment variables

### Running Tests

```bash
# Fast unit tests (no external dependencies)
cargo test -p xt-core --lib

# All tests including integration
cargo test --workspace

# With output visibility
cargo test --workspace -- --nocapture

# Specific test by name
cargo test -p xt-core test_parse_strings
```

### Environment Variables for Large File Tests

```bash
# Windows PowerShell
$env:XTRANSLATOR_TEST_SKYRIM_ESM = "C:\Path\To\Skyrim.esm"

# Linux/macOS
export XTRANSLATOR_TEST_SKYRIM_ESM="/path/to/Skyrim.esm"
```

## Key Technical Details

### ESP/ESM File Format

- **Record Header**: 16 bytes, not included in data size
- **GRUP Header**: 24 bytes, includes its own header in data size
- **Compressed Records**: Format `[4-byte decompressedSize LE] + [zlib data]`
- **String Lookup**: Fields store `string_id` (4 bytes), actual text in .STRINGS files

### Strings File Formats

Three variants with different encoding:

- `.STRINGS` (listIndex=0): null-terminated UTF-8 strings
- `.DLSTRINGS` (listIndex=1): 4-byte LE length prefix + content
- `.ILSTRINGS` (listIndex=2): 4-byte LE length prefix + content

**Codepage System**: Delphi uses UTF-8 first, then Windows codepage fallback (932 Japanese, 936 Chinese, 949 Korean, 950 Traditional Chinese, 1250-1257 European).

### SST v8 Dictionary Format

- UTF-16LE string encoding (Delphi compatible)
- FNV-1a hash on UTF-16 low bytes
- `EspPointer` is 24 bytes (little-endian)
- Bidirectional roundtrip with Delphi xTranslator

### XML Format

Delphi-compatible export/import:

```xml
<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<SSTXMLRessources>
  <Params><Addon>Skyrim</Addon><Source>english</Source><Dest>chinese</Dest></Params>
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

## Architecture Patterns

### IPC Layer (Tauri Commands)

Commands are registered in `src-tauri/src/main.rs` and implemented in `commands.rs`. Frontend calls via `invoke()` in `ui/src/api/strings.ts`.

**Key Commands:**

- `load_esp(espPath, stringsDir, language, game)` - Load and parse ESP file
- `load_sst(path)` / `save_sst(path, masters?)` - SST dictionary management
- `update_translation(id, translation)` - Update single string
- `get_strings_chunk(offset, limit)` - Paginated string fetch for virtual scroll
- `heuristic_search(source, minSimilarity, maxResults)` - Find similar translations
- `translate_string(source, provider, apiKey?)` - AI translation
- `export_xml(filePath, ...)` / `import_xml(filePath)` - XML exchange
- `list_bsa_files(path)` / `extract_bsa_file(path, file, out)` - BSA browser
- `parse_pex_strings(path)` - PEX string extraction
- `load_fuz_mapping(espDir)` - FUZ audio mapping
- `build_dialog_tree()` - Dialog/NPC view
- `start_batch_translate(files)` / `start_batch_export(files)` - Batch processing
- `load_config()` / `save_config(dto)` - Config persistence (JSON, theme/language/API key/proxy)
- `get_api_config()` - Provider metadata from ApiTranslator.txt

### State Management

- **Backend**: `AppState` in `src-tauri/src/main.rs` holds `Mutex<Vec<SkyString>>`, API keys, dirty flag, and `ApiTranslatorConfig`
- **Batch**: `BatchExecutor` in `src-tauri/src/batch.rs` is a separate `Mutex`-guarded state machine (Idle → Running → Done) with `AtomicBool` cancel flag, managed independently from `AppState`
- **Frontend**: Zustand store in `ui/src/stores/appStore.ts` — `allItems` holds full dataset, `items` is the filtered/sorted view for display
- **Virtual Scroll**: React Window `FixedSizeList` renders visible rows only
- **Data Refresh**: Single updates use optimistic local update; SST/XML load triggers full reload
- **Cache**: `EsmCache` in `cache.rs` uses SHA-256 content-addressed binary cache (bincode) for ESP parse results — keyed by file content hash, auto-invalidates on change

### Shared Dictionary Matcher

`matching.rs` implements a 4-tier matching system shared by both XML import and SST load:

| Tier | Strategy | Key | Confidence |
| --- | --- | --- | --- |
| T1 | Exact triple | (str_id, record_sig, field_sig) | very high |
| T2 | EDID hash | (edid_hash, record_sig, field_sig) | high |
| T3 | Normalized source | (normalized_hash, record_sig, field_sig) | high |
| T4 | Vocabulary overlap | word_hashes Jaccard >= 0.5 | medium |

Ambiguous matches (multiple candidates at same tier) are not auto-applied.

### Key Type Mappings (Delphi → Rust)

| Delphi | Rust | File |
| --- | --- | --- |
| `tSkyStr` | `SkyString` | `crates/xt-core/src/types/sky_string.rs` |
| `rEspPointer` | `EspPointer` | `crates/xt-core/src/types/esp_pointer.rs` |
| `sStrParams` | `SkyStringParams` | `crates/xt-core/src/types/params.rs` |
| `StringHash()` | `string_hash()` | `crates/xt-core/src/types/esp_pointer.rs` |
| `parseStringsEx()` | `StringsFile::load()` | `crates/xt-core/src/strings/mod.rs` |

## Common Development Tasks

### Adding a New IPC Command

1. Add DTOs to `crates/xt-shared/src/dto.rs` (derive `Serialize, Deserialize`)
2. Add TypeScript interfaces to `ui/src/api/strings.ts`
3. Implement command in `src-tauri/src/commands.rs`
4. Register in `src-tauri/src/main.rs` via `generate_handler!`
5. Export frontend wrapper from `ui/src/api/strings.ts`
6. Build and test: `cargo test -p xt-core --lib` + `npx tsc --noEmit`

### Adding a New Translation Provider

1. Implement `TranslationProvider` trait in `crates/xt-core/src/translation_api/`
2. Add provider variant to enum
3. Add command handler in `src-tauri/src/commands.rs`
4. Add UI in EditorPanel for provider selection

### Parsing New Record Types

1. Add field signature to record definitions in `crates/xt-core/src/esp/`
2. Update string extraction logic
3. Add tests with actual game data

## Important Files to Know

| File | Purpose |
| --- | --- |
| `crates/xt-core/src/types/sky_string.rs` | Core string data structure |
| `crates/xt-core/src/strings/mod.rs` | Strings file parsing/writing |
| `crates/xt-core/src/sst/v8.rs` | SST dictionary format |
| `crates/xt-core/src/xml/mod.rs` | XML import/export |
| `crates/xt-core/src/matching.rs` | Shared tiered dictionary matcher (T1-T4) |
| `crates/xt-core/src/heuristic/mod.rs` | Similarity search algorithms |
| `crates/xt-core/src/vmad.rs` | VMAD script string decoder |
| `crates/xt-core/src/cache.rs` | SHA-256 content-addressed ESP cache |
| `crates/xt-core/src/tcsc.rs` | Traditional/Simplified Chinese conversion |
| `crates/xt-core/src/config.rs` | App config persistence (JSON) |
| `crates/xt-core/src/translation_api/config.rs` | API translator config (ApiTranslator.txt) |
| `crates/xt-shared/src/dto.rs` | IPC data transfer objects |
| `src-tauri/src/commands.rs` | All Tauri command implementations |
| `src-tauri/src/batch.rs` | Batch processing state machine |
| `ui/src/stores/appStore.ts` | Frontend state management |
| `ui/src/api/strings.ts` | Tauri invoke wrappers + TypeScript DTOs |
| `ui/src/components/StringTable.tsx` | Virtual scroll table component |
| `ui/src/components/EditorPanel.tsx` | Translation editor UI |

## Documentation

- `docs/sst_v8_format.md` - SST v8 binary format specification
- `docs/esp_format.md` - ESP/ESM file format analysis
- `docs/strings_format.md` - Strings file format analysis
- `docs/pex_format.md` - PEX binary format (layout, opcodes, value types)
- `docs/fuz_format.md` - FUZ audio container format
- `docs/bsa_format.md` - BSA v0x68/v0x69 format analysis
- `docs/bsa_findings.md` - BSA archive analysis (compression, isolation, performance)
- `docs/esp_grup_tracking.md` - ESP GRUP hierarchy and parent FormID tracking
- `docs/i18n_architecture.md` - Multi-language architecture and translation workflow
- `docs/feature_comparison.md` - Full gap analysis vs Delphi original
- `docs/execution_plan.md` - Execution plan for remaining tasks
- `docs/toolchain_and_roadmap.md` - Dependencies, warnings cleanup, v2 roadmap

## Gotchas & Edge Cases

1. **Tauri Dev Workaround**: `tauri.conf.json` sets `beforeDevCommand: "echo ok"` because `cd ui && npm run dev` fails in Windows PowerShell. Development requires two terminals.

2. **FNV-1a Hash Quirk**: Delphi's `StringHash()` uses FNV-1a on UTF-16 **low bytes only**. Must match exactly for SST compatibility.

3. **ESP dsize Semantics**: Record `dsize` excludes the 16-byte header; GRUP `dsize` includes its own 24-byte header. Critical for correct parsing.

4. **Codepage Fallback**: Don't assume UTF-8. Always use `CodepageConfig` when reading/writing strings files.

5. **Virtual Scroll Performance**: `get_all_strings()` transfers ~15-20MB for 76k items. Client-side filter/sort is near-instant after initial load.

6. **Update by ID, Not Index**: After filtering/sorting, array indices don't match backend. Always use `id` field for updates.

7. **Bethesda Compression**: 44,153+ compressed records in Skyrim.esm (NAVM, LAND, CELL, NPC_). Format is `[4-byte decompressedSize LE] + [zlib data]`.

## Version Compatibility

- **Rust**: 1.70+ (edition 2021)
- **Tauri**: 2.x
- **Node.js**: 18+
- **Platforms**: Windows (primary), macOS, Linux

## Delphi Compatibility Matrix

| Feature | Status |
| --- | --- |
| SST v8 Read/Write | Full roundtrip |
| Strings File Read | All three formats |
| Strings File Write | Codepage support |
| ESP Parsing | 76k+ strings from Skyrim.esm |
| XML Import/Export | Delphi-compatible |
| Heuristic Search | Levenshtein + LCS + LCP |
| Translation API | OpenAI + DeepL + API config |
| PEX String Extraction | Parser + PexPanel, write-back v2 |
| BSA/BA2 Archive Browser | BsaBrowser + list_all_files + extract |
| FUZ Audio Mapping | FuzFile parse + WAV playback |
| Dialog/NPC View | DialogView component |
| UI i18n | 10 languages (react-i18next) |
| Batch Processor | BatchExecutor + BatchPanel |
| Config Persistence | JSON config (theme/language/API key/proxy) |
| TCSC Conversion | Full (OpenCC + Delphi dicts, IPC + UI) |
