#!/usr/bin/env pwsh
# Launch xTranslator in dev mode (Vite + Tauri)
# Usage: ./dev.ps1

$ErrorActionPreference = "Stop"
$Root = $PSScriptRoot

# Kill any existing Vite or Tauri processes
$processes = @("node", "xtranslator-tauri")
foreach ($p in $processes) {
    Get-Process -Name $p -ErrorAction SilentlyContinue | Stop-Process -Force
}

Write-Host "Starting xTranslator dev environment..." -ForegroundColor Green

# Start Vite dev server in background
$viteJob = Start-Job -ScriptBlock {
    param($root)
    Set-Location "$root\ui"
    npm run dev
} -ArgumentList $Root

# Wait for Vite to be ready (port 5173)
Write-Host "Waiting for Vite on :5173..." -ForegroundColor Cyan
$maxWait = 30
$waited = 0
while ($waited -lt $maxWait) {
    Start-Sleep -Seconds 1
    try {
        $conn = Get-NetTCPConnection -LocalPort 5173 -ErrorAction SilentlyContinue
        if ($conn) {
            Write-Host "Vite is ready!" -ForegroundColor Green
            break
        }
    } catch {}
    $waited++
    Write-Host "  ...waiting ($waited/$maxWait)" -ForegroundColor Gray
}

if ($waited -ge $maxWait) {
    Write-Error "Vite failed to start within ${maxWait}s. Check the Vite log above."
    exit 1
}

# Start Tauri
Write-Host "Starting Tauri app..." -ForegroundColor Cyan
Set-Location "$Root"
cargo run -p xtranslator-tauri

# Cleanup: kill background job when Tauri exits
Stop-Job $viteJob
Remove-Job $viteJob
