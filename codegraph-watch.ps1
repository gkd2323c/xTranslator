param(
    [string]$ProjectDir = "C:\Users\gkd2323c\Documents\xTranslator",
    [int]$DebounceMs = 2000
)

Set-Location $ProjectDir

$watcher = New-Object System.IO.FileSystemWatcher
$watcher.Path = $ProjectDir
$watcher.IncludeSubdirectories = $true
$watcher.NotifyFilter = [System.IO.NotifyFilters]::FileName -bor [System.IO.NotifyFilters]::LastWrite -bor [System.IO.NotifyFilters]::DirectoryName
$watcher.Filter = "*.*"

$excludeDirs = @('\.git', '\.codegraph', 'node_modules', 'target', 'dist')
$watchAction = {
    $path = $Event.SourceEventArgs.FullPath
    $relative = $path.Substring($ProjectDir.Length)
    foreach ($ex in $excludeDirs) {
        if ($relative -match $ex) { return }
    }

    $ext = [System.IO.Path]::GetExtension($path)
    if ($ext -notin @('.rs', '.ts', '.tsx', '.js', '.jsx', '.css', '.scss', '.html', '.json', '.toml')) { return }

    Write-Host "[codegraph-watch] Change: $relative"
    & codegraph sync 2>&1 | Out-Null
}

$debounce = @{}
$onChanged = {
    $path = $Event.SourceEventArgs.FullPath
    $relative = $path.Substring($ProjectDir.Length)
    foreach ($ex in $excludeDirs) {
        if ($relative -match $ex) { return }
    }
    $ext = [System.IO.Path]::GetExtension($path)
    if ($ext -notin @('.rs', '.ts', '.tsx', '.js', '.jsx', '.css', '.scss', '.html', '.json', '.toml')) { return }

    $now = Get-Date
    $script:debounce[$path] = $now
    Start-Sleep -Milliseconds 2000
    if ($script:debounce[$path] -eq $now) {
        $script:debounce.Remove($path)
        Write-Host "[codegraph-watch] Sync: $relative"
        & codegraph sync 2>&1 | Out-Null
    }
}

Register-ObjectEvent $watcher "Created" -Action $onChanged > $null
Register-ObjectEvent $watcher "Changed" -Action $onChanged > $null
Register-ObjectEvent $watcher "Deleted" -Action $onChanged > $null
Register-ObjectEvent $watcher "Renamed" -Action $onChanged > $null

$watcher.EnableRaisingEvents = $true

Write-Host "[codegraph-watch] Watching $ProjectDir (debounce: ${DebounceMs}ms)"
Write-Host "[codegraph-watch] Press Ctrl+C to stop"

while ($true) { Start-Sleep -Seconds 10 }
