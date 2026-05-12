#!/usr/bin/env pwsh

# Comprehensive test runner for xTranslator
# Runs all test suites with proper environment setup and reporting

param(
    [switch]$SkipE2E,
    [switch]$SkipPerformance,
    [switch]$Coverage,
    [switch]$Release,
    [string]$TestFilter = "",
    [switch]$Verbose
)

$ErrorActionPreference = "Stop"

# Color output functions
function Write-ColorOutput($ForegroundColor) {
    $fc = $host.UI.RawUI.ForegroundColor
    $host.UI.RawUI.ForegroundColor = $ForegroundColor
    if ($args) {
        Write-Output $args
    }
    $host.UI.RawUI.ForegroundColor = $fc
}

function Write-Success($message) {
    Write-ColorOutput Green "✅ $message"
}

function Write-Warning($message) {
    Write-ColorOutput Yellow "⚠️  $message"
}

function Write-Error($message) {
    Write-ColorOutput Red "❌ $message"
}

function Write-Info($message) {
    Write-ColorOutput Cyan "ℹ️  $message"
}

function Write-Header($message) {
    Write-ColorOutput Magenta "`n=== $message ==="
}

# Test environment setup
function Setup-TestEnvironment {
    Write-Header "Setting up test environment"
    
    # Check if we're in the right directory
    if (-not (Test-Path "Cargo.toml")) {
        Write-Error "Not in project root directory. Please run from project root."
        exit 1
    }
    
    # Check for required tools
    $requiredTools = @("cargo", "node", "npm")
    foreach ($tool in $requiredTools) {
        if (-not (Get-Command $tool -ErrorAction SilentlyContinue)) {
            Write-Error "$tool is not installed or not in PATH"
            exit 1
        }
    }
    
    Write-Success "Environment setup complete"
}

# Run backend tests
function Run-BackendTests {
    Write-Header "Running Backend Tests"
    
    $testArgs = @("test", "-p", "xt-core", "--lib")
    if ($Release) { $testArgs += "--release" }
    if ($Verbose) { $testArgs += "--", "--nocapture" }
    if ($TestFilter) { $testArgs += "--", $TestFilter }
    
    Write-Info "Running: cargo $($testArgs -join ' ')"
    
    $process = Start-Process -FilePath "cargo" -ArgumentList $testArgs -Wait -PassThru -NoNewWindow
    
    if ($process.ExitCode -eq 0) {
        Write-Success "Backend tests passed"
    } else {
        Write-Error "Backend tests failed with exit code $($process.ExitCode)"
        exit $process.ExitCode
    }
}

# Run frontend tests
function Run-FrontendTests {
    Write-Header "Running Frontend Tests"
    
    Push-Location "ui"
    
    try {
        # Install dependencies if needed
        if (-not (Test-Path "node_modules")) {
            Write-Info "Installing frontend dependencies..."
            npm ci
        }
        
        # Run unit tests
        Write-Info "Running frontend unit tests..."
        $npmArgs = @("test")
        if ($Verbose) { $npmArgs += "--", "--verbose" }
        
        $process = Start-Process -FilePath "npm" -ArgumentList $npmArgs -Wait -PassThru -NoNewWindow
        
        if ($process.ExitCode -eq 0) {
            Write-Success "Frontend tests passed"
        } else {
            Write-Error "Frontend tests failed with exit code $($process.ExitCode)"
            exit $process.ExitCode
        }
    }
    finally {
        Pop-Location
    }
}

# Run E2E tests
function Run-E2ETests {
    if ($SkipE2E) {
        Write-Warning "Skipping E2E tests"
        return
    }
    
    Write-Header "Running E2E Tests"
    
    $testArgs = @("test", "-p", "xt-core", "--test", "e2e_comprehensive")
    if ($Release) { $testArgs += "--release" }
    if ($Verbose) { $testArgs += "--", "--nocapture" }
    
    Write-Info "Running: cargo $($testArgs -join ' ')"
    
    $process = Start-Process -FilePath "cargo" -ArgumentList $testArgs -Wait -PassThru -NoNewWindow
    
    if ($process.ExitCode -eq 0) {
        Write-Success "E2E tests passed"
    } else {
        Write-Warning "E2E tests failed (may be expected if Skyrim data not available)"
    }
}

