## Context

The codebase already contains most of the raw material for enhanced dictionary application:

- `crates/xt-core/src/matching.rs` implements `enhanced_import_match()` for XML entries with exact, EDID, vocabulary, and normalized tiers.
- `crates/xt-core/src/xml/mod.rs` delegates XML import to that matcher.
- `src-tauri/src/commands.rs::load_sst` still applies SST dictionaries with only `(str_id, record_sig, field_sig)` exact matching.
- `XmlImportResponse` already reports per-tier counts, but `LoadSstResponse` only reports matched/unmatched.
- `SkyString` already carries `source_normalized`, `normalized_hash`, `word_hashes`, and `esp_ptr.edid_hash`.

The next step is consolidation: make dictionary application a shared capability, then route both XML and SST through it with clear statistics and ambiguity handling.

```
             XML entries              SST entries
                 │                        │
                 └──────────┬─────────────┘
                            ▼
                  DictionaryApplyEntry
                            │
                            ▼
        ┌──────────────────────────────────────┐
        │       shared dictionary matcher       │
        ├──────────────────────────────────────┤
        │ T1 exact triple                       │
        │ T2 EDID + record/field               │
        │ T3 normalized source + record/field  │
        │ T4 vocabulary overlap + record/field │
        └──────────────────────────────────────┘
                            │
        ┌───────────────────┴───────────────────┐
        ▼                                       ▼
   applied matches                       ambiguous/unmatched
```

## Goals / Non-Goals

**Goals:**

- Reuse one matching engine for XML and SST dictionary application.
- Preserve exact matching as the highest-confidence tier.
- Make fuzzy tiers deterministic and conservative.
- Track ambiguous candidates separately and avoid applying them automatically.
- Return per-tier stats and updated IDs to IPC callers.
- Keep DTO changes backward-compatible with `#[serde(default)]`.

**Non-Goals:**

- Do not implement a manual review UI in this change unless the existing surfaces can show a summary cheaply.
- Do not modify ESP files directly.
- Do not change SST v8 serialization format.
- Do not add new matching dependencies.
- Do not lower thresholds to maximize match count at the cost of correctness.

## Decisions

1. Introduce a neutral dictionary entry type.

   The matcher should not depend on `XmlStringEntry`. Add a small internal type such as `DictionaryApplyEntry` containing the fields needed for matching: source, translation, optional EDID, `EspPointer` or the relevant pointer fields, params when available, and source format.

   Rejected: make SST conversion pretend to be XML. That would couple unrelated formats and obscure params behavior.

2. Reorder fuzzy tiers to favor exact source equality before vocabulary overlap.

   Existing XML matching tries vocabulary before normalized hash. For automatic application, exact normalized source is safer than Jaccard overlap. Proposed order:

   - T1 exact triple
   - T2 EDID + record/field, disambiguated by normalized source when needed
   - T3 normalized source + record/field
   - T4 vocabulary overlap + record/field, unique best candidate above threshold

   Rejected: keep vocabulary before normalized. It can claim matches that exact normalized matching would explain more safely.

3. Treat ambiguous fuzzy matches as non-applied.

   If a tier finds multiple plausible candidates and cannot reduce them to one deterministic winner, it should increment an ambiguous count and leave strings unchanged. This is more important than maximizing matched count.

   Rejected: choose the first candidate. That would make dictionary application order-sensitive and hard to trust.

4. Preserve SST params only when the matched entry is applied.

   SST entries carry translation params while XML entries do not. The shared apply path should support optional params so SST can preserve translated/incomplete/validated flags, while XML keeps its current behavior of setting translated based on imported text.

   Rejected: ignore SST params globally. That loses Delphi dictionary state.

5. Expand responses compatibly.

   Add optional/defaulted fields to `LoadSstResponse` mirroring XML tier stats where useful. Existing frontend callers can continue reading matched/unmatched.

## Risks / Trade-offs

- [Risk] Vocabulary matching may false-positive short generic strings → Mitigation: require same record/field, minimum token count, unique best score, and threshold tests for short strings.
- [Risk] Changing XML tier order can alter counts → Mitigation: update tests to assert safer tier semantics and keep total matched behavior stable where possible.
- [Risk] SST params and XML status semantics differ → Mitigation: model params as optional on the neutral dictionary entry.
- [Risk] O(N*M) scans may be slow on large dictionaries → Mitigation: start with existing behavior, then add indexes by exact triple, EDID, normalized hash, and record/field if tests or real data show performance issues.
