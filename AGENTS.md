# AGENTS.md — xTranslator Rust Rewrite

## Think Before Coding

- State assumptions explicitly. If uncertain, ask.
- If multiple interpretations exist, present them — don't pick silently.
- Prefer minimum code that solves the problem. No speculative abstractions.
- Don't "improve" adjacent code. Match existing style. Surgical changes only.
- Remove imports/variables/functions that **your** changes made unused.

## Workspace Structure

Cargo workspace with 4 members:

| Member | Role | Key Entrypoints |
|--------|------|-----------------|
| `crates/xt-core` | Core library: ESP parser + record tree + write-back, BA2, MCM, PEX compile/decompile, ESP compare, strings, SST, XML, BSA, heuristic search, translation API, ESP cache, FUZ, TCSC, data configs | `src/lib.rs` |
| `crates/xt-shared` | IPC DTOs shared between backend and frontend | `src/dto.rs` |
| `crates/xt-cli` | CLI tool (legacy, mostly superseded by Tauri UI) | `src/main.rs` |
| `src-tauri` | Tauri 2.x desktop app backend | `src/main.rs`, `src/commands.rs` |
| `ui/` | React + Vite frontend (NOT a workspace member) | `src/main.tsx` |

## Build & Test Commands

```bash
# Full backend build
cargo build -p xtranslator-tauri

# Core library tests (no external deps)
cargo test -p xt-core --lib

# Cache tests
cargo test -p xt-core --lib cache

# Run a single test
cargo test -p xt-core --lib test_name_here

# E2E tests (requires real Skyrim.esm at D:\SteamLibrary\...)
cargo test -p xt-core --test e2e_real_data

# TypeScript check
cd ui && npx tsc --noEmit

# Frontend dev server (run separately — see Tauri Dev Gotcha below)
cd ui && npm run dev

# Full Tauri app (after cd ui && npm run dev in another terminal)
cargo run -p xtranslator-tauri
```

## Tauri Dev Startup

`tauri.conf.json` sets `beforeDevCommand: "echo ok"` because `cd ui && npm run dev` fails in Windows PowerShell.

### Recommended: One-Click Script

```powershell
# From project root — starts Vite + Tauri automatically
.\dev.ps1
```

This script:
1. Kills any stale `node` / `xtranslator-tauri` processes
2. Starts Vite dev server (`:5173`) in a background job
3. Waits for port 5173 to be ready (max 30s)
4. Launches `cargo run -p xtranslator-tauri`
5. Cleans up the background job when Tauri exits

### Manual (if script fails)

1. Terminal 1: `cd ui && npm run dev` (starts Vite on :5173)
2. Terminal 2: `cargo run -p xtranslator-tauri` (Tauri connects to :5173)

For production builds, `beforeBuildCommand` runs `cd ui && npm run build` correctly.

## Critical Architecture Rules

### Backend-Frontend IPC

- **DTO source of truth**: `crates/xt-shared/src/dto.rs` defines Rust structs; `ui/src/api/strings.ts` mirrors them in TypeScript. **Keep both in sync** when adding fields.
- **Data strategy**: ESP 加载后前端通过 `get_strings_chunk` 分块拉取全量数据（每批 10K 条 ~2MB JSON，76K 条约 8 批），之后筛选/排序/滚动全部在客户端完成（零延迟）。`query_strings_command` 保留作为降级方案。
- **Frontend state pipeline**: `appStore.allItems` (全量 DTO) → 客户端 filter/sort → `appStore.items` (显示用) → react-window `List` 虚拟渲染。SidePanel 统计基于 `allItems` 而非 `items`。
- **Update by ID, not index**: `update_translation` takes a `u32 id` and looks up the string in the Vec. Frontend uses `selectedId` (not array index) — indices become invalid after filtering/sorting. Store 方法: `setSelectedById()`, `updateItemTranslation(id, text)`.
- **Data refresh after mutation**: SST 加载 / XML 导入 → 后端 mutate `AppState.strings` → 前端重新 `loadAllStrings()` 分块刷新全量数据。单条翻译更新 → 前端本地 `updateItemTranslation(id, text)`（零 IPC）。

### ESP Cache

- **Location**: `%LOCALAPPDATA%/xTranslator/cache/` (Windows), `~/.cache/xTranslator/` (Linux/macOS).
- **Cache key**: SHA-256 hash of the ESP file (content-addressable). If ESP content changes, cache auto-misses.
- **Storage format**: `{sha256}.cache` files containing bincode-serialized `CachePayload { version, strings, compressed_records, strings_loaded }`.
- **Integration**: `load_esp` command checks cache before parsing. On hit → returns instantly with `cached: true`, `parse_time_ms: 0`. On miss → parses normally, then stores cache for next time.
- **Pruning**: Max 50 cache entries; oldest removed on `store()`. Manual clear via deleting the cache directory.
- **Module**: `crates/xt-core/src/cache.rs` (`EsmCache`, `CachePayload`, `hash_file`).
- **DTO field**: `LoadEspResponse.cached` (bool, `#[serde(default)]`) — frontend can show "Loaded from cache" vs parse time.

### ESP Write-Back (T42-T45)

- **Record Tree**: `EspField` → `EspRecord` → `EspGrup` → `EspFile` — full in-memory parse tree built during ESP parsing (`enable_esp_mode()`).
- **Write Commands**:
  - `save_esp`: Apply translations to field buffers → rebuild records (XXXX management, zlib recompression) → serialize → save with optional backup
  - `finalize_esp`: Apply SST translations → rebuild → serialize → export .STRINGS/.DLSTRINGS/.ILSTRINGS
  - `delocalize_esp`: Convert localized ESP to delocalized format (sequential IDs from 1)
