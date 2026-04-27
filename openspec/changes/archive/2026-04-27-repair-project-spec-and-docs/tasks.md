## 1. Preserve Current Fix

- [x] 1.1 Review the existing OpenAI provider env-var test isolation change and keep it scoped to test code.
- [x] 1.2 Ensure `SPEC.md` records the env-var failure mode and invariant without changing unrelated product requirements.

## 2. Repair OpenSpec Specs

- [x] 2.1 Convert `openspec/specs/batch-processing/spec.md` from archived delta format to canonical `## Purpose` / `## Requirements` format.
- [x] 2.2 Convert `openspec/specs/string-normalization/spec.md` from archived delta format to canonical `## Purpose` / `## Requirements` format.
- [x] 2.3 Add or archive the new `project-governance` capability so it becomes part of the committed OpenSpec spec set.
- [x] 2.4 Run `openspec validate --specs --strict` and confirm all committed specs pass.

## 3. Align Documentation

- [x] 3.1 Update README translation provider claims to match implemented providers: OpenAI-compatible and DeepL.
- [x] 3.2 Review `docs/feature_comparison.md`, `docs/toolchain_and_roadmap.md`, and stale format docs for contradictions with current SPEC tasks.
- [x] 3.3 Keep roadmap-only items clearly separated from implemented feature lists.

## 4. Verify Baseline

- [x] 4.1 Run `cargo test -p xt-core --lib`.
- [x] 4.2 Run `cargo build -p xtranslator-tauri`.
- [x] 4.3 Run `cd ui && npx tsc --noEmit`.
- [x] 4.4 Record any unavailable verification, such as real-data E2E tests requiring local Skyrim data.

## 5. Handoff

- [x] 5.1 Summarize changed files, simplifications made, and remaining risks.
- [x] 5.2 Recommend the next feature change, likely `enhance-dictionary-apply`, after this governance repair is complete.
