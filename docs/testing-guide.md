# xTranslator Testing Guide

## Overview

This guide covers the comprehensive testing framework for xTranslator, including unit tests, integration tests, E2E tests, and performance benchmarks.

## Test Structure

### Backend Tests (Rust)

```
tests/
├── smoke_test.rs              # Core functionality smoke tests
├── e2e_comprehensive.rs       # Full E2E test suite
├── performance_benchmarks.rs   # Performance benchmarks
├── test_data_generator.rs     # Test data generation utilities
└── Cargo.toml                 # Test configuration

crates/xt-core/src/
├── lib.rs                     # Main library with unit tests
└── **/*.rs                    # Module-specific unit tests
```

### Frontend Tests (TypeScript)

```
ui/
├── src/stores/
│   └── appStore.test.ts       # Store logic tests
├── tests/e2e/
│   └── full-workflow.spec.ts  # Playwright E2E tests
└── package.json               # Test scripts configuration
```

## Running Tests

### Quick Start

```bash
# Run all backend tests
cargo test --workspace

# Run all frontend tests
cd ui && npm test

# Run comprehensive test suite
.\scripts\run-tests.ps1
```

### Individual Test Suites

#### Backend Tests

```bash
# Unit tests only
cargo test -p xt-core --lib

# Smoke tests (requires Skyrim data)
cargo test --release -p xt-core --test smoke_test

# E2E comprehensive tests
cargo test --release -p xt-core --test e2e_comprehensive

# Performance benchmarks
cargo test --release -p xt-core --test performance_benchmarks

# Specific test
cargo test -p xt-core --lib test_name_here
```

#### Frontend Tests

```bash
# Unit tests
cd ui && npm test

# E2E tests with Playwright (55 tests)
cd ui && npm run test:e2e

# E2E tests with UI
cd ui && npm run test:e2e:ui

# Debug E2E tests
cd ui && npm run test:e2e:debug
```

### Test Runner Script

The PowerShell script `scripts\run-tests.ps1` provides a comprehensive test runner:

```powershell
# Run all tests
.\scripts\run-tests.ps1

# Skip E2E tests (faster)
.\scripts\run-tests.ps1 -SkipE2E

# Skip performance tests
.\scripts\run-tests.ps1 -SkipPerformance

# Generate coverage report
.\scripts\run-tests.ps1 -Coverage

# Run in release mode
.\scripts\run-tests.ps1 -Release

# Verbose output
.\scripts\run-tests.ps1 -Verbose

# Filter specific tests
.\scripts\run-tests.ps1 -TestFilter "smoke"
```

## Test Data Requirements

### Skyrim Data (Optional)

Some tests require Skyrim SE installation:

```
D:\SteamLibrary\steamapps\common\Skyrim Special Edition\Data\Skyrim.esm
```

If Skyrim data is not available, tests will be skipped with a warning.

### Synthetic Test Data

For reproducible testing without Skyrim data:

```bash
# Generate test data
cargo run --bin test_data_generator

# This creates synthetic data in temp directories for testing
```

## Test Categories

### 1. Unit Tests

- **Purpose**: Test individual functions and modules
- **Location**: `crates/xt-core/src/**/*.rs`
- **Count**: 299 tests
- **Coverage**: Core parsing, string handling, SST, XML, cache

### 2. Smoke Tests

- **Purpose**: Validate core workflows end-to-end
- **Location**: `tests/smoke_test.rs`
- **Tests**: 8 comprehensive tests
- **Requirements**: Skyrim data (optional)

#### Smoke Test Coverage

1. **ESP Parsing**: Load and validate Skyrim.esm
2. **Edit/Save/Reload**: Translation workflow validation
3. **SST Roundtrip**: Dictionary save/load verification
4. **XML Roundtrip**: Import/export validation
5. **Performance**: Parsing speed and memory usage
6. **Error Handling**: Invalid inputs and edge cases
7. **Data Integrity**: Structure and content validation
8. **Multi-format**: Different Strings file formats

### 3. E2E Tests

- **Purpose**: Full application workflow testing
- **Location**: `tests/e2e_comprehensive.rs`
- **Tests**: 10 comprehensive scenarios
- **Requirements**: Skyrim data (optional)

#### E2E Test Coverage

1. **ESP Parsing Comprehensive**: Full parsing with validation
2. **Translation Workflow**: Complete edit→save→verify cycle
3. **SST Operations**: Dictionary creation and manipulation
4. **XML Roundtrip**: Import/export with matching
5. **Performance Benchmarks**: Large dataset operations
6. **Error Handling**: Graceful failure scenarios
7. **BSA Fallback**: Archive fallback testing
8. **Multi-game**: Basic compatibility checks

### 4. Performance Tests

- **Purpose**: Measure and validate performance characteristics
- **Location**: `tests/performance_benchmarks.rs`
- **Tests**: 10 benchmark scenarios
- **Metrics**: Speed, memory usage, throughput

#### Performance Benchmarks

1. **ESP Parsing**: Parsing speed with large files
2. **Memory Usage**: Memory consumption analysis
3. **Filtering**: Search and filter performance
4. **Sorting**: Large dataset sorting
5. **File I/O**: Save/load operations
6. **Heuristic Search**: Translation lookup speed
7. **API Simulation**: Translation API response times
8. **Concurrent Operations**: Multi-threading performance
9. **Memory Pressure**: Multiple parser instances
10. **Stress Testing**: Synthetic large datasets