- **Backup**: Before any ESP write, create `.backup.<timestamp>` unless user opts out (V29).
- **XXXX Handling**: Backward iteration through fields; automatic insertion/removal when field size crosses 65535 boundary.
- **Compression**: Compressed records output as `[4-byte decompressedSize LE] + [zlib data]`; record header dsize = compressed output length (V30).
- **Module**: `crates/xt-core/src/esp/record_tree.rs`, `src/esp/parser.rs`

### Data Formats (Bethesda)

- **Strings files**: `.STRINGS` = null-terminated; `.DLSTRINGS` / `.ILSTRINGS` = 4-byte length prefix.
- **ESP compressed records**: `[4-byte decompressedSize LE] + [zlib data]`. Decompress before parsing subrecords.
- **ESP dsize semantics**: Record `dsize` **excludes** the 16B record header; GRUP `dsize` **includes** its own 24B header (GenericHeader 8B + GrupHeader 16B).
- **Codepage fallback**: UTF-8 primary; on decode failure, fall back to Windows codepage via `CodepageTable` (932/936/949/950/1250-1257).

### Status Values

SkyString status strings used in DTOs and frontend:
- `"translated"` — has non-empty translation
- `"incomplete"` — partial/work-in-progress
- `"locked"` — non-translatable (e.g., GMST numeric DATA fields)

### GMST:DATA Filtering

GMST records contain a `DATA` field that can be either:
- **Numeric** (int/float) — filtered out, not translatable
- **String reference** (when EDID starts with `s`) — kept and resolved via Strings files

Filtering logic: during ESP parsing, if a GMST record's `EDID` field starts with `'s'`, its `DATA` field is treated as a string ID and looked up in `.STRINGS`. Otherwise (EDID starts with `f`/`i`/`b` or missing), the DATA field is assumed numeric and skipped.

### Heuristic Search

- Only searches strings already marked `translated`.
- Uses Levenshtein distance + LCS + LCP.
- Default threshold: 0.5 similarity, max 5 results.
- Backend: `xt-core/src/heuristic/mod.rs`; IPC: `heuristic_search` command.

### Translation Provider

- Two providers: `OpenAI` (OpenAI/DeepSeek/Qwen etc.) and `DeepL` (auto-detects free/pro).
- `AppState` holds separate API keys for each provider in `Mutex<Option<String>>` (memory only).
- `current_provider: Mutex<ProviderType>` controls which provider `translate_string` uses.
- IPC commands: `set_openai_api_key`, `set_deepl_api_key`, `set_translation_provider`, `get_translation_providers`.
- Provider trait in `xt-core/src/translation_api/mod.rs`; implementations in `openai.rs` / `deepl.rs`.

### String Normalization

- Normalizes source text for heuristic search and dictionary matching (case-fold, strip punctuation, compress whitespace).
- Module: `xt-core/src/normalization.rs`.
- `SkyString.source_normalized` and `normalized_hash` computed in `SkyString::new()`.

### XML Import/Export

- Export: `export_xml` command → `write_xml_export()` → Delphi-compatible UTF-8 XML with entity escaping.
- Import: `import_xml` command → `parse_xml_file()` → `import_xml_to_sky_strings()` — matches by `(str_id, record_sig, field_sig)` triple. Returns `XmlImportResponse { matched, unmatched, total, updated_ids }`.

### Record Types Filtering

- SidePanel Record Types 列表点击即可过滤表格内容（如只显示 INFO 记录）。
- 再次点击同一个类型或点击 "Clear filter" 取消过滤。
- 过滤逻辑在 `applyFilterAndSort` 中按 `record_sig === recordFilter` 匹配。
- 与 `statusFilter` 和 `filter`（文本搜索）是 AND 关系，可叠加使用。

## Adding a New IPC Command

1. Add DTOs to `crates/xt-shared/src/dto.rs` (derive `Serialize, Deserialize`).
2. Add TypeScript interfaces to `ui/src/api/strings.ts`.
3. Implement command in `src-tauri/src/commands.rs`.
4. Register in `src-tauri/src/main.rs` via `generate_handler!`.
5. Export frontend wrapper from `ui/src/api/strings.ts`.
6. Build and run `cargo test -p xt-core --lib` + `npx tsc --noEmit`.

**Note**: Large payload commands (>1MB) may hit WebView2 IPC limits. For bulk data, consider chunking or compression.

## Style & Conventions

- Rust: `snake_case`, 2021 edition, `anyhow` for errors, `thiserror` for custom error enums.
- Frontend: React functional components, Zustand for state (`ui/src/stores/appStore.ts`), react-hot-toast for notifications, lucide-react for icons.
- **Zustand selectors**: Use `useAppStore((s) => s.field)` instead of `const store = useAppStore()`. This prevents re-renders on unrelated state changes. Only select the fields the component actually needs.
- **react-window v2 API**: Package `react-window@2.x` uses `rowComponent`/`rowCount`/`rowHeight`/`rowProps` (NOT v1's `children`/`itemCount`/`itemSize`). Row component receives `{ ariaAttributes, index, style, ...rowProps }`. Do NOT install `@types/react-window` — v2 ships its own types.
- Comments: Mix of English and Chinese (legacy from original authors). Add new comments in English.

## Known Limitations

- E2E tests require Skyrim SE installed at `D:\SteamLibrary\steamapps\common\Skyrim Special Edition\Data\Skyrim.esm`.
- `record_defs` loading is best-effort; if `Data/<Game>/record_defs` is missing, parser falls back to generic parsing.
