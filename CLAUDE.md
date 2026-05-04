# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

xTranslator is a tool for translating Bethesda game mods (Skyrim, Fallout 4, Starfield, Fallout 76, Fallout NV) between languages. It's being incrementally rewritten in Rust with a Tauri 2.x + React frontend from the original Delphi codebase (~67k lines).

**Key Capabilities:**

- Load ESP/ESM files and extract translatable strings (including VMAD script properties)
- Support for .STRINGS/.DLSTRINGS/.ILSTRINGS files with codepage fallback
- SST dictionary format (bidirectional compatibility with Delphi xTranslator)
- XML import/export for translation exchange with tiered matching (T1-T4)
- Heuristic search with translation suggestions (Levenshtein + LCS + LCP + Delphi 6-dimension scoring)
- Translation API integration (OpenAI, DeepL, etc.) with API config parsing (ApiTranslator.txt) and CRLF protection
- Virtual scrolling with client-side filtering
- Traditional/Simplified Chinese conversion (TCSC, OpenCC + Delphi dictionaries)
- Config persistence (JSON, theme/language/API keys/proxy/esp_mode survive restart)
- Batch processing (file-level and string-level, with progress events and cancellation)
- BSA/BA2 archive browser with extraction
- PEX script string extraction, write-back, and decompilation
- FUZ audio mapping, WAV playback, and LIP lip-sync parsing
- Dialog/NPC tree view
- UI i18n (10 languages via react-i18next)
- RTL/Arabic text processing (logical→presentation shaping, bidirectional reorder)
- Crash-safe translation cache (JSONL journal with recovery on restart)
- ESP write-back (save_esp, finalize_esp, delocalize_esp)

## Project Structure

```
xTranslator/
├── crates/
│   ├── xt-core/              # Core business logic (Rust)
│   │   ├── src/esp/          # ESP/ESM binary parser + record tree
│   │   ├── src/strings/      # .STRINGS files + codepage encoding
│   │   ├── src/sst/          # SST dictionary v8 format
│   │   ├── src/xml/          # Delphi XML export format parser
│   │   ├── src/heuristic/    # Similarity search + Delphi 6-dim scoring
│   │   ├── src/translation_api/ # Translation providers + API config + CRLF protection
│   │   ├── src/pex/          # PEX parser, string extraction, decompiler
│   │   ├── src/ba2/          # BA2 archive format (Fallout 4, Starfield)
│   │   ├── src/bsa/          # BSA v0x68/v0x69 archive format
│   │   ├── src/fuz/          # FUZ audio container + LIP lip-sync
│   │   ├── src/matching.rs   # T1-T4 tiered dictionary matcher
│   │   ├── src/normalization.rs # String normalization for matching
│   │   ├── src/vocabulary.rs # vocabulary.txt parser + heuristic enrichment
│   │   ├── src/vmad.rs       # VMAD script string decoder
│   │   ├── src/mcm.rs        # MCM translation file support
│   │   ├── src/data_config.rs # ctdaFunc.txt, fieldSizeRef.txt, etc.
│   │   ├── src/rtl.rs        # RTL/Arabic text processing
│   │   ├── src/tcsc.rs       # Traditional/Simplified Chinese conversion
│   │   ├── src/config.rs     # App config persistence (JSON)
│   │   ├── src/cache.rs      # Legacy bincode ESP cache
│   │   ├── src/sqlite_cache.rs # SQLite ESP cache (WAL mode, replaces bincode)
│   │   ├── src/cache_index.rs # mtime+size→SHA-256 index for fast cache lookup
│   │   ├── src/translation_cache.rs # JSONL journal for crash-safe translation recovery
│   │   ├── src/batch_queue.rs # String-level concurrent batch translation
│   │   ├── src/testing.rs    # Test utilities
│   │   └── src/types/        # Core types (SkyString, EspPointer, etc.)
│   ├── xt-shared/            # IPC DTOs shared between Tauri backend/frontend
│   └── xt-cli/               # CLI tool for testing, batch ops, golden diff
├── src-tauri/                # Tauri 2.x desktop app backend
│   ├── src/main.rs           # App setup, command registration (~64 commands)
│   ├── src/commands.rs       # IPC command implementations
│   └── src/batch.rs          # File-level batch processing state machine
├── ui/                       # React + TypeScript frontend (Vite)
│   ├── src/api/              # Tauri invoke wrappers + DTO types
│   ├── src/stores/           # Zustand state management
│   └── src/components/       # See "Key Components" below
├── tests/                    # Integration tests
│   └── fixtures/             # Test data files
└── docs/                     # Format documentation
```

## Build Commands

### Full Application