### 5. Frontend Tests

- **Purpose**: UI component and interaction testing
- **Location**: `ui/src/stores/appStore.test.ts`, `ui/e2e/`
- **Tests**: Vitest unit tests + 64 Playwright E2E scenarios

#### Frontend Test Coverage

1. **Store Logic**: Filter, sort, and state management
2. **UI Components**: Component rendering and interaction
3. **User Workflows**: Complete user journeys
4. **Error States**: UI error handling
5. **Performance**: Large dataset rendering

#### E2E Test Infrastructure

E2E tests use Playwright with automatic mock data injection:

```
ui/e2e/
├── fixtures/base.ts              # Test fixture with auto-seed
├── mocks/tauri-core.ts            # Mock Tauri API layer
├── app.spec.ts                    # Core app functionality
├── components.spec.ts             # UI component behavior
├── workflows.spec.ts              # User workflows
└── playwright.config.ts           # Playwright configuration
```

**Key features**:
- Mock data auto-loaded via `window.__e2eAutoSeed()` on page load
- Vite aliases replace Tauri API calls in test mode
- Zustand store directly injected with test data
- 15 test categories with tagged tests (`@nav`, `@edit`, `@batch`, etc.)

See [E2E 测试文档](./e2e-test-issues.md) for detailed setup and troubleshooting.

## CI/CD Integration

### GitHub Actions Workflow

The `ci-test-workflow.yml` provides:

- **Multi-platform**: Windows, Linux, macOS
- **Multi-version**: Rust stable, Node.js 18/20
- **Parallel Execution**: Backend and frontend tests
- **Coverage Reports**: Code coverage generation
- **Quality Checks**: Formatting, linting, security audit

### Local CI Simulation

```bash
# Run CI-like tests locally
.\scripts\run-tests.ps1 -Release -Coverage

# Code quality checks
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cd ui && npm run build
```

## Test Data Management

### Test Data Generator

The `test_data_generator.rs` utility creates:

- **Synthetic Strings**: Configurable test datasets
- **SST Dictionaries**: Test dictionaries with known content
- **XML Exports**: Structured test XML files
- **Strings Files**: Various format test files
- **Vocabulary Files**: Test translation vocabularies

### Usage

```rust
use tests::test_data_generator::TestDataGenerator;

let generator = TestDataGenerator::new()?;
let strings = generator.generate_sky_strings(1000);
let sst_path = generator.generate_sst_dictionary(&strings)?;
```

## Performance Expectations

### Benchmarks

| Operation | Target | Notes |
|-----------|--------|--------|
| ESP Parsing | < 30s | Skyrim.esm (75K+ strings) |
| String Filtering | < 100ms | 100K items |
| Sorting | < 500ms | 100K items |
| SST Save/Load | < 5s | Full dictionary |
| Memory Usage | < 500MB | 100K strings |

### Regression Testing

Performance tests automatically fail if:

- Parsing speed degrades > 20%
- Memory usage increases > 25%
- Filter/sort operations exceed thresholds

## Troubleshooting

### Common Issues

1. **Skyrim Data Not Found**
   ```
   ⚠️  Skipping E2E tests - Skyrim.esm not found
   ```
   **Solution**: Install Skyrim SE or use synthetic test data

2. **Permission Errors**
   ```
   Permission denied: tests/fixtures/
   ```
   **Solution**: Run with appropriate permissions or use temp directories

3. **Memory Issues**
   ```
   Out of memory during test
   ```
   **Solution**: Run tests individually or increase system memory

4. **Time Outs**
   ```
   Test timed out after 60 seconds
   ```
   **Solution**: Use release mode or optimize test data size

### Debug Mode

For detailed debugging:

```bash
# Verbose test output
cargo test -- --nocapture

# Debug E2E tests
cd ui && PWDEBUG=1 npm run test:e2e

# Single test debugging
cargo test --exact test_name -- --nocapture
```

## Contributing

### Adding New Tests

1. **Unit Tests**: Add to relevant module in `src/`
2. **Integration Tests**: Add to `tests/` directory
3. **E2E Tests**: Extend existing test files
4. **Performance Tests**: Add benchmarks to `performance_benchmarks.rs`

### Test Guidelines

- **Descriptive Names**: Use clear, descriptive test names
- **Isolation**: Tests should not depend on each other
- **Cleanup**: Always clean up temporary files
- **Assertions**: Use specific assertion messages
- **Documentation**: Document test purpose and requirements

### Code Coverage

Maintain > 80% code coverage:

```bash
# Generate coverage report
cargo tarpaulin --workspace --lib --test '*' --out Html

# View coverage report
open target/tarpaulin/index.html
```

## Release Testing

Before releases:

1. **Full Test Suite**: `.\scripts\run-tests.ps1 -Release`
2. **Performance Validation**: Ensure benchmarks pass
3. **Cross-Platform**: Test on Windows, Linux, macOS
4. **Integration Testing**: Test with real Skyrim data
5. **Documentation**: Update test documentation

This comprehensive testing framework ensures xTranslator maintains high quality and reliability across all supported platforms and use cases.
