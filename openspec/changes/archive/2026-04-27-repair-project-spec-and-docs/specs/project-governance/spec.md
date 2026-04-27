## ADDED Requirements

### Requirement: OpenSpec specs validate strictly
The repository SHALL keep committed OpenSpec specs in a form that passes strict spec validation.

#### Scenario: Validate all specs
- **WHEN** `openspec validate --specs --strict` is run from the repository root
- **THEN** all committed specs SHALL pass validation

### Requirement: Public documentation matches implemented behavior
Public-facing documentation SHALL distinguish implemented features from roadmap or aspirational features.

#### Scenario: Translation provider documentation
- **WHEN** documentation lists supported translation providers
- **THEN** the list SHALL match providers exposed by the implemented provider registry or explicitly mark missing providers as roadmap items

### Requirement: Environment-sensitive tests isolate process environment
Tests that mutate or assert against `XT_TRANSLATE_API_*` environment variables SHALL isolate those mutations and restore previous values.

#### Scenario: Ambient translation API base URL
- **WHEN** a developer runs tests with `XT_TRANSLATE_API_BASE` already set in the shell
- **THEN** default OpenAI provider tests SHALL still pass without depending on that ambient value

### Requirement: Baseline verification remains green
The repository SHALL define and run lightweight baseline checks before claiming project-governance repair is complete.

#### Scenario: Standard verification
- **WHEN** the repair is ready for review
- **THEN** `cargo test -p xt-core --lib`, `cargo build -p xtranslator-tauri`, and `cd ui && npx tsc --noEmit` SHALL pass