```bash
# Dev mode (recommended): one-click script — kills stale processes, starts Vite on :5173, launches Tauri
.\dev.ps1

# Manual dev (two terminals):
# Terminal 1: cd ui && npm run dev
# Terminal 2: cargo run -p xtranslator-tauri

# Production build
cargo tauri build
```

### Rust Crates

```bash
# Build all workspace crates
cargo build --workspace

# Run all tests
cargo test --workspace

# Run specific crate tests (fast, no external deps)
cargo test -p xt-core --lib

# Run single test by name
cargo test -p xt-core test_parse_strings

# Run doc tests
cargo test --doc

# Run CLI tool
cargo run -p xt-cli -- --help

# Typecheck frontend
cd ui && npx tsc --noEmit
```

### Frontend

```bash
cd ui
npm run dev      # Development server (localhost:5173)
npm run build    # Production build (outputs to ui/dist)
npm run preview  # Preview production build
npm run test     # Vitest
```

## Key Components

**Top-level** (`ui/src/components/`): `MenuBar`, `SidePanel`, `StringTable`, `EditorPanel`, `StatusBar`, `ContextMenu`, `BatchTranslateBar`, `RecoveryPromptModal`, `DialogView`, `BatchPanel`, `BsaBrowser`, `FuzPanel`, `SettingsDialog`, `DataConfigsPanel`, `EspComparePanel`, `FinalizePanel`, `PexPanel`, `McmPanel`

**Bottom panels** (`ui/src/components/bottom/`): `HeuristicPanel`, `EspTreePanel`, `QuestsPanel`, `LogPanel`, `VocabularyPanel`

**UI primitives** (`ui/src/components/ui/`): `Button`, `Input`, `Textarea`, `Select`, `Badge`, `Modal`, `Section`, `KeyValueRow`, `EmptyState`, `StatusDot`, `ProgressBar`, `Spinner`

## Architecture Patterns

### IPC Layer (Tauri Commands)

Commands registered in `src-tauri/src/main.rs` (~64 commands), implemented in `commands.rs`. Frontend calls via `invoke()` wrappers in `ui/src/api/strings.ts`.

**Data flow:** ESP loads → frontend chunks via `get_strings_chunk` (25K/batch, concurrency 3, ~2MB JSON) → client-side filter/sort/scroll. `query_strings_command` is the fallback.

**Update by ID, not index:** `update_translation(id, text)` uses `u32 id`. Frontend uses `selectedId` — indices break after filtering/sorting.

**Large payloads (>1MB)** may hit WebView2 `postMessage` limits. Use chunking.

### Frontend State Pipeline

`appStore.allItems` (full DTO) → client filter/sort → `appStore.items` (display) → `react-window` `List` virtual render.

**SidePanel stats are based on `allItems`, not `items`.**

### Zustand Pattern

Use `useAppStore((s) => s.field)` — never `const store = useAppStore()`. Select only what the component needs.

### react-window v2 API