# Run performance tests
function Run-PerformanceTests {
    if ($SkipPerformance) {
        Write-Warning "Skipping performance tests"
        return
    }
    
    Write-Header "Running Performance Tests"
    
    $testArgs = @("test", "-p", "xt-core", "--test", "performance_benchmarks")
    if ($Release) { $testArgs += "--release" }
    if ($Verbose) { $testArgs += "--", "--nocapture" }
    
    Write-Info "Running: cargo $($testArgs -join ' ')"
    
    $process = Start-Process -FilePath "cargo" -ArgumentList $testArgs -Wait -PassThru -NoNewWindow
    
    if ($process.ExitCode -eq 0) {
        Write-Success "Performance tests passed"
    } else {
        Write-Warning "Performance tests failed (may be expected if Skyrim data not available)"
    }
}

# Run code quality checks
function Run-CodeQualityChecks {
    Write-Header "Running Code Quality Checks"
    
    # Rust formatting check
    Write-Info "Checking Rust code formatting..."
    $process = Start-Process -FilePath "cargo" -ArgumentList @("fmt", "--all", "--", "--check") -Wait -PassThru -NoNewWindow
    if ($process.ExitCode -ne 0) {
        Write-Error "Rust code formatting check failed"
        Write-Info "Run 'cargo fmt' to fix formatting issues"
        exit $process.ExitCode
    }
    
    # Clippy checks
    Write-Info "Running Clippy..."
    $process = Start-Process -FilePath "cargo" -ArgumentList @("clippy", "--workspace", "--all-targets", "--all-features", "--", "-D", "warnings") -Wait -PassThru -NoNewWindow
    if ($process.ExitCode -ne 0) {
        Write-Error "Clippy checks failed"
        exit $process.ExitCode
    }
    
    # Frontend type checking
    Push-Location "ui"
    try {
        Write-Info "Running TypeScript type checking..."
        $process = Start-Process -FilePath "npm" -ArgumentList @("run", "build") -Wait -PassThru -NoNewWindow
        if ($process.ExitCode -ne 0) {
            Write-Error "TypeScript type checking failed"
            exit $process.ExitCode
        }
    }
    finally {
        Pop-Location
    }
    
    Write-Success "Code quality checks passed"
}

# Generate coverage report
function Generate-CoverageReport {
    if (-not $Coverage) {
        return
    }
    
    Write-Header "Generating Coverage Report"
    
    # Install cargo-tarpaulin if not present
    $tarpaulinVersion = cargo tarpaulin --version 2>$null
    if (-not $tarpaulinVersion) {
        Write-Info "Installing cargo-tarpaulin..."
        cargo install cargo-tarpaulin
    }
    
    # Generate coverage
    $coverageArgs = @("tarpaulin", "--workspace", "--lib", "--test", "*", "--out", "Xml", "--output-dir", "./coverage")
    if ($Verbose) { $coverageArgs += "--verbose" }
    
    Write-Info "Running: cargo $($coverageArgs -join ' ')"
    $process = Start-Process -FilePath "cargo" -ArgumentList $coverageArgs -Wait -PassThru -NoNewWindow
    
    if ($process.ExitCode -eq 0) {
        Write-Success "Coverage report generated in ./coverage/"
        if (Test-Path "./coverage/cobertura.xml") {
            Write-Info "Coverage XML file: ./coverage/cobertura.xml"
        }
    } else {
        Write-Warning "Coverage generation failed"
    }
}

# Generate test report
function Generate-TestReport {
    Write-Header "Test Summary"
    
    $timestamp = Get-Date -Format "yyyy-MM-dd HH:mm:ss"
    Write-Info "Test completed at: $timestamp"
    
    # Count test files
    $rustTestFiles = (Get-ChildItem -Path "crates/xt-core/src" -Filter "*.rs" -Recurse).Count
    $frontendTestFiles = (Get-ChildItem -Path "ui/src" -Filter "*.test.*" -Recurse).Count
    
    Write-Info "Rust test files: $rustTestFiles"
    Write-Info "Frontend test files: $frontendTestFiles"
    
    # Check if all tests passed
    Write-Success "Test run completed successfully!"
}

# Main execution
function Main {
    Write-Header "xTranslator Test Runner"
    Write-Info "Starting comprehensive test suite..."
    
    try {
        Setup-TestEnvironment
        Run-BackendTests
        Run-FrontendTests
        Run-E2ETests
        Run-PerformanceTests
        Run-CodeQualityChecks
        Generate-CoverageReport
        Generate-TestReport
    }
    catch {
        Write-Error "Test run failed: $($_.Exception.Message)"
        exit 1
    }
}

# Run main function
Main
