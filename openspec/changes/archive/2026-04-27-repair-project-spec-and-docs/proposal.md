## Why

The project has reached a v1-style feature-complete state, but its planning and documentation surfaces are drifting: existing OpenSpec specs fail strict validation, README provider claims exceed the implemented provider set, and env-dependent tests can fail under a developer's local configuration. Fixing this first gives future changes a trustworthy baseline.

## What Changes

- Repair existing OpenSpec spec formatting so `openspec validate --specs --strict` succeeds.
- Align public project documentation with implemented behavior, especially translation provider support.
- Keep the current OpenAI environment-variable test isolation fix captured in project history and specs.
- Establish a lightweight project-governance capability for future documentation/spec/test hygiene.
- No application feature behavior changes are intended.

## Capabilities

### New Capabilities
- `project-governance`: Covers repository-level rules for OpenSpec validity, documentation truthfulness, and environment-sensitive verification.

### Modified Capabilities

None.

## Impact

- Affected artifacts: `openspec/specs/**/spec.md`, README/project docs, `SPEC.md`, and env-sensitive translation API tests.
- Affected commands: `openspec validate --specs --strict`, `cargo test -p xt-core --lib`, `cargo build -p xtranslator-tauri`, and `cd ui && npx tsc --noEmit`.
- No new dependencies, no IPC/API changes, and no frontend behavior changes.