Uses `rowComponent`/`rowCount`/`rowHeight`/`rowProps` (NOT v1's `children`/`itemCount`/`itemSize`). Row receives `{ ariaAttributes, index, style, ...rowProps }`. **Do NOT install `@types/react-window`** — v2 ships its own types.

### Backend State (`AppState`)

Holds: `strings`, `sst_old_data`, `file_info`, `openai_api_key`, `deepl_api_key`, `current_provider`, `is_dirty`, `api_config`, `vocabulary`, `batch_queue`, `esp_file`, `codepage_table`. All behind `Mutex`.

- **File-level batch:** `BatchExecutor` in `src-tauri/src/batch.rs` — separate `Mutex`-guarded state machine (Idle → Running → Done) with `AtomicBool` cancel flag.
- **String-level batch:** `BatchQueue` in `crates/xt-core/src/batch_queue.rs` — concurrent translation with progress events.
- **ESP write-back:** `esp_file: Mutex<Option<EspFile>>` holds in-memory record tree for `save_esp`/`finalize_esp`.

### ESP Cache

Three-layer cache system:

1. **CacheIndex** (`cache_index.rs`): JSON file mapping `(path, mtime, size)` → SHA-256. Avoids full hash on unchanged files.
2. **SQLite cache** (`sqlite_cache.rs`): WAL mode, NORMAL synchronous. `{sha256}.db` per cached ESP. Replaces legacy bincode format.
3. **Legacy bincode** (`cache.rs`): Still present, being phased out.

Location: `%LOCALAPPDATA%/xTranslator/cache/` (Windows) / `~/.cache/xTranslator/` (Unix).

### T1-T4 Dictionary Matching

Shared by XML import and SST load (`crates/xt-core/src/matching.rs`):

| Tier | Key | Confidence |
| --- | --- | --- |
| T1 | `(str_id, record_sig, field_sig)` exact triple | very high |
| T2 | `(edid_hash, record_sig, field_sig)` | high |
| T3 | `(normalized_hash, record_sig, field_sig)` | high |
| T4 | word_hashes Jaccard ≥ 0.5 | medium |

Ambiguous matches (multiple candidates at same tier) are not auto-applied.

### Translation Cache

Crash-safe JSONL journal (`translation_cache.rs`). Each translated string flushed immediately. On restart: `check_pending_cache` → `apply_translation_cache` → `discard_translation_cache`.

## Adding a New IPC Command

1. Add DTOs to `crates/xt-shared/src/dto.rs` (`#[derive(Serialize, Deserialize)]`)
2. Add TypeScript interfaces to `ui/src/api/strings.ts`
3. Implement in `src-tauri/src/commands.rs`
4. Register in `src-tauri/src/main.rs` via `generate_handler!`
5. Export frontend wrapper from `ui/src/api/strings.ts`
6. Verify: `cargo test -p xt-core --lib` + `npx tsc --noEmit`

## Important Files

| File | Purpose |
| --- | --- |
| `crates/xt-core/src/types/sky_string.rs` | Core string data structure |
| `crates/xt-core/src/esp/parser.rs` | ESP/ESM binary parser |
| `crates/xt-core/src/esp/record_tree.rs` | ESP record tree for write-back |
| `crates/xt-core/src/matching.rs` | T1-T4 dictionary matcher |
| `crates/xt-core/src/sqlite_cache.rs` | SQLite ESP cache |
| `crates/xt-core/src/cache_index.rs` | mtime+size→SHA-256 cache index |
| `crates/xt-core/src/translation_cache.rs` | JSONL translation recovery journal |
| `crates/xt-core/src/sst/v8.rs` | SST v8 format |
| `crates/xt-core/src/xml/mod.rs` | XML import/export |
| `crates/xt-core/src/heuristic/mod.rs` | Similarity search |
| `crates/xt-core/src/heuristic/delphi_scoring.rs` | Delphi 6-dimension scoring |
| `crates/xt-core/src/translation_api/` | Translation providers |
| `crates/xt-core/src/pex/decompile.rs` | PEX decompiler |
| `crates/xt-core/src/batch_queue.rs` | String-level batch translation |
| `crates/xt-core/src/rtl.rs` | RTL/Arabic text processing |
| `crates/xt-shared/src/dto.rs` | IPC DTOs |
| `src-tauri/src/commands.rs` | All Tauri commands |
| `src-tauri/src/batch.rs` | File-level batch state machine |
| `ui/src/stores/appStore.ts` | Frontend Zustand store |
| `ui/src/api/strings.ts` | Tauri invoke wrappers + TS DTOs |
| `ui/src/components/StringTable.tsx` | Virtual scroll table |
| `ui/src/components/EditorPanel.tsx` | Translation editor |

## Gotchas & Edge Cases

1. **Tauri Dev Workaround**: `tauri.conf.json` sets `beforeDevCommand: "echo ok"` because `cd ui && npm run dev` fails in PowerShell. Use `.\dev.ps1` or two terminals.

2. **FNV-1a Hash Quirk**: Delphi's `StringHash()` uses FNV-1a on UTF-16 **low bytes only**. Must match exactly for SST compatibility.

3. **ESP dsize Semantics**: Record `dsize` excludes the 16-byte header; GRUP `dsize` includes its own 24-byte header. Critical for correct parsing.

4. **Codepage Fallback**: Don't assume UTF-8. Always use `CodepageConfig` when reading/writing strings files.

5. **Update by ID, Not Index**: After filtering/sorting, array indices don't match backend. Always use `id` field for updates.

6. **Bethesda Compression**: 44,153+ compressed records in Skyrim.esm (NAVM, LAND, CELL, NPC_). Format is `[4-byte decompressedSize LE] + [zlib data]`.

7. **GMST:DATA Filtering**: If GMST `EDID` starts with `'s'`, treat `DATA` as string ID → look up in `.STRINGS`. Otherwise (`f`/`i`/`b` or missing) → numeric, skip.

8. **VMAD Negative str_id**: VMAD script strings encode byte offset as negative `str_id`. `is_vmad: esp_ptr.str_id < 0`.

9. **ESP mode**: When `esp_mode=true` (persisted in config.json), save operations write back to ESP file directly. Otherwise write .STRINGS files.

10. **MCM Partial Detection**: Translation is "partial" if non-empty, differs from source, and < 30% of source length.

## Version Compatibility

- **Rust**: 1.70+ (edition 2021)
- **Tauri**: 2.x
- **Node.js**: 18+
- **Platforms**: Windows (primary), macOS, Linux

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
