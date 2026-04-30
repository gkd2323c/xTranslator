---
name: xt-guardrails
description: xTranslator-specific guardrails for planning edits and choosing the smallest correct verification set.
---

# xTranslator Guardrails

Use this skill whenever work touches the xTranslator repo and you need project-specific reminders or a verification plan.

## What this skill is for

- Convert changed files into the right verification commands.
- Prevent common xTranslator mistakes:
  - editing `crates/xt-shared/src/dto.rs` without updating `ui/src/api/strings.ts`
  - editing Tauri commands without checking registration in `src-tauri/src/main.rs`
  - treating visible row index as identity instead of `selectedId`
  - assuming Tauri dev should use `beforeDevCommand` instead of `dev.ps1` or a separate Vite terminal
- Keep verification proportional to the patch instead of always running everything.

## Project invariants to enforce

- Rust DTO source of truth lives in `crates/xt-shared/src/dto.rs`; TypeScript mirror lives in `ui/src/api/strings.ts`.
- Frontend string identity is `selectedId`; do not rely on filtered/sorted row index.
- Bulk string loading uses `get_strings_chunk`; `query_strings_command` is a fallback path, not the primary large-data path.
- Tauri dev startup on Windows should use `dev.ps1` or a separate `cd ui && npm run dev` terminal before `cargo run -p xtranslator-tauri`.
- New Tauri commands must be imported into `src-tauri/src/main.rs` and listed in `generate_handler![...]`.

## Workflow

1. Inspect current changes:

```powershell
git diff --name-only
```

2. Ask the helper script for recommended checks:

```powershell
powershell -ExecutionPolicy Bypass -File .\plugins\xt-dev-guardrails\scripts\infer-checks.ps1
```

3. If you already know the file set, pass them explicitly:

```powershell
powershell -ExecutionPolicy Bypass -File .\plugins\xt-dev-guardrails\scripts\infer-checks.ps1 `
  crates/xt-shared/src/dto.rs ui/src/api/strings.ts src-tauri/src/commands.rs
```

4. Execute the suggested checks and report actual evidence, not just intent.

## What the helper script now validates directly

- If new Rust DTO/export names appear in `crates/xt-shared/src/dto.rs`, it checks whether matching TypeScript exports exist in `ui/src/api/strings.ts`.
- If new TypeScript export names appear in `ui/src/api/strings.ts`, it checks whether matching Rust DTO names exist in `crates/xt-shared/src/dto.rs`.
- If a DTO/interface with the same name changed on either side, it compares field-name sets and reports missing fields on either side.
- If new `pub async fn` Tauri commands appear in `src-tauri/src/commands.rs`, it checks whether they are both imported and registered in `src-tauri/src/main.rs`.
- If new backend commands appear, it checks whether `ui/src/api/strings.ts` contains a matching `invoke("command_name")`.
- If new frontend `invoke("command_name")` calls appear, it checks whether a matching backend Tauri command exists and is registered in `src-tauri/src/main.rs`.

## Expected output contract

- State which invariant(s) are relevant.
- State the minimum checks required.
- Mention any required mirror file or command registration updates.
- If a recommended check was skipped, say why.
