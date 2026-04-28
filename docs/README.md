# xTranslator Documentation

This directory is organized by reader need: start with the project overview, then jump into architecture, format notes, compatibility analysis, or release QA.

## Start Here

| Document | Use it for |
|----------|------------|
| [`../README.md`](../README.md) | Product overview, features, quick start, build/test commands |
| [`../PLAN.md`](../PLAN.md) | Current v1.0 status, implemented feature matrix, high-level roadmap |
| [`../SPEC.md`](../SPEC.md) | Canonical goals, constraints, interfaces, invariants, and completed task list |
| [`../ARCHITECTURE.md`](../ARCHITECTURE.md) | Data flow, module responsibilities, IPC patterns, and implementation rules |
| [`release_qa_2026-04-28.md`](release_qa_2026-04-28.md) | Release verification checklist and real-data smoke test plan |

## Current Planning And Roadmap

| Document | Use it for |
|----------|------------|
| [`feature_comparison.md`](feature_comparison.md) | Gap analysis against Delphi xTranslator and next-priority candidates |
| [`toolchain_and_roadmap.md`](toolchain_and_roadmap.md) | Dependency map, warning-cleanup notes, and v2 roadmap |
| [`delphi_analysis.md`](delphi_analysis.md) | Delphi source findings mapped to Rust implementation areas |
| [`../legacy/original-delphi/README.md`](../legacy/original-delphi/README.md) | Original Delphi project archive layout |

## Architecture Notes

| Document | Use it for |
|----------|------------|
| [`i18n_architecture.md`](i18n_architecture.md) | UI localization architecture and language coverage |
| [`esp_grup_tracking.md`](esp_grup_tracking.md) | ESP GRUP parent tracking and dialog tree context |

## File Format References

| Document | Use it for |
|----------|------------|
| [`esp_format.md`](esp_format.md) | ESP/ESM binary layout, compressed records, GRUP sizing, translatable field extraction |
| [`strings_format.md`](strings_format.md) | `.STRINGS`, `.DLSTRINGS`, `.ILSTRINGS`, codepage behavior, write-back details |
| [`sst_v8_format.md`](sst_v8_format.md) | SST v8 binary format and Delphi-compatible params |
| [`bsa_format.md`](bsa_format.md) | BSA v0x68/v0x69 structure, BSAhash64, compression, current BA2 boundary |
| [`pex_format.md`](pex_format.md) | PEX binary layout and translatable string extraction notes |
| [`fuz_format.md`](fuz_format.md) | FUZ container structure and dialogue/audio association notes |

## Archive

Completed implementation plans and one-off daily plans live in [`archive/`](archive/). They are kept for historical context, but current work should use the docs above first.

| Document | Historical context |
|----------|--------------------|
| [`archive/execution_plan_v1.md`](archive/execution_plan_v1.md) | v1 execution plan after completion |
| [`archive/phase1_5_execution_plan.md`](archive/phase1_5_execution_plan.md) | Tauri UI foundation plan |
| [`archive/bsa_implementation_plan.md`](archive/bsa_implementation_plan.md) | Completed BSA support implementation plan |
| [`archive/bsa_findings.md`](archive/bsa_findings.md) | BSA browser implementation findings, superseded by `bsa_format.md` |
| [`archive/today_plan_2026-04-27.md`](archive/today_plan_2026-04-27.md) | One-day cleanup / auto-backup / undo-redo plan |

## Maintenance Rules

- Keep `SPEC.md` as the canonical source for goals, interfaces, invariants, and task completion.
- Keep `README.md` concise and user-facing; detailed implementation notes belong here or in `ARCHITECTURE.md`.
- Move completed execution plans to `docs/archive/` instead of leaving them in the active docs list.
- When support status changes, update `PLAN.md`, `README.md`, `SPEC.md`, `feature_comparison.md`, and this index together.
- Keep the original Delphi project under `legacy/original-delphi/`; keep `Data/` at the repository root because the rewrite uses it at runtime/tests.
