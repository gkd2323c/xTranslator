# xTranslator (Rust)

A Rust + Tauri 2.x rewrite of the Delphi-based Bethesda game mod translation tool.

## What It Does

Parses ESP/ESM files (Skyrim SE, Fallout 4, Starfield, etc.), extracts translatable strings, and provides a desktop UI for editing translations with dictionary (SST) and XML exchange support.

## Current Status

**All 26 spec tasks complete — v1.0.** The app can load ESP+Strings, display 76K+ strings in a virtual-scrolled table, edit translations, translate via API, browse BSA archives, extract PEX strings, map FUZ audio, view NPC dialogs, import XML dictionaries with multi-tier enhanced matching (exact→EDID→vocab→normalized), and switch between 10 UI languages.

### Implemented

| Feature | Status | Notes |
|---------|--------|-------|
| ESP/ESM parsing | Done | Decompresses zlib records, resolves strings via .STRINGS/.DLSTRINGS/.ILSTRINGS |
| BSA archive support | Done | Loads strings from BSA (Skyrim SE) when standalone files are missing |
| SST v8 dictionary | Done | Full read/write, Delphi-compatible UTF-16LE, roundtrip verified |
| XML import/export | Done | Delphi-compatible format with entity escaping |
| Heuristic search | Done | Levenshtein + LCS + LCP for translation suggestions |
| Translation API | Done | OpenAI + DeepL providers, runtime key setting, provider switching |
| Virtual scrolling | Done | react-window, 76K items smooth, client-side filter/sort |
| Record type filtering | Done | SidePanel click-to-filter by record signature |
| Codepage fallback | Done | 932/936/949/950/1250-1257 |
| UI locking overlay | Done | Blocks interaction during load/import/export |
| XML progress bar | Done | Shows stage + percentage during import/export |
| DeepL translation | Done | Free/Pro auto-detection, env key or runtime setting |
| String normalization | Done | NFKC normalization + tokenization for search/matching |
| Regex search/replace | Done | Regex filter toggle + Replace All with confirmation + capture groups |
| Strings write-back dedup | Done | Shared data offsets via HashMap, ~17% smaller files |
| Theme system | Done | Dark/light/gray/auto, CSS variables + localStorage + matchMedia |
| Auto-backup | Done | 5-min SST snapshots, rotate last 10, silent fail |
| Undo/Redo | Done | Stack-based (max 100), Ctrl+Z/Y, IPC sync |
| BSA/BA2 archive browser | Done | list_all_files + BsaBrowser component + unit tests |
| PEX script string extraction | Done | parser + string extraction + PexPanel, write-back v2 |
| FUZ audio mapping | Done | FuzFile parse + scan + WAV playback |
| NPC map / dialog view | Done | parent_form_id tracking via GRUP s_type, DialogView grouping |
| UI multi-language i18n | Done | react-i18next, 10 languages, zh-CN default |
| Batch processor | Done | BatchExecutor + BatchPanel, multi-file translate/export |
| Enhanced dictionary matching | Done | Multi-tier XML import: exact→EDID→vocab→normalized, ~60%→~85% hit rate |

### All Spec Tasks Complete

All 26 SPEC.md tasks (§T) now marked `x`. See [`docs/feature_comparison.md`](docs/feature_comparison.md) for remaining gap analysis vs Delphi original (MCM, ESPCompare, ESM cache, etc.).

## Quick Start

```bash
# One-click dev startup (PowerShell)
.\dev.ps1

# Or manual:
cd ui && npm run dev        # Terminal 1: Vite on :5173
cargo run -p xtranslator-tauri  # Terminal 2: Tauri

# Build
cargo build -p xtranslator-tauri --release
```

## Test

```bash
# Rust unit tests (97 tests)
cargo test -p xt-core

# E2E tests (requires Skyrim SE at D:\SteamLibrary\...)
cargo test -p xt-core --test e2e_real_data

# TypeScript check
cd ui && npx tsc --noEmit
```

## Workspace Structure

```
xTranslator/
├── crates/
│   ├── xt-core/        # Core library: ESP parser, strings, SST, XML, BSA, PEX, FUZ, heuristic, translation API
│   ├── xt-shared/      # IPC DTOs (Rust ↔ TypeScript)
│   └── xt-cli/         # CLI testing tool
├── src-tauri/          # Tauri 2.x backend (main.rs, commands.rs, batch.rs)
├── ui/                 # React + Vite frontend
└── docs/               # Format specs and analysis docs
```

## Key Design Decisions

- **Full-load + client-side virtual scroll**: Frontend pulls all 76K DTOs in chunks (~2MB per batch), then filter/sort/scroll entirely client-side (<10ms).
- **Update by ID, not index**: `update_translation` takes a `u32 id` and scans the Vec. Array indices are invalid after filtering/sorting.
- **Strings write-back**: Translation results go to .STRINGS files, ESP itself is not modified (same strategy as Delphi).
- **BSA fallback**: If standalone Strings files are missing, scan `.bsa` archives in the ESP directory and extract from `strings/` folder.

## Docs

- [`ARCHITECTURE.md`](ARCHITECTURE.md) — Data flow, module details, adding IPC commands
- [`AGENTS.md`](AGENTS.md) — Coding conventions and project rules for AI assistants
- [`docs/bsa_format.md`](docs/bsa_format.md) — BSA v0x68/v0x69 format analysis
- [`docs/esp_format.md`](docs/esp_format.md) — ESP/ESM binary format
- [`docs/sst_v8_format.md`](docs/sst_v8_format.md) — SST v8 dictionary binary format
- [`docs/feature_comparison.md`](docs/feature_comparison.md) — Full gap analysis vs Delphi original
- [`docs/pex_format.md`](docs/pex_format.md) — PEX binary format (layout, opcodes, value types)
- [`docs/fuz_format.md`](docs/fuz_format.md) — FUZ audio container format
- [`docs/bsa_findings.md`](docs/bsa_findings.md) — BSA archive analysis (compression, isolation, performance)
- [`docs/esp_grup_tracking.md`](docs/esp_grup_tracking.md) — ESP GRUP hierarchy and parent FormID tracking
- [`docs/i18n_architecture.md`](docs/i18n_architecture.md) — Multi-language architecture and translation workflow
- [`docs/toolchain_and_roadmap.md`](docs/toolchain_and_roadmap.md) — Dependencies, warnings cleanup, v2 roadmap

## License

Original Delphi tool by McGuffin (MPL 1.1). This Rust rewrite is a clean-room reimplementation.
