# Release QA Checklist - 2026-04-28

## Automated Verification

| Check | Result | Notes |
|------|--------|-------|
| `cargo test -p xt-core --lib` | PASS | 125 passed |
| `cargo test -p xt-core --lib matching` | PASS | 22 dictionary-apply tests passed |
| `cargo build -p xtranslator-tauri` | PASS | Debug build completed |
| `cd ui && npx tsc --noEmit` | PASS | TypeScript check completed |
| `cd ui && npm run build` | PASS | TypeScript + Vite production build completed |

## Real-Data Smoke Pass

Run this before tagging or shipping a release candidate when local Bethesda game data is available.

| Scenario | Expected Result | Status |
|----------|-----------------|--------|
| Load Skyrim SE `Skyrim.esm` with language `english` | Strings load, progress overlay completes, table fills with full dataset | Not run |
| Reload the same ESP | Cache hit is reported and parse time is near zero | Not run |
| Load an SST dictionary | Match stats show exact/fallback/semantic counts, table refreshes | Not run |
| Save SST | Output file opens again and oldData entries are preserved | Not run |
| Import XML | Match stats show exact/fallback/semantic counts, table refreshes | Not run |
| Export XML | Export contains only translated entries and reparses successfully | Not run |
| Save Strings | `.STRINGS`, `.DLSTRINGS`, and `.ILSTRINGS` outputs preserve source IDs and translated entries | Not run |
| Batch translate/export cancel | Cancellation stops after the current cooperative checkpoint and leaves clear status | Not run |

## Compatibility Notes

- Dictionary apply parity is covered by unit tests for exact/EDID/normalized/vocab tiers, pending skips, locked/incomplete precedence, tag-only updates, string ID replacement, index warnings, and preserved oldData.
- BSA v0x68/v0x69 and BA2 General browsing/extraction are implemented; texture BA2 variants and archive injection remain out of scope.
- Delphi-generated reference comparisons are still the main residual gap for a 99% compatibility claim.
