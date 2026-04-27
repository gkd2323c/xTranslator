## Context

xTranslator now has a large set of completed v1 tasks, but its planning surfaces are not yet reliable enough for the next wave of work. `openspec validate --specs --strict` currently fails because archived capability specs still use change-delta headings instead of canonical spec headings. README and comparison docs also contain a few stale or aspirational statements, especially around translation provider support and strings deduplication. A small OpenAI provider test fix is already needed because tests that read `XT_TRANSLATE_API_*` can fail under a developer's ambient shell environment.

This change is deliberately a repository-governance repair, not a product feature.

## Goals / Non-Goals

**Goals:**

- Make existing OpenSpec specs valid under strict validation.
- Define a reusable governance capability for spec/doc/test hygiene.
- Align public documentation with implemented provider support and current known limitations.
- Preserve the env-var test isolation fix and record it in `SPEC.md`.
- Re-run the standard verification commands before completion.

**Non-Goals:**

- Do not add translation providers.
- Do not change runtime translation behavior.
- Do not implement dictionary matching, finalization, MCM, BA2, PEX write-back, or UI features.
- Do not rewrite the documentation set beyond stale or contradictory statements.

## Decisions

1. Treat existing capability specs as canonical specs, not delta specs.

   The archived changes already capture change history. The current `openspec/specs/**/spec.md` files should describe the current state with `## Purpose` and `## Requirements`, so strict validation can be a meaningful gate.

   Rejected: leave the specs as archived-style deltas. That preserves old wording but keeps `openspec validate --specs --strict` useless.

2. Add `project-governance` as a new capability.

   The repair concerns repository behavior rather than end-user translation behavior. A small governance capability gives future agents a concrete spec to validate when documentation, OpenSpec artifacts, or env-sensitive tests change.

   Rejected: modify `batch-processing` or `string-normalization` requirements just to carry this work. Those capabilities are product behavior; this change is about project integrity.

3. Fix documentation by narrowing claims to implemented behavior.

   README currently lists providers that are not present in `ProviderType::all()`. The least risky correction is to state OpenAI-compatible and DeepL as implemented, and leave Google/Microsoft/Youdao/Baidu as non-current or future-compatible references only if needed.

   Rejected: add the missing providers during this change. That would turn a planning/repair task into new feature work.

4. Verify with both OpenSpec and code checks.

   Completion requires `openspec validate --specs --strict` plus existing build/test/typecheck commands. This proves both planning artifacts and code remain usable.

## Risks / Trade-offs

- Spec wording may accidentally change intended behavior → keep batch-processing and string-normalization requirements semantically equivalent while only changing headings/structure.
- Documentation cleanup may understate aspirational roadmap items → keep roadmap/v2 items separate from implemented feature lists.
- Verification can be slowed by environment-specific E2E data → treat Skyrim real-data E2E as optional unless the local `Skyrim.esm` path is available.
