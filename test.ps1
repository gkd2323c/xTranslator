# xTranslator - Quick Test Suite (no Skyrim data required)
$ErrorActionPreference = "Stop"
Set-Location $PSScriptRoot

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "  xTranslator - Quick Test" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

Write-Host "Unit tests (xt-core)..." -ForegroundColor Yellow
cargo test -p xt-core --lib --quiet
if ($LASTEXITCODE -ne 0) { throw "Unit tests failed" }

Write-Host "Basic benchmarks..." -ForegroundColor Yellow
cargo test --release -p xtranslator-tests --test basic_benchmarks --quiet
if ($LASTEXITCODE -ne 0) { throw "Benchmarks failed" }

Write-Host "Test data generator..." -ForegroundColor Yellow
cargo test --release -p xtranslator-tests --test test_data_generator --quiet
if ($LASTEXITCODE -ne 0) { throw "Data generator tests failed" }

Write-Host ""
Write-Host "All tests passed." -ForegroundColor Green
