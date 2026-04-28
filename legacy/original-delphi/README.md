# Original Delphi xTranslator

This directory contains the original Delphi/VCL xTranslator project files that were kept as reference material for the Rust + Tauri rewrite.

## Contents

| Path | Contents |
|------|----------|
| `xTranslator.dpr`, `xTranslator.dproj`, `*.res`, `*.otares`, `*.mes` | Delphi project and build artifacts/resources |
| `TESVT_*.pas`, `TESVT_*.dfm` | Original Delphi source units and VCL forms |
| `SynHighlighter*.pas`, `SynHighlighter*.msg` | Original syntax highlighter sources/messages |
| `FolderUtil.pas`, `LibHunspell.pas` | Original helper units |
| `Batch/`, `Misc/`, `Res/`, `_pics/`, `zlz4/` | Original app resources, examples, documentation assets, and Delphi LZ4 sources |
| `_ReadMe.txt`, `HeaderList.txt`, `_config.inc`, `compilOpt.optset`, `Feather.ico` | Original project support files |

## Relationship To The Rewrite

- The active Rust/Tauri rewrite lives at the repository root in `crates/`, `src-tauri/`, and `ui/`.
- `Data/` intentionally remains at the repository root because the rewrite still uses it for game-specific record definitions, codepage tables, and related data.
- Format and behavior notes derived from these Delphi files live in `docs/`, especially `docs/delphi_analysis.md`, `docs/esp_format.md`, `docs/strings_format.md`, `docs/sst_v8_format.md`, `docs/bsa_format.md`, `docs/pex_format.md`, and `docs/fuz_format.md`.
- Treat this directory as read-only reference unless the goal is explicitly to preserve or annotate the original project.
