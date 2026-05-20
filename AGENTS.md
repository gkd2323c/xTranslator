# AGENTS.md — xTranslator

## Workspace

Rust Cargo workspace (4 members) + React/Vite frontend:

| Member | Role | Entrypoint |
|--------|------|-----------|
| `crates/xt-core` | Core library: ESP parser, strings, SST, XML, BSA, heuristic, translation API, cache, TCSC, config | `src/lib.rs` |
| `crates/xt-shared` | IPC DTOs (shared between Tauri backend & frontend) | `src/dto.rs` |
| `crates/xt-cli` | CLI tool (legacy, superseded by Tauri UI) | `src/main.rs` |
| `src-tauri` | Tauri 2.x desktop backend | `src/main.rs`, `src/commands.rs`, `src/batch.rs` |
| `ui/` | React + TypeScript + Vite frontend (**not** a Cargo member) | `src/main.tsx` |

**`Data/`** — shared game definitions used by the rewrite.

## Dev Startup

**One-click (recommended):** `.\dev.ps1` — kills stale processes, starts Vite on :5173, waits for port, launches Tauri, cleans up on exit.

**Manual:** Terminal 1: `cd ui && npm run dev` · Terminal 2: `cargo run -p xtranslator-tauri`

`tauri.conf.json` sets `beforeDevCommand: "echo ok"` because `cd ui && npm run dev` fails in Windows PowerShell. Production builds run `cd ui && npm run build` correctly.

## Build & Test Commands

```bash
# Backend
cargo build -p xtranslator-tauri          # full app
cargo build --workspace                    # all crates
cargo test -p xt-core --lib                # core unit tests (no deps)
cargo test -p xt-core --lib cache          # cache tests
cargo test -p xt-core --lib <test_name>    # single test
cargo test --workspace                     # all tests
cargo test -p xt-core --test e2e_real_data # E2E (needs Skyrim.esm)

# Frontend
cd ui && npx tsc --noEmit                  # typecheck
cd ui && npm run dev                       # dev server (:5173)
cd ui && npm run build                     # production build
cd ui && npm run test                      # vitest
```

**E2E test prerequisite:** Skyrim SE at `D:\SteamLibrary\steamapps\common\Skyrim Special Edition\Data\Skyrim.esm` (or set `XTRANSLATOR_TEST_SKYRIM_ESM`).

## Critical Architecture

### IPC / DTO Sync
- **Source of truth:** `crates/xt-shared/src/dto.rs` (Rust) ↔ `ui/src/api/strings.ts` (TypeScript). **Keep both in sync** when adding fields.
- **Data flow:** ESP loads → frontend chunks via `get_strings_chunk` (25K/batch, concurrency 3, ~2MB JSON) → client-side filter/sort/scroll. `query_strings_command` is the fallback.
- **Update by ID, not index:** `update_translation(id, text)` uses `u32 id`. Frontend uses `selectedId` — indices break after filtering/sorting. Store methods: `setSelectedById()`, `updateItemTranslation(id, text)`.
- **Data refresh:** SST load / XML import mutates `AppState.strings` on backend → frontend re-calls `loadAllStrings()`. Single translation update → frontend local `updateItemTranslation` (zero IPC).
- **Large payloads (>1MB)** may hit WebView2 IPC limits. Consider chunking or compression.

### Frontend State Pipeline
`appStore.allItems` (full DTO) → client filter/sort → `appStore.items` (display) → `react-window` `List` virtual render. **SidePanel stats are based on `allItems`, not `items`.**

### zustand Pattern
Use `useAppStore((s) => s.field)` — never `const store = useAppStore()`. Select only what the component needs to avoid unnecessary re-renders.

