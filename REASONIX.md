# xTranslator — Reasonix working knowledge

## Stack

- **Rust 2021** edition, workspace with 4 crates + Tauri backend
- **Tauri 2.x** desktop shell (tauri-plugin-shell, tauri-plugin-dialog)
- **React 18 + TypeScript** frontend, **Vite 5** bundler, **zustand** state management
- **react-window v2** virtual list (rowComponent/rowCount API — NOT v1's children/itemCount)
- **vitest** (unit) + **Playwright** (E2E), **serde** IPC, MPL-2.0 license

## Layout

| Path | Role |
|------|------|
| `crates/xt-core/` | Core library — ESP parser, SST, XML, matching, translation API, cache, BSA, PEX |
| `crates/xt-shared/` | IPC DTOs (Rust `#[derive(Serialize,Deserialize)]` — source of truth) |
| `crates/xt-cli/` | Legacy CLI (superseded by Tauri UI) |
| `src-tauri/` | Tauri 2.x desktop app — commands in `commands.rs`, batch in `batch.rs` |
| `ui/` | React + Vite frontend (`src/main.tsx`) |
| `Data/` | Per-game record definitions, codepages, vocabulary (loaded at runtime) |
| `docs/` | Architecture docs, format specs (ESP, SST, BSA, strings) |
| `legacy/` | Original Delphi source (reference only) |
| `tests/` | Integration tests crate (E2E real data, smoke tests) |

## Commands

```bash
cargo test -p xt-core --lib          # core unit tests (no deps)
cargo test --workspace               # all tests
cargo build -p xtranslator-tauri     # debug build
cargo tauri build                    # release build (via build.bat)
cargo clippy -p xt-core -p xt-shared -- -D warnings

cd ui && npm run dev                 # Vite dev server (:5173)
cd ui && npm run build               # production build
cd ui && npm test                    # vitest
cd ui && npx tsc --noEmit            # typecheck
cd ui && npm run test:e2e            # Playwright E2E

.\dev.ps1                            # one-click: kills stale, starts Vite, launches Tauri
```

## Conventions

- **Rust tests** use `#[cfg(test)] mod tests { ... }` colocated in source files; integration tests live in `crates/xt-core/tests/`.
- **Frontend tests** (`*.test.ts`) colocated with source, use vitest + jsdom.
- **TS strict mode** enabled with `noUnusedLocals` / `noUnusedParameters`.
- **IPC DTOs** defined in `crates/xt-shared/src/dto.rs` (Rust) and mirrored verbatim in `ui/src/api/strings.ts` — both must stay in sync.
- **Zustand stores** use selector pattern: `useAppStore((s) => s.field)` — never `const store = useAppStore()`.
- **Tauri commands** registered via `generate_handler!` in `src-tauri/src/main.rs`; implement in `src-tauri/src/commands.rs`.

## Watch out for

- **react-window v2** API differs from v1: uses `rowComponent`/`rowCount`/`rowHeight` — do NOT install `@types/react-window` (v2 ships its own types).
- **IPC DTO sync** is manual — adding a field to a Rust struct requires the identical change in the TS interface, or serialization breaks at runtime.
- **E2E tests** mock Tauri APIs via `ui/e2e/mocks/tauri-core.ts` when `VITE_E2E=true`.
- **`Misc/ApiTranslator.txt`** is loaded on startup for translation API provider config.
- **`Data/`** directory is needed at runtime per game — missing record defs cause fallback to generic ESP parsing.
- **ESLint / Prettier** not configured — rely on `cargo clippy` and `tsc --noEmit` for quality.
- **Debug builds** parse Skyrim.esm 100x+ slower than release; use `.\build.bat` for release.
