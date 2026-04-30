# Release QA Checklist

Use this checklist before tagging or shipping a release candidate. Treat it as a reusable template, not as a record of one specific release.

## Automated Verification

Run the current standard checks and record the actual results for the release you are preparing.

| Check | Expected Result | Notes |
|------|-----------------|-------|
| `cargo test -p xt-core --lib -- --nocapture` | PASS | Core library regression suite |
| `cargo build -p xtranslator-tauri` | PASS | Desktop backend build |
| `cd ui && npx tsc --noEmit` | PASS | TypeScript type check |
| `cd ui && npm run build` | PASS | Production frontend build |

## Real-Data Smoke Pass

Run these when local Bethesda game data is available.

| Scenario | Expected Result | Status |
|----------|-----------------|--------|
| Load Skyrim SE `Skyrim.esm` with language `english` | Strings load, progress overlay completes, table fills with full dataset | Not run |
| Reload the same ESP | Cache hit is reported and parse time is near zero | Not run |
| Load an SST dictionary | Match stats show exact/fallback/semantic counts, table refreshes | Not run |
| Save SST | Output file opens again and oldData entries are preserved | Not run |
| Import XML | Match stats show exact/fallback/semantic counts, table refreshes | Not run |
| Export XML | Export contains only translated entries and reparses successfully | Not run |
| Save Strings | `.STRINGS`, `.DLSTRINGS`, and `.ILSTRINGS` outputs preserve source IDs and translated entries | Not run |
| Compare two ESP files | Identical/added/removed/modified buckets look consistent and no obviously duplicated field matches appear | Not run |
| Batch translate/export cancel | Cancellation stops after the current cooperative checkpoint and leaves clear status | Not run |

## Compatibility Notes

- Dictionary apply parity is primarily tracked by unit tests and `docs/feature_comparison.md`.
- Archive support currently covers BSA v0x68/v0x69 and BA2 General workflows; texture BA2 variants and archive injection remain out of scope.
- Delphi-generated reference comparisons are still the main residual gap for any high-confidence compatibility claim.

## Maintenance

- Keep this file generic and reusable.
- Put dated release outcomes or one-off release notes in `docs/archive/`.
- Update `README.md` only with the latest verified project baseline, not with per-release checklist results.
