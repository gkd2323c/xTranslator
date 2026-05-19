# Copilot instructions for xTranslator

Purpose: concise guidance for Copilot-style agents to work effectively in this repository.

1) Build, test, and lint (commands)
- Backend (Rust):
  - Full build: `cargo build -p xtranslator-tauri`
  - Workspace build: `cargo build --workspace`
  - Run all tests: `cargo test --workspace`
  - Core unit tests (no external deps): `cargo test -p xt-core --lib`
  - Run a single core test: `cargo test -p xt-core --lib <test_name>`
  - E2E (requires Skyrim SE): `cargo test -p xt-core --test e2e_real_data` (set XTRANSLATOR_TEST_SKYRIM_ESM or install Skyrim.esm at the expected path)

- Frontend (ui):
  - Type-check: `cd ui && npx tsc --noEmit`
  - Dev server: `cd ui && npm run dev`
  - Production build: `cd ui && npm run build`
  - Tests (Vitest): `cd ui && npm run test`

- One-click dev (Windows):
  - From repo root: `.\dev.ps1` (starts Vite on :5173 then runs the Tauri backend)

2) High-level architecture (quick map)
- Cargo workspace members:
  - `crates/xt-core` — core library: ESP parser/record tree, strings, SST, XML, BSA, heuristics, translation API, cache.
  - `crates/xt-shared` — IPC DTOs (source of truth for types between Rust and TS).
  - `crates/xt-cli` — legacy CLI tool.
- Tauri backend: `src-tauri` — exposes commands, batch processing, and IPC handlers.
- Frontend: `ui/` — React + Vite, Zustand store, virtualized list (react-window v2).
- Data/ — game record/definition assets used by parsers.

Data flow summary:
- Backend loads/parses ESP/SST/BSA and exposes chunked string data via IPC (`get_strings_chunk`).
- Frontend fetches in batches (default documented: 25K items/batch in README; AGENTS.md references 10K/25K variants). Client performs filtering/sorting locally and updates by ID (not index).
- DTOs are defined in `crates/xt-shared/src/dto.rs` and mirrored in `ui/src/api/strings.ts` — keep these in sync.

3) Key conventions and gotchas (important, non-obvious)
- Update-by-ID: translations are updated by `u32 id`. Frontend uses `selectedId` not array index — indices change after filtering/sorting.
- DTO sync is critical: any new fields must be added to `crates/xt-shared/src/dto.rs` and to TypeScript DTOs in `ui/src/api/strings.ts`.
- Chunking and IPC limits: watch WebView2 IPC size limits for payloads >1MB — prefer chunking/compression.
- react-window v2 API: uses `rowComponent`, `rowCount`, `rowHeight`, `rowProps`. Do NOT add `@types/react-window` (v2 ships its own types).
- Zustand usage: components should call `useAppStore((s) => s.field)` instead of reading the whole store object.
- ESP write-back: full in-memory record tree (EspField → EspRecord → EspGrup → EspFile) is used to rebuild and serialize files. Backup files `.backup.<timestamp>` are created before writes.
- Bethesda format quirks:
  - Record `dsize` excludes the 16B record header; GRUP `dsize` includes its own 24B header.
  - Compressed records: `[4-byte decompressedSize LE] + [zlib data]` — decompress before parsing subrecords.
  - Strings files: `.STRINGS` (null-terminated) vs `.DLSTRINGS`/`.ILSTRINGS` (4-byte LE length prefix).
  - FNV-1a hashing quirk for SST: Delphi's `StringHash()` hashes UTF-16 low bytes only — required for compatibility.
- Caching: ESP cache stored at `%LOCALAPPDATA%/xTranslator/cache/` (Windows) or `~/.cache/xTranslator/`. Keyed by SHA-256 of ESP content.
- XML import matching uses tiered keys (T1-T4). Ambiguous matches at same tier are NOT auto-applied.
- E2E tests require a real Skyrim.esm (path documented in README/AGENTS). Tests will fail without it.

4) Files and docs Copilot should consult first
- README.md (top-level) — features, build, and high-level notes.
- AGENTS.md — workspace map, dev startup, architecture, conventions.
- CLAUDE.md — short quick-reference for tooling and core principles.
- `crates/xt-shared/src/dto.rs` and `ui/src/api/strings.ts` — source-of-truth DTOs to check when changing IPC.
- `src-tauri/src/commands.rs` and `src-tauri/src/batch.rs` — backend command implementations and batch logic.
- `crates/xt-core/src/esp/` and `crates/xt-core/src/sst/` — parsing and serialization details.

5) Recommended search patterns and tool use
- Prefer code-intel / AGENTS.md guidance when available. For quick file search use glob/greg patterns: `rg 'get_strings_chunk|update_translation'` or `rg 'dto.rs'`.
- For modifying or adding IPC commands follow the annotated steps in AGENTS.md (dto → ts dto → commands.rs → main.rs → ui wrapper → tests).

6) Where to run checks after code changes
- Run `cargo test -p xt-core --lib` and `npx tsc --noEmit` after changes that touch Rust types or DTOs.
- If frontend changes were made, run `cd ui && npm run test` and `cd ui && npm run build`.

References
- AGENTS.md, README.md, CLAUDE.md in repo root contain fuller explanations. Use them as authoritative sources.

---
Created for Copilot sessions to accelerate correct, low-risk changes. If desired, this file can be extended to include example CI snippets or common grep patterns.
