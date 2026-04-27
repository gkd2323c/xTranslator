## Why

Dictionary application is the remaining P0 gap in the translation workflow: imports can already update exact matches, and XML has an enhanced matcher, but SST loading still uses exact triples and the UI/API do not expose match confidence or reviewable ambiguous matches. Improving this creates a stronger reuse loop for existing translation work across ESP revisions.

## What Changes

- Introduce a shared dictionary-apply engine for SST/XML-style entries.
- Apply matches in ordered tiers: exact triple, EDID-based, normalized source, and vocabulary overlap.
- Report per-tier counts, unmatched counts, and ambiguous matches instead of silently applying risky candidates.
- Extend SST loading to use the same matching behavior currently available to XML import.
- Keep low-confidence matches out of automatic application unless they are unique and above threshold.
- Surface enough response data for the frontend to show match quality and reload/update affected rows.

## Capabilities

### New Capabilities
- `dictionary-apply`: Covers applying imported dictionary translations to loaded ESP strings with tiered confidence, ambiguity handling, and match statistics.

### Modified Capabilities

None.

## Impact

- Core modules: `crates/xt-core/src/matching.rs`, `crates/xt-core/src/xml/mod.rs`, `crates/xt-core/src/sst/v8.rs` integration points.
- IPC/DTO: `crates/xt-shared/src/dto.rs`, `src-tauri/src/commands.rs`, and TypeScript mirrors in `ui/src/api/strings.ts`.
- Frontend state/UI: SST/XML import result handling in Zustand/App surfaces.
- Tests: matching unit tests, SST/XML command behavior tests, TypeScript typecheck.
- No new dependencies are expected.