### react-window v2 API
Uses `rowComponent`/`rowCount`/`rowHeight`/`rowProps` (NOT v1's `children`/`itemCount`/`itemSize`). Row receives `{ ariaAttributes, index, style, ...rowProps }`. **Do NOT install `@types/react-window`** — v2 ships its own types.

## ESP Write-Back (T42-T45)

- **Record tree:** `EspField` → `EspRecord` → `EspGrup` → `EspFile` — built during parsing (`enable_esp_mode()`).
- **Commands:** `save_esp` (apply translations → rebuild → serialize → save w/ backup), `finalize_esp` (SST → export .STRINGS/.DLSTRINGS/.ILSTRINGS), `delocalize_esp` (sequential IDs from 1).
- **Backup:** `.backup.<timestamp>` before any ESP write (configurable, V29).
- **XXXX handling:** Backward iteration; auto-insert/remove when field size crosses 65535.
- **Compression:** `[4-byte decompressedSize LE] + [zlib data]`; record header `dsize` = compressed output length.
- **Modules:** `crates/xt-core/src/esp/record_tree.rs`, `src/esp/parser.rs`

## ESP Cache

- **Location:** `%LOCALAPPDATA%/xTranslator/cache/` (Windows) / `~/.cache/xTranslator/` (Unix).
- **Key:** SHA-256 of ESP content (content-addressable). `CacheIndex` uses mtime+size as a fast-path before full hashing.
- **Format:** `{sha256}.sqlite` with `sqlite_cache::CachePayload` metadata and row-level string storage.
- **Integration:** `load_esp` checks `SqliteCache` before parsing. `LoadEspResponse.cached` (bool) exposed to frontend.
- **Pruning:** Max 50 entries; oldest removed on `store()`. Manual clear = delete cache dir.
- **Module:** `crates/xt-core/src/sqlite_cache.rs`

## T1-T4 Dictionary Matching

Shared by XML import and SST load (`crates/xt-core/src/matching.rs`):

| Tier | Key | Confidence |
|------|-----|-----------|
| T1 | `(str_id, record_sig, field_sig)` exact triple | very high |
| T2 | `(edid_hash, record_sig, field_sig)` | high |
| T3 | `(normalized_hash, record_sig, field_sig)` | high |
| T4 | word_hashes Jaccard ≥ 0.5 | medium |

**Ambiguous matches** (multiple candidates at same tier) are **not** auto-applied.

## Bethesda Format Gotchas

- **ESP dsize:** Record `dsize` **excludes** 16B header; GRUP `dsize` **includes** its own 24B header.
- **Compressed records:** `[4-byte decompressedSize LE] + [zlib data]`. Decompress before parsing subrecords.
- **Strings files:** `.STRINGS` = null-terminated; `.DLSTRINGS`/`.ILSTRINGS` = 4-byte LE length prefix.
- **Codepage fallback:** UTF-8 primary; on decode failure → Windows codepage (932/936/949/950/1250-1257). Always use `CodepageConfig`.
- **FNV-1a hash quirk:** Delphi's `StringHash()` hashes UTF-16 **low bytes only**. Must match exactly for SST roundtrip.
- **GMST:DATA filtering:** If GMST `EDID` starts with `'s'`, treat `DATA` as string ID → look up in `.STRINGS`. Otherwise (`f`/`i`/`b` or missing) → numeric, skip.
- **VMAD negative str_id:** VMAD script strings encode byte offset as negative `str_id`. `is_vmad: esp_ptr.str_id < 0`.
- **MCM partial detection:** Translation is "partial" if non-empty, differs from source, and < 30% of source length.

## Adding a New IPC Command

1. Add DTOs to `crates/xt-shared/src/dto.rs` (`#[derive(Serialize, Deserialize)]`)
2. Add TypeScript interfaces to `ui/src/api/strings.ts`
3. Implement in `src-tauri/src/commands.rs`
4. Register in `src-tauri/src/main.rs` via `generate_handler!`
5. Export frontend wrapper from `ui/src/api/strings.ts`
6. Verify: `cargo test -p xt-core --lib` + `npx tsc --noEmit`

## Key Files

| File | Purpose |
|------|---------|
| `crates/xt-core/src/types/sky_string.rs` | Core string data structure |
| `crates/xt-core/src/esp/record_tree.rs` | ESP record tree for write-back |
| `crates/xt-core/src/esp/parser.rs` | ESP/ESM binary parser |
| `crates/xt-core/src/matching.rs` | T1-T4 dictionary matcher |
| `crates/xt-core/src/sqlite_cache.rs` | ESP content-addressed SQLite cache |
| `crates/xt-core/src/sst/v8.rs` | SST v8 format |
| `crates/xt-core/src/xml/mod.rs` | XML import/export |
| `crates/xt-core/src/heuristic/mod.rs` | Similarity search |
| `crates/xt-core/src/translation_api/` | Translation providers |
| `crates/xt-shared/src/dto.rs` | IPC DTOs |
| `src-tauri/src/commands.rs` | All Tauri commands |
| `src-tauri/src/batch.rs` | Batch processing state machine |
| `ui/src/stores/appStore.ts` | Frontend Zustand store |
| `ui/src/api/strings.ts` | Tauri invoke wrappers + TS DTOs |

## Style & Conventions

- **Rust:** `snake_case`, 2021 edition, `anyhow` for errors, `thiserror` for custom error enums.
- **Frontend:** React functional components, Zustand, react-hot-toast, lucide-react.
- **Comments:** Legacy mix of English/Chinese. New comments in English.
- **Remove** imports/variables/functions that your changes made unused.
- **Match** existing code style; surgical changes only. No speculative abstractions.

## Frontend Architecture (Layout Redesign — v2)

See `LAYOUT_REDESIGN_PLAN.md` for full details. All three phases completed.

### Component Tree
```
App
├── EditorDialog        (Modal xl, opened by double-click/Enter)
├── 9× Tool Dialogs     (Modal lg, gated by activePanel store)
│   ├── BatchPanel, BsaBrowser, PexPanel, FuzPanel, DialogView
│   ├── McmPanel, EspComparePanel, FinalizePanel, DataConfigsPanel
├── MenuBar             (compact 32px toolbar: search, filters, file ops, TCSC, tools, theme/lang)
├── BatchTranslateBar   (inline progress bar for string-level batch)
├── app-body
│   └── app-main
│       ├── app-table-area  → StringTable (react-window v2)
│       └── app-bottom-panel (flex:1, 10 tabs)
│           ├── home → SidePanel (stats cards + progress)
│           ├── vocabulary → VocabularyPanel (searchable pairs)
│           ├── log → LogPanel (color-coded, searchable, auto-scroll)
│           ├── heuristic/espTree/quests
│           ├── pex/dialogs (PexPanel/DialogView, also available as dialogs)
│           └── headerProc/headerWizard
│           └── headerProc/headerWizard
├── StatusBar
└── (SettingsDialog, ToolboxDialog — opened via MenuBar buttons)
```

### Store State (appStore.ts)
- **activePanel** — single-value mutex for 9 tool dialogs (replaces 9 booleans)
- **editorOpen** — EditorDialog visibility
- **selectedId/selectedItem** — current editing target
- **allItems/items** — full dataset vs filtered display set
- **selectedIds** — multi-select set (ctrl+click, shift+click range, context-menu batch)
- **logs** — LogEntry[] (max 500), addLog/clearLogs actions

- **Theme** — `"obsidian" | "dark" | "light" | "slate" | "auto"` (dark → obsidian alias)

### Key Interaction Patterns
- **Escape chain:** editor close → panel close → deselect row
- **Double-click/Enter:** opens EditorDialog for selected row
- **Ctrl+Enter:** save translation in EditorDialog
- **Ctrl+↑/↓:** jump to next/prev untranslated item (in EditorDialog)
- **Ctrl+Z/Y:** undo/redo translation changes

### Known Gaps vs Delphi Original (see docs/feature_comparison.md)
- Syntax highlighting in editor (SynEdit → not yet implemented)
- VirtualTreeView multi-select / inline edit (react-window v2 is simpler)
- PEX script editor / FUZ LIP / BA2 texture archives
- More auxiliary panels (File Browser, Quest Stage Editor)

## Document Maintenance

After every significant feature commit, verify key metrics across all docs. See `docs/README.md#maintenance-rules` for the full checklist and audit commands.

Quick checks:
- `cargo test -p xt-core --lib` → update test count in `ARCHITECTURE.md` + `RELEASE.md`
- SPEC task count → update `README.md`(中/英) + `RELEASE.md`
- API provider count → `README.md` + `ARCHITECTURE.md` + `docs/development_roadmap.md`
- Batch size → `README.md` + `ARCHITECTURE.md` (all should say 25K)
- Latest commit → `RELEASE.md`
- When deleting a doc file, run `rg <filename> --include '*.md'` to find stale references

## Known Limitations

- E2E tests need real Skyrim.esm.
- `record_defs` loading is best-effort; falls back to generic parsing if missing.
- BA2 texture-specific variants and archive injection are out of scope.
