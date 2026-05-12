# xTranslator

## Project Overview

xTranslator is a modern Rust-based translator for Bethesda game mods (Skyrim, Skyrim SE, Fallout 4, Starfield). It is a complete rewrite of the original Delphi xTranslator tool, featuring a Tauri 2.x desktop UI with a React frontend. 

The project aims for 100% SST v8 bidirectional compatibility with the Delphi version, supporting ESP/ESM parsing and write-back, various strings formats, BSA/BA2 archives, PEX scripts, and FUZ audio. It integrates with multiple translation APIs (OpenAI, DeepL, Baidu, Youdao, Azure, Google) and features heuristic search capabilities.

## Architecture & Structure

The project is structured as a Cargo workspace with a separate Vite/React frontend:

- `crates/xt-core/`: Core Rust library handling ESP parsing, record trees, strings, SST, XML, BSA, heuristics, and APIs.
- `crates/xt-shared/`: Shared DTOs for IPC communication between the Rust backend and TypeScript frontend.
- `src-tauri/`: Tauri backend application, managing state and exposing IPC commands.
- `ui/`: React + Vite frontend using TypeScript, Zustand for state management, `react-window` for virtual scrolling, and `react-i18next` for i18n.
- `Data/`: Shared game definitions loaded by `xt-core`.

**Key Architectural Decisions:**
- **Data Flow:** Full-load with client-side virtual scroll. The frontend fetches data in chunks (`get_strings_chunk`) and performs all filtering and sorting client-side for performance.
- **IPC Source of Truth:** `crates/xt-shared/src/dto.rs` defines the Rust structs, which must be kept in sync with the TypeScript interfaces in `ui/src/api/strings.ts`.
- **Updates by ID:** Translation updates are performed by unique ID, not by array index, as indices change during filtering/sorting.

## Building and Testing

**Prerequisites:** Rust 1.70+, Node.js 18+, and Tauri CLI (`cargo install tauri-cli`).

### Quick Start
Use the provided PowerShell script for a one-click development startup (starts Vite dev server and Tauri app):
```powershell
.\dev.ps1
```

### Manual Commands
- **Frontend Dev Server:**
  ```bash
  cd ui && npm run dev
  ```
- **Run Tauri App** (ensure Vite server is running first):
  ```bash
  cargo run -p xtranslator-tauri
  ```
- **Type Checking (Frontend):**
  ```bash
  cd ui && npx tsc --noEmit
  ```
- **Backend Build:**
  ```bash
  cargo build -p xtranslator-tauri
  ```
- **Run Unit Tests:**
  ```bash
  cargo test -p xt-core --lib
  ```
- **Run E2E Tests** (Requires Skyrim SE installed):
  ```bash
  cargo test -p xt-core --test e2e_real_data
  ```

## Development Conventions

- **Adding IPC Commands:** 
  1. Define DTOs in `crates/xt-shared/src/dto.rs`.
  2. Add TS interfaces in `ui/src/api/strings.ts`.
  3. Implement in `src-tauri/src/commands.rs` and register in `src-tauri/src/main.rs`.
  4. Export frontend wrapper in `ui/src/api/strings.ts`.
- **State Updates:** For large data mutations (e.g., XML import), the frontend must reload the full dataset. For single-item translation updates, use optimistic local updates with zero IPC overhead.
- **Documentation:** The project maintains canonical goals and tasks in `SPEC.md` and detailed architectural notes in `ARCHITECTURE.md`.
