## Why

Recent Delphi source analysis shows that xTranslator's dictionary apply behavior is more than matching text: it carries status transitions, pending entries, old-data preservation, tag-only updates, string ID changes, and warning semantics through SST and XML workflows. The Rust rewrite now has a shared tiered matcher, so the next useful step is to align its apply semantics with the Delphi workflow before adding more formats.

## What Changes

- Extend dictionary application semantics for SST loads and XML imports with Delphi-compatible handling of pending, locked, incomplete, translated, and validated states.
- Preserve or report unapplied SST entries as old data so loading and saving dictionaries does not silently discard useful historical translations.
- Add deterministic handling for tag-only application, optional string ID replacement, and index/indexMax mismatch warnings.
- Keep the existing conservative matching tiers and ambiguity behavior; do not auto-apply ambiguous candidates.
- Update project documentation and Delphi analysis notes so they describe the current shared matcher and the remaining parity gaps accurately.
- Add focused regression tests around the Delphi-derived apply semantics before implementation changes.

## Capabilities

### New Capabilities

- None.

### Modified Capabilities

- `dictionary-apply`: add Delphi-compatible apply state semantics on top of the existing exact, EDID, normalized, and vocabulary matching tiers.

## Impact

- Affected Rust core modules: `crates/xt-core/src/matching.rs`, `crates/xt-core/src/sst/v8.rs`, `crates/xt-core/src/types/params.rs`, and related tests.
- Affected Tauri commands and DTOs: `src-tauri/src/commands.rs`, `crates/xt-shared/src/dto.rs`, and `ui/src/api/strings.ts` if responses need old-data, warning, or apply-option fields.
- Affected documentation: `docs/feature_comparison.md`, `docs/delphi_analysis.md`, and OpenSpec `dictionary-apply`.
- No new third-party dependencies are expected.
