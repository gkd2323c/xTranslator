# Skyrim SE validation hardening regression check script
# Based on docs/skyrim-se-validation-plan.md Step 6
#
# Usage:
#   .\scripts\validate_skyrim_se.ps1                    # Run L1/L2 + build checks
#   .\scripts\validate_skyrim_se.ps1 -WithGoldenDiff    # Also run golden snapshot diff (needs Skyrim.esm)

param(
    [switch]$WithGoldenDiff,
    [string]$SkyrimEsmPath = $env:XTRANSLATOR_TEST_SKYRIM_ESM
)

$ErrorActionPreference = "Stop"
$script:Failed = $false

function Write-Step($n, $desc) {
    Write-Host "`n[Step $n] $desc" -ForegroundColor Cyan
}

function Write-Pass($msg) {
    Write-Host "  [PASS] $msg" -ForegroundColor Green
}

function Write-Fail($msg) {
    Write-Host "  [FAIL] $msg" -ForegroundColor Red
    $script:Failed = $true
}

function Write-Info($msg) {
    Write-Host "  [INFO] $msg" -ForegroundColor DarkGray
}

function Run-Command($cmd, $args_arr, $label) {
    $proc = Start-Process -FilePath $cmd -ArgumentList $args_arr -Wait -PassThru -NoNewWindow
    if ($proc.ExitCode -eq 0) {
        Write-Pass $label
        return $true
    } else {
        Write-Fail "$label (exit code $($proc.ExitCode))"
        return $false
    }
}

# -- Step 1: L1 Baseline --
Write-Step 1 "L1 Baseline - Backend unit tests"
Run-Command "cargo" @("test", "-p", "xt-core", "--lib") "xt-core unit tests (299 tests)"

# -- Step 2: L2 Module Tests --
Write-Step 2 "L2 Module - Specific module tests"
Run-Command "cargo" @("test", "-p", "xt-core", "--lib", "sst::v8::tests") "SST v8 tests"
Run-Command "cargo" @("test", "-p", "xt-core", "--lib", "xml::tests") "XML tests"
Run-Command "cargo" @("test", "-p", "xt-core", "--lib", "esp::record_tree::tests") "ESP record_tree tests"
Run-Command "cargo" @("test", "-p", "xt-core", "--lib", "vmad::tests") "VMAD tests"

# -- Step 3: Build Checks --
Write-Step 3 "Build Checks - Tauri backend + frontend"
Run-Command "cargo" @("build", "-p", "xtranslator-tauri") "Tauri backend build"

Push-Location "ui"
Run-Command "cmd.exe" @("/c", "npm", "run", "build") "Frontend build (tsc + vite)"
Pop-Location

# -- Step 4: Golden snapshot diff (optional) --
if ($WithGoldenDiff) {
    Write-Step 4 "Golden Snapshot Diff"

    if (-not $SkyrimEsmPath -or -not (Test-Path $SkyrimEsmPath)) {
        Write-Fail "Skyrim.esm not found. Set env XTRANSLATOR_TEST_SKYRIM_ESM or pass -SkyrimEsmPath."
        Write-Info "Example: `$env:XTRANSLATOR_TEST_SKYRIM_ESM = 'D:\Games\SkyrimSE\Data\Skyrim.esm'"
    } else {
        Write-Info "Using Skyrim.esm: $SkyrimEsmPath"

        $goldenFile = "docs\skyrim-se-golden-2026-05-24.md"
        $tmpFile = [System.IO.Path]::GetTempFileName()

        $proc = Start-Process -FilePath "cargo" `
            -ArgumentList @("run", "-p", "xt-cli", "--", "stats", $SkyrimEsmPath, "SkyrimSE") `
            -Wait -PassThru -NoNewWindow -RedirectStandardOutput $tmpFile

        if ($proc.ExitCode -ne 0) {
            Write-Fail "xt-cli stats failed"
        } else {
            $newContent = Get-Content $tmpFile -Raw
            $oldContent = Get-Content $goldenFile -Raw -ErrorAction SilentlyContinue

            if (-not $oldContent) {
                Write-Fail "Golden snapshot not found: $goldenFile"
            } else {
                $metrics = @(
                    @{ Label = "Total strings";     Pattern = "Total strings \| (\d+)" },
                    @{ Label = "Top-level GRUPs";   Pattern = "Top-level GRUPs \| (\d+)" },
                    @{ Label = "Sub GRUPs";         Pattern = "Sub GRUPs \| (\d+)" },
                    @{ Label = "CELL strings";      Pattern = "CELL strings \| (\d+)" },
                    @{ Label = "WRLD strings";      Pattern = "WRLD strings \| (\d+)" }
                )

                foreach ($m in $metrics) {
                    $newMatch = [regex]::Match($newContent, $m.Pattern)
                    $oldMatch = [regex]::Match($oldContent, $m.Pattern)

                    if (-not $newMatch.Success -or -not $oldMatch.Success) {
                        Write-Fail "Cannot extract $($m.Label)"
                        continue
                    }

                    $newVal = [int]$newMatch.Groups[1].Value
                    $oldVal = [int]$oldMatch.Groups[1].Value
                    $diff = [math]::Abs($newVal - $oldVal)
                    $pct = if ($oldVal -gt 0) { ($diff / $oldVal) * 100 } else { 0 }

                    if ($pct -gt 5.0) {
                        Write-Fail "$($m.Label): $oldVal -> $newVal (diff ${pct:F1}%, threshold 5%)"
                    } elseif ($diff -ne 0) {
                        Write-Pass "$($m.Label): $oldVal -> $newVal (diff ${pct:F1}%, within threshold)"
                    } else {
                        Write-Pass "$($m.Label): $newVal (no change)"
                    }
                }
            }
        }

        Remove-Item $tmpFile -ErrorAction SilentlyContinue
    }
} else {
    Write-Info "Skipping golden snapshot diff (pass -WithGoldenDiff to enable, requires Skyrim.esm)"
}

# -- Summary --
Write-Host "`n========================================" -ForegroundColor Magenta
if ($script:Failed) {
    Write-Host "VALIDATION FAILED - Check errors above" -ForegroundColor Red
    exit 1
} else {
    Write-Host "[PASS] All validation checks passed" -ForegroundColor Green
    exit 0
}
